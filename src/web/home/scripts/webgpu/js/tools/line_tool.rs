// ── Line Tool ────────────────────────────────────────────────────────────────
// Handles:
//   __lineClick(ndcX, ndcY)   — click dispatch (1st click = start, 2nd = edge)
//   __finishLineClick(snap)   — create edge A→B, reset state
//
// Hotkey: L
// State:  sketchState.line = { active, startPointId, startWorld }
//
// Flow:
//   Click 1 on point/grid → save startPointId + startWorld
//   Click 2 on point/grid → createEdge(A, B), reset
//   Enter / Esc            → finish mode without creating edge

pub const JS: &str = r##"
      // ─────────────────────────────────────────────────────────
      // __lineClick(ndcX, ndcY) — Line Tool click handler
      //
      // Click 1 on point/grid → save startPointId + startWorld
      // Click 2 on point/grid → create edge A→B, reset line state
      // Enter → finish mode (no edge)
      // ─────────────────────────────────────────────────────────
      window.__lineClick = function(ndcX, ndcY) {
        // ── Priority 0: fresh pick at click-time NDC ──────────────────────────
        // Most reliable — avoids timing gap between pointermove and pointerup.
        const freshPickId   = window.__pickPointAt ? window.__pickPointAt(ndcX, ndcY) : null;
        const freshPickData = freshPickId ? sketchState.points.find(p => p.id === freshPickId) : null;

        let snap = null;
        if (freshPickId && freshPickData) {
          snap = {
            kind: 'point', pointId: freshPickId,
            gx: freshPickData.gx, gy: freshPickData.gy, gz: freshPickData.gz,
            x: freshPickData.x,   y: freshPickData.y,   z: freshPickData.z,
            valid: true,
          };
        } else {
          // ── Priority 1: hoverWorld from last pointermove ──────────────────
          const hw = sketchState.hoverWorld;
          if (hw) {
            const snapPtId = (sketchState.snap && sketchState.snap.kind === 'point')
              ? sketchState.snap.pointId : null;
            snap = {
              kind: snapPtId ? 'point' : (hw.snapKind || 'grid'),
              pointId: snapPtId || null,
              gx: hw.gx, gy: hw.gy, gz: hw.gz,
              x: hw.x,   y: hw.y,   z: hw.z,
              valid: true,
            };
          } else {
            // ── Priority 2: plane raycast (last resort) ───────────────────
            const hit = window.__raycastSketchPlane && window.__raycastSketchPlane(ndcX, ndcY);
            if (hit && window.__resolveSnapTarget) {
              const canvasEl = document.getElementById('matterCanvas');
              const mpx = canvasEl
                ? { x: (ndcX + 1) * 0.5 * canvasEl.width, y: (1 - ndcY) * 0.5 * canvasEl.height }
                : { x: 0, y: 0 };
              snap = window.__resolveSnapTarget(
                { x: hit.freeX, y: hit.freeY, z: hit.freeZ }, mpx, { force: true }
              );
            }
          }
        }

        if (!snap || !snap.valid) {
          window.__setStatusMessage('Line: no snap target');
          return;
        }

        console.log('[Line] click snap=', snap, 'line=', sketchState.line);

        const line = sketchState.line;

        // ── FIRST CLICK ──────────────────────────────────────────────────────
        if (!line.active) {
          sketchState.line = {
            active: true,
            startPointId: snap.kind === 'point' ? snap.pointId : null,
            startWorld: {
              x: snap.x, y: snap.y, z: snap.z,
              gx: snap.gx, gy: snap.gy, gz: snap.gz,
            },
          };
          const label = snap.kind === 'point'
            ? 'snapped to point · click second point'
            : 'grid · click second point';
          window.__setStatusMessage('⬡ Line start · ' + label);
          return;
        }

        // ── SECOND CLICK ─────────────────────────────────────────────────────
        window.__finishLineClick(snap);
      };

      // ─────────────────────────────────────────────────────────
      // __finishLineClick(snap) — create edge A→B
      // ─────────────────────────────────────────────────────────
      window.__finishLineClick = async function(snap) {
        const line = sketchState.line;
        if (!line || !line.active || !line.startWorld) return;

        const g    = sketchState.gridSize || 1.0;
        const mode = sketchState.engineMode || 'backend';

        let aId = line.startPointId;
        let bId = snap.kind === 'point' ? snap.pointId : null;

        window.__pushHistory();

        // Create start point if it was on grid (no existing point)
        if (!aId) {
          const a  = line.startWorld;
          const gx = Math.round(a.x / g);
          const gy = Math.round(a.y / g);
          const gz = Math.round(a.z / g);
          if (mode === 'backend') {
            const r = await window.__backendAddPoint(gx, gy, gz);
            aId = r?.pointId || null;
          } else if (mode === 'wasm' || mode === 'hybrid') {
            const r = await window.__wasmAddPointAndApply(gx, gy, gz);
            aId = r?.ok ? r.pointId : null;
          } else {
            aId = window.__addPoint(gx, gy, gz)?.id || null;
          }
        }

        // Create end point if it was on grid (no existing point)
        if (!bId) {
          const gx = Math.round(snap.x / g);
          const gy = Math.round(snap.y / g);
          const gz = Math.round(snap.z / g);
          if (mode === 'backend') {
            const r = await window.__backendAddPoint(gx, gy, gz);
            bId = r?.pointId || null;
          } else if (mode === 'wasm' || mode === 'hybrid') {
            const r = await window.__wasmAddPointAndApply(gx, gy, gz);
            bId = r?.ok ? r.pointId : null;
          } else {
            bId = window.__addPoint(gx, gy, gz)?.id || null;
          }
        }

        if (!aId || !bId) {
          window.__setStatusMessage('Line: failed to resolve points');
          sketchState.line = { active: false, startPointId: null, startWorld: null };
          return;
        }
        if (aId === bId) {
          window.__setStatusMessage('Line: same point — click a different location');
          sketchState.line = { active: false, startPointId: null, startWorld: null };
          return;
        }

        // ── Create edge ───────────────────────────────────────────────────────
        if (mode === 'backend') {
          await window.__backendAddEdge({ pointId: aId }, { pointId: bId });
        } else if (mode === 'wasm' || mode === 'hybrid') {
          await window.__wasmAddEdgeAndApply({ pointId: aId }, { pointId: bId });
        } else {
          window.__addEdge(aId, bId);
        }

        sketchState.line  = { active: false, startPointId: null, startWorld: null };
        sketchState.phase = 'idle';

        if (window.__notifySketchChanged)  window.__notifySketchChanged();
        if (window.__updateSketchInspector) window.__updateSketchInspector();
        window.__setStatusMessage('✓ Line created · click to start new line');
      };
"##;
