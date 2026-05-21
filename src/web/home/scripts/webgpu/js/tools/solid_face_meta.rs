// ── Solid Face Input Adapter ─────────────────────────────────────────────
//
//  Thin layer that wires DOM mouse events on the WebGPU canvas to the
//  CadInteraction subsystem (selection store + face picker + debug overlay).
//
//  All heavy logic (face metadata, raycast, picking, store, overlay) lives
//  in `cad/interaction/*`. This file only:
//    - listens to canvas click  → picks a face → updates store
//    - listens to canvas hover  → updates store hover state
//
//  Legacy compatibility shims (kept so older callers still work):
//    window.__buildFaceMetadata(result)   → picking.buildFaceMetadata
//    window.__solidFacePick(x, y)         → picking.pickFace

pub const JS: &str = r##"
(function registerSolidFaceInput() {

  // ── Legacy shims so older callers keep working during migration ─────────
  window.__buildFaceMetadata = function(result) {
    var P = window.CadInteraction && window.CadInteraction.picking;
    return P ? P.buildFaceMetadata(result) : [];
  };
  window.__solidFacePick = function(px, py) {
    var P = window.CadInteraction && window.CadInteraction.picking;
    return P ? P.pickFace(px, py) : null;
  };

  function _ctx() {
    return {
      sel:     window.CadInteraction && window.CadInteraction.selection,
      pick:    window.CadInteraction && window.CadInteraction.picking,
      overlay: window.CadInteraction
              && window.CadInteraction.overlays
              && window.CadInteraction.overlays.debug,
    };
  }

  function _pixelCoords(canvas, e) {
    var rect = canvas.getBoundingClientRect();
    var dpr  = window.devicePixelRatio || 1;
    return { x: (e.clientX - rect.left) * dpr, y: (e.clientY - rect.top) * dpr };
  }

  // ── Canvas click → face select / object deselect ──────────────────────
  document.addEventListener('click', function(e) {
    var canvas = document.getElementById('webgpu-canvas');
    if (!canvas || e.target !== canvas) return;
    // Multi-body mode: pickFace iterates CAD.renderBodies. Legacy mode
    // still needs __lastSolidResult. Either path is OK.
    var haveBodies = (window.CAD && window.CAD.renderBodies && window.CAD.renderBodies.length > 0);
    if (!haveBodies && (!window.__lastSolidResult || !window.__lastSolidResult.faces)) return;

    var c = _ctx();
    if (!c.sel || !c.pick) return;

    var p   = _pixelCoords(canvas, e);
    var hit = c.pick.pickFace(p.x, p.y);
    if (hit) {
      c.sel.set({
        selected:     true,
        mode:         0,                          // OBJECT mode (whole-solid rim)
        faceId:       hit.face.face_id,
        sourceFaceId: hit.face.source_face_id,
      });

      // ── Sync selection to CAD.document + sketchState ──────────────
      if (hit.bodyId) {
        try {
          var ss = window.sketchState;
          if (ss) {
            if (!(ss.selectedBodyIds instanceof Set)) ss.selectedBodyIds = new Set();
            else ss.selectedBodyIds.clear();
            ss.selectedBodyIds.add(hit.bodyId);
            // Clear other selection kinds so Inspector switches to Body.
            if (ss.selectedFaceIds && ss.selectedFaceIds.clear) ss.selectedFaceIds.clear();
            if (ss.selectedEdgeIds && ss.selectedEdgeIds.clear) ss.selectedEdgeIds.clear();
            if (ss.selectedPointIds && ss.selectedPointIds.clear) ss.selectedPointIds.clear();
            ss.selectedProfileId = null;
          }
        } catch (_) {}
        try {
          if (window.CAD && window.CAD.document) {
            window.CAD.document.setSelection('body', hit.bodyId, {
              faceId:       hit.localFaceId,
              globalFaceId: hit.globalFaceId,
              featureId:    hit.featureId,
              sourceFaceId: hit.face.source_face_id,
              point:        hit.point,
            });
          }
        } catch (_) {}
        try {
          if (window.CAD && window.CAD.ui) {
            window.CAD.ui.emit('selection:changed', { type: 'body', id: hit.bodyId, faceId: hit.localFaceId });
            window.CAD.ui.emit('document:changed',  { kind: 'select', bodyId: hit.bodyId });
          }
        } catch (_) {}
      }

      console.log('[FaceInput] ✅ click face_id=' + hit.face.face_id +
        (hit.bodyId ? ' body=' + hit.bodyId + ' gf=' + hit.globalFaceId : '') +
        ' src=' + hit.face.source_face_id +
        ' t='   + hit.t.toFixed(4));
      if (window.__setStatusMessage)
        window.__setStatusMessage(
          (hit.bodyId ? hit.bodyId + ' · ' : '') +
          'Face F' + hit.face.face_id + ' selected'
        );
    } else {
      // Clicked empty area → clear selection
      c.sel.clear();
      try {
        var ss2 = window.sketchState;
        if (ss2 && ss2.selectedBodyIds && ss2.selectedBodyIds.clear) ss2.selectedBodyIds.clear();
        if (window.CAD && window.CAD.document) window.CAD.document.setSelection('none', null);
        if (window.CAD && window.CAD.ui) {
          window.CAD.ui.emit('selection:changed', { type: 'none' });
          window.CAD.ui.emit('document:changed',  { kind: 'deselect' });
        }
      } catch (_) {}
    }
  }, false);

  // ── Canvas hover → face hover highlight ───────────────────────────────
  var _lastHover = 0;
  document.addEventListener('mousemove', function(e) {
    var canvas = document.getElementById('webgpu-canvas');
    if (!canvas || e.target !== canvas) return;
    var haveBodies = (window.CAD && window.CAD.renderBodies && window.CAD.renderBodies.length > 0);
    if (!haveBodies && (!window.__lastSolidResult || !window.__lastSolidResult.faces)) return;

    var c = _ctx();
    if (!c.sel || !c.pick) return;

    var p   = _pixelCoords(canvas, e);
    var hit = c.pick.pickFace(p.x, p.y);
    var fid = hit ? hit.face.face_id : 0;
    if (fid !== _lastHover) {
      _lastHover = fid;
      c.sel.setHover({
        faceId:       hit ? hit.face.face_id        : 0,
        sourceFaceId: hit ? hit.face.source_face_id : 0,
      });
    }
  }, false);

  console.log('[solid-face-input] ready — wired to CadInteraction');
})();
"##;
