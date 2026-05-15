// ── Line Tool ────────────────────────────────────────────────────────────────
// CAD-style polyline tool. Each click extends the chain from the last point.
//
// Handles:
//   __lineClick(ndcX, ndcY)   — click dispatch (1st = anchor, 2nd+ = extend chain)
//   __finishLineChain(reason) — terminate chain (Enter / Esc / tool switch)
//   __resolveLineSnap(...)    — fresh snap pick at click moment
//
// Hotkey: L
// State:  sketchState.line = { active, startPointId, startWorld }
//
// Flow:
//   Click 1 → ensure point A exists, set startPointId = A.id
//   Click 2 → ensure point B exists, create edge A→B, set startPointId = B.id
//   Click 3 → ensure point C exists, create edge B→C, set startPointId = C.id
//   ...
//   Enter / Esc → finish chain (no edge), reset state
//
// Snap priority (per click, fresh at click time):
//   0) __pickPointAt(ndcX, ndcY)             — screen-space pick of existing point
//   1) sketchState.hoverWorld + sketchState.snap (from last pointermove)
//   2) __raycastSketchPlane + __resolveSnapTarget — last-resort plane intersection
//
// Existing points have absolute priority — Line Tool never duplicates them,
// and they can be picked even if they don't lie on the current working plane.

pub const JS: &str = r##"
      // ─────────────────────────────────────────────────────────
      // __resolveLineSnap(ndcX, ndcY) — pick snap target at click moment
      // Returns { kind, pointId, x, y, z, gx, gy, gz, valid } or null.
      // ─────────────────────────────────────────────────────────
      window.__resolveLineSnap = function(ndcX, ndcY) {
        // ── Priority 0: fresh screen-space pick of existing point ──
        const freshPickId = window.__pickPointAt ? window.__pickPointAt(ndcX, ndcY) : null;
        if (freshPickId) {
          const p = sketchState.points.find(pt => pt.id === freshPickId);
          if (p) {
            return {
              kind: 'point', pointId: freshPickId,
              x: p.x, y: p.y, z: p.z,
              gx: p.gx, gy: p.gy, gz: p.gz,
              valid: true,
            };
          }
        }

        // ── Priority 1: hoverWorld from last pointermove ──
        const hw = sketchState.hoverWorld;
        if (hw) {
          const snapPtId = (sketchState.snap && sketchState.snap.kind === 'point')
            ? sketchState.snap.pointId : null;
          return {
            kind: snapPtId ? 'point' : (hw.snapKind || 'grid'),
            pointId: snapPtId || null,
            x: hw.x, y: hw.y, z: hw.z,
            gx: hw.gx, gy: hw.gy, gz: hw.gz,
            valid: true,
          };
        }

        // ── Priority 2: plane raycast (last resort) ──
        if (window.__raycastSketchPlane && window.__resolveSnapTarget) {
          const hit = window.__raycastSketchPlane(ndcX, ndcY);
          if (hit) {
            const canvasEl = document.getElementById('matterCanvas');
            const mpx = canvasEl
              ? { x: (ndcX + 1) * 0.5 * canvasEl.width, y: (1 - ndcY) * 0.5 * canvasEl.height }
              : { x: 0, y: 0 };
            const t = window.__resolveSnapTarget(
              { x: hit.freeX, y: hit.freeY, z: hit.freeZ }, mpx, { force: true }
            );
            if (t && t.valid !== false) {
              return {
                kind: t.kind || 'grid',
                pointId: t.pointId || null,
                x: t.x, y: t.y, z: t.z,
                gx: t.gx, gy: t.gy, gz: t.gz,
                valid: true,
              };
            }
          }
        }

        return null;
      };

      // ─────────────────────────────────────────────────────────
      // __lineClick(ndcX, ndcY) — Line Tool click handler (polyline)
      //
      // Click 1     : anchor — ensure point A, set startPointId = A.id
      // Click 2+    : extend — ensure point B, create edge prev→B, startPointId = B.id
      // No edge dup : if click resolves to startPointId, ignore.
      // ─────────────────────────────────────────────────────────
      window.__lineClick = async function(ndcX, ndcY) {
        const snap = window.__resolveLineSnap(ndcX, ndcY);

        console.log('[Line click]', {
          snap,
          line: sketchState.line ? { ...sketchState.line } : null,
          activeTool: sketchState.activeTool,
        });

        if (!snap || !snap.valid) {
          window.__setStatusMessage('Line: no snap target');
          return;
        }

        const line = sketchState.line || { active: false, startPointId: null, startWorld: null };
        const g    = sketchState.gridSize || 1.0;

        // Helper: resolve snap → pointId. Reuses existing point when snap.kind === 'point',
        // else creates a fresh grid point via the unified engine wrapper.
        // __createPointViaEngine itself dedupes via __findPointAtGrid.
        const ensurePoint = async (s) => {
          if (s.kind === 'point' && s.pointId) return s.pointId;
          const gx = Math.round(s.gx !== undefined ? s.gx : s.x / g);
          const gy = Math.round(s.gy !== undefined ? s.gy : s.y / g);
          const gz = Math.round(s.gz !== undefined ? s.gz : s.z / g);
          return await window.__createPointViaEngine(gx, gy, gz);
        };

        // ── FIRST CLICK — anchor the chain ──────────────────────────────────
        if (!line.active) {
          window.__pushHistory();
          const aId = await ensurePoint(snap);
          if (!aId) {
            console.warn('[Line failed] could not create anchor point', { snap });
            window.__setStatusMessage('Line: failed to create start point');
            return;
          }
          // Pull authoritative coords from the actual stored point (snap might be free-world).
          const aPt = sketchState.points.find(p => p.id === aId);
          const startWorld = aPt
            ? { x: aPt.x, y: aPt.y, z: aPt.z, gx: aPt.gx, gy: aPt.gy, gz: aPt.gz }
            : { x: snap.x, y: snap.y, z: snap.z, gx: snap.gx, gy: snap.gy, gz: snap.gz };

          sketchState.line  = { active: true, startPointId: aId, startWorld };
          sketchState.phase = 'line-chain';

          const label = snap.kind === 'point' ? 'snapped to point' : 'on grid';
          window.__setStatusMessage('⬡ Line start (' + label + ') · click next point · Enter/Esc finish');
          if (window.__notifySketchChanged)   window.__notifySketchChanged();
          if (window.__updateSketchInspector) window.__updateSketchInspector();
          return;
        }

        // ── EXTEND CHAIN — every subsequent click ───────────────────────────
        const aId = line.startPointId;
        if (!aId) {
          console.warn('[Line failed] active chain has no startPointId', { line });
          sketchState.line = { active: false, startPointId: null, startWorld: null };
          return;
        }

        window.__pushHistory();
        const bId = await ensurePoint(snap);

        if (!bId) {
          console.warn('[Line failed]', { aId, bId, snap, line });
          window.__setStatusMessage('Line: failed to create next point');
          return;
        }
        if (aId === bId) {
          // Same anchor — don't create degenerate edge but keep chain alive.
          console.warn('[Line] skip — same point as anchor', { aId, snap });
          window.__setStatusMessage('Line: same point — click a different location');
          return;
        }

        // ── Create edge A → B ────────────────────────────────────────────────
        await window.__createEdgeViaEngine(aId, bId, 'normal');
        console.log('[Line edge created]', { aId, bId });

        // ── Continue chain from B ───────────────────────────────────────────
        const bPt = sketchState.points.find(p => p.id === bId);
        const nextStartWorld = bPt
          ? { x: bPt.x, y: bPt.y, z: bPt.z, gx: bPt.gx, gy: bPt.gy, gz: bPt.gz }
          : { x: snap.x, y: snap.y, z: snap.z, gx: snap.gx, gy: snap.gy, gz: snap.gz };

        sketchState.line = {
          active: true,
          startPointId: bId,
          startWorld: nextStartWorld,
        };
        sketchState.phase = 'line-chain';

        if (window.__notifySketchChanged)   window.__notifySketchChanged();
        if (window.__updateSketchInspector) window.__updateSketchInspector();
        window.__setStatusMessage('✓ Edge created · click next point · Enter/Esc finish');
      };

      // ─────────────────────────────────────────────────────────
      // __finishLineChain(reason) — terminate polyline without edge.
      // Called by Enter / Esc / tool switch.
      // ─────────────────────────────────────────────────────────
      window.__finishLineChain = function(reason) {
        const line = sketchState.line;
        const wasActive = !!(line && (line.active || line.startPointId));
        sketchState.line  = { active: false, startPointId: null, startWorld: null };
        sketchState.phase = 'idle';
        if (wasActive) {
          window.__setStatusMessage('Line finished' + (reason ? ' · ' + reason : ''));
        }
        if (window.__updateSketchInspector) window.__updateSketchInspector();
      };
"##;
