// ── JS: WASM bridge — geometry_engine ────────────────────────────────────────
// Single unified WASM module: 2D sketch + 3D extrude in one binary.
// Path: /wasm/geometry_engine/geometry_engine.js

pub const JS: &str = r##"
      // ── WASM runtime state ─────────────────────────────────────
      window.wasmState = {
        status: 'not_loaded',     // 'not_loaded' | 'loading' | 'ready' | 'error'
        info: null,               // geometry_engine_info() parsed JSON
        error: null,
        lastValidateMs: 0,
        lastAddMs: 0,
      };

      let __wasmModulePromise = null;
      window.__loadSketchWasm = async function() {
        if (wasmState.status === 'ready')   return true;
        if (wasmState.status === 'loading') {
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
            const v = '?v=' + Date.now();
            __wasmModulePromise = import('/wasm/geometry_engine/geometry_engine.js' + v);
          }
          const mod = await __wasmModulePromise;
          await mod.default({ module_or_path: '/wasm/geometry_engine/geometry_engine_bg.wasm?v=' + Date.now() });
          window.__wasmModule = mod;

          try {
            const exported = Object.keys(mod).filter(k => k.startsWith('wasm_') || k.startsWith('sketch_') || k === 'extrude_json');
            console.log('[geometry_engine] loaded; exports:', exported);
          } catch (_) {}

          try {
            const info = JSON.parse(mod.geometry_engine_info());
            wasmState.info = info;
            console.log('[geometry_engine] info:', info);
          } catch (e) {
            wasmState.info = null;
          }

          // ── Smoke-test: HORIZONTAL constraint ─────────────────
          try {
            const testSketch = {
              schema: 'sketch_graph', version: 1,
              workingPlane: 'XZ', gridSize: 0.01,
              points: [
                { id: 'p1', gx: 0,  gy: 0, gz: 0,  x: 0.0,  y: 0.0, z: 0.0  },
                { id: 'p2', gx: 10, gy: 0, gz: 2,  x: 0.1,  y: 0.0, z: 0.02 }
              ],
              edges: [{ id: 'e1', a: 'p1', b: 'p2' }],
              constraints: [{ id: 'c1', type: 'HORIZONTAL', targetType: 'edge', targetId: 'e1', value: null }],
              profiles: []
            };
            const solved = JSON.parse(mod.wasm_solve_constraints(JSON.stringify({ sketch: testSketch })));
            if (solved.ok && solved.sketch) {
              const pts = solved.sketch.points;
              const p1gz = pts.find(p => p.id === 'p1')?.gz;
              const p2gz = pts.find(p => p.id === 'p2')?.gz;
              if (p1gz === p2gz) {
                console.log('%c[geometry_engine] ✅ solver OK', 'color:green;font-weight:bold');
              } else {
                console.warn('[geometry_engine] ⚠️ solver: p1.gz', p1gz, '!== p2.gz', p2gz);
              }
            }
          } catch (e) {
            console.error('[geometry_engine] smoke-test FAILED:', e);
          }

          wasmState.status = 'ready';
          window.__setStatusMessage('geometry_engine готов'
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

      window.__ensureSketchWasm = async function() {
        if (wasmState.status === 'ready') return true;
        return await window.__loadSketchWasm();
      };

      // ── Validate sketch ───────────────────────────────────────
      window.__wasmValidateSketch = async function() {
        if (!(await window.__ensureSketchWasm())) return null;
        const payload = JSON.stringify({ sketch: window.__sketchToJSON() });
        const t0  = performance.now();
        const out = window.__wasmModule.wasm_validate_sketch(payload);
        wasmState.lastValidateMs = performance.now() - t0;
        try { return JSON.parse(out); } catch (e) { return null; }
      };

      // ── Low-level callers ─────────────────────────────────────
      window.__wasmAddPoint = function(request) {
        if (wasmState.status !== 'ready' || !window.__wasmModule) return null;
        const t0 = performance.now();
        const out = window.__wasmModule.wasm_add_point(JSON.stringify(request));
        wasmState.lastAddMs = performance.now() - t0;
        try { return JSON.parse(out); } catch (e) { return null; }
      };

      window.__wasmAddEdge = function(request) {
        if (wasmState.status !== 'ready' || !window.__wasmModule) return null;
        const t0 = performance.now();
        const out = window.__wasmModule.wasm_add_edge(JSON.stringify(request));
        wasmState.lastAddMs = performance.now() - t0;
        try { return JSON.parse(out); } catch (e) { return null; }
      };

      window.__wasmMovePoint = function(request) {
        if (wasmState.status !== 'ready' || !window.__wasmModule) return null;
        const t0 = performance.now();
        const out = window.__wasmModule.wasm_move_point(JSON.stringify(request));
        wasmState.lastAddMs = performance.now() - t0;
        try { return JSON.parse(out); } catch (e) { return null; }
      };

      // ── sketch_extrude_json — THE unified 2D→3D path ─────────
      // Input:  { sketch: SketchGraph, depth_m, plane?, bevel?, profile_id? }
      // Output: { ok, positions, normals, face_ids, indices, vertex_count, ... }
      window.__wasmSketchExtrude = function(request) {
        if (wasmState.status !== 'ready' || !window.__wasmModule) return null;
        const t0 = performance.now();
        const out = window.__wasmModule.sketch_extrude_json(JSON.stringify(request));
        const dt = performance.now() - t0;
        try {
          const r = JSON.parse(out);
          r.__dt = dt.toFixed(1);
          return r;
        } catch (e) { return null; }
      };

      // ── Sketch signature for hybrid cross-check ───────────────
      window.__sketchSignature = function(g) {
        if (!g) return '';
        const pts = (g.points || []).map(p => p.id + '@' + p.gx + ',' + p.gy + ',' + p.gz).sort().join('|');
        const eds = (g.edges  || []).map(e => { const lo = e.a<=e.b?e.a:e.b, hi = e.a<=e.b?e.b:e.a; return e.id+':'+lo+'-'+hi; }).sort().join('|');
        return 'P[' + pts + '] E[' + eds + ']';
      };

      // ── High-level: add point + apply ────────────────────────
      window.__wasmAddPointAndApply = async function(gx, gy, gz) {
        if (!(await window.__ensureSketchWasm())) {
          window.__setStatusMessage('WASM не готов');
          return { ok: false, error: wasmState.error || 'wasm not ready' };
        }
        const result = window.__wasmAddPoint({
          sketch: window.__sketchToJSON(),
          workingPlane: sketchState.workingPlane,
          gridSize: sketchState.gridSize,
          gx: gx|0, gy: gy|0, gz: gz|0,
        });
        sketchState.lastWasmMs = wasmState.lastAddMs;
        if (!result) return { ok: false, error: 'parse' };
        if (result.ok) {
          window.__applyBackendSketchResult(result);
          const pid = result.createdPointId || result.reusedPointId;
          window.__setStatusMessage((result.createdPointId ? 'Точка создана ' : 'Точка переиспользована ') + pid + ' (' + wasmState.lastAddMs.toFixed(1) + ' мс)');
          return { ok: true, pointId: pid, created: !!result.createdPointId };
        }
        window.__setStatusMessage(result.message || 'Точка отклонена');
        return { ok: false, error: result.message };
      };

      window.__wasmAddEdgeAndApply = async function(startRef, endRef) {
        if (!(await window.__ensureSketchWasm())) {
          window.__setStatusMessage('WASM не готов');
          return { ok: false, error: wasmState.error || 'wasm not ready' };
        }
        const result = window.__wasmAddEdge({
          sketch: window.__sketchToJSON(),
          workingPlane: sketchState.workingPlane,
          gridSize: sketchState.gridSize,
          start: startRef,
          end: endRef,
        });
        sketchState.lastWasmMs = wasmState.lastAddMs;
        if (!result) return { ok: false, error: 'parse' };
        if (result.sketch) window.__applyBackendSketchResult(result);
        if (result.ok) {
          window.__setStatusMessage('Ребро создано ' + (result.createdEdgeId||'?') + ' (' + wasmState.lastAddMs.toFixed(1) + ' мс)');
          return { ok: true, edgeId: result.createdEdgeId };
        }
        window.__setStatusMessage(result.message || 'Ребро отклонено');
        return { ok: false, error: result.message };
      };

      // ── Engine mode ───────────────────────────────────────────
      window.__setEngineMode = async function(mode) {
        if (!['backend','wasm','hybrid'].includes(mode)) mode = 'wasm';
        sketchState.engineMode = mode;
        sketchState.lastCommandMsg = '—';
        sketchState.useBackendCommands = (mode === 'backend');
        if (mode === 'wasm' || mode === 'hybrid') {
          window.__ensureSketchWasm().then(() => {
            if (window.__updateSketchInspector) window.__updateSketchInspector();
          });
        }
        window.__setStatusMessage('Режим: ' + mode.toUpperCase());
        if (window.__updateSketchInspector) window.__updateSketchInspector();
      };

      window.__setUseWasmEngine = function(v) { window.__setEngineMode(v ? 'wasm' : 'backend'); };

      // ── Eager preload ─────────────────────────────────────────
      if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', () => {
          window.__loadSketchWasm().catch(e => console.warn('[wasm preload]', e));
        });
      } else {
        setTimeout(() => { window.__loadSketchWasm().catch(e => console.warn('[wasm preload]', e)); }, 0);
      }
"##;
//
// Phase 10 shipped: lazy ES-module import + validate.
// Phase 11 adds:   __wasmAddPoint / __wasmAddEdge + Hybrid cross-check.
//
// Three engine modes (sketchState.engineMode):
//   * 'backend' — HTTP-only (Phase 7 path).
//   * 'wasm'    — pure browser (no backend).
//   * 'hybrid'  — WASM applies instantly, backend cross-checks asynchronously
