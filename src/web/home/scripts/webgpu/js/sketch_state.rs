// ── JS: Sketch data model — wireframe core (points + edges) ──────────────────
// Domain: Sketch — id-based SketchGraph + helpers + history.

pub const JS: &str = r##"
      // ── Selection mode constants ───────────────────────────────
      window.SelectionMode = Object.freeze({
        SELECT: "select", POINT: "point", LINE: "line",
        GRAB:   "grab",   DELETE: "delete",
      });

      // ── Sketch State ───────────────────────────────────────────
      const sketchState = {
        points: [], edges: [],
        selectedPointIds: new Set(),
        selectedEdgeIds:  new Set(),

        hoverPointId: null,
        hoverEdgeId:  null,
        hoverWorld:   null,        // { x, y, z, gx, gy, gz } snapped to working plane
        hoverGridFree: null,       // unsnapped intersection (for marker)

        activeTool: "select",
        phase:      "idle",

        // ── Line tool state ──
        line: {
          startPointId: null,      // anchor in chain
          previewPoint: null,      // { gx,gy,gz,x,y,z } ghost end
          previewLength: 0,
          previewValid: true,
        },

        // ── Snap status (for HUD) ──
        snap: { kind: "grid", pointId: null, gx: 0, gy: 0, gz: 0 },

        // ── Grid / plane ──
        gridSize: 1.0,
        workingPlane: "XZ",        // XZ | XY | YZ
        showGrid: true,
        plane: "XZ",               // legacy alias for render_loop

        // ── Grab snapshot ──
        grab: {
          active: false, pointIds: [],
          startMouseWorld: null,
          originalPoints: new Map(),
          axisLock: null,
        },

        // ── Undo/redo ──
        _history: { undo: [], redo: [] },
        _historyLimit: 100,
      };
      window.sketchState = sketchState;

      // ── Id generation ──────────────────────────────────────────
      let __pointCounter = 0, __edgeCounter = 0;
      window.__nextPointId = () => "p_" + (++__pointCounter);
      window.__nextEdgeId  = () => "e_" + (++__edgeCounter);

      // ── Coordinate helpers ─────────────────────────────────────
      window.__gridToWorld = function(gx, gy, gz) {
        const g = sketchState.gridSize || 1.0;
        return { x: gx * g, y: gy * g, z: gz * g };
      };
      window.__worldToGrid = function(x, y, z) {
        const g = sketchState.gridSize || 1.0;
        return { gx: Math.round(x / g), gy: Math.round(y / g), gz: Math.round(z / g) };
      };
      // Snap a free world (x,y,z) to grid, clamping off-plane axis to 0.
      window.__snapWorldToGrid = function(world, plane) {
        const g = sketchState.gridSize || 1.0;
        const pl = plane || sketchState.workingPlane || "XZ";
        let gx = Math.round(world.x / g);
        let gy = Math.round(world.y / g);
        let gz = Math.round(world.z / g);
        if (pl === "XZ") gy = 0;
        if (pl === "XY") gz = 0;
        if (pl === "YZ") gx = 0;
        return { gx, gy, gz, x: gx * g, y: gy * g, z: gz * g };
      };
      window.__findPointAtGrid = function(gx, gy, gz) {
        for (const p of sketchState.points) {
          if (p.gx === gx && p.gy === gy && p.gz === gz) return p;
        }
        return null;
      };
      window.__findEdgeBetween = function(aId, bId) {
        for (const e of sketchState.edges) {
          if ((e.a === aId && e.b === bId) || (e.a === bId && e.b === aId)) return e;
        }
        return null;
      };
      window.__edgeLength = function(edge) {
        const byId = new Map(sketchState.points.map(p => [p.id, p]));
        const a = byId.get(edge.a), b = byId.get(edge.b);
        if (!a || !b) return 0;
        return Math.hypot(b.x - a.x, b.y - a.y, b.z - a.z);
      };
      window.__pointDegree = function(pointId) {
        let n = 0;
        for (const e of sketchState.edges) if (e.a === pointId || e.b === pointId) n++;
        return n;
      };

      // ── Mutations ──────────────────────────────────────────────
      window.__addPoint = function(gx, gy, gz) {
        const existing = window.__findPointAtGrid(gx, gy, gz);
        if (existing) return existing;
        const g = sketchState.gridSize || 1.0;
        const p = { id: window.__nextPointId(), gx, gy, gz, x: gx * g, y: gy * g, z: gz * g };
        sketchState.points.push(p);
        return p;
      };
      window.__addEdge = function(aId, bId) {
        if (!aId || !bId || aId === bId) return null;
        const existing = window.__findEdgeBetween(aId, bId);
        if (existing) return existing;
        const e = { id: window.__nextEdgeId(), a: aId, b: bId };
        sketchState.edges.push(e);
        return e;
      };
      window.__deleteSelected = function() {
        const sp = sketchState.selectedPointIds;
        const se = sketchState.selectedEdgeIds;
        if (sp.size === 0 && se.size === 0) return false;
        sketchState.edges  = sketchState.edges.filter(e =>
          !se.has(e.id) && !sp.has(e.a) && !sp.has(e.b)
        );
        sketchState.points = sketchState.points.filter(p => !sp.has(p.id));
        sketchState.selectedPointIds = new Set();
        sketchState.selectedEdgeIds  = new Set();
        if (sketchState.hoverPointId && !sketchState.points.some(p => p.id === sketchState.hoverPointId)) sketchState.hoverPointId = null;
        if (sketchState.hoverEdgeId  && !sketchState.edges.some(e  => e.id === sketchState.hoverEdgeId )) sketchState.hoverEdgeId  = null;
        if (sketchState.line.startPointId && !sketchState.points.some(p => p.id === sketchState.line.startPointId)) {
          sketchState.line.startPointId = null;
        }
        return true;
      };
      window.__countOpenEnds = function() {
        const deg = new Map();
        for (const e of sketchState.edges) {
          deg.set(e.a, (deg.get(e.a) || 0) + 1);
          deg.set(e.b, (deg.get(e.b) || 0) + 1);
        }
        let n = 0; for (const v of deg.values()) if (v === 1) n++;
        return n;
      };

      // ── Working plane ──────────────────────────────────────────
      window.__setWorkingPlane = function(plane) {
        if (!['XZ','XY','YZ'].includes(plane)) return;
        sketchState.workingPlane = plane;
        sketchState.plane         = plane;       // legacy
        // Reset hover snap so preview jumps to new plane immediately.
        sketchState.hoverWorld    = null;
        const el = document.getElementById('si-plane');
        if (el) el.textContent = plane;
        const sb = document.getElementById('mini-plane');
        if (sb) sb.textContent = plane;
        log('◇ Working plane → ' + plane, '#67e8f9');
      };

      // ── History (undo / redo) ──────────────────────────────────
      function __cloneSnapshot() {
        return {
          points: sketchState.points.map(p => ({ id:p.id, gx:p.gx, gy:p.gy, gz:p.gz, x:p.x, y:p.y, z:p.z })),
          edges:  sketchState.edges.map(e => ({ id:e.id, a:e.a, b:e.b })),
          selPts: [...sketchState.selectedPointIds],
          selEds: [...sketchState.selectedEdgeIds],
          pCtr:   __pointCounter,
          eCtr:   __edgeCounter,
        };
      }
      function __applySnapshot(s) {
        sketchState.points = s.points.map(p => ({ ...p }));
        sketchState.edges  = s.edges.map(e => ({ ...e }));
        sketchState.selectedPointIds = new Set(s.selPts);
        sketchState.selectedEdgeIds  = new Set(s.selEds);
        __pointCounter = s.pCtr;
        __edgeCounter  = s.eCtr;
        sketchState.hoverPointId = null;
        sketchState.hoverEdgeId  = null;
        sketchState.line.startPointId = null;
      }
      window.__pushHistory = function() {
        sketchState._history.undo.push(__cloneSnapshot());
        if (sketchState._history.undo.length > sketchState._historyLimit) sketchState._history.undo.shift();
        sketchState._history.redo.length = 0;
      };
      window.__undo = function() {
        const hist = sketchState._history;
        if (!hist.undo.length) return false;
        hist.redo.push(__cloneSnapshot());
        __applySnapshot(hist.undo.pop());
        log('↶ undo', '#a78bfa');
        return true;
      };
      window.__redo = function() {
        const hist = sketchState._history;
        if (!hist.redo.length) return false;
        hist.undo.push(__cloneSnapshot());
        __applySnapshot(hist.redo.pop());
        log('↷ redo', '#a78bfa');
        return true;
      };

      // ── Serialisation ──────────────────────────────────────────
      window.__sketchToJSON = function() {
        return {
          points: sketchState.points.map(p => ({ id:p.id, gx:p.gx, gy:p.gy, gz:p.gz, x:p.x, y:p.y, z:p.z })),
          edges:  sketchState.edges.map(e => ({ id:e.id, a:e.a, b:e.b })),
          workingPlane: sketchState.workingPlane,
        };
      };
"##;
