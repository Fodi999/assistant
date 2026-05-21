// ── Right Inspector — contextual Object tab driven by CAD.ui ────────────────
//
// The cad_side_panel already renders tabs Grid/Snap/View/Object/Dev. We do NOT
// rewrite it. We:
//   1. Inject a new section `<div id="cad-obj-ctx">` AT THE TOP of the Object
//      page (data-page="object"), keeping all existing sections below.
//   2. Render that section based on CAD.ui.getSelectionData(). It re-renders
//      automatically on `selection:changed`.
//   3. Provide safe fallbacks: if the data shape isn't there, show 'No selection'.

pub const JS: &str = r##"
(function() {
  if (window.__cadRightInspectorInited) return;
  window.__cadRightInspectorInited = true;

  // ── Stylesheet for the contextual block ────────────────────────
  const css = document.createElement('style');
  css.id = 'cad-right-inspector-css';
  css.textContent = [
    '#cad-obj-ctx {',
    '  margin: 0 0 10px 0; padding: 10px 12px;',
    '  background: rgba(15,23,42,0.55);',
    '  border: 1px solid rgba(148,163,184,0.18);',
    '  border-radius: 10px;',
    '  font: 500 12px/1.4 "JetBrains Mono", monospace;',
    '  color: #e2e8f0;',
    '}',
    '#cad-obj-ctx .ctx-title {',
    '  font-size: 10px; letter-spacing: 0.08em; text-transform: uppercase;',
    '  color: rgba(103,232,249,0.85); margin-bottom: 8px; font-weight: 700;',
    '}',
    '#cad-obj-ctx .ctx-row { display: flex; justify-content: space-between; padding: 2px 0; }',
    '#cad-obj-ctx .ctx-lbl { color: rgba(148,163,184,0.75); }',
    '#cad-obj-ctx .ctx-val { color: #f1f5f9; }',
    '#cad-obj-ctx .ctx-empty { color: rgba(148,163,184,0.65); font-style: italic; }',
    '#cad-obj-ctx .ctx-divider { height: 1px; background: rgba(148,163,184,0.16); margin: 8px -12px; }',
    '#cad-obj-ctx .ctx-btn-row { display: flex; gap: 6px; margin-top: 8px; }',
    '#cad-obj-ctx .ctx-btn {',
    '  flex: 1; padding: 6px 8px; border-radius: 6px; cursor: pointer;',
    '  font: 600 10.5px/1 "JetBrains Mono", monospace; letter-spacing: 0.04em;',
    '  text-transform: uppercase; transition: all .12s;',
    '  background: rgba(103,232,249,0.10);',
    '  border: 1px solid rgba(103,232,249,0.45);',
    '  color: #67e8f9;',
    '}',
    '#cad-obj-ctx .ctx-btn:hover { background: rgba(103,232,249,0.18); }',
    '#cad-obj-ctx .ctx-btn.ctx-btn-ghost {',
    '  background: rgba(148,163,184,0.08); color: rgba(226,232,240,0.85);',
    '  border-color: rgba(148,163,184,0.22);',
    '}',
    '#cad-obj-ctx .ctx-btn.ctx-btn-danger {',
    '  background: rgba(248,113,113,0.10); color: #f87171; border-color: rgba(248,113,113,0.4);',
    '}',
    '#cad-obj-ctx input.ctx-input, #cad-obj-ctx select.ctx-input {',
    '  background: rgba(15,23,42,0.85); border: 1px solid rgba(148,163,184,0.25);',
    '  color: #f1f5f9; border-radius: 5px; padding: 4px 6px;',
    '  font: 500 11.5px/1 "JetBrains Mono", monospace; width: 90px;',
    '}',
    '#cad-obj-ctx input.ctx-input:focus, #cad-obj-ctx select.ctx-input:focus {',
    '  border-color: rgba(103,232,249,0.6); outline: none;',
    '}',
    '#cad-obj-ctx .ctx-section { margin-bottom: 6px; }',
    '#cad-obj-ctx .ctx-toggle { color: #94a3b8; cursor: pointer; }',
    '#cad-obj-ctx .ctx-toggle.on { color: #34d399; }',
  ].join('\n');
  document.head.appendChild(css);

  function _row(lbl, val) {
    return '<div class="ctx-row"><span class="ctx-lbl">' + lbl + '</span>'
         + '<span class="ctx-val">' + (val == null ? '—' : val) + '</span></div>';
  }
  function _fmtMm(v)   { return (v == null ? '—' : (Number(v).toFixed(2) + ' мм')); }
  function _fmtMm2(v)  { return (v == null ? '—' : (Number(v).toFixed(2) + ' мм²')); }
  function _esc(s)     { return String(s == null ? '' : s).replace(/[<>&"]/g, c => ({ '<':'&lt;','>':'&gt;','&':'&amp;','"':'&quot;' }[c])); }

  // ── Render contextual block based on selection ─────────────────
  function _render(data) {
    const host = document.getElementById('cad-obj-ctx');
    if (!host) return;
    data = data || (window.CAD && window.CAD.ui ? window.CAD.ui.getSelectionData() : { type: 'none' });

    const t = data.type;
    let html = '';

    if (t === 'none') {
      html =
        '<div class="ctx-title">No Selection</div>'
      + '<div class="ctx-empty">Ничего не выбрано — отображается статистика сцены.</div>'
      + '<div class="ctx-divider"></div>'
      + (data.sketch
          ? _row('Точки',    data.sketch.points)
          + _row('Рёбра',    data.sketch.edges)
          + _row('Профили',  data.sketch.profiles)
          + _row('Плоскость', data.sketch.plane)
          : '');
    }
    else if (t === 'sketch') {
      const s = data.sketch || {};
      html =
        '<div class="ctx-title">Sketch</div>'
      + _row('ID',        _esc(s.id))
      + _row('Плоскость', s.plane)
      + _row('Точек',     s.points)
      + _row('Рёбер',     s.edges)
      + _row('Профилей',  s.profiles)
      + '<div class="ctx-divider"></div>'
      + '<div class="ctx-row">'
      +   '<span class="ctx-lbl">Видимый</span>'
      +   '<span class="ctx-toggle ' + (s.visible ? 'on' : '') + '" onclick="window.__cadCtxToggleVisible(\'sketch\')">'
      +     (s.visible ? '👁 ON' : '👁 off')
      +   '</span>'
      + '</div>'
      + '<div class="ctx-btn-row">'
      +   '<button class="ctx-btn ctx-btn-ghost" onclick="window.__cadCtxEditSketch()">Edit Sketch</button>'
      + '</div>';
    }
    else if (t === 'profile') {
      const p = data.profile || {};
      html =
        '<div class="ctx-title">Profile</div>'
      + _row('ID',        _esc(p.id))
      + _row('Замкнут',   p.closed ? 'true' : 'false')
      + _row('Плоскость', p.plane)
      + _row('Площадь',   _fmtMm2(p.area))
      + _row('Рёбер',     p.edgeCount)
      + '<div class="ctx-btn-row">'
      +   '<button class="ctx-btn" onclick="window.__cadCtxStartExtrude()">⬆ Extrude (E)</button>'
      + '</div>';
    }
    else if (t === 'edge') {
      const e = data.edge || {};
      html =
        '<div class="ctx-title">Edge</div>'
      + _row('ID',     _esc(e.id))
      + _row('Выбрано', e.count)
      + (e.info ? _row('Концы', e.info.a + ' → ' + e.info.b) : '');
    }
    else if (t === 'point') {
      const p = data.point || {};
      html =
        '<div class="ctx-title">Point</div>'
      + _row('ID',     _esc(p.id))
      + _row('Выбрано', p.count);
    }
    else if (t === 'face') {
      const f = data.face || {};
      html =
        '<div class="ctx-title">Face</div>'
      + _row('ID', _esc(f.id));
    }
    else if (t === 'body') {
      const b = data.body || {};
      html =
        '<div class="ctx-title">Body</div>'
      + _row('ID',        _esc(b.id))
      + _row('Тип',       b.type || 'solid')
      + _row('Источник',  _esc(b.source))
      + _row('Вершин',    b.vertCount)
      + _row('Граней',    b.faceCount)
      + ((b.selectedFaceId != null && b.selectedFaceId !== 0)
          ? ('<div class="ctx-divider"></div>'
            + '<div class="ctx-sub">Picked face</div>'
            + _row('Face',     'F' + b.selectedFaceId)
            + _row('Global',   b.selectedGlobalFaceId || '—')
            + _row('Source',   b.selectedSourceFaceId || '—'))
          : '')
      + '<div class="ctx-divider"></div>'
      + '<div class="ctx-row">'
      +   '<span class="ctx-lbl">Видимый</span>'
      +   '<span class="ctx-toggle ' + (b.visible ? 'on' : '') + '" onclick="window.__cadCtxToggleVisible(\'body\',\'' + _esc(b.id) + '\')">'
      +     (b.visible ? '👁 ON' : '👁 off')
      +   '</span>'
      + '</div>'
      + '<div class="ctx-btn-row">'
      +   '<button class="ctx-btn ctx-btn-ghost" onclick="window.__cadCtxExportObj()">⬇ OBJ</button>'
      + '</div>';
    }
    else if (t === 'extrude') {
      const f = data.feature || {};
      html =
        '<div class="ctx-title">Feature — Extrude</div>'
      + _row('ID', _esc(f.id))
      + '<div class="ctx-row">'
      +   '<span class="ctx-lbl">Глубина</span>'
      +   '<input id="ctx-ex-depth" class="ctx-input" type="number" min="0.1" step="0.1" value="' + (f.depthMm || 10) + '">'
      + '</div>'
      + '<div class="ctx-row">'
      +   '<span class="ctx-lbl">Направление</span>'
      +   '<select id="ctx-ex-dir" class="ctx-input">'
      +     '<option value="positive"'  + (f.direction === 'positive'  ? ' selected' : '') + '>Positive</option>'
      +     '<option value="negative"'  + (f.direction === 'negative'  ? ' selected' : '') + '>Negative</option>'
      +     '<option value="symmetric"' + (f.direction === 'symmetric' ? ' selected' : '') + '>Symmetric</option>'
      +   '</select>'
      + '</div>'
      + '<div class="ctx-row">'
      +   '<span class="ctx-lbl">Операция</span>'
      +   '<select id="ctx-ex-op" class="ctx-input">'
      +     '<option value="new_body"' + (f.operation === 'new_body' ? ' selected' : '') + '>New Body</option>'
      +     '<option value="join"'     + (f.operation === 'join'     ? ' selected' : '') + '>Join</option>'
      +     '<option value="cut"'      + (f.operation === 'cut'      ? ' selected' : '') + '>Cut</option>'
      +   '</select>'
      + '</div>'
      + '<div class="ctx-btn-row">'
      +   '<button class="ctx-btn" onclick="window.__cadCtxApplyExtrude()">Apply ⏎</button>'
      +   '<button class="ctx-btn ctx-btn-ghost" onclick="window.__cadCtxCancelExtrude()">Cancel Esc</button>'
      + '</div>';
    }
    else {
      html = '<div class="ctx-title">Selection</div><div class="ctx-empty">Unknown selection type.</div>';
    }

    host.innerHTML = html;
  }

  // ── Mount the container at the top of the Object tab ───────────
  function _mount() {
    const objPage = document.querySelector('#cad-side-panel .csp-page[data-page="object"]');
    if (!objPage) return false;
    if (document.getElementById('cad-obj-ctx')) return true;
    const host = document.createElement('div');
    host.id = 'cad-obj-ctx';
    objPage.insertBefore(host, objPage.firstChild);
    _render();
    return true;
  }
  function _tryMount(tries) {
    if (_mount()) return;
    if (tries <= 0) return;
    setTimeout(() => _tryMount(tries - 1), 250);
  }
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => _tryMount(10));
  } else {
    _tryMount(10);
  }

  // ── Re-render on selection / mode changes ──────────────────────
  if (window.CAD && window.CAD.ui) {
    window.CAD.ui.on('selection:changed', _render);
    window.CAD.ui.on('mode:changed',      () => _render());
    // After Apply Extrude — switch focus to the freshly-created body.
    window.CAD.ui.on('document:changed', (ev) => {
      if (!ev || ev.kind !== 'extrude' || !ev.body) return;
      const ss = window.sketchState;
      if (ss) {
        // Clear previous selections.
        const clr = (s) => { if (!s) return; if (s.clear) s.clear(); else if (Array.isArray(s)) s.length = 0; };
        clr(ss.selectedPointIds); clr(ss.selectedEdgeIds);
        clr(ss.selectedFaceIds);
        ss.selectedProfileId = null;
        // Select new body.
        if (!ss.selectedBodyIds) ss.selectedBodyIds = new Set();
        if (ss.selectedBodyIds.add) ss.selectedBodyIds.add(ev.body.id);
        else if (Array.isArray(ss.selectedBodyIds)) ss.selectedBodyIds.push(ev.body.id);
      }
      if (window.CAD.document) window.CAD.document.setSelection('body', ev.body.id);
      // Close any leftover preview panel.
      if (typeof window.__closeSolidPreview === 'function') window.__closeSolidPreview();
      _render();
      if (window.__setStatusMessage) window.__setStatusMessage('✓ ' + ev.body.name + ' создан');
    });
  }

  // ── Context action stubs — safe wrappers ───────────────────────
  window.__cadCtxStartExtrude = function() {
    const ss = window.sketchState;
    const pid = ss && ss.selectedProfileId;
    // Prefer the real entry-point (handles closed profile + face/edge fallback).
    if (pid && typeof window.__startSolidExtrude === 'function') {
      return window.__startSolidExtrude(pid);
    }
    if (typeof window.__handleContextExtrude === 'function') {
      return window.__handleContextExtrude();
    }
    if (typeof window.__startSolidExtrude === 'function') {
      return window.__startSolidExtrude();
    }
    if (typeof window.__extrudeToSolid === 'function') {
      return window.__extrudeToSolid();
    }
    console.warn('[CAD.ui] no extrude entry-point available');
  };

  window.__cadCtxApplyExtrude = function() {
    const depth = parseFloat((document.getElementById('ctx-ex-depth') || {}).value);
    const dir   = (document.getElementById('ctx-ex-dir') || {}).value;
    const op    = (document.getElementById('ctx-ex-op')  || {}).value;

    // Push UI parameters into the live solid-extrude state.
    const xs = window.__solidExtrudeState;
    if (xs) {
      if (isFinite(depth) && depth > 0) xs.depthMm = depth;
      // Direction / operation are stored as private fields for now
      // (gizmo currently always extrudes positive — see TODO below).
      if (dir) xs._direction = dir;
      if (op)  xs._operation = op;
    }
    // Mirror for legacy readers.
    if (window.sketchState) {
      window.sketchState.extrude = window.sketchState.extrude || {};
      if (isFinite(depth) && depth > 0) window.sketchState.extrude.depthMm = depth;
      if (dir) window.sketchState.extrude.direction = dir;
      if (op)  window.sketchState.extrude.operation = op;
    }

    // Commit through the real pipeline (which wraps via CAD.document).
    if (typeof window.__commitSolidExtrude === 'function') {
      return window.__commitSolidExtrude();
    }
    if (typeof window.__applyExtrude === 'function')   return window.__applyExtrude();
    if (typeof window.__confirmExtrude === 'function') return window.__confirmExtrude();
    console.warn('[CAD.ui] no extrude apply hook found');
  };

  window.__cadCtxCancelExtrude = function() {
    if (typeof window.__cancelSolidExtrude === 'function') {
      return window.__cancelSolidExtrude();
    }
    if (typeof window.__cancelExtrude === 'function') return window.__cancelExtrude();
    if (typeof window.__closeSolidPreview === 'function') window.__closeSolidPreview();
    if (window.sketchState && window.sketchState.extrude) {
      window.sketchState.extrude.active  = false;
      window.sketchState.extrude.preview = false;
    }
  };

  window.__cadCtxExportObj = function() {
    // 1) Selected body → look up in document store for stored objData.
    const ss = window.sketchState;
    const id = ss && ss.selectedBodyIds && (ss.selectedBodyIds.size
                ? [...ss.selectedBodyIds][0]
                : (Array.isArray(ss.selectedBodyIds) && ss.selectedBodyIds[0]));
    if (id && window.CAD && window.CAD.document) {
      const b = window.CAD.document.findBody(id);
      if (b && b.objData) {
        return _downloadText(b.objData, 'body_' + id + '.obj');
      }
    }
    // 2) Last extrude result (legacy single-body model).
    const r = window.__lastSolidResult;
    if (r && r.obj_data) {
      return _downloadText(r.obj_data, 'solid_' + Date.now() + '.obj');
    }
    // 3) Any backend export hook.
    if (typeof window.__exportSelectedBodyObj === 'function') return window.__exportSelectedBodyObj();
    if (typeof window.__exportObj === 'function')              return window.__exportObj();
    console.warn('[CAD.ui] no OBJ export hook found');
    if (window.__setStatusMessage) window.__setStatusMessage('⚠ Нет данных OBJ');
  };

  function _downloadText(text, filename) {
    const blob = new Blob([text], { type: 'text/plain' });
    const url  = URL.createObjectURL(blob);
    const a    = document.createElement('a');
    a.href = url; a.download = filename;
    document.body.appendChild(a); a.click(); document.body.removeChild(a);
    URL.revokeObjectURL(url);
    if (window.__setStatusMessage) window.__setStatusMessage('⬇ ' + filename);
  }
  window.__cadCtxEditSketch = function() {
    if (window.__setSketchTool) window.__setSketchTool('select');
    if (window.__setStatusMessage) window.__setStatusMessage('Sketch edit mode');
  };
  window.__cadCtxToggleVisible = function(kind, id) {
    const ss = window.sketchState; if (!ss) return;
    const docu = window.CAD && window.CAD.document;
    if (kind === 'sketch') {
      ss.visible = !(ss.visible !== false);
    } else if (kind === 'body') {
      const real = String(id || '').replace(/^body_/, '');
      const fullId = real.startsWith('body_') ? real : ('body_' + real);
      let b = docu ? (docu.findBody(real) || docu.findBody(fullId)) : null;
      if (!b) b = (ss.bodies || []).find(x => String(x.id) === String(real) || String(x.id) === fullId);
      if (b) {
        b.visible = !(b.visible !== false);
        // Apply visibility to the WebGPU multi-body scene.
        if (window.CAD && window.CAD.renderer && typeof window.CAD.renderer.setBodyVisible === 'function') {
          window.CAD.renderer.setBodyVisible(b.id, b.visible);
        }
      }
    } else if (kind === 'feature') {
      const real = String(id || '').replace(/^(feature_|extrude_)/, '');
      const f = docu ? docu.findFeature('extrude_' + real) || docu.findFeature(real) : null;
      if (f) f.suppressed = !f.suppressed;
    }
    _render();
  };

  // ── Enter / Escape for extrude UI (when ctx open) ──────────────
  document.addEventListener('keydown', (e) => {
    const ctx = document.getElementById('cad-obj-ctx');
    if (!ctx || !ctx.querySelector('#ctx-ex-depth')) return;
    const t = e.target;
    if (t && (t.tagName === 'INPUT' || t.tagName === 'SELECT' || t.tagName === 'TEXTAREA')) {
      if (e.key === 'Enter') { e.preventDefault(); window.__cadCtxApplyExtrude(); }
      return;
    }
    if (e.key === 'Enter')      { e.preventDefault(); window.__cadCtxApplyExtrude(); }
    else if (e.key === 'Escape'){ e.preventDefault(); window.__cadCtxCancelExtrude(); }
  });

  console.log('[CAD.ui] right_inspector ready');
})();
"##;
