// ── Profile Constraints: Analyze + Make Rectangle / Square / Equalize ───────
//
// Phase 18. Professional CAD-style profile checker and auto-fixer.
//
// Exposes on window:
//   __analyzeProfile(profile)            -> report
//   __analyzeSelectedProfile()           -> report
//   __makeSelectedProfileRectangle(opts) -> { ok, error? }
//   __makeSelectedProfileSquare(opts)    -> { ok, error? }
//   __equalizeSelectedEdges(opts)        -> { ok, error? }
//   __profileErrorEdgeIds(report)        -> { errors:Set, warnings:Set }
//
// All point moves go through `window.__movePointViaEngine`, so they
// inherit WASM-first + backend-sync from the CAD engine.

pub const JS: &str = r##"
      (function registerProfileConstraints() {

        // Tolerances in mm (UI semantics).
        const TOL_GOOD_MM = 0.01;
        const TOL_WARN_MM = 0.10;

        function stepMm() {
          return (window.__cadInternalStepMm && window.__cadInternalStepMm()) || 0.01;
        }
        function gridToMm(g) { return g * stepMm(); }
        function mmToGrid(mm) { return Math.round((Number(mm) || 0) / stepMm()); }

        // Project a grid-space point onto a plane's 2D (u, v).
        //   XZ -> (gx, gz)
        //   XY -> (gx, gy)
        //   YZ -> (gz, gy)
        function projGrid(plane, p) {
          if (plane === 'XY') return { u: p.gx, v: p.gy };
          if (plane === 'YZ') return { u: p.gz, v: p.gy };
          return { u: p.gx, v: p.gz }; // XZ
        }

        // Unproject (u, v) back to grid (gx, gy, gz), preserving the
        // off-plane coordinate from `keep` (or 0 if missing).
        function unprojGrid(plane, u, v, keep) {
          if (plane === 'XY') return { gx: u, gy: v, gz: keep ? keep.gz : 0 };
          if (plane === 'YZ') return { gx: keep ? keep.gx : 0, gy: v, gz: u };
          return { gx: u, gy: keep ? keep.gy : 0, gz: v }; // XZ
        }

        function severityOf(driftMm) {
          if (driftMm <= TOL_GOOD_MM) return 'ok';
          if (driftMm <= TOL_WARN_MM) return 'warn';
          return 'error';
        }

        // ── Analyzer ─────────────────────────────────────────────────────
        // Reports drift in mm for each edge of a (preferably 4-side) profile.
        window.__analyzeProfile = function(prof) {
          if (!prof || !prof.pointIds || !prof.edgeIds) {
            return { ok:false, error:'no profile' };
          }
          const plane = prof.plane || sketchState.workingPlane || 'XZ';
          const pById = new Map(sketchState.points.map(p => [p.id, p]));
          const eById = new Map(sketchState.edges.map(e => [e.id, e]));

          const ring = prof.pointIds.map(id => pById.get(id)).filter(Boolean);
          if (ring.length !== prof.pointIds.length) {
            return { ok:false, error:'missing points' };
          }

          const uv = ring.map(p => ({ p, ...projGrid(plane, p) }));

          // Bounding box (grid units).
          let minU = Infinity, maxU = -Infinity, minV = Infinity, maxV = -Infinity;
          for (const q of uv) {
            if (q.u < minU) minU = q.u;
            if (q.u > maxU) maxU = q.u;
            if (q.v < minV) minV = q.v;
            if (q.v > maxV) maxV = q.v;
          }
          const widthMm  = gridToMm(maxU - minU);
          const heightMm = gridToMm(maxV - minV);

          // Edge analysis.
          const edges = [];
          const errors = [];
          for (const eid of prof.edgeIds) {
            const e = eById.get(eid);
            if (!e) continue;
            const a = pById.get(e.a), b = pById.get(e.b);
            if (!a || !b) continue;
            const ua = projGrid(plane, a), ub = projGrid(plane, b);
            const du = ub.u - ua.u, dv = ub.v - ua.v;
            const absDu = Math.abs(du), absDv = Math.abs(dv);
            const lenG  = Math.hypot(du, dv);
            const lenMm = gridToMm(lenG);

            // Classify orientation by dominant axis.
            let orient = 'diagonal';
            let driftMm = 0;
            if (absDu >= absDv) {
              orient  = 'horizontal';      // u-aligned
              driftMm = gridToMm(absDv);
            } else {
              orient  = 'vertical';        // v-aligned
              driftMm = gridToMm(absDu);
            }

            edges.push({ id: e.id, orient, lengthMm: lenMm, driftMm });

            const sev = severityOf(driftMm);
            if (sev !== 'ok') {
              errors.push({
                kind:     'not_axis_aligned',
                edgeId:   e.id,
                orient,
                driftMm:  driftMm,
                severity: sev,
              });
            }
          }

          // Quadrilateral-only checks: length match per opposite pair, 90°.
          let type = 'unknown';
          const sideLengthsMm = edges.map(e => e.lengthMm);
          if (ring.length === 4 && edges.length === 4) {
            type = 'quadrilateral';
            // Expected length = bbox side aligned with that edge orient.
            for (const e of edges) {
              const expectedMm = (e.orient === 'horizontal') ? widthMm : heightMm;
              const diffMm = Math.abs(e.lengthMm - expectedMm);
              const sev = severityOf(diffMm);
              if (sev !== 'ok') {
                errors.push({
                  kind:       'length_mismatch',
                  edgeId:     e.id,
                  expectedMm,
                  actualMm:   e.lengthMm,
                  diffMm,
                  severity:   sev,
                });
              }
            }
            // Vertex angles.
            for (let i = 0; i < ring.length; i++) {
              const prev = uv[(i + ring.length - 1) % ring.length];
              const cur  = uv[i];
              const next = uv[(i + 1) % ring.length];
              const ax = prev.u - cur.u, ay = prev.v - cur.v;
              const bx = next.u - cur.u, by = next.v - cur.v;
              const la = Math.hypot(ax, ay), lb = Math.hypot(bx, by);
              if (la === 0 || lb === 0) continue;
              const cos = (ax * bx + ay * by) / (la * lb);
              const ang = Math.acos(Math.max(-1, Math.min(1, cos))) * 180 / Math.PI;
              const dev = Math.abs(ang - 90);
              // Angle tolerance ≈ 0.1° good, 1° warn, else error.
              let sev = 'ok';
              if (dev > 1) sev = 'error';
              else if (dev > 0.1) sev = 'warn';
              if (sev !== 'ok') {
                errors.push({
                  kind:           'angle_not_90',
                  vertexPointId:  cur.p.id,
                  angleDeg:       ang,
                  severity:       sev,
                });
              }
            }
          }

          return {
            ok: errors.length === 0,
            type,
            plane,
            widthMm, heightMm,
            sideLengthsMm,
            edges,
            errors,
          };
        };

        window.__analyzeSelectedProfile = function() {
          const prof = window.__getSelectedProfile && window.__getSelectedProfile();
          if (!prof) return { ok:false, error:'no selected profile' };
          return window.__analyzeProfile(prof);
        };

        // Helper: split errors by severity into edge-id sets.
        window.__profileErrorEdgeIds = function(report) {
          const out = { errors: new Set(), warnings: new Set() };
          if (!report || !report.errors) return out;
          for (const e of report.errors) {
            if (!e.edgeId) continue;
            if (e.severity === 'error') out.errors.add(e.edgeId);
            else if (e.severity === 'warn') out.warnings.add(e.edgeId);
          }
          return out;
        };

        // ── Quad reorder: map 4 points to bbox corners (BL,BR,TR,TL) by
        //    nearest-corner assignment while keeping topology stable.
        // Returns array of { point, targetU, targetV } in original ring order.
        function assignToCorners(uv, minU, maxU, minV, maxV) {
          const corners = [
            { name:'BL', u:minU, v:minV },
            { name:'BR', u:maxU, v:minV },
            { name:'TR', u:maxU, v:maxV },
            { name:'TL', u:minU, v:maxV },
          ];
          // Greedy nearest-corner assignment, each corner used once.
          const used = new Set();
          const result = uv.map((q) => {
            let best = -1, bestD = Infinity;
            for (let i = 0; i < corners.length; i++) {
              if (used.has(i)) continue;
              const c = corners[i];
              const du = c.u - q.u, dv = c.v - q.v;
              const d  = du * du + dv * dv;
              if (d < bestD) { bestD = d; best = i; }
            }
            used.add(best);
            return { point: q.p, targetU: corners[best].u, targetV: corners[best].v };
          });
          return result;
        }

        async function applyTargets(plane, targets) {
          for (const t of targets) {
            const cur = t.point;
            const tg  = unprojGrid(plane, t.targetU, t.targetV, cur);
            if (tg.gx === cur.gx && tg.gy === cur.gy && tg.gz === cur.gz) continue;
            const r = await window.__movePointViaEngine(cur.id, tg.gx, tg.gy, tg.gz);
            if (!r || !r.ok) {
              return { ok:false, error:'move ' + cur.id + ' failed: '
                + ((r && r.error) || '?') };
            }
          }
          return { ok:true };
        }

        // ── Make Rectangle ───────────────────────────────────────────────
        // Strategy:
        //   1. If wasm_solve_constraints is available → add H/V constraints
        //      and solve in one shot via WASM (preferred, instant).
        //   2. Fallback: manually snap each point to bounding-box corner
        //      via __movePointViaEngine (one request per point).
        window.__makeSelectedProfileRectangle = async function(/*options*/) {
          const prof = window.__getSelectedProfile && window.__getSelectedProfile();
          if (!prof) return { ok:false, error:'no selected profile' };
          if (prof.pointIds.length !== 4) {
            return { ok:false, error:'rectangle requires 4 points (got '
              + prof.pointIds.length + ')' };
          }
          const plane = prof.plane || sketchState.workingPlane || 'XZ';
          const pById = new Map(sketchState.points.map(p => [p.id, p]));
          const eById = new Map(sketchState.edges.map(e => [e.id, e]));
          const ring  = prof.pointIds.map(id => pById.get(id));
          if (ring.some(p => !p)) return { ok:false, error:'missing point' };

          // ── WASM solver path ──────────────────────────────────────────
          const wasm = window.sketchWasm;
          if (wasm && typeof wasm.wasm_solve_constraints === 'function') {
            // Build sketch snapshot with H/V constraints for all profile edges.
            const sketchSnap = {
              schema: 'sketch_graph', version: 1,
              workingPlane: plane,
              gridSize: (sketchState.precision && sketchState.precision.displayGridStepM) || 0.01,
              points: JSON.parse(JSON.stringify(sketchState.points)),
              edges:  JSON.parse(JSON.stringify(sketchState.edges)),
              constraints: JSON.parse(JSON.stringify(sketchState.constraints || [])),
              profiles: []
            };
            // Classify each profile edge as H or V by dominant axis, add constraint.
            const ts = Date.now();
            for (const eid of prof.edgeIds) {
              const e = eById.get(eid);
              if (!e) continue;
              const a = pById.get(e.a), b = pById.get(e.b);
              if (!a || !b) continue;
              const uva = projGrid(plane, a), uvb = projGrid(plane, b);
              const du = Math.abs(uvb.u - uva.u), dv = Math.abs(uvb.v - uva.v);
              const ty = du >= dv ? 'HORIZONTAL' : 'VERTICAL';
              sketchSnap.constraints.push({
                id: '__rect_' + eid + '_' + ts,
                type: ty, targetType: 'edge', targetId: eid, value: null
              });
            }
            console.log('[Make Rectangle] WASM solve, constraints:', sketchSnap.constraints.length);
            let solved;
            try {
              solved = JSON.parse(wasm.wasm_solve_constraints(JSON.stringify({ sketch: sketchSnap })));
            } catch(e) {
              console.warn('[Make Rectangle] WASM error:', e);
              solved = null;
            }
            if (solved && solved.ok && solved.sketch) {
              console.log('[Make Rectangle] WASM OK, moved points:',
                solved.results.flatMap(r => r.moved_points || []));
              // Patch sketchState points from solver result.
              const byId = {};
              for (const p of solved.sketch.points) byId[p.id] = p;
              for (let i = 0; i < sketchState.points.length; i++) {
                const upd = byId[sketchState.points[i].id];
                if (upd) {
                  sketchState.points[i].gx = upd.gx;
                  sketchState.points[i].gy = upd.gy;
                  sketchState.points[i].gz = upd.gz;
                  sketchState.points[i].x  = upd.x;
                  sketchState.points[i].y  = upd.y;
                  sketchState.points[i].z  = upd.z;
                }
              }
              if (window.__recomputeProfiles)   window.__recomputeProfiles();
              if (window.__recomputeValidation) window.__recomputeValidation();
              if (window.__redrawSketch)        window.__redrawSketch();
              if (window.__updateSketchInspector) window.__updateSketchInspector();
              if (window.__setStatusMessage) {
                window.__setStatusMessage('✓ Прямоугольник (WASM solver)');
              }
              return { ok:true };
            }
            console.warn('[Make Rectangle] WASM solve not ok, falling back to bbox snap');
          }

          // ── Fallback: bbox snap ───────────────────────────────────────
          const uv = ring.map(p => ({ p, ...projGrid(plane, p) }));
          let minU = Infinity, maxU = -Infinity, minV = Infinity, maxV = -Infinity;
          for (const q of uv) {
            if (q.u < minU) minU = q.u;
            if (q.u > maxU) maxU = q.u;
            if (q.v < minV) minV = q.v;
            if (q.v > maxV) maxV = q.v;
          }
          if (maxU - minU <= 0 || maxV - minV <= 0) {
            return { ok:false, error:'degenerate bounding box' };
          }
          const targets = assignToCorners(uv, minU, maxU, minV, maxV);
          const res = await applyTargets(plane, targets);
          if (!res.ok) return res;
          if (window.__setStatusMessage) {
            window.__setStatusMessage('Профиль → прямоугольник ('
              + gridToMm(maxU - minU).toFixed(2) + ' × '
              + gridToMm(maxV - minV).toFixed(2) + ' мм)');
          }
          return { ok:true };
        };

        // ── Make Square ──────────────────────────────────────────────────
        window.__makeSelectedProfileSquare = async function(options) {
          const opts = options || {};
          const prof = window.__getSelectedProfile && window.__getSelectedProfile();
          if (!prof) return { ok:false, error:'no selected profile' };
          if (prof.pointIds.length !== 4) {
            return { ok:false, error:'square requires 4 points (got '
              + prof.pointIds.length + ')' };
          }
          const plane = prof.plane || sketchState.workingPlane || 'XZ';
          const pById = new Map(sketchState.points.map(p => [p.id, p]));
          const ring  = prof.pointIds.map(id => pById.get(id));
          if (ring.some(p => !p)) return { ok:false, error:'missing point' };

          const uv = ring.map(p => ({ p, ...projGrid(plane, p) }));
          let minU = Infinity, maxU = -Infinity, minV = Infinity, maxV = -Infinity;
          for (const q of uv) {
            if (q.u < minU) minU = q.u;
            if (q.u > maxU) maxU = q.u;
            if (q.v < minV) minV = q.v;
            if (q.v > maxV) maxV = q.v;
          }
          const widthG  = maxU - minU;
          const heightG = maxV - minV;
          if (widthG <= 0 && heightG <= 0) {
            return { ok:false, error:'degenerate profile' };
          }

          // Size: explicit sizeMm > 0, else average of width/height.
          let sizeG;
          if (isFinite(opts.sizeMm) && opts.sizeMm > 0) {
            sizeG = mmToGrid(opts.sizeMm);
          } else if (opts.use === 'width')  sizeG = widthG;
          else if (opts.use === 'height') sizeG = heightG;
          else sizeG = Math.round((widthG + heightG) / 2);

          if (sizeG <= 0) return { ok:false, error:'size too small' };

          const cu = Math.round((minU + maxU) / 2);
          const cv = Math.round((minV + maxV) / 2);
          const halfA = Math.floor(sizeG / 2);
          const halfB = sizeG - halfA;
          const sMinU = cu - halfA, sMaxU = cu + halfB;
          const sMinV = cv - halfA, sMaxV = cv + halfB;

          const targets = assignToCorners(uv, sMinU, sMaxU, sMinV, sMaxV);
          const res = await applyTargets(plane, targets);
          if (!res.ok) return res;

          if (window.__setStatusMessage) {
            window.__setStatusMessage('Профиль → квадрат ('
              + gridToMm(sizeG).toFixed(2) + ' мм)');
          }
          return { ok:true };
        };

        // ── Equalize selected edges ──────────────────────────────────────
        // First version: set all selected edges to the average length, each
        // resized in 'fixA_moveB' free direction.
        window.__equalizeSelectedEdges = async function(/*options*/) {
          const ids = [...(sketchState.selectedEdgeIds || [])];
          if (ids.length < 2) {
            return { ok:false, error:'select at least 2 edges' };
          }
          let sum = 0, n = 0;
          for (const id of ids) {
            const em = window.__edgeMmById && window.__edgeMmById(id);
            if (em && isFinite(em.lengthMm)) { sum += em.lengthMm; n++; }
          }
          if (n === 0) return { ok:false, error:'no measurable edges' };
          const avgMm = sum / n;
          for (const id of ids) {
            const r = await window.__setEdgeLengthMm(id, avgMm, { mode:'fixA_moveB' });
            if (!r || !r.ok) {
              return { ok:false, error:'edge ' + id + ' failed: '
                + ((r && r.error) || '?') };
            }
          }
          if (window.__setStatusMessage) {
            window.__setStatusMessage('Уравнено ' + ids.length
              + ' рёбер → ' + avgMm.toFixed(2) + ' мм');
          }
          return { ok:true, avgMm };
        };

        // Cached last report — read by inspector + overlay highlighter.
        window.__profileCheckState = { report: null, profileId: null };

        // Wrapper used by the UI button. Caches the report and refreshes UI.
        window.__runProfileCheck = function() {
          const prof = window.__getSelectedProfile && window.__getSelectedProfile();
          if (!prof) {
            window.__profileCheckState = { report:null, profileId:null };
            if (window.__updateSketchInspector) window.__updateSketchInspector();
            return null;
          }
          const rep = window.__analyzeProfile(prof);
          window.__profileCheckState = { report: rep, profileId: prof.id };
          if (window.__updateSketchInspector) window.__updateSketchInspector();
          return rep;
        };

      })();
"##;
