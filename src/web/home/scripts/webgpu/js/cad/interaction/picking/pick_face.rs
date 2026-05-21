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
    var r = window.__lastSolidResult;
    if (!r || !r.faces || !r.faces.length) return null;

    var ray = P.makeCameraRay(canvasX, canvasY);
    if (!ray) return null;

    var pos     = r.positions;
    var bestT   = Infinity;
    var bestFace = null;

    for (var fi = 0; fi < r.faces.length; fi++) {
      var face = r.faces[fi];
      var tris = face.triangle_indices;
      for (var ti = 0; ti < tris.length; ti += 3) {
        var i0 = tris[ti], i1 = tris[ti + 1], i2 = tris[ti + 2];
        var ax = pos[i0 * 3], ay = pos[i0 * 3 + 1], az = pos[i0 * 3 + 2];
        var bx = pos[i1 * 3], by = pos[i1 * 3 + 1], bz = pos[i1 * 3 + 2];
        var cx = pos[i2 * 3], cy = pos[i2 * 3 + 1], cz = pos[i2 * 3 + 2];
        var t  = P.rayTri(
          ray.ox, ray.oy, ray.oz, ray.dx, ray.dy, ray.dz,
          ax, ay, az,  bx, by, bz,  cx, cy, cz
        );
        if (t > 0 && t < bestT) { bestT = t; bestFace = face; }
      }
    }

    if (!bestFace) return null;
    return {
      face: bestFace,
      t:    bestT,
      point: [
        ray.ox + ray.dx * bestT,
        ray.oy + ray.dy * bestT,
        ray.oz + ray.dz * bestT,
      ],
    };
  };

  console.log('[CadInteraction.picking] pickFace ready');
})();
"##;
