// ── JS: keyboard and particle-count controls ──────────────────────────────────────
// Domain: User input — setParticleCount helper, keyboard shortcut dispatch.

pub const JS: &str = r##"
      // ── Particle count switching ────────────────────────────────
      function setParticleCount(n) {
        n = Math.max(1, Math.min(MAX_PARTICLES, Math.floor(n)));
        if (n === NUM_SPHERES) return;
        NUM_SPHERES = n;
        sphereData  = buildParticles(n);
        device.queue.writeBuffer(sphereBuf, 0, sphereData);
        updateCameraForCount(n);
        log(`↻ particles = ${n.toLocaleString()}`, '#a78bfa');
      }

      // ── Keyboard ────────────────────────────────────────────────
      const onKey = (e) => {
        switch (e.key) {
          case '1': setParticleCount(   1_000); break;
          case '2': setParticleCount(  10_000); break;
          case '3': setParticleCount( 100_000); break;
          case '4': setParticleCount( 500_000); break;
          case '5': setParticleCount(1_000_000); break;
          case '+': case '=': setParticleCount(Math.round(NUM_SPHERES * 1.5)); break;
          case '-': case '_': setParticleCount(Math.round(NUM_SPHERES / 1.5)); break;
          case 'r': case 'R': cam.autoRotate = !cam.autoRotate; break;
          case 'b': case 'B':
            // Shift+B → benchmark, plain B → toggle "PARTICLE SCENE" debug HUD
            if (e.shiftKey) {
              if (!bench.running) runBenchmark();
            } else {
              window.__toggleHud?.();
            }
            break;
          case '[':
            if (cellSdf.on) cellSdf.radius = Math.max(0,   cellSdf.radius - 0.05);
            else            shape.roundness = Math.max(0,   shape.roundness - 0.1);
            break;
          case ']':
            if (cellSdf.on) cellSdf.radius = Math.min(0.5, cellSdf.radius + 0.05);
            else            shape.roundness = Math.min(1,   shape.roundness + 0.1);
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
            // preset: mask color + hide low (edges+corners highlighted)
            cellSdf.colorMode = 2;
            cellSdf.hideLow   = true;
            log('◇ preset = face/edge/corner', '#67e8f9');
            break;
        }
      };
      window.addEventListener('keydown', onKey);
"##;
