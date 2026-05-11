// ── JS: Application state — particles, camera, scene, formation, input dispatcher ──
// Domain: Application state — runtime state objects and pointer/keyboard event wiring.
// Sketch logic is delegated to: sketch_state.rs · sketch_pick.rs · sketch_tools.rs

pub const JS: &str = r##"
      // ── Particle buffer constants ──────────────────────────────
      const PARTICLE_STRIDE = 32;
      const HARD_CAP        = 5_000_000;
      const deviceCap       = Math.floor(device.limits.maxStorageBufferBindingSize / PARTICLE_STRIDE);
      const MAX_PARTICLES   = Math.min(HARD_CAP, deviceCap);

      let   NUM_SPHERES  = 1;
      const CLOUD_VOLUME = (4 / 3) * Math.PI * Math.pow(5.5, 3);
      log(`✓ MAX_PARTICLES = ${(MAX_PARTICLES/1e6).toFixed(2)}M  (buffer ${(MAX_PARTICLES*32/1048576).toFixed(0)} MB)`, '#a78bfa');

      function buildParticles(count) {
        const data = new Float32Array(count * 8);
        for (let i = 0; i < count; i++) {
          const b = i * 8;
          data[b+0] = 0.0; data[b+1] = 0.0; data[b+2] = 0.0;
          data[b+3] = 0.5;
          data[b+4] = 0.8; data[b+5] = 0.8; data[b+6] = 0.8;
          data[b+7] = 0.0;
        }
        return data;
      }
      let sphereData = buildParticles(NUM_SPHERES);

      const FORM_SCALE    = { cube: 1.8, cloud: 1.5, wall: 1.6 };
      const FORM_OBJSCALE = { cube: 0.18, cloud: 0.30, wall: 0.22 };
      function halfHeight(mode) { return FORM_SCALE[mode] * FORM_OBJSCALE[mode]; }

      // ── Scene state ────────────────────────────────────────────
      const sceneState = {
        objectPosition: [0.0, 0.0, 0.0],
        objectRotation: [0.0, 0.0, 0.0],
        objectScale:    [1.0, 1.0, 1.0],
        baseMeshDim:    [2.0, 2.0, 2.0],
        objectBevel:    0.040,
        objectProfile:  1.0,
        objectRoundness: 0.0,
        selected:       false,
        selectionMode:  0,
        engineMode:     'CAD',
      };

      // ── Camera ─────────────────────────────────────────────────
      const cam = {
        yaw:    Math.PI / 4,
        pitch:  Math.PI / 6,
        dist:   6.4,
        target: [0.0, 1.0, 0.0],
        autoRotate: false,
        ortho: false,
      };

      window.setCameraPreset = function(preset) {
        switch (preset) {
          case 'front': cam.yaw = 0.0;         cam.pitch = 0.0;                  break;
          case 'right': cam.yaw = Math.PI*0.5; cam.pitch = 0.0;                  break;
          case 'top':   cam.yaw = 0.0;         cam.pitch = -Math.PI*0.5 + 0.001; break;
          case 'iso':   cam.yaw = 0.785;       cam.pitch = -0.615;               break;
        }
        log(`◇ camera preset: ${preset}`, '#67e8f9');
      };

      window.setCameraProjection = function(mode) {
        cam.ortho = (mode === 'ortho');
        log(`◇ projection: ${mode}`, '#67e8f9');
      };

      const shape = { roundness: 0.0 };
      function shapeExponent(r) { return 22.0; }

      function updateCameraForCount(count) {
        cam.dist = count < 10 ? 6.4 : count < 1000 ? 8.0 : formationDefaults['cube'].dist;
      }

      // ── Formation ──────────────────────────────────────────────
      const formationDefaults = {
        cube:  { objectPosition:[0.0,0.0,0.0], objectScale:[FORM_OBJSCALE.cube,  FORM_OBJSCALE.cube,  FORM_OBJSCALE.cube],  dist:6.4, target:[0.0,1.0,0.0] },
        cloud: { objectPosition:[0.0,0.0,0.0], objectScale:[FORM_OBJSCALE.cloud, FORM_OBJSCALE.cloud, FORM_OBJSCALE.cloud], dist:6.4, target:[0.0,1.0,0.0] },
        wall:  { objectPosition:[0.0,0.0,0.0], objectScale:[FORM_OBJSCALE.wall,  FORM_OBJSCALE.wall,  FORM_OBJSCALE.wall],  dist:6.4, target:[0.0,1.0,0.0] },
      };
      const formation = { mode: 'cube', target: 1.0, mix: 1.0 };

      function setFormation(mode) {
        formation.mode   = mode;
        formation.target = 1.0;
        const d = formationDefaults[mode] || formationDefaults.cube;
        if (d) {
          sceneState.objectPosition = d.objectPosition.slice();
          sceneState.objectScale    = d.objectScale;
          cam.target = d.target.slice();
          updateCameraForCount(NUM_SPHERES);
        }
        log(`◇ formation = ${mode}`, '#f0abfc');
      }

      // ── Cell-SDF / Floor grid ──────────────────────────────────
      const cellSdf   = { on: true, radius: 0.05, colorMode: 0, hideLow: false };
      const floorGrid = { scale: 1.0 };

      function toggleCellSdf()  { cellSdf.on = !cellSdf.on; log(`◇ cell-sdf = ${cellSdf.on ? 'ON' : 'off'}`, '#67e8f9'); }
      function cycleColorMode() { cellSdf.colorMode = (cellSdf.colorMode+1)%3; log(`◇ debug color = ${['normal','normals-RGB','mask-color'][cellSdf.colorMode]}`, '#fbbf24'); }

      const mouse = { ndcX: 999, ndcY: 999, active: false };

      // ══════════════════════════════════════════════════════════
      // POINTER EVENTS  (sketch logic → __handleSketchClick / __handleSketchKey)
      // ══════════════════════════════════════════════════════════
      let dragging = false, panning = false, lastX = 0, lastY = 0, startX = 0, startY = 0;

      canvas.addEventListener('pointerdown', (e) => {
        canvas.setPointerCapture(e.pointerId);
        dragging = true;
        panning  = e.shiftKey || e.button === 2 || (e.button === 1 && e.shiftKey);
        lastX = e.clientX; lastY = e.clientY;
        startX = e.clientX; startY = e.clientY;

        if (e.button === 0 && sceneState.selectionMode === 4) {
          const rect = canvas.getBoundingClientRect();
          const ndcX = ((e.clientX - rect.left) / rect.width) * 2 - 1;
          const ndcY = 1 - ((e.clientY - rect.top) / rect.height) * 2;
          const hit  = window.__raycastSketchPlane(ndcX, ndcY, { skipGridSnap: true });
          if (hit) {
            const pickR = Math.max(0.5, cam.dist * 0.05);
            for (let i = 0; i < sketchState.points.length; i++) {
              const pp = sketchState.points[i];
              if (Math.hypot(hit.x-pp.x, hit.y-pp.y, hit.z-pp.z) < pickR) {
                sketchState.draggedPtIndex = i;
                startX = -9999;
                log(`[sketch] ⤢ Grabbed point ${i}`, '#a78bfa');
                break;
              }
            }
          }
        }
      });

      canvas.addEventListener('pointerup', (e) => {
        dragging = false; panning = false;
        try { canvas.releasePointerCapture(e.pointerId); } catch {}

        if (sketchState.draggedPtIndex !== undefined) {
          sketchState.draggedPtIndex = undefined;
          startX = 0; startY = 0;
          return;
        }
        if (startX < -100) { startX = 0; startY = 0; return; }

        const dist = (sceneState.selectionMode === 4 && startX !== -9999)
          ? 0
          : Math.hypot(e.clientX - startX, e.clientY - startY);
        startX = 0; startY = 0;

        log(`[click] button=${e.button} dist=${dist.toFixed(1)} selMode=${sceneState.selectionMode}`, '#6b7280');

        if (dist < 15 && e.button === 0) {
          const rect = canvas.getBoundingClientRect();
          const ndcX = ((e.clientX - rect.left) / rect.width) * 2 - 1;
          const ndcY = 1 - ((e.clientY - rect.top) / rect.height) * 2;

          if (sceneState.selectionMode === 4) {
            if (window.__handleSketchClick) window.__handleSketchClick(ndcX, ndcY);
          } else {
            const hit = window.__raycastCadSolids && window.__raycastCadSolids(ndcX, ndcY);
            if (hit) {
              window.selectedFaceId  = hit.faceId;
              window.selectedSolidId = hit.solid.id;
              sceneState.selected = true;
              log(`◇ Pick: solid=${hit.solid.id} faceId=${hit.faceId} t=${hit.t.toFixed(3)}`, '#fbbf24');
            } else {
              window.selectedFaceId  = 0;
              window.selectedSolidId = null;
              sceneState.selected = !sceneState.selected;
              if (sceneState.selected) {
                cam.target = sceneState.objectPosition.slice();
                cam.target[1] += sceneState.baseMeshDim[1] / 2.0;
              }
              const msg = sceneState.selected ? 'Выделен (Фокус на объекте)' : 'Снято выделение';
              log(`◇ Объект: ${msg}`, sceneState.selected ? '#fbbf24' : '#9ca3af');
            }
          }
        }
      });

      canvas.addEventListener('pointermove', (e) => {
        const rect = canvas.getBoundingClientRect();
        mouse.ndcX = ((e.clientX - rect.left) / rect.width) * 2 - 1;
        mouse.ndcY = 1 - ((e.clientY - rect.top) / rect.height) * 2;
        mouse.active = true;

        if (sceneState.selectionMode === 4) {
          const raw = window.__raycastSketchPlane(mouse.ndcX, mouse.ndcY, { skipGridSnap: true, _isHover: true });
          if (raw) window.__sketchMouseWorld = { x: raw.x, y: raw.y, z: raw.z };
          sketchState.hover = window.__raycastSketchPlane(mouse.ndcX, mouse.ndcY, { _isHover: true, ignoreIndex: sketchState.draggedPtIndex });
        } else {
          sketchState.hover = null;
          if (!dragging && window.__raycastCadSolids && (window.solids || []).length) {
            const hh = window.__raycastCadSolids(mouse.ndcX, mouse.ndcY);
            window.hoveredFaceId = hh ? hh.faceId : 0;
          } else {
            window.hoveredFaceId = 0;
          }
        }

        if (!dragging) return;

        if (sketchState.draggedPtIndex !== undefined && sketchState.hover) {
          sketchState.points[sketchState.draggedPtIndex] = {
            x: sketchState.hover.x, y: sketchState.hover.y, z: sketchState.hover.z,
          };
          if (window.extrudePreview && window.extrudePreview.active)
            window.extrudePreview.points = sketchState.points.map(p => ({ ...p }));
          if (window.__updateSketchUI) window.__updateSketchUI();
          return;
        }

        if (sceneState.selectionMode === 4 && !e.shiftKey && e.buttons === 1) return;

        const dx = e.clientX - lastX;
        const dy = e.clientY - lastY;
        lastX = e.clientX; lastY = e.clientY;

        if (panning) {
          const k   = cam.dist * 0.0015;
          const cy  = Math.cos(cam.yaw), sy = Math.sin(cam.yaw);
          const cp  = Math.cos(cam.pitch);
          const fwdX = -sy*cp, fwdY = -Math.sin(cam.pitch), fwdZ = cy*cp;
          let rxv = -fwdZ, ryv = 0, rzv = fwdX;
          const rL = Math.hypot(rxv, ryv, rzv) || 1; rxv /= rL; rzv /= rL;
          const uxv = ryv*fwdZ - rzv*fwdY;
          const uyv = rzv*fwdX - rxv*fwdZ;
          const uzv = rxv*fwdY - ryv*fwdX;
          cam.target[0] -= (dx*rxv - dy*uxv) * k;
          cam.target[1] -= (dx*ryv - dy*uyv) * k;
          cam.target[2] -= (dx*rzv - dy*uzv) * k;
        } else {
          cam.yaw   += dx * 0.005;
          cam.pitch += dy * 0.005;
        }
      });

      canvas.addEventListener('pointerleave', () => { mouse.active = false; });

      let startPinchDist = 0;
      canvas.addEventListener('gesturestart',  (e) => { e.preventDefault(); startPinchDist = cam.dist; });
      canvas.addEventListener('gesturechange', (e) => { e.preventDefault(); cam.dist = Math.max(0.5, Math.min(80, startPinchDist / e.scale)); });
      canvas.addEventListener('gestureend',    (e) => e.preventDefault());

      canvas.addEventListener('wheel', (e) => {
        e.preventDefault();
        cam.dist = Math.max(0.05, Math.min(200, cam.dist * Math.exp(e.deltaY * (e.ctrlKey ? 0.01 : 0.002))));
      }, { passive: false });

      canvas.addEventListener('contextmenu', (e) => e.preventDefault());

      document.addEventListener('keydown', (e) => {
        if (sceneState.selectionMode === 4) {
          if (window.__handleSketchKey && window.__handleSketchKey(e)) return;
        }
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
