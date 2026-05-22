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
      // Single source of truth: orchestrator updates CadInteraction.selection,
      // sketchState, CAD.document, shader globals and emits UI events.
      if (window.CAD && window.CAD.selection
          && typeof window.CAD.selection.selectFace === 'function') {
        window.CAD.selection.selectFace(hit);
      } else {
        // Defensive fallback (orchestrator not loaded yet).
        c.sel.set({
          selected:     true,
          mode:         0,
          faceId:       hit.face.face_id,
          sourceFaceId: hit.face.source_face_id,
        });
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
      // Clicked empty area → clear via orchestrator (full deselect).
      if (window.CAD && window.CAD.selection
          && typeof window.CAD.selection.clear === 'function') {
        window.CAD.selection.clear();
      } else {
        c.sel.clear();
      }
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
