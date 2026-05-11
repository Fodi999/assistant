// ── JS: Sketch tools — Line / Rectangle / Circle / Dimension + keyboard bindings ──
// Domain: Sketch — tool dispatch, isolated from state events and rendering.

pub const JS: &str = r##"
      // ─────────────────────────────────────────────────────────────
      // __handleSketchClick(ndcX, ndcY)
      // Called by state.rs pointerup when selectionMode === 4.
      // Dispatches the click to the active tool.
      // ─────────────────────────────────────────────────────────────
      window.__handleSketchClick = function(ndcX, ndcY) {
        const tool = (window.editorState && window.editorState.activeSketchTool) || 'line';

        // Select tool — no geometry creation
        if (tool === 'select') {
          log(`[sketch] Select tool: clicked empty space`, '#6b7280');
          if (window.__updateSketchUI) window.__updateSketchUI();
          return;
        }

        // ── Phase gate ─────────────────────────────────────────────
        const phase = sketchState.phase || 'drawing';
        if (phase === 'extrude_preview') {
          log('[sketch] ✋ locked: extrude preview active — Cancel or Create Solid', '#f59e0b');
          return;
        }
        if (phase === 'solid_created') {
          log('[sketch] ✋ locked: sketch is reference (solid created) — start new sketch', '#f59e0b');
          return;
        }

        log(`[sketch] tool="${tool}" raycast fn=${typeof window.__raycastSketchPlane}`, '#a78bfa');
        if (!['select', 'line', 'rectangle', 'circle', 'dimension'].includes(tool)) {
          log(`[sketch] ✗ unknown tool — skip`, '#f87171');
          return;
        }

        log(`[sketch] NDC x=${ndcX.toFixed(3)} y=${ndcY.toFixed(3)}`, '#6b7280');
        const snappedHit = window.__raycastSketchPlane(ndcX, ndcY);
        log(`[sketch] hit=${JSON.stringify(snappedHit)}`, snappedHit ? '#38bdf8' : '#f87171');
        if (!snappedHit) {
          log(`[sketch] ✗ raycast returned null — check __raycastSketchPlane`, '#f87171');
          return;
        }
        const hx = snappedHit.x, hy = snappedHit.y, hz = snappedHit.z;

        // ── LINE TOOL: chain of points, close by clicking first point ──
        if (tool === 'line') {
          if (sketchState.closed || phase === 'closed_profile') {
            log('[sketch] ✋ profile already closed — click "New Sketch" to draw again', '#f59e0b');
            return;
          }
          if (sketchState.points.length > 2 && snappedHit.snapType === 'first') {
            sketchState.closed = true;
            log(`✓ Sketch closed (${sketchState.points.length} pts)`, '#10b981');
            if (window.__setSketchPhase) window.__setSketchPhase('closed_profile', 'auto-close on first-point click');
            if (window.__updateSketchUI) window.__updateSketchUI();
            return;
          }
          const last = sketchState.points[sketchState.points.length - 1];
          if (last && Math.hypot(hx-last.x, hy-last.y, hz-last.z) < 1e-4) return;
          sketchState.points.push({ x: hx, y: hy, z: hz });
          log(`+ Pt ${sketchState.points.length}: ${hx.toFixed(3)}, ${hy.toFixed(3)}, ${hz.toFixed(3)}`, '#38bdf8');
          if (phase !== 'drawing') window.__setSketchPhase && window.__setSketchPhase('drawing', 'first point');
          if (window.__updateSketchUI) window.__updateSketchUI();
          return;
        }

        // ── RECTANGLE TOOL: delegated to sketch_rect.rs ──
        if (tool === 'rectangle') {
          if (window.__handleRectTool) window.__handleRectTool(hx, hy, hz, snappedHit);
          return;
        }

        // ── CIRCLE TOOL: center + radius ──
        if (tool === 'circle') {
          if (window.__handleCircleTool) window.__handleCircleTool(hx, hy, hz);
          return;
        }

        // ── DIMENSION TOOL: delegated to sketch_dim.rs ──
        if (tool === 'dimension') {
          if (window.__handleDimTool) window.__handleDimTool(hx, hy, hz);
          return;
        }
      };

      // ─────────────────────────────────────────────────────────────
      // __handleSketchKey(e)
      // Called by state.rs keydown when selectionMode === 4.
      // Returns true if the event was consumed.
      // ─────────────────────────────────────────────────────────────
      window.__handleSketchKey = function(e) {
        const k = e.key.toLowerCase();

        if (k === 'escape') {
          // Priority 1: cancel extrude preview
          if (window.extrudePreview && window.extrudePreview.active) {
            window.extrudePreview.active = false;
            window.extrudePreview.points = [];
            if (sketchState.closed && window.__setSketchPhase)
              window.__setSketchPhase('closed_profile', 'preview cancelled (Esc)');
            log('▣ Extrude preview cancelled (Esc)', '#f59e0b');
            if (window.__updateSketchUI) window.__updateSketchUI();
            return true;
          }
          // Priority 2: cancel pending tool
          if (sketchState.pendingStart) {
            if (window.__cancelRectTool)   window.__cancelRectTool();
            if (window.__cancelCircleTool) window.__cancelCircleTool();
            if (window.__cancelDimTool)    window.__cancelDimTool();
            sketchState.pendingStart = null;
            sketchState.pendingTool  = null;
            log('✕ Tool cancelled', '#fbbf24');
          } else if (sketchState.points.length > 0 && !sketchState.closed) {
            sketchState.points     = [];
            sketchState.dimensions = [];
            if (window.__setSketchPhase) window.__setSketchPhase('drawing', 'sketch cleared (Esc)');
            log('✕ Sketch cleared', '#fbbf24');
          }
          if (window.__updateSketchUI) window.__updateSketchUI();
          return true;
        }

        if (k === 'enter' && sketchState.points.length > 2 && !sketchState.closed) {
          sketchState.closed = true;
          if (window.__setSketchPhase) window.__setSketchPhase('closed_profile', 'closed via Enter');
          log(`✓ Sketch closed (Enter)`, '#10b981');
          if (window.__updateSketchUI) window.__updateSketchUI();
          return true;
        }

        if (k === 'backspace' && sketchState.points.length > 0 && !sketchState.closed) {
          sketchState.points.pop();
          log(`↶ Removed last point`, '#fbbf24');
          if (window.__updateSketchUI) window.__updateSketchUI();
          return true;
        }

        // Don't steal input from form fields
        if (e.target && (e.target.tagName === 'INPUT' || e.target.tagName === 'SELECT' || e.target.tagName === 'TEXTAREA')) return false;

        // Tool hotkeys
        if (k === 'l' && window.__setSketchTool) { window.__setSketchTool('line');      return true; }
        if (k === 'r' && window.__setSketchTool) { window.__setSketchTool('rectangle'); return true; }
        if (k === 'o' && window.__setSketchTool) { window.__setSketchTool('circle');    return true; }
        if (k === 'd' && window.__setSketchTool) { window.__setSketchTool('dimension'); return true; }

        return false;
      };
"##;
