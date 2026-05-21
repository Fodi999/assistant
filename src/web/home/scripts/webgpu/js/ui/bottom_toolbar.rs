// ── Bottom Toolbar polish — group #universal-toolbar buttons & active accent ─
//
// The HTML `<nav id="universal-toolbar">` already exists in matter_lab.rs with
// `.utb-btn[data-tool]` and `.utb-sep` separators. We do NOT change markup —
// we only inject:
//   * Stylesheet for clean active state (cyan accent), thinner borders,
//     consistent spacing, professional dark glass background.
//   * Tiny group labels under each `.utb-sep` block (Select / Sketch / Modify /
//     Solid / Utility) rendered as ::before pseudo on a wrapper class.
//   * Hotkey hints appended as small superscripts inside button title (already
//     in title attr; we additionally render visible mini-badges).

pub const JS: &str = r##"
(function() {
  if (window.__cadBottomToolbarInited) return;
  window.__cadBottomToolbarInited = true;

  // ── Hotkey map for visible mini-badges ─────────────────────────
  const HOTKEY = {
    select: 'S', point: 'P', line: 'L', rect: 'R',
    circle: 'C', grab: 'G', delete: '⌫',
  };
  const HOTKEY_BY_ID = {
    'btn-square'   : '⇧R',
    'btn-extrude'  : 'E',
    'btn-ortho'    : 'O',
    'btn-help'     : '?',
    'btn-fix-point': 'F',
    'btn-solve'    : '⇧S',
  };

  // ── Stylesheet ─────────────────────────────────────────────────
  const css = document.createElement('style');
  css.id = 'cad-bottom-toolbar-css';
  css.textContent = [
    '#universal-toolbar {',
    '  background: rgba(15,23,42,0.78) !important;',
    '  border: 1px solid rgba(148,163,184,0.18) !important;',
    '  border-radius: 12px !important;',
    '  backdrop-filter: blur(10px);',
    '  box-shadow: 0 6px 22px rgba(0,0,0,0.5);',
    '  padding: 6px 8px !important;',
    '  gap: 4px !important;',
    '  font: 500 11px/1.1 "JetBrains Mono", monospace !important;',
    '}',
    '#universal-toolbar .utb-btn {',
    '  position: relative;',
    '  background: transparent !important;',
    '  border: 1px solid transparent !important;',
    '  color: rgba(226,232,240,0.78) !important;',
    '  border-radius: 8px !important;',
    '  padding: 6px 10px !important;',
    '  display: inline-flex; flex-direction: column; align-items: center; gap: 2px;',
    '  min-width: 46px;',
    '  cursor: pointer;',
    '  transition: color .12s, background .12s, border-color .12s, transform .08s;',
    '}',
    '#universal-toolbar .utb-btn .utb-label {',
    '  font-size: 10px !important; letter-spacing: 0.03em; opacity: 0.82;',
    '}',
    '#universal-toolbar .utb-btn:hover {',
    '  background: rgba(148,163,184,0.10) !important;',
    '  border-color: rgba(148,163,184,0.18) !important;',
    '  color: #f1f5f9 !important;',
    '}',
    /* Active state — cyan accent */
    '#universal-toolbar .utb-btn.active,',
    '#universal-toolbar .utb-btn[data-toggle="ortho"].active {',
    '  background: rgba(103,232,249,0.12) !important;',
    '  border-color: rgba(103,232,249,0.55) !important;',
    '  color: #67e8f9 !important;',
    '  box-shadow: 0 0 0 1px rgba(103,232,249,0.10), 0 4px 12px rgba(103,232,249,0.10);',
    '}',
    /* Delete = subtle red accent */
    '#universal-toolbar .utb-btn[data-tool="delete"]:hover {',
    '  color: #f87171 !important;',
    '  border-color: rgba(248,113,113,0.35) !important;',
    '  background: rgba(248,113,113,0.08) !important;',
    '}',
    /* Hotkey mini-badge */
    '#universal-toolbar .utb-btn .cad-hk {',
    '  position: absolute; top: 2px; right: 3px;',
    '  font: 500 8.5px/1 "JetBrains Mono", monospace;',
    '  color: rgba(148,163,184,0.55);',
    '  padding: 0 3px; border-radius: 3px;',
    '  background: rgba(148,163,184,0.08);',
    '  pointer-events: none;',
    '}',
    '#universal-toolbar .utb-btn.active .cad-hk { color: #67e8f9; background: rgba(103,232,249,0.16); }',

    /* Group separators (taller, with optional label) */
    '#universal-toolbar .utb-sep {',
    '  width: 1px !important; height: 28px !important;',
    '  background: linear-gradient(to bottom, transparent, rgba(148,163,184,0.30), transparent) !important;',
    '  margin: 0 4px !important;',
    '}',
  ].join('\n');
  document.head.appendChild(css);

  // ── Append hotkey badges to each tool button ───────────────────
  function _badgeAll() {
    const nav = document.getElementById('universal-toolbar');
    if (!nav) return;
    nav.querySelectorAll('.utb-btn').forEach(btn => {
      if (btn.querySelector('.cad-hk')) return; // idempotent
      const tool = btn.dataset.tool;
      let hk = tool ? HOTKEY[tool] : null;
      if (!hk && btn.id) hk = HOTKEY_BY_ID[btn.id];
      if (!hk) return;
      const span = document.createElement('span');
      span.className = 'cad-hk';
      span.textContent = hk;
      btn.appendChild(span);
    });
  }
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', _badgeAll);
  } else {
    setTimeout(_badgeAll, 0);
  }
  setTimeout(_badgeAll, 600);
  setTimeout(_badgeAll, 1500);

  console.log('[CAD.ui] bottom_toolbar ready');
})();
"##;
