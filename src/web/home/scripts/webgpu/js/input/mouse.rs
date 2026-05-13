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
      // Separate tracking for grab movement — works even without button press (touchpad hover).
      let grabLastX = -1, grabLastY = -1;
      // True when the current pointer-down landed on a gizmo handle.
      // Prevents pointerup from firing __handleSketchClick (which confirms grab).
      let gizmoHandleDrag = false;

      window.__isPointerDragging = () => dragging;
      window.__grabIsScreenProjection = false;
      window.__resetGrabTracking = function() { grabLastX = -1; grabLastY = -1; };

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
          if (Math.hypot(px - h.x, py - h.y) <= h.r + 8) return h.axis;
        }
        return null;
      }

      // ── Pointer down ─────────────────────────────────────────────
      canvas.addEventListener('pointerdown', (e) => {
        canvas.setPointerCapture(e.pointerId);
        dragging       = true;
        dragMoved      = false;
        gizmoHandleDrag = false;

        const rect2   = canvas.getBoundingClientRect();
        const px      = e.clientX - rect2.left;
        const py      = e.clientY - rect2.top;
        const hitAxis = __hitGizmoHandle(px, py);

        const isGrab = sketchState.grab?.active;
        const isCopy = sketchState.copy?.active;

        // ── Gizmo handle click: start or re-configure grab ──
        if (hitAxis !== null && e.button === 0 && !isCopy) {
          gizmoHandleDrag = true;
          window.__hitGizmoOnDown = true;

          if (isGrab) {
            // Already grabbing — just change axis lock + reset base
            const tState = sketchState.grab;
            tState.axisLock = (hitAxis === 'FREE') ? null : hitAxis;
            tState.screenAcc = { x: 0, y: 0, z: 0 };  // reset accumulator on re-lock
            if (sketchState.hoverWorld) {
              tState.startMouseWorld = { x: sketchState.hoverWorld.x, y: sketchState.hoverWorld.y, z: sketchState.hoverWorld.z };
            }
            if (sketchState.precision?.lastMouseScreen) {
              tState.startScreen = { x: sketchState.precision.lastMouseScreen.x, y: sketchState.precision.lastMouseScreen.y };
            }
            const byId = new Map(sketchState.points.map(p => [p.id, p]));
            tState.dragBase = new Map();
            for (const id of tState.pointIds) {
              const p = byId.get(id);
              if (p) tState.dragBase.set(id, { x: p.x, y: p.y, z: p.z });
            }
          } else {
            // Not yet grabbing — start grab from selection via gizmo
            if (window.__startGrabFromGizmo) {
              window.__startGrabFromGizmo(hitAxis, e.clientX, e.clientY);
            }
          }

          panning  = false;
          orbiting = false;
          lastX = e.clientX; lastY = e.clientY;
          startX = e.clientX; startY = e.clientY;
          return;
        }

        if (isCopy && e.button === 0) {
          const tState = sketchState.copy;
          const hitAxisC = __hitGizmoHandle(px, py);
          tState.axisLock = hitAxisC || null;
          if (sketchState.hoverWorld) {
            tState.startMouseWorld = { x: sketchState.hoverWorld.x, y: sketchState.hoverWorld.y, z: sketchState.hoverWorld.z };
          }
          if (sketchState.precision?.lastMouseScreen) {
            tState.startScreen = { x: sketchState.precision.lastMouseScreen.x, y: sketchState.precision.lastMouseScreen.y };
          }
          tState.baseDelta = { dx: tState.delta.dx, dy: tState.delta.dy, dz: tState.delta.dz };
          if (hitAxisC !== null) {
            gizmoHandleDrag = true;
            window.__hitGizmoOnDown = true;
            panning  = false;
            orbiting = false;
            lastX = e.clientX; lastY = e.clientY;
            startX = e.clientX; startY = e.clientY;
            return;
          }
        }

        // If grab is active — no orbiting/panning, just drag to move points
        if (sketchState.grab?.active && e.button === 0) {
          panning  = false;
          orbiting = false;
          lastX = e.clientX; lastY = e.clientY;
          startX = e.clientX; startY = e.clientY;
          return;
        }

        const wantsPan = (e.button === 2) || (e.button === 1)
                       || (e.shiftKey && !e.metaKey && !e.ctrlKey && e.button === 0)
                       || (spaceHeld   && e.button === 0);
        // Drawing tools (line / point / delete) must not start an orbit on left click —
        // the click is for geometry creation, not camera movement.
        const drawingTool = ['line', 'point', 'delete'].includes(sketchState.activeTool);
        panning  = wantsPan;
        orbiting = !wantsPan && !drawingTool;
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
          // Reset the one-shot flag so future empty-canvas clicks work normally.
          window.__hitGizmoOnDown = false;

          // If they dragged from a gizmo handle — keep grab active (don't confirm).
          // User confirms with Enter or by clicking empty canvas.
          if (dragMoved && sketchState.grab?.active) {
            // Just update status — do NOT confirm, let user keep dragging other axes.
            if (window.__setStatusMessage) {
              const lock = sketchState.grab.axisLock;
              window.__setStatusMessage('⤢ Grab' + (lock ? ' · ' + lock : ' · free') + ' — drag again · Enter ✓ · Esc ✗');
            }
          } else if (!dragMoved) {
            // Pure tap on handle: axis was locked, show status.
            if (sketchState.grab?.active && window.__setStatusMessage) {
              const lock = sketchState.grab.axisLock;
              window.__setStatusMessage('⤢ Grab ready — drag to move' + (lock ? ' · ' + lock : ' · free') + ' · Enter ✓ · Esc ✗');
            }
          }
          panning = false; orbiting = false;
          return;
        }

        // Drawing tools (line / point) use a larger click threshold and ignore
        // dragMoved — touchpad micro-movement must not suppress geometry creation.
        const drawingClick = ['line', 'point'].includes(sketchState.activeTool);
        const clickThresh  = drawingClick ? 18 : CLICK_THRESH_PX;
        const clickOk      = drawingClick
          ? (dist < clickThresh && !panning)
          : (dist < clickThresh && !panning && !dragMoved);

        // Suppress click-confirm when the down-event was on a gizmo handle.
        if (!wasGizmo && wasDragging && e.button === 0 && clickOk) {
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
        const grabTarget = hit || sketchState.hoverWorld;
        if (sketchState.copy.active) window.__updateCopyConnect(grabTarget);

        // ── Gizmo handle hover (works on selection AND during grab) ──
        {
          const rect3 = canvas.getBoundingClientRect();
          const hAxis = __hitGizmoHandle(e.clientX - rect3.left, e.clientY - rect3.top);
          window.__gizmoHoverAxis = hAxis || null;
          if (hAxis) {
            canvas.style.cursor = 'grab';
          } else if (sketchState.grab.active) {
            canvas.style.cursor = 'crosshair';
          } else {
            __setCursorForTool();
          }
        }
        if (window.__perfSample) window.__perfSample('pick', performance.now() - __pfPick);

        // ── Grab movement — independent of dragging/buttons ───────────────────
        // Works with touchpad hover (e.buttons===0) and regular mouse drag alike.
        if (sketchState.grab?.active) {
          const useProj = gizmoHandleDrag || window.__grabIsScreenProjection;

          // Initialise tracking on first call after grab started.
          if (grabLastX < 0) { grabLastX = e.clientX; grabLastY = e.clientY; }
          const gdx = e.clientX - grabLastX;
          const gdy = e.clientY - grabLastY;
          grabLastX = e.clientX; grabLastY = e.clientY;

          if (useProj && (gdx !== 0 || gdy !== 0)) {
            const grab3 = sketchState.grab;
            const lock3 = grab3.axisLock || null;
            let wdx = 0, wdy = 0, wdz = 0;

            const axDirs = window.__gizmoAxisScreenDirs;
            if ((lock3 === 'X' || lock3 === 'Y' || lock3 === 'Z') && axDirs && axDirs[lock3]) {
              // dot(mouseDeltaPx, axisScreenDir) / pxPerUnit → world delta along axis
              const ad = axDirs[lock3];
              const pxSq = ad.pxPerUnit * ad.pxPerUnit || 1;
              const worldDelta = (gdx * ad.dx + gdy * ad.dy) / pxSq;
              if (lock3 === 'X') wdx = worldDelta;
              else if (lock3 === 'Y') wdy = worldDelta;
              else                    wdz = worldDelta;
            } else {
              // Camera basis decomposition for plane locks and FREE
              const k3  = cam.dist / canvas.height;
              const cY3 = cam.yaw, cP3 = cam.pitch;
              const rX3 = Math.cos(cY3), rZ3 = Math.sin(cY3);
              const uX3 =  Math.sin(cY3) * Math.sin(cP3);
              const uY3 =  Math.cos(cP3);
              const uZ3 = -Math.cos(cY3) * Math.sin(cP3);
              wdx = (gdx * rX3 - gdy * uX3) * k3;
              wdy = (gdx * 0   - gdy * uY3) * k3;
              wdz = (gdx * rZ3 - gdy * uZ3) * k3;
              if      (lock3 === 'XY') { wdz = 0; }
              else if (lock3 === 'XZ') { wdy = 0; }
              else if (lock3 === 'YZ') { wdx = 0; }
              // FREE: no zeroing
            }

            if (!grab3.screenAcc) grab3.screenAcc = { x: 0, y: 0, z: 0 };
            grab3.screenAcc.x += wdx;
            grab3.screenAcc.y += wdy;
            grab3.screenAcc.z += wdz;

            const g3 = sketchState.gridSize || 1.0;
            const fdx = Math.round(grab3.screenAcc.x / g3) * g3;
            const fdy = Math.round(grab3.screenAcc.y / g3) * g3;
            const fdz = Math.round(grab3.screenAcc.z / g3) * g3;

            const byId3 = new Map(sketchState.points.map(p => [p.id, p]));
            for (const id3 of grab3.pointIds) {
              const base3 = grab3.dragBase.get(id3);
              const p3    = byId3.get(id3);
              if (!base3 || !p3) continue;
              p3.x = base3.x + fdx;
              p3.y = base3.y + fdy;
              p3.z = base3.z + fdz;
              const g4 = sketchState.gridSize || 1.0;
              p3.gx = Math.round(p3.x / g4);
              p3.gy = Math.round(p3.y / g4);
              p3.gz = Math.round(p3.z / g4);
            }
          } else if (!useProj) {
            window.__updateGrab(grabTarget);
          }

          // Mark drag moved if pointer has travelled enough.
          if (Math.hypot(e.clientX - startX, e.clientY - startY) >= CLICK_THRESH_PX) dragMoved = true;
          return; // skip orbit / pan
        }

        // No grab active — reset grab tracking so next grab starts clean.
        grabLastX = -1; grabLastY = -1;

        if (!dragging) return;

        const dx = e.clientX - lastX;
        const dy = e.clientY - lastY;
        lastX = e.clientX; lastY = e.clientY;
        // Only mark drag as "moved" when actually orbiting or panning — not on
        // drawing tools (line / point / delete) where a tiny touchpad wobble
        // must not suppress the click that creates geometry.
        if ((orbiting || panning) && Math.hypot(e.clientX - startX, e.clientY - startY) >= CLICK_THRESH_PX) dragMoved = true;

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
