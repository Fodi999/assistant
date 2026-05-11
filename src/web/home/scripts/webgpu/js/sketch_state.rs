// ── JS: Sketch data model — wireframe core + constraints + profiles ─────────
// Domain: Sketch — id-based SketchGraph + helpers + history + validation.

pub const JS: &str = r##"
      // ── Selection mode constants ───────────────────────────────
      window.SelectionMode = Object.freeze({
        SELECT: "select", POINT: "point", LINE: "line",
        GRAB:   "grab",   DELETE: "delete",
      });

      // ── Sketch State ───────────────────────────────────────────
      const sketchState = {
        points: [], edges: [],
        constraints: [],
        profiles: [],
        validation: { isolatedIds: [], openEndIds: [] },
        showValidation: true,
        statusMessage: null,

        selectedPointIds: new Set(),
        selectedEdgeIds:  new Set(),

        hoverPointId: null,
        hoverEdgeId:  null,
        hoverWorld:   null,
        hoverGridFree: null,

        activeTool: "select",
        phase:      "idle",

        // ── Line tool state ──
        line: {
          startPointId: null,
          previewPoint: null,
          previewLength: 0,
          previewValid: true,
        },

        // ── Snap status ──
        snap: { kind: "grid", pointId: null, gx: 0, gy: 0, gz: 0 },

        // ── Grid / plane ──
        gridSize: 1.0,
        workingPlane: "XZ",
        showGrid: true,
        plane: "XZ",

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
      let __pointCounter = 0, __edgeCounter = 0, __constraintCounter = 0, __profileCounter = 0;
      window.__nextPointId      = () => "p_" + (++__pointCounter);
      window.__nextEdgeId       = () => "e_" + (++__edgeCounter);
      window.__nextConstraintId = () => "c_" + (++__constraintCounter);

      // ── Status message (auto-clearing) ─────────────────────────
      let __statusTimer = null;
      window.__setStatusMessage = function(msg, ttl) {
        sketchState.statusMessage = msg || null;
        if (__statusTimer) clearTimeout(__statusTimer);
        if (msg) {
          __statusTimer = setTimeout(() => { sketchState.statusMessage = null; }, ttl || 2500);
          if (typeof log === 'function') log(msg, '#fbbf24');
        }
      };

      // ── Coordinate helpers ─────────────────────────────────────
      window.__gridToWorld = function(gx, gy, gz) {
        const g = sketchState.gridSize || 1.0;
        return { x: gx * g, y: gy * g, z: gz * g };
      };
      window.__worldToGrid = function(x, y, z) {
        const g = sketchState.gridSize || 1.0;
        return { gx: Math.round(x / g), gy: Math.round(y / g), gz: Math.round(z / g) };
      };
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
      window.__edgesAtPoint = function(pointId) {
        const out = [];
        for (const e of sketchState.edges) if (e.a === pointId || e.b === pointId) out.push(e.id);
        return out;
      };

      // ── Constraint helpers ─────────────────────────────────────
      window.__getConstraintsForTarget = function(targetId) {
        return sketchState.constraints.filter(c => c.targetId === targetId);
      };
      window.__getConstraintForTarget = function(type, targetId) {
        for (const c of sketchState.constraints) {
          if (c.type === type && c.targetId === targetId) return c;
        }
        return null;
      };
      window.__addConstraint = function(type, targetType, targetId, value) {
        // Replace existing of same type+target.
        const existing = window.__getConstraintForTarget(type, targetId);
        if (existing) { existing.value = (value === undefined) ? existing.value : value; return existing; }
        const c = { id: window.__nextConstraintId(), type, targetType, targetId, value: value == null ? null : value };
        sketchState.constraints.push(c);
        return c;
      };
      window.__removeConstraint = function(constraintId) {
        const before = sketchState.constraints.length;
        sketchState.constraints = sketchState.constraints.filter(c => c.id !== constraintId);
        return sketchState.constraints.length !== before;
      };
      window.__removeConstraintsForTarget = function(targetId) {
        const before = sketchState.constraints.length;
        sketchState.constraints = sketchState.constraints.filter(c => c.targetId !== targetId);
        return sketchState.constraints.length !== before;
      };
      window.__isPointFixed = function(pointId) {
        return !!window.__getConstraintForTarget('fixed_point', pointId);
      };
      window.__getEdgeLengthConstraint = function(edgeId) {
        return window.__getConstraintForTarget('edge_length', edgeId);
      };
      window.__hasHorizontalConstraint = function(edgeId) {
        return !!window.__getConstraintForTarget('horizontal', edgeId);
      };
      window.__hasVerticalConstraint = function(edgeId) {
        return !!window.__getConstraintForTarget('vertical', edgeId);
      };

      // ── Geometry mutators applied by constraints ───────────────
      function __planeClamp(point) {
        const pl = sketchState.workingPlane || 'XZ';
        if (pl === 'XZ') point.y = 0;
        if (pl === 'XY') point.z = 0;
        if (pl === 'YZ') point.x = 0;
      }
      function __refreshGridCoords(point) {
        const g = sketchState.gridSize || 1.0;
        point.gx = Math.round(point.x / g);
        point.gy = Math.round(point.y / g);
        point.gz = Math.round(point.z / g);
      }

      // Apply explicit edge length. Returns true on success.
      window.__applyEdgeLength = function(edge, length) {
        const byId = new Map(sketchState.points.map(p => [p.id, p]));
        const a = byId.get(edge.a), b = byId.get(edge.b);
        if (!a || !b) return false;
        const aFixed = window.__isPointFixed(a.id);
        const bFixed = window.__isPointFixed(b.id);
        if (aFixed && bFixed) {
          window.__setStatusMessage('Cannot dimension edge with both endpoints fixed');
          return false;
        }
        let dx = b.x - a.x, dy = b.y - a.y, dz = b.z - a.z;
        let len = Math.hypot(dx, dy, dz);
        if (len < 1e-6) {
          // Fallback direction: plane-horizontal axis.
          const pl = sketchState.workingPlane || 'XZ';
          if (pl === 'YZ') { dx = 0; dy = 0; dz = 1; }
          else             { dx = 1; dy = 0; dz = 0; }
          len = 1;
        }
        const ux = dx / len, uy = dy / len, uz = dz / len;
        const moveB = !bFixed;
        const target = moveB ? b : a;
        const anchor = moveB ? a : b;
        const sign   = moveB ? 1 : -1;
        target.x = anchor.x + sign * ux * length;
        target.y = anchor.y + sign * uy * length;
        target.z = anchor.z + sign * uz * length;
        __planeClamp(target);
        __refreshGridCoords(target);
        return true;
      };

      // Align edge along a world axis ('x' | 'y' | 'z') by moving one endpoint.
      function __applyAxisAlign(edge, axis) {
        const byId = new Map(sketchState.points.map(p => [p.id, p]));
        const a = byId.get(edge.a), b = byId.get(edge.b);
        if (!a || !b) return false;
        const aFixed = window.__isPointFixed(a.id);
        const bFixed = window.__isPointFixed(b.id);
        if (aFixed && bFixed) {
          window.__setStatusMessage('Cannot constrain edge with both endpoints fixed');
          return false;
        }
        const moveB = !bFixed;
        const target = moveB ? b : a;
        const anchor = moveB ? a : b;
        if (axis === 'x') { target.y = anchor.y; target.z = anchor.z; }
        if (axis === 'y') { target.x = anchor.x; target.z = anchor.z; }
        if (axis === 'z') { target.x = anchor.x; target.y = anchor.y; }
        __planeClamp(target);
        __refreshGridCoords(target);
        return true;
      }
      window.__applyHorizontal = function(edge) {
        const pl = sketchState.workingPlane || 'XZ';
        // horizontal axis: XZ→X, XY→X, YZ→Z
        const axis = (pl === 'YZ') ? 'z' : 'x';
        return __applyAxisAlign(edge, axis);
      };
      window.__applyVertical = function(edge) {
        const pl = sketchState.workingPlane || 'XZ';
        // vertical axis: XZ→Z, XY→Y, YZ→Y
        const axis = (pl === 'XZ') ? 'z' : 'y';
        return __applyAxisAlign(edge, axis);
      };

      // ── Validation ─────────────────────────────────────────────
      window.__countOpenEnds = function() {
        return window.__getOpenEndPointIds().length;
      };
      window.__countIsolatedPoints = function() {
        return window.__getIsolatedPointIds().length;
      };
      window.__getOpenEndPointIds = function() {
        const deg = new Map();
        for (const e of sketchState.edges) {
          deg.set(e.a, (deg.get(e.a) || 0) + 1);
          deg.set(e.b, (deg.get(e.b) || 0) + 1);
        }
        const out = [];
        for (const p of sketchState.points) {
          if ((deg.get(p.id) || 0) === 1) out.push(p.id);
        }
        return out;
      };
      window.__getIsolatedPointIds = function() {
        const deg = new Map();
        for (const e of sketchState.edges) {
          deg.set(e.a, (deg.get(e.a) || 0) + 1);
          deg.set(e.b, (deg.get(e.b) || 0) + 1);
        }
        const out = [];
        for (const p of sketchState.points) {
          if ((deg.get(p.id) || 0) === 0) out.push(p.id);
        }
        return out;
      };
      window.__recomputeValidation = function() {
        sketchState.validation.isolatedIds = window.__getIsolatedPointIds();
        sketchState.validation.openEndIds  = window.__getOpenEndPointIds();
      };

      // ── Profile detection (simple cycles, length 3..20) ────────
      function __detectProfilePlane(pointIds) {
        const byId = new Map(sketchState.points.map(p => [p.id, p]));
        const pts  = pointIds.map(id => byId.get(id)).filter(Boolean);
        if (!pts.length) return 'unknown';
        const eps = 1e-4;
        const sameY = pts.every(p => Math.abs(p.y - pts[0].y) < eps);
        if (sameY) return 'XZ';
        const sameZ = pts.every(p => Math.abs(p.z - pts[0].z) < eps);
        if (sameZ) return 'XY';
        const sameX = pts.every(p => Math.abs(p.x - pts[0].x) < eps);
        if (sameX) return 'YZ';
        return 'unknown';
      }

      function __findSimpleCycles() {
        const N = sketchState.points.length;
        if (N < 3) return [];
        const idIdx = new Map();
        sketchState.points.forEach((p, i) => idIdx.set(p.id, i));
        const adj = Array.from({ length: N }, () => []);
        const edgeBetween = new Map();
        for (const e of sketchState.edges) {
          const ai = idIdx.get(e.a), bi = idIdx.get(e.b);
          if (ai == null || bi == null || ai === bi) continue;
          adj[ai].push(bi);
          adj[bi].push(ai);
          const k = ai < bi ? (ai + '-' + bi) : (bi + '-' + ai);
          edgeBetween.set(k, e.id);
        }
        const seen = new Set();
        const cycles = [];
        const MAX_LEN  = 20;
        const MAX_CYCLES = 64;
        function dfs(start, u, path, onPath) {
          if (cycles.length >= MAX_CYCLES) return;
          if (path.length > MAX_LEN) return;
          const neighbors = adj[u];
          for (let i = 0; i < neighbors.length; i++) {
            const v = neighbors[i];
            if (v < start) continue;
            if (v === start && path.length >= 3) {
              const second = path[1], last = path[path.length - 1];
              const canonical = (second <= last) ? path.slice() : [path[0], ...path.slice(1).reverse()];
              const key = canonical.join(',');
              if (!seen.has(key)) { seen.add(key); cycles.push(canonical); }
              continue;
            }
            if (onPath[v]) continue;
            onPath[v] = 1;
            path.push(v);
            dfs(start, v, path, onPath);
            path.pop();
            onPath[v] = 0;
          }
        }
        for (let i = 0; i < N; i++) {
          if (cycles.length >= MAX_CYCLES) break;
          const onPath = new Uint8Array(N);
          onPath[i] = 1;
          dfs(i, i, [i], onPath);
        }
        // Map index cycles → point/edge id cycles.
        return cycles.map(cyc => {
          const pointIds = cyc.map(idx => sketchState.points[idx].id);
          const edgeIds  = [];
          for (let i = 0; i < cyc.length; i++) {
            const a = cyc[i], b = cyc[(i + 1) % cyc.length];
            const k = a < b ? (a + '-' + b) : (b + '-' + a);
            const eid = edgeBetween.get(k);
            if (eid) edgeIds.push(eid);
          }
          return { pointIds, edgeIds };
        });
      }

      window.__recomputeProfiles = function() {
        __profileCounter = 0;
        const raw = __findSimpleCycles();
        sketchState.profiles = raw.map(cyc => ({
          id: 'profile_' + (++__profileCounter),
          pointIds: cyc.pointIds,
          edgeIds:  cyc.edgeIds,
          plane:    __detectProfilePlane(cyc.pointIds),
          closed:   true,
        }));
      };

      window.__getProfileForEdge = function(edgeId) {
        for (const prof of sketchState.profiles) {
          if (prof.edgeIds.indexOf(edgeId) !== -1) return prof;
        }
        return null;
      };

      window.__notifySketchChanged = function() {
        window.__recomputeValidation();
        window.__recomputeProfiles();
      };

      // ── Mutations ──────────────────────────────────────────────
      window.__addPoint = function(gx, gy, gz) {
        const existing = window.__findPointAtGrid(gx, gy, gz);
        if (existing) return existing;
        const g = sketchState.gridSize || 1.0;
        const p = { id: window.__nextPointId(), gx, gy, gz, x: gx * g, y: gy * g, z: gz * g };
        sketchState.points.push(p);
        window.__notifySketchChanged();
        return p;
      };
      window.__addEdge = function(aId, bId) {
        if (!aId || !bId || aId === bId) return null;
        const existing = window.__findEdgeBetween(aId, bId);
        if (existing) return existing;
        const e = { id: window.__nextEdgeId(), a: aId, b: bId };
        sketchState.edges.push(e);
        window.__notifySketchChanged();
        return e;
      };
      window.__deleteSelected = function() {
        const sp = sketchState.selectedPointIds;
        const se = sketchState.selectedEdgeIds;
        if (sp.size === 0 && se.size === 0) return false;
        // Determine edges to remove (selected + incident on removed points).
        const removedEdges = new Set();
        for (const e of sketchState.edges) {
          if (se.has(e.id) || sp.has(e.a) || sp.has(e.b)) removedEdges.add(e.id);
        }
        sketchState.edges  = sketchState.edges.filter(e => !removedEdges.has(e.id));
        sketchState.points = sketchState.points.filter(p => !sp.has(p.id));
        // Cascade constraint removal.
        sketchState.constraints = sketchState.constraints.filter(c => {
          if (c.targetType === 'point' && sp.has(c.targetId)) return false;
          if (c.targetType === 'edge'  && removedEdges.has(c.targetId)) return false;
          return true;
        });
        sketchState.selectedPointIds = new Set();
        sketchState.selectedEdgeIds  = new Set();
        if (sketchState.hoverPointId && !sketchState.points.some(p => p.id === sketchState.hoverPointId)) sketchState.hoverPointId = null;
        if (sketchState.hoverEdgeId  && !sketchState.edges.some(e  => e.id === sketchState.hoverEdgeId )) sketchState.hoverEdgeId  = null;
        if (sketchState.line.startPointId && !sketchState.points.some(p => p.id === sketchState.line.startPointId)) {
          sketchState.line.startPointId = null;
        }
        window.__notifySketchChanged();
        return true;
      };

      // ── Working plane ──────────────────────────────────────────
      window.__setWorkingPlane = function(plane) {
        if (!['XZ','XY','YZ'].includes(plane)) return;
        sketchState.workingPlane = plane;
        sketchState.plane         = plane;
        sketchState.hoverWorld    = null;
        const el = document.getElementById('si-plane');
        if (el) el.textContent = plane;
        const sb = document.getElementById('mini-plane');
        if (sb) sb.textContent = plane;
        if (typeof log === 'function') log('◇ Working plane → ' + plane, '#67e8f9');
      };

      // ── History (undo / redo) ──────────────────────────────────
      function __cloneSnapshot() {
        return {
          points: sketchState.points.map(p => ({ id:p.id, gx:p.gx, gy:p.gy, gz:p.gz, x:p.x, y:p.y, z:p.z })),
          edges:  sketchState.edges.map(e => ({ id:e.id, a:e.a, b:e.b })),
          constraints: sketchState.constraints.map(c => ({ id:c.id, type:c.type, targetType:c.targetType, targetId:c.targetId, value:c.value })),
          selPts: [...sketchState.selectedPointIds],
          selEds: [...sketchState.selectedEdgeIds],
          pCtr:   __pointCounter,
          eCtr:   __edgeCounter,
          cCtr:   __constraintCounter,
        };
      }
      function __applySnapshot(s) {
        sketchState.points = s.points.map(p => ({ ...p }));
        sketchState.edges  = s.edges.map(e => ({ ...e }));
        sketchState.constraints = (s.constraints || []).map(c => ({ ...c }));
        sketchState.selectedPointIds = new Set(s.selPts);
        sketchState.selectedEdgeIds  = new Set(s.selEds);
        __pointCounter      = s.pCtr;
        __edgeCounter       = s.eCtr;
        __constraintCounter = s.cCtr || 0;
        sketchState.hoverPointId = null;
        sketchState.hoverEdgeId  = null;
        sketchState.line.startPointId = null;
        window.__notifySketchChanged();
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
        if (typeof log === 'function') log('↶ undo', '#a78bfa');
        return true;
      };
      window.__redo = function() {
        const hist = sketchState._history;
        if (!hist.redo.length) return false;
        hist.undo.push(__cloneSnapshot());
        __applySnapshot(hist.redo.pop());
        if (typeof log === 'function') log('↷ redo', '#a78bfa');
        return true;
      };

      // ── Serialisation ──────────────────────────────────────────
      window.__sketchToJSON = function() {
        return {
          points: sketchState.points.map(p => ({ id:p.id, gx:p.gx, gy:p.gy, gz:p.gz, x:p.x, y:p.y, z:p.z })),
          edges:  sketchState.edges.map(e => ({ id:e.id, a:e.a, b:e.b })),
          constraints: sketchState.constraints.map(c => ({ ...c })),
          workingPlane: sketchState.workingPlane,
        };
      };
"##;
