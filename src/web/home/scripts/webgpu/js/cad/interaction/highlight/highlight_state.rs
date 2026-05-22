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

    // ── Selected face id → UBO ────────────────────────────────────────────
    // In multi-body mode the per-vertex `cellMask` attribute holds
    // globalFaceId (= bodySlot*1000 + localFaceId). The shader compares
    // `cellMask == selected_face_id`, so we MUST feed globalFaceId here.
    //
    // The selection orchestrator (window.CAD.selection) is the canonical
    // source — it already wrote __solidSelGlobalFaceId. Prefer that.
    // Fallback to snap.sourceFaceId (legacy single-body path).
    var gf = (window.CAD && window.CAD.selection
              && window.CAD.selection.current
              && window.CAD.selection.current.globalFaceId) | 0;
    if (!gf) gf = (window.__solidSelGlobalFaceId | 0) || (snap.sourceFaceId | 0);
    window.__solidSelGlobalFaceId = gf;
    window.__solidSelFaceId       = gf;

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
