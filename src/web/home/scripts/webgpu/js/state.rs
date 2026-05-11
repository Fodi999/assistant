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
        objectRotation: [0.0, 0.0, 0.0],
        objectScale:    [1.0, 1.0, 1.0], // Массив scale X Y Z
        baseMeshDim:    [2.0, 2.0, 2.0], // Габариты формы в метрах
        objectBevel:    0.040,
        objectProfile:  1.0,
        objectRoundness: 0.0,
        selected:       false, // Отслеживаем выделен ли объект
        selectionMode:  0, // 0=Body, 1=Face, 2=Edge, 3=Vert, 4=Sketch
        engineMode:     'CAD', // 'PARTICLES' | 'CAD'
      };

      const sketchState = {
        points: [],   // { x, y, z }
        closed: false,
        plane: 'XZ',  // 'XZ' (y=0), 'XY' (z=0), 'YZ' (x=0)
        showGrid: true,
        // Tool state: for Rect/Circle 2-click workflow
        pendingTool: null,   // 'rectangle' | 'circle' | null
        pendingStart: null,  // {x,y,z} first corner / center
        // Snap visualization (filled by render_loop)
        hover: null,         // {x,y,z, snapType: 'grid'|'point'|'origin'|'first'}
      };

      // Extrude Preview state (frontend only, no backend yet)
      // Activated by Sketch Inspector "Preview Extrude" button
      window.extrudePreview = {
        active:    false,
        profileId: null,
        plane:     'XZ',
        direction: [0, 1, 0],  // unit vector
        distance:  1.0,
        points:    [],         // snapshot of base-profile points
      };

      let isPanning = false;

      // --- UI Settings Sync ---
      const planeSel = document.getElementById('sketch-plane-select');
      if (planeSel) {
        planeSel.addEventListener('change', e => {
          if (window.__setSketchPlane) window.__setSketchPlane(e.target.value);
          else sketchState.plane = e.target.value;
        });
      }
      const gridSnapSel = document.getElementById('sketch-grid-snap-select');
      if (gridSnapSel) {
        gridSnapSel.addEventListener('change', e => {
          window.sketchGridSnap = parseFloat(e.target.value);
          // refresh status overlay
          const elG = document.getElementById('sketch-info-grid');
          if (elG) {
            const snap = window.sketchGridSnap;
            elG.textContent = snap >= 1 ? snap+' m' : (snap*100).toFixed(snap<0.01?1:0)+' cm';
          }
        });
      }
      const gridToggle = document.getElementById('sketch-grid-toggle');
      if (gridToggle) {
        gridToggle.addEventListener('change', e => { sketchState.showGrid = e.target.checked; });
      }

      // Make snapStep configurable globally
      window.sketchGridSnap = 0.1;
      function snapToGrid(value) {
        const s = window.sketchGridSnap;
        if (!s || s <= 0) return value; // snap disabled
        return Math.round(value / s) * s;
      }

      // Per-formation presets — нижняя грань прилипает к полу
      const formationDefaults = {
        cube:  { objectPosition:[0.0, 0.0,  0.0], objectScale: [FORM_OBJSCALE.cube, FORM_OBJSCALE.cube, FORM_OBJSCALE.cube],  dist: 6.4, target:[0.0, 1.0,  0.0] },
        cloud: { objectPosition:[0.0, 0.0, 0.0], objectScale: [FORM_OBJSCALE.cloud, FORM_OBJSCALE.cloud, FORM_OBJSCALE.cloud], dist: 6.4, target:[0.0, 1.0, 0.0] },
        wall:  { objectPosition:[0.0, 0.0,  0.0], objectScale: [FORM_OBJSCALE.wall, FORM_OBJSCALE.wall, FORM_OBJSCALE.wall],  dist: 6.4, target:[0.0, 1.0,  0.0] },
      };

      // ── 4. Camera state ─────────────────────────────────────────
      const cam = {
        yaw:    Math.PI / 4,     // ~45 градусов вправо, как дефолтная камера Blender
        pitch:  Math.PI / 6,     // ~30 градусов вниз 
        dist:   6.4,             // Адаптированная дефолтная дистанция к 2.0m объекту
        target: [0.0, 1.0, 0.0],
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
          cam.dist = 6.4;
        } else if (count < 1000) {
          cam.dist = 8.0;
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

      // ─────────────────────────────────────────────────────────
      // Sketch plane raycaster — shared by pointer handlers & overlay
      // Returns {x,y,z, snapType} or null (no intersection).
      // ─────────────────────────────────────────────────────────
      window.__raycastSketchPlane = function(ndcX, ndcY, opts) {
        opts = opts || {};
        const asp = canvas.width / canvas.height;
        const cp = Math.cos(cam.pitch), sp = Math.sin(cam.pitch);
        const cy = Math.cos(cam.yaw),   sy = Math.sin(cam.yaw);
        const fwdX = -sy * cp, fwdY = -sp, fwdZ = cy * cp;
        const roX = cam.target[0] - fwdX * cam.dist;
        const roY = cam.target[1] - fwdY * cam.dist;
        const roZ = cam.target[2] - fwdZ * cam.dist;
        let rX = fwdY*0 - fwdZ*1, rY = fwdZ*0 - fwdX*0, rZ = fwdX*1 - fwdY*0;
        const rL = Math.hypot(rX,rY,rZ) || 1; rX/=rL; rY/=rL; rZ/=rL;
        const uX = rY*fwdZ - rZ*fwdY, uY = rZ*fwdX - rX*fwdZ, uZ = rX*fwdY - rY*fwdX;
        let oX = roX, oY = roY, oZ = roZ, dX, dY, dZ;
        if (cam.ortho) {
          const oh = cam.dist * 0.45;
          oX += rX*(ndcX*asp)*oh + uX*ndcY*oh;
          oY += rY*(ndcX*asp)*oh + uY*ndcY*oh;
          oZ += rZ*(ndcX*asp)*oh + uZ*ndcY*oh;
          const fL = Math.hypot(fwdX,fwdY,fwdZ);
          dX=fwdX/fL; dY=fwdY/fL; dZ=fwdZ/fL;
        } else {
          const fl = 2.414;
          const dx = fwdX*fl + rX*(ndcX*asp) + uX*ndcY;
          const dy = fwdY*fl + rY*(ndcX*asp) + uY*ndcY;
          const dz = fwdZ*fl + rZ*(ndcX*asp) + uZ*ndcY;
          const L = Math.hypot(dx,dy,dz);
          dX=dx/L; dY=dy/L; dZ=dz/L;
        }
        let t, hit = false, hx, hy, hz;
        const p = sketchState.plane;
        if (p === 'XZ' && Math.abs(dY) > 1e-6) {
          t = -oY/dY; if (t>0) { hx = oX+dX*t; hy = 0; hz = oZ+dZ*t; hit = true; }
        } else if (p === 'XY' && Math.abs(dZ) > 1e-6) {
          t = -oZ/dZ; if (t>0) { hx = oX+dX*t; hy = oY+dY*t; hz = 0; hit = true; }
        } else if (p === 'YZ' && Math.abs(dX) > 1e-6) {
          t = -oX/dX; if (t>0) { hx = 0; hy = oY+dY*t; hz = oZ+dZ*t; hit = true; }
        }
        if (!hit) return null;
        let snapType = 'free';
        // Snap to existing points first (priority over grid)
        const pickR = (cam.dist * 0.03);
        if (sketchState.points.length > 0) {
          for (let i = 0; i < sketchState.points.length; i++) {
            const pp = sketchState.points[i];
            if (Math.hypot(hx-pp.x, hy-pp.y, hz-pp.z) < pickR) {
              return { x: pp.x, y: pp.y, z: pp.z, snapType: (i===0 ? 'first' : 'point') };
            }
          }
        }
        // Snap to origin
        if (Math.hypot(hx,hy,hz) < pickR) {
          return { x: 0, y: 0, z: 0, snapType: 'origin' };
        }
        // Grid snap
        if (opts.skipGridSnap !== true) {
          hx = snapToGrid(hx); hy = snapToGrid(hy); hz = snapToGrid(hz);
          if (window.sketchGridSnap > 0) snapType = 'grid';
        }
        return { x: hx, y: hy, z: hz, snapType };
      };

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
        
        const dist = Math.hypot(e.clientX - startX, e.clientY - startY);
        // Увеличили порог клика для тачпадов mac, где пальцы скользят при клике
        if (dist < 15 && e.button === 0) { // Только левый клик
          if (sceneState.selectionMode === 4) {
            // Sketch Mode logic — only create points for line/rectangle/circle tools
            const tool = (window.editorState && window.editorState.activeSketchTool) || 'line';
            if (tool !== 'line' && tool !== 'rectangle' && tool !== 'circle') {
              return;
            }
            const rect = canvas.getBoundingClientRect();
            const ndcX = ((e.clientX - rect.left) / rect.width) * 2 - 1;
            const ndcY = 1 - ((e.clientY - rect.top) / rect.height) * 2;
            const hit = window.__raycastSketchPlane(ndcX, ndcY);
            if (!hit) return;
            const hx = hit.x, hy = hit.y, hz = hit.z;

            // ── LINE TOOL: chain of points, close by clicking first point ──
            if (tool === 'line') {
              if (sketchState.closed) {
                // Start a new sketch
                sketchState.points = [];
                sketchState.closed = false;
              }
              if (sketchState.points.length > 2 && hit.snapType === 'first') {
                sketchState.closed = true;
                log(`✓ Sketch closed (${sketchState.points.length} pts)`, '#10b981');
                if (window.__updateSketchUI) window.__updateSketchUI();
                return;
              }
              // Avoid duplicate last point
              const last = sketchState.points[sketchState.points.length-1];
              if (last && Math.hypot(hx-last.x, hy-last.y, hz-last.z) < 1e-4) return;
              sketchState.points.push({ x: hx, y: hy, z: hz });
              log(`+ Pt ${sketchState.points.length}: ${hx.toFixed(3)}, ${hy.toFixed(3)}, ${hz.toFixed(3)}`, '#38bdf8');
              if (window.__updateSketchUI) window.__updateSketchUI();
              return;
            }

            // ── RECTANGLE TOOL: 2 clicks ──
            if (tool === 'rectangle') {
              if (!sketchState.pendingStart) {
                sketchState.pendingStart = { x: hx, y: hy, z: hz };
                sketchState.pendingTool = 'rectangle';
                log(`◻ Rect corner 1: ${hx.toFixed(3)}, ${hz.toFixed(3)}`, '#38bdf8');
                return;
              }
              const s = sketchState.pendingStart;
              const pl = sketchState.plane;
              let pts;
              if (pl === 'XZ') {
                pts = [
                  { x: s.x, y: 0, z: s.z },
                  { x: hx,  y: 0, z: s.z },
                  { x: hx,  y: 0, z: hz  },
                  { x: s.x, y: 0, z: hz  },
                ];
              } else if (pl === 'XY') {
                pts = [
                  { x: s.x, y: s.y, z: 0 },
                  { x: hx,  y: s.y, z: 0 },
                  { x: hx,  y: hy,  z: 0 },
                  { x: s.x, y: hy,  z: 0 },
                ];
              } else { // YZ
                pts = [
                  { x: 0, y: s.y, z: s.z },
                  { x: 0, y: hy,  z: s.z },
                  { x: 0, y: hy,  z: hz  },
                  { x: 0, y: s.y, z: hz  },
                ];
              }
              sketchState.points = pts;
              sketchState.closed = true;
              sketchState.pendingStart = null;
              sketchState.pendingTool = null;
              log(`✓ Rectangle done`, '#10b981');
              if (window.__updateSketchUI) window.__updateSketchUI();
              return;
            }

            // ── CIRCLE TOOL: center + radius ──
            if (tool === 'circle') {
              if (!sketchState.pendingStart) {
                sketchState.pendingStart = { x: hx, y: hy, z: hz };
                sketchState.pendingTool = 'circle';
                log(`○ Circle center`, '#38bdf8');
                return;
              }
              const c = sketchState.pendingStart;
              const pl = sketchState.plane;
              let dxp, dyp;
              if (pl === 'XZ')      { dxp = hx - c.x; dyp = hz - c.z; }
              else if (pl === 'XY') { dxp = hx - c.x; dyp = hy - c.y; }
              else                  { dxp = hy - c.y; dyp = hz - c.z; }
              const r = Math.hypot(dxp, dyp);
              if (r < 1e-4) { sketchState.pendingStart = null; return; }
              const N = 32;
              const pts = [];
              for (let i = 0; i < N; i++) {
                const a = (i / N) * Math.PI * 2;
                const ca = Math.cos(a) * r, sa = Math.sin(a) * r;
                if (pl === 'XZ')      pts.push({ x: c.x+ca, y: 0,    z: c.z+sa });
                else if (pl === 'XY') pts.push({ x: c.x+ca, y: c.y+sa, z: 0 });
                else                  pts.push({ x: 0, y: c.y+ca, z: c.z+sa });
              }
              sketchState.points = pts;
              sketchState.closed = true;
              sketchState.pendingStart = null;
              sketchState.pendingTool = null;
              log(`✓ Circle r=${r.toFixed(3)}m`, '#10b981');
              if (window.__updateSketchUI) window.__updateSketchUI();
              return;
            }
          } else {
            sceneState.selected = !sceneState.selected;
            if (sceneState.selected) {
              cam.target = sceneState.objectPosition.slice();
              cam.target[1] += sceneState.baseMeshDim[1] / 2.0;
            }
            const msg = sceneState.selected ? 'Выделен (Фокус на объекте)' : 'Снято выделение';
            log(`◇ Объект: ${msg}`, sceneState.selected ? '#fbbf24' : '#9ca3af');
          }
        }
      });
      canvas.addEventListener('pointermove', (e) => {
        // NDC for sand cursor (in screen space, aspect-corrected later in shader)
        const rect = canvas.getBoundingClientRect();
        mouse.ndcX = ((e.clientX - rect.left) / rect.width)  * 2 - 1;
        mouse.ndcY = 1 - ((e.clientY - rect.top) / rect.height) * 2;
        mouse.active = true;

        // ── Track world coords of cursor on active sketch plane (status overlay + hover)
        if (sceneState.selectionMode === 4) {
          // Raw (unsnapped) world position for status bar
          const raw = window.__raycastSketchPlane(mouse.ndcX, mouse.ndcY, { skipGridSnap: true });
          if (raw) window.__sketchMouseWorld = { x: raw.x, y: raw.y, z: raw.z };
          // Snapped position with snapType for overlay rendering
          const snapped = window.__raycastSketchPlane(mouse.ndcX, mouse.ndcY);
          sketchState.hover = snapped;
        } else {
          sketchState.hover = null;
        }

        if (!dragging) return;
        
        // Prevent camera orbiting in Sketch mode with Left Click
        // But allow navigating with Right Click (e.buttons === 2) or Middle Click (e.buttons === 4)
        if (sceneState.selectionMode === 4 && !e.shiftKey && e.buttons === 1) return;
        
        const dx = e.clientX - lastX;
        const dy = e.clientY - lastY;
        lastX = e.clientX; lastY = e.clientY;

        if (panning) {
          // pan target on the camera plane
          const k = cam.dist * 0.0015;
          // Plane-aware pan: build camera right/up from yaw/pitch and translate target along them.
          // This works correctly for XY (front), XZ (top), YZ (right) sketch views.
          const cy = Math.cos(cam.yaw),   sy = Math.sin(cam.yaw);
          const cp = Math.cos(cam.pitch), sp = Math.sin(cam.pitch);
          const fwdX = -sy * cp, fwdY = -sp, fwdZ = cy * cp;
          let rxv = fwdY * 0 - fwdZ * 1;
          let ryv = fwdZ * 0 - fwdX * 0;
          let rzv = fwdX * 1 - fwdY * 0;
          const rL = Math.hypot(rxv, ryv, rzv) || 1;
          rxv /= rL; ryv /= rL; rzv /= rL;
          const uxv = ryv * fwdZ - rzv * fwdY;
          const uyv = rzv * fwdX - rxv * fwdZ;
          const uzv = rxv * fwdY - ryv * fwdX;
          cam.target[0] -= (dx * rxv - dy * uxv) * k;
          cam.target[1] -= (dx * ryv - dy * uyv) * k;
          cam.target[2] -= (dx * rzv - dy * uzv) * k;
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
        // ── Sketch Mode shortcuts ──
        if (sceneState.selectionMode === 4) {
          const k = e.key.toLowerCase();
          if (k === 'escape') {
            // Cancel current chain / pending tool
            if (sketchState.pendingStart) {
              sketchState.pendingStart = null;
              sketchState.pendingTool  = null;
              log('✕ Tool cancelled', '#fbbf24');
            } else if (sketchState.points.length > 0 && !sketchState.closed) {
              sketchState.points = [];
              log('✕ Sketch cleared', '#fbbf24');
            }
            if (window.__updateSketchUI) window.__updateSketchUI();
            return;
          }
          if (k === 'enter' && sketchState.points.length > 2 && !sketchState.closed) {
            sketchState.closed = true;
            log(`✓ Sketch closed (Enter)`, '#10b981');
            if (window.__updateSketchUI) window.__updateSketchUI();
            return;
          }
          if (k === 'backspace' && sketchState.points.length > 0 && !sketchState.closed) {
            sketchState.points.pop();
            log(`↶ Removed last point`, '#fbbf24');
            if (window.__updateSketchUI) window.__updateSketchUI();
            return;
          }
          if (e.target && (e.target.tagName === 'INPUT' || e.target.tagName === 'SELECT' || e.target.tagName === 'TEXTAREA')) return;
          if (k === 'l' && window.__setSketchTool) { window.__setSketchTool('line'); return; }
          if (k === 'r' && window.__setSketchTool) { window.__setSketchTool('rectangle'); return; }
          if (k === 'o' && window.__setSketchTool) { window.__setSketchTool('circle'); return; }
          if (k === 'd' && window.__setSketchTool) { window.__setSketchTool('dimension'); return; }
        }

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
          case 'f':          sceneState.objectPosition[1] = sceneState.objectScale[1] * FORM_SCALE[formation.mode]; break;
          case 'c':          
            cam.target = sceneState.objectPosition.slice();
            cam.target[1] += sceneState.baseMeshDim[1] / 2.0;
            log('◇ camera focused on object', '#67e8f9');
            break;
        }
      });
"##;
