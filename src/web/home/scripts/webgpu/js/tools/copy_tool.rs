// ── Copy Connect Tool — thin WASM wrapper ────────────────────────────────────
// Geometry logic lives in geometry_engine::tools::copy (Rust/WASM).
// This file handles: source collection, screen-space delta, axis lock, cancel.
// Commit geometry → wasm_tool_copy → __applySketchDelta.
//
// Hotkey: Shift+G
// State:  sketchState.copy = { active, source, pointIds, edges, originals, ... }

pub const JS: &str = r##"

      function __collectCopySource() {
        const eById = new Map(sketchState.edges.map(e => [e.id, e]));
        if (sketchState.selectedProfileId) {
          const prof = window.__getProfileById
            ? window.__getProfileById(sketchState.selectedProfileId) : null;
          if (prof && prof.pointIds && prof.pointIds.length) {
            const edges = (prof.edgeIds || []).map(id => eById.get(id)).filter(Boolean)
              .map(e => [e.a, e.b, e.kind || 'normal']);
            return { source: 'profile', pointIds: [...prof.pointIds], edges };
          }
        }
        if (sketchState.selectedEdgeIds && sketchState.selectedEdgeIds.size > 0) {
          const ptSet = new Set(); const edges = [];
          for (const eid of sketchState.selectedEdgeIds) {
            const e = eById.get(eid); if (!e) continue;
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

      window.__startCopyConnect = function() {
        if (sketchState.grab.active) {
          window.__setStatusMessage('Finish grab first'); return;
        }
        const src = __collectCopySource();
        if (!src) {
          window.__setStatusMessage('Select profile, edges, or points to copy'); return;
        }
        const byId = new Map(sketchState.points.map(p => [p.id, p]));
        const originals = new Map();
        const validIds  = [];
        for (const id of src.pointIds) {
          const p = byId.get(id); if (!p) continue;
          originals.set(id, { x: p.x, y: p.y, z: p.z });
          validIds.push(id);
        }
        if (!validIds.length) { window.__setStatusMessage('Copy: nothing valid'); return; }
        const cpStartScreen = sketchState.precision?.lastMouseScreen
          ? { x: sketchState.precision.lastMouseScreen.x, y: sketchState.precision.lastMouseScreen.y }
          : { x: 0, y: 0 };
        sketchState.copy = {
          active: true, source: src.source,
          pointIds: validIds, edges: src.edges, originals,
          startScreen: cpStartScreen, delta: { dx: 0, dy: 0, dz: 0 }, axisLock: null,
        };
        const label = src.source === 'profile' ? 'profile'
                    : src.source === 'edges' ? (src.edges.length + ' edges')
                    : (validIds.length + ' pt');
        window.__setStatusMessage('⎘ Copy Connect ' + label + ' — X/Y/Z · Enter ✓ · Esc ✗');
        if (window.__updateSketchInspector) window.__updateSketchInspector();
      };

      window.__copyAxisToggle = function(axis) {
        const cp = sketchState.copy;
        cp.axisLock = (cp.axisLock === axis) ? null : axis;
        const cur = sketchState.precision?.lastMouseScreen;
        if (cur) cp.startScreen = { x: cur.x, y: cur.y };
        cp.delta = { dx: 0, dy: 0, dz: 0 };
        window.__setStatusMessage && window.__setStatusMessage(
          '⎘ Copy Connect · ' + (cp.axisLock || 'FREE') + ' · move mouse · Enter ✓ · Esc ✗');
      };

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
        else { dx = screenDx * worldPerPixel; dz = -screenDy * worldPerPixel; }
        cp.delta.dx = Math.round(dx / g) * g;
        cp.delta.dy = Math.round(dy / g) * g;
        cp.delta.dz = Math.round(dz / g) * g;
        window.__setStatusMessage && window.__setStatusMessage(
          '⎘ Copy · Δ ' + cp.delta.dx.toFixed(2) + ', ' + cp.delta.dy.toFixed(2) +
          ', ' + cp.delta.dz.toFixed(2) + (cp.axisLock ? ' · ' + cp.axisLock : '') +
          ' · Enter ✓ · Esc ✗');
      };

      window.__confirmCopyConnect = async function() {
        const cp = sketchState.copy;
        if (!cp.active) return;
        const { dx, dy, dz } = cp.delta;
        if (dx === 0 && dy === 0 && dz === 0) {
          window.__setStatusMessage('Copy: zero offset — move cursor first'); return;
        }

        const wm = window.__wasmModule;
        if (!wm || typeof wm.wasm_tool_copy !== 'function') {
          window.__setStatusMessage('Copy: WASM not loaded'); return;
        }

        if (window.__pushHistory) window.__pushHistory();

        const byId = new Map(sketchState.points.map(p => [p.id, p]));
        const wasmPoints = cp.pointIds.map(id => {
          const orig = cp.originals.get(id) || {};
          return { id, x: orig.x || 0, y: orig.y || 0, z: orig.z || 0 };
        });
        const wasmEdges = cp.edges.map(([a, b, kind]) => ({ a, b, kind: kind || 'normal' }));

        const raw   = wm.wasm_tool_copy(JSON.stringify({
          points: wasmPoints, edges: wasmEdges,
          dx, dy, dz,
          grid_size: sketchState.gridSize || 0.01,
          id_offset: Date.now() % 1000000000,
        }));
        const delta = JSON.parse(raw);

        if (!delta.ok) {
          window.__setStatusMessage('Copy: ' + (delta.error || 'error')); return;
        }

        window.__applySketchDelta(delta);
        const total = delta.new_points.length;
        const connectors = delta.new_edges.length - wasmEdges.length;

        sketchState.copy = {
          active: false, source: null, pointIds: [], edges: [],
          originals: new Map(), startScreen: null,
          delta: { dx: 0, dy: 0, dz: 0 }, axisLock: null,
        };
        if (window.__notifySketchChanged) window.__notifySketchChanged();
        window.__setStatusMessage(
          '⎘ Copy ✓ ' + total + ' pt · ' + wasmEdges.length + ' edge · ' + connectors + ' connector');
        if (window.__updateSketchInspector) window.__updateSketchInspector();
      };

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
