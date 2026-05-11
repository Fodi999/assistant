// ── JS: Sketch Dimension tool — 2-click linear annotation ──
// Domain: Sketch — isolated dimension tool, called from sketch_tools.rs dispatcher.

pub const JS: &str = r##"
      // ─────────────────────────────────────────────────────────────
      // __handleDimTool(hx, hy, hz)
      // Click 1 — store point 1.
      // Click 2 — compute distance, push annotation to sketchState.dimensions.
      // ─────────────────────────────────────────────────────────────
      window.__handleDimTool = function(hx, hy, hz) {
        if (!sketchState.pendingStart) {
          // First click — store origin
          sketchState.pendingStart = { x: hx, y: hy, z: hz };
          sketchState.pendingTool  = 'dimension';
          log(`↔ Dim point 1: ${hx.toFixed(3)}, ${hz.toFixed(3)}`, '#a78bfa');
          return;
        }

        // Second click — measure
        const p1   = sketchState.pendingStart;
        const p2   = { x: hx, y: hy, z: hz };
        const dist = Math.hypot(p2.x - p1.x, p2.y - p1.y, p2.z - p1.z);

        // Human-readable label
        const label = dist < 0.01  ? (dist * 1000).toFixed(1) + ' mm'
                    : dist < 1.0   ? (dist * 100).toFixed(1)  + ' cm'
                    :                dist.toFixed(3)           + ' m';

        if (!Array.isArray(sketchState.dimensions)) sketchState.dimensions = [];
        sketchState.dimensions.push({ p1, p2, label, id: 'dim-' + Date.now() });

        sketchState.pendingStart = null;
        sketchState.pendingTool  = null;

        log(`✓ Dim: ${label}`, '#a78bfa');
        if (window.__updateSketchUI) window.__updateSketchUI();
      };

      // ─────────────────────────────────────────────────────────────
      // __cancelDimTool()
      // Clears pending dimension state (called on Esc).
      // ─────────────────────────────────────────────────────────────
      window.__cancelDimTool = function() {
        if (sketchState.pendingTool === 'dimension') {
          sketchState.pendingStart = null;
          sketchState.pendingTool  = null;
          log('✕ Dimension cancelled', '#fbbf24');
        }
      };

      // ─────────────────────────────────────────────────────────────
      // __getDimPreviewLine(hx, hy, hz)
      // Returns [p1, p2] for a ghost line while hovering before click 2.
      // Returns null if no pending dimension.
      // ─────────────────────────────────────────────────────────────
      window.__getDimPreviewLine = function(hx, hy, hz) {
        if (!sketchState.pendingStart || sketchState.pendingTool !== 'dimension') return null;
        const p1 = sketchState.pendingStart;
        const p2 = { x: hx, y: hy, z: hz };
        const dist = Math.hypot(p2.x - p1.x, p2.y - p1.y, p2.z - p1.z);
        const label = dist < 0.01  ? (dist * 1000).toFixed(1) + ' mm'
                    : dist < 1.0   ? (dist * 100).toFixed(1)  + ' cm'
                    :                dist.toFixed(3)           + ' m';
        return { p1, p2, label };
      };
"##;
