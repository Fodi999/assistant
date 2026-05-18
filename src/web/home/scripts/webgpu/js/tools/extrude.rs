// ── Edge Extrude — Blender-style wall surface generator ──────────────────────
// Behaviour:
//   E           → enter extrude mode (validates selection first)
//   type digits → build height in mm (e.g. "2500")
//   Enter       → commit: create top points + wall edges + store wallSurface
//   Esc         → cancel extrude
//
// Selection:
//   - uses selectedEdgeIds (one or many)
//   - falls back to selectedProfileId edges
//   - requires at least 1 valid edge
//
// Direction (per active plane):
//   XZ → Y  (height)
//   XY → Z  (depth)
//   YZ → X  (width)
//
// Data model: sketchState.wallSurfaces[{id,sourceEdgeId,bottomA,bottomB,topA,topB,height}]
// Units: UI mm → world metres (÷ 1000)

pub const JS: &str = r##"
      // ── Extrude direction per working plane ──────────────────
      window.__getExtrudeDir = function(plane) {
        if (plane === 'XY') return { x: 0, y: 0, z: 1 };
        if (plane === 'YZ') return { x: 1, y: 0, z: 0 };
        return { x: 0, y: 1, z: 0 };  // XZ default
      };

      // ── Collect edges to extrude ─────────────────────────────
      window.__collectExtrudeEdges = function() {
        const edges = [];
        const pById = new Map(sketchState.points.map(p => [p.id, p]));

        // 1. selected edges
        if (sketchState.selectedEdgeIds.size > 0) {
          for (const eid of sketchState.selectedEdgeIds) {
            const e = sketchState.edges.find(x => x.id === eid);
            if (e && pById.get(e.a) && pById.get(e.b)) edges.push(e);
          }
        }
        // 2. fallback: selected profile edges
        if (!edges.length && sketchState.selectedProfileId) {
          const prof = window.__getProfileById
            ? window.__getProfileById(sketchState.selectedProfileId)
            : null;
          if (prof && prof.edgeIds) {
            for (const eid of prof.edgeIds) {
              const e = sketchState.edges.find(x => x.id === eid);
              if (e && pById.get(e.a) && pById.get(e.b)) edges.push(e);
            }
          }
        }
        return edges;
      };

      // ── Start extrude mode ───────────────────────────────────
      window.__startEdgeExtrude = function() {
        if (sketchState.grab?.active || sketchState.copy?.active) {
          window.__setStatusMessage('Extrude: завершите текущую операцию сначала');
          return;
        }
        if (sketchState.activeTool === 'line' && sketchState.line?.startPointId) {
          window.__setStatusMessage('Extrude: завершите линию сначала');
          return;
        }
        const edges = window.__collectExtrudeEdges();
        if (!edges.length) {
          window.__setStatusMessage('Extrude: выберите линии или профиль');
          return;
        }
        sketchState.extrude.active      = true;
        sketchState.extrude.heightInput = '';
        sketchState.extrude.edgeIds     = edges.map(e => e.id);
        window.__setStatusMessage(
          'Extrude · ' + edges.length + ' рёбер · введите высоту мм · Enter ✓ · Esc ✗'
        );
        if (window.__updateSketchInspector) window.__updateSketchInspector();
      };

      // ── Cancel extrude mode ──────────────────────────────────
      window.__cancelEdgeExtrude = function() {
        sketchState.extrude.active      = false;
        sketchState.extrude.heightInput = '';
        sketchState.extrude.edgeIds     = [];
        window.__setStatusMessage('Extrude отменён');
        if (window.__updateSketchInspector) window.__updateSketchInspector();
      };

      // ── Commit extrude ───────────────────────────────────────
      window.__commitEdgeExtrude = async function() {
        const ex = sketchState.extrude;
        if (!ex.active) return;

        const heightMm = parseFloat(ex.heightInput);
        if (!isFinite(heightMm) || heightMm === 0) {
          window.__setStatusMessage('Extrude: введите высоту в мм, например 2500');
          return;
        }

        const heightM = heightMm / 1000.0;  // mm → metres
        const plane   = sketchState.workingPlane || 'XZ';
        const dir     = window.__getExtrudeDir(plane);
        const pById   = new Map(sketchState.points.map(p => [p.id, p]));
        const gs      = sketchState.gridSize || 0.001;

        const edges = ex.edgeIds
          .map(id => sketchState.edges.find(e => e.id === id))
          .filter(Boolean);

        if (!edges.length) {
          window.__cancelEdgeExtrude();
          return;
        }

        window.__pushHistory();

        // Cache: bottom point id → top point id (shared corners)
        const topPointMap = new Map();

        async function getOrCreateTopPoint(bottomId) {
          if (topPointMap.has(bottomId)) return topPointMap.get(bottomId);
          const bp = pById.get(bottomId);
          if (!bp) return null;
          const tx = bp.x + dir.x * heightM;
          const ty = bp.y + dir.y * heightM;
          const tz = bp.z + dir.z * heightM;
          // Convert world → grid coords
          const tgx = Math.round(tx / gs);
          const tgy = Math.round(ty / gs);
          const tgz = Math.round(tz / gs);
          // Check if top point already exists
          let existing = sketchState.points.find(
            p => p.gx === tgx && p.gy === tgy && p.gz === tgz
          );
          let topId;
          if (existing) {
            topId = existing.id;
          } else {
            topId = await window.__createPointViaEngine(tgx, tgy, tgz);
          }
          topPointMap.set(bottomId, topId);
          return topId;
        }

        const createdWalls = [];

        for (const edge of edges) {
          const bA = pById.get(edge.a);
          const bB = pById.get(edge.b);
          if (!bA || !bB) continue;

          const topAId = await getOrCreateTopPoint(edge.a);
          const topBId = await getOrCreateTopPoint(edge.b);
          if (!topAId || !topBId) continue;

          // Top edge A–B
          const existTopEdge = sketchState.edges.find(
            e => (e.a === topAId && e.b === topBId) || (e.a === topBId && e.b === topAId)
          );
          if (!existTopEdge) {
            await window.__createEdgeViaEngine(topAId, topBId, 'normal');
          }

          // Vertical edge A
          const existVertA = sketchState.edges.find(
            e => (e.a === edge.a && e.b === topAId) || (e.a === topAId && e.b === edge.a)
          );
          if (!existVertA) {
            await window.__createEdgeViaEngine(edge.a, topAId, 'normal');
          }

          // Vertical edge B
          const existVertB = sketchState.edges.find(
            e => (e.a === edge.b && e.b === topBId) || (e.a === topBId && e.b === edge.b)
          );
          if (!existVertB) {
            await window.__createEdgeViaEngine(edge.b, topBId, 'normal');
          }

          // Refresh pById after point creation
          const pByIdFresh = new Map(sketchState.points.map(p => [p.id, p]));
          const tA = pByIdFresh.get(topAId);
          const tB = pByIdFresh.get(topBId);
          if (!tA || !tB) continue;

          // Wall surface record
          const wallId = 'wall_' + Date.now() + '_' + Math.random().toString(36).slice(2, 7);
          createdWalls.push({
            id:           wallId,
            sourceEdgeId: edge.id,
            bottomA:      { x: bA.x, y: bA.y, z: bA.z },
            bottomB:      { x: bB.x, y: bB.y, z: bB.z },
            topA:         { x: tA.x, y: tA.y, z: tA.z },
            topB:         { x: tB.x, y: tB.y, z: tB.z },
            height:       heightM,
            plane:        plane,
            topAId:       topAId,
            topBId:       topBId,
          });
        }

        // Append wall surfaces
        sketchState.wallSurfaces.push(...createdWalls);

        // Reset extrude state
        ex.active      = false;
        ex.heightInput = '';
        ex.edgeIds     = [];

        if (window.__notifySketchChanged)   window.__notifySketchChanged();
        if (window.__updateSketchInspector) window.__updateSketchInspector();

        const heightDisplay = Math.abs(heightMm).toFixed(0);
        window.__setStatusMessage(
          '✓ Extrude ' + heightDisplay + ' мм · ' + createdWalls.length + ' стен создано'
        );
        console.log('[Extrude] committed', createdWalls.length, 'walls, h=', heightM, 'm');
      };

      // ── Extrude numeric key handler (called from hotkeys.rs) ─
      window.__handleExtrudeKey = function(e) {
        if (!sketchState.extrude.active) return false;
        const k = e.key.toLowerCase();

        if (k === 'escape') {
          window.__cancelEdgeExtrude();
          e.preventDefault();
          return true;
        }
        if (k === 'enter') {
          window.__commitEdgeExtrude();
          e.preventDefault();
          return true;
        }
        // Digits + sign + decimal
        if (/^[0-9]$/.test(e.key) ||
            (e.key === '-' && sketchState.extrude.heightInput === '') ||
            e.key === '.') {
          sketchState.extrude.heightInput += e.key;
          const mm = parseFloat(sketchState.extrude.heightInput);
          const cnt = sketchState.extrude.edgeIds.length;
          window.__setStatusMessage(
            'Extrude · ' + cnt + ' рёбер · ' +
            (isFinite(mm) ? mm.toFixed(0) + ' мм' : '…') +
            ' · Enter ✓ · Esc ✗'
          );
          if (window.__updateSketchInspector) window.__updateSketchInspector();
          e.preventDefault();
          return true;
        }
        if (k === 'backspace') {
          sketchState.extrude.heightInput = sketchState.extrude.heightInput.slice(0, -1);
          if (window.__updateSketchInspector) window.__updateSketchInspector();
          e.preventDefault();
          return true;
        }
        return true; // consume all keys while extrude active
      };
"##;

