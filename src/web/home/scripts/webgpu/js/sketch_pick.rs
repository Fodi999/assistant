// ── JS: Sketch raycasting — plane-aware intersection + screen-space picking ──
// Domain: Sketch — pure geometry.

pub const JS: &str = r##"
      // ─────────────────────────────────────────────────────────
      // Internal: build a camera ray for (ndcX, ndcY).
      // Returns { ox, oy, oz, dx, dy, dz }.
      // ─────────────────────────────────────────────────────────
      function __pickRay(ndcX, ndcY) {
        const asp = canvas.width / canvas.height;
        const cp = Math.cos(cam.pitch), sp = Math.sin(cam.pitch);
        const cyy = Math.cos(cam.yaw),  syy = Math.sin(cam.yaw);
        const fwdX = -syy * cp, fwdY = -sp, fwdZ = cyy * cp;
        const roX = cam.target[0] - fwdX * cam.dist;
        const roY = cam.target[1] - fwdY * cam.dist;
        const roZ = cam.target[2] - fwdZ * cam.dist;
        let rX, rY, rZ;
        if (Math.abs(fwdY) > 0.999) { rX = 0; rY = fwdZ; rZ = -fwdY; }
        else                        { rX = -fwdZ; rY = 0; rZ = fwdX; }
        const rL = Math.hypot(rX, rY, rZ) || 1; rX /= rL; rY /= rL; rZ /= rL;
        const uX = rY*fwdZ - rZ*fwdY, uY = rZ*fwdX - rX*fwdZ, uZ = rX*fwdY - rY*fwdX;
        let ox = roX, oy = roY, oz = roZ, dx, dy, dz;
        if (cam.ortho) {
          const oh = cam.dist * (cam.orthoScale || 0.45);
          ox += rX*(ndcX*asp)*oh + uX*ndcY*oh;
          oy += rY*(ndcX*asp)*oh + uY*ndcY*oh;
          oz += rZ*(ndcX*asp)*oh + uZ*ndcY*oh;
          const fL = Math.hypot(fwdX, fwdY, fwdZ);
          dx = fwdX/fL; dy = fwdY/fL; dz = fwdZ/fL;
        } else {
          const fl = Math.tan((cam.fov || 45) * Math.PI / 360);
          const vx = fwdX*fl + rX*(ndcX*asp) + uX*ndcY;
          const vy = fwdY*fl + rY*(ndcX*asp) + uY*ndcY;
          const vz = fwdZ*fl + rZ*(ndcX*asp) + uZ*ndcY;
          const L = Math.hypot(vx, vy, vz) || 1;
          dx = vx/L; dy = vy/L; dz = vz/L;
        }
        return { ox, oy, oz, dx, dy, dz };
      }

      // ─────────────────────────────────────────────────────────
      // __raycastSketchPlane(ndcX, ndcY) → { x, y, z, gx, gy, gz, freeX, freeY, freeZ } | null
      // Uses active workingPlane. Returns grid-snapped point + raw hit.
      // ─────────────────────────────────────────────────────────
      window.__raycastSketchPlane = function(ndcX, ndcY) {
        const r = __pickRay(ndcX, ndcY);
        const plane = sketchState.workingPlane || "XZ";
        let t = -1, axisDot = 0;
        if (plane === "XZ")      { axisDot = r.dy; t = -r.oy / r.dy; }
        else if (plane === "XY") { axisDot = r.dz; t = -r.oz / r.dz; }
        else if (plane === "YZ") { axisDot = r.dx; t = -r.ox / r.dx; }
        if (Math.abs(axisDot) < 1e-6 || t <= 0) return null;
        let hx = r.ox + r.dx * t;
        let hy = r.oy + r.dy * t;
        let hz = r.oz + r.dz * t;
        if (plane === "XZ") hy = 0;
        if (plane === "XY") hz = 0;
        if (plane === "YZ") hx = 0;
        const snapped = window.__snapWorldToGrid({ x: hx, y: hy, z: hz }, plane);
        return {
          x: snapped.x, y: snapped.y, z: snapped.z,
          gx: snapped.gx, gy: snapped.gy, gz: snapped.gz,
          freeX: hx, freeY: hy, freeZ: hz,
        };
      };

      // ─────────────────────────────────────────────────────────
      // Project (x,y,z) world → screen-pixel (uses canvas size).
      // Returns { x, y } in canvas pixels, or null if behind camera.
      // ─────────────────────────────────────────────────────────
      window.__worldToScreenPx = function(x, y, z) {
        const cp = Math.cos(cam.pitch), sp = Math.sin(cam.pitch);
        const cy = Math.cos(cam.yaw),   sy = Math.sin(cam.yaw);
        const fwdX = -sy * cp, fwdY = -sp, fwdZ = cy * cp;
        const roX = cam.target[0] - fwdX * cam.dist;
        const roY = cam.target[1] - fwdY * cam.dist;
        const roZ = cam.target[2] - fwdZ * cam.dist;
        let rX, rY, rZ;
        if (Math.abs(fwdY) > 0.999) { rX = 0; rY = fwdZ; rZ = -fwdY; }
        else                        { rX = -fwdZ; rY = 0; rZ = fwdX; }
        const rL = Math.hypot(rX, rY, rZ) || 1; rX /= rL; rY /= rL; rZ /= rL;
        const uX = rY*fwdZ - rZ*fwdY, uY = rZ*fwdX - rX*fwdZ, uZ = rX*fwdY - rY*fwdX;
        const dx = x - roX, dy = y - roY, dz = z - roZ;
        const vwX = dx*rX  + dy*rY  + dz*rZ;
        const vwY = dx*uX  + dy*uY  + dz*uZ;
        const vwZ = dx*fwdX + dy*fwdY + dz*fwdZ;
        const asp = canvas.width / canvas.height;
        let ndcX, ndcY;
        if (cam.ortho) {
          const oh = cam.dist * (cam.orthoScale || 0.45);
          ndcX = (vwX / oh) / asp;
          ndcY = (vwY / oh);
          if (vwZ < -50 || vwZ > 1000) return null;
        } else {
          if (vwZ < 0.05) return null;
          const fL = 1.0 / Math.tan((cam.fov || 45) * Math.PI / 360);
          ndcX = (vwX * fL) / vwZ / asp;
          ndcY = (vwY * fL) / vwZ;
        }
        return { x: (ndcX + 1) * 0.5 * canvas.width, y: (1 - ndcY) * 0.5 * canvas.height };
      };

      // ─────────────────────────────────────────────────────────
      // __pickPointAt(ndcX, ndcY) → pointId | null
      // Screen-space pixel distance. Radius ~ 14 px.
      // ─────────────────────────────────────────────────────────
      window.__pickPointAt = function(ndcX, ndcY) {
        const pts = sketchState.points;
        if (!pts.length) return null;
        // Convert ndc → screen-px for distance comparison.
        const mx = (ndcX + 1) * 0.5 * canvas.width;
        const my = (1 - ndcY) * 0.5 * canvas.height;
        const pickR = 14.0;
        let best = null, bestD = Infinity;
        for (const p of pts) {
          const s = window.__worldToScreenPx(p.x, p.y, p.z);
          if (!s) continue;
          const d = Math.hypot(s.x - mx, s.y - my);
          if (d < pickR && d < bestD) { bestD = d; best = p.id; }
        }
        return best;
      };

      // ─────────────────────────────────────────────────────────
      // __pickEdgeAt(ndcX, ndcY) → edgeId | null
      // Screen-space segment-point distance. Radius ~ 7 px.
      // ─────────────────────────────────────────────────────────
      window.__pickEdgeAt = function(ndcX, ndcY) {
        const edges = sketchState.edges;
        if (!edges.length) return null;
        const mx = (ndcX + 1) * 0.5 * canvas.width;
        const my = (1 - ndcY) * 0.5 * canvas.height;
        const pickR = 7.0;
        const byId = new Map(sketchState.points.map(p => [p.id, p]));
        let best = null, bestD = Infinity;
        for (const e of edges) {
          const a = byId.get(e.a), b = byId.get(e.b);
          if (!a || !b) continue;
          const sa = window.__worldToScreenPx(a.x, a.y, a.z);
          const sb = window.__worldToScreenPx(b.x, b.y, b.z);
          if (!sa || !sb) continue;
          const abx = sb.x - sa.x, aby = sb.y - sa.y;
          const ab2 = abx*abx + aby*aby;
          const t = ab2 > 1e-6 ? Math.max(0, Math.min(1, ((mx - sa.x) * abx + (my - sa.y) * aby) / ab2)) : 0;
          const cx = sa.x + abx * t, cy = sa.y + aby * t;
          const d = Math.hypot(cx - mx, cy - my);
          if (d < pickR && d < bestD) { bestD = d; best = e.id; }
        }
        return best;
      };
"##;
