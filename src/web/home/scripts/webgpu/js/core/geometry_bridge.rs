// ── JS: Geometry Bridge — связь geometry_engine с WebGPU core ─────────────────
//
//  Этот модуль — единственная точка соединения между сервером (geometry_engine
//  на Rust) и GPU-сценой (WebGPU кор).
//
//  API (все функции на window.__geoBridge):
//
//    __geoBridge.upload(result)
//      Загружает ответ geometry_engine (positions/normals/face_ids/indices) в
//      cadPosBuf / cadNormalBuf / cadFaceIdBuf / cadIndexBuf через
//      window.__uploadSolidToScene.
//
//    __geoBridge.extrude(spec)          → Promise<result>
//      POST /api/matter/sketch/extrude, затем upload.
//      spec = { plane, depth, profile: [{x,y,z},...], bevel? }
//
//    __geoBridge.boolean(op, specA, specB)  → Promise<result>
//      POST /api/matter/geometry/boolean, затем upload.
//      op = "union" | "subtract" | "intersect"
//
//    __geoBridge.undo()
//      Откатывает последнюю операцию (загружает предыдущий меш в GPU).
//
//  Состояние (window.__geoBridge.state):
//    history   — стек операций (max 32), каждая = {op, result}
//    current   — последний загруженный result
//    loading   — флаг ожидания ответа сервера
//
//  Интеграция:
//    Загружается как часть core/ — ПОСЛЕ buffers.rs (cadPosBuf определён) и
//    ПЕРЕД render_loop (render loop читает cadIndexCount).

pub const JS: &str = r##"
// ── Geometry Bridge ──────────────────────────────────────────────────────────
window.__geoBridge = (() => {
  const MAX_HISTORY = 32;

  const state = {
    history: [],   // [{op, result}, ...]
    current: null, // last uploaded result
    loading: false,
  };

  // ── Internal: upload result to WebGPU CAD buffers ──────────────────────
  function _upload(result) {
    if (!result || !result.positions || !result.indices) return false;
    if (typeof window.__uploadSolidToScene !== 'function') {
      console.warn('[geoBridge] __uploadSolidToScene not ready yet');
      return false;
    }
    window.__uploadSolidToScene(result);

    // Build / rebuild face metadata after every upload
    if (typeof window.__buildFaceMetadata === 'function') {
      result.faces = window.__buildFaceMetadata(result);
      window.__lastSolidResult = result;
      if (window.__solidFacePanel) {
        window.__solidFacePanel.show();
        window.__solidFacePanel.update(result.faces, null);
      }
    }

    state.current = result;
    return true;
  }

  // ── Internal: push to history ──────────────────────────────────────────
  function _push(op, result) {
    state.history.push({ op, result });
    if (state.history.length > MAX_HISTORY) state.history.shift();
  }

  // ── Internal: status helper ────────────────────────────────────────────
  function _status(msg) {
    if (typeof window.__setStatusMessage === 'function') window.__setStatusMessage(msg);
  }

  // ── Internal: generic POST to geometry endpoint ────────────────────────
  async function _post(url, body) {
    const t0 = performance.now();
    const resp = await fetch(url, {
      method:  'POST',
      headers: { 'Content-Type': 'application/json' },
      body:    JSON.stringify(body),
    });
    const dt = (performance.now() - t0).toFixed(0);
    if (!resp.ok) {
      const err = await resp.json().catch(() => ({ error: resp.statusText }));
      throw new Error((err.error || resp.statusText) + ' (' + dt + ' мс)');
    }
    const result = await resp.json();
    result.__dt = dt;
    return result;
  }

  // ── Public: manual upload ──────────────────────────────────────────────
  function upload(result, opLabel) {
    _push(opLabel || 'manual', result);
    _upload(result);
  }

  // ── Public: extrude ────────────────────────────────────────────────────
  // spec = { plane, depth, profile: [{x,y,z},...], bevel? }
  async function extrude(spec) {
    if (state.loading) { _status('⏳ уже выполняется операция…'); return null; }
    state.loading = true;
    _status('⏳ extrude: строю solid…');
    try {
      const result = await _post('/api/matter/sketch/extrude', spec);
      _push('extrude', result);
      _upload(result);
      _status(
        '✅ Extrude: ' + result.vertex_count + ' вершин · ' +
        result.triangle_count + ' треуг · ' + result.__dt + ' мс'
      );
      console.log('[geoBridge.extrude]', {
        verts: result.vertex_count,
        tris:  result.triangle_count,
        dt:    result.__dt + ' мс',
      });
      return result;
    } catch (e) {
      _status('✗ extrude: ' + e.message);
      console.error('[geoBridge.extrude]', e);
      return null;
    } finally {
      state.loading = false;
    }
  }

  // ── Public: boolean CSG ────────────────────────────────────────────────
  // op = "union" | "subtract" | "intersect"
  // specA, specB = { plane, depth, profile: [{x,y,z},...], bevel? }
  async function boolean(op, specA, specB) {
    if (state.loading) { _status('⏳ уже выполняется операция…'); return null; }
    state.loading = true;
    const opLabel = { union: '∪ Union', subtract: '− Subtract', intersect: '∩ Intersect' }[op] || op;
    _status('⏳ ' + opLabel + ': считаю CSG…');
    try {
      const result = await _post('/api/matter/geometry/boolean', { op, a: specA, b: specB });
      _push(op, result);
      _upload(result);
      _status(
        '✅ ' + opLabel + ': ' +
        result.vertex_count + ' вершин · ' +
        result.triangle_count + ' треуг · ' +
        result.face_count + ' граней · ' +
        result.__dt + ' мс'
      );
      console.log('[geoBridge.boolean/' + op + ']', {
        verts:  result.vertex_count,
        tris:   result.triangle_count,
        faces:  result.face_count,
        kernel: result.kernel,
        dt:     result.__dt + ' мс',
      });
      return result;
    } catch (e) {
      _status('✗ boolean/' + op + ': ' + e.message);
      console.error('[geoBridge.boolean/' + op + ']', e);
      return null;
    } finally {
      state.loading = false;
    }
  }

  // ── Public: undo ───────────────────────────────────────────────────────
  function undo() {
    if (state.history.length < 2) {
      _status('⚠ нечего отменять');
      return false;
    }
    state.history.pop(); // remove current
    const prev = state.history[state.history.length - 1];
    if (prev && _upload(prev.result)) {
      _status('↩ Undo: откатился к «' + (prev.op || '?') + '»');
      return true;
    }
    return false;
  }

  // ── Public: clear scene ────────────────────────────────────────────────
  function clearScene() {
    const empty = {
      positions: [],
      normals:   [],
      face_ids:  [],
      indices:   [],
      vertex_count:   0,
      triangle_count: 0,
    };
    _upload(empty);
    state.history = [];
    state.current = null;
    _status('🗑 сцена очищена');
  }

  // ── Keyboard shortcut: Ctrl+Z = undo ──────────────────────────────────
  if (!window.__geoBridgeKeyInited) {
    window.__geoBridgeKeyInited = true;
    document.addEventListener('keydown', function(e) {
      if ((e.ctrlKey || e.metaKey) && e.key === 'z' && !e.shiftKey) {
        // Only intercept if no modal/input is focused
        const tag = document.activeElement?.tagName?.toLowerCase();
        if (tag === 'input' || tag === 'textarea') return;
        if (window.__geoBridge.undo()) {
          e.preventDefault();
          e.stopPropagation();
        }
      }
    }, false);
  }

  return { state, upload, extrude, boolean, undo, clearScene };
})();
"##;
