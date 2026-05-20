// ── JS: WASM bridge (Phase 10 + 11) ───────────────────────────────────────
// Domain: Sketch — shared sketch_engine in the browser.
//
// Phase 10 shipped: lazy ES-module import + validate.
// Phase 11 adds:   __wasmAddPoint / __wasmAddEdge + Hybrid cross-check.
//
// Three engine modes (sketchState.engineMode):
//   * 'backend' — HTTP-only (Phase 7 path).
//   * 'wasm'    — pure browser (no backend).
//   * 'hybrid'  — WASM applies instantly, backend cross-checks asynchronously
//                 and a signature diff is reported via lastSyncStatus.

pub const JS: &str = r##"
      // ── WASM runtime state ─────────────────────────────────────
      window.wasmState = {
        status: 'not_loaded',     // 'not_loaded' | 'loading' | 'ready' | 'error'
        info: null,               // wasm_engine_info() parsed JSON
        error: null,              // last init error message
        lastValidateMs: 0,        // last __wasmValidateSketch duration
        lastAddMs: 0,             // last __wasmAddPoint / __wasmAddEdge duration
      };

      // ── Lazy ES-module import ─────────────────────────────────
      // Object-style init signature is required by wasm-bindgen ≥0.2.93;
      // the positional `mod.default('/path.wasm')` form is deprecated.
      let __wasmModulePromise = null;
      window.__loadSketchWasm = async function() {
        if (wasmState.status === 'ready')   return true;
        if (wasmState.status === 'loading') {
          // Race: another caller is mid-init. Spin until ready/error.
          while (wasmState.status === 'loading') {
            await new Promise(r => setTimeout(r, 16));
          }
          return wasmState.status === 'ready';
        }
        wasmState.status = 'loading';
        wasmState.error  = null;
        if (window.__updateSketchInspector) window.__updateSketchInspector();
        try {
          if (!__wasmModulePromise) {
            // Cache-buster: a per-page-load nonce so the browser never serves
            // a stale sketch_engine.js / _bg.wasm after `make wasm`.
            const v = '?v=' + Date.now();
            __wasmModulePromise = import('/wasm/sketch_engine/sketch_engine.js' + v);
          }
          const mod = await __wasmModulePromise;
          await mod.default({ module_or_path: '/wasm/sketch_engine/sketch_engine_bg.wasm?v=' + Date.now() });
          window.__wasmModule = mod;
          // Phase 17 self-check — surface available exports in the console
          // so we can verify wasm_move_point is actually shipped.
          try {
            const exported = Object.keys(mod).filter(k => k.startsWith('wasm_'));
            console.log('[sketch_wasm] loaded; exports:', exported);
            if (!mod.wasm_move_point) {
              console.warn('[sketch_wasm] wasm_move_point MISSING from this bundle — server is serving a stale build.');
            }
          } catch (_) {}
          // Handshake check.
          try {
            const info = JSON.parse(mod.wasm_engine_info());
            wasmState.info = info;
          } catch (e) {
            wasmState.info = null;
          }
          // ── Solver smoke-test 1: HORIZONTAL ─────────────────────────────
          try {
            const testSketch = {
              schema: 'sketch_graph', version: 1,
              workingPlane: 'XZ', gridSize: 0.01,
              points: [
                { id: 'p1', gx: 0, gy: 0, gz: 0,  x: 0.0,  y: 0.0, z: 0.0  },
                { id: 'p2', gx: 10, gy: 0, gz: 2, x: 0.1, y: 0.0, z: 0.02 }
              ],
              edges: [{ id: 'e1', a: 'p1', b: 'p2' }],
              constraints: [{ id: 'c1', type: 'HORIZONTAL', targetType: 'edge', targetId: 'e1', value: null }],
              profiles: []
            };
            console.log('[WASM TEST] engine info:', JSON.parse(mod.wasm_engine_info()));
            console.log('[WASM TEST] solve INPUT:', testSketch);
            const raw = mod.wasm_solve_constraints(JSON.stringify({ sketch: testSketch }));
            console.log('[WASM TEST] solve RAW:', raw);
            const solved = JSON.parse(raw);
            console.log('[WASM TEST] solve OUTPUT:', solved);
            if (solved.ok && solved.sketch) {
              const pts = solved.sketch.points;
              const p1gz = pts.find(p => p.id === 'p1')?.gz;
              const p2gz = pts.find(p => p.id === 'p2')?.gz;
              if (p1gz === p2gz) {
                console.log('%c[WASM TEST] ✅ HORIZONTAL solver OK — p1.gz === p2.gz ===', 'color:green;font-weight:bold', p1gz);
              } else {
                console.warn('[WASM TEST] ⚠️ p1.gz:', p1gz, '!== p2.gz:', p2gz, '— solver may not be running');
              }
            } else {
              console.warn('[WASM TEST] solve returned ok=false or no sketch:', solved);
            }
          } catch (e) {
            console.error('[WASM TEST] solver smoke-test FAILED:', e);
          }
          // ── Solver smoke-test 2: RECTANGLE (4 constraints + FIXED_LENGTH) ─
          try {
            // Slightly imperfect rectangle on XZ plane:
            //   p1(0,0) p2(10,0) p3(10,5) p4(0,4)  ← p4.gz off by 1
            // After solve: top/bottom HORIZONTAL, left/right VERTICAL,
            //   width FIXED_LENGTH=100 grid → gx span = 100
            //   height FIXED_LENGTH=50 grid → gz span = 50
            const rectSketch = {
              schema: 'sketch_graph', version: 1,
              workingPlane: 'XZ', gridSize: 0.01,
              points: [
                { id: 'p1', gx:  0, gy: 0, gz:  0, x: 0.00, y: 0.0, z: 0.00 },
                { id: 'p2', gx: 11, gy: 0, gz:  1, x: 0.11, y: 0.0, z: 0.01 },
                { id: 'p3', gx: 11, gy: 0, gz:  6, x: 0.11, y: 0.0, z: 0.06 },
                { id: 'p4', gx:  1, gy: 0, gz:  5, x: 0.01, y: 0.0, z: 0.05 }
              ],
              edges: [
                { id: 'eTop',    a: 'p4', b: 'p3' },
                { id: 'eBottom', a: 'p1', b: 'p2' },
                { id: 'eLeft',   a: 'p1', b: 'p4' },
                { id: 'eRight',  a: 'p2', b: 'p3' }
              ],
              constraints: [
                { id: 'cT', type: 'HORIZONTAL',   targetType: 'edge', targetId: 'eTop',    value: null },
                { id: 'cB', type: 'HORIZONTAL',   targetType: 'edge', targetId: 'eBottom', value: null },
                { id: 'cL', type: 'VERTICAL',     targetType: 'edge', targetId: 'eLeft',   value: null },
                { id: 'cR', type: 'VERTICAL',     targetType: 'edge', targetId: 'eRight',  value: null },
                { id: 'cW', type: 'FIXED_LENGTH', targetType: 'edge', targetId: 'eBottom', value: 100  },
                { id: 'cH', type: 'FIXED_LENGTH', targetType: 'edge', targetId: 'eLeft',   value: 50   }
              ],
              profiles: []
            };
            console.groupCollapsed('[WASM TEST 2] Rectangle + FIXED_LENGTH');
            console.log('INPUT:', rectSketch);
            const raw2 = mod.wasm_solve_constraints(JSON.stringify({ sketch: rectSketch }));
            const r2   = JSON.parse(raw2);
            console.log('OUTPUT:', r2);
            if (r2.ok && r2.sketch) {
              const p = id => r2.sketch.points.find(pt => pt.id === id);
              const allOk = r2.results.every(res => res.ok);
              console.log('All constraints ok:', allOk);
              console.log('Results:', r2.results.map(res => res.constraint_id + ': ' + (res.ok ? '✅' : '❌ ' + res.message)).join(' | '));
              console.log('Points after solve:',
                'p1(' + p('p1').gx + ',' + p('p1').gz + ')',
                'p2(' + p('p2').gx + ',' + p('p2').gz + ')',
                'p3(' + p('p3').gx + ',' + p('p3').gz + ')',
                'p4(' + p('p4').gx + ',' + p('p4').gz + ')'
              );
              if (allOk) {
                console.log('%c[WASM TEST 2] ✅ Rectangle solved!', 'color:green;font-weight:bold');
              } else {
                console.warn('[WASM TEST 2] ⚠️ Some constraints failed');
              }
            } else {
              console.warn('[WASM TEST 2] ❌ solve failed:', r2);
            }
            console.groupEnd();
          } catch (e) {
            console.error('[WASM TEST 2] rectangle test FAILED:', e);
          }
          // ────────────────────────────────────────────────────────────────
          wasmState.status = 'ready';
          window.__setStatusMessage('WASM движок готов'
            + (wasmState.info ? (' v' + wasmState.info.version) : ''));
        } catch (e) {
          wasmState.status = 'error';
          wasmState.error  = String(e && e.message || e);
          window.__setStatusMessage('WASM: ошибка загрузки — ' + wasmState.error);
          __wasmModulePromise = null;
        }
        if (window.__updateSketchInspector) window.__updateSketchInspector();
        return wasmState.status === 'ready';
      };

      // Lazy auto-loader — used by command helpers in 'wasm' / 'hybrid' mode.
      window.__ensureSketchWasm = async function() {
        if (wasmState.status === 'ready') return true;
        return await window.__loadSketchWasm();
      };

      // ── Validation (Phase 10) ─────────────────────────────────
      window.__wasmValidateSketch = async function() {
        if (!(await window.__ensureSketchWasm())) return null;
        const payload = JSON.stringify({ sketch: window.__sketchToJSON() });
        const t0  = performance.now();
        const out = window.__wasmModule.wasm_validate_sketch(payload);
        const dt  = performance.now() - t0;
        wasmState.lastValidateMs = dt;
        let parsed = null;
        try { parsed = JSON.parse(out); } catch (e) { parsed = null; }
        if (parsed) {
          window.__setStatusMessage('WASM проверка ОК ' + dt.toFixed(2) + ' мс');
        } else {
          window.__setStatusMessage('WASM: ошибка разбора результата проверки');
        }
        return parsed;
      };

      // ── Low-level callers — accept an AddPoint/AddEdge request,
      //    return parsed SketchCommandResult, set lastAddMs.
      // ──────────────────────────────────────────────────────────
      window.__wasmAddPoint = function(request) {
        if (wasmState.status !== 'ready' || !window.__wasmModule) return null;
        const payload = JSON.stringify(request);
        const t0  = performance.now();
        const out = window.__wasmModule.wasm_add_point(payload);
        const dt  = performance.now() - t0;
        wasmState.lastAddMs = dt;
        try { return JSON.parse(out); } catch (e) { return null; }
      };

      window.__wasmAddEdge = function(request) {
        if (wasmState.status !== 'ready' || !window.__wasmModule) return null;
        const payload = JSON.stringify(request);
        const t0  = performance.now();
        const out = window.__wasmModule.wasm_add_edge(payload);
        const dt  = performance.now() - t0;
        wasmState.lastAddMs = dt;
        try { return JSON.parse(out); } catch (e) { return null; }
      };

      window.__wasmMovePoint = function(request) {
        if (wasmState.status !== 'ready' || !window.__wasmModule) {
          console.warn('[wasmMovePoint] not ready', { status: wasmState.status, hasModule: !!window.__wasmModule });
          return null;
        }
        if (!window.__wasmModule.wasm_move_point) {
          console.warn('[wasmMovePoint] wasm_move_point not exported from module. Exported keys:', Object.keys(window.__wasmModule).join(','));
          return null;
        }
        const payload = JSON.stringify(request);
        console.log('[wasmMovePoint] calling wasm_move_point with payload len', payload.length, 'gridSize=', request.gridSize);
        let out;
        try {
          const t0 = performance.now();
          out = window.__wasmModule.wasm_move_point(payload);
          const dt = performance.now() - t0;
          wasmState.lastAddMs = dt;
        } catch (e) {
          console.error('[wasmMovePoint] wasm threw:', e);
          return null;
        }
        try { return JSON.parse(out); } catch (e) {
          console.error('[wasmMovePoint] JSON.parse failed, raw output:', out);
          return null;
        }
      };

      // ── Sketch signature for hybrid cross-check ───────────────
      // Stable canonical fingerprint: ignores ordering, normalises edge
      // endpoints (lo-hi) and includes profile shape.
      window.__sketchSignature = function(g) {
        if (!g) return '';
        const pts = (g.points || [])
          .map(p => p.id + '@' + p.gx + ',' + p.gy + ',' + p.gz)
          .sort().join('|');
        const eds = (g.edges || []).map(e => {
          const lo = e.a <= e.b ? e.a : e.b;
          const hi = e.a <= e.b ? e.b : e.a;
          return e.id + ':' + lo + '-' + hi;
        }).sort().join('|');
        const prs = (g.profiles || [])
          .map(p => p.id + '#' + ((p.edgeIds || []).length))
          .sort().join('|');
        return 'P[' + pts + '] E[' + eds + '] R[' + prs + ']';
      };

      // ── Hybrid cross-checkers — fire-and-forget backend POSTs.
      //    `preSketch` is the SketchGraph captured BEFORE the WASM op,
      //    so the backend receives the same input and we can compare
      //    against the WASM result for divergence detection.
      // ──────────────────────────────────────────────────────────
      async function __hybridCrossCheckPoint(preSketch, gx, gy, gz, wasmResult) {
        sketchState.lastSyncStatus = 'pending';
        try {
          const t0 = performance.now();
          const res = await fetch('/api/matter/sketch/add-point', {
            method:  'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
              sketch:       preSketch,
              workingPlane: sketchState.workingPlane,
              gridSize:     sketchState.gridSize,
              gx: gx | 0, gy: gy | 0, gz: gz | 0,
            }),
          });
          const dt = performance.now() - t0;
          sketchState.lastBackendMs = dt;
          if (window.__perfSample) window.__perfSample('backend', dt);
          if (!res.ok) {
            sketchState.lastSyncStatus = 'err';
            if (window.__perfMarkBackendError) window.__perfMarkBackendError();
            return;
          }
          const json = await res.json();
          __hybridCompare(json, wasmResult);
        } catch (e) {
          sketchState.lastSyncStatus = 'err';
          if (window.__perfMarkBackendError) window.__perfMarkBackendError();
        } finally {
          if (window.__updateSketchInspector) window.__updateSketchInspector();
        }
      }

      async function __hybridCrossCheckEdge(preSketch, startRef, endRef, wasmResult) {
        sketchState.lastSyncStatus = 'pending';
        try {
          const t0 = performance.now();
          const res = await fetch('/api/matter/sketch/add-edge', {
            method:  'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
              sketch:       preSketch,
              workingPlane: sketchState.workingPlane,
              gridSize:     sketchState.gridSize,
              start: startRef,
              end:   endRef,
            }),
          });
          const dt = performance.now() - t0;
          sketchState.lastBackendMs = dt;
          if (window.__perfSample) window.__perfSample('backend', dt);
          if (!res.ok) {
            sketchState.lastSyncStatus = 'err';
            if (window.__perfMarkBackendError) window.__perfMarkBackendError();
            return;
          }
          const json = await res.json();
          __hybridCompare(json, wasmResult);
        } catch (e) {
          sketchState.lastSyncStatus = 'err';
          if (window.__perfMarkBackendError) window.__perfMarkBackendError();
        } finally {
          if (window.__updateSketchInspector) window.__updateSketchInspector();
        }
      }

      function __hybridCompare(backendResult, wasmResult) {
        const sigW = window.__sketchSignature(wasmResult   && wasmResult.sketch);
        const sigB = window.__sketchSignature(backendResult && backendResult.sketch);
        if (sigW && sigB && sigW === sigB) {
          sketchState.lastSyncStatus = 'ok';
        } else {
          sketchState.lastSyncStatus = 'diff';
          console.warn('[hybrid] sketch divergence', { wasm: sigW, backend: sigB });
        }
      }

      // ── High-level apply helpers — public API consumed by sketch_tools.
      //    Shape mirrors __backendAddPoint / __backendAddEdge so call-sites
      //    can switch on engineMode with minimal branching.
      // ──────────────────────────────────────────────────────────
      window.__wasmAddPointAndApply = async function(gx, gy, gz) {
        if (!(await window.__ensureSketchWasm())) {
          window.__setStatusMessage('WASM не готов — точка не добавлена');
          return { ok: false, error: wasmState.error || 'wasm not ready' };
        }
        const preSketch = window.__sketchToJSON();
        const request = {
          sketch:       preSketch,
          workingPlane: sketchState.workingPlane,
          gridSize:     sketchState.gridSize,
          gx: gx | 0, gy: gy | 0, gz: gz | 0,
        };
        const result = window.__wasmAddPoint(request);
        sketchState.lastWasmMs = wasmState.lastAddMs;
        if (!result) {
          window.__setStatusMessage('WASM: ошибка разбора точки');
          return { ok: false, error: 'parse' };
        }
        if (result.ok) {
          window.__applyBackendSketchResult(result);
          const pid = result.createdPointId || result.reusedPointId;
          const tag = (sketchState.engineMode === 'hybrid') ? 'Hybrid' : 'WASM';
          const created = !!result.createdPointId;
          const msg = tag + (created ? ' created point ' : ' reused point ') + pid
            + ' (' + wasmState.lastAddMs.toFixed(2) + ' ms)';
          sketchState.lastCommandMsg = msg;
          window.__setStatusMessage(msg);
          if (sketchState.engineMode === 'hybrid') {
            __hybridCrossCheckPoint(preSketch, gx, gy, gz, result);
          }
          return {
            ok: true,
            pointId: pid,
            created,
            message: result.message,
          };
        }
        const errMsg = result.message || 'WASM rejected point';
        sketchState.lastCommandMsg = '✕ ' + errMsg;
        window.__setStatusMessage(errMsg);
        return { ok: false, error: result.message };
      };

      window.__wasmAddEdgeAndApply = async function(startRef, endRef) {
        if (!(await window.__ensureSketchWasm())) {
          window.__setStatusMessage('WASM не готов — ребро не добавлено');
          return { ok: false, error: wasmState.error || 'wasm not ready' };
        }
        const preSketch = window.__sketchToJSON();
        const request = {
          sketch:       preSketch,
          workingPlane: sketchState.workingPlane,
          gridSize:     sketchState.gridSize,
          start: startRef,
          end:   endRef,
        };
        const result = window.__wasmAddEdge(request);
        sketchState.lastWasmMs = wasmState.lastAddMs;
        if (!result) {
          window.__setStatusMessage('WASM: ошибка разбора ребра');
          return { ok: false, error: 'parse' };
        }
        // Even on ok=false (duplicate / self-loop) the engine may have
        // already inserted endpoints — apply the returned sketch.
        if (result.sketch) window.__applyBackendSketchResult(result);
        if (result.ok) {
          const tag = (sketchState.engineMode === 'hybrid') ? 'Hybrid' : 'WASM';
          const eid = result.createdEdgeId || '?';
          const msg = tag + ' created edge ' + eid
            + ' (' + wasmState.lastAddMs.toFixed(2) + ' ms)';
          sketchState.lastCommandMsg = msg;
          window.__setStatusMessage(msg);
          if (sketchState.engineMode === 'hybrid') {
            __hybridCrossCheckEdge(preSketch, startRef, endRef, result);
          }
          return {
            ok: true,
            edgeId: result.createdEdgeId,
            createdPointId: result.createdPointId,
            message: result.message,
          };
        }
        const errMsg = result.message || 'WASM rejected edge';
        sketchState.lastCommandMsg = '✕ ' + errMsg;
        window.__setStatusMessage(errMsg);
        return { ok: false, error: result.message };
      };

      // ── Engine mode switch ────────────────────────────────────
      window.__setEngineMode = async function(mode) {
        if (mode !== 'backend' && mode !== 'wasm' && mode !== 'hybrid') {
          mode = 'backend';
        }
        sketchState.engineMode = mode;
        // Clear stale result from previous mode so inspector shows '—' not old data.
        sketchState.lastCommandMsg = '—';
        sketchState.lastSyncStatus = '—';
        // Mirror to legacy flag so existing reads still resolve sensibly.
        sketchState.useBackendCommands = (mode !== 'wasm');
        if (mode === 'wasm' || mode === 'hybrid') {
          // Fire-and-forget pre-load; failure surfaces via wasmState.status.
          window.__ensureSketchWasm().then(() => {
            if (window.__updateSketchInspector) window.__updateSketchInspector();
          });
        }
        window.__setStatusMessage('Режим движка: ' + mode.toUpperCase());
        if (window.__updateSketchInspector) window.__updateSketchInspector();
      };

      // Back-compat shim — Phase 10 inspector used a checkbox + this fn.
      window.__setUseWasmEngine = function(v) {
        window.__setEngineMode(v ? 'wasm' : 'backend');
      };

      // ── Eager preload — start fetching WASM immediately on page load ──────
      // The module is already hinted via <link rel="modulepreload"> in <head>,
      // so the browser has it in the preload cache. This call instantiates it
      // in the background so the first user action is instant.
      if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', () => {
          window.__loadSketchWasm().catch(e => console.warn('[wasm preload]', e));
        });
      } else {
        // DOMContentLoaded already fired (inline scripts run after it).
        setTimeout(() => {
          window.__loadSketchWasm().catch(e => console.warn('[wasm preload]', e));
        }, 0);
      }
"##;
