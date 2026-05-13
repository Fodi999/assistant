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
          const g     = sketchState.gridSize || 1.0;
          const N     = 20;
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

            // Constraint badges + length.
            const dimC = window.__getEdgeLengthConstraint && window.__getEdgeLengthConstraint(e.id);
            const isH  = window.__hasHorizontalConstraint && window.__hasHorizontalConstraint(e.id);
            const isV  = window.__hasVerticalConstraint   && window.__hasVerticalConstraint(e.id);
            const len  = Math.hypot(b.x-a.x, b.y-a.y, b.z-a.z);
            const mx   = (sa.x + sb.x) * 0.5;
            const my   = (sa.y + sb.y) * 0.5;

            if (dimC || isHover || isSel) {
              const txt = len.toFixed(2) + 'u';
              ctx.font = '11px "JetBrains Mono", system-ui, monospace';
              const w = ctx.measureText(txt).width + 8;
              const bright = !!dimC;
              ctx.fillStyle = bright ? 'rgba(15,23,42,0.95)' : 'rgba(15,23,42,0.85)';
              ctx.fillRect(mx - w/2, my - 18, w, 16);
              ctx.fillStyle = bright ? '#67e8f9' : (isSel ? '#fb923c' : '#facc15');
              ctx.textAlign = 'center'; ctx.textBaseline = 'middle';
              ctx.fillText(txt, mx, my - 10);
            }
            if (isH || isV) {
              const tag = isH ? 'H' : 'V';
              ctx.font = 'bold 11px "JetBrains Mono", system-ui, monospace';
              const wt = ctx.measureText(tag).width + 8;
              ctx.fillStyle = 'rgba(167,139,250,0.95)';
              ctx.fillRect(mx + 8, my - 8, wt, 16);
              ctx.fillStyle = '#0f172a';
              ctx.textAlign = 'center'; ctx.textBaseline = 'middle';
              ctx.fillText(tag, mx + 8 + wt/2, my);
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
                const txt = len.toFixed(2) + 'u';
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

          // ── Validation markers (drawn under points so points still visible) ──
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

          // ── Snap marker + ghost preview point ──
          if (sketchState.hoverWorld &&
              (sketchState.activeTool === 'point' || sketchState.activeTool === 'line')) {
            const hw = sketchState.hoverWorld;
            // Use the stored cursor screen position so the crosshair is
            // always exactly at the pointer tip (snapped world projected back
            // via w2s can drift by sub-pixel on non-integer grids).
            let cx, cy;
            if (hw.screenX !== undefined && hw.screenY !== undefined) {
              cx = hw.screenX;
              cy = hw.screenY;
            } else {
              const _c = w2s(hw.x, hw.y, hw.z);
              if (!_c) { cx = null; }
              else { cx = _c.x; cy = _c.y; }
            }
            if (cx !== null && cx !== undefined) {
              const kind = sketchState.snap.kind;
              const snapPoint = kind === 'point';
              const snapFree  = kind === 'free';
              const snapColor = snapPoint ? '#10b981'
                              : snapFree  ? '#cbd5e1'
                              : '#67e8f9';
              if (!snapPoint) {
                ctx.save();
                ctx.beginPath();
                ctx.arc(cx, cy, 5.5, 0, Math.PI * 2);
                ctx.fillStyle = snapFree
                  ? 'rgba(203,213,225,0.25)'
                  : 'rgba(56,189,248,0.35)';
                ctx.strokeStyle = snapColor;
                ctx.setLineDash([2, 2]);
                ctx.lineWidth = 1.5;
                ctx.fill();
                ctx.stroke();
                ctx.restore();
              }
              ctx.strokeStyle = snapPoint ? 'rgba(16,185,129,0.9)'
                              : snapFree  ? 'rgba(203,213,225,0.7)'
                              : 'rgba(250,204,21,0.7)';
              ctx.lineWidth = 1;
              ctx.beginPath();
              ctx.moveTo(cx - 10, cy); ctx.lineTo(cx + 10, cy);
              ctx.moveTo(cx, cy - 10); ctx.lineTo(cx, cy + 10);
              ctx.stroke();
              const fmtC = window.__fmtCoord || (v => Number(v).toFixed(3));
              const line1 = 'g ' + hw.gx + ',' + hw.gy + ',' + hw.gz;
              const line2 = fmtC(hw.x) + ' ' + fmtC(hw.y) + ' ' + fmtC(hw.z);
              ctx.font = '11px "JetBrains Mono", system-ui, monospace';
              const w = Math.max(ctx.measureText(line1).width, ctx.measureText(line2).width) + 10;
              ctx.fillStyle = 'rgba(15,23,42,0.88)';
              ctx.fillRect(cx + 14, cy - 18, w, 34);
              ctx.fillStyle = snapColor;
              ctx.textAlign = 'left'; ctx.textBaseline = 'middle';
              ctx.fillText(line1, cx + 19, cy - 9);
              ctx.fillStyle = '#e2e8f0';
              ctx.fillText(line2, cx + 19, cy + 7);
            }
          }

          // ── Projection drafting overlays (Phase 13) ──
          if (sketchState.draftMode === 'projection') {
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
              ctx.save();
              ctx.setLineDash([3, 3]);
              ctx.lineWidth = 1;
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
