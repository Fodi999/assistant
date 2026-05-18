// ── JS: Keyboard input ────────────────────────────────────────────────────────
// Domain: User input — keydown/keyup handlers.
// Handles:
//   - Space held flag (pan modifier)
//   - Sketch tool hotkeys via __handleSketchKey
//   - Particle count shortcuts (1–5, +/-)
//   - Camera / scene shortcuts (r, b, c, v, w, s, n, m, i, f, [, ])
//   - Perf HUD toggle (Shift+P via perf_hud.rs __initPerfHud)

pub const JS: &str = r##"
      // ── Guard: keyboard listeners must be registered only once ───
      // startWebGpuScene() can be called multiple times (auto-open +
      // manual re-open). Without this guard every call stacks another
      // keydown listener, causing shortcuts to fire N times.

      // These must live on window so mouse.rs and other modules can read them
      // even when the keyboard init guard skips re-registration.
      if (typeof window.__spaceHeld === 'undefined') window.__spaceHeld = false;
      if (typeof window.__spacePressTime === 'undefined') window.__spacePressTime = 0;

      // Compatibility shim: code inside startWebGpuScene references the bare
      // names; point them at the window properties so they stay accessible
      // regardless of whether the if-block below runs.
      Object.defineProperty(window, 'spaceHeld', {
        get() { return window.__spaceHeld; },
        set(v) { window.__spaceHeld = v; },
        configurable: true,
      });
      if (!window.hasOwnProperty('_spacePressTime')) {
        Object.defineProperty(window, '_spacePressTime', {
          get() { return window.__spacePressTime; },
          set(v) { window.__spacePressTime = v; },
          configurable: true,
        });
      }

      if (!window.__keyboardInited) {
      window.__keyboardInited = true;

      // ── Particle count helper (used by keyboard shortcuts) ──────
      function setParticleCount(n) {
        n = Math.max(1, Math.min(MAX_PARTICLES, Math.floor(n)));
        if (n === NUM_SPHERES) return;
        NUM_SPHERES = n;
        sphereData  = buildParticles(n);
        device.queue.writeBuffer(sphereBuf, 0, sphereData);
        updateCameraForCount(n);
        log(`↻ particles = ${n.toLocaleString()}`, '#a78bfa');
      }

      // ── Space key — pan modifier + frame scene ───────────────────
      // (spaceHeld / _spacePressTime live on window, defined above the guard)

      // ── Shortcuts / cursor-info toggle helpers ────────────────────────
      window.__cursorInfoVisible = false;

      function __toggleShortcutsOverlay() {
        const ov = document.getElementById('shortcuts-overlay');
        if (!ov) return;
        ov.style.display = ov.style.display === 'none' ? '' : 'none';
      }

      function __toggleCursorInfo() {
        window.__cursorInfoVisible = !window.__cursorInfoVisible;
        // Update ? button visual state
        const btn = document.getElementById('shortcuts-toggle');
        if (btn) btn.classList.toggle('active', window.__cursorInfoVisible);
        if (!window.__cursorInfoVisible) {
          const hud = document.getElementById('cursor-hud');
          if (hud) hud.style.display = 'none';
        }
        if (window.__setStatusMessage)
          window.__setStatusMessage(window.__cursorInfoVisible ? '⊙ Курсор ВКЛ' : '⊙ Курсор ВЫКЛ');
      }

      // Event delegation — works regardless of DOM ready state
      document.addEventListener('click', (e) => {
        const id = e.target && e.target.id;
        if (id === 'shortcuts-toggle') {
          __toggleShortcutsOverlay();
          return;
        }
        if (id === 'shortcuts-close') {
          const ov = document.getElementById('shortcuts-overlay');
          if (ov) ov.style.display = 'none';
          return;
        }
      });

      document.addEventListener('keydown', (e) => {
        if (e.code === 'Space') {
          spaceHeld = true;
          _spacePressTime = performance.now();
          e.preventDefault();
        }

        // ── Numpad camera presets (Blender / SketchUp style) ─────────
        // Numpad 1 = Front, 3 = Right, 7 = Top, 9 = Back
        // Numpad 5 = Ortho toggle, Numpad 0 = Iso
        // Ctrl+Numpad = opposite view (Blender: Ctrl+1=Back, Ctrl+3=Left, Ctrl+7=Bottom)
        if (e.code.startsWith('Numpad') && !e.ctrlKey && !e.metaKey && !e.altKey) {
          // Don't fire if an input is focused
          const ae = document.activeElement;
          if (!ae || (ae.tagName !== 'INPUT' && ae.tagName !== 'TEXTAREA')) {
            const __animCam = function(targetYaw, targetPitch) {
              const FRAMES = 20;
              const y0 = cam.yaw, p0 = cam.pitch;
              // Normalize yaw delta to shortest arc
              let dy = ((targetYaw - y0) % (Math.PI*2) + Math.PI*3) % (Math.PI*2) - Math.PI;
              let dp = targetPitch - p0;
              let f = 0;
              function _s() {
                f++;
                const ease = 1 - Math.pow(1 - f/FRAMES, 3);
                cam.yaw   = y0 + dy * ease;
                cam.pitch = p0 + dp * ease;
                if (f < FRAMES) requestAnimationFrame(_s);
              }
              requestAnimationFrame(_s);
            };
            let handled = true;
            switch (e.code) {
              case 'Numpad1': __animCam(0,              0);                     break; // Front
              case 'Numpad3': __animCam(Math.PI*0.5,    0);                     break; // Right
              case 'Numpad7': __animCam(cam.yaw,        -Math.PI*0.5 + 0.001); break; // Top
              case 'Numpad9': __animCam(Math.PI,        0);                     break; // Back
              case 'Numpad0': __animCam(Math.PI*0.25,  -0.615);                 break; // Iso
              case 'Numpad5': cam.ortho = !cam.ortho;
                              if (window.__setStatusMessage)
                                window.__setStatusMessage(cam.ortho ? '□ Ortho' : '⊙ Perspective');
                              break;
              case 'NumpadAdd':      cam.dist = Math.max(0.01, cam.dist * 0.8); break; // Zoom in
              case 'NumpadSubtract': cam.dist = Math.min(500,  cam.dist * 1.25);break; // Zoom out
              default: handled = false;
            }
            if (handled) { e.preventDefault(); return; }
          }
        }
        // Ctrl+Numpad: opposite views
        if (e.ctrlKey && e.code.startsWith('Numpad')) {
          const ae = document.activeElement;
          if (!ae || (ae.tagName !== 'INPUT' && ae.tagName !== 'TEXTAREA')) {
            const __animCam = function(ty, tp) {
              const FRAMES = 20; const y0 = cam.yaw, p0 = cam.pitch;
              let dy = ((ty - y0) % (Math.PI*2) + Math.PI*3) % (Math.PI*2) - Math.PI;
              let f = 0;
              function _s() { f++; const e2 = 1 - Math.pow(1 - f/FRAMES, 3);
                cam.yaw = y0 + dy*e2; cam.pitch = p0 + (tp-p0)*e2;
                if (f < FRAMES) requestAnimationFrame(_s); }
              requestAnimationFrame(_s);
            };
            let handled = true;
            switch (e.code) {
              case 'Numpad1': __animCam(Math.PI,       0);                     break; // Back
              case 'Numpad3': __animCam(-Math.PI*0.5,  0);                     break; // Left
              case 'Numpad7': __animCam(cam.yaw,        Math.PI*0.5 - 0.001); break; // Bottom
              default: handled = false;
            }
            if (handled) { e.preventDefault(); return; }
          }
        }
        // ? key — toggle cursor info HUD
        if (e.key === '?' && !e.ctrlKey && !e.metaKey) {
          __toggleCursorInfo();
          return;
        }
        // / key (same physical key, no shift) — open shortcuts overlay
        if (e.key === '/' && !e.ctrlKey && !e.metaKey && !e.shiftKey) {
          __toggleShortcutsOverlay();
          return;
        }
        // Track Shift for temporary snap-disable / free-mode (Phase 12).
        if (e.shiftKey && window.sketchState && window.sketchState.precision) {
          window.sketchState.precision.shiftHeld = true;
        }
        // Delegate to sketch tool handler first.
        if (window.__handleSketchKey && window.__handleSketchKey(e)) return;
      });

      document.addEventListener('keyup', (e) => {
        if (e.code === 'Space') {
          spaceHeld = false;
          // Если удержание было меньше 300 мс — считаем кликом, центрируем сцену
          if (performance.now() - _spacePressTime < 300) {
            if (window.__frameCenterScene) window.__frameCenterScene();
          }
        }
        if (!e.shiftKey && window.sketchState && window.sketchState.precision) {
          window.sketchState.precision.shiftHeld = false;
        }
      });

      // ── General keyboard shortcuts ───────────────────────────────
      const onKey = (e) => {
        switch (e.key) {
          case '1': setParticleCount(    1_000); break;
          case '2': setParticleCount(   10_000); break;
          case '3': setParticleCount(  100_000); break;
          case '4': setParticleCount(  500_000); break;
          case '5': setParticleCount(1_000_000); break;
          case '+': case '=': setParticleCount(Math.round(NUM_SPHERES * 1.5)); break;
          case '-': case '_': setParticleCount(Math.round(NUM_SPHERES / 1.5)); break;
          case 'r': case 'R': cam.autoRotate = !cam.autoRotate; break;
          case 'b': case 'B':
            if (e.shiftKey) { if (!bench.running) runBenchmark(); }
            else            { window.__toggleHud?.(); }
            break;
          case '[':
            // Sketch grid size halve (Phase 12). Falls back to legacy cellSdf/shape if no sketch.
            if (window.sketchState && window.__cycleGridSize) {
              window.__cycleGridSize(-1);
            } else if (cellSdf.on) {
              cellSdf.radius     = Math.max(0,   cellSdf.radius     - 0.05);
            } else {
              shape.roundness    = Math.max(0,   shape.roundness    - 0.1);
            }
            break;
          case ']':
            if (window.sketchState && window.__cycleGridSize) {
              window.__cycleGridSize(+1);
            } else if (cellSdf.on) {
              cellSdf.radius     = Math.min(0.5, cellSdf.radius     + 0.05);
            } else {
              shape.roundness    = Math.min(1,   shape.roundness    + 0.1);
            }
            break;
          case 'c': case 'C': setFormation('cloud'); break;
          case 'v': case 'V': setFormation('cube');  break;
          case 'w': case 'W': setFormation('wall');  break;
          case 's': case 'S': toggleCellSdf();       break;
          case 'n': case 'N':
            cellSdf.colorMode = (cellSdf.colorMode === 1) ? 0 : 1;
            log(`◇ debug = ${cellSdf.colorMode === 1 ? 'normals-RGB' : 'off'}`, '#fbbf24');
            break;
          case 'm': case 'M':
            cellSdf.colorMode = (cellSdf.colorMode === 2) ? 0 : 2;
            log(`◇ debug = ${cellSdf.colorMode === 2 ? 'mask-color' : 'off'}`, '#f0abfc');
            break;
          case 'i': case 'I':
            cellSdf.hideLow = !cellSdf.hideLow;
            log(`◇ hide-low = ${cellSdf.hideLow ? 'ON (edges+corners only)' : 'off'}`, '#f0abfc');
            break;
          case 'f': case 'F':
            cellSdf.colorMode = 2;
            cellSdf.hideLow   = true;
            log('◇ preset = face/edge/corner', '#67e8f9');
            break;
        }
      };
      window.addEventListener('keydown', onKey);

      } // end __keyboardInited guard
"##;
