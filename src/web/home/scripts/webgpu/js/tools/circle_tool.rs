// ── Circle Tool — thin WASM wrapper ─────────────────────────────────────────
// Geometry logic lives in geometry_engine::tools::circle (Rust/WASM).
// This file only handles: snap, 2-click FSM (centre + rim), cancel.
//
// WASM call: wasm_tool_circle(json) → SketchDelta
// Applied via: window.__applySketchDelta(delta)
//
// Hotkey: C
// State:  sketchState.circle = { active: false, centerSnap: null }

pub const JS: &str = r##"

      window.__circleClick = async function(ndcX, ndcY) {
        const snap = window.__resolveLineSnap ? window.__resolveLineSnap(ndcX, ndcY) : null;
        if (!snap || !snap.valid) {
          window.__setStatusMessage && window.__setStatusMessage('Circle: no snap target');
          return;
        }
        const circle = sketchState.circle || { active: false, centerSnap: null };
        const gs     = sketchState.gridSize || 1.0;
        const plane  = sketchState.workingPlane || 'XZ';

        const snapToGrid = (s) => ({
          gx: Math.round(s.gx !== undefined ? s.gx : s.x / gs),
          gy: Math.round(s.gy !== undefined ? s.gy : s.y / gs),
          gz: Math.round(s.gz !== undefined ? s.gz : s.z / gs),
        });

        if (!circle.active) {
          const g1 = snapToGrid(snap);
          sketchState.circle = { active: true, centerSnap: g1 };
          sketchState.phase  = 'circle-rim';
          const label = snap.kind === 'point' ? 'snapped to point' : 'on grid';
          window.__setStatusMessage && window.__setStatusMessage(
            '⬤ Circle centre (' + label + ') · click rim point · Esc cancel');
          if (window.__notifySketchChanged) window.__notifySketchChanged();
          return;
        }

        const gc  = circle.centerSnap;
        const gr  = snapToGrid(snap);

        // Compute radius in grid units
        let radiusSq;
        if      (plane === 'XY') radiusSq = (gr.gx-gc.gx)**2 + (gr.gy-gc.gy)**2;
        else if (plane === 'YZ') radiusSq = (gr.gy-gc.gy)**2 + (gr.gz-gc.gz)**2;
        else                     radiusSq = (gr.gx-gc.gx)**2 + (gr.gz-gc.gz)**2;

        if (radiusSq < 0.25) {
          window.__setStatusMessage && window.__setStatusMessage(
            'Circle: radius too small · click further from centre');
          return;
        }

        const wm = window.__wasmModule;
        if (!wm || typeof wm.wasm_tool_circle !== 'function') {
          window.__setStatusMessage && window.__setStatusMessage('Circle: WASM not loaded');
          sketchState.circle = { active: false, centerSnap: null };
          sketchState.phase  = 'idle';
          return;
        }

        window.__pushHistory();
        const raw = wm.wasm_tool_circle(JSON.stringify({
          center_gx: gc.gx, center_gy: gc.gy, center_gz: gc.gz,
          radius: Math.sqrt(radiusSq),
          plane,
          segments: 32,
          id_offset: Date.now() % 1000000000,
        }));
        const delta = JSON.parse(raw);

        if (!delta.ok) {
          window.__setStatusMessage && window.__setStatusMessage('Circle: ' + (delta.error || 'error'));
          sketchState.circle = { active: false, centerSnap: null };
          sketchState.phase  = 'idle';
          return;
        }

        window.__applySketchDelta(delta);
        sketchState.circle = { active: false, centerSnap: null };
        sketchState.phase  = 'idle';
        if (window.__notifySketchChanged)   window.__notifySketchChanged();
        if (window.__updateSketchInspector) window.__updateSketchInspector();
        window.__setStatusMessage && window.__setStatusMessage(
          '✓ Circle (' + delta.new_points.length + ' pts, r≈' +
          (Math.sqrt(radiusSq) * gs).toFixed(2) + ')');
      };

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
