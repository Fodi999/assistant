// ── JS: Application state — camera, scene, input dispatcher ─────────────────
// Domain: Application — runtime state objects + pointer/keyboard wiring.

pub const JS: &str = r##"
      const PARTICLE_STRIDE = 32;
      const HARD_CAP        = 5_000_000;
      const deviceCap       = Math.floor(device.limits.maxStorageBufferBindingSize / PARTICLE_STRIDE);
      const MAX_PARTICLES   = Math.min(HARD_CAP, deviceCap);
      let   NUM_SPHERES     = 1;
      const CLOUD_VOLUME    = (4 / 3) * Math.PI * Math.pow(5.5, 3);

      function buildParticles(count) {
        const data = new Float32Array(count * 8);
        for (let i = 0; i < count; i++) {
          const b = i * 8;
          data[b+0] = 1000; data[b+1] = 1000; data[b+2] = 1000;
          data[b+3] = 0.0001;
          data[b+4] = 0; data[b+5] = 0; data[b+6] = 0; data[b+7] = 0;
        }
        return data;
      }
      let sphereData = buildParticles(NUM_SPHERES);

      const sceneState = {
        engineMode:      'SKETCH',
        objectPosition:  [0.0, 0.0, 0.0],
        objectRotation:  [0.0, 0.0, 0.0],
        objectScale:     [1.0, 1.0, 1.0],
        baseMeshDim:     [2.0, 2.0, 2.0],
        objectBevel:     0.040,
        objectProfile:   1.0,
        objectRoundness: 0.0,
        selected:        false,
      };

      const cam = {
        yaw:    Math.PI / 4,
        pitch:  Math.PI / 6,
        dist:   10.0,
        target: [0.0, 0.0, 0.0],
        autoRotate: false,
        ortho: false,
        orthoScale: 0.45,
        fov: 45,
      };

      window.setCameraPreset = function(preset) {
        switch (preset) {
          case 'front': cam.yaw = 0.0;         cam.pitch = 0.0;                  break;
          case 'right': cam.yaw = Math.PI*0.5; cam.pitch = 0.0;                  break;
          case 'top':   cam.yaw = 0.0;         cam.pitch = -Math.PI*0.5 + 0.001; break;
          case 'iso':   cam.yaw = 0.785;       cam.pitch = -0.615;               break;
        }
      };
      window.setCameraProjection = function(mode) { cam.ortho = (mode === 'ortho'); };

      const shape = { roundness: 0.0 };
      function shapeExponent(r) { return 22.0; }
      function updateCameraForCount(_n) {}

      const FORM_SCALE    = { cube: 1.8, cloud: 1.5, wall: 1.6 };
      const FORM_OBJSCALE = { cube: 0.18, cloud: 0.30, wall: 0.22 };
      const formation     = { mode: 'cube', target: 1.0, mix: 1.0 };
      const formationDefaults = { cube:{}, cloud:{}, wall:{} };
      function setFormation(m) { formation.mode = m; }
      const cellSdf   = { on: false, radius: 0.05, colorMode: 0, hideLow: false };
      const floorGrid = { scale: 1.0 };
      function toggleCellSdf()  { cellSdf.on = !cellSdf.on; }
      function cycleColorMode() { cellSdf.colorMode = (cellSdf.colorMode+1)%3; }

      const mouse = { ndcX: 999, ndcY: 999, active: false };

      // ── Touchpad-safe gestures ────────────────────────────────
      // - Plain left drag = ORBIT  (unless Space held)
      // - Right / middle / Shift+left / Space+left = PAN
      // - Alt/Option+left = ORBIT explicitly (touchpad friendly)
      // - Click only fires if pointermove dist < CLICK_THRESH_PX
      const CLICK_THRESH_PX = 5;
      let spaceHeld = false;
      let dragging  = false, panning = false, orbiting = false;
      let lastX = 0, lastY = 0, startX = 0, startY = 0;
      let dragMoved = false;

      function __setCursorForTool() {
        const t = sketchState.activeTool;
        let cur = 'default';
        if (t === 'point' || t === 'line') cur = 'crosshair';
        else if (t === 'grab')             cur = 'move';
        else if (t === 'delete')           cur = 'not-allowed';
        else if (t === 'select')           cur = 'default';
        canvas.style.cursor = cur;
      }
      window.__setCursorForTool = __setCursorForTool;

      canvas.addEventListener('pointerdown', (e) => {
        canvas.setPointerCapture(e.pointerId);
        dragging = true;
        dragMoved = false;
        const wantsPan = (e.button === 2) || (e.button === 1)
                       || (e.shiftKey && e.button === 0)
                       || (spaceHeld   && e.button === 0);
        panning  = wantsPan;
        orbiting = !wantsPan;
        lastX = e.clientX; lastY = e.clientY;
        startX = e.clientX; startY = e.clientY;
      });

      canvas.addEventListener('pointerup', (e) => {
        const wasDragging = dragging;
        dragging = false;
        try { canvas.releasePointerCapture(e.pointerId); } catch {}
        const dist = Math.hypot(e.clientX - startX, e.clientY - startY);
        startX = 0; startY = 0;
        if (wasDragging && e.button === 0 && dist < CLICK_THRESH_PX && !panning && !dragMoved) {
          const rect = canvas.getBoundingClientRect();
          const ndcX = ((e.clientX - rect.left) / rect.width) * 2 - 1;
          const ndcY = 1 - ((e.clientY - rect.top) / rect.height) * 2;
          if (window.__handleSketchClick) window.__handleSketchClick(ndcX, ndcY, e.shiftKey);
        }
        panning  = false;
        orbiting = false;
      });

      canvas.addEventListener('dblclick', (e) => {
        const rect = canvas.getBoundingClientRect();
        const ndcX = ((e.clientX - rect.left) / rect.width) * 2 - 1;
        const ndcY = 1 - ((e.clientY - rect.top) / rect.height) * 2;
        if (window.__handleSketchDoubleClick) window.__handleSketchDoubleClick(ndcX, ndcY);
      });

      canvas.addEventListener('pointermove', (e) => {
        const rect = canvas.getBoundingClientRect();
        mouse.ndcX = ((e.clientX - rect.left) / rect.width) * 2 - 1;
        mouse.ndcY = 1 - ((e.clientY - rect.top) / rect.height) * 2;
        mouse.active = true;

        // Perf: hover / picking block.
        const __pfPick = performance.now();

        // Hover + snap (always while pointer is on canvas, regardless of drag).
        const __pfPick = performance.now();
        const hit = window.__raycastSketchPlane && window.__raycastSketchPlane(mouse.ndcX, mouse.ndcY);
        sketchState.hoverWorld = hit || null;
        if (hit) {
          // Snap classification.
          const overPt = window.__pickPointAt(mouse.ndcX, mouse.ndcY);
          if (overPt) {
            const p = sketchState.points.find(pp => pp.id === overPt);
            sketchState.snap = { kind: 'point', pointId: overPt, gx: p ? p.gx : 0, gy: p ? p.gy : 0, gz: p ? p.gz : 0 };
          } else {
            sketchState.snap = { kind: 'grid', pointId: null, gx: hit.gx, gy: hit.gy, gz: hit.gz };
          }
        } else {
          sketchState.snap = { kind: 'none', pointId: null, gx: 0, gy: 0, gz: 0 };
        }

        const tool = sketchState.activeTool;
        if (tool === 'select' || tool === 'delete') {
          sketchState.hoverPointId = window.__pickPointAt(mouse.ndcX, mouse.ndcY);
          sketchState.hoverEdgeId  = sketchState.hoverPointId ? null : window.__pickEdgeAt(mouse.ndcX, mouse.ndcY);
          // Phase 8: profile hover only when no point/edge under cursor.
          if (tool === 'select' && !sketchState.hoverPointId && !sketchState.hoverEdgeId && hit) {
            sketchState.hoverProfileId = window.__pickProfileAtWorld(hit.freeX, hit.freeY, hit.freeZ);
          } else {
            sketchState.hoverProfileId = null;
          }
        } else if (tool === 'line') {
          // For Line tool: highlight existing point under cursor as snap target.
          sketchState.hoverPointId = window.__pickPointAt(mouse.ndcX, mouse.ndcY);
          sketchState.hoverEdgeId  = null;
          sketchState.hoverProfileId = null;
        } else {
          sketchState.hoverPointId = null;
          sketchState.hoverEdgeId  = null;
          sketchState.hoverProfileId = null;
        }
        if (window.__updateLinePreview) window.__updateLinePreview();
        if (sketchState.grab.active && hit) window.__updateGrab(hit);

        if (window.__perfSample) window.__perfSample('pick', performance.now() - __pfPick);

        if (!dragging) return;

        const dx = e.clientX - lastX;
        const dy = e.clientY - lastY;
        lastX = e.clientX; lastY = e.clientY;
        if (Math.hypot(e.clientX - startX, e.clientY - startY) >= CLICK_THRESH_PX) dragMoved = true;

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
        } else if (orbiting) {
          cam.yaw   += dx * 0.005;
          cam.pitch += dy * 0.005;
          cam.pitch = Math.max(-Math.PI/2 + 0.05, Math.min(Math.PI/2 - 0.05, cam.pitch));
        }
      });

      canvas.addEventListener('pointerleave', () => { mouse.active = false; });

      canvas.addEventListener('wheel', (e) => {
        e.preventDefault();
        cam.dist = Math.max(0.5, Math.min(200, cam.dist * Math.exp(e.deltaY * (e.ctrlKey ? 0.01 : 0.002))));
      }, { passive: false });

      canvas.addEventListener('contextmenu', (e) => e.preventDefault());

      document.addEventListener('keydown', (e) => {
        if (e.code === 'Space') { spaceHeld = true; }
        if (window.__handleSketchKey && window.__handleSketchKey(e)) return;
      });
      document.addEventListener('keyup', (e) => {
        if (e.code === 'Space') { spaceHeld = false; }
      });
"##;
