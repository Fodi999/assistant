// ── Face picker ──────────────────────────────────────────────────────────
//
//  Tests a screen-space ray against all logical faces of the current solid.
//  Returns the nearest hit or null.
//
//  API:
//    window.CadInteraction.picking.pickFace(canvasX, canvasY)
//      → { face, t, point: [x,y,z] } | null
//
//    canvasX/Y are *physical* pixel coords (CSS coords × devicePixelRatio).

pub const JS: &str = r##"
(function registerPickFace() {
  var P = window.CadInteraction.picking;
  if (!P || !P.rayTri || !P.makeCameraRay) {
    console.warn('[CadInteraction.picking] pickFace skipped — raycast missing');
    return;
  }

  P.pickFace = function(canvasX, canvasY) {
    var ray = P.makeCameraRay(canvasX, canvasY);
    if (!ray) return null;

    // ── Multi-body path ─────────────────────────────────────────────
    // Iterate over every visible body in CAD.renderBodies (+ optional
    // transient preview body). For each body, raycast its logical faces.
    // Body positions and face.triangle_indices are body-local.
    var bodies = (window.CAD && window.CAD.renderBodies)
      ? window.CAD.renderBodies.filter(function(b) {
          return b.visible !== false && b.faces && b.faces.length && b.positions;
        })
      : [];
    var pv = window.CAD && window.CAD._previewBody;
    if (pv && pv.faces && pv.faces.length && pv.positions) bodies = bodies.concat([pv]);

    // Legacy fallback: if no multi-body data yet, fall back to
    // window.__lastSolidResult so older flows still pick.
    if (bodies.length === 0) {
      var r = window.__lastSolidResult;
      if (!r || !r.faces || !r.faces.length) return null;
      bodies = [{
        id:        null,
        featureId: null,
        positions: r.positions,
        faces:     r.faces,
      }];
    }

    var bestT      = Infinity;
    var bestFace   = null;
    var bestBody   = null;
    var bestBodyIx = -1;

    for (var bi = 0; bi < bodies.length; bi++) {
      var body = bodies[bi];
      var pos  = body.positions;
      var faces = body.faces;
      for (var fi = 0; fi < faces.length; fi++) {
        var face = faces[fi];
        var tris = face.triangle_indices;
        if (!tris) continue;
        for (var ti = 0; ti < tris.length; ti += 3) {
          var i0 = tris[ti], i1 = tris[ti + 1], i2 = tris[ti + 2];
          var ax = pos[i0 * 3], ay = pos[i0 * 3 + 1], az = pos[i0 * 3 + 2];
          var bx = pos[i1 * 3], by = pos[i1 * 3 + 1], bz = pos[i1 * 3 + 2];
          var cx = pos[i2 * 3], cy = pos[i2 * 3 + 1], cz = pos[i2 * 3 + 2];
          var t  = P.rayTri(
            ray.ox, ray.oy, ray.oz, ray.dx, ray.dy, ray.dz,
            ax, ay, az,  bx, by, bz,  cx, cy, cz
          );
          if (t > 0 && t < bestT) {
            bestT      = t;
            bestFace   = face;
            bestBody   = body;
            bestBodyIx = bi;
          }
        }
      }
    }

    if (!bestFace) return null;

    // Compute globalFaceId for caller convenience. For the preview body
    // we use slot 998 (matches FACE_ID_STRIDE convention in buffers.rs).
    var globalFaceId = 0;
    if (bestBody && bestBody.id) {
      var renderArr = (window.CAD && window.CAD.renderBodies) || [];
      var idx = renderArr.indexOf(bestBody);
      var slot = (idx >= 0) ? (idx + 1) : 998;
      globalFaceId = slot * 1000 + (bestFace.face_id | 0);
    }

    return {
      face:         bestFace,
      t:            bestT,
      point: [
        ray.ox + ray.dx * bestT,
        ray.oy + ray.dy * bestT,
        ray.oz + ray.dz * bestT,
      ],
      body:         bestBody || null,
      bodyId:       bestBody ? bestBody.id      : null,
      featureId:    bestBody ? bestBody.featureId : null,
      localFaceId:  bestFace.face_id | 0,
      globalFaceId: globalFaceId,
    };
  };

  console.log('[CadInteraction.picking] pickFace ready');
})();
"##;
