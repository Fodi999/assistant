// ── Copy Connect Tool ────────────────────────────────────────────────────────
// Handles:
//   __startCopyConnect   — Shift+G start
//   __updateCopyConnect  — mouse move delta
//   __confirmCopyConnect — Enter / click confirm
//   __cancelCopyConnect  — Esc cancel

pub const JS: &str = r##"
      // ─────────────────────────────────────────────────────────
      // __collectCopySource() — resolve what to copy from selection
      // ─────────────────────────────────────────────────────────
      function __collectCopySource() {
        const eById = new Map(sketchState.edges.map(e => [e.id, e]));

        if (sketchState.selectedProfileId) {
          const prof = window.__getProfileById
            ? window.__getProfileById(sketchState.selectedProfileId)
            : null;
          if (prof && prof.pointIds && prof.pointIds.length) {
            const edges = (prof.edgeIds || [])
              .map(id => eById.get(id))
              .filter(Boolean)
              .map(e => [e.a, e.b, e.kind || 'normal']);
            return { source: 'profile', pointIds: [...prof.pointIds], edges };
          }
        }
        if (sketchState.selectedEdgeIds && sketchState.selectedEdgeIds.size > 0) {
          const ptSet = new Set();
          const edges = [];
          for (const eid of sketchState.selectedEdgeIds) {
            const e = eById.get(eid);
            if (!e) continue;
            ptSet.add(e.a); ptSet.add(e.b);
            edges.push([e.a, e.b, e.kind || 'normal']);
          }
          if (ptSet.size) return { source: 'edges', pointIds: [...ptSet], edges };
        }
        if (sketchState.selectedPointIds && sketchState.selectedPointIds.size > 0) {
          return { source: 'points', pointIds: [...sketchState.selectedPointIds], edges: [] };
        }
        return null;
      }

      // ─────────────────────────────────────────────────────────
      // __startCopyConnect() — Shift+G
      // ─────────────────────────────────────────────────────────
      window.__startCopyConnect = function() {
        if (sketchState.grab.active) {
          window.__setStatusMessage('Finish grab first');
          return;
        }
        const src = __collectCopySource();
        if (!src) {
          window.__setStatusMessage('Select profile, edges, or points to copy');
          return;
        }
        const byId = new Map(sketchState.points.map(p => [p.id, p]));
        const originals = new Map();
        const validIds = [];
        for (const id of src.pointIds) {
          const p = byId.get(id);
          if (!p) continue;
          originals.set(id, { x: p.x, y: p.y, z: p.z });
          validIds.push(id);
        }
        if (!validIds.length) {
          window.__setStatusMessage('Copy: nothing valid to copy');
          return;
        }
        const cpStartScreen = (sketchState.precision && sketchState.precision.lastMouseScreen)
          ? { x: sketchState.precision.lastMouseScreen.x, y: sketchState.precision.lastMouseScreen.y }
          : { x: 0, y: 0 };
        sketchState.copy = {
          active: true,
          source: src.source,
          pointIds: validIds,
          edges: src.edges,
          originals,
          startScreen: cpStartScreen,
          delta: { dx: 0, dy: 0, dz: 0 },
          axisLock: null,
        };
        const label = src.source === 'profile' ? 'profile'
                    : src.source === 'edges'   ? (src.edges.length + ' edge' + (src.edges.length === 1 ? '' : 's'))
                    : (validIds.length + ' pt');
        window.__setStatusMessage('⎘ Copy Connect ' + label + ' — X/Y/Z lock · Enter confirm · Esc cancel');
        if (window.__updateSketchInspector) window.__updateSketchInspector();
      };

      window.__copyAxisToggle = function(axis) {
        const cp = sketchState.copy;
        cp.axisLock = (cp.axisLock === axis) ? null : axis;
        // Re-anchor start screen so delta resets from current position
        const cur = sketchState.precision?.lastMouseScreen;
        if (cur) cp.startScreen = { x: cur.x, y: cur.y };
        cp.delta = { dx: 0, dy: 0, dz: 0 };
        const lockName = cp.axisLock || 'free';
        if (window.__setStatusMessage) {
          window.__setStatusMessage('⎘ Copy Connect · ' + lockName.toUpperCase() + ' · move mouse · Enter ✓ · Esc ✗');
        }
      };

      // ─────────────────────────────────────────────────────────
      // __updateCopyConnect() — screen-space delta formula
      // ─────────────────────────────────────────────────────────
      window.__updateCopyConnect = function() {
        const cp = sketchState.copy;
        if (!cp?.active) return;

        const curScreen   = sketchState.precision?.lastMouseScreen;
        const startScreen = cp.startScreen;
        if (!curScreen || !startScreen) return;

        const g = sketchState.gridSize || 1.0;
        const worldPerPixel = cam.dist / canvas.height;

        const screenDx = curScreen.x - startScreen.x;
        const screenDy = curScreen.y - startScreen.y;

        let dx = 0, dy = 0, dz = 0;

        if      (cp.axisLock === 'Y')  { dy = -screenDy * worldPerPixel; }
        else if (cp.axisLock === 'X')  { dx =  screenDx * worldPerPixel; }
        else if (cp.axisLock === 'Z')  { dz = -screenDy * worldPerPixel; }
        else if (cp.axisLock === 'XY') { dx =  screenDx * worldPerPixel; dy = -screenDy * worldPerPixel; }
        else if (cp.axisLock === 'XZ') { dx =  screenDx * worldPerPixel; dz = -screenDy * worldPerPixel; }
        else if (cp.axisLock === 'YZ') { dz =  screenDx * worldPerPixel; dy = -screenDy * worldPerPixel; }
        else {
          // Free: map screen X → world X, screen Y → world Z (sketch plane)
          dx = screenDx * worldPerPixel;
          dz = -screenDy * worldPerPixel;
        }

        cp.delta.dx = Math.round(dx / g) * g;
        cp.delta.dy = Math.round(dy / g) * g;
        cp.delta.dz = Math.round(dz / g) * g;

        if (window.__setStatusMessage) {
          window.__setStatusMessage(
            '⎘ Copy Connect · Δ ' +
            cp.delta.dx.toFixed(2) + ', ' +
            cp.delta.dy.toFixed(2) + ', ' +
            cp.delta.dz.toFixed(2) +
            (cp.axisLock ? ' · ' + cp.axisLock : '') +
            ' · Enter ✓ · Esc ✗'
          );
        }
      };

      // ─────────────────────────────────────────────────────────
      // __confirmCopyConnect() — Enter / click confirm
      // ─────────────────────────────────────────────────────────
      window.__confirmCopyConnect = async function() {
        const cp = sketchState.copy;
        if (!cp.active) return;
        const { dx, dy, dz } = cp.delta;
        if (dx === 0 && dy === 0 && dz === 0) {
          window.__setStatusMessage('Copy: zero offset — move cursor first');
          return;
        }
        if (window.__pushHistory) window.__pushHistory();
        const g = sketchState.gridSize || 1.0;
        const origToCopy = new Map();
        const originals = cp.originals;

        for (const id of cp.pointIds) {
          const orig = originals.get(id);
          if (!orig) continue;
          const gx = Math.round((orig.x + dx) / g);
          const gy = Math.round((orig.y + dy) / g);
          const gz = Math.round((orig.z + dz) / g);
          const newId = await window.__createPointViaEngine(gx, gy, gz);
          if (newId) origToCopy.set(id, newId);
        }
        for (const [a, b, kind] of cp.edges) {
          const a2 = origToCopy.get(a);
          const b2 = origToCopy.get(b);
          if (a2 && b2 && a2 !== b2) {
            await window.__createEdgeViaEngine(a2, b2, kind || 'normal');
          }
        }
        let connectorCount = 0;
        for (const [origId, newId] of origToCopy.entries()) {
          if (!origId || !newId || origId === newId) continue;
          await window.__createEdgeViaEngine(origId, newId, 'normal');
          connectorCount += 1;
        }

        const total = origToCopy.size;
        sketchState.copy = {
          active: false, source: null, pointIds: [], edges: [],
          originals: new Map(), startScreen: null,
          delta: { dx: 0, dy: 0, dz: 0 }, axisLock: null,
        };
        if (window.__notifySketchChanged) window.__notifySketchChanged();
        window.__setStatusMessage(
          '⎘ Copy Connect ✓ ' + total + ' pt · ' + cp.edges.length + ' edge · ' + connectorCount + ' connector'
        );
        if (window.__updateSketchInspector) window.__updateSketchInspector();
      };

      // ─────────────────────────────────────────────────────────
      // __cancelCopyConnect() — Esc
      // ─────────────────────────────────────────────────────────
      window.__cancelCopyConnect = function() {
        sketchState.copy = {
          active: false, source: null, pointIds: [], edges: [],
          originals: new Map(), startScreen: null,
          delta: { dx: 0, dy: 0, dz: 0 }, axisLock: null,
        };
        window.__setStatusMessage('Copy Connect cancelled');
        if (window.__updateSketchInspector) window.__updateSketchInspector();
      };
"##;
