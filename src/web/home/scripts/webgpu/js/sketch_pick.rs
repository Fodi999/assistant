// ── JS: Sketch raycasting — plane intersection, snap, CAD solid picking ────────
// Domain: Sketch — all raycast logic isolated from state and tools.

pub const JS: &str = r##"
      // ─────────────────────────────────────────────────────────────
      // __raycastSketchPlane — ray vs active sketch plane.
      // Returns { x, y, z, snapType } or null.
      // snapType: 'first' | 'point' | 'origin' | 'align' | 'grid' | 'free'
      // opts: { ignoreIndex, skipGridSnap, _isHover }
      // ─────────────────────────────────────────────────────────────
      window.__raycastSketchPlane = function(ndcX, ndcY, opts) {
        opts = opts || {};
        const asp = canvas.width / canvas.height;
        const cp = Math.cos(cam.pitch), sp = Math.sin(cam.pitch);
        const cy = Math.cos(cam.yaw),   sy = Math.sin(cam.yaw);
        const fwdX = -sy * cp, fwdY = -sp, fwdZ = cy * cp;
        const roX = cam.target[0] - fwdX * cam.dist;
        const roY = cam.target[1] - fwdY * cam.dist;
        const roZ = cam.target[2] - fwdZ * cam.dist;
        let rX = fwdY*0 - fwdZ*1, rY = fwdZ*0 - fwdX*0, rZ = fwdX*1 - fwdY*0;
        const rL = Math.hypot(rX, rY, rZ) || 1; rX /= rL; rY /= rL; rZ /= rL;
        const uX = rY*fwdZ - rZ*fwdY, uY = rZ*fwdX - rX*fwdZ, uZ = rX*fwdY - rY*fwdX;
        let oX = roX, oY = roY, oZ = roZ, dX, dY, dZ;
        if (cam.ortho) {
          const oh = cam.dist * 0.45;
          oX += rX*(ndcX*asp)*oh + uX*ndcY*oh;
          oY += rY*(ndcX*asp)*oh + uY*ndcY*oh;
          oZ += rZ*(ndcX*asp)*oh + uZ*ndcY*oh;
          const fL = Math.hypot(fwdX, fwdY, fwdZ);
          dX = fwdX/fL; dY = fwdY/fL; dZ = fwdZ/fL;
        } else {
          const fl = 2.414;
          const dx = fwdX*fl + rX*(ndcX*asp) + uX*ndcY;
          const dy = fwdY*fl + rY*(ndcX*asp) + uY*ndcY;
          const dz = fwdZ*fl + rZ*(ndcX*asp) + uZ*ndcY;
          const L = Math.hypot(dx, dy, dz);
          dX = dx/L; dY = dy/L; dZ = dz/L;
        }
        if (!opts._isHover) {
          log(`[ray] ortho=${cam.ortho} plane=${sketchState.plane} pitch=${cam.pitch.toFixed(3)} yaw=${cam.yaw.toFixed(3)}`, '#6366f1');
          log(`[ray] origin=(${oX.toFixed(2)},${oY.toFixed(2)},${oZ.toFixed(2)}) dir=(${dX.toFixed(3)},${dY.toFixed(3)},${dZ.toFixed(3)})`, '#6366f1');
        }
        let t, hit = false, hx, hy, hz;
        const p = sketchState.plane;
        if (p === 'XZ' && Math.abs(dY) > 1e-6) {
          t = -oY/dY; if (t > 0) { hx = oX+dX*t; hy = 0; hz = oZ+dZ*t; hit = true; }
          else if (!opts._isHover) log(`[ray] XZ: t=${t?.toFixed(3)} ≤ 0 → miss`, '#f87171');
        } else if (p === 'XY' && Math.abs(dZ) > 1e-6) {
          t = -oZ/dZ; if (t > 0) { hx = oX+dX*t; hy = oY+dY*t; hz = 0; hit = true; }
          else if (!opts._isHover) log(`[ray] XY: t=${t?.toFixed(3)} ≤ 0 → miss`, '#f87171');
        } else if (p === 'YZ' && Math.abs(dX) > 1e-6) {
          t = -oX/dX; if (t > 0) { hx = 0; hy = oY+dY*t; hz = oZ+dZ*t; hit = true; }
          else if (!opts._isHover) log(`[ray] YZ: t=${t?.toFixed(3)} ≤ 0 → miss`, '#f87171');
        } else {
          if (!opts._isHover) log(`[ray] MISS — plane=${p} |dY|=${Math.abs(dY).toFixed(6)} |dZ|=${Math.abs(dZ).toFixed(6)} |dX|=${Math.abs(dX).toFixed(6)}`, '#f87171');
        }
        if (!hit) return null;

        let snapType = 'free';
        const pickR = Math.max(0.5, cam.dist * 0.05);

        // 1. Direct Point Snap (highest priority)
        if (sketchState.points.length > 0) {
          for (let i = 0; i < sketchState.points.length; i++) {
            if (i === opts.ignoreIndex) continue;
            const pp = sketchState.points[i];
            if (Math.hypot(hx-pp.x, hy-pp.y, hz-pp.z) < pickR) {
              return { x: pp.x, y: pp.y, z: pp.z, snapType: (i === 0 ? 'first' : 'point') };
            }
          }
        }
        // 2. Origin Snap
        if (Math.hypot(hx, hy, hz) < pickR) {
          return { x: 0, y: 0, z: 0, snapType: 'origin' };
        }
        // 3. Smart Guides / Axis Alignment Snap
        let alignedX = false, alignedY = false, alignedZ = false;
        if (sketchState.points.length > 0) {
          for (let i = 0; i < sketchState.points.length; i++) {
            if (i === opts.ignoreIndex) continue;
            const pp = sketchState.points[i];
            if (!alignedX && Math.abs(hx - pp.x) < pickR) { hx = pp.x; alignedX = true; }
            if (!alignedY && Math.abs(hy - pp.y) < pickR) { hy = pp.y; alignedY = true; }
            if (!alignedZ && Math.abs(hz - pp.z) < pickR) { hz = pp.z; alignedZ = true; }
          }
          if (alignedX || alignedY || alignedZ) snapType = 'align';
        }
        // 4. Grid Snap (fallback)
        if (opts.skipGridSnap !== true) {
          if (!alignedX) hx = snapToGrid(hx);
          if (!alignedY) hy = snapToGrid(hy);
          if (!alignedZ) hz = snapToGrid(hz);
          if (!alignedX && !alignedY && !alignedZ && window.sketchGridSnap > 0) snapType = 'grid';
        }
        return { x: hx, y: hy, z: hz, snapType };
      };

      // ─────────────────────────────────────────────────────────────
      // __buildPickRay — shared camera-space ray (used by CAD picker).
      // Returns { ox, oy, oz, dx, dy, dz }.
      // ─────────────────────────────────────────────────────────────
      window.__buildPickRay = function(ndcX, ndcY) {
        const asp = canvas.width / canvas.height;
        const cp = Math.cos(cam.pitch), sp = Math.sin(cam.pitch);
        const cy = Math.cos(cam.yaw),   sy = Math.sin(cam.yaw);
        const fwdX = -sy * cp, fwdY = -sp, fwdZ = cy * cp;
        const roX = cam.target[0] - fwdX * cam.dist;
        const roY = cam.target[1] - fwdY * cam.dist;
        const roZ = cam.target[2] - fwdZ * cam.dist;
        let rX = fwdY*0 - fwdZ*1, rY = fwdZ*0 - fwdX*0, rZ = fwdX*1 - fwdY*0;
        const rL = Math.hypot(rX, rY, rZ) || 1; rX /= rL; rY /= rL; rZ /= rL;
        const uX = rY*fwdZ - rZ*fwdY, uY = rZ*fwdX - rX*fwdZ, uZ = rX*fwdY - rY*fwdX;
        let oX = roX, oY = roY, oZ = roZ, dX, dY, dZ;
        if (cam.ortho) {
          const oh = cam.dist * 0.45;
          oX += rX*(ndcX*asp)*oh + uX*ndcY*oh;
          oY += rY*(ndcX*asp)*oh + uY*ndcY*oh;
          oZ += rZ*(ndcX*asp)*oh + uZ*ndcY*oh;
          const fL = Math.hypot(fwdX, fwdY, fwdZ);
          dX = fwdX/fL; dY = fwdY/fL; dZ = fwdZ/fL;
        } else {
          const fl = 2.414;
          const dx = fwdX*fl + rX*(ndcX*asp) + uX*ndcY;
          const dy = fwdY*fl + rY*(ndcX*asp) + uY*ndcY;
          const dz = fwdZ*fl + rZ*(ndcX*asp) + uZ*ndcY;
          const L = Math.hypot(dx, dy, dz);
          dX = dx/L; dY = dy/L; dZ = dz/L;
        }
        return { ox: oX, oy: oY, oz: oZ, dx: dX, dy: dY, dz: dZ };
      };

      // ─────────────────────────────────────────────────────────────
      // __raycastCadSolids — Möller–Trumbore ray vs all solid meshes.
      // Returns { solid, faceId, tri, t, point } or null.
      // ─────────────────────────────────────────────────────────────
      window.__raycastCadSolids = function(ndcX, ndcY) {
        const solids = window.solids || [];
        if (!solids.length) return null;
        const r = window.__buildPickRay(ndcX, ndcY);
        const ox = r.ox, oy = r.oy, oz = r.oz, dx = r.dx, dy = r.dy, dz = r.dz;
        const EPS = 1e-7;
        let bestT = Infinity, bestSolid = null, bestFaceId = 0, bestTri = -1;
        for (let si = 0; si < solids.length; si++) {
          const solid = solids[si];
          const m = solid.mesh;
          if (!m || !m.positions || !m.indices) continue;
          const pos = m.positions, idx = m.indices, faces = m.triangleGlobalFaceIds;
          const triCount = (idx.length / 3) | 0;
          for (let t = 0; t < triCount; t++) {
            const i0 = idx[t*3+0]*3, i1 = idx[t*3+1]*3, i2 = idx[t*3+2]*3;
            const ax = pos[i0], ay = pos[i0+1], az = pos[i0+2];
            const bx = pos[i1], by = pos[i1+1], bz = pos[i1+2];
            const cx = pos[i2], cy = pos[i2+1], cz = pos[i2+2];
            const e1x = bx-ax, e1y = by-ay, e1z = bz-az;
            const e2x = cx-ax, e2y = cy-ay, e2z = cz-az;
            const px = dy*e2z - dz*e2y, py = dz*e2x - dx*e2z, pz = dx*e2y - dy*e2x;
            const det = e1x*px + e1y*py + e1z*pz;
            if (det > -EPS && det < EPS) continue;
            const invDet = 1.0 / det;
            const tvx = ox-ax, tvy = oy-ay, tvz = oz-az;
            const u = (tvx*px + tvy*py + tvz*pz) * invDet;
            if (u < 0 || u > 1) continue;
            const qx = tvy*e1z - tvz*e1y, qy = tvz*e1x - tvx*e1z, qz = tvx*e1y - tvy*e1x;
            const v = (dx*qx + dy*qy + dz*qz) * invDet;
            if (v < 0 || u+v > 1) continue;
            const tt = (e2x*qx + e2y*qy + e2z*qz) * invDet;
            if (tt > EPS && tt < bestT) {
              bestT = tt; bestSolid = solid; bestTri = t;
              bestFaceId = faces ? faces[t] : 0;
            }
          }
        }
        if (!bestSolid) return null;
        return {
          solid: bestSolid,
          faceId: bestFaceId,
          tri: bestTri,
          t: bestT,
          point: { x: ox+dx*bestT, y: oy+dy*bestT, z: oz+dz*bestT },
        };
      };
"##;
