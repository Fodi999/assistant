// ── Raycast utilities ────────────────────────────────────────────────────
//
//  Camera-ray construction + Möller–Trumbore ray-triangle intersection.
//  Reads camera state from `window.cam` (yaw/pitch/dist/target/fov), which
//  must match the basis built by render_loop_ubo.rs to ensure picking
//  rays exactly correspond to what the user sees.
//
//  API:
//    window.CadInteraction.picking.makeCameraRay(canvasX, canvasY)
//      → { ox, oy, oz, dx, dy, dz } | null
//      coords are in *physical* canvas pixels (multiply CSS coords by DPR).
//
//    window.CadInteraction.picking.rayTri(rox, roy, roz, rdx, rdy, rdz,
//                                          ax, ay, az, bx, by, bz, cx, cy, cz)
//      → t  (>0 = hit distance, -1 = miss)

pub const JS: &str = r##"
(function registerRaycast() {
  window.CadInteraction.picking = window.CadInteraction.picking || {};

  window.CadInteraction.picking.rayTri = function(
    rox, roy, roz, rdx, rdy, rdz,
    ax,  ay,  az,  bx,  by,  bz,  cx2, cy2, cz2
  ) {
    var EPS = 1e-7;
    var e1x = bx - ax,  e1y = by - ay,  e1z = bz - az;
    var e2x = cx2 - ax, e2y = cy2 - ay, e2z = cz2 - az;
    var hx  = rdy * e2z - rdz * e2y;
    var hy  = rdz * e2x - rdx * e2z;
    var hz  = rdx * e2y - rdy * e2x;
    var a   = e1x * hx + e1y * hy + e1z * hz;
    if (Math.abs(a) < EPS) return -1;
    var f   = 1.0 / a;
    var sx  = rox - ax, sy = roy - ay, sz = roz - az;
    var u   = f * (sx * hx + sy * hy + sz * hz);
    if (u < 0 || u > 1) return -1;
    var qx  = sy * e1z - sz * e1y;
    var qy  = sz * e1x - sx * e1z;
    var qz  = sx * e1y - sy * e1x;
    var v   = f * (rdx * qx + rdy * qy + rdz * qz);
    if (v < 0 || u + v > 1) return -1;
    var t   = f * (e2x * qx + e2y * qy + e2z * qz);
    return t > EPS ? t : -1;
  };

  // Build a world-space ray from canvas pixel coords.
  // Basis must match render_loop_ubo.rs (right = cross(worldUp, fwd)).
  window.CadInteraction.picking.makeCameraRay = function(canvasX, canvasY) {
    var canvas = document.getElementById('webgpu-canvas');
    if (!canvas) return null;
    var W = canvas.width, H = canvas.height;
    if (W === 0 || H === 0) return null;

    var c     = window.cam || {};
    var yaw   = c.yaw   || 0;
    var pitch = c.pitch || 0;
    var dist  = c.dist  || 10;
    var tgt   = c.target || [0, 0, 0];

    var cy = Math.cos(yaw),   sy = Math.sin(yaw);
    var cp = Math.cos(pitch), sp = Math.sin(pitch);

    var fwdX = -sy * cp, fwdY = -sp, fwdZ = cy * cp;
    var roX  = tgt[0] - fwdX * dist;
    var roY  = tgt[1] - fwdY * dist;
    var roZ  = tgt[2] - fwdZ * dist;

    // right = cross(worldUp=[0,1,0], fwd) = [fwdZ, 0, -fwdX]
    var rX, rY, rZ;
    if (Math.abs(fwdY) > 0.999) { rX = 0;    rY = fwdZ; rZ = -fwdY; }
    else                        { rX = fwdZ; rY = 0;    rZ = -fwdX; }
    var rL = Math.hypot(rX, rY, rZ) || 1; rX /= rL; rY /= rL; rZ /= rL;

    var uX = rY * fwdZ - rZ * fwdY;
    var uY = rZ * fwdX - rX * fwdZ;
    var uZ = rX * fwdY - rY * fwdX;

    var ndcX =  (canvasX / W) * 2 - 1;
    var ndcY = -(canvasY / H) * 2 + 1;

    var fov    = (c.fov || 45) * Math.PI / 180;
    var aspect = W / H;
    var th     = Math.tan(fov * 0.5);

    var rdX = fwdX + rX * ndcX * th * aspect + uX * ndcY * th;
    var rdY = fwdY + rY * ndcX * th * aspect + uY * ndcY * th;
    var rdZ = fwdZ + rZ * ndcX * th * aspect + uZ * ndcY * th;
    var dl  = Math.hypot(rdX, rdY, rdZ) || 1;
    rdX /= dl; rdY /= dl; rdZ /= dl;

    return { ox: roX, oy: roY, oz: roZ, dx: rdX, dy: rdY, dz: rdZ };
  };

  console.log('[CadInteraction.picking] raycast ready');
})();
"##;
