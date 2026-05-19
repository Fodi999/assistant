// ── JS: Render loop — UBO upload, draw passes, sketch overlay ────────────────
// Domain: Rendering — per-frame camera math, GPU pass, plane-aware 2D overlay.

pub const JS: &str = r##"
      const t0 = performance.now();
      let frameCount = 0;
      let lastFpsTime = t0, fpsAcc = 0, fps = 0;
      let lastFrameTime = t0;
      const ubo = new Float32Array(64);

      function frame() {
        const now = performance.now();
        const t   = (now - t0) / 1000.0;
        const dt  = (now - lastFrameTime) / 1000.0;
        const cpuFrameMs = now - lastFrameTime;
        lastFrameTime = now;
        if (frameCount === 0) {
          log('sketch core ready');
          setTimeout(() => { const d = document.getElementById('gpu-diag'); if (d) d.style.display = 'none'; }, 4000);
        }
        frameCount++;
        fpsAcc++;
        if (now - lastFpsTime >= 500) {
          fps = fpsAcc * 1000 / (now - lastFpsTime);
          lastFpsTime = now; fpsAcc = 0;
          if (globalThis.__matterPerf) {
            globalThis.__matterPerf.fps     = fps;
            globalThis.__matterPerf.frameMs = cpuFrameMs;
          }
        }
        // Perf HUD: per-frame sample (skip 1st frame — bogus delta).
        if (frameCount > 1 && window.__perfSample) {
          window.__perfSample('frame', cpuFrameMs);
        }
        if (window.perfState) window.perfState.fps = fps;
        if (cam.autoRotate) cam.yaw += dt * 0.25;

        const cy = Math.cos(cam.yaw),   sy = Math.sin(cam.yaw);
        const cp = Math.cos(cam.pitch), sp = Math.sin(cam.pitch);
        const fwdX = -sy * cp, fwdY = -sp, fwdZ = cy * cp;
        const roX = cam.target[0] - fwdX * cam.dist;
        const roY = cam.target[1] - fwdY * cam.dist;
        const roZ = cam.target[2] - fwdZ * cam.dist;
        let rX, rY, rZ;
        if (Math.abs(fwdY) > 0.999) { rX = 0; rY = fwdZ; rZ = -fwdY; }
        else                        { rX = -fwdZ; rY = 0; rZ = fwdX; }
        const rL = Math.hypot(rX, rY, rZ) || 1; rX /= rL; rY /= rL; rZ /= rL;
        const uX = rY*fwdZ - rZ*fwdY;
        const uY = rZ*fwdX - rX*fwdZ;
        const uZ = rX*fwdY - rY*fwdX;

        ubo[ 0] = t;
        ubo[ 1] = canvas.width;
        ubo[ 2] = canvas.height;
        ubo[ 3] = 0.0;
        ubo[ 4] = roX; ubo[ 5] = roY; ubo[ 6] = roZ; ubo[ 7] = 0;
        ubo[ 8] = rX;  ubo[ 9] = rY;  ubo[10] = rZ;  ubo[11] = 0;
        ubo[12] = uX;  ubo[13] = uY;  ubo[14] = uZ;  ubo[15] = 0;
        ubo[16] = fwdX; ubo[17] = fwdY; ubo[18] = fwdZ; ubo[19] = 0;
        ubo[20] = mouse.ndcX; ubo[21] = mouse.ndcY;
        ubo[22] = mouse.active ? 1.0 : 0.0;
        ubo[23] = shapeExponent(0.0);
        ubo[24] = 0; ubo[25] = 0; ubo[26] = 1; ubo[27] = 1.0;
        ubo[28] = 0; ubo[29] = 0; ubo[30] = 0; ubo[31] = 0;
        ubo[32] = sceneState.objectPosition[0];
        ubo[33] = sceneState.objectPosition[1];
        ubo[34] = sceneState.objectPosition[2];
        ubo[35] = 0;
        ubo[36] = floorGrid.scale;
        ubo[37] = cam.ortho ? 1.0 : 0.0;
        ubo[38] = 0; ubo[39] = 0;
        ubo[40] = sceneState.objectRotation[0];
        ubo[41] = sceneState.objectRotation[1];
        ubo[42] = sceneState.objectRotation[2]; ubo[43] = 0;
        ubo[44] = sceneState.objectScale[0];
        ubo[45] = sceneState.objectScale[1];
        ubo[46] = sceneState.objectScale[2]; ubo[47] = 0;
        ubo[48] = sceneState.baseMeshDim[0];
        ubo[49] = sceneState.baseMeshDim[1];
        ubo[50] = sceneState.baseMeshDim[2]; ubo[51] = 0;
        ubo[52] = sceneState.objectBevel;
        ubo[53] = sceneState.objectProfile;
        ubo[54] = sceneState.objectRoundness; ubo[55] = 0;
        ubo[56] = sketchState.showGrid ? 1.0 : 0.0;
        ubo[57] = 0; ubo[58] = 0; ubo[59] = 0;
        ubo[60] = 0; ubo[61] = 0; ubo[62] = 0; ubo[63] = 0;
        device.queue.writeBuffer(uniformBuf, 0, ubo);

        // Perf: WebGPU render block.
        const __pfRender = performance.now();
        const enc  = device.createCommandEncoder();
        ensureDepth();
        const view      = gpuCtx.getCurrentTexture().createView();
        const depthView = depthTex.createView();
        {
          const pass = enc.beginRenderPass({
            colorAttachments: [{ view, clearValue: { r: 0.04, g: 0.06, b: 0.10, a: 1 }, loadOp: 'clear', storeOp: 'store' }],
            depthStencilAttachment: { view: depthView, depthClearValue: 1.0, depthLoadOp: 'clear', depthStoreOp: 'store' },
          });
          pass.setPipeline(bgPipeline);
          pass.setBindGroup(0, bindGroup);
          pass.draw(3);
          pass.end();
        }
        device.queue.submit([enc.finish()]);
        if (window.__perfSample) window.__perfSample('render', performance.now() - __pfRender);

        // Perf: 2D overlay block.
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
          // Use the *display* grid step (separate from the engine's internal
          // 0.01 mm precision step). Falls back to gridSize for legacy.
          const pr    = sketchState.precision;
          const g     = (pr && pr.displayGridStepM > 0)
                        ? pr.displayGridStepM
                        : (sketchState.gridSize || 1.0);
          // Adaptive cell count: keep visible extent ≈ 0.5 m for small steps,
          // 20 cells minimum, 200 maximum (legacy big-step sketches stay calm).
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

          // ── 3D Helper: Orbit pivot ring ──────────────────────────────────────
          // Drawn AFTER grid/axes but BEFORE sketch geometry (background layer).
          // Visible only when actively orbiting + flag enabled.
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
              } else { // YZ
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
            // Small pivot dot at camera target
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

          const pById = new Map(sketchState.points.map(p => [p.id, p]));
          const eById = new Map(sketchState.edges.map(e  => [e.id, e]));

          // ── Closed profile fills (under edges) ──
          if (sketchState.profiles && sketchState.profiles.length) {
            for (const prof of sketchState.profiles) {
              const ringPts = prof.pointIds.map(id => pById.get(id)).filter(Boolean);
              if (ringPts.length < 3) continue;
              const screenPts = ringPts.map(p => w2s(p.x, p.y, p.z));
              if (screenPts.some(s => !s)) continue;
              const isSelected = sketchState.selectedProfileId === prof.id;
              const isHover    = !isSelected && sketchState.hoverProfileId === prof.id;
              const isFullySelected = prof.edgeIds.every(eid => sketchState.selectedEdgeIds.has(eid));
              ctx.save();
              ctx.beginPath();
              ctx.moveTo(screenPts[0].x, screenPts[0].y);
              for (let i = 1; i < screenPts.length; i++) ctx.lineTo(screenPts[i].x, screenPts[i].y);
              ctx.closePath();
              if (isSelected) {
                ctx.fillStyle   = 'rgba(251,146,60,0.22)';
                ctx.strokeStyle = 'rgba(251,146,60,0.90)';
                ctx.lineWidth   = 2.0;
              } else if (isHover) {
                ctx.fillStyle   = 'rgba(56,189,248,0.20)';
                ctx.strokeStyle = 'rgba(56,189,248,0.75)';
                ctx.lineWidth   = 1.6;
              } else if (isFullySelected) {
                ctx.fillStyle   = 'rgba(251,146,60,0.18)';
                ctx.strokeStyle = 'rgba(251,146,60,0.55)';
                ctx.lineWidth   = 1.2;
              } else {
                ctx.fillStyle   = 'rgba(56,189,248,0.06)';
                ctx.strokeStyle = 'rgba(56,189,248,0.25)';
                ctx.lineWidth   = 1.0;
              }
              ctx.fill();
              ctx.stroke();
              ctx.restore();
            }
          }

          // ── Wall Surfaces (Edge Extrude) — drawn BEFORE edges ──
          // Transparent quad fill + wire edges, Blender-style overlay.
          if (sketchState.wallSurfaces && sketchState.wallSurfaces.length) {
            for (const wall of sketchState.wallSurfaces) {
              const sBA = w2s(wall.bottomA.x, wall.bottomA.y, wall.bottomA.z);
              const sBB = w2s(wall.bottomB.x, wall.bottomB.y, wall.bottomB.z);
              const sTA = w2s(wall.topA.x,    wall.topA.y,    wall.topA.z);
              const sTB = w2s(wall.topB.x,    wall.topB.y,    wall.topB.z);
              if (!sBA || !sBB || !sTA || !sTB) continue;
              ctx.save();
              // Transparent face fill
              ctx.beginPath();
              ctx.moveTo(sBA.x, sBA.y);
              ctx.lineTo(sBB.x, sBB.y);
              ctx.lineTo(sTB.x, sTB.y);
              ctx.lineTo(sTA.x, sTA.y);
              ctx.closePath();
              ctx.fillStyle = 'rgba(255,170,40,0.10)';
              ctx.fill();
              // Wall wire edges
              ctx.strokeStyle = 'rgba(255,180,40,0.75)';
              ctx.lineWidth   = 1.8;
              ctx.setLineDash([]);
              // top edge
              ctx.beginPath(); ctx.moveTo(sTA.x, sTA.y); ctx.lineTo(sTB.x, sTB.y); ctx.stroke();
              // vertical A
              ctx.beginPath(); ctx.moveTo(sBA.x, sBA.y); ctx.lineTo(sTA.x, sTA.y); ctx.stroke();
              // vertical B
              ctx.beginPath(); ctx.moveTo(sBB.x, sBB.y); ctx.lineTo(sTB.x, sTB.y); ctx.stroke();
              ctx.restore();
            }
          }

          // ── Extrude Live Preview ─────────────────────────────────────────────
          // While extrude is active (before commit) draw the wall quad in real-time.
          // This makes the surface follow the gizmo arrow immediately during drag.
          if (sketchState.extrude && sketchState.extrude.active
              && sketchState.extrude.edgeIds && sketchState.extrude.edgeIds.length) {
            const ex      = sketchState.extrude;
            const inp     = document.getElementById('__extrude-modal-input');
            const heightMm = parseFloat(inp ? inp.value : (ex.heightInput || '0')) || 0;
            const heightM  = heightMm / 1000;
            const plane    = sketchState.workingPlane || 'XZ';
            const dir      = window.__getExtrudeDir ? window.__getExtrudeDir(plane) : { x:0, y:1, z:0 };

            if (heightM > 0.0001) {
              for (const edgeId of ex.edgeIds) {
                const edge = sketchState.edges.find(e => e.id === edgeId);
                if (!edge) continue;
                const pA = pById.get(edge.a), pB = pById.get(edge.b);
                if (!pA || !pB) continue;

                const sBA = w2s(pA.x, pA.y, pA.z);
                const sBB = w2s(pB.x, pB.y, pB.z);
                const sTA = w2s(pA.x + dir.x * heightM, pA.y + dir.y * heightM, pA.z + dir.z * heightM);
                const sTB = w2s(pB.x + dir.x * heightM, pB.y + dir.y * heightM, pB.z + dir.z * heightM);
                if (!sBA || !sBB || !sTA || !sTB) continue;

                ctx.save();
                // Filled face — brighter during drag
                const dragging = window.__extrudeGizmoDrag;
                ctx.beginPath();
                ctx.moveTo(sBA.x, sBA.y);
                ctx.lineTo(sBB.x, sBB.y);
                ctx.lineTo(sTB.x, sTB.y);
                ctx.lineTo(sTA.x, sTA.y);
                ctx.closePath();
                ctx.fillStyle = dragging ? 'rgba(255,200,60,0.22)' : 'rgba(255,170,40,0.14)';
                ctx.fill();

                // Wire edges — dashed to show it's preview
                ctx.strokeStyle = dragging ? 'rgba(255,220,80,0.95)' : 'rgba(255,180,40,0.85)';
                ctx.lineWidth   = 2;
                ctx.setLineDash([6, 3]);
                // top edge
                ctx.beginPath(); ctx.moveTo(sTA.x, sTA.y); ctx.lineTo(sTB.x, sTB.y); ctx.stroke();
                // verticals
                ctx.beginPath(); ctx.moveTo(sBA.x, sBA.y); ctx.lineTo(sTA.x, sTA.y); ctx.stroke();
                ctx.beginPath(); ctx.moveTo(sBB.x, sBB.y); ctx.lineTo(sTB.x, sTB.y); ctx.stroke();
                ctx.setLineDash([]);
                ctx.restore();
              }
            }
          }

          // ── Edges ──
          const grabPointSet = sketchState.grab.active
            ? new Set(sketchState.grab.pointIds)
            : null;
          for (const e of sketchState.edges) {
            const a = pById.get(e.a), b = pById.get(e.b);
            if (!a || !b) continue;
            const sa = w2s(a.x, a.y, a.z), sb = w2s(b.x, b.y, b.z);
            if (!sa || !sb) continue;
            const isHover = sketchState.hoverEdgeId === e.id;
            const isSel   = sketchState.selectedEdgeIds.has(e.id);
            const isGrab  = grabPointSet && grabPointSet.has(e.a) && grabPointSet.has(e.b);
            const edgeCol = isGrab ? '#facc15' : isSel ? '#fb923c' : isHover ? '#facc15' : null;
            const kind    = e.kind || 'normal';
            ctx.save();
            if (kind === 'construction') {
              ctx.strokeStyle = edgeCol || '#67e8f9';
              ctx.lineWidth   = (isSel || isGrab) ? 1.6 : 1.2;
              ctx.setLineDash([4, 3]);
            } else if (kind === 'hidden') {
              ctx.strokeStyle = edgeCol || '#94a3b8';
              ctx.lineWidth   = (isSel || isGrab) ? 1.8 : 1.4;
              ctx.setLineDash([6, 4]);
            } else {
              ctx.strokeStyle = edgeCol || '#cbd5e1';
              ctx.lineWidth   = (isSel || isGrab) ? 3.0 : isHover ? 2.5 : 2.0;
            }
            ctx.beginPath();
            ctx.moveTo(sa.x, sa.y);
            ctx.lineTo(sb.x, sb.y);
            ctx.stroke();
            ctx.restore();

            // ── Constraint badges ──────────────────────────────────────────
            // Collect all constraint icons for this edge, then render them
            // stacked perpendicular to the edge (Fusion/SolidWorks style).
            {
              // All constraints whose targetId === e.id
              const eCons = sketchState.constraints.filter(c => c.targetId === e.id);

              // Map type → icon glyph + color (keys uppercase, lookup normalised)
              const iconMap = {
                HORIZONTAL:   { glyph: 'H', color: '#a78bfa' },
                VERTICAL:     { glyph: 'V', color: '#a78bfa' },
                FIXED_LENGTH: { glyph: 'L', color: '#34d399' },
                EQUAL:        { glyph: '=', color: '#fbbf24' },
                EQUAL_LENGTH: { glyph: '=', color: '#fbbf24' },
                PERPENDICULAR:{ glyph: '⊥', color: '#f472b6' },
                PARALLEL:     { glyph: '∥', color: '#f472b6' },
                MIDPOINT:     { glyph: '◇', color: '#38bdf8' },
                COINCIDENT:   { glyph: '●', color: '#fb923c' },
                EDGE_LENGTH:  { glyph: 'L', color: '#34d399' },
              };

              if (eCons.length > 0) {
                const mx = (sa.x + sb.x) * 0.5;
                const my = (sa.y + sb.y) * 0.5;
                // Edge direction vector in screen space
                const dx = sb.x - sa.x, dy = sb.y - sa.y;
                const len = Math.sqrt(dx*dx + dy*dy) || 1;
                // Perpendicular offset direction (always toward top-left of edge)
                let nx = -dy / len, ny = dx / len;
                // Flip so icons go above the edge (negative screen-Y = up)
                if (ny > 0) { nx = -nx; ny = -ny; }

                const BADGE_W = 16, BADGE_H = 16, GAP = 4;
                const totalW  = eCons.length * BADGE_W + (eCons.length - 1) * GAP;
                // Start X at edge midpoint offset perpendicular
                const PERP_DIST = 18;
                const bx0 = mx + nx * PERP_DIST - totalW * 0.5;
                const by0 = my + ny * PERP_DIST - BADGE_H * 0.5;

                ctx.save();
                ctx.font = 'bold 10px "JetBrains Mono", system-ui, monospace';
                ctx.textAlign = 'center';
                ctx.textBaseline = 'middle';

                eCons.forEach((c, i) => {
                  const info = iconMap[(c.type || '').toUpperCase()] || { glyph: '?', color: '#64748b' };
                  const bx = bx0 + i * (BADGE_W + GAP);
                  const by = by0;

                  // Pill background
                  const radius = 4;
                  ctx.beginPath();
                  ctx.roundRect
                    ? ctx.roundRect(bx, by, BADGE_W, BADGE_H, radius)
                    : (ctx.rect(bx, by, BADGE_W, BADGE_H));
                  ctx.fillStyle = 'rgba(15,23,42,0.88)';
                  ctx.fill();
                  ctx.strokeStyle = info.color;
                  ctx.lineWidth = 1.2;
                  ctx.stroke();

                  // Glyph
                  ctx.fillStyle = info.color;
                  ctx.fillText(info.glyph, bx + BADGE_W * 0.5, by + BADGE_H * 0.5);
                });

                ctx.restore();
              }

              // ── CAD Dimension line (FIXED_LENGTH constraint) ─────────────
              // Full SolidWorks-style: extension lines + arrowheads + value
              const dimC = window.__getEdgeLengthConstraint && window.__getEdgeLengthConstraint(e.id);
              if (dimC && dimC.value != null) {
                const edx = sb.x - sa.x, edy = sb.y - sa.y;
                const edgeScr = Math.sqrt(edx*edx + edy*edy) || 1;
                // Unit edge direction (screen)
                const ux = edx / edgeScr, uy = edy / edgeScr;
                // Perpendicular unit vector — always toward the "up" side
                let nx = -uy, ny = ux;
                if (ny > 0) { nx = -nx; ny = -ny; }

                const OFFSET = 32;  // pixels: perpendicular offset of dim line
                const EXT_OVR = 5;  // extra overshoot of extension lines
                const ARROW  = 7;   // arrowhead length
                const ARROW_W = 3;  // arrowhead half-width

                // Dim line endpoints (offset from edge midpoints — actually from A/B)
                const d1x = sa.x + nx * OFFSET, d1y = sa.y + ny * OFFSET;
                const d2x = sb.x + nx * OFFSET, d2y = sb.y + ny * OFFSET;

                ctx.save();
                ctx.strokeStyle = '#34d399';
                ctx.fillStyle   = '#34d399';
                ctx.lineWidth   = 1.3;
                ctx.setLineDash([]);

                // Extension line A: from edge point toward dim line (with overshoot)
                ctx.beginPath();
                ctx.moveTo(sa.x + nx * 5, sa.y + ny * 5);
                ctx.lineTo(d1x + nx * EXT_OVR, d1y + ny * EXT_OVR);
                ctx.stroke();

                // Extension line B
                ctx.beginPath();
                ctx.moveTo(sb.x + nx * 5, sb.y + ny * 5);
                ctx.lineTo(d2x + nx * EXT_OVR, d2y + ny * EXT_OVR);
                ctx.stroke();

                // Dimension line (between A and B at offset)
                if (edgeScr >= ARROW * 3) {
                  // Normal: line with inside arrows
                  ctx.beginPath();
                  ctx.moveTo(d1x, d1y);
                  ctx.lineTo(d2x, d2y);
                  ctx.stroke();
                  // Arrow at d1 pointing toward d2
                  ctx.beginPath();
                  ctx.moveTo(d1x, d1y);
                  ctx.lineTo(d1x + ux*ARROW + ny*ARROW_W, d1y + uy*ARROW - nx*ARROW_W);
                  ctx.lineTo(d1x + ux*ARROW - ny*ARROW_W, d1y + uy*ARROW + nx*ARROW_W);
                  ctx.closePath(); ctx.fill();
                  // Arrow at d2 pointing toward d1
                  ctx.beginPath();
                  ctx.moveTo(d2x, d2y);
                  ctx.lineTo(d2x - ux*ARROW + ny*ARROW_W, d2y - uy*ARROW - nx*ARROW_W);
                  ctx.lineTo(d2x - ux*ARROW - ny*ARROW_W, d2y - uy*ARROW + nx*ARROW_W);
                  ctx.closePath(); ctx.fill();
                } else {
                  // Too short: outside arrows
                  ctx.beginPath();
                  ctx.moveTo(d1x - ux*20, d1y - uy*20);
                  ctx.lineTo(d1x, d1y);
                  ctx.stroke();
                  ctx.beginPath();
                  ctx.moveTo(d2x + ux*20, d2y + uy*20);
                  ctx.lineTo(d2x, d2y);
                  ctx.stroke();
                  ctx.beginPath();
                  ctx.moveTo(d1x, d1y);
                  ctx.lineTo(d1x - ux*ARROW + ny*ARROW_W, d1y - uy*ARROW - nx*ARROW_W);
                  ctx.lineTo(d1x - ux*ARROW - ny*ARROW_W, d1y - uy*ARROW + nx*ARROW_W);
                  ctx.closePath(); ctx.fill();
                  ctx.beginPath();
                  ctx.moveTo(d2x, d2y);
                  ctx.lineTo(d2x + ux*ARROW + ny*ARROW_W, d2y + uy*ARROW - nx*ARROW_W);
                  ctx.lineTo(d2x + ux*ARROW - ny*ARROW_W, d2y + uy*ARROW + nx*ARROW_W);
                  ctx.closePath(); ctx.fill();
                }

                // Dimension text (value in mm, at dim line midpoint)
                const dmx = (d1x + d2x) * 0.5, dmy = (d1y + d2y) * 0.5;
                const txt = Number(dimC.value).toFixed(1) + ' мм';
                ctx.font = 'bold 11px "JetBrains Mono", system-ui, monospace';
                const tw = ctx.measureText(txt).width + 8;
                ctx.fillStyle = 'rgba(10,20,35,0.93)';
                // Pill background
                const ph = 16, pr = 4;
                ctx.beginPath();
                ctx.roundRect
                  ? ctx.roundRect(dmx - tw/2, dmy - ph/2, tw, ph, pr)
                  : ctx.rect(dmx - tw/2, dmy - ph/2, tw, ph);
                ctx.fill();
                ctx.strokeStyle = '#34d399';
                ctx.lineWidth = 1;
                ctx.stroke();
                ctx.fillStyle = '#34d399';
                ctx.textAlign = 'center'; ctx.textBaseline = 'middle';
                ctx.fillText(txt, dmx, dmy);

                ctx.restore();
              }
            }
          }

          // ── Line tool: preview line + live length ──
          if (sketchState.activeTool === 'line' && sketchState.line.startPointId) {
            const anchor = pById.get(sketchState.line.startPointId);
            const target = sketchState.line.previewPoint;
            if (anchor && target) {
              const sa = w2s(anchor.x, anchor.y, anchor.z);
              const sb = w2s(target.x, target.y, target.z);
              if (sa && sb) {
                const valid = sketchState.line.previewValid;
                ctx.save();
                ctx.setLineDash([6, 4]);
                ctx.strokeStyle = valid ? 'rgba(56,189,248,0.85)' : 'rgba(239,68,68,0.85)';
                ctx.lineWidth = 2;
                ctx.beginPath();
                ctx.moveTo(sa.x, sa.y); ctx.lineTo(sb.x, sb.y);
                ctx.stroke();
                ctx.restore();
                const len = sketchState.line.previewLength || 0;
                const txt = window.__fmtLength ? window.__fmtLength(len) : (len * 1000).toFixed(1) + ' mm';
                const mx = (sa.x + sb.x) * 0.5;
                const my = (sa.y + sb.y) * 0.5;
                ctx.font = '12px "JetBrains Mono", system-ui, monospace';
                const w = ctx.measureText(txt).width + 10;
                ctx.fillStyle = 'rgba(15,23,42,0.9)';
                ctx.fillRect(mx - w/2, my - 22, w, 18);
                ctx.fillStyle = valid ? '#38bdf8' : '#ef4444';
                ctx.textAlign = 'center'; ctx.textBaseline = 'middle';
                ctx.fillText(txt, mx, my - 13);
              }
            }
          }

          // ── Rect tool: preview rectangle ──
          if (sketchState.activeTool === 'rect' && sketchState.rect && sketchState.rect.active && sketchState.rect.startSnap && sketchState.hoverWorld) {
            const g1   = sketchState.rect.startSnap;
            const hw   = sketchState.hoverWorld;
            const gs   = sketchState.gridSize || 1.0;
            const plane = sketchState.workingPlane || 'XZ';
            // Build 4 world corners for preview
            let c0, c1, c2, c3;
            const g2gx = Math.round(hw.gx !== undefined ? hw.gx : hw.x / gs);
            const g2gy = Math.round(hw.gy !== undefined ? hw.gy : hw.y / gs);
            const g2gz = Math.round(hw.gz !== undefined ? hw.gz : hw.z / gs);
            if (plane === 'XZ') {
              c0 = { x: g1.gx*gs, y: 0, z: g1.gz*gs };
              c1 = { x: g2gx*gs, y: 0, z: g1.gz*gs };
              c2 = { x: g2gx*gs, y: 0, z: g2gz*gs };
              c3 = { x: g1.gx*gs, y: 0, z: g2gz*gs };
            } else if (plane === 'XY') {
              c0 = { x: g1.gx*gs, y: g1.gy*gs, z: 0 };
              c1 = { x: g2gx*gs, y: g1.gy*gs, z: 0 };
              c2 = { x: g2gx*gs, y: g2gy*gs, z: 0 };
              c3 = { x: g1.gx*gs, y: g2gy*gs, z: 0 };
            } else {
              c0 = { x: 0, y: g1.gy*gs, z: g1.gz*gs };
              c1 = { x: 0, y: g1.gy*gs, z: g2gz*gs };
              c2 = { x: 0, y: g2gy*gs, z: g2gz*gs };
              c3 = { x: 0, y: g2gy*gs, z: g1.gz*gs };
            }
            const sc0 = w2s(c0.x, c0.y, c0.z);
            const sc1 = w2s(c1.x, c1.y, c1.z);
            const sc2 = w2s(c2.x, c2.y, c2.z);
            const sc3 = w2s(c3.x, c3.y, c3.z);
            if (sc0 && sc1 && sc2 && sc3) {
              ctx.save();
              ctx.setLineDash([6, 4]);
              ctx.strokeStyle = 'rgba(56,189,248,0.85)';
              ctx.lineWidth = 2;
              ctx.beginPath();
              ctx.moveTo(sc0.x, sc0.y);
              ctx.lineTo(sc1.x, sc1.y);
              ctx.lineTo(sc2.x, sc2.y);
              ctx.lineTo(sc3.x, sc3.y);
              ctx.closePath();
              ctx.stroke();
              // Filled semi-transparent rect
              ctx.fillStyle = 'rgba(56,189,248,0.07)';
              ctx.fill();
              ctx.restore();
              // Anchor corner dot
              ctx.beginPath();
              ctx.arc(sc0.x, sc0.y, 5, 0, Math.PI * 2);
              ctx.fillStyle = '#10b981';
              ctx.fill();
            }
          }

          // ── Circle tool: preview circle ──
          if (sketchState.activeTool === 'circle' && sketchState.circle && sketchState.circle.active &&
              sketchState.circle.centerSnap && sketchState.hoverWorld) {
            const gc    = sketchState.circle.centerSnap;
            const hw    = sketchState.hoverWorld;
            const gs    = sketchState.gridSize || 1.0;
            const plane = sketchState.workingPlane || 'XZ';
            const g2gx  = Math.round(hw.gx !== undefined ? hw.gx : hw.x / gs);
            const g2gy  = Math.round(hw.gy !== undefined ? hw.gy : hw.y / gs);
            const g2gz  = Math.round(hw.gz !== undefined ? hw.gz : hw.z / gs);

            let radiusSq;
            if (plane === 'XZ')      radiusSq = (g2gx - gc.gx) ** 2 + (g2gz - gc.gz) ** 2;
            else if (plane === 'XY') radiusSq = (g2gx - gc.gx) ** 2 + (g2gy - gc.gy) ** 2;
            else                     radiusSq = (g2gy - gc.gy) ** 2 + (g2gz - gc.gz) ** 2;

            if (radiusSq >= 0.25) {
              const radius = Math.sqrt(radiusSq) * gs; // world-space radius

              // World-space centre
              let cx, cy, cz;
              if (plane === 'XZ')      { cx = gc.gx * gs; cy = 0;          cz = gc.gz * gs; }
              else if (plane === 'XY') { cx = gc.gx * gs; cy = gc.gy * gs; cz = 0; }
              else                     { cx = 0;          cy = gc.gy * gs; cz = gc.gz * gs; }

              // Project centre + two rim reference points to screen space
              const sc = w2s(cx, cy, cz);

              // To get screen radius we project a rim point and measure distance
              let rx, ry, rz;
              if (plane === 'XZ')      { rx = cx + radius; ry = 0;          rz = cz; }
              else if (plane === 'XY') { rx = cx + radius; ry = cy;         rz = 0; }
              else                     { rx = 0;           ry = cy + radius; rz = cz; }
              const sr = w2s(rx, ry, rz);

              if (sc && sr) {
                const scrRadius = Math.hypot(sr.x - sc.x, sr.y - sc.y);
                if (scrRadius > 1) {
                  ctx.save();
                  ctx.setLineDash([6, 4]);
                  ctx.strokeStyle = 'rgba(56,189,248,0.85)';
                  ctx.lineWidth   = 2;
                  ctx.beginPath();
                  ctx.arc(sc.x, sc.y, scrRadius, 0, Math.PI * 2);
                  ctx.stroke();
                  // Semi-transparent fill
                  ctx.fillStyle = 'rgba(56,189,248,0.06)';
                  ctx.fill();
                  ctx.restore();
                  // Centre dot
                  ctx.beginPath();
                  ctx.arc(sc.x, sc.y, 5, 0, Math.PI * 2);
                  ctx.fillStyle = '#10b981';
                  ctx.fill();
                  // Radius label
                  ctx.save();
                  ctx.font      = '11px monospace';
                  ctx.fillStyle = 'rgba(56,189,248,0.9)';
                  ctx.fillText('r≈' + (radius).toFixed(2), sc.x + 8, sc.y - 8);
                  ctx.restore();
                }
              }
            }
          }

          if (sketchState.showValidation) {
            const isoSet = new Set(sketchState.validation.isolatedIds);
            const oeSet  = new Set(sketchState.validation.openEndIds);
            for (const p of sketchState.points) {
              const s = w2s(p.x, p.y, p.z);
              if (!s) continue;
              if (isoSet.has(p.id)) {
                ctx.save();
                ctx.beginPath();
                ctx.arc(s.x, s.y, 12, 0, Math.PI * 2);
                ctx.strokeStyle = 'rgba(244,114,182,0.85)';
                ctx.setLineDash([3, 2]);
                ctx.lineWidth = 1.5;
                ctx.stroke();
                ctx.restore();
              } else if (oeSet.has(p.id)) {
                ctx.save();
                ctx.beginPath();
                ctx.arc(s.x, s.y, 10, 0, Math.PI * 2);
                ctx.strokeStyle = 'rgba(239,68,68,0.75)';
                ctx.lineWidth = 1.2;
                ctx.stroke();
                ctx.restore();
              }
            }
          }

          // ── Points ──
          for (const p of sketchState.points) {
            const s = w2s(p.x, p.y, p.z);
            if (!s) continue;
            const isHover  = sketchState.hoverPointId === p.id;
            const isSel    = sketchState.selectedPointIds.has(p.id);
            const isGrabPt = grabPointSet && grabPointSet.has(p.id);
            const isAnchor = sketchState.line.startPointId === p.id;
            const isFixed  = window.__isPointFixed && window.__isPointFixed(p.id);
            const r = (isSel || isGrabPt) ? 8 : isAnchor ? 7.5 : isHover ? 7 : 6;
            ctx.beginPath();
            ctx.arc(s.x, s.y, r, 0, Math.PI * 2);
            ctx.fillStyle = isGrabPt ? '#facc15' : isSel ? '#fb923c' : isHover ? '#facc15' : isAnchor ? '#10b981' : '#38bdf8';
            ctx.fill();
            ctx.strokeStyle = '#0f172a';
            ctx.lineWidth = 1.8;
            ctx.stroke();
            if (isFixed) {
              // Square lock marker.
              const d = r + 4;
              ctx.save();
              ctx.strokeStyle = '#fbbf24';
              ctx.lineWidth = 1.8;
              ctx.strokeRect(s.x - d, s.y - d, d * 2, d * 2);
              // Tiny dot above to mimic lock shackle.
              ctx.beginPath();
              ctx.arc(s.x, s.y - d - 4, 2.2, 0, Math.PI * 2);
              ctx.fillStyle = '#fbbf24';
              ctx.fill();
              ctx.restore();
            }
          }

          // ═══════════════════════════════════════════════════════════
          // PRECISION CAD CURSOR v1
          // ─ Crosshair with center gap
          // ─ Snap marker: grid=cross · point=square · free=nothing
          // ─ Detached auto-flip tooltip (34–48 px offset)
          // ─ Alt = precision mode (bigger markers + larger offset)
          // ═══════════════════════════════════════════════════════════
          if (sketchState.hoverWorld &&
              (sketchState.activeTool === 'point' || sketchState.activeTool === 'line' || sketchState.activeTool === 'rect' || sketchState.activeTool === 'circle')) {
            const hw     = sketchState.hoverWorld;
            const prec   = !!sketchState.precisionMode;
            const cs     = sketchState.cursorSettings || {};
            const kind   = sketchState.snap.kind;            // 'grid' | 'point' | 'free' | 'none'
            const snapPt = kind === 'point';
            const snapFr = kind === 'free';

            // ── Screen position ──────────────────────────────────
            let cx, cy;
            if (hw.screenX !== undefined) { cx = hw.screenX; cy = hw.screenY; }
            else {
              const _c = w2s(hw.x, hw.y, hw.z);
              if (!_c) { cx = null; } else { cx = _c.x; cy = _c.y; }
            }
            if (cx === null || cx === undefined) { /* skip */ } else {

            // ── Constants (normal / precision) ───────────────────
            const _dpr       = window.devicePixelRatio || 1;
            const CROSS_ARM   = prec ? 14 : 10;  // half-length of crosshair arm (device px)
            const CROSS_GAP   = prec ?  4 :  3;  // center dead-zone radius
            const MARKER_SZ   = prec ?  4 :  3;  // snap marker half-size
            const LBL_OX      = prec ? 48 : 34;  // tooltip X offset
            const LBL_OY      = prec ? 32 : 26;  // tooltip Y offset
            const snapGrid = kind === 'grid';

            // ── Determine if cursor is truly near a grid intersection ──────
            // snap.kind='grid' always — it just means "nearest grid node snapped".
            // We check the screen-distance between the actual mouse pixel and the
            // projected snapped world position. If < SNAP_HIT_PX the cursor is
            // visually ON the intersection → highlight.
            const SNAP_HIT_PX = 10;  // device pixels threshold
            let onGridIntersection = false;
            if (snapGrid) {
              const _gs = w2s(hw.x, hw.y, hw.z);
              if (_gs) {
                const _sd = Math.hypot(cx - _gs.x, cy - _gs.y);
                onGridIntersection = _sd < SNAP_HIT_PX;
              }
            }

            // Colors: on grid intersection = bright yellow glow, point = green, free/roaming = cyan
            const CROSS_COLOR = snapPt            ? 'rgba(16,185,129,0.95)'
                              : onGridIntersection ? 'rgba(250,255,80,1.00)'
                              : snapFr            ? 'rgba(203,213,225,0.50)'
                              :                     'rgba(103,232,249,0.70)';   // roaming between grid lines

            ctx.save();

            // ── 1. Crosshair — тонкий плюс с дыркой в центре ────
            // Canvas buffer = device pixels (no ctx.scale), lineWidth 1 = 1 device px = thin on Retina
            ctx.strokeStyle = CROSS_COLOR;
            ctx.lineWidth   = onGridIntersection ? 2 : 1;   // чуть толще при попадании в перекрёсток

            // Grid intersection → glow через shadowBlur
            if (onGridIntersection) {
              ctx.shadowColor = 'rgba(250,255,80,0.75)';
              ctx.shadowBlur  = 6 * _dpr;
            }

            ctx.beginPath();
            ctx.moveTo(cx - CROSS_ARM, cy); ctx.lineTo(cx - CROSS_GAP, cy);
            ctx.moveTo(cx + CROSS_GAP, cy); ctx.lineTo(cx + CROSS_ARM, cy);
            ctx.moveTo(cx, cy - CROSS_ARM); ctx.lineTo(cx, cy - CROSS_GAP);
            ctx.moveTo(cx, cy + CROSS_GAP); ctx.lineTo(cx, cy + CROSS_ARM);
            ctx.stroke();

            // Сбрасываем glow чтобы не влиял на остальные элементы
            ctx.shadowColor = 'transparent';
            ctx.shadowBlur  = 0;

            // ── 2. Snap marker ───────────────────────────────────
            if (cs.showSnapMarker !== false) {
              const sm = MARKER_SZ;
              ctx.strokeStyle = CROSS_COLOR;
              ctx.lineWidth   = 1;
              if (snapPt) {
                // Endpoint → green square
                ctx.strokeRect(cx - sm, cy - sm, sm * 2, sm * 2);
              } else if (onGridIntersection) {
                // Grid intersection → маленький заполненный квадрат (dot) в центре
                ctx.fillStyle = CROSS_COLOR;
                ctx.fillRect(cx - 2, cy - 2, 4, 4);
              }
              // free → no extra marker (crosshair alone is enough)
            }

            // ── 3. Precision mode: extra outer ring ─────────────
            if (prec) {
              ctx.beginPath();
              ctx.arc(cx, cy, CROSS_ARM + 4, 0, Math.PI * 2);
              ctx.strokeStyle = onGridIntersection ? 'rgba(250,255,80,0.30)' : 'rgba(103,232,249,0.25)';
              ctx.lineWidth = 1;
              ctx.stroke();
            }

            // ── 4. Snap kind badge (small, near cursor, auto-flip) ──────
            // Only shows snap type: GRID / POINT / MID / ORTHO X|Z / FREE
            // Full coordinates stay in the HUD — cursor stays clean.
            if (cs.showCoords !== false) {
              // determine badge text
              let badge;
              const previewPt = sketchState.line && sketchState.line.previewPoint;
              if (sketchState.orthoLock && previewPt && previewPt._orthoAxis) {
                badge = previewPt._orthoAxis;          // e.g. "ORTHO X" / "ORTHO Z"
              } else if (kind === 'point')  { badge = 'POINT'; }
              else if (kind === 'grid')     { badge = 'GRID';  }
              else if (kind === 'free')     { badge = 'FREE';  }
              else                          { badge = null; }

              if (badge) {
                ctx.font = (prec ? '10px' : '9.5px') + ' "JetBrains Mono", system-ui, monospace';
                const tw   = ctx.measureText(badge).width;
                const boxW = tw + 10;
                const boxH = 16;
                const cw = ctx.canvas.width, ch = ctx.canvas.height;
                let lx = cx + LBL_OX;
                let ly = cy + LBL_OY;
                if (lx + boxW > cw - 12) lx = cx - LBL_OX - boxW;
                if (ly + boxH > ch - 12) ly = cy - LBL_OY - boxH;

                ctx.fillStyle   = 'rgba(10,14,26,0.88)';
                ctx.strokeStyle = snapPt          ? 'rgba(16,185,129,0.50)'
                                : sketchState.orthoLock ? 'rgba(251,191,36,0.55)'
                                :                  'rgba(56,189,248,0.35)';
                ctx.lineWidth = 1;
                const rr = 3;
                ctx.beginPath();
                ctx.moveTo(lx + rr, ly);
                ctx.lineTo(lx + boxW - rr, ly);        ctx.arcTo(lx + boxW, ly,          lx + boxW, ly + rr,          rr);
                ctx.lineTo(lx + boxW, ly + boxH - rr); ctx.arcTo(lx + boxW, ly + boxH,  lx + boxW - rr, ly + boxH,  rr);
                ctx.lineTo(lx + rr, ly + boxH);        ctx.arcTo(lx,        ly + boxH,  lx,          ly + boxH - rr, rr);
                ctx.lineTo(lx, ly + rr);               ctx.arcTo(lx,        ly,          lx + rr,    ly,             rr);
                ctx.closePath();
                ctx.fill(); ctx.stroke();

                ctx.fillStyle    = sketchState.orthoLock ? '#fbbf24'
                                 : snapPt               ? '#6ee7b7'
                                 :                        '#67e8f9';
                ctx.textAlign    = 'left';
                ctx.textBaseline = 'middle';
                ctx.fillText(badge, lx + 5, ly + boxH * 0.5);
              }
            }

            ctx.restore();
            } // end cx !== null
          }

          // ── Projection drafting overlays (Phase 13) ──
          if (sketchState.draftMode === 'projection' && window.__showProjectionGuide !== false) {
            // Plane badge (top-left, below plane pills).
            const lbl = window.__planeDescriptor
              ? window.__planeDescriptor(sketchState.workingPlane)
              : (sketchState.workingPlane || 'XZ');
            ctx.font = 'bold 12px "JetBrains Mono", system-ui, monospace';
            const bw = ctx.measureText(lbl).width + 16;
            ctx.fillStyle = 'rgba(15,23,42,0.92)';
            ctx.fillRect(16, 60, bw, 24);
            ctx.strokeStyle = 'rgba(56,189,248,0.55)';
            ctx.lineWidth = 1;
            ctx.strokeRect(16, 60, bw, 24);
            ctx.fillStyle = '#67e8f9';
            ctx.textAlign = 'left'; ctx.textBaseline = 'middle';
            ctx.fillText(lbl, 24, 72);

            // Guide lines for hovered / selected points along the in-plane axes.
            const pl = sketchState.workingPlane || 'XZ';
            const guideTargets = [];
            if (sketchState.hoverPointId) {
              const p = pById.get(sketchState.hoverPointId);
              if (p) guideTargets.push({ p, color: 'rgba(250,204,21,0.55)' });
            }
            for (const id of sketchState.selectedPointIds) {
              const p = pById.get(id);
              if (p) guideTargets.push({ p, color: 'rgba(251,146,60,0.65)' });
            }
            if (guideTargets.length && sketchState.projection.showGuides) {
              const D = 50;
              const guideAlpha = window.__fadeBackgroundHelpers ? 0.28 : 0.65;
              ctx.save();
              ctx.setLineDash([3, 3]);
              ctx.lineWidth = 1;
              ctx.globalAlpha = guideAlpha;
              for (const g of guideTargets) {
                const p = g.p;
                let line1A, line1B, line2A, line2B;
                if (pl === 'XZ') {
                  line1A = w2s(-D, p.y, p.z); line1B = w2s(D, p.y, p.z); // X-axis guide
                  line2A = w2s(p.x, p.y, -D); line2B = w2s(p.x, p.y,  D); // Z-axis guide
                } else if (pl === 'XY') {
                  line1A = w2s(-D, p.y, p.z); line1B = w2s(D, p.y, p.z);
                  line2A = w2s(p.x, -D, p.z); line2B = w2s(p.x,  D, p.z);
                } else { // YZ
                  line1A = w2s(p.x, p.y, -D); line1B = w2s(p.x, p.y, D);
                  line2A = w2s(p.x, -D, p.z); line2B = w2s(p.x,  D, p.z);
                }
                ctx.strokeStyle = g.color;
                ctx.beginPath();
                if (line1A && line1B) { ctx.moveTo(line1A.x, line1A.y); ctx.lineTo(line1B.x, line1B.y); }
                if (line2A && line2B) { ctx.moveTo(line2A.x, line2A.y); ctx.lineTo(line2B.x, line2B.y); }
                ctx.stroke();
                // Coordinate label near the point with visible coords for this plane.
                const map = window.__projectionCoordsForPlane && window.__projectionCoordsForPlane(p, pl);
                if (map) {
                  const ps = w2s(p.x, p.y, p.z);
                  if (ps) {
                    const fmt = window.__fmtCoord || (v => Number(v).toFixed(2));
                    const t = pl + ' · ' + map.hAxis + '=' + fmt(map.h) + ' ' + map.vAxis + '=' + fmt(map.v)
                            + '  (' + map.hiddenAxis + '=' + fmt(map.hidden) + ')';
                    ctx.font = '10px "JetBrains Mono", system-ui, monospace';
                    const tw2 = ctx.measureText(t).width + 8;
                    ctx.setLineDash([]);
                    ctx.fillStyle = 'rgba(15,23,42,0.85)';
                    ctx.fillRect(ps.x + 10, ps.y + 10, tw2, 16);
                    ctx.fillStyle = '#67e8f9';
                    ctx.textAlign = 'left'; ctx.textBaseline = 'middle';
                    ctx.fillText(t, ps.x + 14, ps.y + 18);
                    ctx.setLineDash([3, 3]);
                  }
                }
              }
              ctx.restore();
            }
          }

          // ── Drafting Overlay (Phase 16) ──────────────────────────
          // Engineering-drawing decorations on top of geometry. Pure visual.
          (function drawDraftingOverlay() {
            const df = sketchState.drafting;
            if (!df) return;

            // Reset hit-test registry for this frame.
            sketchState.draftingHitLabels = [];

            const internalMm = ((sketchState.precision && sketchState.precision.internalStepM) || 0.00001) * 1000;
            const DIM_COL    = 'rgba(226,232,240,0.85)';
            const DIM_FILL   = 'rgba(15,23,42,0.85)';
            const EXT_COL    = 'rgba(148,163,184,0.75)';
            const CENTER_COL = 'rgba(167,139,250,0.65)';
            const fontMain   = '11px "JetBrains Mono", system-ui, monospace';
            const fontTiny   = '10px "JetBrains Mono", system-ui, monospace';

            // ── Primitive: arrowhead at (x,y) along direction (dx,dy) ──
            function arrow(x, y, dx, dy, size) {
              const L  = Math.hypot(dx, dy) || 1;
              const ux = dx / L, uy = dy / L;
              const px = -uy,    py = ux; // perpendicular
              const bx = x - ux * size,   by = y - uy * size;
              const lx = bx + px * size * 0.4, ly = by + py * size * 0.4;
              const rx = bx - px * size * 0.4, ry = by - py * size * 0.4;
              ctx.beginPath();
              ctx.moveTo(x, y); ctx.lineTo(lx, ly); ctx.lineTo(rx, ry); ctx.closePath();
              ctx.fill();
            }

            // ── Primitive: full dimension line A→B with extension lines,
            //              arrowheads and centered label. Screen-space.
            function drawDimension(saX, saY, sbX, sbY, label, opts, hitMeta) {
              opts = opts || {};
              const off  = (opts.offsetPx  != null) ? opts.offsetPx  : (df.dimensionOffsetPx || 20);
              const arrS = (opts.arrowPx   != null) ? opts.arrowPx   : (df.arrowSizePx       || 7);
              const gap  = (opts.gapPx     != null) ? opts.gapPx     : (df.textGapPx         || 6);
              const flip = !!opts.flip; // mirror offset to the other side
              let dx = sbX - saX, dy = sbY - saY;
              const L = Math.hypot(dx, dy);
              if (L < 1) return;
              const ux = dx / L, uy = dy / L;
              // normal: 90° CCW; flip → flip side.
              let nx = -uy, ny = ux;
              if (flip) { nx = -nx; ny = -ny; }
              const off1 = off;
              const dax = saX + nx * off1, day = saY + ny * off1;
              const dbx = sbX + nx * off1, dby = sbY + ny * off1;

              ctx.save();
              ctx.lineWidth = 1;
              ctx.strokeStyle = EXT_COL;
              // Extension lines (from geometry to slightly past the dim line).
              ctx.beginPath();
              ctx.moveTo(saX + nx * gap, saY + ny * gap);
              ctx.lineTo(saX + nx * (off1 + 4), saY + ny * (off1 + 4));
              ctx.moveTo(sbX + nx * gap, sbY + ny * gap);
              ctx.lineTo(sbX + nx * (off1 + 4), sbY + ny * (off1 + 4));
              ctx.stroke();

              // Dimension line.
              ctx.strokeStyle = DIM_COL;
              ctx.lineWidth = 1.1;
              ctx.beginPath();
              ctx.moveTo(dax, day); ctx.lineTo(dbx, dby);
              ctx.stroke();

              // Arrowheads (pointing outward at each end).
              ctx.fillStyle = DIM_COL;
              arrow(dax, day, -ux, -uy, arrS);
              arrow(dbx, dby,  ux,  uy, arrS);

              // Label centered.
              const mx = (dax + dbx) * 0.5;
              const my = (day + dby) * 0.5;
              ctx.font = fontMain;
              const tw = ctx.measureText(label).width + 10;
              const th = 16;
              ctx.fillStyle = DIM_FILL;
              ctx.fillRect(mx - tw / 2, my - th / 2, tw, th);
              ctx.fillStyle = '#e2e8f0';
              ctx.textAlign = 'center'; ctx.textBaseline = 'middle';
              ctx.fillText(label, mx, my);
              ctx.restore();

              // Register hit rect (6px padding) for click-to-edit.
              if (hitMeta) {
                const pad = 6;
                sketchState.draftingHitLabels.push(Object.assign({}, hitMeta, {
                  rect: { x: mx - tw / 2 - pad, y: my - th / 2 - pad, w: tw + pad * 2, h: th + pad * 2 },
                }));
              }
            }

            // ── Dimensions for selected edges ──
            if (df.showDimensions) {
              for (const eid of sketchState.selectedEdgeIds) {
                const e = eById.get(eid); if (!e) continue;
                const a = pById.get(e.a), b = pById.get(e.b); if (!a || !b) continue;
                const sa = w2s(a.x, a.y, a.z), sb = w2s(b.x, b.y, b.z);
                if (!sa || !sb) continue;
                const lenMm = window.__edgeLengthMm(a, b);
                drawDimension(sa.x, sa.y, sb.x, sb.y, window.__formatDim(lenMm), undefined,
                  { kind: 'edge_length_dimension', edgeId: eid, aPointId: a.id, bPointId: b.id, valueMm: lenMm });
              }

              // ── Profile main dimensions (width on top, height on right) ──
              const profId = sketchState.selectedProfileId;
              if (profId) {
                const prof = (sketchState.profiles || []).find(p => p.id === profId);
                if (prof && prof.pointIds && prof.pointIds.length >= 3) {
                  // Compute screen-space bounding box of the profile.
                  let minSx = Infinity, minSy = Infinity, maxSx = -Infinity, maxSy = -Infinity;
                  // Also compute world-space extents in the working plane axes
                  // so the label is exact engineering length.
                  let minGx = Infinity, maxGx = -Infinity;
                  let minGy = Infinity, maxGy = -Infinity;
                  let minGz = Infinity, maxGz = -Infinity;
                  let okScreen = true;
                  for (const pid of prof.pointIds) {
                    const p = pById.get(pid); if (!p) { okScreen = false; break; }
                    const s = w2s(p.x, p.y, p.z); if (!s) { okScreen = false; break; }
                    if (s.x < minSx) minSx = s.x;
                    if (s.y < minSy) minSy = s.y;
                    if (s.x > maxSx) maxSx = s.x;
                    if (s.y > maxSy) maxSy = s.y;
                    if (p.gx < minGx) minGx = p.gx; if (p.gx > maxGx) maxGx = p.gx;
                    if (p.gy < minGy) minGy = p.gy; if (p.gy > maxGy) maxGy = p.gy;
                    if (p.gz < minGz) minGz = p.gz; if (p.gz > maxGz) maxGz = p.gz;
                  }
                  if (okScreen) {
                    // Pick width/height axes from the plane.
                    const pl = prof.plane || sketchState.workingPlane || 'XZ';
                    let widthMm, heightMm;
                    if      (pl === 'XZ') { widthMm = (maxGx - minGx) * internalMm; heightMm = (maxGz - minGz) * internalMm; }
                    else if (pl === 'XY') { widthMm = (maxGx - minGx) * internalMm; heightMm = (maxGy - minGy) * internalMm; }
                    else                  { widthMm = (maxGz - minGz) * internalMm; heightMm = (maxGy - minGy) * internalMm; }

                    // Top horizontal dim (width): from top-left to top-right.
                    if (widthMm > 0) {
                      drawDimension(minSx, minSy, maxSx, minSy,
                                    window.__formatDim(widthMm),
                                    { flip: true /* offset upward */ },
                                    { kind: 'profile_width_dimension', profileId: profId, valueMm: widthMm });
                    }
                    // Right vertical dim (height): from top-right to bottom-right.
                    if (heightMm > 0) {
                      drawDimension(maxSx, minSy, maxSx, maxSy,
                                    window.__formatDim(heightMm),
                                    {},
                                    { kind: 'profile_height_dimension', profileId: profId, valueMm: heightMm });
                    }
                  }
                }
              }
            }

            // ── Edge lengths (every edge, throttled when many) ──
            if (df.showEdgeLengths) {
              const total = sketchState.edges.length;
              const showAll = total < 20;
              for (const e of sketchState.edges) {
                const isHover = sketchState.hoverEdgeId === e.id;
                const isSel   = sketchState.selectedEdgeIds.has(e.id);
                if (!showAll && !isHover && !isSel) continue;
                if (df.showDimensions && isSel) continue; // already drawn as proper dim
                const a = pById.get(e.a), b = pById.get(e.b); if (!a || !b) continue;
                const sa = w2s(a.x, a.y, a.z), sb = w2s(b.x, b.y, b.z);
                if (!sa || !sb) continue;
                const lenMm = window.__edgeLengthMm(a, b);
                const txt = window.__formatDim(lenMm);
                const mx = (sa.x + sb.x) * 0.5, my = (sa.y + sb.y) * 0.5;
                ctx.font = fontTiny;
                const tw = ctx.measureText(txt).width + 8;
                ctx.fillStyle = DIM_FILL;
                ctx.fillRect(mx - tw / 2, my - 8, tw, 14);
                ctx.fillStyle = '#cbd5e1';
                ctx.textAlign = 'center'; ctx.textBaseline = 'middle';
                ctx.fillText(txt, mx, my - 1);
                // Register hit rect for click-to-edit.
                const pad = 6;
                sketchState.draftingHitLabels.push({
                  kind: 'edge_length_dimension', edgeId: e.id, aPointId: a.id, bPointId: b.id, valueMm: lenMm,
                  rect: { x: mx - tw / 2 - pad, y: my - 8 - pad, w: tw + pad * 2, h: 14 + pad * 2 },
                });
              }
            }

            // ── Point coordinate labels (hovered + selected only) ──
            if (df.showPointLabels) {
              const ids = new Set(sketchState.selectedPointIds);
              if (sketchState.hoverPointId) ids.add(sketchState.hoverPointId);
              for (const id of ids) {
                const p = pById.get(id); if (!p) continue;
                const s = w2s(p.x, p.y, p.z); if (!s) continue;
                const c = window.__pointCoordsMm(p);
                const t = 'X ' + c.x + '  Y ' + c.y + '  Z ' + c.z;
                ctx.font = fontTiny;
                const tw = ctx.measureText(t).width + 8;
                ctx.fillStyle = DIM_FILL;
                ctx.fillRect(s.x + 10, s.y - 22, tw, 14);
                ctx.fillStyle = '#cbd5e1';
                ctx.textAlign = 'left'; ctx.textBaseline = 'middle';
                ctx.fillText(t, s.x + 14, s.y - 15);
              }
            }

            // ── Centerlines (dashed crosses through profile centroids) ──
            if (df.showCenterlines && sketchState.profiles && sketchState.profiles.length) {
              ctx.save();
              ctx.setLineDash([8, 3, 2, 3]);
              ctx.lineWidth = 0.8;
              ctx.strokeStyle = CENTER_COL;
              for (const prof of sketchState.profiles) {
                let cx = 0, cy = 0, cz = 0, n = 0;
                for (const pid of prof.pointIds) {
                  const p = pById.get(pid); if (!p) { n = 0; break; }
                  cx += p.x; cy += p.y; cz += p.z; n++;
                }
                if (!n) continue;
                cx /= n; cy /= n; cz /= n;
                const sc = w2s(cx, cy, cz); if (!sc) continue;
                ctx.beginPath();
                ctx.moveTo(sc.x - 18, sc.y); ctx.lineTo(sc.x + 18, sc.y);
                ctx.moveTo(sc.x, sc.y - 18); ctx.lineTo(sc.x, sc.y + 18);
                ctx.stroke();
              }
              ctx.restore();
            }

            // ── Grid numbers (ruler along left + bottom viewport edges) ──
            if (df.showGridNumbers) {
              const pr      = sketchState.precision || {};
              const dispM   = (pr.displayGridStepM > 0) ? pr.displayGridStepM : 0.001;
              const pl      = sketchState.workingPlane || 'XZ';
              const minStep = 60; // do not draw labels closer than 60 px
              // For each axis on this plane, walk integer multiples of dispM
              // around the visible extent and project to screen.
              function rulerAxis(getWorld, dimEdge /* 'bottom' | 'left' */, axisName) {
                // Scan from -N .. N around origin; in practice the camera frames
                // origin, so this catches everything visible.
                const N = 200;
                let lastPx = -Infinity;
                ctx.font = fontTiny;
                ctx.fillStyle = '#94a3b8';
                ctx.textAlign = (dimEdge === 'bottom') ? 'center' : 'right';
                ctx.textBaseline = (dimEdge === 'bottom') ? 'bottom' : 'middle';
                for (let i = -N; i <= N; i++) {
                  const w = getWorld(i * dispM);
                  const s = w2s(w[0], w[1], w[2]);
                  if (!s) continue;
                  const key = (dimEdge === 'bottom') ? s.x : s.y;
                  if (Math.abs(key - lastPx) < minStep) continue;
                  // Clip to viewport edge zone.
                  if (dimEdge === 'bottom') {
                    if (s.x < 30 || s.x > sk.width - 30) continue;
                  } else {
                    if (s.y < 30 || s.y > sk.height - 30) continue;
                  }
                  lastPx = key;
                  const valMm = (i * dispM) * 1000;
                  const txt = window.__formatDim(valMm);
                  if (dimEdge === 'bottom') {
                    ctx.fillText(txt, s.x, sk.height - 6);
                  } else {
                    ctx.fillText(txt, sk.width - 6, s.y);
                  }
                }
              }
              // Bottom ruler = horizontal axis of current plane.
              // Left   ruler = vertical   axis of current plane.
              if (pl === 'XZ') {
                rulerAxis(v => [v, 0, 0], 'bottom', 'X');
                rulerAxis(v => [0, 0, v], 'left',   'Z');
              } else if (pl === 'XY') {
                rulerAxis(v => [v, 0, 0], 'bottom', 'X');
                rulerAxis(v => [0, v, 0], 'left',   'Y');
              } else { // YZ
                rulerAxis(v => [0, 0, v], 'bottom', 'Z');
                rulerAxis(v => [0, v, 0], 'left',   'Y');
              }
            }
          })();

          // ── Status message banner (bottom-center) ──
          if (sketchState.statusMessage) {
            const txt = sketchState.statusMessage;
            ctx.font = '12px "JetBrains Mono", system-ui, monospace';
            const tw = ctx.measureText(txt).width + 20;
            const y0 = sk.height - 70;
            ctx.fillStyle = 'rgba(15,23,42,0.92)';
            ctx.fillRect(sk.width/2 - tw/2, y0, tw, 26);
            ctx.strokeStyle = 'rgba(251,191,36,0.55)';
            ctx.lineWidth = 1;
            ctx.strokeRect(sk.width/2 - tw/2, y0, tw, 26);
            ctx.fillStyle = '#fbbf24';
            ctx.textAlign = 'center'; ctx.textBaseline = 'middle';
            ctx.fillText(txt, sk.width/2, y0 + 13);
          }

          // ── Grab HUD + Axis Gizmo (tools/grab_gizmo.rs) ──
          if (typeof window.__drawGrabGizmo === 'function') {
            window.__drawGrabGizmo(ctx, sketchState, w2s, sk);
          }
          // ── Extrude Height Gizmo (tools/extrude_gizmo.rs) ──
          if (typeof window.__drawExtrudeGizmo === 'function') {
            window.__drawExtrudeGizmo(ctx, sketchState, w2s, sk);
          }
          // ── Solid Extrude Gizmo / Plasticity-style (tools/solid_extrude_gizmo.rs) ──
          if (typeof window.__drawSolidExtrudeGizmo === 'function') {
            window.__drawSolidExtrudeGizmo(ctx, w2s);
          }
          if (sketchState.grab.active) {
            const grab = sketchState.grab;
            const lock = grab.axisLock;
            const lockColor = lock === 'X' ? '#ef4444' : lock === 'Y' ? '#22c55e' : lock === 'Z' ? '#3b82f6' : '#a78bfa';
            const byId = new Map(sketchState.points.map(p => [p.id, p]));
            let cx = 0, cy0 = 0, cz = 0, n = 0;
            for (const id of grab.pointIds) { const p = byId.get(id); if (!p) continue; cx += p.x; cy0 += p.y; cz += p.z; n++; }
            if (n > 0) { cx /= n; cy0 /= n; cz /= n; }
            const center = w2s(cx, cy0, cz);
            if (false) { // legacy inline code below — kept for reference, replaced by grab_gizmo.rs

            // ── Draw Minimalist Axis Gizmo (Plasticity Style) ──
            const hoveredHandle = window.__gizmoHoverAxis || null;
            if (center) {
              const arrowLen = 7;
              const arrowW   = 1.5;
              const HIT_R    = 12;
              const axes = [
                { axis: 'X', color: '#ef4444', wx: arrowLen, wy: 0, wz: 0 },
                { axis: 'Y', color: '#22c55e', wx: 0, wy: arrowLen, wz: 0 },
                { axis: 'Z', color: '#3b82f6', wx: 0, wy: 0, wz: arrowLen },
              ];
              const pOffset = 1.0;
              const pSz = 1.8;
              const pln = [
                { axis: 'XY', color: '#fcd34d', p1: [pOffset, pOffset, 0], p2: [pOffset+pSz, pOffset, 0], p3: [pOffset+pSz, pOffset+pSz, 0], p4: [pOffset, pOffset+pSz, 0] },
                { axis: 'YZ', color: '#fcd34d', p1: [0, pOffset, pOffset], p2: [0, pOffset+pSz, pOffset], p3: [0, pOffset+pSz, pOffset+pSz], p4: [0, pOffset, pOffset+pSz] },
                { axis: 'XZ', color: '#fcd34d', p1: [pOffset, 0, pOffset], p2: [pOffset+pSz, 0, pOffset], p3: [pOffset+pSz, 0, pOffset+pSz], p4: [pOffset, 0, pOffset+pSz] },
              ];

              const handles = [];
              ctx.save();

              // Planar squares
              for (const p of pln) {
                const s1 = w2s(cx + p.p1[0], cy0 + p.p1[1], cz + p.p1[2]);
                const s2 = w2s(cx + p.p2[0], cy0 + p.p2[1], cz + p.p2[2]);
                const s3 = w2s(cx + p.p3[0], cy0 + p.p3[1], cz + p.p3[2]);
                const s4 = w2s(cx + p.p4[0], cy0 + p.p4[1], cz + p.p4[2]);
                if (!s1 || !s2 || !s3 || !s4) continue;

                const isLocked  = (lock === p.axis);
                const isHovered = (hoveredHandle === p.axis);

                ctx.beginPath();
                ctx.moveTo(s1.x, s1.y); ctx.lineTo(s2.x, s2.y);
                ctx.lineTo(s3.x, s3.y); ctx.lineTo(s4.x, s4.y);
                ctx.closePath();

                if (isLocked || isHovered) {
                  ctx.fillStyle = p.color;
                  ctx.globalAlpha = 0.6;
                  ctx.fill();
                  ctx.strokeStyle = '#fff';
                  ctx.globalAlpha = 0.9;
                  ctx.lineWidth = 1;
                  ctx.stroke();
                } else {
                  ctx.fillStyle = 'rgba(255, 255, 255, 0.2)';
                  if (lock) ctx.globalAlpha = 0.05;
                  else ctx.globalAlpha = 1.0;
                  ctx.fill();
                  ctx.strokeStyle = 'rgba(255, 255, 255, 0.4)';
                  if (!lock) ctx.stroke();
                }

                // Hit center
                const hx = (s1.x + s3.x) / 2;
                const hy = (s1.y + s3.y) / 2;
                handles.push({ axis: p.axis, x: hx, y: hy, r: 12 });
              }

              // Axes
              for (const a of axes) {
                const tip = w2s(cx + a.wx, cy0 + a.wy, cz + a.wz);
                const base = w2s(cx + a.wx*0.2, cy0 + a.wy*0.2, cz + a.wz*0.2); // slight gap from origin
                if (!tip || !base) continue;
                const isLocked  = (lock === a.axis);
                const isHovered = (hoveredHandle === a.axis);
                
                // Dim other axes when one is locked
                if (lock && !isLocked) {
                    ctx.globalAlpha = 0.15;
                } else {
                    ctx.globalAlpha = (isLocked || isHovered) ? 1.0 : 0.85;
                }
                const angle = Math.atan2(tip.y - base.y, tip.x - base.x);

                // Minimalist Shaft
                ctx.strokeStyle = a.color;
                ctx.lineWidth   = isHovered || isLocked ? arrowW + 1 : arrowW;
                ctx.beginPath();
                ctx.moveTo(base.x, base.y);
                ctx.lineTo(tip.x, tip.y);
                ctx.stroke();

                // Sleek Arrowhead
                ctx.fillStyle = a.color;
                const hw = 3.5, hl = 12;
                ctx.beginPath();
                ctx.moveTo(tip.x + 2*Math.cos(angle), tip.y + 2*Math.sin(angle));
                ctx.lineTo(tip.x - hl*Math.cos(angle) + hw*Math.sin(angle),
                           tip.y - hl*Math.sin(angle) - hw*Math.cos(angle));
                ctx.lineTo(tip.x - hl*Math.cos(angle) - hw*Math.sin(angle),
                           tip.y - hl*Math.sin(angle) + hw*Math.cos(angle));
                ctx.closePath();
                ctx.fill();

                // Push hit area around the tip
                handles.push({ axis: a.axis, x: tip.x, y: tip.y, r: HIT_R });
              }

              // Center FREE handle (screen space)
              const cIsLocked = (lock === 'FREE');
              const cIsHovered = (hoveredHandle === 'FREE');
              ctx.globalAlpha = (cIsLocked || cIsHovered) ? 1.0 : (lock ? 0.15 : 0.8);
              ctx.beginPath();
              ctx.arc(center.x, center.y, 4, 0, Math.PI * 2);
              ctx.fillStyle = '#fff';
              ctx.fill();
              if (cIsLocked || cIsHovered) {
                ctx.beginPath();
                ctx.arc(center.x, center.y, 8, 0, Math.PI * 2);
                ctx.strokeStyle = '#fff';
                ctx.lineWidth = 1;
                ctx.stroke();
              }
              handles.push({ axis: 'FREE', x: center.x, y: center.y, r: 12 });

              ctx.globalAlpha = 1.0;
              ctx.restore();

              window.__gizmoHandles = handles;
              window.__gizmoCenterScreen = center;
            } else {
              window.__gizmoHandles = null;
            }

            // ── Active axis dashed infinite guide ──
            if (lock && grab.startMouseWorld) {
              const o = grab.startMouseWorld;
              const d = 1000; // make it truly long across screen
              let p1, p2;
              if (lock === 'X') { p1 = w2s(o.x - d, o.y, o.z); p2 = w2s(o.x + d, o.y, o.z); }
              if (lock === 'Y') { p1 = w2s(o.x, o.y - d, o.z); p2 = w2s(o.x, o.y + d, o.z); }
              if (lock === 'Z') { p1 = w2s(o.x, o.y, o.z - d); p2 = w2s(o.x, o.y, o.z + d); }
              if (p1 && p2) {
                ctx.save();
                ctx.setLineDash([4, 4]);
                ctx.strokeStyle = lockColor;
                ctx.globalAlpha = 0.5;
                ctx.lineWidth = 1.0;
                ctx.beginPath(); ctx.moveTo(p1.x, p1.y); ctx.lineTo(p2.x, p2.y); ctx.stroke();
                ctx.restore();
              }
            }

            // ── Delta readout near cursor ──
            if (grab.pointIds.length > 0) {
              const byId2 = byId;
              const sampleId   = grab.pointIds[0];
              const sampleOrig = grab.originalPoints.get(sampleId);
              const sampleNow  = byId2.get(sampleId);
              if (sampleOrig && sampleNow) {
                const ddx = (sampleNow.x - sampleOrig.x).toFixed(2);
                const ddy = (sampleNow.y - sampleOrig.y).toFixed(2);
                const ddz = (sampleNow.z - sampleOrig.z).toFixed(2);
                const dist = Math.hypot(sampleNow.x - sampleOrig.x, sampleNow.y - sampleOrig.y, sampleNow.z - sampleOrig.z).toFixed(2);
                const deltaLabel = (lock ? lock + ' ' : '') + '|Δ|' + dist
                  + '  X' + ddx + ' Y' + ddy + ' Z' + ddz;
                const scrX = (sketchState.hoverWorld && sketchState.hoverWorld.screenX) || (center ? center.x : sk.width/2);
                const scrY = (sketchState.hoverWorld && sketchState.hoverWorld.screenY)
                  ? sketchState.hoverWorld.screenY - 22
                  : (center ? center.y - 22 : 80);
                ctx.save();
                ctx.font = '11px "JetBrains Mono", system-ui, monospace';
                const tw2 = ctx.measureText(deltaLabel).width + 12;
                ctx.fillStyle = 'rgba(15,23,42,0.9)';
                ctx.fillRect(scrX - tw2/2, scrY - 10, tw2, 19);
                ctx.fillStyle = lockColor;
                ctx.textAlign = 'center'; ctx.textBaseline = 'middle';
                ctx.fillText(deltaLabel, scrX, scrY - 1);
                ctx.restore();
              }
            }

            // ── Banner ──
            const txt = '⤢ GRAB ' + grab.pointIds.length + (lock ? (' · ' + lock + '-axis') : ' · free') + '  X/Y/Z lock · Enter confirm · Esc cancel';
            ctx.font = '12px "JetBrains Mono", system-ui, monospace';
            const tw = ctx.measureText(txt).width + 16;
            ctx.fillStyle = 'rgba(15,23,42,0.92)';
            ctx.fillRect(sk.width/2 - tw/2, 16, tw, 26);
            ctx.fillStyle = lockColor;
            ctx.textAlign = 'center'; ctx.textBaseline = 'middle';
            ctx.fillText(txt, sk.width/2, 29);
            } // end if(false) legacy block
          }

          // ── Copy Connect preview (Phase 14) ──
          if (sketchState.copy.active) {
            const cp = sketchState.copy;
            const { dx, dy, dz } = cp.delta;
            const lock = cp.axisLock;
            const lockColor = lock === 'X' ? '#ef4444' : lock === 'Y' ? '#22c55e' : lock === 'Z' ? '#3b82f6' : '#22d3ee';
            const cyan = '#22d3ee';

            const byId = new Map(sketchState.points.map(p => [p.id, p]));
            // Build screen-space positions of original and copied points.
            const origScr = new Map();
            const copyScr = new Map();
            for (const id of cp.pointIds) {
              const o = cp.originals.get(id);
              if (!o) continue;
              const so = w2s(o.x, o.y, o.z);
              const sc = w2s(o.x + dx, o.y + dy, o.z + dz);
              if (so) origScr.set(id, so);
              if (sc) copyScr.set(id, sc);
            }

            ctx.save();

            // Inner copied edges (cyan dashed).
            ctx.setLineDash([5, 4]);
            ctx.strokeStyle = cyan;
            ctx.lineWidth = 1.6;
            for (const [a, b] of cp.edges) {
              const pa = copyScr.get(a);
              const pb = copyScr.get(b);
              if (!pa || !pb) continue;
              ctx.beginPath(); ctx.moveTo(pa.x, pa.y); ctx.lineTo(pb.x, pb.y); ctx.stroke();
            }
            // Connector edges (thinner cyan dashed) original → copy.
            ctx.setLineDash([3, 4]);
            ctx.strokeStyle = 'rgba(34,211,238,0.75)';
            ctx.lineWidth = 1.0;
            for (const id of cp.pointIds) {
              const a = origScr.get(id);
              const b = copyScr.get(id);
              if (!a || !b) continue;
              ctx.beginPath(); ctx.moveTo(a.x, a.y); ctx.lineTo(b.x, b.y); ctx.stroke();
            }
            ctx.setLineDash([]);

            // Copied preview points (cyan dots).
            ctx.fillStyle = cyan;
            for (const p of copyScr.values()) {
              ctx.beginPath(); ctx.arc(p.x, p.y, 3.2, 0, Math.PI * 2); ctx.fill();
            }

            // Axis-lock guide line through start point.
            if (lock && cp.startMouseWorld) {
              const o = cp.startMouseWorld;
              const dst = 50;
              let p1, p2;
              if (lock === 'X') { p1 = w2s(o.x - dst, o.y, o.z); p2 = w2s(o.x + dst, o.y, o.z); }
              if (lock === 'Y') { p1 = w2s(o.x, o.y - dst, o.z); p2 = w2s(o.x, o.y + dst, o.z); }
              if (lock === 'Z') { p1 = w2s(o.x, o.y, o.z - dst); p2 = w2s(o.x, o.y, o.z + dst); }
              if (p1 && p2) {
                ctx.setLineDash([4, 4]);
                ctx.strokeStyle = lockColor;
                ctx.lineWidth = 1.5;
                ctx.beginPath(); ctx.moveTo(p1.x, p1.y); ctx.lineTo(p2.x, p2.y); ctx.stroke();
                ctx.setLineDash([]);
              }
            }
            ctx.restore();

            // HUD banner (top-center).
            const dist = Math.hypot(dx, dy, dz);
            const head = 'COPY ' + cp.pointIds.length + 'pt'
              + (lock ? (' · ' + lock + '-axis') : '')
              + '  ΔX ' + dx.toFixed(2) + '  ΔY ' + dy.toFixed(2) + '  ΔZ ' + dz.toFixed(2)
              + '  · |Δ| ' + dist.toFixed(2);
            ctx.save();
            ctx.font = '12px "JetBrains Mono", system-ui, monospace';
            const tw = ctx.measureText(head).width + 16;
            ctx.fillStyle = 'rgba(15,23,42,0.92)';
            ctx.fillRect(sk.width/2 - tw/2, 46, tw, 24);
            ctx.fillStyle = lockColor;
            ctx.textAlign = 'center'; ctx.textBaseline = 'middle';
            ctx.fillText(head, sk.width/2, 58);
            ctx.restore();
          }
        }

        if (window.__perfSample)    window.__perfSample('overlay', performance.now() - __pfOverlay);
        if (window.__updatePerfHud) window.__updatePerfHud();
        if (window.__cadPanelTick)  window.__cadPanelTick();

        gpuRafId = requestAnimationFrame(frame);
      }
      gpuRafId = requestAnimationFrame(frame);

      window.__stopWebGpuScene = function() {
        if (gpuRafId) cancelAnimationFrame(gpuRafId);
        gpuRafId = null;
      };
    }
    window.startWebGpuScene = startWebGpuScene;
"##;
