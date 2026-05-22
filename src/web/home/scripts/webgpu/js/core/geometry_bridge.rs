// ── JS: Geometry Bridge — geometry_engine WASM → WebGPU ──────────────────────
//
//  Единственная точка соединения между geometry_engine (WASM) и GPU-сценой.
//
//  API (window.__geoBridge):
//    upload(result)          — загрузить готовый меш в GPU буферы
//    extrude(spec)           — WASM sketch_extrude_json → upload
//    undo()                  — откатить последнюю операцию
//    clearScene()            — очистить сцену

pub const JS: &str = r##"
window.__geoBridge = (() => {
  const MAX_HISTORY = 32;
  const state = { history: [], current: null, loading: false };

  function _upload(result) {
    if (!result || !result.positions || !result.indices) return false;
    if (typeof window.__uploadSolidToScene !== 'function') {
      console.warn('[geoBridge] __uploadSolidToScene not ready');
      return false;
    }
    window.__uploadSolidToScene(result);
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

  function _push(op, result) {
    state.history.push({ op, result });
    if (state.history.length > MAX_HISTORY) state.history.shift();
  }

  function _status(msg) {
    if (typeof window.__setStatusMessage === 'function') window.__setStatusMessage(msg);
  }

  // ── Public: manual upload ──────────────────────────────────────────────
  function upload(result, opLabel) {
    _push(opLabel || 'manual', result);
    _upload(result);
  }

  // ── Public: extrude via WASM geometry_engine ──────────────────────────
  // spec = { sketch: SketchGraph, depth_m, plane?, bevel?, profile_id? }
  // OR legacy: { plane, depth, profile: [{x,y,z},...], bevel? }
  async function extrude(spec) {
    if (state.loading) { _status('⏳ уже выполняется…'); return null; }
    state.loading = true;
    _status('⏳ extrude…');
    try {
      if (!(await window.__ensureSketchWasm())) {
        throw new Error('geometry_engine WASM не загружен');
      }
      const result = window.__wasmSketchExtrude(spec);
      if (!result || !result.ok) {
        throw new Error(result?.error || 'extrude вернул ok=false');
      }
      _push('extrude', result);
      _upload(result);
      _status('✅ Extrude: ' + result.vertex_count + ' вершин · ' + result.triangle_count + ' треуг · ' + result.__dt + ' мс (WASM)');
      return result;
    } catch (e) {
      _status('✗ extrude: ' + e.message);
      console.error('[geoBridge.extrude]', e);
      return null;
    } finally {
      state.loading = false;
    }
  }

  // ── Public: undo ───────────────────────────────────────────────────────
  function undo() {
    if (state.history.length < 2) { _status('⚠ нечего отменять'); return false; }
    state.history.pop();
    const prev = state.history[state.history.length - 1];
    if (prev && _upload(prev.result)) {
      _status('↩ Undo: «' + (prev.op || '?') + '»');
      return true;
    }
    return false;
  }

  // ── Public: clear scene ────────────────────────────────────────────────
  function clearScene() {
    _upload({ positions: [], normals: [], face_ids: [], indices: [], vertex_count: 0, triangle_count: 0 });
    state.history = [];
    state.current = null;
    _status('🗑 сцена очищена');
  }

  // Ctrl+Z = undo
  if (!window.__geoBridgeKeyInited) {
    window.__geoBridgeKeyInited = true;
    document.addEventListener('keydown', function(e) {
      if ((e.ctrlKey || e.metaKey) && e.key === 'z' && !e.shiftKey) {
        const tag = document.activeElement?.tagName?.toLowerCase();
        if (tag === 'input' || tag === 'textarea') return;
        if (window.__geoBridge.undo()) { e.preventDefault(); e.stopPropagation(); }
      }
    }, false);
  }

  return { state, upload, extrude, undo, clearScene };
})();
"##;
