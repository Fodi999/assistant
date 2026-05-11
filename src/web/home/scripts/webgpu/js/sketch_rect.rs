// ── JS: Sketch Rectangle Tool — draws a closed 4-point rect from two corner clicks ──
// Domain: Sketch tools — isolated rectangle logic.
// API:  window.__handleRectTool(hx, hy, hz, snappedHit) → void
//       window.__cancelRectTool() → void

pub const JS: &str = r##"
      // ─────────────────────────────────────────────────────────────
      // Rectangle tool: 2-click workflow.
      //   Click 1 → store pendingStart (first corner)
      //   Click 2 → generate 4 points, close profile
      //
      // Draws 4 line-segments forming a rectangle on the active plane:
      //   XZ: top-down floor rectangle  (Y = 0)
      //   XY: front-face rectangle      (Z = 0)
      //   YZ: side-face rectangle       (X = 0)
      //
      // The result is stored in sketchState.points[] (closed = true).
      // ─────────────────────────────────────────────────────────────
      window.__handleRectTool = function(hx, hy, hz, snappedHit) {
        // ── Click 1: store first corner ───────────────────────────
        if (!sketchState.pendingStart) {
          sketchState.pendingStart = { x: hx, y: hy, z: hz };
          sketchState.pendingTool  = 'rectangle';
          log(`◻ Rect corner 1: (${hx.toFixed(3)}, ${hz.toFixed(3)})`, '#38bdf8');
          if (window.__updateSketchUI) window.__updateSketchUI();
          return;
        }

        // ── Click 2: build 4 corner points, close profile ─────────
        const s  = sketchState.pendingStart;
        const pl = sketchState.plane;

        // Guard: ignore degenerate rectangles (zero area)
        let area = 0;
        if (pl === 'XZ') area = Math.abs((hx - s.x) * (hz - s.z));
        else if (pl === 'XY') area = Math.abs((hx - s.x) * (hy - s.y));
        else                  area = Math.abs((hy - s.y) * (hz - s.z));

        if (area < 1e-6) {
          log('◻ Rect: zero area — ignored', '#f59e0b');
          sketchState.pendingStart = null;
          sketchState.pendingTool  = null;
          if (window.__updateSketchUI) window.__updateSketchUI();
          return;
        }

        // Build 4 corners (CCW winding when viewed from above / front)
        let pts;
        if (pl === 'XZ') {
          pts = [
            { x: s.x, y: 0, z: s.z },  // TL
            { x: hx,  y: 0, z: s.z },  // TR
            { x: hx,  y: 0, z: hz  },  // BR
            { x: s.x, y: 0, z: hz  },  // BL
          ];
        } else if (pl === 'XY') {
          pts = [
            { x: s.x, y: s.y, z: 0 },
            { x: hx,  y: s.y, z: 0 },
            { x: hx,  y: hy,  z: 0 },
            { x: s.x, y: hy,  z: 0 },
          ];
        } else { // YZ
          pts = [
            { x: 0, y: s.y, z: s.z },
            { x: 0, y: hy,  z: s.z },
            { x: 0, y: hy,  z: hz  },
            { x: 0, y: s.y, z: hz  },
          ];
        }

        sketchState.points       = pts;
        sketchState.closed       = true;
        sketchState.pendingStart = null;
        sketchState.pendingTool  = null;

        // Log dimensions for feedback
        let w = 0, h = 0;
        if (pl === 'XZ')      { w = Math.abs(hx - s.x); h = Math.abs(hz - s.z); }
        else if (pl === 'XY') { w = Math.abs(hx - s.x); h = Math.abs(hy - s.y); }
        else                  { w = Math.abs(hy - s.y); h = Math.abs(hz - s.z); }
        const fmt = v => v >= 1 ? v.toFixed(3) + ' m' : (v * 100).toFixed(1) + ' cm';
        log(`✓ Rect  ${fmt(w)} × ${fmt(h)}  (${pts.length} pts, plane ${pl})`, '#10b981');

        if (window.__setSketchPhase) window.__setSketchPhase('closed_profile', 'rectangle complete');
        if (window.__updateSketchUI) window.__updateSketchUI();
      };

      // Cancel rect mid-draw (called by Esc in sketch_tools.rs)
      window.__cancelRectTool = function() {
        if (sketchState.pendingTool === 'rectangle') {
          sketchState.pendingStart = null;
          sketchState.pendingTool  = null;
          log('✕ Rect cancelled', '#fbbf24');
          if (window.__updateSketchUI) window.__updateSketchUI();
        }
      };

      // ─────────────────────────────────────────────────────────────
      // Hover preview helper — returns the 4 rect corners for the
      // render_loop to draw a ghost rectangle while pendingTool='rectangle'.
      // Returns null when not in rect-pending state.
      // ─────────────────────────────────────────────────────────────
      window.__getRectPreviewPoints = function() {
        if (sketchState.pendingTool !== 'rectangle' || !sketchState.pendingStart || !sketchState.hover) return null;
        const s  = sketchState.pendingStart;
        const hv = sketchState.hover;
        const pl = sketchState.plane;
        if (pl === 'XZ') return [
          { x: s.x,   y: 0, z: s.z   },
          { x: hv.x,  y: 0, z: s.z   },
          { x: hv.x,  y: 0, z: hv.z  },
          { x: s.x,   y: 0, z: hv.z  },
        ];
        if (pl === 'XY') return [
          { x: s.x,  y: s.y,  z: 0 },
          { x: hv.x, y: s.y,  z: 0 },
          { x: hv.x, y: hv.y, z: 0 },
          { x: s.x,  y: hv.y, z: 0 },
        ];
        // YZ
        return [
          { x: 0, y: s.y,  z: s.z  },
          { x: 0, y: hv.y, z: s.z  },
          { x: 0, y: hv.y, z: hv.z },
          { x: 0, y: s.y,  z: hv.z },
        ];
      };
"##;
