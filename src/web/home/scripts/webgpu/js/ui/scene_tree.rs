// ── Left Scene Tree — collapsible side panel with sketches/profiles/bodies/features
//
// Brand new DOM element, mounted to <body>. Toggle button on the left edge.
// Reads CAD.ui.getSceneTreeData() and re-renders on `selection:changed`.

pub const JS: &str = r##"
(function() {
  if (window.__cadSceneTreeInited) return;
  window.__cadSceneTreeInited = true;

  // ── Stylesheet ─────────────────────────────────────────────────
  const css = document.createElement('style');
  css.id = 'cad-scene-tree-css';
  css.textContent = [
    '#cad-scene-tree {',
    '  position: fixed; top: 56px; left: 12px; z-index: 10020;',
    '  width: 240px; max-height: calc(100vh - 200px);',
    '  display: flex; flex-direction: column;',
    '  background: rgba(15,23,42,0.82);',
    '  border: 1px solid rgba(148,163,184,0.18);',
    '  border-radius: 10px;',
    '  backdrop-filter: blur(10px);',
    '  box-shadow: 0 6px 22px rgba(0,0,0,0.5);',
    '  font: 500 12px/1.4 "JetBrains Mono", monospace;',
    '  color: #e2e8f0; user-select: none;',
    '  transition: transform .18s ease, opacity .18s ease;',
    '}',
    '#cad-scene-tree.collapsed { transform: translateX(-260px); opacity: 0; pointer-events: none; }',
    '#cad-scene-tree .cst-hdr {',
    '  display: flex; align-items: center; justify-content: space-between;',
    '  padding: 8px 10px; border-bottom: 1px solid rgba(148,163,184,0.16);',
    '  font-size: 10px; letter-spacing: 0.08em; text-transform: uppercase;',
    '  color: rgba(103,232,249,0.85); font-weight: 700;',
    '}',
    '#cad-scene-tree .cst-hdr-btn {',
    '  background: transparent; border: none; color: rgba(148,163,184,0.7);',
    '  cursor: pointer; padding: 0 4px; font-size: 14px;',
    '}',
    '#cad-scene-tree .cst-hdr-btn:hover { color: #67e8f9; }',
    '#cad-scene-tree .cst-body { overflow-y: auto; padding: 6px 4px; }',
    '#cad-scene-tree .cst-group {',
    '  font-size: 9.5px; letter-spacing: 0.1em; text-transform: uppercase;',
    '  color: rgba(148,163,184,0.65); margin: 8px 8px 4px;',
    '}',
    '#cad-scene-tree .cst-item {',
    '  display: flex; align-items: center; gap: 6px;',
    '  padding: 4px 8px; margin: 0 2px; border-radius: 5px;',
    '  cursor: pointer; transition: background .12s, color .12s;',
    '}',
    '#cad-scene-tree .cst-item:hover { background: rgba(148,163,184,0.08); }',
    '#cad-scene-tree .cst-item.selected {',
    '  background: rgba(103,232,249,0.12);',
    '  color: #67e8f9;',
    '  box-shadow: inset 2px 0 0 #67e8f9;',
    '}',
    '#cad-scene-tree .cst-icon { width: 16px; text-align: center; opacity: 0.8; }',
    '#cad-scene-tree .cst-name { flex: 1; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }',
    '#cad-scene-tree .cst-vis {',
    '  font-size: 11px; padding: 0 4px; border-radius: 3px;',
    '  color: rgba(148,163,184,0.55); cursor: pointer;',
    '}',
    '#cad-scene-tree .cst-vis:hover { color: #f1f5f9; }',
    '#cad-scene-tree .cst-vis.on    { color: #34d399; }',
    '#cad-scene-tree .cst-empty {',
    '  color: rgba(148,163,184,0.55); font-style: italic;',
    '  padding: 4px 10px; font-size: 11px;',
    '}',
    /* Reopen button (when collapsed) */
    '#cad-scene-tree-reopen {',
    '  position: fixed; top: 56px; left: 12px; z-index: 10021;',
    '  width: 28px; height: 28px; border-radius: 7px;',
    '  background: rgba(15,23,42,0.78); border: 1px solid rgba(148,163,184,0.22);',
    '  color: rgba(226,232,240,0.8); cursor: pointer; font-size: 14px;',
    '  display: none; align-items: center; justify-content: center;',
    '  backdrop-filter: blur(10px);',
    '}',
    '#cad-scene-tree-reopen:hover { color: #67e8f9; border-color: rgba(103,232,249,0.5); }',
    '#cad-scene-tree.collapsed ~ #cad-scene-tree-reopen { display: inline-flex; }',
  ].join('\n');
  document.head.appendChild(css);

  // ── DOM ───────────────────────────────────────────────────────
  function _mount() {
    if (document.getElementById('cad-scene-tree')) return;

    const panel = document.createElement('div');
    panel.id = 'cad-scene-tree';
    panel.innerHTML =
        '<div class="cst-hdr">'
      +   '<span>Scene</span>'
      +   '<button class="cst-hdr-btn" id="cst-collapse" title="Свернуть">×</button>'
      + '</div>'
      + '<div class="cst-body" id="cst-body"></div>';

    const reopen = document.createElement('button');
    reopen.id = 'cad-scene-tree-reopen';
    reopen.type = 'button';
    reopen.title = 'Открыть Scene Tree';
    reopen.textContent = '☰';

    document.body.appendChild(panel);
    document.body.appendChild(reopen);

    panel.querySelector('#cst-collapse').addEventListener('click', () => {
      panel.classList.add('collapsed');
    });
    reopen.addEventListener('click', () => {
      panel.classList.remove('collapsed');
    });
  }

  // ── Icon for each node type ────────────────────────────────────
  const ICONS = {
    sketch:  '✎',
    profile: '◇',
    body:    '◼',
    feature: '⚙',
  };
  const GROUPS = [
    { key: 'sketch',  label: 'Sketches'  },
    { key: 'profile', label: 'Profiles'  },
    { key: 'body',    label: 'Bodies'    },
    { key: 'feature', label: 'Features'  },
  ];

  function _esc(s) { return String(s == null ? '' : s).replace(/[<>&"]/g, c => ({ '<':'&lt;','>':'&gt;','&':'&amp;','"':'&quot;' }[c])); }

  function _render() {
    const body = document.getElementById('cst-body');
    if (!body) return;
    const nodes = (window.CAD && window.CAD.ui) ? window.CAD.ui.getSceneTreeData() : [];

    let html = '';
    let total = 0;
    GROUPS.forEach(g => {
      const items = nodes.filter(n => n.type === g.key);
      html += '<div class="cst-group">' + g.label + ' · ' + items.length + '</div>';
      if (!items.length) {
        html += '<div class="cst-empty">— нет —</div>';
        return;
      }
      total += items.length;
      items.forEach(n => {
        const visClass = n.visible ? 'cst-vis on' : 'cst-vis';
        const visIcon  = n.visible ? '👁' : '·';
        html +=
            '<div class="cst-item ' + (n.selected ? 'selected' : '') + '"'
          +   ' data-id="' + _esc(n.id) + '" data-type="' + g.key + '">'
          +   '<span class="cst-icon">' + ICONS[g.key] + '</span>'
          +   '<span class="cst-name" title="' + _esc(n.name) + '">' + _esc(n.name) + '</span>'
          +   '<span class="' + visClass + '" data-vis="1">' + visIcon + '</span>'
          + '</div>';
      });
    });

    if (total === 0) {
      html += '<div class="cst-empty" style="margin-top:8px">Сцена пуста</div>';
    }
    body.innerHTML = html;

    // Wire clicks
    body.querySelectorAll('.cst-item').forEach(el => {
      el.addEventListener('click', (e) => {
        const id   = el.dataset.id;
        const type = el.dataset.type;
        if (e.target && e.target.dataset && e.target.dataset.vis) {
          // visibility toggle
          if (window.__cadCtxToggleVisible) window.__cadCtxToggleVisible(type, id);
          _render();
          return;
        }
        if (window.__cadSceneTreeSelect) window.__cadSceneTreeSelect(type, id);
      });
    });
  }

  // ── Selection sink (defensive — only touches if hook exists) ───
  window.__cadSceneTreeSelect = function(type, id) {
    const ss = window.sketchState; if (!ss) return;
    // Clear other selections then assign
    function clear(s) { if (!s) return; if (s.clear) s.clear(); else s.length = 0; }
    clear(ss.selectedPointIds); clear(ss.selectedEdgeIds);
    clear(ss.selectedFaceIds);  clear(ss.selectedBodyIds);
    ss.selectedProfileId = null;

    if (type === 'profile') {
      const raw = id.replace(/^profile_/, '');
      ss.selectedProfileId = isNaN(+raw) ? raw : +raw;
    } else if (type === 'body') {
      const raw = id.replace(/^body_/, '');
      const bid = isNaN(+raw) ? raw : +raw;
      if (!ss.selectedBodyIds) ss.selectedBodyIds = new Set();
      if (ss.selectedBodyIds.add) ss.selectedBodyIds.add(bid);
      else if (Array.isArray(ss.selectedBodyIds)) ss.selectedBodyIds.push(bid);
    } else if (type === 'feature') {
      // Feature click → focus on the body it produced.
      const docu = window.CAD && window.CAD.document;
      const f = docu ? (docu.findFeature(id) || docu.findFeature(String(id).replace(/^feature_/, ''))) : null;
      if (f && f.bodyId) {
        if (!ss.selectedBodyIds) ss.selectedBodyIds = new Set();
        if (ss.selectedBodyIds.add) ss.selectedBodyIds.add(f.bodyId);
        else if (Array.isArray(ss.selectedBodyIds)) ss.selectedBodyIds.push(f.bodyId);
      }
    }
    if (window.CAD && window.CAD.document) {
      window.CAD.document.setSelection(type, id);
    }
    if (window.__updateSketchInspector) window.__updateSketchInspector();
    _render();
  };

  // ── Boot ───────────────────────────────────────────────────────
  function _boot() {
    _mount();
    _render();
    if (window.CAD && window.CAD.ui) {
      window.CAD.ui.on('selection:changed', _render);
      window.CAD.ui.on('mode:changed',      _render);
      window.CAD.ui.on('document:changed',  _render);
    }
  }
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', _boot);
  } else {
    _boot();
  }
  setTimeout(_render, 800);
  setTimeout(_render, 1800);

  console.log('[CAD.ui] scene_tree ready');
})();
"##;
