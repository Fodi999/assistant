// ── JS: Sketch Circle Tool — draws a closed N-point circle from center + radius click ──
// Domain: Sketch tools — isolated circle logic.
// API:  window.__handleCircleTool(hx, hy, hz, snappedHit) → void
//       window.__cancelCircleTool() → void
//       window.__getCirclePreviewPoints() → [{x,y,z}] | null

pub const JS: &str = r##"
      // ─────────────────────────────────────────────────────────────
      // Circle tool: 2-click workflow.
      //   Click 1 → store pendingStart (center point)
      //   Click 2 → compute radius, generate N points, close profile
      //
      // Produces a regular polygon with CIRCLE_SEGMENTS segments.
      // Plane-aware: XZ / XY / YZ.
      // ─────────────────────────────────────────────────────────────
      const CIRCLE_SEGMENTS = 48; // higher → smoother circle

      window.__handleCircleTool = function(hx, hy, hz) {
        // ── Click 1: store center ──────────────────────────────────
        if (!sketchState.pendingStart) {
          sketchState.pendingStart = { x: hx, y: hy, z: hz };
          sketchState.pendingTool  = 'circle';
          log(`○ Circle center: (${hx.toFixed(3)}, ${hy.toFixed(3)}, ${hz.toFixed(3)})`, '#38bdf8');
          if (window.__updateSketchUI) window.__updateSketchUI();
          return;
        }

        // ── Click 2: compute radius, build N corner points ─────────
        const c  = sketchState.pendingStart;
        const pl = sketchState.plane;

        // Radius in the active plane
        let dxp, dyp;
        if (pl === 'XZ')      { dxp = hx - c.x; dyp = hz - c.z; }
        else if (pl === 'XY') { dxp = hx - c.x; dyp = hy - c.y; }
        else                  { dxp = hy - c.y; dyp = hz - c.z; }

        const r = Math.hypot(dxp, dyp);
        if (r < 1e-4) {
          log('○ Circle: zero radius — ignored', '#f59e0b');
          sketchState.pendingStart = null;
          sketchState.pendingTool  = null;
          if (window.__updateSketchUI) window.__updateSketchUI();
          return;
        }

        const pts = _buildCirclePoints(c, r, pl, CIRCLE_SEGMENTS);
        sketchState.points       = pts;
        sketchState.closed       = true;
        sketchState.pendingStart = null;
        sketchState.pendingTool  = null;

        const fmt = v => v >= 1 ? v.toFixed(3) + ' m' : (v * 100).toFixed(1) + ' cm';
        log(`✓ Circle  r=${fmt(r)}  (${pts.length} pts, plane ${pl})`, '#10b981');

        if (window.__setSketchPhase) window.__setSketchPhase('closed_profile', 'circle complete');
        if (window.__updateSketchUI) window.__updateSketchUI();
      };

      // Internal helper: build evenly-spaced circle points
      function _buildCirclePoints(center, r, pl, N) {
        const pts = [];
        for (let i = 0; i < N; i++) {
          const a  = (i / N) * Math.PI * 2;
          const ca = Math.cos(a) * r;
          const sa = Math.sin(a) * r;
          if (pl === 'XZ')      pts.push({ x: center.x + ca, y: 0,            z: center.z + sa });
          else if (pl === 'XY') pts.push({ x: center.x + ca, y: center.y + sa, z: 0            });
          else                  pts.push({ x: 0,             y: center.y + ca, z: center.z + sa });
        }
        return pts;
      }

      // Cancel circle mid-draw (called by Esc in sketch_tools.rs)
      window.__cancelCircleTool = function() {
        if (sketchState.pendingTool === 'circle') {
          sketchState.pendingStart = null;
          sketchState.pendingTool  = null;
          log('✕ Circle cancelled', '#fbbf24');
          if (window.__updateSketchUI) window.__updateSketchUI();
        }
      };

      // ─────────────────────────────────────────────────────────────
      // Hover preview: returns N circle points using current hover pos
      // as the radius point, so render_loop can draw a ghost circle.
      // Returns null when not in circle-pending state.
      // ─────────────────────────────────────────────────────────────
      window.__getCirclePreviewPoints = function() {
        if (sketchState.pendingTool !== 'circle' || !sketchState.pendingStart || !sketchState.hover) return null;
        const c  = sketchState.pendingStart;
        const hv = sketchState.hover;
        const pl = sketchState.plane;

        let dxp, dyp;
        if (pl === 'XZ')      { dxp = hv.x - c.x; dyp = hv.z - c.z; }
        else if (pl === 'XY') { dxp = hv.x - c.x; dyp = hv.y - c.y; }
        else                  { dxp = hv.y - c.y; dyp = hv.z - c.z; }

        const r = Math.hypot(dxp, dyp);
        if (r < 1e-4) return null;
        return _buildCirclePoints(c, r, pl, CIRCLE_SEGMENTS);
      };
"##;
