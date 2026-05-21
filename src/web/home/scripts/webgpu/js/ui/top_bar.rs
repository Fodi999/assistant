// ── Top Status Bar — restyle existing #mini-bar + sync via CAD.ui ────────────
//
// The HTML #mini-bar is already produced by matter_lab.rs (mode/tool/plane/snap
// cells + ? shortcut button). matter_ui.rs writes #mini-mode/#mini-tool/...
// every render. We DO NOT change the markup or those writes; we just:
//   * Inject a cleaner stylesheet (less saturated colours, accent on active).
//   * Add a small "MODE" pill prefix in front (Free 3D / Sketch / Solid Edit).
//   * Mirror snap/grid extra info using CAD.ui events.

pub const JS: &str = r##"
(function() {
  if (window.__cadTopBarInited) return;
  window.__cadTopBarInited = true;

  // ── Stylesheet override ────────────────────────────────────────
  const css = document.createElement('style');
  css.id = 'cad-top-bar-css';
  css.textContent = [
    '#mini-bar {',
    '  background: rgba(15,23,42,0.78) !important;',
    '  border: 1px solid rgba(148,163,184,0.18) !important;',
    '  border-radius: 10px !important;',
    '  backdrop-filter: blur(10px);',
    '  box-shadow: 0 4px 18px rgba(0,0,0,0.45);',
    '  padding: 4px 10px !important;',
    '  font: 500 11.5px/1 "JetBrains Mono", monospace !important;',
    '  color: rgba(226,232,240,0.85);',
    '  display: inline-flex; align-items: center; gap: 8px;',
    '}',
    '#mini-bar .mb-cell b {',
    '  color: rgba(148,163,184,0.7);',
    '  font-weight: 600; letter-spacing: 0.04em;',
    '  text-transform: uppercase; font-size: 10px;',
    '  margin-right: 4px;',
    '}',
    '#mini-bar .mb-cell span:not(.mb-sep) { color: #e2e8f0; }',
    '#mini-bar .mb-sep { color: rgba(148,163,184,0.35); margin: 0 2px; }',
    '#mini-bar #shortcuts-toggle {',
    '  background: rgba(148,163,184,0.10);',
    '  border: 1px solid rgba(148,163,184,0.20);',
    '  color: rgba(226,232,240,0.85);',
    '  width: 22px; height: 22px; border-radius: 6px;',
    '  cursor: pointer; font-size: 12px; padding: 0;',
    '}',
    '#mini-bar #shortcuts-toggle:hover { color: #67e8f9; border-color: rgba(103,232,249,0.45); }',

    /* New MODE pill, prepended to the bar via JS */
    '#mini-bar .cad-mode-pill {',
    '  display: inline-flex; align-items: center; gap: 4px;',
    '  height: 22px; padding: 0 8px;',
    '  border-radius: 6px;',
    '  background: rgba(99,102,241,0.18);',
    '  border: 1px solid rgba(99,102,241,0.45);',
    '  color: #c7d2fe; font-weight: 600;',
    '  text-transform: uppercase; letter-spacing: 0.06em; font-size: 10px;',
    '}',
    '#mini-bar .cad-mode-pill[data-mode="solid_edit"] {',
    '  background: rgba(251,191,36,0.15); border-color: rgba(251,191,36,0.5); color: #fde68a;',
    '}',
    '#mini-bar .cad-mode-pill[data-mode="free3d"] {',
    '  background: rgba(52,211,153,0.12); border-color: rgba(52,211,153,0.4); color: #6ee7b7;',
    '}',
    '#mini-bar .cad-active-tool {',
    '  color: #67e8f9 !important; font-weight: 600;',
    '}',
    '#mini-bar .cad-extra {',
    '  color: rgba(148,163,184,0.7); font-size: 10.5px;',
    '}',
  ].join('\n');
  document.head.appendChild(css);

  // ── Inject MODE pill at the very left of the existing mini-bar ─
  function _ensureModePill() {
    const bar = document.getElementById('mini-bar');
    if (!bar) return null;
    let pill = bar.querySelector('.cad-mode-pill');
    if (!pill) {
      pill = document.createElement('span');
      pill.className = 'cad-mode-pill';
      pill.dataset.mode = 'sketch';
      pill.textContent = 'SKETCH';
      bar.insertBefore(pill, bar.firstChild);
    }
    return pill;
  }

  // ── Add grid/help cells if missing ─────────────────────────────
  function _ensureExtraCells() {
    const bar = document.getElementById('mini-bar');
    if (!bar) return;
    if (!bar.querySelector('[data-cad-grid]')) {
      const sep = document.createElement('span'); sep.className = 'mb-sep'; sep.textContent = '·';
      const cell = document.createElement('span');
      cell.className = 'mb-cell cad-extra'; cell.dataset.cadGrid = '1';
      cell.innerHTML = '<b>Сетка</b> <span id="mini-grid">10 мм</span>';
      const helpBtn = document.getElementById('shortcuts-toggle');
      if (helpBtn) { bar.insertBefore(sep, helpBtn); bar.insertBefore(cell, helpBtn); }
      else         { bar.appendChild(sep); bar.appendChild(cell); }
    }
  }

  function _refresh() {
    const pill = _ensureModePill();
    _ensureExtraCells();
    if (!window.CAD || !window.CAD.ui) return;
    const mode  = window.CAD.ui.getMode();
    const tool  = window.CAD.ui.getActiveTool();
    const plane = window.CAD.ui.getPlane();
    const snap  = window.CAD.ui.getSnapInfo();
    const grid  = window.CAD.ui.getGridMm();

    if (pill) {
      pill.dataset.mode = mode;
      pill.textContent = mode === 'solid_edit' ? 'SOLID EDIT'
                       : mode === 'free3d'     ? 'FREE 3D'
                       : 'SKETCH';
    }

    const set = (id, v) => { const el = document.getElementById(id); if (el) el.textContent = v; };
    set('mini-tool',  (tool || 'select').toUpperCase());
    set('mini-plane', plane);
    set('mini-snap',  snap.on ? snap.label : 'off');
    set('mini-grid',  grid + ' мм');

    // Highlight tool value with accent
    const tEl = document.getElementById('mini-tool');
    if (tEl) tEl.classList.add('cad-active-tool');
  }

  function _wire() {
    _refresh();
    if (window.CAD && window.CAD.ui) {
      window.CAD.ui.on('tool:changed',  _refresh);
      window.CAD.ui.on('mode:changed',  _refresh);
      window.CAD.ui.on('plane:changed', _refresh);
      window.CAD.ui.on('snap:changed',  _refresh);
      window.CAD.ui.on('grid:changed',  _refresh);
    }
  }
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', _wire);
  } else {
    setTimeout(_wire, 0);
  }
  // also retry once after WebGPU bootstrap (mini-bar mounted in matter_lab)
  setTimeout(_refresh, 600);
  setTimeout(_refresh, 1500);

  console.log('[CAD.ui] top_bar ready');
})();
"##;
