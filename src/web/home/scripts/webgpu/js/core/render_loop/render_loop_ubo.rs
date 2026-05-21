// ── Part 1: Frame init, camera math, UBO upload, WebGPU render passes ─────────

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
        // ── Solid selection state → UBO ─────────────────────────────────────
        // u9.z = isSelected (1.0 = yes), u9.w = selectionMode (0=object,1=face)
        // u15.x = selected_face_id (source face_id from kernel), u15.y = hovered_face_id
        ubo[38] = (window.__solidSelected   ? 1.0 : 0.0);
        ubo[39] = (window.__solidSelMode    || 0);
        ubo[60] = (window.__solidSelFaceId  || 0);
        ubo[61] = (window.__solidHoverFaceId|| 0);
        ubo[62] = 0; ubo[63] = 0;
        device.queue.writeBuffer(uniformBuf, 0, ubo);

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
        if (cadIndexCount > 0 && (window.__debugSolidRender ? window.__debugSolidRender.drawSolid : true)) {
          const cadPass = enc.beginRenderPass({
            colorAttachments: [{ view, loadOp: 'load', storeOp: 'store' }],
            depthStencilAttachment: { view: depthView, depthClearValue: 1.0, depthLoadOp: 'load', depthStoreOp: 'store' },
          });
          cadPass.setPipeline(cadPipeline);
          cadPass.setBindGroup(0, bindGroup);
          cadPass.setVertexBuffer(0, cadPosBuf);
          cadPass.setVertexBuffer(1, cadNormalBuf);
          cadPass.setVertexBuffer(2, cadFaceIdBuf);
          cadPass.setIndexBuffer(cadIndexBuf, 'uint32');
          cadPass.drawIndexed(cadIndexCount);
          cadPass.end();
        }
        device.queue.submit([enc.finish()]);
        if (window.__perfSample) window.__perfSample('render', performance.now() - __pfRender);
"##;
