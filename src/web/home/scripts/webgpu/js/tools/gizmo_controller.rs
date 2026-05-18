// ── Gizmo Controller — единственный диспетчер pointer-событий для гизмо ──────
//
// CAD/Plasticity-style behaviour:
//   * G key / toolbar Grab    → command mode (gizmo shown, no movement)
//   * pointer hover on handle → hover highlight, no movement
//   * pointer DOWN on handle  → starts gizmo drag (calls __startGrabFromGizmo)
//   * pointer MOVE while down → __updateGizmoDrag
//   * pointer UP while drag   → __confirmGrab
//   * Esc                     → __cancelGrab (via hotkeys.rs)
//   * G X 120 Enter           → numeric move (via hotkeys.rs → __confirmGrab)
//
// Geometry never moves on plain mousemove. Movement requires:
//   sketchState.gizmoDrag.active === true  AND
//   sketchState.grab.active === true       AND
//   sketchState.grab.dragging === true
//
// Exports:
//   window.__gizmoPointerDown(mouse, ev) → bool  (true = consumed)
//   window.__gizmoPointerMove(mouse, ev) → bool
//   window.__gizmoPointerUp(mouse, ev)   → bool
//   window.__gizmoCancel()               → bool
//   window.__hitTestGrabGizmo(mx, my)    → 'X'|'Y'|'Z'|'FREE'|null
//   window.__ensureGizmoState()
//
// mouse.rs must delegate FIRST:
//   pointerdown: if (window.__gizmoPointerDown?.(mouse, ev)) return;
//   pointermove: if (window.__gizmoPointerMove?.(mouse, ev)) return;
//   pointerup:   if (window.__gizmoPointerUp?.(mouse, ev)) return;

pub const JS: &str = r##"
      // ── Ensure gizmo + gizmoDrag state exist on sketchState ──
      window.__ensureGizmoState = function() {
        if (!sketchState.gizmo) {
          sketchState.gizmo = { hoverAxis: null, activeAxis: null };
        }
        if (!sketchState.gizmoDrag) {
          sketchState.gizmoDrag = { active: false, axis: null, pointerId: null };
        }
      };
      window.__ensureGizmoState();

      // ── Hit-test against handles published by drawGrabGizmo ──────────────
      // Handles array is filled every frame by grab_gizmo.rs into
      // window.__gizmoHandles = [{axis:'X', x, y, r, ox, oy}, ...]
      // x,y = arrowhead tip in DEVICE pixels; ox,oy = shaft origin in DEVICE px.
      // IMPORTANT: all coords are DEVICE pixels — divide by dpr for CSS.
      //
      // Hit logic: point is in circle around tip OR within CORRIDOR along shaft.
      window.__hitTestGrabGizmo = function(mx, my) {
        const handles = window.__gizmoHandles;
        if (!handles || !handles.length) return null;

        const dpr = window.devicePixelRatio || 1;
        // Extra padding around all hit areas (CSS px)
        const PAD = 10;

        // Helper: distance from point (px,py) to segment (ax,ay)-(bx,by)
        function distToSegment(px, py, ax, ay, bx, by) {
          const dx = bx - ax, dy = by - ay;
          const lenSq = dx*dx + dy*dy;
          if (lenSq === 0) return Math.hypot(px - ax, py - ay);
          let t = ((px - ax)*dx + (py - ay)*dy) / lenSq;
          t = Math.max(0, Math.min(1, t));
          return Math.hypot(px - (ax + t*dx), py - (ay + t*dy));
        }

        // ── Priority pass: FREE ring always wins if cursor is inside it ──────
        // Arrow corridors start at origin too (ox=origin), so distToSegment=0
        // for all arrows at center — FREE must be checked first.
        for (const h of handles) {
          if (h.axis !== 'FREE') continue;
          const hx = h.x / dpr, hy = h.y / dpr, hr = h.r / dpr;
          if (Math.hypot(mx - hx, my - hy) <= hr + PAD) return 'FREE';
        }

        // ── Arrow / axis handles (corridor hit) ───────────────────────────
        let bestAxis = null;
        let bestDist = Infinity;
        for (const h of handles) {
          if (h.axis === 'FREE') continue;  // already handled above
          const hx = h.x / dpr;
          const hy = h.y / dpr;
          const hr = h.r / dpr;

          let d;
          if (h.ox != null && h.oy != null) {
            // Shaft corridor hit (all along the arrow length)
            const sox = h.ox / dpr;
            const soy = h.oy / dpr;
            const CORRIDOR = hr * 0.5 + PAD;  // half-corridor width in CSS px
            d = distToSegment(mx, my, sox, soy, hx, hy);
            if (d > CORRIDOR) d = Infinity;
          } else {
            // Legacy: circle hit only
            d = Math.hypot(mx - hx, my - hy);
            if (d > hr + PAD) d = Infinity;
          }
          if (d < bestDist) {
            bestDist = d;
            bestAxis = h.axis;
          }
        }
        return bestAxis;
      };

      // ── Reset gizmoDrag (used by confirm/cancel/tool-switch/Esc) ──────────
      window.__resetGizmoDrag = function() {
        window.__ensureGizmoState();
        sketchState.gizmoDrag.active    = false;
        sketchState.gizmoDrag.axis      = null;
        sketchState.gizmoDrag.pointerId = null;
        sketchState.gizmo.activeAxis    = null;
      };

      // ─────────────────────────────────────────────────────────────────────
      // __gizmoPointerDown(mouse, ev) → bool
      // Returns true if event was consumed by gizmo (caller must `return`).
      // ─────────────────────────────────────────────────────────────────────
      window.__gizmoPointerDown = function(mouse, ev) {
        if (ev.button !== 0) return false;
        if (sketchState.copy?.active) return false;  // copy tool has its own flow

        window.__ensureGizmoState();

        const canvas = document.getElementById('webgpu-canvas');
        if (!canvas) return false;
        const rect = canvas.getBoundingClientRect();
        const mx = ev.clientX - rect.left;
        const my = ev.clientY - rect.top;

        const hitAxis = window.__hitTestGrabGizmo(mx, my);
        const handles = window.__gizmoHandles;
        const _dpr = window.devicePixelRatio || 1;
        console.log('[GIZMO DOWN]', {
          'cursor(css)': { mx: mx.toFixed(1), my: my.toFixed(1) },
          hitAxis,
          dpr: _dpr,
          'handles(css)': handles ? handles.map(h => ({axis:h.axis, x:(h.x/_dpr).toFixed(1), y:(h.y/_dpr).toFixed(1), r:(h.r/_dpr).toFixed(1)})) : null,
          grabActive: sketchState.grab?.active,
          grabMode: sketchState.grab?.mode,
        });
        if (!hitAxis) {
          console.log('[GIZMO DOWN] → miss, not consumed');
          return false;
        }

        // Mark gizmoDrag active BEFORE starting grab so the move handler
        // can validate the chain. __startGrabFromGizmo must not reset this.
        sketchState.gizmoDrag = {
          active: true,
          axis: hitAxis,
          pointerId: ev.pointerId,
        };
        sketchState.gizmo.activeAxis = hitAxis;

        const isGrab = sketchState.grab?.active;

        if (isGrab) {
          // Re-configure axis lock for existing grab session
          const g = sketchState.grab;
          g.axisLock = (hitAxis === 'FREE') ? null : hitAxis;
          g.dragAxis = hitAxis;
          g.useDragPlane = hitAxis !== 'FREE';
          g.dragging = true;
          g.mode = 'gizmo-drag';
          g.screenAcc = { x: 0, y: 0, z: 0 };

          // Re-anchor startScreen to gizmo center
          const _ctr = window.__gizmoCenterScreen;
          if (_ctr) g.startScreen = { x: _ctr.x, y: _ctr.y };

          // Reset drag base to current positions + recompute startCenter
          const byId = new Map(sketchState.points.map(p => [p.id, p]));
          g.dragBase = new Map();
          let cx=0, cy=0, cz=0, n=0;
          for (const id of g.pointIds) {
            const p = byId.get(id);
            if (p) {
              g.dragBase.set(id, { x: p.x, y: p.y, z: p.z });
              cx+=p.x; cy+=p.y; cz+=p.z; n++;
            }
          }
          if (n) g.startCenter = { x:cx/n, y:cy/n, z:cz/n };
          g.startDragPoint = null;  // will be set on next __updateGizmoDrag
        } else {
          // No grab yet — start one from gizmo (NDC coords, not clientX/Y)
          if (window.__startGrabFromGizmo) {
            window.__startGrabFromGizmo(hitAxis, mouse.ndcX, mouse.ndcY);
          }
          // __startGrabFromGizmo creates grab; ensure dragging flag is set
          if (sketchState.grab) {
            sketchState.grab.dragging = true;
            sketchState.grab.mode     = 'gizmo-drag';
            sketchState.grab.source   = 'gizmo';
            sketchState.grab.dragAxis = hitAxis;
            sketchState.grab.useDragPlane = hitAxis !== 'FREE';
          }
        }

        // If grab wasn't created (e.g. no selection) — undo gizmoDrag
        if (!sketchState.grab?.active) {
          console.log('[GIZMO DOWN] → grab not created (no selection?), aborting');
          window.__resetGizmoDrag();
          return false;
        }

        console.log('[GIZMO DOWN] → consumed axis=' + hitAxis + ' mode=' + sketchState.grab?.mode);
        try { canvas.setPointerCapture(ev.pointerId); } catch (_) {}
        ev.preventDefault();
        ev.stopPropagation();
        canvas.style.cursor = 'grabbing';
        return true;
      };

      // ─────────────────────────────────────────────────────────────────────
      // __gizmoPointerMove(mouse, ev) → bool
      //   * If actively dragging a handle → __updateGizmoDrag, return true
      //   * Else → only update hoverAxis + cursor, return false (let other
      //            handlers continue: snap, orbit-prep, etc.)
      // ─────────────────────────────────────────────────────────────────────
      window.__gizmoPointerMove = function(mouse, ev) {
        window.__ensureGizmoState();

        const canvas = document.getElementById('webgpu-canvas');
        if (!canvas) return false;
        const rect = canvas.getBoundingClientRect();
        const mx = ev.clientX - rect.left;
        const my = ev.clientY - rect.top;

        // 1) Active drag — move geometry
        if (sketchState.gizmoDrag.active &&
            sketchState.grab?.active &&
            sketchState.grab.dragging) {
          if (window.__updateGizmoDrag) {
            window.__updateGizmoDrag(mouse.ndcX, mouse.ndcY);
          }
          canvas.style.cursor = 'grabbing';
          return true;  // consume — skip orbit/pan/select
        }

        // 2) Hover-only — update hoverAxis (cheap, no geometry change)
        const hitAxis = window.__hitTestGrabGizmo(mx, my);
        if (sketchState.gizmo.hoverAxis !== hitAxis) {
          console.log('[GIZMO HOVER]', hitAxis, '| handles:', window.__gizmoHandles?.length ?? 'null');
          sketchState.gizmo.hoverAxis = hitAxis;
          window.__gizmoHoverAxis = hitAxis;  // legacy alias for renderer
          canvas.style.cursor = hitAxis ? 'pointer' : 'default';
          window.__requestRedraw?.();
        }
        return false;  // not consumed — caller may still run snap/hover logic
      };

      // ─────────────────────────────────────────────────────────────────────
      // __gizmoPointerUp(mouse, ev) → bool
      // If currently dragging → confirm and consume.
      // ─────────────────────────────────────────────────────────────────────
      window.__gizmoPointerUp = function(mouse, ev) {
        window.__ensureGizmoState();

        if (!sketchState.gizmoDrag.active) return false;

        const canvas = document.getElementById('webgpu-canvas');
        const pointerId = sketchState.gizmoDrag.pointerId;

        // Strict: confirm ONLY if this was a real gizmo-drag (not command mode)
        const isRealGizmoDrag =
          sketchState.gizmoDrag?.active === true &&
          sketchState.grab?.active === true &&
          sketchState.grab?.dragging === true &&
          sketchState.grab?.mode === 'gizmo-drag';

        console.log('[GIZMO UP]', { isRealGizmoDrag, grabMode: sketchState.grab?.mode, grabDragging: sketchState.grab?.dragging });

        if (isRealGizmoDrag && window.__confirmGrab) {
          window.__confirmGrab().catch?.(e => console.warn('[gizmo] confirmGrab', e));
        }

        window.__resetGizmoDrag();

        if (canvas && pointerId != null) {
          try { canvas.releasePointerCapture(pointerId); } catch (_) {}
        }
        if (canvas) canvas.style.cursor = 'default';

        ev.preventDefault();
        ev.stopPropagation();
        return true;
      };

      // ─────────────────────────────────────────────────────────────────────
      // __gizmoCancel() — called by Esc (hotkeys.rs) / tool switch
      // ─────────────────────────────────────────────────────────────────────
      window.__gizmoCancel = function() {
        window.__ensureGizmoState();
        const wasActive = sketchState.gizmoDrag.active || sketchState.grab?.active;
        if (!wasActive) return false;

        if (sketchState.grab?.active && window.__cancelGrab) {
          window.__cancelGrab();
        }
        window.__resetGizmoDrag();
        const canvas = document.getElementById('webgpu-canvas');
        if (canvas) canvas.style.cursor = 'default';
        return true;
      };
"##;
