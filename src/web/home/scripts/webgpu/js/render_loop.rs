// ── JS: render loop, UBO upload, draw calls, cleanup ─────────────────────────────
// Domain: Rendering — per-frame camera math, UBO upload, render passes, RAF.

pub const JS: &str = r##"
      // ── 7. Render loop ──────────────────────────────────────────
      const t0 = performance.now();
      let frameCount = 0;
      let lastFpsTime = t0, fpsAcc = 0, fps = 0;
      let lastFrameTime = t0;
      const ubo = new Float32Array(60); // 15 × vec4

      function frame() {
        const now = performance.now();
        const t   = (now - t0) / 1000.0;
        const dt  = (now - lastFrameTime) / 1000.0;
        const cpuFrameMs = now - lastFrameTime;
        lastFrameTime = now;
        if (frameCount === 0) {
          log('🚀 render loop запущен!', '#67e8f9');
          log('🖱  drag · wheel · 1-5 · C/V/W form · [/] shape · R · B=HUD · Shift+B=bench', '#a78bfa');
          setTimeout(() => { const diagEl = document.getElementById('gpu-diag'); if (diagEl) diagEl.style.display = 'none'; }, 4000);
        }
        frameCount++;
        fpsAcc++;
        if (now - lastFpsTime >= 500) {
          fps = fpsAcc * 1000 / (now - lastFpsTime);
          lastFpsTime = now; fpsAcc = 0;
          // update legacy HUD only when it is visible
          const hudEl = document.getElementById('gpu-hud');
          if (hudEl && hudEl.style.display !== 'none') updateHud(fps);
          if (globalThis.__matterPerf) {
            globalThis.__matterPerf.fps     = fps;
            globalThis.__matterPerf.frameMs = cpuFrameMs;
          }
        }

        // auto-rotate
        if (cam.autoRotate) cam.yaw += dt * 0.25;

        // build camera basis from spherical coords
        const cy = Math.cos(cam.yaw),   sy = Math.sin(cam.yaw);
        const cp = Math.cos(cam.pitch), sp = Math.sin(cam.pitch);
        // forward from camera → target
        const fwdX = -sy * cp, fwdY = -sp, fwdZ = cy * cp;
        // ro = target - fwd * dist
        const roX = cam.target[0] - fwdX * cam.dist;
        const roY = cam.target[1] - fwdY * cam.dist;
        const roZ = cam.target[2] - fwdZ * cam.dist;
        // right = fwd × worldUp
        const wuX = 0, wuY = 1, wuZ = 0;
        let rX = fwdY * wuZ - fwdZ * wuY;
        let rY = fwdZ * wuX - fwdX * wuZ;
        let rZ = fwdX * wuY - fwdY * wuX;
        const rL = Math.hypot(rX, rY, rZ) || 1;
        rX /= rL; rY /= rL; rZ /= rL;
        // up = right × fwd
        const uX = rY * fwdZ - rZ * fwdY;
        const uY = rZ * fwdX - rX * fwdZ;
        const uZ = rX * fwdY - rY * fwdX;

        // u0: time, w, h, pushStrength
        ubo[ 0] = t;
        ubo[ 1] = canvas.width;
        ubo[ 2] = canvas.height;
        ubo[ 3] = 0.45;            // push strength
        // u1: ro
        ubo[ 4] = roX; ubo[ 5] = roY; ubo[ 6] = roZ; ubo[ 7] = 0;
        // u2: right
        ubo[ 8] = rX;  ubo[ 9] = rY;  ubo[10] = rZ;  ubo[11] = 0;
        // u3: up
        ubo[12] = uX;  ubo[13] = uY;  ubo[14] = uZ;  ubo[15] = 0;
        // u4: fwd
        ubo[16] = fwdX; ubo[17] = fwdY; ubo[18] = fwdZ; ubo[19] = 0;
        // u5: mouse + shape exponent
        ubo[20] = mouse.ndcX;
        ubo[21] = mouse.ndcY;
        ubo[22] = mouse.active ? 1.0 : 0.0;
        ubo[23] = shapeExponent(shape.roundness);
        // ── ease formation.mix toward target (≈0.4 s settle) ─────
        {
          const k = 1.0 - Math.exp(-dt * 5.5);
          formation.mix += (formation.target - formation.mix) * k;
          // SNAP to exact endpoint once close enough — otherwise residual phase
          // (mix never reaches 1.0 via exp lerp) keeps every cube rotated by a
          // few degrees, breaking face-to-face alignment under tilted views.
          if (Math.abs(formation.target - formation.mix) < 0.002) {
            formation.mix = formation.target;
          }
        }
        // u6: formMix, formMode, paramA, paramB
        //   cube: paramA = grid side (full side³ solid),     paramB = scale
        //         interior cells hidden in shader → only 6s²−12s+8 visible.
        //   wall: paramA = cols,                              paramB = rows  (cols·rows ≤ N)
        let formModeId = 0, paramA = 1, paramB = 1.6;
        if (formation.mode === 'cube') {
          formModeId = 1;
          // side³ ≤ N → every cell of the solid grid has a particle.
          // Surface = 6s²−12s+8 visible; rest culled as interior.
          // Fix: When particle count is 1, side should be 1 to center it natively
          paramA = NUM_SPHERES <= 1 ? 1 : Math.max(2, Math.floor(Math.cbrt(NUM_SPHERES)));
          paramB = FORM_SCALE.cube;   // 1.8 — must match FORM_SCALE in state.rs
        } else if (formation.mode === 'wall') {
          formModeId = 2;
          // pick cols so that cols·rows ≤ N with aspect ≈ 1.6:1
          const cols = Math.max(1, Math.floor(Math.sqrt(NUM_SPHERES * 1.6)));
          const rows = Math.max(1, Math.floor(NUM_SPHERES / cols));
          paramA = cols;
          paramB = rows;
        }
        ubo[24] = formation.mix;
        ubo[25] = formModeId;
        ubo[26] = paramA;
        ubo[27] = paramB;
        // u7: cellSdfOn, cellRadius, colorMode, hideLow
        ubo[28] = cellSdf.on ? 1.0 : 0.0;
        ubo[29] = cellSdf.radius;
        ubo[30] = cellSdf.colorMode;
        ubo[31] = cellSdf.hideLow ? 1.0 : 0.0;
        // u8: object placement (scene)
        ubo[32] = sceneState.objectPosition[0];
        ubo[33] = sceneState.objectPosition[1];
        ubo[34] = sceneState.objectPosition[2];
        ubo[35] = 0;
        // u9: floor settings + camera projection + object selection
        ubo[36] = floorGrid.scale;
        ubo[37] = cam.ortho ? 1.0 : 0.0;
        ubo[38] = sceneState.selected ? 1.0 : 0.0; // u9.z = isSelected flag
        ubo[39] = sceneState.selectionMode; // u9.w = selectionMode (0=Object,1=Face,2=Edge,3=Vertex)
        // u10: object rotation (in degrees from UI, passed to shader)
        ubo[40] = sceneState.objectRotation[0];
        ubo[41] = sceneState.objectRotation[1];
        ubo[42] = sceneState.objectRotation[2];
        ubo[43] = 0;
        // u11: object scale (in XYZ)
        ubo[44] = sceneState.objectScale[0];
        ubo[45] = sceneState.objectScale[1];
        ubo[46] = sceneState.objectScale[2];
        ubo[47] = 0;
        
        // u12: object base dimensions (XYZ)
        ubo[48] = sceneState.baseMeshDim[0];
        ubo[49] = sceneState.baseMeshDim[1];
        ubo[50] = sceneState.baseMeshDim[2];
        ubo[51] = 0;

        // u13: Geometry properties (Bevel, Segments, Roundness)
        ubo[52] = sceneState.objectBevel;
        ubo[53] = sceneState.objectProfile;
        ubo[54] = sceneState.objectRoundness;
        ubo[55] = 0;

        // u14: Sketching Grids + Planes
        ubo[56] = sketchState.showGrid ? 1.0 : 0.0;
        let pId = 0.0;
        if (sketchState.plane === 'XY') pId = 1.0;
        else if (sketchState.plane === 'YZ') pId = 2.0;
        ubo[57] = pId;
        ubo[58] = 0;
        ubo[59] = 0;
        
        device.queue.writeBuffer(uniformBuf, 0, ubo);

        const enc  = device.createCommandEncoder();
        ensureDepth();
        const view      = gpuCtx.getCurrentTexture().createView();
        const depthView = depthTex.createView();

        {
          const pass = enc.beginRenderPass({
            colorAttachments: [{
              view, clearValue: { r: 0.02, g: 0.04, b: 0.09, a: 1 },
              loadOp: 'clear', storeOp: 'store',
            }],
            depthStencilAttachment: {
              view: depthView,
              depthClearValue: 1.0,
              depthLoadOp: 'clear',
              depthStoreOp: 'store',
            },
          });
          pass.setPipeline(bgPipeline);
          pass.setBindGroup(0, bindGroup);
          pass.draw(3);
          pass.end();
        }
        {
          const pass = enc.beginRenderPass({
            colorAttachments: [{
              view, loadOp: 'load', storeOp: 'store',
            }],
            depthStencilAttachment: {
              view: depthView,
              depthLoadOp: 'load',
              depthStoreOp: 'store',
            },
          });
          // ── Engine mode switch ─────────────────────────────────
          // PARTICLE mode: instanced billboards raymarched via SDF (full morph/cloud)
          // CAD mode:      rasterized cube mesh with clean Blender solid shading
          if (sceneState.engineMode === 'CAD') {
            pass.setPipeline(cadPipeline);
            pass.setBindGroup(0, bindGroup);
            if (cadIndexCount > 0) {
              pass.setVertexBuffer(0, cadPosBuf);
              pass.setVertexBuffer(1, cadNormalBuf);
              pass.setVertexBuffer(2, cadFaceIdBuf);
              pass.setIndexBuffer(cadIndexBuf, 'uint32');
              pass.drawIndexed(cadIndexCount);
            }
          } else {
            pass.setPipeline(spherePipeline);
            pass.setBindGroup(0, bindGroup);
            // 36 verts per instance: billboard quad uses first 6, cube mesh uses all 36.
            pass.draw(36, NUM_SPHERES);
          }
          pass.end();
        }

        device.queue.submit([enc.finish()]);
        
        // --- Sketch Canvas Overlay ---
        if (sceneState.selectionMode === 4 && sketchState && typeof sketchState.points !== 'undefined') {
          const sk = document.getElementById('sketch-canvas');
          if (!sk) return;
          // Sync canvas resolution to viewport
          if (sk.width !== canvas.width || sk.height !== canvas.height) {
            sk.width = canvas.width; sk.height = canvas.height;
          }
          const ctx = sk.getContext('2d');
          ctx.clearRect(0, 0, sk.width, sk.height);

          // ── World → Screen projection (matches WebGPU camera) ──
          function w2s(x, y, z) {
            const dx = x - roX, dy = y - roY, dz = z - roZ;
            const vwX = dx*rX  + dy*rY  + dz*rZ;
            const vwY = dx*uX  + dy*uY  + dz*uZ;
            const vwZ = dx*fwdX + dy*fwdY + dz*fwdZ;
            const asp = sk.width / sk.height;
            let ndcX, ndcY;
            if (cam.ortho) {
              if (vwZ < -50 || vwZ > 1000) return null;
              const oh = cam.dist * 0.45;
              ndcX = (vwX / oh) / asp;
              ndcY = (vwY / oh);
            } else {
              if (vwZ < 0.05) return null;
              const fL = 2.414;
              ndcX = (vwX * fL) / vwZ / asp;
              ndcY = (vwY * fL) / vwZ;
            }
            return {
              x: (ndcX + 1) * 0.5 * sk.width,
              y: (1 - ndcY) * 0.5 * sk.height,
            };
          }

          // ── Format distance (auto mm/cm/m) ──
          function fmtDist(m) {
            if (m < 0.01) return (m*1000).toFixed(1) + ' mm';
            if (m < 1.0)  return (m*100).toFixed(1) + ' cm';
            return m.toFixed(3) + ' m';
          }

          // ── Origin marker (small cross at 0,0,0) ──
          {
            const o = w2s(0,0,0);
            if (o) {
              ctx.strokeStyle = 'rgba(255,255,255,0.35)';
              ctx.lineWidth = 1;
              ctx.beginPath();
              ctx.moveTo(o.x-6, o.y); ctx.lineTo(o.x+6, o.y);
              ctx.moveTo(o.x, o.y-6); ctx.lineTo(o.x, o.y+6);
              ctx.stroke();
            }
          }

          const tool = (window.editorState && window.editorState.activeSketchTool) || 'line';
          const pts = sketchState.points;
          const hover = sketchState.hover;
          const plane = sketchState.plane;

          // ── Draw committed polyline ──
          if (pts.length > 0) {
            ctx.beginPath();
            let first = true;
            for (const p of pts) {
              const s = w2s(p.x, p.y, p.z);
              if (!s) continue;
              if (first) { ctx.moveTo(s.x, s.y); first = false; }
              else        ctx.lineTo(s.x, s.y);
            }
            if (sketchState.closed) {
              const f = w2s(pts[0].x, pts[0].y, pts[0].z);
              if (f) ctx.lineTo(f.x, f.y);
              ctx.fillStyle = 'rgba(56,189,248,0.15)';
              ctx.fill();
              ctx.strokeStyle = '#38bdf8';
              ctx.lineWidth = 2;
            } else {
              ctx.strokeStyle = '#cbd5e1';
              ctx.lineWidth = 1.5;
            }
            ctx.stroke();

            // Dimension labels on each segment
            ctx.font = '11px "JetBrains Mono", monospace';
            for (let i = 1; i < pts.length; i++) {
              const a = pts[i-1], b = pts[i];
              const d = Math.hypot(b.x-a.x, b.y-a.y, b.z-a.z);
              if (d < 0.005) continue;
              const sa = w2s(a.x,a.y,a.z), sb = w2s(b.x,b.y,b.z);
              if (!sa || !sb) continue;
              const mx = (sa.x+sb.x)/2, my = (sa.y+sb.y)/2;
              const dx = sb.x-sa.x, dy = sb.y-sa.y, L = Math.hypot(dx,dy) || 1;
              // Offset perpendicular
              const ox = -dy/L * 10, oy = dx/L * 10;
              const lbl = fmtDist(d);
              ctx.fillStyle = 'rgba(15,23,42,0.85)';
              const w = ctx.measureText(lbl).width + 6;
              ctx.fillRect(mx+ox-w/2, my+oy-8, w, 14);
              ctx.fillStyle = '#fbbf24';
              ctx.textAlign = 'center';
              ctx.textBaseline = 'middle';
              ctx.fillText(lbl, mx+ox, my+oy);
            }
          }

          // ── Point markers ──
          for (let i = 0; i < pts.length; i++) {
            const p = pts[i];
            const s = w2s(p.x,p.y,p.z);
            if (!s) continue;
            const isFirst = (i === 0 && pts.length > 2 && !sketchState.closed);
            const r = isFirst ? 6 : 4;
            ctx.beginPath();
            ctx.arc(s.x, s.y, r, 0, Math.PI*2);
            ctx.fillStyle = isFirst ? '#10b981' : '#38bdf8';
            ctx.fill();
            ctx.strokeStyle = '#0f172a';
            ctx.lineWidth = 1.5;
            ctx.stroke();
          }

          // ── Rubber-band preview to hover ──
          if (hover && !sketchState.closed && mouse.active) {
            const sh = w2s(hover.x, hover.y, hover.z);
            if (sh) {
              ctx.save();
              ctx.setLineDash([4, 4]);

              // Line tool: from last point to hover
              if (tool === 'line' && pts.length > 0) {
                const last = pts[pts.length-1];
                const sl = w2s(last.x,last.y,last.z);
                if (sl) {
                  ctx.beginPath();
                  ctx.moveTo(sl.x, sl.y);
                  ctx.lineTo(sh.x, sh.y);
                  ctx.strokeStyle = '#94a3b8';
                  ctx.lineWidth = 1.5;
                  ctx.stroke();
                  // Live dimension
                  const d = Math.hypot(hover.x-last.x, hover.y-last.y, hover.z-last.z);
                  if (d > 0.001) {
                    const mx = (sl.x+sh.x)/2, my = (sl.y+sh.y)/2;
                    const lbl = fmtDist(d);
                    ctx.setLineDash([]);
                    ctx.fillStyle = 'rgba(15,23,42,0.9)';
                    const w = ctx.measureText(lbl).width + 8;
                    ctx.fillRect(mx-w/2, my-18, w, 14);
                    ctx.fillStyle = '#fbbf24';
                    ctx.textAlign = 'center'; ctx.textBaseline = 'middle';
                    ctx.fillText(lbl, mx, my-11);
                  }
                }
              }

              // Rectangle tool: preview rect from pendingStart to hover
              if (tool === 'rectangle' && sketchState.pendingStart) {
                const s0 = sketchState.pendingStart;
                let corners;
                if (plane === 'XZ')      corners = [[s0.x,0,s0.z],[hover.x,0,s0.z],[hover.x,0,hover.z],[s0.x,0,hover.z]];
                else if (plane === 'XY') corners = [[s0.x,s0.y,0],[hover.x,s0.y,0],[hover.x,hover.y,0],[s0.x,hover.y,0]];
                else                     corners = [[0,s0.y,s0.z],[0,hover.y,s0.z],[0,hover.y,hover.z],[0,s0.y,hover.z]];
                ctx.beginPath();
                let first = true;
                for (const c of corners) {
                  const s = w2s(c[0],c[1],c[2]);
                  if (!s) continue;
                  if (first) { ctx.moveTo(s.x,s.y); first=false; } else ctx.lineTo(s.x,s.y);
                }
                ctx.closePath();
                ctx.strokeStyle = '#94a3b8';
                ctx.lineWidth = 1.5;
                ctx.stroke();
                ctx.fillStyle = 'rgba(148,163,184,0.10)';
                ctx.fill();
                // Width/Height labels
                let w_, h_;
                if (plane === 'XZ')      { w_ = Math.abs(hover.x-s0.x); h_ = Math.abs(hover.z-s0.z); }
                else if (plane === 'XY') { w_ = Math.abs(hover.x-s0.x); h_ = Math.abs(hover.y-s0.y); }
                else                     { w_ = Math.abs(hover.y-s0.y); h_ = Math.abs(hover.z-s0.z); }
                ctx.setLineDash([]);
                ctx.fillStyle = '#fbbf24';
                ctx.font = '11px "JetBrains Mono", monospace';
                ctx.textAlign = 'left'; ctx.textBaseline = 'top';
                ctx.fillText(`W ${fmtDist(w_)}  H ${fmtDist(h_)}`, sh.x+10, sh.y+10);
              }

              // Circle tool: preview circle from pendingStart center to hover radius
              if (tool === 'circle' && sketchState.pendingStart) {
                const c = sketchState.pendingStart;
                let dxp, dyp;
                if (plane === 'XZ')      { dxp = hover.x - c.x; dyp = hover.z - c.z; }
                else if (plane === 'XY') { dxp = hover.x - c.x; dyp = hover.y - c.y; }
                else                     { dxp = hover.y - c.y; dyp = hover.z - c.z; }
                const r = Math.hypot(dxp, dyp);
                const N = 48;
                ctx.beginPath();
                for (let i = 0; i <= N; i++) {
                  const a = (i / N) * Math.PI * 2;
                  const ca = Math.cos(a)*r, sa = Math.sin(a)*r;
                  let pt;
                  if (plane === 'XZ')      pt = w2s(c.x+ca, 0, c.z+sa);
                  else if (plane === 'XY') pt = w2s(c.x+ca, c.y+sa, 0);
                  else                     pt = w2s(0, c.y+ca, c.z+sa);
                  if (!pt) continue;
                  if (i === 0) ctx.moveTo(pt.x, pt.y); else ctx.lineTo(pt.x, pt.y);
                }
                ctx.strokeStyle = '#94a3b8';
                ctx.lineWidth = 1.5;
                ctx.stroke();
                ctx.fillStyle = 'rgba(148,163,184,0.10)';
                ctx.fill();
                ctx.setLineDash([]);
                ctx.fillStyle = '#fbbf24';
                ctx.font = '11px "JetBrains Mono", monospace';
                ctx.textAlign = 'left'; ctx.textBaseline = 'top';
                ctx.fillText(`R ${fmtDist(r)}  ⌀ ${fmtDist(r*2)}`, sh.x+10, sh.y+10);
              }
              ctx.restore();
            }
          }

          // ── Hover snap indicator ──
          if (hover && mouse.active) {
            const sh = w2s(hover.x, hover.y, hover.z);
            if (sh) {
              const snapColors = {
                grid:   '#38bdf8',
                point:  '#a78bfa',
                first:  '#10b981',
                origin: '#f472b6',
                free:   '#cbd5e1',
              };
              const col = snapColors[hover.snapType] || '#cbd5e1';
              ctx.strokeStyle = col;
              ctx.lineWidth = 1.5;
              ctx.beginPath();
              if (hover.snapType === 'point' || hover.snapType === 'first') {
                ctx.arc(sh.x, sh.y, 8, 0, Math.PI*2);
              } else if (hover.snapType === 'origin') {
                ctx.rect(sh.x-7, sh.y-7, 14, 14);
              } else {
                // small + cross
                ctx.moveTo(sh.x-7, sh.y); ctx.lineTo(sh.x+7, sh.y);
                ctx.moveTo(sh.x, sh.y-7); ctx.lineTo(sh.x, sh.y+7);
              }
              ctx.stroke();
              // Coordinate label
              const p = plane;
              let a='X', b='Z', av=hover.x, bv=hover.z;
              if (p === 'XY')      { a='X'; b='Y'; av=hover.x; bv=hover.y; }
              else if (p === 'YZ') { a='Y'; b='Z'; av=hover.y; bv=hover.z; }
              const lbl = `${a}:${av.toFixed(3)} ${b}:${bv.toFixed(3)}`;
              ctx.font = '10px "JetBrains Mono", monospace';
              ctx.fillStyle = 'rgba(15,23,42,0.85)';
              const w = ctx.measureText(lbl).width + 6;
              ctx.fillRect(sh.x+12, sh.y-6, w, 12);
              ctx.fillStyle = col;
              ctx.textAlign = 'left'; ctx.textBaseline = 'middle';
              ctx.fillText(lbl, sh.x+15, sh.y);
            }
          }
        }
        // -----------------------------

        // --- Extrude Preview Overlay (frontend-only, transparent prism) ---
        if (window.extrudePreview && window.extrudePreview.active && window.extrudePreview.points.length >= 3) {
          const sk = document.getElementById('sketch-canvas');
          if (sk) {
            if (sk.width !== canvas.width || sk.height !== canvas.height) {
              sk.width = canvas.width; sk.height = canvas.height;
            }
            const ctx = sk.getContext('2d');
            const ep = window.extrudePreview;
            // Re-use w2s closure pattern locally
            function w2s2(x, y, z) {
              const dx = x - roX, dy = y - roY, dz = z - roZ;
              const vwX = dx*rX  + dy*rY  + dz*rZ;
              const vwY = dx*uX  + dy*uY  + dz*uZ;
              const vwZ = dx*fwdX + dy*fwdY + dz*fwdZ;
              const asp = sk.width / sk.height;
              let ndcX, ndcY;
              if (cam.ortho) {
                if (vwZ < -50 || vwZ > 1000) return null;
                const oh = cam.dist * 0.45;
                ndcX = (vwX / oh) / asp;
                ndcY = (vwY / oh);
              } else {
                if (vwZ < 0.05) return null;
                const fL = 2.414;
                ndcX = (vwX * fL) / vwZ / asp;
                ndcY = (vwY * fL) / vwZ;
              }
              return { x:(ndcX+1)*0.5*sk.width, y:(1-ndcY)*0.5*sk.height };
            }
            const base = ep.points;
            const dx = ep.direction[0] * ep.distance;
            const dy = ep.direction[1] * ep.distance;
            const dz = ep.direction[2] * ep.distance;
            const top = base.map(p => ({ x:p.x+dx, y:p.y+dy, z:p.z+dz }));

            // Side walls (translucent fill)
            ctx.save();
            ctx.fillStyle   = 'rgba(167, 139, 250, 0.18)';
            ctx.strokeStyle = 'rgba(167, 139, 250, 0.6)';
            ctx.lineWidth = 1;
            for (let i = 0; i < base.length; i++) {
              const j = (i + 1) % base.length;
              const a = w2s2(base[i].x, base[i].y, base[i].z);
              const b = w2s2(base[j].x, base[j].y, base[j].z);
              const c = w2s2(top[j].x,  top[j].y,  top[j].z);
              const d = w2s2(top[i].x,  top[i].y,  top[i].z);
              if (!a || !b || !c || !d) continue;
              ctx.beginPath();
              ctx.moveTo(a.x, a.y);
              ctx.lineTo(b.x, b.y);
              ctx.lineTo(c.x, c.y);
              ctx.lineTo(d.x, d.y);
              ctx.closePath();
              ctx.fill();
              ctx.stroke();
            }
            // Top cap outline
            ctx.beginPath();
            for (let i = 0; i < top.length; i++) {
              const s = w2s2(top[i].x, top[i].y, top[i].z);
              if (!s) continue;
              if (i === 0) ctx.moveTo(s.x, s.y); else ctx.lineTo(s.x, s.y);
            }
            ctx.closePath();
            ctx.fillStyle = 'rgba(167, 139, 250, 0.25)';
            ctx.strokeStyle = '#a78bfa';
            ctx.lineWidth = 1.5;
            ctx.fill();
            ctx.stroke();
            // Distance label at first edge midpoint
            const a0 = w2s2(base[0].x, base[0].y, base[0].z);
            const t0 = w2s2(top[0].x,  top[0].y,  top[0].z);
            if (a0 && t0) {
              const mx = (a0.x+t0.x)/2, my = (a0.y+t0.y)/2;
              const lbl = `↥ ${ep.distance < 1 ? (ep.distance*100).toFixed(1)+' cm' : ep.distance.toFixed(3)+' m'}`;
              ctx.font = '11px "JetBrains Mono", monospace';
              ctx.fillStyle = 'rgba(15,23,42,0.9)';
              const w = ctx.measureText(lbl).width + 8;
              ctx.fillRect(mx+8, my-7, w, 14);
              ctx.fillStyle = '#a78bfa';
              ctx.textAlign = 'left'; ctx.textBaseline = 'middle';
              ctx.fillText(lbl, mx+12, my);
            }
            ctx.restore();
          }
        }

        gpuRafId = requestAnimationFrame(frame);
      }

      gpuRafId = requestAnimationFrame(frame);
      console.log('%c[WebGPU] 🚀 v1.4 5M particles', 'color:#67e8f9;font-weight:bold');
      setBadge(`✓ WebGPU · ${fmtN(NUM_SPHERES)} particles`, '#34d399');

      // ── 8. Cleanup ──────────────────────────────────────────────
      document.getElementById('close-chefos')?.addEventListener('click', () => {
        if (gpuRafId) { cancelAnimationFrame(gpuRafId); gpuRafId = null; }
        document.body.classList.remove('gpu-active');
        if (grad) grad.style.visibility = '';
        if (grid) grid.style.visibility = '';
        gpuCtx.unconfigure?.();
        window.removeEventListener('resize', resizeCanvas);
        window.removeEventListener('keydown', onKey);
        hud?.remove();
      }, { once: true });
    }
"##;
