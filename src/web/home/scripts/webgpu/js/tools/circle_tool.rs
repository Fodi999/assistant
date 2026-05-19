// ── Circle Tool ──────────────────────────────────────────────────────────────
// CAD-style 2-click circle: click centre, click rim point → N-gon polyline
// (32 edges) approximating a circle. No constraints added — the geometry
// itself encodes the shape; future constraint types (RADIUS / FIXED_RADIUS)
// can be added later via __addConstraint.
//
// Handles:
//   __circleClick(ndcX, ndcY)   — click dispatch (1st = centre, 2nd = rim)
//   __cancelCircleTool()        — cancel on Esc / tool switch
//
// Hotkey: C
// State:  sketchState.circle = { active: false, centerSnap: null }
//
// Flow:
//   Click 1 → snap centre → store centerSnap, phase = 'circle-rim'
//   Click 2 → snap rim    → compute radius, create 32 pts + 32 edges, reset
//
// Preview:
//   render_loop draws dashed circle arc while circle.active && hoverWorld

pub const JS: &str = r##"

      // ─────────────────────────────────────────────────────────
      // __circleClick(ndcX, ndcY) — Circle Tool click handler
      // ─────────────────────────────────────────────────────────
      window.__circleClick = async function(ndcX, ndcY) {
        const snap = window.__resolveLineSnap ? window.__resolveLineSnap(ndcX, ndcY) : null;

        if (!snap || !snap.valid) {
          window.__setStatusMessage && window.__setStatusMessage('Circle: no snap target');
          return;
        }

        const circle = sketchState.circle || { active: false, centerSnap: null };
        const gs     = sketchState.gridSize || 1.0;

        const snapToGrid = (s) => ({
          gx: Math.round(s.gx !== undefined ? s.gx : s.x / gs),
          gy: Math.round(s.gy !== undefined ? s.gy : s.y / gs),
          gz: Math.round(s.gz !== undefined ? s.gz : s.z / gs),
        });

        // ── FIRST CLICK — store centre ────────────────────────────────────
        if (!circle.active) {
          const g1 = snapToGrid(snap);
          sketchState.circle = { active: true, centerSnap: g1 };
          sketchState.phase  = 'circle-rim';
          const label = snap.kind === 'point' ? 'snapped to point' : 'on grid';
          window.__setStatusMessage && window.__setStatusMessage(
            '⬤ Circle centre (' + label + ') · click rim point to set radius · Esc cancel'
          );
          if (window.__notifySketchChanged) window.__notifySketchChanged();
          return;
        }

        // ── SECOND CLICK — create circle ──────────────────────────────────
        const gc  = circle.centerSnap;
        const gr  = snapToGrid(snap);
        const plane = sketchState.workingPlane || 'XZ';

        // Compute radius in grid units using the working plane axes
        let radiusSq;
        if (plane === 'XZ') {
          radiusSq = (gr.gx - gc.gx) ** 2 + (gr.gz - gc.gz) ** 2;
        } else if (plane === 'XY') {
          radiusSq = (gr.gx - gc.gx) ** 2 + (gr.gy - gc.gy) ** 2;
        } else { // YZ
          radiusSq = (gr.gy - gc.gy) ** 2 + (gr.gz - gc.gz) ** 2;
        }

        if (radiusSq < 0.25) {
          // Radius < 0.5 grid units — degenerate, keep active
          window.__setStatusMessage && window.__setStatusMessage(
            'Circle: radius too small · click further from centre'
          );
          return;
        }

        const radius = Math.sqrt(radiusSq); // grid units
        const SEG    = 32;

        window.__pushHistory();

        // Build SEG evenly-spaced grid-coord corners around the circle
        const corners = [];
        for (let i = 0; i < SEG; i++) {
          const angle = (2 * Math.PI * i) / SEG;
          const cos   = Math.cos(angle);
          const sin   = Math.sin(angle);
          if (plane === 'XZ') {
            corners.push({
              gx: Math.round(gc.gx + radius * cos),
              gy: 0,
              gz: Math.round(gc.gz + radius * sin),
            });
          } else if (plane === 'XY') {
            corners.push({
              gx: Math.round(gc.gx + radius * cos),
              gy: Math.round(gc.gy + radius * sin),
              gz: 0,
            });
          } else { // YZ
            corners.push({
              gx: 0,
              gy: Math.round(gc.gy + radius * cos),
              gz: Math.round(gc.gz + radius * sin),
            });
          }
        }

        // Create the SEG perimeter points
        const ptIds = [];
        for (const c of corners) {
          const id = await window.__createPointViaEngine(c.gx, c.gy, c.gz);
          if (!id) {
            console.error('[Circle] failed to create point', c);
            window.__setStatusMessage && window.__setStatusMessage('Circle: failed to create point');
            sketchState.circle = { active: false, centerSnap: null };
            sketchState.phase  = 'idle';
            return;
          }
          ptIds.push(id);
        }

        // Create SEG edges forming a closed loop: 0→1, 1→2, …, (SEG-1)→0
        const edgeIds = [];
        for (let i = 0; i < SEG; i++) {
          const eid = await window.__createEdgeViaEngine(
            ptIds[i], ptIds[(i + 1) % SEG], 'normal'
          );
          if (eid) edgeIds.push(eid);
        }

        console.log('[Circle] created', {
          ptIds,
          edgeIds,
          centre: gc,
          radius,
          plane,
        });

        // Reset circle state
        sketchState.circle = { active: false, centerSnap: null };
        sketchState.phase  = 'idle';

        if (window.__notifySketchChanged)   window.__notifySketchChanged();
        if (window.__updateSketchInspector) window.__updateSketchInspector();
        window.__setStatusMessage && window.__setStatusMessage(
          '✓ Circle created (' + ptIds.length + ' pts, ' + edgeIds.length + ' edges, r≈' +
          (radius * gs).toFixed(2) + ')'
        );
      };

      // ─────────────────────────────────────────────────────────
      // __cancelCircleTool() — cancel circle on Esc / tool switch
      // ─────────────────────────────────────────────────────────
      window.__cancelCircleTool = function() {
        const wasActive = sketchState.circle && sketchState.circle.active;
        sketchState.circle = { active: false, centerSnap: null };
        sketchState.phase  = 'idle';
        if (wasActive) {
          window.__setStatusMessage && window.__setStatusMessage('Circle cancelled');
          if (window.__notifySketchChanged) window.__notifySketchChanged();
        }
      };

"##;
