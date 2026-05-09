// ── JS: Matter Lab UI binding — sync state ↔ DOM, action handlers ────────────
// Domain: Presentation — wires DOM events of the new Matter Lab panels into
// existing engine objects (cam, shape, formation, cellSdf, setParticleCount).

pub const JS: &str = r##"
      // ── 8b. Matter Lab UI ───────────────────────────────────────

      // ── DOM sync ──
      function syncMatterUi() {
        engineState.performance.fps     = __matterPerf.fps;
        engineState.performance.frameMs = __matterPerf.frameMs;

        const setText = (id, v) => { const el = document.getElementById(id); if (el) el.textContent = v; };

        setText('fpsValue',       engineState.performance.fps.toFixed(0));
        setText('frameValue',     engineState.performance.frameMs.toFixed(1) + 'ms');
        
      }

      function resetMatter() {
        // No-op for now
      }

      // ── bind once on startup ──
      function bindMatterUi() {
        // close button → return to landing
        const close = document.getElementById('close-chefos');
        if (close) close.addEventListener('click', () => {
          document.body.classList.remove('engine-open');
        });

        // periodic refresh for fps / frame
        setInterval(syncMatterUi, 250);
        syncMatterUi();
      }

      // expose for render_loop perf hook
      globalThis.__matterPerf = __matterPerf;

      // delay binding until DOM is ready (script is at bottom but be safe)
      if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', bindMatterUi);
      } else {
        bindMatterUi();
      }

"##;
