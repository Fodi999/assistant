// ── JS: Performance HUD — frame/render/overlay/pick/backend timing ─────────
// Domain: Diagnostics — rolling-average metrics + DOM HUD (throttled to 4 Hz).
// Hotkey: Shift+P toggles visibility. Does NOT conflict with P (Point tool).

pub const JS: &str = r##"
      // ── perfState ─────────────────────────────────────────────
      window.perfState = {
        enabled: true,
        collapsed: false,

        fps: 0,
        frameMs: 0,
        renderMs: 0,
        overlayMs: 0,
        pickMs: 0,
        backendMs: 0,

        frameSamples:   [],
        renderSamples:  [],
        overlaySamples: [],
        pickSamples:    [],
        backendSamples: [],

        lastFrameTime: performance.now(),
        lastHudUpdate: 0,

        dpr: window.devicePixelRatio || 1,
        resolutionScale: 1,

        backendError: false,
        backendCalls: 0,
      };

      const PERF_MAX_SAMPLES = 60;

      function __perfPush(arr, v) {
        arr.push(v);
        if (arr.length > PERF_MAX_SAMPLES) arr.shift();
      }
      function __perfAvg(arr) {
        if (!arr.length) return 0;
        let s = 0;
        for (let i = 0; i < arr.length; i++) s += arr[i];
        return s / arr.length;
      }

      // Record a single sample for a named metric.
      //   name ∈ {'frame','render','overlay','pick','backend'}
      window.__perfSample = function(name, ms) {
        const s = window.perfState;
        if (!s || !isFinite(ms)) return;
        switch (name) {
          case 'frame':   __perfPush(s.frameSamples,   ms); s.frameMs   = ms; break;
          case 'render':  __perfPush(s.renderSamples,  ms); s.renderMs  = ms; break;
          case 'overlay': __perfPush(s.overlaySamples, ms); s.overlayMs = ms; break;
          case 'pick':    __perfPush(s.pickSamples,    ms); s.pickMs    = ms; break;
          case 'backend':
            __perfPush(s.backendSamples, ms);
            s.backendMs = ms;
            s.backendError = false;
            s.backendCalls++;
            break;
        }
      };

      // Lightweight begin/end pair — returns an opaque token.
      window.__perfBegin = function(name) {
        return { name, t: performance.now() };
      };
      window.__perfEnd = function(tok) {
        if (!tok) return;
        window.__perfSample(tok.name, performance.now() - tok.t);
      };

      // Mark backend as errored (show 'ERR' instead of ms) on transport failure.
      window.__perfMarkBackendError = function() {
        if (window.perfState) window.perfState.backendError = true;
      };

      // Toggle visibility (Shift+P).
      window.__togglePerfHud = function() {
        const s = window.perfState; if (!s) return;
        s.enabled = !s.enabled;
        const hud = document.getElementById('perf-hud');
        if (hud) hud.style.display = s.enabled ? '' : 'none';
      };

      function __perfFpsClass(fps) {
        if (fps >= 55) return 'ok';
        if (fps >= 30) return 'warn';
        return 'bad';
      }
      function __perfFrameClass(ms) {
        if (ms > 33)   return 'bad';
        if (ms > 16.7) return 'warn';
        return 'ok';
      }

      // DOM update — throttled to ~4 Hz to keep HUD itself cheap.
      window.__updatePerfHud = function() {
        const s = window.perfState;
        if (!s || !s.enabled) return;
        const now = performance.now();
        if (now - s.lastHudUpdate < 250) return;
        s.lastHudUpdate = now;

        const hud = document.getElementById('perf-hud');
        if (!hud) return;

        const frameAvg = __perfAvg(s.frameSamples);
        const fps      = frameAvg > 0 ? (1000 / frameAvg) : s.fps;

        const setText = (id, v) => { const el = document.getElementById(id); if (el) el.textContent = v; };
        const setVal  = (id, v, cls) => {
          const el = document.getElementById(id);
          if (!el) return;
          el.textContent = v;
          el.className   = 'perf-val' + (cls ? (' ' + cls) : '');
        };

        setVal('perf-fps',    fps.toFixed(0),                __perfFpsClass(fps));
        setVal('perf-frame',  frameAvg.toFixed(1) + ' ms',   __perfFrameClass(frameAvg));
        setText('perf-render',  __perfAvg(s.renderSamples).toFixed(2)  + ' ms');
        setText('perf-overlay', __perfAvg(s.overlaySamples).toFixed(2) + ' ms');
        setText('perf-pick',    __perfAvg(s.pickSamples).toFixed(2)    + ' ms');
        if (s.backendError) {
          setVal('perf-backend', 'ERR', 'bad');
        } else if (s.backendCalls === 0) {
          setVal('perf-backend', '—', '');
        } else {
          setVal('perf-backend', __perfAvg(s.backendSamples).toFixed(1) + ' ms', '');
        }

        const ss = window.sketchState;
        if (ss) {
          setText('perf-pts',      ss.points.length);
          setText('perf-edges',    ss.edges.length);
          setText('perf-profiles', (ss.profiles && ss.profiles.length) || 0);
          const sel = (ss.selectedPointIds ? ss.selectedPointIds.size : 0)
                    + (ss.selectedEdgeIds  ? ss.selectedEdgeIds.size  : 0);
          setText('perf-selected', sel);
          // Engine mode metrics (Phase 11).
          setText('perf-mode', ss.engineMode || 'backend');
          const wms = ss.lastWasmMs || 0;
          const bms = ss.lastBackendMs || 0;
          setText('perf-wasm-ms', wms > 0 ? wms.toFixed(2) + ' ms' : '—');
          setText('perf-be-ms',   bms > 0 ? bms.toFixed(1) + ' ms' : '—');
        }

        s.dpr = window.devicePixelRatio || 1;
        setText('perf-dpr',   s.dpr.toFixed(2));
        setText('perf-scale', s.resolutionScale.toFixed(2));
        const cv = document.getElementById('webgpu-canvas');
        if (cv) setText('perf-canvas', cv.width + '×' + cv.height);
      };

      // Header click → collapse/expand body.
      document.addEventListener('DOMContentLoaded', () => {
        const hd = document.getElementById('perf-hud-header');
        const ph = document.getElementById('perf-hud');
        if (hd) {
          hd.addEventListener('click', () => {
            const s = window.perfState;
            const body = document.getElementById('perf-hud-body');
            if (!s || !body) return;
            s.collapsed = !s.collapsed;
            body.style.display = s.collapsed ? 'none' : '';
            const caret = document.getElementById('perf-hud-caret');
            if (caret) caret.textContent = s.collapsed ? '\u25b8' : '\u25be';
          });
        }
        // Make perf-hud draggable via its header
        if (ph && hd && window.__modalTheme) {
          window.__modalTheme.makeDraggable(ph, hd);
          window.__modalTheme.blockCanvasEvents(ph);
        }
      });
"##;
