// ── Rectangle Tool ───────────────────────────────────────────────────────────
// CAD-style 2-click rectangle: click corner A, click corner B → 4 points +
// 4 edges + HORIZONTAL/VERTICAL constraints applied via WASM solver.
//
// Handles:
//   __rectClick(ndcX, ndcY)   — click dispatch (1st = corner A, 2nd = corner B)
//   __cancelRectTool()        — cancel on Esc / tool switch
//
// Hotkey: R
// State:  sketchState.rect = { active: false, startSnap: null }
//
// Flow:
//   Click 1 → snap corner A → store startSnap, phase = 'rect-corner'
//   Click 2 → snap corner B → create 4 pts + 4 edges + H/V constraints, reset
//
// Preview:
//   render_loop draws dashed rect outline while rect.active && hoverWorld

pub const JS: &str = r##"

      // ─────────────────────────────────────────────────────────
      // __rectClick(ndcX, ndcY) — Rectangle Tool click handler
      // ─────────────────────────────────────────────────────────
      window.__rectClick = async function(ndcX, ndcY) {
        const snap = window.__resolveLineSnap ? window.__resolveLineSnap(ndcX, ndcY) : null;

        if (!snap || !snap.valid) {
          window.__setStatusMessage && window.__setStatusMessage('Rect: no snap target');
          return;
        }

        const rect = sketchState.rect || { active: false, startSnap: null };
        const g    = sketchState.gridSize || 1.0;

        const snapToGrid = (s) => ({
          gx: Math.round(s.gx !== undefined ? s.gx : s.x / g),
          gy: Math.round(s.gy !== undefined ? s.gy : s.y / g),
          gz: Math.round(s.gz !== undefined ? s.gz : s.z / g),
        });

        // ── FIRST CLICK — store corner A ──────────────────────────────────
        if (!rect.active) {
          const g1 = snapToGrid(snap);
          sketchState.rect = { active: true, startSnap: g1 };
          sketchState.phase = 'rect-corner';
          const label = snap.kind === 'point' ? 'snapped to point' : 'on grid';
          window.__setStatusMessage && window.__setStatusMessage(
            '⬡ Rect corner A (' + label + ') · click opposite corner · Esc cancel'
          );
          if (window.__notifySketchChanged)   window.__notifySketchChanged();
          return;
        }

        // ── SECOND CLICK — create rectangle ──────────────────────────────
        const g2 = snapToGrid(snap);
        const g1 = rect.startSnap;

        if (g1.gx === g2.gx || g1.gz === g2.gz) {
          // Degenerate — corners collinear on working plane, keep active
          window.__setStatusMessage && window.__setStatusMessage(
            'Rect: corners must differ in both axes · click another point'
          );
          return;
        }

        window.__pushHistory();

        const plane = sketchState.workingPlane || 'XZ';

        // Build 4 corner grid coords depending on working plane
        // Points go clockwise: TL, TR, BR, BL (when viewed from positive normal)
        let corners;
        if (plane === 'XZ') {
          corners = [
            { gx: g1.gx, gy: 0, gz: g1.gz },
            { gx: g2.gx, gy: 0, gz: g1.gz },
            { gx: g2.gx, gy: 0, gz: g2.gz },
            { gx: g1.gx, gy: 0, gz: g2.gz },
          ];
        } else if (plane === 'XY') {
          corners = [
            { gx: g1.gx, gy: g1.gy, gz: 0 },
            { gx: g2.gx, gy: g1.gy, gz: 0 },
            { gx: g2.gx, gy: g2.gy, gz: 0 },
            { gx: g1.gx, gy: g2.gy, gz: 0 },
          ];
        } else { // YZ
          corners = [
            { gx: 0, gy: g1.gy, gz: g1.gz },
            { gx: 0, gy: g1.gy, gz: g2.gz },
            { gx: 0, gy: g2.gy, gz: g2.gz },
            { gx: 0, gy: g2.gy, gz: g1.gz },
          ];
        }

        // Create the 4 corner points (deduped by __createPointViaEngine)
        const ptIds = [];
        for (const c of corners) {
          const id = await window.__createPointViaEngine(c.gx, c.gy, c.gz);
          if (!id) {
            console.error('[Rect] failed to create point', c);
            window.__setStatusMessage && window.__setStatusMessage('Rect: failed to create point');
            sketchState.rect = { active: false, startSnap: null };
            sketchState.phase = 'idle';
            return;
          }
          ptIds.push(id);
        }

        // Create 4 edges: 0→1 (top), 1→2 (right), 2→3 (bottom), 3→0 (left)
        const edgeIds = [];
        for (let i = 0; i < 4; i++) {
          const eid = await window.__createEdgeViaEngine(ptIds[i], ptIds[(i + 1) % 4], 'normal');
          if (eid) edgeIds.push(eid);
        }

        console.log('[Rect] created', { ptIds, edgeIds, corners, plane });

        // Apply H/V constraints via WASM solver if available
        if (window.__solveRectConstraints) {
          window.__solveRectConstraints(ptIds, plane);
        }

        // Reset rect state
        sketchState.rect = { active: false, startSnap: null };
        sketchState.phase = 'idle';

        if (window.__notifySketchChanged)   window.__notifySketchChanged();
        if (window.__updateSketchInspector) window.__updateSketchInspector();
        window.__setStatusMessage && window.__setStatusMessage(
          '✓ Rectangle created (' + ptIds.length + ' pts, ' + edgeIds.length + ' edges)'
        );
      };

      // ─────────────────────────────────────────────────────────
      // __solveRectConstraints(ptIds, plane) — add H/V constraints
      // Uses WASM solver first, bbox-snap fallback.
      // ─────────────────────────────────────────────────────────
      window.__solveRectConstraints = function(ptIds, plane) {
        if (ptIds.length !== 4) return;

        // ptIds[0]→[1] = top (horizontal), ptIds[1]→[2] = right (vertical),
        // ptIds[2]→[3] = bottom (horizontal), ptIds[3]→[0] = left (vertical)
        const horizontalPairs = [[ptIds[0], ptIds[1]], [ptIds[3], ptIds[2]]];
        const verticalPairs   = [[ptIds[1], ptIds[2]], [ptIds[0], ptIds[3]]];

        const getPoint = (id) => sketchState.points.find(p => p.id === id);

        if (window.__wasmSolveConstraints) {
          try {
            const pts = ptIds.map(id => {
              const p = getPoint(id);
              return p ? { id, gx: p.gx, gy: p.gy, gz: p.gz } : null;
            }).filter(Boolean);

            if (pts.length !== 4) throw new Error('missing points');

            const edges = [
              { id: 'eT', p1: ptIds[0], p2: ptIds[1] },
              { id: 'eR', p1: ptIds[1], p2: ptIds[2] },
              { id: 'eB', p1: ptIds[3], p2: ptIds[2] },
              { id: 'eL', p1: ptIds[0], p2: ptIds[3] },
            ];

            const constraints = [];
            let ci = 0;
            for (const [a, b] of horizontalPairs) {
              constraints.push({ id: 'cH' + ci++, type: 'HORIZONTAL', edgeId: edges.find(e =>
                (e.p1 === a && e.p2 === b) || (e.p1 === b && e.p2 === a))?.id || 'eT' });
            }
            for (const [a, b] of verticalPairs) {
              constraints.push({ id: 'cV' + ci++, type: 'VERTICAL', edgeId: edges.find(e =>
                (e.p1 === a && e.p2 === b) || (e.p1 === b && e.p2 === a))?.id || 'eL' });
            }

            const snap = { points: pts, edges, constraints };
            const result = window.__wasmSolveConstraints(snap);
            if (result && result.points) {
              for (const rp of result.points) {
                const sp = getPoint(rp.id);
                if (sp) {
                  sp.gx = rp.gx; sp.gy = rp.gy; sp.gz = rp.gz;
                  const gs = sketchState.gridSize || 1;
                  if (plane === 'XZ') { sp.x = rp.gx * gs; sp.y = 0; sp.z = rp.gz * gs; }
                  else if (plane === 'XY') { sp.x = rp.gx * gs; sp.y = rp.gy * gs; sp.z = 0; }
                  else { sp.x = 0; sp.y = rp.gy * gs; sp.z = rp.gz * gs; }
                }
              }
              console.log('[Rect] WASM constraints applied');
              return;
            }
          } catch(err) {
            console.warn('[Rect] WASM constraint solve failed, using bbox-snap', err);
          }
        }

        // Fallback: enforce H/V via direct coordinate alignment
        const [p0, p1, p2, p3] = ptIds.map(getPoint);
        if (!p0 || !p1 || !p2 || !p3) return;

        if (plane === 'XZ') {
          const gz_top = p0.gz; const gz_bot = p2.gz;
          const gx_L   = p0.gx; const gx_R   = p1.gx;
          const gs = sketchState.gridSize || 1;
          p0.gx = gx_L; p0.gz = gz_top; p0.x = gx_L * gs; p0.z = gz_top * gs; p0.y = 0;
          p1.gx = gx_R; p1.gz = gz_top; p1.x = gx_R * gs; p1.z = gz_top * gs; p1.y = 0;
          p2.gx = gx_R; p2.gz = gz_bot; p2.x = gx_R * gs; p2.z = gz_bot * gs; p2.y = 0;
          p3.gx = gx_L; p3.gz = gz_bot; p3.x = gx_L * gs; p3.z = gz_bot * gs; p3.y = 0;
        } else if (plane === 'XY') {
          const gy_top = p0.gy; const gy_bot = p2.gy;
          const gx_L   = p0.gx; const gx_R   = p1.gx;
          const gs = sketchState.gridSize || 1;
          p0.gx = gx_L; p0.gy = gy_top; p0.x = gx_L * gs; p0.y = gy_top * gs; p0.z = 0;
          p1.gx = gx_R; p1.gy = gy_top; p1.x = gx_R * gs; p1.y = gy_top * gs; p1.z = 0;
          p2.gx = gx_R; p2.gy = gy_bot; p2.x = gx_R * gs; p2.y = gy_bot * gs; p2.z = 0;
          p3.gx = gx_L; p3.gy = gy_bot; p3.x = gx_L * gs; p3.y = gy_bot * gs; p3.z = 0;
        } else { // YZ
          const gy_top = p0.gy; const gy_bot = p2.gy;
          const gz_L   = p0.gz; const gz_R   = p1.gz;
          const gs = sketchState.gridSize || 1;
          p0.gy = gy_top; p0.gz = gz_L; p0.y = gy_top * gs; p0.z = gz_L * gs; p0.x = 0;
          p1.gy = gy_top; p1.gz = gz_R; p1.y = gy_top * gs; p1.z = gz_R * gs; p1.x = 0;
          p2.gy = gy_bot; p2.gz = gz_R; p2.y = gy_bot * gs; p2.z = gz_R * gs; p2.x = 0;
          p3.gy = gy_bot; p3.gz = gz_L; p3.y = gy_bot * gs; p3.z = gz_L * gs; p3.x = 0;
        }
        console.log('[Rect] bbox-snap constraints applied');
      };

      // ─────────────────────────────────────────────────────────
      // __cancelRectTool() — cancel rect on Esc / tool switch
      // ─────────────────────────────────────────────────────────
      window.__cancelRectTool = function() {
        const wasActive = sketchState.rect && sketchState.rect.active;
        sketchState.rect  = { active: false, startSnap: null };
        sketchState.phase = 'idle';
        if (wasActive) {
          window.__setStatusMessage && window.__setStatusMessage('Rect cancelled');
          if (window.__notifySketchChanged) window.__notifySketchChanged();
        }
      };

"##;
