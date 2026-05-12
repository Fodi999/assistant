// ── JS: Mouse input ───────────────────────────────────────────────────────────
// Domain: User input — pointerdown/up/move/dblclick/leave + click dispatch.
// Handles:
//   - Orbit  : plain left drag
//   - Pan    : right / middle / Shift+left / Space+left
//   - Click  : fires __handleSketchClick if drag distance < CLICK_THRESH_PX
//   - DblClick: fires __handleSketchDoubleClick
//   - Hover  : updates mouse.ndcX/Y, sketchState.hoverPointId/EdgeId/ProfileId
//   - Snap   : classifies cursor hit as 'point' | 'grid' | 'none'
//   - Grab   : propagates drag to __updateGrab when sketchState.grab.active
//   - Cursor : sets crosshair / move / not-allowed per active sketch tool

pub const JS: &str = r##"
      const CLICK_THRESH_PX = 5;
      let dragging  = false, panning = false, orbiting = false;
      let lastX = 0, lastY = 0, startX = 0, startY = 0;
      let dragMoved = false;

      const mouse = { ndcX: 999, ndcY: 999, active: false };

      function __setCursorForTool() {
        const t = sketchState.activeTool;
        let cur = 'default';
        if      (t === 'point' || t === 'line') cur = 'crosshair';
        else if (t === 'grab')                  cur = 'move';
        else if (t === 'delete')                cur = 'not-allowed';
        canvas.style.cursor = cur;
      }
      window.__setCursorForTool = __setCursorForTool;

      // ── Pointer down ─────────────────────────────────────────────
      canvas.addEventListener('pointerdown', (e) => {
        canvas.setPointerCapture(e.pointerId);
        dragging  = true;
        dragMoved = false;
        const wantsPan = (e.button === 2) || (e.button === 1)
                       || (e.shiftKey && e.button === 0)
                       || (spaceHeld   && e.button === 0);
        panning  = wantsPan;
        orbiting = !wantsPan;
        lastX = e.clientX; lastY = e.clientY;
        startX = e.clientX; startY = e.clientY;
      });

      // ── Pointer up / click ───────────────────────────────────────
      canvas.addEventListener('pointerup', (e) => {
        const wasDragging = dragging;
        dragging = false;
        try { canvas.releasePointerCapture(e.pointerId); } catch {}
        const dist = Math.hypot(e.clientX - startX, e.clientY - startY);
        startX = 0; startY = 0;
        if (wasDragging && e.button === 0 && dist < CLICK_THRESH_PX && !panning && !dragMoved) {
          const rect = canvas.getBoundingClientRect();
          const ndcX = ((e.clientX - rect.left) / rect.width)  * 2 - 1;
          const ndcY = 1 - ((e.clientY - rect.top)  / rect.height) * 2;
          if (window.__handleSketchClick) window.__handleSketchClick(ndcX, ndcY, e.shiftKey);
        }
        panning  = false;
        orbiting = false;
      });

      // ── Double-click ─────────────────────────────────────────────
      canvas.addEventListener('dblclick', (e) => {
        const rect = canvas.getBoundingClientRect();
        const ndcX = ((e.clientX - rect.left) / rect.width)  * 2 - 1;
        const ndcY = 1 - ((e.clientY - rect.top)  / rect.height) * 2;
        if (window.__handleSketchDoubleClick) window.__handleSketchDoubleClick(ndcX, ndcY);
      });

      // ── Pointer move — hover, snap, orbit, pan ───────────────────
      canvas.addEventListener('pointermove', (e) => {
        const rect = canvas.getBoundingClientRect();
        mouse.ndcX = ((e.clientX - rect.left) / rect.width)  * 2 - 1;
        mouse.ndcY = 1 - ((e.clientY - rect.top)  / rect.height) * 2;
        mouse.active = true;

        // Hover + snap (always, even without dragging).
        const __pfPick = performance.now();
        const hit = window.__raycastSketchPlane && window.__raycastSketchPlane(mouse.ndcX, mouse.ndcY);

        if (hit && window.__resolveSnapTarget) {
          // Convert NDC → screen px for snap radius checks.
          const mpx = {
            x: (mouse.ndcX + 1) * 0.5 * canvas.width,
            y: (1 - mouse.ndcY) * 0.5 * canvas.height,
          };
          const free   = { x: hit.freeX, y: hit.freeY, z: hit.freeZ };
          const target = window.__resolveSnapTarget(free, mpx, null);
          // Populate hoverWorld with the snapped position (not raw).
          // screenX/Y = actual cursor tip on canvas — used by render_loop
          // so the crosshair is drawn exactly under the pointer.
          sketchState.hoverWorld = {
            x: target.x, y: target.y, z: target.z,
            gx: target.gx, gy: target.gy, gz: target.gz,
            freeX: hit.freeX, freeY: hit.freeY, freeZ: hit.freeZ,
            snapKind: target.kind, pointId: target.pointId,
            screenX: mpx.x, screenY: mpx.y,
          };
          // Snap status for HUD / mini-bar.
          sketchState.snap = {
            kind: target.kind,
            pointId: target.pointId,
            gx: target.gx, gy: target.gy, gz: target.gz,
          };
        } else {
          sketchState.hoverWorld = null;
          sketchState.snap = { kind: 'none', pointId: null, gx: 0, gy: 0, gz: 0 };
        }

        const tool = sketchState.activeTool;
        if (tool === 'select' || tool === 'delete') {
          sketchState.hoverPointId = window.__pickPointAt(mouse.ndcX, mouse.ndcY);
          sketchState.hoverEdgeId  = sketchState.hoverPointId
            ? null : window.__pickEdgeAt(mouse.ndcX, mouse.ndcY);
          if (tool === 'select' && !sketchState.hoverPointId && !sketchState.hoverEdgeId && hit) {
            sketchState.hoverProfileId = window.__pickProfileAtWorld(hit.freeX, hit.freeY, hit.freeZ);
          } else {
            sketchState.hoverProfileId = null;
          }
        } else if (tool === 'line') {
          sketchState.hoverPointId  = window.__pickPointAt(mouse.ndcX, mouse.ndcY);
          sketchState.hoverEdgeId   = null;
          sketchState.hoverProfileId = null;
        } else {
          sketchState.hoverPointId  = null;
          sketchState.hoverEdgeId   = null;
          sketchState.hoverProfileId = null;
        }

        if (window.__updateLinePreview) window.__updateLinePreview();
        if (sketchState.grab.active && hit) window.__updateGrab(hit);
        if (window.__perfSample) window.__perfSample('pick', performance.now() - __pfPick);

        if (!dragging) return;

        const dx = e.clientX - lastX;
        const dy = e.clientY - lastY;
        lastX = e.clientX; lastY = e.clientY;
        if (Math.hypot(e.clientX - startX, e.clientY - startY) >= CLICK_THRESH_PX) dragMoved = true;

        if (panning) {
          const k  = cam.dist * 0.0015;
          const cy = Math.cos(cam.yaw),  sy = Math.sin(cam.yaw);
          const cp = Math.cos(cam.pitch);
          const fwdX = -sy*cp, fwdY = -Math.sin(cam.pitch), fwdZ = cy*cp;
          let rxv = -fwdZ, ryv = 0, rzv = fwdX;
          const rL = Math.hypot(rxv, ryv, rzv) || 1; rxv /= rL; rzv /= rL;
          const uxv = ryv*fwdZ - rzv*fwdY;
          const uyv = rzv*fwdX - rxv*fwdZ;
          const uzv = rxv*fwdY - ryv*fwdX;
          cam.target[0] -= (dx*rxv - dy*uxv) * k;
          cam.target[1] -= (dx*ryv - dy*uyv) * k;
          cam.target[2] -= (dx*rzv - dy*uzv) * k;
        } else if (orbiting) {
          cam.yaw   += dx * 0.005;
          cam.pitch += dy * 0.005;
          cam.pitch = Math.max(-Math.PI/2 + 0.05, Math.min(Math.PI/2 - 0.05, cam.pitch));
        }
      });

      canvas.addEventListener('pointerleave', () => { mouse.active = false; });
      canvas.addEventListener('contextmenu',  (e) => e.preventDefault());
"##;
