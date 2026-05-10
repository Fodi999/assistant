// ── JS: application state — particles, camera, shape, formation, input ───────────
// Domain: Application state — all runtime state objects and pointer event wiring.

pub const JS: &str = r##"
      // Each particle = 8 floats = 32 bytes.
      // Cap MAX_PARTICLES at whatever the GPU storage-buffer limit allows.
      const PARTICLE_STRIDE = 32;
      const HARD_CAP        = 5_000_000;
      const deviceCap       = Math.floor(device.limits.maxStorageBufferBindingSize / PARTICLE_STRIDE);
      const MAX_PARTICLES   = Math.min(HARD_CAP, deviceCap);
      // Start with 1 particle (the "default cube"), max up to 1M
      let   NUM_SPHERES     = 1;
      const CLOUD_VOLUME    = (4 / 3) * Math.PI * Math.pow(5.5, 3);
      log(`✓ MAX_PARTICLES = ${(MAX_PARTICLES/1e6).toFixed(2)}M  (buffer ${(MAX_PARTICLES*32/1048576).toFixed(0)} MB)`, '#a78bfa');

      function buildParticles(count) {
        const data = new Float32Array(count * 8);
        for (let i = 0; i < count; i++) {
          const b = i * 8;
          data[b + 0] = 0.0;
          data[b + 1] = 0.0;
          data[b + 2] = 0.0;
          data[b + 3] = 0.5; // radius is half cell
          // Grey color for the single square/cube
          data[b + 4] = 0.8;
          data[b + 5] = 0.8;
          data[b + 6] = 0.8;
          data[b + 7] = 0.0;
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
        objectPosition: [0.0, 0.0, 0.0],
        objectScale:    FORM_OBJSCALE.cube,
        selected:       false, // Отслеживаем выделен ли объект
      };

      // Per-formation presets — нижняя грань прилипает к полу
      const formationDefaults = {
        cube:  { objectPosition:[0.0, 0.0,  0.0], objectScale: FORM_OBJSCALE.cube,  dist: 3.25, target:[0.0, 0.0,  0.0] },
        cloud: { objectPosition:[0.0, 0.0, 0.0], objectScale: FORM_OBJSCALE.cloud, dist: 3.75, target:[0.0, 0.0, 0.0] },
        wall:  { objectPosition:[0.0, 0.0,  0.0], objectScale: FORM_OBJSCALE.wall,  dist: 3.5, target:[0.0, 0.0,  0.0] },
      };

      // ── 4. Camera state ─────────────────────────────────────────
      const cam = {
        yaw:    Math.PI / 4,     // ~45 градусов вправо, как дефолтная камера Blender
        pitch:  Math.PI / 6,     // ~30 градусов вниз 
        dist:   8.0,             // Дефолтная удобная дистанция к дефолтному кубу
        target: [0.0, 0.0, 0.0],
        autoRotate: false,
        ortho: false,
      };

      window.setCameraPreset = function(preset) {
        switch(preset) {
          case 'front': cam.yaw = 0.0; cam.pitch = 0.0; break;
          case 'right': cam.yaw = Math.PI * 0.5; cam.pitch = 0.0; break;
          case 'top':   cam.yaw = 0.0; cam.pitch = -Math.PI * 0.5 + 0.001; break;
          case 'iso':   cam.yaw = 0.785; cam.pitch = -0.615; break;
        }
        log(`◇ camera preset: ${preset}`, '#67e8f9');
      };
      
      window.setCameraProjection = function(mode) {
        cam.ortho = (mode === 'ortho');
        log(`◇ projection: ${mode}`, '#67e8f9');
      };
      // shape parameter: 0 = super-cube · 0.5 = octahedron (triangle silhouette) · 1 = super-sphere
      const shape = { roundness: 0.0 }; // Force grid square
      // Piecewise map slider [0..1] → superquadric exponent n  (|x|ⁿ+|y|ⁿ+|z|ⁿ=1):
      //   r=0.0 → n=22  cube       (sharp planar faces)
      //   r=0.5 → n=1   octahedron (diamond: triangular silhouette, exact L1 SDF)
      //   r=1.0 → n=2   sphere     (perfectly round)
      // Lower half r∈[0,0.5] sweeps cube → octahedron (n: 22 → 1).
      // Upper half r∈[0.5,1] sweeps octahedron → sphere (n: 1 → 2).
      function shapeExponent(r) {
        return 22.0; // always cube
      }

      function updateCameraForCount(count) {
        if (count < 10) {
          cam.dist = 2.0;
        } else if (count < 1000) {
          cam.dist = 2.5;
        } else {
          cam.dist = formationDefaults['cube'].dist;
        }
      }

      // ── Formation state ────────────────────────────────────────
      const formation = { mode: 'cube', target: 1.0, mix: 1.0 };
      function setFormation(mode) {
        formation.mode   = mode;
        formation.target = 1.0;
        // re-frame scene per formation
        const d = formationDefaults[mode] || formationDefaults.cube;
        if (d) {
          sceneState.objectPosition = d.objectPosition.slice();
          sceneState.objectScale    = d.objectScale;
          cam.target = d.target.slice();
          updateCameraForCount(NUM_SPHERES);
        }
        log(`◇ formation = ${mode}`, '#f0abfc');
      }

      // ── Cell-SDF (kernel::particle_shape port) ────────────────
      // on        : when true, cube formation uses per-cell SDF instead of
      //             billboard imposters → flush seams + rounded outer hull.
      // radius    : 0..0.5 corner radius (cell-local units).
      // colorMode : 0 normal · 1 normals-as-RGB · 2 colour-by-SlotKind.
      // hideLow   : true → cull cells with ≤ 1 exposed face (show only edges/corners).
      const cellSdf = { on: true, radius: 0.05, colorMode: 0, hideLow: false };
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
      let dragging = false, panning = false, lastX = 0, lastY = 0, startX = 0, startY = 0;
      canvas.addEventListener('pointerdown', (e) => {
        canvas.setPointerCapture(e.pointerId);
        dragging = true;
        // Pan if Shift is pressed, or Middle Mouse Button + Shift, or Right Click
        panning  = e.shiftKey || e.button === 2 || (e.button === 1 && e.shiftKey);
        lastX = e.clientX; lastY = e.clientY;
        startX = e.clientX; startY = e.clientY;
      });
      
      canvas.addEventListener('pointerup', (e) => {
        dragging = false; panning = false;
        try { canvas.releasePointerCapture(e.pointerId); } catch {}
        
        // --- ПРОСТОЙ ВЫДЕЛИТЕЛЬ (КЛИК ДЛЯ ВЫДЕЛЕНИЯ) ---
        // Если курсор не сдвигался во время нажатия (< 5 пикселей), считаем это кликом.
        const dist = Math.hypot(e.clientX - startX, e.clientY - startY);
        if (dist < 5 && e.button === 0) { // Только левый клик
          sceneState.selected = !sceneState.selected;
          
          // Тот самый трюк для удобного зума: если объект выделен, смещаем фокус камеры точно на него!
          if (sceneState.selected) {
            cam.target = sceneState.objectPosition.slice(); 
          }
          
          const msg = sceneState.selected ? 'Выделен (Фокус на объекте)' : 'Снято выделение';
          log(`◇ Объект: ${msg}`, sceneState.selected ? '#fbbf24' : '#9ca3af');
        }
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
          // Orbit (Blender style)
          cam.yaw   += dx * 0.005;
          cam.pitch += dy * 0.005;
        }
      });
      canvas.addEventListener('pointerleave', () => { mouse.active = false; });
      
      // --- Поддержка нативного Pinch-to-Zoom для Safari (Mac/iOS) ---
      let startPinchDist = 0;
      canvas.addEventListener('gesturestart', (e) => {
        e.preventDefault();
        startPinchDist = cam.dist;
      });
      canvas.addEventListener('gesturechange', (e) => {
        e.preventDefault();
        let minZ = 0.5;
        if (state.formMix > 0.8) minZ = Math.max(0.5, 2.5 * state.objScale);
        
        // e.scale > 1 = раздвигаем пальцы (Zoom In), e.scale < 1 = сдвигаем (Zoom Out)
        // В Three.js и Blender мы просто делим начальную дистанцию на масштаб
        cam.dist = Math.max(minZ, Math.min(80, startPinchDist / e.scale));
      });
      canvas.addEventListener('gestureend', (e) => e.preventDefault());

      canvas.addEventListener('wheel', (e) => {
        e.preventDefault();
        
        // В Blender можно приближаться вплотную (макро-съемка). 
        // Мы ставим минимальную дистанцию в 0.05 (почти в упор), 
        // а логарифмический масштаб ниже сам замедлит зум при приближении.
        const minZ = 0.05;

        // --- ПРОСТОЙ И НАДЕЖНЫЙ ЗУМ (КАК В BLENDER) ---
        let speed = e.ctrlKey ? 0.01 : 0.002;
        const factor = Math.exp(e.deltaY * speed);
        
        // Чем меньше cam.dist, тем меньше шаг зума — идеальная эмуляция Blender
        cam.dist = Math.max(minZ, Math.min(200, cam.dist * factor));
      }, { passive: false });
      canvas.addEventListener('contextmenu', (e) => e.preventDefault());

      // Keyboard move tool
      document.addEventListener('keydown', (e) => {
        // Move object along X,Y,Z axes: ←/→ = X, ↑/↓ = Z, Q/E = Y
        // F = rest to floor, C = focus camera
        let moved = false;
        const step = 0.05 * (floorGrid.scale === 1000.0 ? 0.01 : floorGrid.scale === 100.0 ? 0.1 : 1.0);
        
        switch (e.key.toLowerCase()) {
          case 'arrowleft':  sceneState.objectPosition[0] -= step; break;
          case 'arrowright': sceneState.objectPosition[0] += step; break;
          case 'arrowup':    sceneState.objectPosition[2] -= step; break;
          case 'arrowdown':  sceneState.objectPosition[2] += step; break;
          case 'q':          sceneState.objectPosition[1] += step; break;
          case 'e':          sceneState.objectPosition[1] -= Math.min(sceneState.objectPosition[1], step); break;
          case 'f':          sceneState.objectPosition[1] = sceneState.objectScale * FORM_SCALE[formation.mode]; break;
          case 'c':          
            cam.target = sceneState.objectPosition.slice();
            log('◇ camera focused on object', '#67e8f9');
            break;
        }
      });
"##;
