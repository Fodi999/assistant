// ── Highlight bridge ─────────────────────────────────────────────────────
//
//  Connects the selection store to the render pipeline by mirroring the
//  selection snapshot into legacy globals that `render_loop_ubo.rs` reads
//  every frame:
//
//    window.__solidSelected     ← snap.selected
//    window.__solidSelMode      ← snap.mode
//    window.__solidSelFaceId    ← snap.sourceFaceId   (kernel face_id)
//    window.__solidHoverFaceId  ← snap.hoverSourceFaceId
//
//  In a future iteration the UBO writer should subscribe directly to the
//  store and these globals can disappear.
//
//  API:
//    window.CadInteraction.highlight.syncToUbo()  — manual force sync
//    window.CadInteraction.highlight.clear()      — resets everything

pub const JS: &str = r##"
(function registerHighlightBridge() {
  var sel = window.CadInteraction && window.CadInteraction.selection;
  if (!sel) {
    console.warn('[CadInteraction.highlight] no selection store — abort');
    return;
  }

  function _mirror(snap) {
    window.__solidSelected    = !!snap.selected;
    window.__solidSelMode     = snap.mode             || 0;
    window.__solidSelFaceId   = snap.sourceFaceId     || 0;
    window.__solidHoverFaceId = snap.hoverSourceFaceId || 0;
  }

  // Subscribe — also fires once immediately with current snapshot
  var _unsub = sel.subscribe(_mirror);

  window.CadInteraction.highlight = {
    syncToUbo: function() { _mirror(sel.snapshot()); },
    clear:     function() { sel.clear(); },
    _unsub:    _unsub,
  };

  console.log('[CadInteraction.highlight] bridge ready (store → UBO globals)');
})();
"##;
