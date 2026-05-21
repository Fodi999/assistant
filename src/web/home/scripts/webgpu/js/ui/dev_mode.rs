// ── Dev Mode — gate FACE DEBUG / PERF / dev panels behind a single flag ─────
//
// Default state: devMode = false → all *.cad-dev-only / known debug panels
// are HIDDEN. User Mode shows ONLY the professional UI shell.
//
// Hotkey:   Ctrl+Shift+D   →  toggle dev mode.
// Button:   #cad-dev-toggle (top-right corner, tiny) — also flips devMode.
//
// Gated elements (id):
//   - perf-hud                      (Performance HUD — Shift+P)
//   - __cad_face_overlay            (FACE DEBUG list at left)
// Any element marked with class `cad-dev-only` is also hidden in user mode.

pub const JS: &str = r##"
(function() {
  if (window.__cadDevModeInited) return;
  window.__cadDevModeInited = true;

  // ── CSS that hides debug panels unless body.cad-dev-mode ───────
  const style = document.createElement('style');
  style.id = 'cad-dev-mode-css';
  style.textContent = [
    /* Hide known debug elements by default (User Mode) */
    'body:not(.cad-dev-mode) #perf-hud,',
    'body:not(.cad-dev-mode) #__cad_face_overlay,',
    'body:not(.cad-dev-mode) #gpu-hud,',
    'body:not(.cad-dev-mode) #__se_preview_panel,',
    'body:not(.cad-dev-mode) .cad-dev-only {',
    '  display: none !important;',
    '}',
    /* Dev mode badge */
    '#cad-dev-toggle {',
    '  position: fixed; top: 10px; right: 14px; z-index: 10050;',
    '  width: 28px; height: 22px; padding: 0;',
    '  display: inline-flex; align-items: center; justify-content: center;',
    '  font: 600 10px/1 "JetBrains Mono", monospace;',
    '  letter-spacing: 0.08em; text-transform: uppercase;',
    '  background: rgba(15,23,42,0.72);',
    '  color: rgba(148,163,184,0.85);',
    '  border: 1px solid rgba(148,163,184,0.22);',
    '  border-radius: 6px; cursor: pointer;',
    '  backdrop-filter: blur(6px);',
    '  transition: color .12s, border-color .12s, background .12s;',
    '}',
    '#cad-dev-toggle:hover { color: #67e8f9; border-color: rgba(103,232,249,0.45); }',
    'body.cad-dev-mode #cad-dev-toggle {',
    '  color: #fde68a; border-color: rgba(252,211,77,0.55);',
    '  background: rgba(252,211,77,0.10);',
    '}',
  ].join('\n');
  document.head.appendChild(style);

  // ── Tiny toggle button (top-right) ─────────────────────────────
  function _mountToggle() {
    if (document.getElementById('cad-dev-toggle')) return;
    const btn = document.createElement('button');
    btn.id = 'cad-dev-toggle';
    btn.type = 'button';
    btn.title = 'Dev Mode (Ctrl+Shift+D)';
    btn.textContent = 'DEV';
    btn.addEventListener('click', () => {
      if (window.CAD && window.CAD.ui) window.CAD.ui.toggleDevMode();
    });
    document.body.appendChild(btn);
  }
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', _mountToggle);
  } else {
    _mountToggle();
  }

  // ── Hotkey Ctrl+Shift+D ────────────────────────────────────────
  document.addEventListener('keydown', (e) => {
    if (!e.ctrlKey || !e.shiftKey) return;
    if (e.key !== 'D' && e.key !== 'd') return;
    // Skip if user is typing in an input/textarea
    const t = e.target;
    if (t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.isContentEditable)) return;
    e.preventDefault();
    e.stopPropagation();
    if (window.CAD && window.CAD.ui) window.CAD.ui.toggleDevMode();
  }, true);

  // ── React to dev mode changes (status message) ─────────────────
  function _onDev(on) {
    if (window.__setStatusMessage) {
      window.__setStatusMessage(on ? 'Dev Mode ON — debug panels visible' : 'User Mode — debug panels hidden');
    }
    // Re-render FACE DEBUG overlay if it was open
    const fo = document.getElementById('__cad_face_overlay');
    if (fo && on && fo.style.display === 'none' && window.CadInteraction
        && window.CadInteraction.overlays && window.CadInteraction.overlays.debug) {
      // leave hidden — user can toggle manually; we just unblock visibility
    }
  }
  if (window.CAD && window.CAD.ui) {
    window.CAD.ui.on('dev:changed', _onDev);
  }

  console.log('[CAD.ui] dev_mode ready — Ctrl+Shift+D to toggle');
})();
"##;
