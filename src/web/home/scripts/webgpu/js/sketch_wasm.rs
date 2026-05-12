// ── JS: WASM bridge — shared sketch_engine in the browser ────────────────
// Domain: Geometry — lazy-loads /wasm/sketch_engine/sketch_engine.js and
// exposes validate / add-point / add-edge as window globals.
//
// Backend HTTP endpoints remain authoritative; this is a parallel local
// engine that can be used for instant validation and offline editing.

pub const JS: &str = r##"
      // wasmState lives on window for inspector visibility.
      window.wasmState = {
        status: 'not_loaded',   // 'not_loaded' | 'loading' | 'ready' | 'error'
        message: '',
        info: null,             // { name, version, schema, schemaVersion }
        module: null,           // the imported ES module
        lastValidate: null,     // last ValidationResult
      };

      // useWasmEngine toggle (UI bridge — NOT yet wired to commands).
      if (typeof sketchState !== 'undefined' && sketchState.useWasmEngine === undefined) {
        sketchState.useWasmEngine = false;
      }

      function __wasmSetStatus(status, message) {
        window.wasmState.status  = status;
        window.wasmState.message = message || '';
        const el = document.getElementById('si-wasm-status');
        if (el) {
          el.textContent = status + (message ? ' · ' + message : '');
          el.className = 'si-wasm-status si-wasm-' + status;
        }
      }

      // Lazy import. Safe to call multiple times — re-uses the module after first load.
      window.__loadSketchWasm = async function() {
        const s = window.wasmState;
        if (s.status === 'ready')   return s.module;
        if (s.status === 'loading') return null;
        __wasmSetStatus('loading', '');
        try {
          const mod = await import('/wasm/sketch_engine/sketch_engine.js');
          // The default export is the init() function (target=web).
          await mod.default('/wasm/sketch_engine/sketch_engine_bg.wasm');
          s.module = mod;
          if (typeof mod.wasm_engine_info === 'function') {
            try { s.info = JSON.parse(mod.wasm_engine_info()); } catch (_) { s.info = null; }
          }
          __wasmSetStatus('ready', s.info ? ('v' + s.info.version) : '');
          if (window.__setStatusMessage) window.__setStatusMessage('Sketch WASM engine loaded');
          return mod;
        } catch (e) {
          __wasmSetStatus('error', String(e && e.message || e));
          if (window.__setStatusMessage) window.__setStatusMessage('WASM load failed: ' + s.message);
          return null;
        }
      };

      // Validate current sketch locally using WASM.
      // Returns parsed ValidationResult or null on failure.
      window.__wasmValidateSketch = async function() {
        const mod = await window.__loadSketchWasm();
        if (!mod || typeof mod.wasm_validate_sketch !== 'function') return null;
        const payload = JSON.stringify({ sketch: window.__sketchToJSON() });
        const t0 = performance.now();
        const raw = mod.wasm_validate_sketch(payload);
        const dt = performance.now() - t0;
        let parsed = null;
        try { parsed = JSON.parse(raw); } catch (_) { return null; }
        window.wasmState.lastValidate = parsed;
        window.wasmState.lastValidateMs = dt;
        if (window.__setStatusMessage) {
          const note = parsed.ok ? 'ok' : (parsed.issues || []).length + ' issue(s)';
          window.__setStatusMessage('WASM validate: ' + note + ' (' + dt.toFixed(2) + ' ms)');
        }
        return parsed;
      };

      // request: { sketch, workingPlane, gridSize, gx, gy, gz }
      window.__wasmAddPoint = async function(request) {
        const mod = await window.__loadSketchWasm();
        if (!mod || typeof mod.wasm_add_point !== 'function') return null;
        try { return JSON.parse(mod.wasm_add_point(JSON.stringify(request))); }
        catch (e) { return null; }
      };

      // request: { sketch, workingPlane, gridSize, start, end }
      window.__wasmAddEdge = async function(request) {
        const mod = await window.__loadSketchWasm();
        if (!mod || typeof mod.wasm_add_edge !== 'function') return null;
        try { return JSON.parse(mod.wasm_add_edge(JSON.stringify(request))); }
        catch (e) { return null; }
      };

      // Toggle: when ON, sketch commands will *prefer* WASM (future phase).
      // Today it only flips the flag; backend commands stay authoritative.
      window.__setUseWasmEngine = function(v) {
        if (typeof sketchState === 'undefined') return;
        sketchState.useWasmEngine = !!v;
        if (window.__setStatusMessage) window.__setStatusMessage('WASM engine: ' + (v ? 'ON' : 'OFF'));
        if (window.__updateSketchInspector) window.__updateSketchInspector();
      };
"##;
