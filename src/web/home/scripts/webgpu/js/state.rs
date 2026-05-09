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
        // shrink billboard size as count grows so cloud doesn't become a wall of light
        // Use min scale clamp so 1 particle doesn't blow up to massive size.
        const effectiveCount = Math.max(count, 5000); 
        const sizeScale = Math.pow(50000 / effectiveCount, 0.42); 
        for (let i = 0; i < count; i++) {
          const b = i * 8;
          // sphere-distributed cloud (denser core, soft falloff)
          // single-particle case: place exactly at origin
          let r = 0, th = 0, ph = 0;
          if (count > 1) {
            r   = Math.pow(Math.random(), 0.55) * 5.5;
            th  = Math.random() * 6.2832;
            ph  = Math.acos(2 * Math.random() - 1);
          }
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
        yaw:    0.785,
        pitch:  -0.615,
        dist:   3.25,
        target: [0.0, halfHeight('cube'), 0.0],
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

      function updateCameraForCount(count) {
        if (count < 10) {
          cam.dist = 2.0;
        } else if (count < 1000) {
          cam.dist = 2.5;
        } else {
          cam.dist = formationDefaults[formation.mode].dist;
        }
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
          cam.target = d.target.slice();
          updateCameraForCount(NUM_SPHERES);
        }
        log(`◇ formation = ${mode}`, '#f0abfc');
        
        // Ensure UI stays updated if exists
        const objPosEl = document.getElementById('ui-obj-pos');
        if (objPosEl) objPosEl.innerText = `X ${sceneState.objectPosition[0].toFixed(2)} / Y ${sceneState.objectPosition[1].toFixed(2)} / Z ${sceneState.objectPosition[2].toFixed(2)}`;
      }

      // ── Cell-SDF (kernel::particle_shape port) ────────────────
      // on        : when true, cube formation uses per-cell SDF instead of
      //             billboard imposters → flush seams + rounded outer hull.
      // radius    : 0..0.5 corner radius (cell-local units).
      // colorMode : 0 normal · 1 normals-as-RGB · 2 colour-by-SlotKind.
      // hideLow   : true → cull cells with ≤ 1 exposed face (show only edges/corners).
      const cellSdf = { on: true, radius: 0.05, colorMode: 0, hideLow: false };
      const floorGrid = { scale: 1.0 }; // 1.0 = m, 100.0 = cm, 1000.0 = mm

        // ── Cube Grid State ──────────────────────────────────────────
        const cubeGridState = {
          side: 1,
          cellSizeMm: 100,
          cellSizeWorld: 0.1,
          selectedCellId: null,
        };

        function cellId(ix, iy, iz, side) {
          return ix + iy * side + iz * side * side;
        }

        function cellCoord(id, side) {
          const ix = id % side;
          const iy = Math.floor(id / side) % side;
          const iz = Math.floor(id / (side * side));
          return { ix, iy, iz };
        }

        window.updateCubeGrid = function updateCubeGrid(newSide) {
          cubeGridState.side = newSide;
          cubeGridState.selectedCellId = null; // reset selection on grid change
          
          // 1. Math calculation for elements
          const side = cubeGridState.side;
          const totalCells = side * side * side;
          const surface = side <= 1 ? 1 : totalCells - Math.pow(side - 2, 3);
          const interior = Math.max(0, totalCells - surface);
          const objSize = side * cubeGridState.cellSizeMm;
          
          // 2. Set Engine State
          NUM_SPHERES = totalCells;
          
          // Ensure object position Y sits perfectly on the floor (Y = totalSize/2)
          // Convert to the scaling required by the engine math:
          const worldObjSize = (objSize / 1000.0) * floorGrid.scale; 
          sceneState.objectPosition[1] = worldObjSize / 2.0;

          // Update DOM UI elements if they exist
          const vSide = document.getElementById('ui-cube-side');
          const vCell = document.getElementById('ui-cube-cell-size');
          const vObj  = document.getElementById('ui-cube-obj-size');
          const vSurf = document.getElementById('surfaceValue');
          const vInt  = document.getElementById('interiorValue');

          if (vSide) vSide.innerText = side;
          if (vCell) vCell.innerText = cubeGridState.cellSizeMm + ' mm';
          if (vObj)  vObj.innerText = objSize + ' mm';
          if (vSurf) vSurf.innerText = String(surface);
          if (vInt)  vInt.innerText = String(interior);

          // Update Inspector UI reset
          updateInspectorUi();

          // Rebuild particle buffer memory rigidly ordered for Cube Grid
          sphereData = new Float32Array(totalCells * 8);
          // Scale down particle sphere sizes if they get huge or scale evenly based on cell size
          // The visual size relies on the objectScale and the local coordinate.
          // Cell size world dictates the local spacing:
          const cellSizeLocal = 1.0 / side; // Map it into local space range [-0.5, 0.5] if shape uses it, but we can just use 1.0/side spacing.
          const halfSide = (side - 1) * 0.5;

          for (let id = 0; id < totalCells; id++) {
            const { ix, iy, iz } = cellCoord(id, side);
            
            // X, Y, Z centered around 0,0,0
            const cx = (ix - halfSide) * cellSizeLocal;
            const cy = (iy - halfSide) * cellSizeLocal;
            const cz = (iz - halfSide) * cellSizeLocal;

            const b = id * 8;
            sphereData[b + 0] = cx;
            sphereData[b + 1] = cy;
            sphereData[b + 2] = cz;
            sphereData[b + 3] = cellSizeLocal * 0.5; // radius is half cell

            // Give alternating checkboard or random colors
            const isSurface = (ix === 0 || ix === side - 1 || iy === 0 || iy === side - 1 || iz === 0 || iz === side - 1);
            if (isSurface) {
              sphereData[b + 4] = 0.8;
              sphereData[b + 5] = 0.8;
              sphereData[b + 6] = 0.8;
            } else {
              sphereData[b + 4] = 0.3;
              sphereData[b + 5] = 0.3;
              sphereData[b + 6] = 0.4;
            }
            sphereData[b + 7] = 0.0;
          }

          if (typeof device !== 'undefined' && typeof sphereBuf !== 'undefined') {
            device.queue.writeBuffer(sphereBuf, 0, sphereData);
          }
          
          log(`◇ Cube Grid: ${side}³ (surface: ${surface}, interior: ${interior})`, '#a78bfa');
        }

        window.selectCell = function(id) {
          if (id === null || id < 0 || id >= NUM_SPHERES) {
            cubeGridState.selectedCellId = null;
          } else {
            cubeGridState.selectedCellId = id;
          }
          updateInspectorUi();
        }

        function updateInspectorUi() {
          const uiId = document.getElementById('ui-sel-id');
          const uiGrid = document.getElementById('ui-sel-grid-coords');
          const uiWorld = document.getElementById('ui-sel-world-coords');
          const uiSize = document.getElementById('ui-sel-size');

          if (!uiId || !uiGrid || !uiWorld || !uiSize) return;

          const id = cubeGridState.selectedCellId;
          const side = cubeGridState.side;

          if (id === null) {
            uiId.innerText = '—';
            uiGrid.innerText = '—';
            uiWorld.innerText = '—';
            uiSize.innerText = '—';
            return;
          }

          const { ix, iy, iz } = cellCoord(id, side);
          uiId.innerText = id;
          uiGrid.innerText = `X ${ix + 1} / Y ${iy + 1} / Z ${iz + 1}`;
          
          // Calculate world position
          const size = cubeGridState.cellSizeWorld;
          const halfSide = (side - 1) * 0.5;

          const lx = (ix - halfSide) * size;
          const ly = (iy - halfSide) * size;
          const lz = (iz - halfSide) * size;
          
          const op = sceneState.objectPosition;
          
          const wx = op[0] + lx;
          const wy = op[1] + ly;
          const wz = op[2] + lz;

          uiWorld.innerText = `X ${wx.toFixed(2)} / Y ${wy.toFixed(2)} / Z ${wz.toFixed(2)}`;
          uiSize.innerText = `${cubeGridState.cellSizeMm} mm`;
        }      function toggleCellSdf() {
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
        
        // Dynamically compute absolute minimum zoom so we don't pierce inside solid shapes.
        // formScale = formation scale block size (e.g. wall or cube side length).
        // We use roughly half that plus a tiny margin to stay outside the model.
        let minZ = 0.5;
        if (state.formMix > 0.8) {
          // If formed, use the form scale. Object scale is state.formScale * state.objScale
          // So the radius of the megashape bounds is roughly formScale * sqrt(3)/2 * objScale
          // For a single particle mesh (formScale == 1.0 or single cube), the size is u9.x (cellSize)
          // We just make sure we are not physically closer than 2.0 * objScale to be super safe. 
          minZ = Math.max(0.5, 2.5 * state.objScale);
        }

        cam.dist = Math.max(minZ, Math.min(80, cam.dist * factor));
      }, { passive: false });
      canvas.addEventListener('contextmenu', (e) => e.preventDefault());

      // Keyboard move tool
      document.addEventListener('keydown', (e) => {
        // Move object along X,Y,Z axes: ←/→ = X, ↑/↓ = Z, Q/E = Y
        // F = rest to floor, C = focus camera
        let moved = false;
        const step = 0.05 * (floorGrid.scale === 1000.0 ? 0.01 : floorGrid.scale === 100.0 ? 0.1 : 1.0);
        
        switch (e.key.toLowerCase()) {
          case 'arrowleft':  sceneState.objectPosition[0] -= step; moved = true; break;
          case 'arrowright': sceneState.objectPosition[0] += step; moved = true; break;
          case 'arrowup':    sceneState.objectPosition[2] -= step; moved = true; break;
          case 'arrowdown':  sceneState.objectPosition[2] += step; moved = true; break;
          case 'q':          sceneState.objectPosition[1] += step; moved = true; break;
          case 'e':          sceneState.objectPosition[1] -= Math.min(sceneState.objectPosition[1], step); moved = true; break;
          case 'f':          sceneState.objectPosition[1] = sceneState.objectScale * FORM_SCALE[formation.mode]; moved = true; break;
          case 'c':          
            cam.target = sceneState.objectPosition.slice();
            log('◇ camera focused on object', '#67e8f9');
            break;
        }

        if (moved) {
          // Sync with DOM easily
          const objPosEl = document.getElementById('ui-obj-pos');
          if (objPosEl) objPosEl.innerText = `X ${sceneState.objectPosition[0].toFixed(2)} / Y ${sceneState.objectPosition[1].toFixed(2)} / Z ${sceneState.objectPosition[2].toFixed(2)}`;
        }
      });
"##;
