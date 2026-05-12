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
      // True when the current pointer-down landed on a gizmo handle.
      // Prevents pointerup from firing __handleSketchClick (which confirms grab).
      let gizmoHandleDrag = false;

      window.__isPointerDragging = () => dragging;

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

      // ── Gizmo handle hit test ─────────────────────────────────
      function __hitGizmoHandle(px, py) {
        const handles = window.__gizmoHandles;
        if (!handles) return null;
        for (const h of handles) {
          if (Math.hypot(px - h.x, py - h.y) <= h.r + 4) return h.axis;
        }
        return null;
      }

      // ── Pointer down ─────────────────────────────────────────────
      canvas.addEventListener('pointerdown', (e) => {
        canvas.setPointerCapture(e.pointerId);
        dragging       = true;
        dragMoved      = false;
        gizmoHandleDrag = false;

        const isGrab = sketchState.grab?.active;
        const isCopy = sketchState.copy?.active;

        if ((isGrab || isCopy) && e.button === 0) {
          const rect2  = canvas.getBoundingClientRect();
          const hitAxis = __hitGizmoHandle(e.clientX - rect2.left, e.clientY - rect2.top);
          const tState = isGrab ? sketchState.grab : sketchState.copy;

          tState.axisLock = hitAxis || null;

          if (sketchState.hoverWorld) {
            tState.startMouseWorld = { x: sketchState.hoverWorld.x, y: sketchState.hoverWorld.y, z: sketchState.hoverWorld.z };
          }
          if (sketchState.precision?.lastMouseScreen) {
             tState.startScreen = { x: sketchState.precision.lastMouseScreen.x, y: sketchState.precision.lastMouseScreen.y };
          }

          if (isGrab) {
            const byId = new Map(sketchState.points.map(p => [p.id, p]));
            tState.dragBase = new Map();
            for (const id of tState.pointIds) {
              const p = byId.get(id);
              if (p) tState.dragBase.set(id, { x: p.x, y: p.y, z: p.z });
            }
          } else {
            tState.baseDelta = { dx: tState.delta.dx, dy: tState.delta.dy, dz: tState.delta.dz };
          }

          gizmoHandleDrag = true;
          window.__hitGizmoOnDown = (hitAxis !== null);

          panning  = false;
          orbiting = false;
          lastX = e.clientX; lastY = e.clientY;
          startX = e.clientX; startY = e.clientY;
          return;
        }

        const wantsPan = (e.button === 2) || (e.button === 1)
                       || (e.shiftKey && !e.metaKey && !e.ctrlKey && e.button === 0)
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
        const wasGizmo = gizmoHandleDrag;
        gizmoHandleDrag = false;
        try { canvas.releasePointerCapture(e.pointerId); } catch {}
        const dist = Math.hypot(e.clientX - startX, e.clientY - startY);
        startX = 0; startY = 0;

        if (wasGizmo) {
          // If they clicked empty space without dragging, confirm grab/copy.
          if (!dragMoved && !window.__hitGizmoOnDown) {
            const rect = canvas.getBoundingClientRect();
            const ndcX = ((e.clientX - rect.left) / rect.width)  * 2 - 1;
            const ndcY = 1 - ((e.clientY - rect.top)  / rect.height) * 2;
            if (window.__handleSketchClick) window.__handleSketchClick(ndcX, ndcY, e.shiftKey || e.metaKey || e.ctrlKey);
          }
          panning = false; orbiting = false;
          return;
        }

        // Suppress click-confirm when the down-event was on a gizmo handle.
        if (!wasGizmo && wasDragging && e.button === 0 && dist < CLICK_THRESH_PX && !panning && !dragMoved) {
          const rect = canvas.getBoundingClientRect();
          const ndcX = ((e.clientX - rect.left) / rect.width)  * 2 - 1;
          const ndcY = 1 - ((e.clientY - rect.top)  / rect.height) * 2;
          if (window.__handleSketchClick) window.__handleSketchClick(ndcX, ndcY, e.shiftKey || e.metaKey || e.ctrlKey);
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
        // Grab / CopyConnect update: pass hit if available, else pass the last
        // known hoverWorld so off-plane axis-lock still gets screen-delta updates.
        const grabTarget = hit || sketchState.hoverWorld;
        if (sketchState.grab.active)  window.__updateGrab(grabTarget);
        if (sketchState.copy.active)  window.__updateCopyConnect(grabTarget);

        // ── Gizmo handle hover (updates cursor + highlight) ──
        if (sketchState.grab.active) {
          const rect3 = canvas.getBoundingClientRect();
          const hAxis = __hitGizmoHandle(e.clientX - rect3.left, e.clientY - rect3.top);
          window.__gizmoHoverAxis = hAxis || null;
          canvas.style.cursor = hAxis ? 'grab' : 'crosshair';
        } else {
          window.__gizmoHoverAxis = null;
        }
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
