// ── Selection modes (parallel to the Selection HUD enum) ─────────────────
//  These map 1:1 to the shader's `selectionMode` (u9.w) values.

pub const JS: &str = r##"
(function registerSelectionModes() {
  window.CadInteraction.SelectionMode = Object.freeze({
    OBJECT: 0,
    FACE:   1,
    EDGE:   2,
    VERTEX: 3,
  });
})();
"##;
