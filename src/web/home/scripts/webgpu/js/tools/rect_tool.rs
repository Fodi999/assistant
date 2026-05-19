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

        // Create 4 edges: 0→1 (top H), 1→2 (right V), 2→3 (bottom H), 3→0 (left V)
        // edgeIds[0]=top, [1]=right, [2]=bottom, [3]=left
        const edgeIds = [];
        for (let i = 0; i < 4; i++) {
          const eid = await window.__createEdgeViaEngine(ptIds[i], ptIds[(i + 1) % 4], 'normal');
          if (eid) edgeIds.push(eid);
        }

        console.log('[Rect] created', { ptIds, edgeIds, corners, plane });

        // ── Add H/V constraints to sketchState.constraints ──────────────────
        // edgeIds[0] = top    → HORIZONTAL
        // edgeIds[1] = right  → VERTICAL
        // edgeIds[2] = bottom → HORIZONTAL
        // edgeIds[3] = left   → VERTICAL
        if (edgeIds.length === 4 && window.__addConstraint) {
          await window.__addConstraint('HORIZONTAL', 'edge', edgeIds[0], null);
          await window.__addConstraint('VERTICAL',   'edge', edgeIds[1], null);
          await window.__addConstraint('HORIZONTAL', 'edge', edgeIds[2], null);
          await window.__addConstraint('VERTICAL',   'edge', edgeIds[3], null);
          console.log('[Rect] H/V constraints added to sketchState');
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

        if (window.__wasmModule && typeof window.__wasmModule.wasm_solve_constraints === 'function') {
          try {
            const pts = ptIds.map(id => {
              const p = getPoint(id);
              return p ? { id, gx: p.gx, gy: p.gy, gz: p.gz } : null;
            }).filter(Boolean);

            if (pts.length !== 4) throw new Error('missing points');

            // Edge schema: { id, a, b } — matches WASM sketch_graph format
            const edges = [
              { id: 'eT', a: ptIds[0], b: ptIds[1] },
              { id: 'eR', a: ptIds[1], b: ptIds[2] },
              { id: 'eB', a: ptIds[3], b: ptIds[2] },
              { id: 'eL', a: ptIds[0], b: ptIds[3] },
            ];

            // Constraint schema: { id, type, targetType, targetId, value }
            const constraints = [
              { id: 'cH0', type: 'HORIZONTAL', targetType: 'edge', targetId: 'eT', value: null },
              { id: 'cH1', type: 'HORIZONTAL', targetType: 'edge', targetId: 'eB', value: null },
              { id: 'cV0', type: 'VERTICAL',   targetType: 'edge', targetId: 'eR', value: null },
              { id: 'cV1', type: 'VERTICAL',   targetType: 'edge', targetId: 'eL', value: null },
            ];

            const sketch = {
              schema: 'sketch_graph',
              version: 1,
              workingPlane: plane,
              gridSize: sketchState.gridSize || 0.01,
              points: pts,
              edges,
              constraints,
              profiles: [],
            };
            const raw    = window.__wasmModule.wasm_solve_constraints(JSON.stringify({ sketch }));
            const result = JSON.parse(raw);
            if (result && result.ok && result.sketch && result.sketch.points) {
              for (const rp of result.sketch.points) {
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

      // ─────────────────────────────────────────────────────────────────────
      // __makeSquare(profileId?) — добавить EQUAL_LENGTH ко всем 4 сторонам
      // профиля-прямоугольника, превращая его в квадрат.
      //
      // Алгоритм:
      //   1. Берём профиль (выбранный или первый)
      //   2. Находим все 4 ребра
      //   3. Находим смежные пары (top+right, right+bottom, bottom+left)
      //      и добавляем EQUAL_LENGTH
      //   4. Принудительно устанавливаем длину самой короткой стороны для
      //      всех рёбер через FIXED_LENGTH чтобы солвер «знал» целевой размер
      //   5. Запускаем wasm_solve_constraints
      // ─────────────────────────────────────────────────────────────────────
      window.__makeSquare = async function(profileId) {
        const ss = window.sketchState;
        if (!ss) return;

        // Пересчитываем профили
        if (window.__recomputeProfiles) window.__recomputeProfiles();

        // Находим профиль
        let prof = null;
        if (profileId) prof = (ss.profiles || []).find(p => p.id === profileId);
        if (!prof && ss.selectedProfileId)
          prof = (ss.profiles || []).find(p => p.id === ss.selectedProfileId);
        if (!prof && ss.profiles && ss.profiles.length) prof = ss.profiles[0];
        if (!prof) {
          window.__setStatusMessage && window.__setStatusMessage('⚠ Квадрат: нет профиля');
          return;
        }
        if (prof.edgeIds.length !== 4) {
          window.__setStatusMessage && window.__setStatusMessage(
            '⚠ Квадрат: нужен прямоугольник (4 ребра), сейчас: ' + prof.edgeIds.length
          );
          return;
        }

        window.__pushHistory && window.__pushHistory();

        const byEdge = new Map((ss.edges || []).map(e => [e.id, e]));
        const byPt   = new Map((ss.points || []).map(p => [p.id, p]));
        const step   = (ss.precision && ss.precision.internalStepM) || 0.00001;
        const stepMm = step * 1000;

        // Вычисляем длины всех 4 рёбер
        const edgeLens = prof.edgeIds.map(eid => {
          const e = byEdge.get(eid);
          if (!e) return { id: eid, len: 0 };
          const a = byPt.get(e.a), b = byPt.get(e.b);
          if (!a || !b) return { id: eid, len: 0 };
          const dx = b.gx - a.gx, dy = b.gy - a.gy, dz = b.gz - a.gz;
          return { id: eid, len: Math.hypot(dx, dy, dz) * stepMm };
        });

        // Целевой размер — минимальная ненулевая сторона (меньшая сторона)
        const nonZero = edgeLens.filter(e => e.len > 0);
        if (!nonZero.length) {
          window.__setStatusMessage && window.__setStatusMessage('⚠ Квадрат: вырожденные рёбра');
          return;
        }
        const targetMm = Math.min(...nonZero.map(e => e.len));
        console.log('[makeSquare] target side =', targetMm.toFixed(2), 'мм, edges:', edgeLens);

        // Удаляем старые EQUAL_LENGTH и FIXED_LENGTH для этих рёбер
        ss.constraints = (ss.constraints || []).filter(c => {
          const t = (c.type || '').toUpperCase();
          if ((t === 'EQUAL_LENGTH' || t === 'FIXED_LENGTH') &&
              prof.edgeIds.includes(c.targetId)) return false;
          return true;
        });

        // Добавляем EQUAL_LENGTH между соседними парами рёбер (3 пары = достаточно)
        const [e0, e1, e2, e3] = prof.edgeIds;
        if (window.__addConstraint) {
          window.__addConstraint('EQUAL_LENGTH', 'edge', e0 + ',' + e1, null);
          window.__addConstraint('EQUAL_LENGTH', 'edge', e1 + ',' + e2, null);
          window.__addConstraint('EQUAL_LENGTH', 'edge', e2 + ',' + e3, null);
        }

        // Фиксируем первое ребро в targetMm — солвер подтянет остальные
        if (window.__addConstraint) {
          window.__addConstraint('FIXED_LENGTH', 'edge', e0, targetMm);
        }

        // Немедленно принудительно выравниваем геометрию (без солвера как fallback)
        _forceSquareGeometry(prof, targetMm, ss, byEdge, byPt, step);

        if (window.__notifySketchChanged) window.__notifySketchChanged();

        // Запускаем солвер чтобы применить ограничения
        if (window.__solveSketchWasm) {
          await window.__solveSketchWasm();
        } else if (window.__solveConstraints) {
          await window.__solveConstraints();
        }

        window.__setStatusMessage && window.__setStatusMessage(
          '⬛ Квадрат создан: сторона ' + targetMm.toFixed(1) + ' мм'
        );

        if (window.__recomputeProfiles) window.__recomputeProfiles();
        if (window.__redrawSketch)      window.__redrawSketch();
        if (window.__updateDofBadge)    window.__updateDofBadge();
      };

      // Принудительное выравнивание сторон (геометрический fallback без солвера)
      function _forceSquareGeometry(prof, targetMm, ss, byEdge, byPt, step) {
        // Находим фиксированную точку профиля (якорь)
        const fixed = prof.pointIds.find(id => window.__isPointFixed && window.__isPointFixed(id));
        const anchorId = fixed || prof.pointIds[0];
        const anchor   = byPt.get(anchorId);
        if (!anchor) return;

        const plane   = ss.workingPlane || 'XZ';
        const target  = Math.round(targetMm / (step * 1000)); // в grid units

        // Определяем два свободных направления плоскости
        let uAxis, vAxis; // 'gx'/'gy'/'gz'
        if (plane === 'XZ') { uAxis = 'gx'; vAxis = 'gz'; }
        else if (plane === 'XY') { uAxis = 'gx'; vAxis = 'gy'; }
        else { uAxis = 'gy'; vAxis = 'gz'; }

        // Строим карту: pointId → позиция в порядке обхода профиля
        // p0 = anchor, p1, p2, p3 по порядку обхода в profile.pointIds
        const ordered = [...prof.pointIds];
        const ai = ordered.indexOf(anchorId);
        const sorted = [...ordered.slice(ai), ...ordered.slice(0, ai)];
        const [id0, id1, id2, id3] = sorted;

        const p0 = byPt.get(id0);
        const p1 = byPt.get(id1);
        const p2 = byPt.get(id2);
        const p3 = byPt.get(id3);
        if (!p0 || !p1 || !p2 || !p3) return;

        // Определяем знаки направлений от p0→p1 и p0→p3
        const du01 = Math.sign(p1[uAxis] - p0[uAxis]) || 1;
        const dv01 = Math.sign(p1[vAxis] - p0[vAxis]) || 0;
        const du03 = Math.sign(p3[uAxis] - p0[uAxis]) || 0;
        const dv03 = Math.sign(p3[vAxis] - p0[vAxis]) || 1;

        const setGPt = (p, u, v) => {
          p[uAxis] = u; p[vAxis] = v;
          p.x = p.gx * step; p.y = p.gy * step; p.z = p.gz * step;
          // Зануляем ось нормали
          if (plane === 'XZ') { p.gy = 0; p.y = 0; }
          else if (plane === 'XY') { p.gz = 0; p.z = 0; }
          else { p.gx = 0; p.x = 0; }
        };

        const u0 = p0[uAxis], v0 = p0[vAxis];
        // p1 = p0 + target × (du01, dv01)
        setGPt(p1, u0 + du01 * target, v0 + dv01 * target);
        // p3 = p0 + target × (du03, dv03)
        setGPt(p3, u0 + du03 * target, v0 + dv03 * target);
        // p2 = p0 + target × (du01+du03, dv01+dv03)
        setGPt(p2, u0 + (du01 + du03) * target, v0 + (dv01 + dv03) * target);

        console.log('[makeSquare] geometry forced', { id0, id1, id2, id3, target, du01, dv01, du03, dv03 });
      }

"##;
