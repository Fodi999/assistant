// ── Part 2: 2D overlay — w2s projection, grid, world axes, orbit pivot ring ──

pub const JS: &str = r##"
        const __pfOverlay = performance.now();
        const sk = document.getElementById('sketch-canvas');
        if (sk) {
          if (sk.width !== canvas.width || sk.height !== canvas.height) {
            sk.width = canvas.width; sk.height = canvas.height;
          }
          const ctx = sk.getContext('2d');
          ctx.clearRect(0, 0, sk.width, sk.height);

          const fL  = 1.0 / Math.tan((cam.fov || 45) * Math.PI / 360);
          const asp = sk.width / sk.height;
          function w2s(x, y, z) {
            const dx = x - roX, dy = y - roY, dz = z - roZ;
            const vwX = dx*rX  + dy*rY  + dz*rZ;
            const vwY = dx*uX  + dy*uY  + dz*uZ;
            const vwZ = dx*fwdX + dy*fwdY + dz*fwdZ;
            let ndcX, ndcY;
            if (cam.ortho) {
              const oh = cam.dist * (cam.orthoScale || 0.45);
              ndcX = (vwX / oh) / asp;
              ndcY = (vwY / oh);
              if (vwZ < -50 || vwZ > 1000) return null;
            } else {
              if (vwZ < 0.05) return null;
              ndcX = (vwX * fL) / vwZ / asp;
              ndcY = (vwY * fL) / vwZ;
            }
            return { x: (ndcX + 1) * 0.5 * sk.width, y: (1 - ndcY) * 0.5 * sk.height };
          }

          // ── Plane-aware grid ──
          const pr    = sketchState.precision;
          const g     = (pr && pr.displayGridStepM > 0)
                        ? pr.displayGridStepM
                        : (sketchState.gridSize || 1.0);
          const N     = Math.max(20, Math.min(200, Math.round(0.5 / g)));
          const plane = sketchState.workingPlane || 'XZ';
          function gridLineA(i) {
            if (plane === 'XZ') return { a: [i*g, 0, -N*g], b: [i*g, 0,  N*g] };
            if (plane === 'XY') return { a: [i*g, -N*g, 0], b: [i*g,  N*g, 0] };
            return { a: [0, i*g, -N*g], b: [0, i*g,  N*g] };
          }
          function gridLineB(i) {
            if (plane === 'XZ') return { a: [-N*g, 0, i*g], b: [ N*g, 0, i*g] };
            if (plane === 'XY') return { a: [-N*g, i*g, 0], b: [ N*g, i*g, 0] };
            return { a: [0, -N*g, i*g], b: [0,  N*g, i*g] };
          }
          ctx.lineWidth = 1;
          ctx.strokeStyle = 'rgba(148,163,184,0.18)';
          ctx.beginPath();
          for (let i = -N; i <= N; i++) {
            const la = gridLineA(i), lb = gridLineB(i);
            const a1 = w2s(la.a[0], la.a[1], la.a[2]), a2 = w2s(la.b[0], la.b[1], la.b[2]);
            if (a1 && a2) { ctx.moveTo(a1.x, a1.y); ctx.lineTo(a2.x, a2.y); }
            const b1 = w2s(lb.a[0], lb.a[1], lb.a[2]), b2 = w2s(lb.b[0], lb.b[1], lb.b[2]);
            if (b1 && b2) { ctx.moveTo(b1.x, b1.y); ctx.lineTo(b2.x, b2.y); }
          }
          ctx.stroke();

          // ── World axes ──
          const origin = w2s(0, 0, 0);
          if (origin) {
            const axes = [
              { p: [N*g, 0, 0], c: '#ef4444' },
              { p: [0, N*g, 0], c: '#22c55e' },
              { p: [0, 0, N*g], c: '#3b82f6' },
            ];
            for (const ax of axes) {
              const tip = w2s(ax.p[0], ax.p[1], ax.p[2]);
              if (!tip) continue;
              ctx.strokeStyle = ax.c;
              ctx.lineWidth = 1.8;
              ctx.beginPath();
              ctx.moveTo(origin.x, origin.y);
              ctx.lineTo(tip.x, tip.y);
              ctx.stroke();
            }
          }

          // ── Orbit pivot ring ──
          if (window.__showOrbitGuide && window.__orbitActive) {
            const helperAlpha = window.__fadeBackgroundHelpers ? 0.12 : 0.30;
            const orbitR = Math.max(0.05, cam.dist * 0.18);
            const SEG = 64;
            const orbPl = sketchState.workingPlane || 'XZ';
            ctx.save();
            ctx.setLineDash([6, 5]);
            ctx.lineWidth = 1.2;
            ctx.strokeStyle = 'rgba(167,139,250,' + helperAlpha + ')';
            ctx.globalAlpha = 1;
            ctx.beginPath();
            let firstValid = true;
            for (let i = 0; i <= SEG; i++) {
              const a = (i / SEG) * Math.PI * 2;
              let wx, wy, wz;
              if (orbPl === 'XZ') {
                wx = cam.target[0] + Math.cos(a) * orbitR;
                wy = cam.target[1];
                wz = cam.target[2] + Math.sin(a) * orbitR;
              } else if (orbPl === 'XY') {
                wx = cam.target[0] + Math.cos(a) * orbitR;
                wy = cam.target[1] + Math.sin(a) * orbitR;
                wz = cam.target[2];
              } else {
                wx = cam.target[0];
                wy = cam.target[1] + Math.cos(a) * orbitR;
                wz = cam.target[2] + Math.sin(a) * orbitR;
              }
              const sp = w2s(wx, wy, wz);
              if (!sp) { firstValid = true; continue; }
              if (firstValid) { ctx.moveTo(sp.x, sp.y); firstValid = false; }
              else ctx.lineTo(sp.x, sp.y);
            }
            ctx.closePath();
            ctx.stroke();
            const pivotS = w2s(cam.target[0], cam.target[1], cam.target[2]);
            if (pivotS) {
              ctx.setLineDash([]);
              ctx.beginPath();
              ctx.arc(pivotS.x, pivotS.y, 3.5, 0, Math.PI * 2);
              ctx.strokeStyle = 'rgba(167,139,250,' + (helperAlpha * 2.5) + ')';
              ctx.lineWidth = 1.2;
              ctx.stroke();
            }
            ctx.restore();
          }
"##;
