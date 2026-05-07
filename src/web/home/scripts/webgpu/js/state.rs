// ── JS: application state — particles, camera, shape, formation, input ───────────
// Domain: Application state — all runtime state objects and pointer event wiring.

pub const JS: &str = r##"
      // ── 3. Particle data (rebuildable up to device limit) ───────
      // Each particle = 8 floats = 32 bytes.
      // Cap MAX_PARTICLES at whatever the GPU storage-buffer limit allows.
      const PARTICLE_STRIDE = 32;
      const HARD_CAP        = 5_000_000;
      const deviceCap       = Math.floor(device.limits.maxStorageBufferBindingSize / PARTICLE_STRIDE);
      const MAX_PARTICLES   = Math.min(HARD_CAP, deviceCap);
      let   NUM_SPHERES     = Math.min(1_000_000, MAX_PARTICLES);
      const CLOUD_VOLUME    = (4 / 3) * Math.PI * Math.pow(5.5, 3);
      log(`✓ MAX_PARTICLES = ${(MAX_PARTICLES/1e6).toFixed(2)}M  (buffer ${(MAX_PARTICLES*32/1048576).toFixed(0)} MB)`, '#a78bfa');

      function buildParticles(count) {
        const data = new Float32Array(count * 8);
        // shrink billboard size as count grows so cloud doesn't become a wall of light
        const sizeScale = Math.pow(50000 / Math.max(count, 1), 0.42); // ≈1 at 50k, ≈0.30 at 1M
        for (let i = 0; i < count; i++) {
          const b = i * 8;
          // sphere-distributed cloud (denser core, soft falloff)
          const r   = Math.pow(Math.random(), 0.55) * 5.5;
          const th  = Math.random() * 6.2832;
          const ph  = Math.acos(2 * Math.random() - 1);
          data[b + 0] = r * Math.sin(ph) * Math.cos(th);
          data[b + 1] = r * Math.cos(ph) * 0.75;
          data[b + 2] = r * Math.sin(ph) * Math.sin(th);
          data[b + 3] = (0.010 + Math.random() * 0.022) * sizeScale;
          const palette = Math.random();
          if (palette < 0.40) {
            data[b + 4] = 0.05 + Math.random() * 0.20;
            data[b + 5] = 0.65 + Math.random() * 0.35;
            data[b + 6] = 0.85 + Math.random() * 0.15;
          } else if (palette < 0.75) {
            data[b + 4] = 0.45 + Math.random() * 0.35;
            data[b + 5] = 0.15 + Math.random() * 0.30;
            data[b + 6] = 0.70 + Math.random() * 0.30;
          } else {
            data[b + 4] = 0.75 + Math.random() * 0.25;
            data[b + 5] = 0.80 + Math.random() * 0.20;
            data[b + 6] = 0.90 + Math.random() * 0.10;
          }
          data[b + 7] = Math.random() * 6.2832;
        }
        return data;
      }
      let sphereData = buildParticles(NUM_SPHERES);

      // ════════════════════════════════════════════════════════════
      // СИСТЕМА КООРДИНАТ (Y-вверх, правая рука)
      //
      //   Y  ↑         Центр мира (0,0,0) — пересечение красной X
      //      │          и синей Z осей на полу.
      //      │   Z ←   Пол — плоскость Y = 0.
      //      │ ↗        Объект стоит на полу: его нижняя грань = Y 0.
      //      └──── X →
      //
      // Позиция материи (shader: particles_vert.rs):
      //   worldPos = localPos * FORM_SCALE * objScale + objPos
      //   objPos.y = FORM_SCALE * objScale  ← нижняя грань на Y=0
      // ════════════════════════════════════════════════════════════

      // Полуразмеры formation (= paramB в render_loop.rs)
      const FORM_SCALE    = { cube: 1.8, cloud: 1.5, wall: 1.6 };
      // Масштаб объекта для каждой формации
      const FORM_OBJSCALE = { cube: 0.18, cloud: 0.30, wall: 0.22 };
      // Y_centre = половина высоты в world units → нижняя грань на Y=0
      function halfHeight(mode) {
        return FORM_SCALE[mode] * FORM_OBJSCALE[mode];
      }

      // ── Scene state (object placement) ─────────────────────────
      // X = лево/право,  Y = вверх/вниз,  Z = вперёд/назад
      const sceneState = {
        objectPosition: [0.0, halfHeight('cube'), 0.0],
        objectScale:    FORM_OBJSCALE.cube,
      };

      // Per-formation presets — нижняя грань прилипает к полу
      const formationDefaults = {
        cube:  { objectPosition:[0.0, halfHeight('cube'),  0.0], objectScale: FORM_OBJSCALE.cube,  dist: 3.25, target:[0.0, halfHeight('cube'),  0.0] },
        cloud: { objectPosition:[0.0, halfHeight('cloud'), 0.0], objectScale: FORM_OBJSCALE.cloud, dist: 3.75, target:[0.0, halfHeight('cloud'), 0.0] },
        wall:  { objectPosition:[0.0, halfHeight('wall'),  0.0], objectScale: FORM_OBJSCALE.wall,  dist: 3.5, target:[0.0, halfHeight('wall'),  0.0] },
      };

      // ── 4. Camera state ─────────────────────────────────────────
      const cam = {
        yaw:    0.81,
        pitch:  0.46,
        dist:   3.25,
        target: [0.0, halfHeight('cube'), 0.0],
        autoRotate: false,
      };
      // shape parameter: 0 = super-cube · 0.5 = octahedron (triangle silhouette) · 1 = super-sphere
      const shape = { roundness: 1.0 };
      // Piecewise map slider [0..1] → superquadric exponent n  (|x|ⁿ+|y|ⁿ+|z|ⁿ=1):
      //   r=0.0 → n=22  cube       (sharp planar faces)
      //   r=0.5 → n=1   octahedron (diamond: triangular silhouette, exact L1 SDF)
      //   r=1.0 → n=2   sphere     (perfectly round)
      // Lower half r∈[0,0.5] sweeps cube → octahedron (n: 22 → 1).
      // Upper half r∈[0.5,1] sweeps octahedron → sphere (n: 1 → 2).
      function shapeExponent(r) {
        const t = Math.max(0, Math.min(1, r));
        if (t <= 0.5) return 22.0 - 42.0 * t;          // 22 → 1
        return 1.0 + 2.0 * (t - 0.5);                  //  1 → 2
      }

      // ── Formation state ────────────────────────────────────────
      // mode: 'cloud' | 'cube' | 'wall'
      // mix:  0 (cloud) → 1 (fully formed); animated each frame
      // Default = cube, so the scene opens just like Blender's "Default Cube".
      const formation = { mode: 'cube', target: 1.0, mix: 1.0 };
      function setFormation(mode) {
        formation.mode   = mode;
        formation.target = (mode === 'cloud') ? 0.0 : 1.0;
        // re-frame scene per formation
        const d = formationDefaults[mode];
        if (d) {
          sceneState.objectPosition = d.objectPosition.slice();
          sceneState.objectScale    = d.objectScale;
          cam.dist   = d.dist;
          cam.target = d.target.slice();
        }
        log(`◇ formation = ${mode}`, '#f0abfc');
      }

      // ── Cell-SDF (kernel::particle_shape port) ────────────────
      // on        : when true, cube formation uses per-cell SDF instead of
      //             billboard imposters → flush seams + rounded outer hull.
      // radius    : 0..0.5 corner radius (cell-local units).
      // colorMode : 0 normal · 1 normals-as-RGB · 2 colour-by-SlotKind.
      // hideLow   : true → cull cells with ≤ 1 exposed face (show only edges/corners).
      const cellSdf = { on: false, radius: 0.25, colorMode: 0, hideLow: false };
      const floorGrid = { scale: 1.0 }; // 1.0 = m, 100.0 = cm, 1000.0 = mm

      function toggleCellSdf() {
        cellSdf.on = !cellSdf.on;
        log(`◇ cell-sdf = ${cellSdf.on ? 'ON' : 'off'}`, '#67e8f9');
      }
      function cycleColorMode() {
        cellSdf.colorMode = (cellSdf.colorMode + 1) % 3;
        const names = ['normal', 'normals-RGB', 'mask-color'];
        log(`◇ debug color = ${names[cellSdf.colorMode]}`, '#fbbf24');
      }

      const mouse = { ndcX: 999, ndcY: 999, active: false };

      // pointer interactions on canvas
      let dragging = false, panning = false, lastX = 0, lastY = 0;
      canvas.addEventListener('pointerdown', (e) => {
        canvas.setPointerCapture(e.pointerId);
        dragging = true;
        panning  = e.shiftKey || e.button === 2;
        lastX = e.clientX; lastY = e.clientY;
      });
      canvas.addEventListener('pointerup', (e) => {
        dragging = false; panning = false;
        try { canvas.releasePointerCapture(e.pointerId); } catch {}
      });
      canvas.addEventListener('pointermove', (e) => {
        // NDC for sand cursor (in screen space, aspect-corrected later in shader)
        const rect = canvas.getBoundingClientRect();
        mouse.ndcX = ((e.clientX - rect.left) / rect.width)  * 2 - 1;
        mouse.ndcY = 1 - ((e.clientY - rect.top) / rect.height) * 2;
        mouse.active = true;

        if (!dragging) return;
        const dx = e.clientX - lastX;
        const dy = e.clientY - lastY;
        lastX = e.clientX; lastY = e.clientY;

        if (panning) {
          // pan target on the camera plane
          const k = cam.dist * 0.0015;
          cam.target[0] -= dx * k * Math.cos(cam.yaw);
          cam.target[2] -= dx * k * Math.sin(cam.yaw);
          cam.target[1] += dy * k;
        } else {
          cam.yaw   += dx * 0.005;
          cam.pitch += dy * 0.005;
          // no clamp — full sphere rotation
        }
      });
      canvas.addEventListener('pointerleave', () => { mouse.active = false; });
      canvas.addEventListener('wheel', (e) => {
        e.preventDefault();
        const factor = Math.exp(e.deltaY * 0.001);
        cam.dist = Math.max(0.5, Math.min(80, cam.dist * factor));
      }, { passive: false });
      canvas.addEventListener('contextmenu', (e) => e.preventDefault());
"##;
