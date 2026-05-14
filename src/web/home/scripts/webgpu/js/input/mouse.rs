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
            // Already grabbing — change axis lock and reset transform base
            const tState = sketchState.grab;
            tState.axisLock = (hitAxis === 'FREE') ? null : hitAxis;
            tState.screenAcc = { x: 0, y: 0, z: 0 };
            if (sketchState.hoverWorld) {
              tState.startMouseWorld = { x: sketchState.hoverWorld.x, y: sketchState.hoverWorld.y, z: sketchState.hoverWorld.z };
            }
            // Re-anchor startScreen to gizmo center (canvas device-px) on re-lock
            const _ctrRL = window.__gizmoCenterScreen;
            const _lmsRL = sketchState.precision?.lastMouseScreen;
            tState.startScreen = _ctrRL
              ? { x: _ctrRL.x, y: _ctrRL.y }
              : _lmsRL ? { x: _lmsRL.x, y: _lmsRL.y } : tState.startScreen;
            // Reset drag base to current point positions (re-lock starts from here)
            const byId = new Map(sketchState.points.map(p => [p.id, p]));
            tState.dragBase = new Map();
            let _rcx=0,_rcy=0,_rcz=0,_rcn=0;
            for (const id of tState.pointIds) {
              const p = byId.get(id);
              if (p) {
                tState.dragBase.set(id, { x: p.x, y: p.y, z: p.z });
                _rcx+=p.x; _rcy+=p.y; _rcz+=p.z; _rcn++;
              }
            }
            // Re-anchor drag plane to current center + reset start drag point
            tState.startCenter = _rcn ? { x:_rcx/_rcn, y:_rcy/_rcn, z:_rcz/_rcn } : tState.startCenter;
            tState.startDragPoint = null; // will be set on next __updateGizmoDrag call
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

        // Always update lastMouseScreen — needed by Copy Connect even off sketch plane
        if (!sketchState.precision) sketchState.precision = {};
        sketchState.precision.lastMouseScreen = {
          x: e.clientX - rect.left,
          y: e.clientY - rect.top,
        };

        // Hover + snap (always, even without dragging).
        // Priority: POINT SNAP → EDGE/GRID SNAP (via __resolveSnapTarget)
        const __pfPick = performance.now();
        const hit = window.__raycastSketchPlane && window.__raycastSketchPlane(mouse.ndcX, mouse.ndcY);

        // Device-pixel cursor position — must match __worldToScreenPx output used inside __resolveSnapTarget.
        const mpx = {
          x: (mouse.ndcX + 1) * 0.5 * canvas.width,
          y: (1 - mouse.ndcY) * 0.5 * canvas.height,
        };

        // ── Point snap priority ───────────────────────────────────────────────
        // __pickPointAt uses NDC; if a point is nearby, snap directly to it —
        // even if it doesn't lie on the current sketch plane hit.
        const _ppId = window.__pickPointAt ? window.__pickPointAt(mouse.ndcX, mouse.ndcY) : null;
        const _ppData = _ppId ? sketchState.points.find(pt => pt.id === _ppId) : null;

        if (_ppId && _ppData) {
          sketchState.hoverPointId = _ppId;
          sketchState.hoverWorld = {
            x: _ppData.x, y: _ppData.y, z: _ppData.z,
            gx: _ppData.gx, gy: _ppData.gy, gz: _ppData.gz,
            freeX: _ppData.x, freeY: _ppData.y, freeZ: _ppData.z,
            snapKind: 'point', pointId: _ppId,
            screenX: mpx.x, screenY: mpx.y,
          };
          sketchState.snap = {
            kind: 'point', pointId: _ppId,
            gx: _ppData.gx, gy: _ppData.gy, gz: _ppData.gz,
          };
        } else if (hit && window.__resolveSnapTarget) {
          // Fallback: snap to grid / edge via plane intersection
          const free   = { x: hit.freeX, y: hit.freeY, z: hit.freeZ };
          const target = window.__resolveSnapTarget(free, mpx, null);
          sketchState.hoverWorld = {
            x: target.x, y: target.y, z: target.z,
            gx: target.gx, gy: target.gy, gz: target.gz,
            freeX: hit.freeX, freeY: hit.freeY, freeZ: hit.freeZ,
            snapKind: target.kind, pointId: target.pointId,
            screenX: mpx.x, screenY: mpx.y,
          };
          sketchState.snap = {
            kind: target.kind, pointId: target.pointId,
            gx: target.gx, gy: target.gy, gz: target.gz,
          };
        } else {
          sketchState.hoverWorld = null;
          sketchState.snap = { kind: 'none', pointId: null, gx: 0, gy: 0, gz: 0 };
        }

        const tool = sketchState.activeTool;
        if (tool === 'select' || tool === 'delete') {
          sketchState.hoverPointId = _ppId || window.__pickPointAt(mouse.ndcX, mouse.ndcY);
          sketchState.hoverEdgeId  = sketchState.hoverPointId
            ? null : window.__pickEdgeAt(mouse.ndcX, mouse.ndcY);
          if (tool === 'select' && !sketchState.hoverPointId && !sketchState.hoverEdgeId && hit) {
            sketchState.hoverProfileId = window.__pickProfileAtWorld(hit.freeX, hit.freeY, hit.freeZ);
          } else {
            sketchState.hoverProfileId = null;
          }
        } else if (tool === 'line') {
          // hoverPointId: prefer direct pick, then fallback snap-to-point from __resolveSnapTarget
          const _snapPtId = (!_ppId && sketchState.snap && sketchState.snap.kind === 'point')
            ? sketchState.snap.pointId : null;
          sketchState.hoverPointId  = _ppId || _snapPtId || null;
          sketchState.hoverEdgeId   = null;
          sketchState.hoverProfileId = null;
        } else {
          sketchState.hoverPointId  = null;
          sketchState.hoverEdgeId   = null;
          sketchState.hoverProfileId = null;
        }

        if (window.__updateLinePreview) window.__updateLinePreview();
        const grabTarget = hit || sketchState.hoverWorld;
        if (sketchState.copy.active) window.__updateCopyConnect();

        // ── Cursor state machine (CAD-style 4 states) ────────────────────────
        // grabbing → grab → pointer → default
        {
          const rect3 = canvas.getBoundingClientRect();
          const hAxis = __hitGizmoHandle(e.clientX - rect3.left, e.clientY - rect3.top);
          window.__gizmoHoverAxis = hAxis || null;
          if (sketchState.grab?.active) {
            // During active grab: grabbing (will be overridden to 'grab' on hover)
            canvas.style.cursor = hAxis ? 'grab' : 'grabbing';
          } else if (hAxis) {
            canvas.style.cursor = 'grab';
          } else if (sketchState.hoverPointId || sketchState.hoverEdgeId || sketchState.hoverProfileId) {
            canvas.style.cursor = 'pointer';
          } else {
            __setCursorForTool();
          }
        }
        if (window.__perfSample) window.__perfSample('pick', performance.now() - __pfPick);

        // ── Grab movement ──────────────────────────────────────────────────────
        // CAD-style: cursor → axis → world-space delta → selected points
        //
        // Flow: SELECT → GIZMO → HIT AXIS → DRAG → CONFIRM / CANCEL
        //
        // Formula (world-space, no screen heuristics):
        //   currentPoint = raycastDragPlane(ndcX, ndcY, center)
        //   delta        = dot(currentPoint - startPoint, axisVector)
        //   newPosition  = basePosition + axisVector * delta
        if (sketchState.grab?.active) {
          const useProj = gizmoHandleDrag || window.__grabIsScreenProjection;

          // Maintain grabLastX/Y for legacy __updateGrab path
          if (grabLastX < 0) { grabLastX = e.clientX; grabLastY = e.clientY; }
          grabLastX = e.clientX; grabLastY = e.clientY;

          if (useProj) {
            // World-space projection path (axis/plane/free lock)
            if (window.__updateGizmoDrag) {
              window.__updateGizmoDrag(mouse.ndcX, mouse.ndcY);
            }
          } else {
            window.__updateGrab(grabTarget);
          }

          // Cursor: grabbing while dragging
          canvas.style.cursor = 'grabbing';

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
