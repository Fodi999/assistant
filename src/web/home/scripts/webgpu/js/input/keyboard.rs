// ── JS: Keyboard input ────────────────────────────────────────────────────────
// Domain: User input — keydown/keyup handlers.
// Handles:
//   - Space held flag (pan modifier)
//   - Sketch tool hotkeys via __handleSketchKey
//   - Particle count shortcuts (1–5, +/-)
//   - Camera / scene shortcuts (r, b, c, v, w, s, n, m, i, f, [, ])
//   - Perf HUD toggle (Shift+P via perf_hud.rs __initPerfHud)

pub const JS: &str = r##"
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
      let spaceHeld = false;
      let _spacePressTime = 0;

      document.addEventListener('keydown', (e) => {
        if (e.code === 'Space') {
          spaceHeld = true;
          _spacePressTime = performance.now();
          e.preventDefault(); // не скроллить страницу
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
"##;
