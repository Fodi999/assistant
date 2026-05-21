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
    if (!window.__lastSolidResult || !window.__lastSolidResult.faces) return;

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
      console.log('[FaceInput] ✅ click face_id=' + hit.face.face_id +
        ' src=' + hit.face.source_face_id +
        ' t='   + hit.t.toFixed(4));
      if (window.__setStatusMessage)
        window.__setStatusMessage('Face F' + hit.face.face_id + ' selected');
    } else {
      // Clicked empty area → clear selection
      c.sel.clear();
    }
  }, false);

  // ── Canvas hover → face hover highlight ───────────────────────────────
  var _lastHover = 0;
  document.addEventListener('mousemove', function(e) {
    var canvas = document.getElementById('webgpu-canvas');
    if (!canvas || e.target !== canvas) return;
    if (!window.__lastSolidResult || !window.__lastSolidResult.faces) return;

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
