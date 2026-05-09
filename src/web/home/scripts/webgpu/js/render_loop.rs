// ── JS: render loop, UBO upload, draw calls, cleanup ─────────────────────────────
// Domain: Rendering — per-frame camera math, UBO upload, render passes, RAF.

pub const JS: &str = r##"
      // ── 7. Render loop ──────────────────────────────────────────
      const t0 = performance.now();
      let frameCount = 0;
      let lastFpsTime = t0, fpsAcc = 0, fps = 0;
      let lastFrameTime = t0;
      const ubo = new Float32Array(40); // 10 × vec4

      function frame() {
        const now = performance.now();
        const t   = (now - t0) / 1000.0;
        const dt  = (now - lastFrameTime) / 1000.0;
        const cpuFrameMs = now - lastFrameTime;
        lastFrameTime = now;
        if (frameCount === 0) {
          log('🚀 render loop запущен!', '#67e8f9');
          log('🖱  drag · wheel · 1-5 · C/V/W form · [/] shape · R · B=HUD · Shift+B=bench', '#a78bfa');
          setTimeout(() => { if (diag) diag.style.display = 'none'; }, 4000);
        }
        frameCount++;
        fpsAcc++;
        if (now - lastFpsTime >= 500) {
          fps = fpsAcc * 1000 / (now - lastFpsTime);
          lastFpsTime = now; fpsAcc = 0;
          // update legacy HUD only when it is visible (toggled by B)
          const hudEl = document.getElementById('gpu-hud');
          if (!bench.running && hudEl && hudEl.style.display !== 'none') updateHud(fps);
          if (globalThis.__matterPerf) {
            globalThis.__matterPerf.fps     = fps;
            globalThis.__matterPerf.frameMs = cpuFrameMs;
          }
        }

        // benchmark tick (collect frame times)
        if (bench.running) benchTick(cpuFrameMs);

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
        ubo[35] = sceneState.objectScale;
        // u9: floor settings + camera projection
        ubo[36] = floorGrid.scale;
        ubo[37] = cam.ortho ? 1.0 : 0.0;
        ubo[38] = 0;
        ubo[39] = 0;
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
          pass.setPipeline(spherePipeline);
          pass.setBindGroup(0, bindGroup);
          // 36 verts per instance: enough for either billboard quad (uses first 6,
          // discards 6..35) or cube mesh (uses all 36 for 6 axis-aligned faces).
          pass.draw(36, NUM_SPHERES);
          pass.end();
        }

        device.queue.submit([enc.finish()]);
        gpuRafId = requestAnimationFrame(frame);
      }

      gpuRafId = requestAnimationFrame(frame);
      // expose bench controls so HUD inline onclick can reach them
      window.__gpuBench      = bench;
      window.__gpuRunBench   = runBenchmark;
      console.log('%c[WebGPU] 🚀 v1.4 5M particles + benchmark', 'color:#67e8f9;font-weight:bold');
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
        document.getElementById('bench-overlay')?.remove();
        hud?.remove();
        delete window.__gpuBench;
        delete window.__gpuRunBench;
      }, { once: true });
    }
"##;
