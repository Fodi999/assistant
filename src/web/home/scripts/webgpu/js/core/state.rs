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

      // mouse NDC state declared in input/mouse.rs
"##;
