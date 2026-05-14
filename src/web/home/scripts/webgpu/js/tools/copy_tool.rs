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
        const startWorld = sketchState.hoverWorld
          ? { x: sketchState.hoverWorld.x, y: sketchState.hoverWorld.y, z: sketchState.hoverWorld.z }
          : { x: 0, y: 0, z: 0 };
        const cpStartScreen = (sketchState.precision && sketchState.precision.lastMouseScreen)
          ? { x: sketchState.precision.lastMouseScreen.x, y: sketchState.precision.lastMouseScreen.y }
          : { x: 0, y: 0 };
        sketchState.copy = {
          active: true,
          source: src.source,
          pointIds: validIds,
          edges: src.edges,
          originals,
          startMouseWorld: startWorld,
          startScreen: cpStartScreen,
          delta: { dx: 0, dy: 0, dz: 0 },
          baseDelta: { dx: 0, dy: 0, dz: 0 },
          axisLock: null,
        };
        const label = src.source === 'profile' ? 'profile'
                    : src.source === 'edges'   ? (src.edges.length + ' edge' + (src.edges.length === 1 ? '' : 's'))
                    : (validIds.length + ' pt');
        window.__setStatusMessage('⎘ Copy Connect ' + label + ' — X/Y/Z lock · Enter confirm · Esc cancel');
        if (window.__updateSketchInspector) window.__updateSketchInspector();
      };

      window.__copyAxisToggle = function(axis) {
        sketchState.copy.axisLock = (sketchState.copy.axisLock === axis) ? null : axis;
      };

      // ─────────────────────────────────────────────────────────
      // __updateCopyConnect(hoverWorld) — mouse move delta
      // ─────────────────────────────────────────────────────────
      window.__updateCopyConnect = function(hoverWorld) {
        const cp = sketchState.copy;
        if (!cp.active || !hoverWorld || !cp.startMouseWorld) return;
        if (!cp.baseDelta) return;

        let dx = hoverWorld.x - cp.startMouseWorld.x;
        let dy = hoverWorld.y - cp.startMouseWorld.y;
        let dz = hoverWorld.z - cp.startMouseWorld.z;

        if      (cp.axisLock === 'X')  { dy = 0; dz = 0; }
        else if (cp.axisLock === 'Y')  { dx = 0; dz = 0; }
        else if (cp.axisLock === 'Z')  { dx = 0; dy = 0; }
        else if (cp.axisLock === 'XY') { dz = 0; }
        else if (cp.axisLock === 'YZ') { dx = 0; }
        else if (cp.axisLock === 'XZ') { dy = 0; }

        const g = sketchState.gridSize || 1.0;
        cp.delta.dx = cp.baseDelta.dx + Math.round(dx / g) * g;
        cp.delta.dy = cp.baseDelta.dy + Math.round(dy / g) * g;
        cp.delta.dz = cp.baseDelta.dz + Math.round(dz / g) * g;
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
          originals: new Map(), startMouseWorld: null,
          delta: { dx: 0, dy: 0, dz: 0 }, baseDelta: { dx: 0, dy: 0, dz: 0 }, axisLock: null,
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
          originals: new Map(), startMouseWorld: null,
          delta: { dx: 0, dy: 0, dz: 0 }, baseDelta: { dx: 0, dy: 0, dz: 0 }, axisLock: null,
        };
        window.__setStatusMessage('Copy Connect cancelled');
        if (window.__updateSketchInspector) window.__updateSketchInspector();
      };
"##;
