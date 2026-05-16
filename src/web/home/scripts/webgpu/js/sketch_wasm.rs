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
            __wasmModulePromise = import('/wasm/sketch_engine/sketch_engine.js');
          }
          const mod = await __wasmModulePromise;
          await mod.default({ module_or_path: '/wasm/sketch_engine/sketch_engine_bg.wasm' });
          window.__wasmModule = mod;
          // Handshake check.
          try {
            const info = JSON.parse(mod.wasm_engine_info());
            wasmState.info = info;
          } catch (e) {
            wasmState.info = null;
          }
          wasmState.status = 'ready';
          window.__setStatusMessage('WASM engine ready'
            + (wasmState.info ? (' v' + wasmState.info.version) : ''));
        } catch (e) {
          wasmState.status = 'error';
          wasmState.error  = String(e && e.message || e);
          window.__setStatusMessage('WASM load failed: ' + wasmState.error);
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
          window.__setStatusMessage('WASM validate ok ' + dt.toFixed(2) + ' ms');
        } else {
          window.__setStatusMessage('WASM validate parse error');
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
        if (wasmState.status !== 'ready' || !window.__wasmModule) return null;
        if (!window.__wasmModule.wasm_move_point) return null;
        const payload = JSON.stringify(request);
        const t0  = performance.now();
        const out = window.__wasmModule.wasm_move_point(payload);
        const dt  = performance.now() - t0;
        wasmState.lastAddMs = dt;
        try { return JSON.parse(out); } catch (e) { return null; }
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
          window.__setStatusMessage('WASM not ready — point not added');
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
          window.__setStatusMessage('WASM add-point parse error');
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
          window.__setStatusMessage('WASM not ready — edge not added');
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
          window.__setStatusMessage('WASM add-edge parse error');
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
        window.__setStatusMessage('Engine mode: ' + mode.toUpperCase());
        if (window.__updateSketchInspector) window.__updateSketchInspector();
      };

      // Back-compat shim — Phase 10 inspector used a checkbox + this fn.
      window.__setUseWasmEngine = function(v) {
        window.__setEngineMode(v ? 'wasm' : 'backend');
      };
"##;
