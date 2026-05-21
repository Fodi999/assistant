// ── JS: Application state — camera, scene ────────────────────────────────────
// Domain: Application — runtime state objects (particles, camera, scene).
// Input handling (keyboard / mouse / touchpad) lives in js/input/.

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
      window.cam = cam;  // exposed for face-picking ray builder

      window.setCameraPreset = function(preset) {
        switch (preset) {
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

      // Expose cam on window so solid_face_meta.rs ray-picker can access it.
      window.cam = cam;

      // ── Frame / Center scene (Space) ─────────────────────────────────────
      // Animates cam.target → centroid of all sketch points (or origin if empty),
      // and cam.dist → a comfortable viewing distance.
      window.__frameCenterScene = function() {
        const ss = window.sketchState;
        let cx = 0, cy = 0, cz = 0, n = 0;

        // Compute centroid of all sketch points
        if (ss && ss.points && ss.points.length > 0) {
          for (const p of ss.points) { cx += p.x; cy += p.y; cz += p.z; }
          n = ss.points.length;
          cx /= n; cy /= n; cz /= n;
        } // empty scene → centroid stays (0,0,0)

        // Animate cam.target → centroid, dist stays as-is
        const FRAMES = 30;
        const t0x = cam.target[0], t0y = cam.target[1], t0z = cam.target[2];
        let f = 0;
        function _step() {
          f++;
          const ease = 1 - Math.pow(1 - f / FRAMES, 3); // cubic ease-out
          cam.target[0] = t0x + (cx - t0x) * ease;
          cam.target[1] = t0y + (cy - t0y) * ease;
          cam.target[2] = t0z + (cz - t0z) * ease;
          // cam.dist intentionally not changed — Space only centres, not zooms
          if (f < FRAMES) requestAnimationFrame(_step);
        }
        requestAnimationFrame(_step);

        if (window.__setStatusMessage)
          window.__setStatusMessage('⌖ Сцена центрирована' + (n ? ' (' + n + ' точек)' : ''));
      };

      // ── Reset camera to default iso view ─────────────────────────────────
      window.__resetCamera = function() {
        const FRAMES = 30;
        const t0x = cam.target[0], t0y = cam.target[1], t0z = cam.target[2];
        const d0 = cam.dist, y0 = cam.yaw, p0 = cam.pitch;
        const ty = Math.PI / 4, tp = -0.615, td = 10.0;
        let f = 0;
        function _step() {
          f++;
          const ease = 1 - Math.pow(1 - f / FRAMES, 3);
          cam.target[0] = t0x * (1 - ease);
          cam.target[1] = t0y * (1 - ease);
          cam.target[2] = t0z * (1 - ease);
          cam.dist  = d0  + (td - d0)  * ease;
          cam.yaw   = y0  + (ty - y0)  * ease;
          cam.pitch = p0  + (tp - p0)  * ease;
          if (f < FRAMES) requestAnimationFrame(_step);
        }
        requestAnimationFrame(_step);
      };

      // mouse NDC state declared in input/mouse.rs
"##;
