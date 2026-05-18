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

      // ── Double-tap detector (trackpad fallback) ───────────────────
      // On macOS Safari, double-tap on a trackpad may trigger smart-zoom
      // before dblclick fires. We detect two rapid pointerdowns instead.
      let __lastTapTime = 0, __lastTapX = 0, __lastTapY = 0;
      let __dblTapFired = false;  // prevent firing both dblclick + pointerdown double-tap
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
        // Unified cursor standard: always default (Arrow) unless tool draws its own crosshair.
        // Gizmo hover/drag overrides happen in the cursor state machine below.
        let cur = 'default';
        if (t === 'point' || t === 'line') cur = 'none';  // crosshair drawn on canvas overlay
        canvas.style.cursor = cur;
      }
      window.__setCursorForTool = __setCursorForTool;

      // ── Gizmo handle hit test (delegates to gizmo_controller.rs) ──
      function __hitGizmoHandle(px, py) {
        return window.__hitTestGrabGizmo ? window.__hitTestGrabGizmo(px, py) : null;
      }

      // ── Pointer down ─────────────────────────────────────────────
      canvas.addEventListener('pointerdown', (e) => {
        // Block canvas interaction while extrude (or other modal tools) are active.
        if (window.sketchState?.extrude?.active) {
          e.preventDefault();
          e.stopPropagation();
          // Re-focus the extrude modal input so the user can keep typing.
          const inp = document.getElementById('__extrude-modal-input');
          if (inp) inp.focus();
          return;
        }
        // Update NDC coords immediately — pointerdown may arrive before a pointermove.
        // This ensures __gizmoPointerDown receives accurate mouse.ndcX/Y.
        {
          const _rect = canvas.getBoundingClientRect();
          mouse.ndcX = ((e.clientX - _rect.left) / _rect.width)  * 2 - 1;
          mouse.ndcY = 1 - ((e.clientY - _rect.top)  / _rect.height) * 2;
          mouse.active = true;
        }
        // 1) Gizmo controller has first say — if it consumes, we're done.
        if (window.__gizmoPointerDown && window.__gizmoPointerDown(mouse, e)) {
          dragging = true;
          dragMoved = false;
          gizmoHandleDrag = true;
          startX = e.clientX; startY = e.clientY;
          lastX = e.clientX;  lastY = e.clientY;
          return;
        }

        canvas.setPointerCapture(e.pointerId);
        dragging       = true;
        dragMoved      = false;
        gizmoHandleDrag = false;

        // ── Trackpad double-tap detector ──────────────────────────
        // Fires __handleSketchDoubleClick via pointerdown timing so it works
        // even when the browser intercepts 'dblclick' for smart-zoom gestures.
        if (e.button === 0) {
          const now = Date.now();
          const ddx = e.clientX - __lastTapX;
          const ddy = e.clientY - __lastTapY;
          const isDoubleTap = (now - __lastTapTime) < 350 && Math.hypot(ddx, ddy) < 20;
          __lastTapTime = now;
          __lastTapX    = e.clientX;
          __lastTapY    = e.clientY;
          if (isDoubleTap) {
            __dblTapFired = true;
            const rectDT  = canvas.getBoundingClientRect();
            const ndcXdt  = ((e.clientX - rectDT.left) / rectDT.width)  * 2 - 1;
            const ndcYdt  = 1 - ((e.clientY - rectDT.top)  / rectDT.height) * 2;
            if (window.__handleSketchDoubleClick)
              window.__handleSketchDoubleClick(ndcXdt, ndcYdt, e.clientX, e.clientY);
            setTimeout(() => { __dblTapFired = false; }, 400);
          }
        }

        const rect2   = canvas.getBoundingClientRect();
        const px      = e.clientX - rect2.left;
        const py      = e.clientY - rect2.top;

        // ── Dimension label click: open editor, suppress orbit/pan/select ──
        if (e.button === 0) {
          const labelHit = window.__hitDraftingLabel?.(px, py);
          if (labelHit) {
            window.__openDimensionEditor?.(labelHit, e.clientX, e.clientY);
            try { canvas.releasePointerCapture(e.pointerId); } catch (_) {}
            dragging        = false;
            panning         = false;
            orbiting        = false;
            gizmoHandleDrag = false;
            e.preventDefault();
            e.stopPropagation();
            return;
          }
        }

        const hitAxis = __hitGizmoHandle(px, py);

        const isGrab = sketchState.grab?.active;
        const isCopy = sketchState.copy?.active;

        // Gizmo handle clicks are handled at the top of pointerdown by
        // __gizmoPointerDown. Here we only handle the copy-tool variant.

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
        // 1) Gizmo controller has first say
        if (window.__gizmoPointerUp && window.__gizmoPointerUp(mouse, e)) {
          dragging = false;
          gizmoHandleDrag = false;
          panning = false; orbiting = false;
          startX = 0; startY = 0;
          return;
        }

        const wasDragging = dragging;
        dragging = false;
        const wasGizmo = gizmoHandleDrag;
        gizmoHandleDrag = false;
        try { canvas.releasePointerCapture(e.pointerId); } catch {}
        const dist = Math.hypot(e.clientX - startX, e.clientY - startY);
        startX = 0; startY = 0;

        if (wasGizmo) {
          window.__hitGizmoOnDown = false;
          // NOTE: confirm/cancel is handled exclusively by __gizmoPointerUp (called above).
          // This block only clears any legacy gizmo state that may remain.
          sketchState.gizmoDrag = { active: false, axis: null, pointerId: null };
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
        window.__orbitActive = false;
      });

      // ── Double-click ─────────────────────────────────────────────
      canvas.addEventListener('dblclick', (e) => {
        // Skip if the pointerdown double-tap detector already handled this.
        if (__dblTapFired) { __dblTapFired = false; return; }
        const rect = canvas.getBoundingClientRect();
        const ndcX = ((e.clientX - rect.left) / rect.width)  * 2 - 1;
        const ndcY = 1 - ((e.clientY - rect.top)  / rect.height) * 2;
        if (window.__handleSketchDoubleClick) window.__handleSketchDoubleClick(ndcX, ndcY, e.clientX, e.clientY);
      });

      // ── Pointer move — hover, snap, orbit, pan ───────────────────
      canvas.addEventListener('pointermove', (e) => {
        const rect = canvas.getBoundingClientRect();
        mouse.ndcX = ((e.clientX - rect.left) / rect.width)  * 2 - 1;
        mouse.ndcY = 1 - ((e.clientY - rect.top)  / rect.height) * 2;
        mouse.active = true;
        // Track global cursor position for cursor-hud positioning
        window._lastMouseX = e.clientX;
        window._lastMouseY = e.clientY;

        // Always update lastMouseScreen — needed by Copy Connect even off sketch plane
        if (!sketchState.precision) sketchState.precision = {};
        sketchState.precision.lastMouseScreen = {
          x: e.clientX - rect.left,
          y: e.clientY - rect.top,
        };

        // 1) Gizmo controller has first say. If actively dragging a handle,
        //    it consumes the event and moves geometry. Otherwise it only
        //    updates hoverAxis and lets the rest of the pipeline run.
        if (window.__gizmoPointerMove && window.__gizmoPointerMove(mouse, e)) {
          if (Math.hypot(e.clientX - startX, e.clientY - startY) >= CLICK_THRESH_PX) dragMoved = true;
          return;
        }

        // ── Dimension label hover (only when not dragging) ──
        // Show text-cursor over editable dim labels so users discover the
        // click-to-edit affordance. Drops back to tool cursor when leaving.
        if (!dragging && !orbiting && !panning) {
          const _pxCss = e.clientX - rect.left;
          const _pyCss = e.clientY - rect.top;
          const labelHover = window.__hitDraftingLabel?.(_pxCss, _pyCss);
          if (labelHover) {
            canvas.style.cursor = 'text';
            window.__draftingLabelHover = labelHover;
            return;
          } else if (window.__draftingLabelHover) {
            window.__draftingLabelHover = null;
            __setCursorForTool();
          }
        }

        // Hover + snap (always, even without dragging).
        // Priority: POINT SNAP → EDGE/GRID SNAP (via __resolveSnapTarget)
        // Track Alt key for precision cursor mode.
        sketchState.precisionMode = !!e.altKey;
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

        // ── Cursor state machine ────────────────────────
        // Standard: default (Arrow) for all tools.
        // Overrides: gizmo-drag → grabbing ; gizmo/entity-hover → pointer ; point/line → none (canvas crosshair)
        {
          const hAxis = sketchState.gizmo?.hoverAxis || null;
          window.__gizmoHoverAxis = hAxis;  // legacy alias for renderer
          if (sketchState.gizmoDrag?.active) {
            canvas.style.cursor = 'grabbing';
          } else if (hAxis) {
            canvas.style.cursor = 'pointer';
          } else if (sketchState.hoverPointId || sketchState.hoverEdgeId || sketchState.hoverProfileId) {
            canvas.style.cursor = 'pointer';
          } else {
            __setCursorForTool();
          }
        }
        if (window.__perfSample) window.__perfSample('pick', performance.now() - __pfPick);

        // Grab movement is fully delegated to gizmo_controller.rs at the top
        // of this handler. No grab-related logic remains here.

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
          window.__orbitActive = true;
        }
      });

      canvas.addEventListener('pointerleave', () => { mouse.active = false; });
      canvas.addEventListener('contextmenu',  (e) => e.preventDefault());
"##;
