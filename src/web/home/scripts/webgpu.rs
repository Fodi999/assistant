pub fn webgpu_js() -> &'static str {
    r##"
    // ══════════════════════════════════════════════════════════════
    // WebGPU Scene v2.1 — Cell-SDF megashape (kernel::particle_shape port)
    //   • drag / shift+drag  — orbit / pan camera
    //   • wheel              — zoom 0.5…80
    //   • 1-5                — 1K / 10K / 100K / 500K / 1M
    //   • + / -              — ×1.5 fine
    //   • C / V / W          — cloud / cube / wall formation
    //   • S                  — toggle Cell-SDF rendering (kernel parity)
    //   • [ / ]              — cell radius (when Cell-SDF on) / shape exp (otherwise)
    //   • N                  — debug: show surface normals as RGB
    //   • M                  — debug: colour by SlotKind (face/edge/corner)
    //   • I                  — debug: hide interior + face cells (edges & corners only)
    //   • F                  — debug preset: M + I together
    //   • R                  — auto-rotate
    //   • B                  — run full benchmark (1K→5M)
    // ══════════════════════════════════════════════════════════════
    let gpuRafId = null;

    async function startWebGpuScene() {
      const canvas = document.getElementById('webgpu-canvas');
      const badge  = document.getElementById('webgpu-status');
      const diag   = document.getElementById('gpu-diag');

      function log(msg, color = '#e2e8f0') {
        console.log('[WebGPU]', msg);
        if (diag) diag.innerHTML += `<span style="color:${color}">${msg}</span><br>`;
      }
      function setBadge(text, color) {
        if (badge) { badge.textContent = text; badge.style.color = color; }
      }

      canvas.width  = window.innerWidth;
      canvas.height = window.innerHeight;
      if (diag) {
        diag.style.display = 'block';
        diag.innerHTML = '<b style="color:#67e8f9">WebGPU Particles v1.3 · 1K…1M · free orbit</b><br>';
      }
      log(`canvas: ${canvas.width}×${canvas.height} | dpr: ${window.devicePixelRatio}`);

      // ── 1. Probe ────────────────────────────────────────────────
      if (!navigator.gpu) { setBadge('✗ WebGPU недоступен', '#f87171'); return; }
      let adapter, device;
      try {
        adapter = await navigator.gpu.requestAdapter({ powerPreference: 'high-performance' });
        if (!adapter) throw new Error('requestAdapter() = null');
        const hasTimestamp = adapter.features.has('timestamp-query');

        // ask for the maximum storage buffer the adapter allows (so we can fit 5M)
        const maxStorageBind = adapter.limits.maxStorageBufferBindingSize;
        const maxBuffer      = adapter.limits.maxBufferSize;
        device = await adapter.requestDevice({
          requiredFeatures: hasTimestamp ? ['timestamp-query'] : [],
          requiredLimits: {
            maxStorageBufferBindingSize: maxStorageBind,
            maxBufferSize:               maxBuffer,
          },
        });
        log(`✓ adapter · maxStorageBuf=${(maxStorageBind/1048576)|0}MB · ts=${hasTimestamp}`, '#34d399');
      } catch (e) { setBadge('✗ ' + e.message, '#fb923c'); return; }
      const hasTimestamp = device.features.has('timestamp-query');

      // ── 2. Configure canvas ─────────────────────────────────────
      const grad = document.querySelector('.render-gradient');
      const grid = document.querySelector('.render-grid');
      if (grad) grad.style.visibility = 'hidden';
      if (grid) grid.style.visibility = 'hidden';
      document.body.classList.add('gpu-active');

      const dpr = window.devicePixelRatio || 1;
      function resizeCanvas() {
        canvas.width  = Math.floor(window.innerWidth  * dpr);
        canvas.height = Math.floor(window.innerHeight * dpr);
      }
      resizeCanvas();
      window.addEventListener('resize', resizeCanvas);

      const fmt    = navigator.gpu.getPreferredCanvasFormat();
      const gpuCtx = canvas.getContext('webgpu');
      gpuCtx.configure({ device, format: fmt, alphaMode: 'opaque' });

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

      // ── 4. Camera state ─────────────────────────────────────────
      // spherical coords around target — pitch is unclamped for full sphere orbit
      const cam = {
        yaw:    0.0,
        pitch:  -0.15,
        dist:   6.0,
        target: [0, 0, 0],
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
      const formation = { mode: 'cloud', target: 0.0, mix: 0.0 };
      function setFormation(mode) {
        formation.mode   = mode;
        formation.target = (mode === 'cloud') ? 0.0 : 1.0;
        log(`◇ formation = ${mode}`, '#f0abfc');
      }

      // ── Cell-SDF (kernel::particle_shape port) ────────────────
      // on        : when true, cube formation uses per-cell SDF instead of
      //             billboard imposters → flush seams + rounded outer hull.
      // radius    : 0..0.5 corner radius (cell-local units).
      // colorMode : 0 normal · 1 normals-as-RGB · 2 colour-by-SlotKind.
      // hideLow   : true → cull cells with ≤ 1 exposed face (show only edges/corners).
      const cellSdf = { on: false, radius: 0.25, colorMode: 0, hideLow: false };
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

      // ── 5. GPU buffers ──────────────────────────────────────────
      // Uniform layout (8 × vec4 = 128 bytes):
      //   u0: time, w, h, pushStrength
      //   u1: roX, roY, roZ, _
      //   u2: rightX, rightY, rightZ, _
      //   u3: upX, upY, upZ, _
      //   u4: fwdX, fwdY, fwdZ, _
      //   u5: mouseX, mouseY, mouseActive, shapeExponent
      //   u6: formMix(0..1), formMode(0=cloud,1=cube,2=wall), formA, formScale
      //   u7: cellSdfOn, cellRadius, colorMode(0/1/2), hideLow(0/1)
      const uniformBuf = device.createBuffer({
        size: 128,
        usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
      });

      let sphereBuf;
      try {
        sphereBuf = device.createBuffer({
          size: MAX_PARTICLES * PARTICLE_STRIDE,
          usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
        });
      } catch (e) {
        setBadge('✗ buffer alloc failed: ' + e.message, '#f87171');
        log('✗ не удалось выделить storage buffer — снизьте MAX_PARTICLES', '#f87171');
        return;
      }
      device.queue.writeBuffer(sphereBuf, 0, sphereData);

      // ── Timestamp query (GPU timing) ────────────────────────────
      let tsQuerySet = null, tsResolveBuf = null, tsReadBuf = null;
      if (hasTimestamp) {
        tsQuerySet  = device.createQuerySet({ type: 'timestamp', count: 2 });
        tsResolveBuf = device.createBuffer({ size: 16, usage: GPUBufferUsage.QUERY_RESOLVE | GPUBufferUsage.COPY_SRC });
        tsReadBuf    = device.createBuffer({ size: 16, usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ });
      }

      const bgl = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } },
        { binding: 1, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'read-only-storage' } },
      ]});
      const bindGroup = device.createBindGroup({
        layout: bgl,
        entries: [
          { binding: 0, resource: { buffer: uniformBuf } },
          { binding: 1, resource: { buffer: sphereBuf  } },
        ],
      });
      const pipelineLayout = device.createPipelineLayout({ bindGroupLayouts: [bgl] });

      // ── 6. WGSL ─────────────────────────────────────────────────
      const shaderSrc = `
struct Uniforms {
  u0: vec4f,   // time, w, h, pushStrength
  u1: vec4f,   // ro
  u2: vec4f,   // right
  u3: vec4f,   // up
  u4: vec4f,   // fwd
  u5: vec4f,   // mouseX, mouseY, mouseActive, shapeExponent
  u6: vec4f,   // formMix, formMode, formA, formScale
  u7: vec4f,   // cellSdfOn, cellRadius, colorMode, hideLow
};
@group(0) @binding(0) var<uniform> u: Uniforms;

struct Sphere { posR: vec4f, colorP: vec4f }
struct Spheres { data: array<Sphere> }
@group(0) @binding(1) var<storage, read> spheres: Spheres;

struct Vert { @builtin(position) pos: vec4f, @location(0) uv: vec2f }

@vertex fn vs_full(@builtin(vertex_index) vi: u32) -> Vert {
  var p = array<vec2f,3>(vec2f(-1,-1), vec2f(3,-1), vec2f(-1,3));
  var o: Vert;
  o.pos = vec4f(p[vi], 0.0, 1.0);
  o.uv  = p[vi] * 0.5 + 0.5;
  return o;
}

// ── Background pass ──────────────────────────────────────
@fragment fn fs_bg(in: Vert) -> @location(0) vec4f {
  let uv  = vec2f(in.uv.x, 1.0 - in.uv.y);
  let asp = u.u0.y / u.u0.z;
  let t   = u.u0.x;
  var col = vec3f(0.02, 0.04, 0.09 + sin(t * 0.3) * 0.03);
  let gd = length(vec2f((uv.x - 0.5) * asp, uv.y - 0.4));
  let g  = smoothstep(0.65, 0.0, gd) * 0.20;
  col   += vec3f(g * 0.2, g * 0.6, g * 1.1);
  col   *= 1.0 - smoothstep(0.0, 0.65, uv.y) * 0.38;
  if uv.y > 0.22 {
    let fz = 1.0 / max(uv.y - 0.22, 0.001);
    let fx = (uv.x - 0.5) * fz * 0.55;
    let lw = 0.022;
    let gxf = fract(fx * 2.0 + t * 0.04);
    let gzf = fract(fz * 0.16);
    let ln = clamp(
      step(1.0-lw, gxf) + step(1.0-lw, 1.0-gxf) +
      step(1.0-lw, gzf) + step(1.0-lw, 1.0-gzf),
      0.0, 1.0);
    let fade = smoothstep(0.0, 0.75, 1.0 - (uv.y-0.22)/0.78);
    col = mix(col, vec3f(0.04, 0.32, 0.52)*fade, ln*fade*0.65);
  }
  let hl = smoothstep(0.007, 0.0, abs(uv.y - 0.34)) * (0.5 + 0.5*sin(t*1.1));
  col += vec3f(0.08, 0.55, 1.0) * hl * 0.55;
  return vec4f(col, 1.0);
}

fn hitSphere(ro: vec3f, rd: vec3f, c: vec3f, r: f32) -> f32 {
  let oc = ro - c;
  let b  = dot(oc, rd);
  let cc = dot(oc, oc) - r * r;
  let h  = b * b - cc;
  if h < 0.0 { return -1.0; }
  return -b - sqrt(h);
}

// ── Particle pass — instanced billboards as TRUE 3D imposters ─
struct Pv {
  @builtin(position) pos:    vec4f,
  @location(0)       quadUV: vec2f,   // -1..1 inside the quad
  @location(1)       color:  vec3f,
  @location(2)       depth:  f32,
  @location(3)       phase:  f32,
  @location(4)       wCenter: vec3f,  // world-space particle center
  @location(5)       size:   f32,     // particle radius (world units)
  @location(6) @interpolate(flat) cellMask: u32, // exposed-faces bitmask
  @location(7) @interpolate(flat) halfCell: f32, // cell half-extent (world)
  @location(8) @interpolate(flat) meshMode: u32, // 1 = cube-mesh path (flat tri), 0 = billboard ray-march
  @location(9) meshN:    vec3f, // world-space face normal (mesh mode); constant across triangle
}

// ─── Cube mesh: 36 verts = 6 faces × 2 tris × 3 verts ─────────
// Returns (localPos in [-1,1]³, outward face normal, faceBit).
// Outward CCW winding so back-face culling can be enabled later.
struct CubeV { pos: vec3f, nrm: vec3f, bit: u32 }
fn cubeVert(vi: u32) -> CubeV {
  let faceIdx = vi / 6u;            // 0..5
  let triVi   = vi % 6u;
  // Two-triangle quad indices: (0,1,2) and (0,2,3)
  var quadIdx = array<u32,6>(0u, 1u, 2u, 0u, 2u, 3u);
  let corner  = quadIdx[triVi];
  // Tangent-space corners CCW: (-,-) (+,-) (+,+) (-,+)
  var tx: f32 = -1.0;  var ty: f32 = -1.0;
  if (corner == 1u) { tx =  1.0; ty = -1.0; }
  if (corner == 2u) { tx =  1.0; ty =  1.0; }
  if (corner == 3u) { tx = -1.0; ty =  1.0; }

  let axis  = faceIdx / 2u;                                // 0=X, 1=Y, 2=Z
  let isPos = (faceIdx & 1u) == 0u;                        // even=+, odd=-
  let s     = select(-1.0, 1.0, isPos);

  var pos: vec3f;  var nrm: vec3f;  var bit: u32;
  if (axis == 0u) {
    // ±X face: outward = ±X. Flip Z on negative face for CCW winding.
    pos = vec3f(s, ty, tx * s);
    nrm = vec3f(s, 0.0, 0.0);
    bit = select(2u, 1u, isPos);
  } else if (axis == 1u) {
    pos = vec3f(tx * s, s, ty);
    nrm = vec3f(0.0, s, 0.0);
    bit = select(8u, 4u, isPos);
  } else {
    pos = vec3f(tx, ty * s, s);
    nrm = vec3f(0.0, 0.0, s);
    bit = select(32u, 16u, isPos);
  }
  return CubeV(pos, nrm, bit);
}

// ─── kernel::particle_shape WGSL port ─────────────────────────
// Mirrors src/infrastructure/geometry/kernel/particle_shape.rs verbatim.
// Layout: bit 0 = +X, 1 = −X, 2 = +Y, 3 = −Y, 4 = +Z, 5 = −Z.
fn maskFromCenter(c: vec3f, halfCell: f32, scale: f32) -> u32 {
  let eps = halfCell * 1.1;
  var m: u32 = 0u;
  if c.x >  scale - eps { m |=  1u; }
  if c.x < -scale + eps { m |=  2u; }
  if c.y >  scale - eps { m |=  4u; }
  if c.y < -scale + eps { m |=  8u; }
  if c.z >  scale - eps { m |= 16u; }
  if c.z < -scale + eps { m |= 32u; }
  return m;
}

// SDF for one cell in cell-local space [-1, 1]³.
// Unexposed slabs stay at ±1 (flush); exposed shrink to ±(1-r);
// length(outside) term auto-rounds edges/corners.
fn sdfCell(p: vec3f, mask: u32, radius: f32) -> f32 {
  let r   = clamp(radius, 0.0, 0.5);
  let sxp = select(0.0, r, (mask &  1u) != 0u);
  let sxn = select(0.0, r, (mask &  2u) != 0u);
  let syp = select(0.0, r, (mask &  4u) != 0u);
  let syn = select(0.0, r, (mask &  8u) != 0u);
  let szp = select(0.0, r, (mask & 16u) != 0u);
  let szn = select(0.0, r, (mask & 32u) != 0u);
  let qx = max(p.x - (1.0 - sxp), -p.x - (1.0 - sxn));
  let qy = max(p.y - (1.0 - syp), -p.y - (1.0 - syn));
  let qz = max(p.z - (1.0 - szp), -p.z - (1.0 - szn));
  let outside = vec3f(max(qx, 0.0), max(qy, 0.0), max(qz, 0.0));
  let inside  = min(max(qx, max(qy, qz)), 0.0);
  return length(outside) + inside;
}

@vertex fn vs_particles(
  @builtin(vertex_index)   vi:   u32,
  @builtin(instance_index) inst: u32,
) -> Pv {
  // 6-vert quad in local space (-1..1)
  var qx = array<f32,6>(-1.0, 1.0,-1.0,  1.0, 1.0,-1.0);
  var qy = array<f32,6>(-1.0,-1.0, 1.0, -1.0, 1.0, 1.0);

  let t   = u.u0.x;
  let asp = u.u0.y / u.u0.z;

  let ro    = u.u1.xyz;
  let right = u.u2.xyz;
  let upv   = u.u3.xyz;
  let fwd   = u.u4.xyz;

  let sp = spheres.data[inst];
  let ph = sp.colorP.w;

  // ── formation parameters ─────────────────────────────────────
  let formMix   = clamp(u.u6.x, 0.0, 1.0);
  let formMode  = u.u6.y;       // 0 cloud · 1 cube · 2 wall
  let formA     = u.u6.z;       // grid side (cube) / row count (wall)
  let formScale = u.u6.w;
  let driftK    = 1.0 - formMix; // suppress drift while forming

  // animated world position (Lissajous drift) — damped by (1-formMix)
  var cloudC = vec3f(
    sp.posR.x + sin(t * 0.28 + ph)        * 0.30 * driftK,
    sp.posR.y + cos(t * 0.37 + ph * 1.27) * 0.22 * driftK,
    sp.posR.z + sin(t * 0.21 + ph * 0.73) * 0.18 * driftK,
  );

  // soft gravity toward scene origin → organic clumps (also damped)
  let radial   = length(sp.posR.xyz);
  let pullAmt  = smoothstep(1.5, 5.5, radial) * 0.35 * driftK;
  let toOrigin = -normalize(sp.posR.xyz + vec3f(1e-4));
  let swirl    = vec3f(-sp.posR.z, 0.0, sp.posR.x) * 0.02 * driftK;
  cloudC      += toOrigin * pullAmt * (0.6 + 0.4 * sin(t * 0.5 + ph));
  cloudC      += swirl    * sin(t * 0.3 + ph * 0.5);

  // ── deterministic formation target from instance index ──────
  // overlap factor depends on shape: spheres need √2 to cover diagonals,
  // cubes (high n) tile exactly with factor 1.0
  // n=1 octahedron · n=2 sphere · n→∞ cube
  let n         = max(u.u5.w, 1.0);
  let cubeness  = clamp((n - 2.0) / 20.0, 0.0, 1.0); // 0 sphere/octa → 1 cube
  let coverK    = mix(1.45, 1.02, cubeness);

  var formed     = vec3f(0.0);
  var targetR    = sp.posR.w;     // target radius when fully formed
  var aliveForm  = true;          // false → particle hidden (extra over grid capacity)
  var halfCell   = sp.posR.w;     // cell half-extent in world units
  var cellMask: u32 = 63u;        // default: all faces exposed (cloud / wall)

  if formMode > 0.5 && formMode < 1.5 {
    // CUBE: solid side³ grid. Every particle is one cell at integer coord
    // (ix, iy, iz). Adjacent cells differ by exactly 2·halfCell in their
    // centre, so unexposed faces are perfectly flush — magnetic packing.
    // Interior cells (mask == 0) are culled; only 6s²−12s+8 stay visible.
    let side    = u32(formA);
    let totalCells = side * side * side;
    if inst < totalCells {
      let ix = inst % side;
      let iy = (inst / side) % side;
      let iz = inst / (side * side);

      // mask via direct face-touch test (= particle_shape::CubeGrid::classify)
      var m: u32 = 0u;
      if ix == side - 1u { m |=  1u; }
      if ix == 0u        { m |=  2u; }
      if iy == side - 1u { m |=  4u; }
      if iy == 0u        { m |=  8u; }
      if iz == side - 1u { m |= 16u; }
      if iz == 0u        { m |= 32u; }

      // cull interior — never contributes pixels
      if m == 0u {
        aliveForm = false;
        formed    = vec3f(0.0);
        targetR   = 0.0;
      } else {
        // centre at (-1+(2i+1)/side)·formScale
        let fx = (f32(ix) + 0.5) / formA * 2.0 - 1.0;
        let fy = (f32(iy) + 0.5) / formA * 2.0 - 1.0;
        let fz = (f32(iz) + 0.5) / formA * 2.0 - 1.0;
        formed   = vec3f(fx, fy, fz) * formScale;
        halfCell = formScale / formA;
        cellMask = m;

        // imposter mode: radius = halfCell (sphere just touches its 6 neighbours);
        // cell-SDF mode: radius = √3·halfCell (billboard bounds the rotated cube).
        let cellR     = halfCell * 1.7321;
        let imposterR = halfCell;
        targetR  = mix(imposterR, cellR, clamp(u.u7.x, 0.0, 1.0));

        // hideLow debug: show only edges & corners
        if u.u7.w > 0.5 {
          let nb = countOneBits(cellMask);
          if nb <= 1u { aliveForm = false; }
        }
      }
    } else {
      aliveForm = false;
      formed    = vec3f(0.0);
      targetR   = 0.0;
    }
  } else if formMode > 1.5 {
    // WALL: cols × rows  (cols·rows ≤ N is guaranteed from JS)
    // Each tile is a real 3D cube with axis-aligned extents:
    //   X half = scale·aspect/cols    Y half = scale/rows    Z half = min(X,Y)
    // We expose +Z / -Z always (front + back of wall) and +X/-X/+Y/-Y only
    // on the outer rim of the rectangle so neighbour seams disappear like the
    // cube formation. halfCell = min(tileX, tileY) keeps tile cube proportions.
    let cols       = u32(formA);
    let rows       = max(formScale, 1.0);
    let rowsU      = u32(rows);
    let totalCells = cols * rowsU;
    if inst < totalCells {
      let r      = inst / cols;
      let c      = inst % cols;
      let scale  = 2.4;
      let aspect = formA / rows;
      // Tile centre in world (Z = 0 plane).
      let uu     = (f32(c) + 0.5) / formA * 2.0 - 1.0;
      let vv     = (f32(r) + 0.5) / rows  * 2.0 - 1.0;
      formed     = vec3f(uu * aspect * scale, vv * scale, 0.0);

      // Tile half-extents (X may differ from Y when aspect ≠ 1).
      let tileX  = scale * aspect / formA;
      let tileY  = scale / rows;
      // Use min for cellMask SDF/mesh maths: tile becomes a square slab.
      halfCell   = min(tileX, tileY);
      // World-extent radius for billboard fallback (covers the whole tile).
      targetR    = max(tileX, tileY);

      // Wall cellMask: edges expose ±X / ±Y; front+back (±Z) always exposed.
      var m: u32 = 16u | 32u;
      if c == cols - 1u { m |=  1u; }
      if c == 0u        { m |=  2u; }
      if r == rowsU - 1u { m |=  4u; }
      if r == 0u         { m |=  8u; }
      cellMask = m;
    } else {
      aliveForm = false;
      formed    = vec3f(0.0, 0.0, -100.0); // off-screen behind
      targetR   = 0.0;
    }
  }

  // smoothly grow / shrink particle to fill its cell when formed
  let mixT  = smoothstep(0.0, 1.0, formMix);
  let size  = mix(sp.posR.w, targetR, mixT);

  // no wobble in formation — particles must lock perfectly still
  let settle = vec3f(0.0);

  var center = mix(cloudC, formed + settle, smoothstep(0.0, 1.0, formMix));

  // sand-push along mouse ray (still works in any mode, slightly weaker when formed)
  let mAct = u.u5.z;
  if mAct > 0.5 {
    let mNdc = vec2f(u.u5.x * asp, u.u5.y);
    let rdM  = normalize(mNdc.x * right + mNdc.y * upv + 1.5 * fwd);
    let toC      = center - ro;
    let projLen  = dot(toC, rdM);
    let perp     = toC - rdM * projLen;
    let dist     = length(perp);
    if dist > 0.001 && projLen > 0.0 {
      let falloff = exp(-dist * 2.2);
      let dir     = perp / dist;
      center += dir * falloff * u.u0.w * (1.0 - formMix);
    }
  }

  // view-space transform
  let rel = center - ro;
  let vx  = dot(rel, right);
  let vy  = dot(rel, upv);
  let vz  = dot(rel, fwd);

  // ── Screen-space LOD ──
  // Project particle radius to pixels: pxR ≈ size · focal / vz · (viewportH / 2)
  let viewH = u.u0.z;
  let pxR   = size * 1.5 / max(vz, 0.05) * (viewH * 0.5);

  // CUBE LOD by cellMask popcount (decimation when shell pixels < 1):
  //   pxR ≥ 1.5  → all surface particles (popcount ≥ 1)               · LOD0
  //   pxR ∈ [0.8, 1.5)  → drop face-interior, keep edges + corners    · LOD1
  //   pxR ∈ [0.4, 0.8)  → drop edges, keep 8 corner cells             · LOD2
  //   pxR < 0.4         → keep only corners with extra inflation       · LOD3
  // Survivors inflate so total covered area ≈ unchanged ("Вариант 2": 2×2×2 → 1 block).
  var lodInflate: f32 = 1.0;
  if formMix > 0.5 && formMode > 0.5 && formMode < 1.5 {
    let bits = countOneBits(cellMask);
    if pxR < 0.4 {
      if bits < 3u { aliveForm = false; }
      lodInflate = 4.0;
    } else if pxR < 0.8 {
      if bits < 2u { aliveForm = false; }
      lodInflate = 2.0;
    } else if pxR < 1.5 {
      lodInflate = 1.4;
    }
  }

  // Universal screen-space minimum: never let a particle fall below 1 px (kills subpixel flicker)
  let curPxR  = pxR * lodInflate;
  let pxBoost = select(1.0, 1.0 / max(curPxR, 0.001), curPxR < 1.0);
  let sizeLod = size * lodInflate * pxBoost;

  if vz < 0.05 || !aliveForm {
    var dead: Pv;
    dead.pos      = vec4f(0.0, 0.0, -2.0, 1.0);
    dead.quadUV   = vec2f(0.0);
    dead.color    = vec3f(0.0);
    dead.depth    = 0.0;
    dead.phase    = 0.0;
    dead.wCenter  = vec3f(0.0);
    dead.size     = 0.0;
    dead.cellMask = 0u;
    dead.halfCell = 0.0;
    dead.meshMode = 0u;
    dead.meshN    = vec3f(0.0);
    return dead;
  }

  // ── Choose render path ───────────────────────────────────────
  // MESH PATH (true 3D cube triangle): activated for fully-formed cube OR
  // wall when the shape exponent is high enough (n > 8 → virtually a cube).
  // Hardware-rasterized triangles → zero ray-march artifacts on tilt.
  let isCubeForm = formMode > 0.5 && formMode < 1.5;
  let isWallForm = formMode > 1.5;
  let useMesh    = formMix > 0.95
                && (isCubeForm || isWallForm)
                && n > 8.0
                && halfCell > 0.0001;

  // For non-mesh mode, only the first 6 verts are real (billboard quad).
  // Verts 6..35 must be killed without affecting depth.
  if !useMesh && vi >= 6u {
    var dead: Pv;
    dead.pos      = vec4f(0.0, 0.0, -2.0, 1.0);
    dead.quadUV   = vec2f(0.0);
    dead.color    = vec3f(0.0);
    dead.depth    = 0.0;
    dead.phase    = 0.0;
    dead.wCenter  = vec3f(0.0);
    dead.size     = 0.0;
    dead.cellMask = 0u;
    dead.halfCell = 0.0;
    dead.meshMode = 0u;
    dead.meshN    = vec3f(0.0);
    return dead;
  }

  if useMesh {
    // ── True 3D cube mesh: 6 faces × 2 tris × 3 verts = 36 verts ──
    let cv = cubeVert(vi);
    // Cull faces that are not exposed (touch a neighbour cube): emit degenerate.
    if (cellMask & cv.bit) == 0u {
      var dead: Pv;
      dead.pos      = vec4f(0.0, 0.0, -2.0, 1.0);
      dead.quadUV   = vec2f(0.0);
      dead.color    = vec3f(0.0);
      dead.depth    = 0.0;
      dead.phase    = 0.0;
      dead.wCenter  = vec3f(0.0);
      dead.size     = 0.0;
      dead.cellMask = 0u;
      dead.halfCell = 0.0;
      dead.meshMode = 0u;
      dead.meshN    = vec3f(0.0);
      return dead;
    }

    // World-space cube vertex (axis-aligned, no per-particle rotation).
    // halfExt scaled by lodInflate so LOD blocks grow without leaving holes.
    let halfExt = halfCell * lodInflate;
    let wp      = center + cv.pos * halfExt;

    // View-space + perspective project (same projection as billboard path).
    let relV = wp - ro;
    let mvx  = dot(relV, right);
    let mvy  = dot(relV, upv);
    let mvz  = dot(relV, fwd);
    if mvz < 0.05 {
      var dead: Pv;
      dead.pos      = vec4f(0.0, 0.0, -2.0, 1.0);
      dead.quadUV   = vec2f(0.0);
      dead.color    = vec3f(0.0);
      dead.depth    = 0.0;
      dead.phase    = 0.0;
      dead.wCenter  = vec3f(0.0);
      dead.size     = 0.0;
      dead.cellMask = 0u;
      dead.halfCell = 0.0;
      dead.meshMode = 0u;
      dead.meshN    = vec3f(0.0);
      return dead;
    }
    let focalM = 1.5;
    let mcx    = mvx * focalM / mvz / asp;
    let mcy    = mvy * focalM / mvz;
    let mzNdc  = clamp(mvz / (mvz + 8.0), 0.0, 0.9999);

    var o: Pv;
    o.pos      = vec4f(mcx, mcy, mzNdc, 1.0);
    o.quadUV   = vec2f(0.0);
    o.color    = sp.colorP.xyz;
    o.depth    = mvz;
    o.phase    = 0.0;                     // no per-particle rotation in mesh mode
    o.wCenter  = center;
    o.size     = halfExt;
    o.cellMask = cellMask;
    o.halfCell = halfCell;
    o.meshMode = 1u;                      // → fragment shader: skip ray-march
    o.meshN    = cv.nrm;                  // world-space face normal (already axis-aligned)
    return o;
  }

  // billboard quad scaled to particle radius (world units)
  // size already lerped above between spawn radius and target cell radius.
  // sizeLod includes screen-space LOD inflation + min-pixel clamp.
  // INFLATE for cube-like shapes: a unit cube's corners reach √3 from origin,
  // so a billboard of half-size R clips the cube into a sphere of radius R.
  // Sphere (n=2) and octahedron (n=1) fit in unit sphere → inflate = 1.
  // Cube (n→∞)  → inflate = √3 ≈ 1.732 so corners stay inside the quad.
  let cubeness2 = clamp((n - 2.0) / 20.0, 0.0, 1.0);
  let inflate   = mix(1.0, 1.7321, cubeness2);
  let billSize  = sizeLod * inflate;
  let lx   = qx[vi] * billSize;
  let ly   = qy[vi] * billSize;

  // perspective project
  let focal = 1.5;
  let cx = (vx + lx) * focal / vz / asp;
  let cy = (vy + ly) * focal / vz;

  // map view-space z (≥0.05) to NDC depth [0..1) monotonically — used
  // only as a coarse sort key; the fragment shader writes the precise
  // per-pixel depth based on the actual ray-sphere hit point.
  let zNdc = clamp(vz / (vz + 8.0), 0.0, 0.9999);

  var o: Pv;
  o.pos      = vec4f(cx, cy, zNdc, 1.0);
  o.quadUV   = vec2f(qx[vi], qy[vi]);
  o.color    = sp.colorP.xyz;
  o.depth    = vz;
  // Damp per-particle rotation while forming → cubes/octa/etc align to world
  // axes when fully formed (phase=0 → rotMat = identity).
  // Use smoothstep with hard zero past 0.92 so residual lerp can't keep cubes
  // tilted by a few degrees (which would break face-to-face seam alignment).
  let phaseDamp = 1.0 - smoothstep(0.0, 0.92, formMix);
  o.phase    = ph * phaseDamp;
  o.wCenter  = center;
  o.size     = billSize;       // billboard half-size in world (used to reconstruct ray pixel)
  o.cellMask = cellMask;
  o.halfCell = halfCell;
  o.meshMode = 0u;
  o.meshN    = vec3f(0.0);
  return o;
}

// ── Per-particle rotation matrix from phase (cheap, deterministic) ──
fn rotMat(ph: f32) -> mat3x3f {
  let cy = cos(ph);
  let sy = sin(ph);
  let cp = cos(ph * 1.37);
  let sp = sin(ph * 1.37);
  // yaw (around y) · pitch (around x)
  return mat3x3f(
    vec3f( cy,    0.0,  sy   ),
    vec3f( sy*sp, cp,  -cy*sp),
    vec3f(-sy*cp, sp,   cy*cp),
  );
}

// ── Superquadric SDF (smoothed for ray-march stability) ──
// |x|ⁿ + |y|ⁿ + |z|ⁿ = 1  → unit superquadric
// NOT a true distance function (not Lipschitz-1); only used as level-set
// for medium n. For n→1 (octa) and n→∞ (cube) use exact SDFs below.
fn sdSuperq(p: vec3f, n: f32) -> f32 {
  let q = abs(p);
  let v = pow(q.x, n) + pow(q.y, n) + pow(q.z, n);
  return pow(v, 1.0 / n) - 1.0;
}

// ── Exact box SDF (unit cube [-1,1]³) — Lipschitz-1, perfect for ray-march ──
fn sdBox(p: vec3f) -> f32 {
  let q = abs(p) - vec3f(1.0);
  return length(max(q, vec3f(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
}

// ── Exact octahedron SDF (|x|+|y|+|z|=1) — Lipschitz, sharp triangular faces ──
fn sdOcta(p: vec3f) -> f32 {
  let q = abs(p);
  return (q.x + q.y + q.z - 1.0) * 0.57735027; // 1/√3 normalises gradient to unit
}

// ── Master shape SDF: dispatch to exact form at extremes ──
fn sdShape(p: vec3f, n: f32) -> f32 {
  if (n >= 8.0)  { return sdBox(p);  }   // n≥8 visually indistinguishable from cube
  if (n <= 1.05) { return sdOcta(p); }   // n≈1 → exact octahedron
  return sdSuperq(p, n);                 // smooth superquadric for the in-between
}

struct FragOut {
  @location(0)              color: vec4f,
  @builtin(frag_depth)      depth: f32,
}

@fragment fn fs_particles(p: Pv) -> FragOut {
  // ── reconstruct world-space ray through this pixel ──
  // billboard pixel position in world: wCenter + right*qx*size + up*qy*size
  let ro    = u.u1.xyz;
  let right = u.u2.xyz;
  let upv   = u.u3.xyz;
  let fwd   = u.u4.xyz;

  let n = max(u.u5.w, 1.0);
  let sphereLikeness = clamp((22.0 - n) / 20.0, 0.0, 1.0); // 1 at n≤2, 0 at n=22

  var nrm:   vec3f;
  var hitW:  vec3f;
  var rd:    vec3f;
  let R                  = p.size;
  let cellOn             = u.u7.x > 0.5 && p.halfCell > 0.0001;

  // ─── MESH PATH (true rasterized cube triangle) ───
  // The hardware rasterizer already placed this fragment exactly on the cube
  // face triangle. Use the flat face normal directly. Zero ray-march, zero
  // depth-precision drama, zero seam artifacts.
  if p.meshMode == 1u {
    nrm  = p.meshN;
    // World-space hit on the face plane (flat shading — face normal is constant).
    // Approximate hit point as wCenter + halfCell · meshN (face centre);
    // this is used only for distance fog and fresnel — both flat per face is OK.
    hitW = p.wCenter + p.meshN * p.halfCell;
    rd   = normalize(hitW - ro);
  } else {
    // billboard ray reconstruction (only needed in non-mesh paths)
    let pixelW = p.wCenter + right * p.quadUV.x * p.size + upv * p.quadUV.y * p.size;
    rd = normalize(pixelW - ro);

    if cellOn {
      // ── kernel::particle_shape ray-march in cell-local space ──
      // pL = (pW - wCenter) / halfCell  →  cell occupies [-1, 1]³
      let h    = p.halfCell;
      let mask = p.cellMask;
      let rad  = u.u7.y;
      let roL  = (ro - p.wCenter) / h;
    let rdL  = rd;  // unit in world; t parameter is in cell-local units (tw = tl·h)

    // bounding sphere of the unit cube has radius √3 ≈ 1.7321
    let bL = dot(roL, rdL);
    let cL = dot(roL, roL) - 3.0;
    let hL = bL * bL - cL;
    if hL < 0.0 { discard; }
    var tCur = max(0.0, -bL - sqrt(hL));
    let tEnd = -bL + sqrt(hL);

    var hit = false;
    var tHit: f32 = tCur;
    for (var i = 0; i < 28; i++) {
      let pL = roL + rdL * tCur;
      let d  = sdfCell(pL, mask, rad);
      if d < 0.0006 { hit = true; tHit = tCur; break; }
      tCur += max(d * 0.9, 0.0015);
      if tCur > tEnd { break; }
    }
    if !hit { discard; }

    let pL  = roL + rdL * tHit;
    let eps = 0.0025;
    let gx  = sdfCell(pL + vec3f(eps,0,0), mask, rad) - sdfCell(pL - vec3f(eps,0,0), mask, rad);
    let gy  = sdfCell(pL + vec3f(0,eps,0), mask, rad) - sdfCell(pL - vec3f(0,eps,0), mask, rad);
    let gz  = sdfCell(pL + vec3f(0,0,eps), mask, rad) - sdfCell(pL - vec3f(0,0,eps), mask, rad);
    nrm  = normalize(vec3f(gx, gy, gz));
    hitW = ro + rd * (tHit * h);
  } else if abs(n - 2.0) < 0.05 {
    // ── analytical sphere intersection (exact n=2 fast-path) ──
    let oc = ro - p.wCenter;
    let b  = dot(oc, rd);
    let c  = dot(oc, oc) - R * R;
    let h  = b * b - c;
    if h < 0.0 { discard; }
    let tHit = -b - sqrt(h);
    if tHit < 0.0 { discard; }
    hitW = ro + rd * tHit;
    nrm  = (hitW - p.wCenter) / R;
  } else {
    // ── ray-march superquadric in particle local space ──
    // p.size is the billboard half-size (inflated). The actual shape half-extent
    // along axes is shapeR = p.size / inflate, where inflate ∈ [1, √3] depending
    // on n. We march in unit-shape space [-1,1]³, scaled by shapeR.
    let cubenessF = clamp((n - 2.0) / 20.0, 0.0, 1.0);
    let inflateF  = mix(1.0, 1.7321, cubenessF);
    let shapeR    = R / inflateF;

    let rot = rotMat(p.phase);
    // transform ray to local space: local = rot · (world - center) / shapeR
    let roL = rot * (ro - p.wCenter) / shapeR;
    let rdL = rot * rd;

    // bounding-sphere entry to skip empty space (radius √3 covers unit cube)
    let bL = dot(roL, rdL);
    let cL = dot(roL, roL) - 3.0;
    let hL = bL * bL - cL;
    if hL < 0.0 { discard; }
    var tCur = max(0.0, -bL - sqrt(hL));
    let tEnd = -bL + sqrt(hL);

    var marched = false;
    var tFinal: f32 = tCur;
    for (var i = 0; i < 24; i++) {
      let pL = roL + rdL * tCur;
      let d  = sdShape(pL, n);
      if d < 0.0005 { marched = true; tFinal = tCur; break; }
      tCur += max(d, 0.002);             // sdShape is Lipschitz at extremes → full step is safe
      if tCur > tEnd { break; }
    }
    if !marched { discard; }

    // local-space normal from SDF gradient (central diff)
    let pL  = roL + rdL * tFinal;
    let eps = 0.002;
    let gx  = sdShape(pL + vec3f(eps,0,0), n) - sdShape(pL - vec3f(eps,0,0), n);
    let gy  = sdShape(pL + vec3f(0,eps,0), n) - sdShape(pL - vec3f(0,eps,0), n);
    let gz  = sdShape(pL + vec3f(0,0,eps), n) - sdShape(pL - vec3f(0,0,eps), n);
    let nL  = normalize(vec3f(gx, gy, gz));
    // back to world space: transpose(rot) = inverse for rotation matrices
    nrm  = transpose(rot) * nL;
    // world hit = ro + rd * (tFinal · shapeR)
    hitW = ro + rd * (tFinal * shapeR);
  }
  } // close outer non-mesh else branch

  // ── lighting on the real 3D normal ──
  let L1 = normalize(vec3f( 0.55,  0.75, -0.30));
  let L2 = normalize(vec3f(-0.40,  0.20,  0.85));
  let v  = -rd;
  let dA = max(dot(nrm, L1), 0.0);
  let dB = max(dot(nrm, L2), 0.0) * 0.45;

  // specular
  let h  = normalize(L1 + v);
  let sp = pow(max(dot(nrm, h), 0.0), 48.0);

  // Fresnel rim
  let fr = pow(1.0 - max(dot(nrm, v), 0.0), 3.2);

  // ── seam suppression for flush-packed cube cells ──
  // When a face touches a neighbour (cellMask bit not set), this is an internal
  // seam, not the outer hull. Kill specular & rim there so the assembled cube
  // looks like one solid surface instead of a grid of individual cubes.
  var seamMul: f32 = 1.0;          // 1 = keep highlights, 0 = full suppression
  if p.halfCell > 0.0001 && p.cellMask != 0u {
    let absN = abs(nrm);
    var faceBit: u32 = 0u;
    if absN.x >= absN.y && absN.x >= absN.z {
      faceBit = select(2u, 1u, nrm.x > 0.0);     // +X = 1, -X = 2
    } else if absN.y >= absN.z {
      faceBit = select(8u, 4u, nrm.y > 0.0);     // +Y = 4, -Y = 8
    } else {
      faceBit = select(32u, 16u, nrm.z > 0.0);   // +Z = 16, -Z = 32
    }
    let isExposed = (p.cellMask & faceBit) != 0u;
    // soft transition near edges (where face is ambiguous): max axis dominance
    let axisDom = max(absN.x, max(absN.y, absN.z));
    let edgeT   = smoothstep(0.6, 0.95, axisDom);
    seamMul = select(1.0 - edgeT, 1.0, isExposed);  // exposed face → 1 always
  }
  let hitVz = max(dot(hitW - ro, fwd), 0.05);
  let t     = u.u0.x;
  let pulse = 0.85 + 0.15 * sin(t * 2.0 + p.phase);
  let fog   = exp(-hitVz * 0.045);

  var col = p.color * 0.14;                                   // ambient
  col    += p.color * (dA + dB) * (0.65 + 0.20 * pulse);      // diffuse
  col    += vec3f(0.95, 0.97, 1.0) * sp * 0.55 * seamMul;     // specular (no seam shine)
  col    += vec3f(0.10, 0.65, 1.00) * fr * (0.30 + 0.20 * sphereLikeness) * seamMul;

  // subtle emissive bias so far particles do not turn pitch black
  col    += p.color * 0.05 * pulse;

  // distance darkening (atmospheric, NOT alpha)
  col *= 0.55 + 0.45 * fog;

  // tone map + gamma
  col = col / (col + vec3f(1.0));
  col = pow(col, vec3f(0.4545));

  // ── debug overrides (after tone-map so colours stay punchy) ──
  let cmode = u.u7.z;
  if cmode > 0.5 && cmode < 1.5 {
    // normals → RGB
    col = nrm * 0.5 + 0.5;
  } else if cmode > 1.5 {
    // colour by SlotKind: gray / cyan / yellow / magenta
    let bits = countOneBits(p.cellMask);
    var sc: vec3f;
    if      bits == 0u { sc = vec3f(0.50, 0.50, 0.50); }      // interior
    else if bits == 1u { sc = vec3f(0.30, 0.85, 0.95); }      // face   = cyan
    else if bits == 2u { sc = vec3f(0.95, 0.85, 0.20); }      // edge   = yellow
    else               { sc = vec3f(0.95, 0.30, 0.85); }      // corner = magenta
    // shade by lambert so silhouette stays readable
    col = sc * (0.35 + 0.65 * max(dot(nrm, L1), 0.0));
  }

  // ── per-pixel depth from real hit point (matches vertex z mapping) ──
  let zNdc = clamp(hitVz / (hitVz + 8.0), 0.0, 0.9999);

  var out: FragOut;
  out.color = vec4f(col, 1.0);   // fully opaque
  out.depth = zNdc;
  return out;
}
`;

      const module = device.createShaderModule({ code: shaderSrc });
      // Surface any WGSL compile errors to the on-page log AND console
      module.getCompilationInfo().then((info) => {
        if (info.messages.length === 0) {
          log('✓ WGSL скомпилирован', '#34d399');
        } else {
          for (const m of info.messages) {
            const tag = m.type === 'error' ? '✗ WGSL' : '⚠ WGSL';
            const colour = m.type === 'error' ? '#f87171' : '#fbbf24';
            log(`${tag} L${m.lineNum}:${m.linePos} — ${m.message}`, colour);
            console[m.type === 'error' ? 'error' : 'warn'](
              `[WGSL ${m.type}] line ${m.lineNum} col ${m.linePos}:`, m.message
            );
          }
        }
      });

      const DEPTH_FMT = 'depth24plus';
      const bgPipeline = device.createRenderPipeline({
        layout: pipelineLayout,
        vertex:   { module, entryPoint: 'vs_full' },
        fragment: { module, entryPoint: 'fs_bg', targets: [{ format: fmt }] },
        primitive: { topology: 'triangle-list' },
        depthStencil: { format: DEPTH_FMT, depthWriteEnabled: false, depthCompare: 'always' },
      });
      const spherePipeline = device.createRenderPipeline({
        layout: pipelineLayout,
        vertex:   { module, entryPoint: 'vs_particles' },
        // fully opaque — no blend, write alpha=1 from fragment
        fragment: { module, entryPoint: 'fs_particles', targets: [{ format: fmt }] },
        primitive: { topology: 'triangle-list' },
        depthStencil: { format: DEPTH_FMT, depthWriteEnabled: true, depthCompare: 'less' },
      });

      // ── Depth texture, recreated on resize ─────────────────────
      let depthTex = device.createTexture({
        size: [canvas.width, canvas.height, 1],
        format: DEPTH_FMT,
        usage: GPUTextureUsage.RENDER_ATTACHMENT,
      });
      let depthW = canvas.width, depthH = canvas.height;
      function ensureDepth() {
        if (canvas.width === depthW && canvas.height === depthH) return;
        depthTex.destroy();
        depthTex = device.createTexture({
          size: [canvas.width, canvas.height, 1],
          format: DEPTH_FMT,
          usage: GPUTextureUsage.RENDER_ATTACHMENT,
        });
        depthW = canvas.width; depthH = canvas.height;
      }
      log(`✓ pipelines готовы (bg + ${NUM_SPHERES.toLocaleString()} частиц)`, '#34d399');

      // ── HUD overlay (top-right) ─────────────────────────────────
      let hud = document.getElementById('gpu-hud');
      if (!hud) {
        hud = document.createElement('div');
        hud.id = 'gpu-hud';
        hud.style.cssText = [
          'position:fixed','top:14px','right:14px','z-index:9999',
          'padding:10px 14px','border-radius:10px',
          'background:rgba(2,6,23,.62)','backdrop-filter:blur(10px)',
          '-webkit-backdrop-filter:blur(10px)',
          'border:1px solid rgba(103,232,249,.25)',
          'font:500 12px/1.5 -apple-system,SF Pro Display,system-ui,monospace',
          'color:#cbd5e1','letter-spacing:.02em','pointer-events:auto','user-select:none',
          'box-shadow:0 8px 32px rgba(0,0,0,.4)',
        ].join(';');
        document.body.appendChild(hud);
      }
      function fmtN(n) {
        if (n >= 1_000_000) return (n / 1_000_000).toFixed(n % 1_000_000 === 0 ? 0 : 1) + 'M';
        if (n >= 1_000)     return (n / 1_000).toFixed(n % 1_000 === 0 ? 0 : 1) + 'K';
        return String(n);
      }
      function updateHud(fps) {
        const density = NUM_SPHERES / CLOUD_VOLUME;     // particles per unit³
        const dStr = density >= 1000
          ? (density / 1000).toFixed(1) + 'K/u³'
          : density.toFixed(0) + '/u³';
        const r    = shape.roundness;
        const nExp = shapeExponent(r).toFixed(1);
        // r: 0 cube → 0.5 octahedron → 1 sphere  (piecewise n: 22 → 1 → 2)
        const shapeLabel =
          r >= 0.92 ? 'sphere'             :   // n ≈ 2.0
          r >= 0.65 ? 'rounded-octa'       :   // n ∈ (1.3 .. 1.85)
          r >= 0.45 ? 'octahedron'         :   // n ≈ 1.0  ← «треугольник»
          r >= 0.25 ? 'squircle'           :   // n ∈ (5 .. 12)
          r >  0.05 ? 'rounded-cube'       :   // n ∈ (12 .. 20)
                      'super-cube';            // n ≈ 22

        // ── Cell-SDF formation stats (cube only) ──
        let cellInfo = '';
        if (formation.mode === 'cube') {
          const side    = Math.max(2, Math.floor(Math.cbrt(NUM_SPHERES)));
          const surface = 6 * side * side - 12 * side + 8;       // hollow shell
          const drawn   = side * side * side;                    // solid grid
          const interior = drawn - surface;                       // culled inside
          const colorNames = ['normal','normals-RGB','mask-color'];
          cellInfo =
            `<div style="margin-top:4px;padding-top:4px;border-top:1px dashed #1e293b">` +
            `<span style="color:#94a3b8">formation</span> <b style="color:#67e8f9">Cube ${side}×${side}×${side}</b></div>` +
            `<div><span style="color:#94a3b8">render</span> <b style="color:${cellSdf.on?'#34d399':'#94a3b8'}">${cellSdf.on?'Cell SDF':'Imposter'}</b>` +
            ` <span style="color:#94a3b8">· r</span> <b style="color:#fbbf24">${cellSdf.radius.toFixed(2)}</b></div>` +
            `<div><span style="color:#94a3b8">surface</span> <b style="color:#a78bfa">${fmtN(surface)}</b>` +
            ` <span style="color:#94a3b8">· interior</span> <b style="color:#475569">${fmtN(Math.max(0, interior))}</b></div>` +
            (cellSdf.colorMode > 0 || cellSdf.hideLow
              ? `<div><span style="color:#94a3b8">debug</span> ` +
                `<b style="color:#fbbf24">${colorNames[cellSdf.colorMode]}</b>` +
                (cellSdf.hideLow ? ` <b style="color:#f0abfc">+ hide-low</b>` : '') +
                `</div>`
              : '');
        }

        hud.innerHTML =
          `<div style="color:#67e8f9;font-weight:600;letter-spacing:.06em">PARTICLE SCENE</div>` +
          `<div style="margin-top:4px"><span style="color:#94a3b8">particles</span> `+
          `<b style="color:#a78bfa">${fmtN(NUM_SPHERES)}</b>` +
          `<span style="color:#475569"> / ${fmtN(MAX_PARTICLES)}</span></div>` +
          `<div><span style="color:#94a3b8">density</span> <b style="color:#f0abfc">${dStr}</b>` +
          ` <span style="color:#94a3b8">· vol</span> <b>${CLOUD_VOLUME.toFixed(0)}u³</b></div>` +
          `<div><span style="color:#94a3b8">shape</span> <b style="color:#fbbf24">${shapeLabel}</b>` +
          ` <span style="color:#475569">n=${nExp}</span></div>` +
          `<div style="pointer-events:auto;margin:4px 0">` +
          `<input id="gpu-shape-slider" type="range" min="0" max="1" step="0.01" value="${r}" ` +
          `style="width:100%;accent-color:#fbbf24"></div>` +
          cellInfo +
          (formation.mode === 'cube'
            ? `<div style="pointer-events:auto;margin:4px 0">` +
              `<input id="gpu-cell-r" type="range" min="0" max="0.5" step="0.01" value="${cellSdf.radius}" ` +
              `style="width:100%;accent-color:#67e8f9"></div>`
            : '') +
          `<div><span style="color:#94a3b8">fps</span> <b style="color:${fps>50?'#34d399':fps>25?'#fbbf24':'#f87171'}">${fps.toFixed(0)}</b>`+
          ` <span style="color:#94a3b8">· dist</span> <b>${cam.dist.toFixed(2)}</b></div>` +
          `<div style="margin-top:6px;display:flex;gap:4px;pointer-events:auto">` +
            ['cloud','cube','wall'].map(m =>
              `<button data-form="${m}" style="flex:1;background:${formation.mode===m?'#0e7490':'#1e293b'};` +
              `border:1px solid ${formation.mode===m?'#67e8f9':'#334155'};color:${formation.mode===m?'#ecfeff':'#cbd5e1'};` +
              `padding:4px 6px;border-radius:6px;cursor:pointer;font-size:11px;font-weight:600;text-transform:uppercase">${m}</button>`
            ).join('') +
          `</div>` +
          `<div style="margin-top:6px;display:flex;gap:6px;pointer-events:auto">` +
          `<button onclick="if(!window.__gpuBench?.running)window.__gpuRunBench?.()" style="flex:1;background:#1e293b;` +
          `border:1px solid #334155;color:#fbbf24;padding:4px 8px;border-radius:6px;` +
          `cursor:pointer;font-size:11px;font-weight:600">🔬 BENCH (B)</button></div>` +
          `<div style="margin-top:4px;color:#64748b;font-size:11px">`+
          `1-5 count · C/V/W form · S cellSDF · [/] r · N normals · M mask · I hide-low · F preset</div>`;
        // wire the slider after each rebuild
        const slider = document.getElementById('gpu-shape-slider');
        if (slider) slider.oninput = (e) => { shape.roundness = parseFloat(e.target.value); };
        const cellSlider = document.getElementById('gpu-cell-r');
        if (cellSlider) cellSlider.oninput = (e) => { cellSdf.radius = parseFloat(e.target.value); };
        // wire formation buttons
        hud.querySelectorAll('button[data-form]').forEach(b => {
          b.onclick = () => setFormation(b.dataset.form);
        });
      }
      updateHud(0);

      // ── Particle count switching ────────────────────────────────
      function setParticleCount(n) {
        n = Math.max(100, Math.min(MAX_PARTICLES, Math.floor(n)));
        if (n === NUM_SPHERES) return;
        NUM_SPHERES = n;
        sphereData  = buildParticles(n);
        device.queue.writeBuffer(sphereBuf, 0, sphereData);
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
          case 'b': case 'B': if (!bench.running) runBenchmark(); break;
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

      // ── Benchmark engine ─────────────────────────────────────────
      // Steps: 1K · 10K · 100K · 500K · 1M · 2M · 5M (filtered by MAX_PARTICLES)
      const ALL_STEPS      = [1_000, 10_000, 100_000, 500_000, 1_000_000, 2_000_000, 5_000_000];
      const BENCH_STEPS    = ALL_STEPS.filter(n => n <= MAX_PARTICLES);
      const BENCH_WARMUP   = 8;    // frames to skip before measuring
      const BENCH_SAMPLES  = 120;  // frames to measure per step

      const bench = {
        running:   false,
        step:      0,
        warmup:    0,
        samples:   [],            // frame-time samples (ms) for current step
        cpuSamples: [],           // CPU submit time (ms)
        results:   [],            // [{count, avgFps, p1Fps, avgMs, maxMs, p99Ms, cpuMs}]
        frameStart: 0,
      };

      // called from frame() when bench.running
      function benchTick(cpuFrameMs) {
        if (bench.warmup < BENCH_WARMUP) { bench.warmup++; return; }
        bench.samples.push(cpuFrameMs);
        bench.cpuSamples.push(cpuFrameMs);
        if (bench.samples.length < BENCH_SAMPLES) return;

        // compute stats
        const times = bench.samples.slice().sort((a, b) => a - b);
        const n     = times.length;
        const avgMs = times.reduce((s, v) => s + v, 0) / n;
        const maxMs = times[n - 1];
        const p99Ms = times[Math.floor(n * 0.99)];
        const p1idx = Math.max(0, Math.floor(n * 0.01));
        // 1% low FPS = avg of bottom 1% worst frame times
        const worst1pct = times.slice(Math.floor(n * 0.99));
        const p1AvgMs   = worst1pct.reduce((s, v) => s + v, 0) / worst1pct.length;
        const avgFps    = 1000 / avgMs;
        const p1Fps     = 1000 / p1AvgMs;

        bench.results.push({
          count: NUM_SPHERES,
          avgFps: avgFps.toFixed(1),
          p1Fps:  p1Fps.toFixed(1),
          avgMs:  avgMs.toFixed(2),
          maxMs:  maxMs.toFixed(2),
          p99Ms:  p99Ms.toFixed(2),
        });

        bench.step++;
        bench.samples = [];
        bench.cpuSamples = [];
        bench.warmup = 0;

        if (bench.step >= BENCH_STEPS.length) {
          bench.running = false;
          showBenchResults();
          setParticleCount(1_000_000);   // restore 1M
          return;
        }
        // next step
        const nextCount = BENCH_STEPS[bench.step];
        if (nextCount > MAX_PARTICLES) {
          // can't go higher — record N/A and finish
          for (let i = bench.step; i < BENCH_STEPS.length; i++) {
            bench.results.push({ count: BENCH_STEPS[i], avgFps: 'OOM', p1Fps: '—',
              avgMs: '—', maxMs: '—', p99Ms: '—' });
          }
          bench.running = false;
          showBenchResults();
          setParticleCount(1_000_000);
          return;
        }
        setParticleCount(nextCount);
      }

      async function runBenchmark() {
        bench.running  = true;
        bench.step     = 0;
        bench.warmup   = 0;
        bench.samples  = [];
        bench.results  = [];
        // remove old result panel
        document.getElementById('bench-overlay')?.remove();
        setParticleCount(BENCH_STEPS[0]);
        log('🔬 Benchmark запущен… нажми B чтобы увидеть результат', '#fbbf24');
        // show progress in HUD
        const origUpdate = updateHud;
        // override HUD to show bench progress
        const benchHudInterval = setInterval(() => {
          if (!bench.running) { clearInterval(benchHudInterval); return; }
          const stepCount = BENCH_STEPS[bench.step] || 0;
          hud.innerHTML =
            `<div style="color:#fbbf24;font-weight:600">🔬 BENCHMARK RUNNING</div>` +
            `<div style="margin-top:4px"><span style="color:#94a3b8">step</span> ` +
            `<b style="color:#f0abfc">${bench.step + 1} / ${BENCH_STEPS.length}</b></div>` +
            `<div><span style="color:#94a3b8">current</span> <b style="color:#a78bfa">${fmtN(stepCount)}</b></div>` +
            `<div><span style="color:#94a3b8">samples</span> <b>${bench.samples.length} / ${BENCH_SAMPLES}</b></div>` +
            `<div style="margin-top:4px;color:#64748b;font-size:11px">auto-collecting…</div>`;
        }, 200);
      }

      function showBenchResults() {
        // remove progress interval is already stopped
        document.getElementById('bench-overlay')?.remove();
        const overlay = document.createElement('div');
        overlay.id = 'bench-overlay';
        overlay.style.cssText = [
          'position:fixed','inset:0','z-index:99999',
          'background:rgba(2,6,23,.88)','backdrop-filter:blur(18px)',
          '-webkit-backdrop-filter:blur(18px)',
          'display:flex','flex-direction:column',
          'align-items:center','justify-content:center',
          'font-family:-apple-system,SF Pro Display,system-ui,monospace',
          'color:#e2e8f0',
        ].join(';');

        const VRAM_MB = (MAX_PARTICLES * 32 / 1048576).toFixed(0);

        let rows = bench.results.map(r => {
          const fpsColor = r.avgFps === 'OOM' ? '#f87171'
            : +r.avgFps >= 60 ? '#34d399'
            : +r.avgFps >= 30 ? '#fbbf24' : '#f87171';
          const p1Color  = r.p1Fps === '—' ? '#475569'
            : +r.p1Fps >= 60 ? '#34d399'
            : +r.p1Fps >= 30 ? '#fbbf24' : '#f87171';
          const bottleneck = r.avgFps === 'OOM'
            ? '<span style="color:#f87171">OUT OF MEM</span>'
            : +r.avgFps < 15
              ? '<span style="color:#f87171">GPU bound ⚠</span>'
              : +r.avgFps < 40
                ? '<span style="color:#fbbf24">GPU bound</span>'
                : '<span style="color:#34d399">CPU/rAF OK</span>';
          return `<tr>
            <td style="padding:7px 14px;color:#a78bfa;font-weight:600">${fmtN(r.count)}</td>
            <td style="padding:7px 14px;color:${fpsColor};font-weight:700">${r.avgFps}</td>
            <td style="padding:7px 14px;color:${p1Color}">${r.p1Fps}</td>
            <td style="padding:7px 14px;color:#94a3b8">${r.avgMs}</td>
            <td style="padding:7px 14px;color:#f87171">${r.maxMs}</td>
            <td style="padding:7px 14px;color:#fb923c">${r.p99Ms}</td>
            <td style="padding:7px 14px">${bottleneck}</td>
          </tr>`;
        }).join('');

        // find cliff — where fps drops below 30
        const cliffIdx = bench.results.findIndex(r => +r.avgFps < 30 && r.avgFps !== 'OOM');
        const cliffNote = cliffIdx > 0
          ? `<p style="margin-top:16px;color:#fbbf24">⚠ Bottleneck начинается при <b style="color:#f0abfc">${fmtN(bench.results[cliffIdx].count)}</b> частиц (fps &lt; 30)</p>`
          : cliffIdx === 0
            ? `<p style="margin-top:16px;color:#f87171">⚠ GPU недостаточен уже с <b>${fmtN(bench.results[0].count)}</b></p>`
            : `<p style="margin-top:16px;color:#34d399">✓ GPU держит &gt;30 fps на всех протестированных ступенях</p>`;

        overlay.innerHTML = `
          <div style="max-width:820px;width:95%;padding:32px;border-radius:16px;
            background:rgba(8,14,36,.95);border:1px solid rgba(103,232,249,.2);
            box-shadow:0 32px 80px rgba(0,0,0,.7)">
            <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:20px">
              <div>
                <h2 style="margin:0;font-size:20px;color:#67e8f9;letter-spacing:.06em">
                  🔬 PARTICLE BENCHMARK RESULTS
                </h2>
                <p style="margin:4px 0 0;font-size:12px;color:#475569">
                  ${BENCH_SAMPLES} frames / step · timestamp-query: ${hasTimestamp ? '<span style="color:#34d399">enabled</span>' : '<span style="color:#64748b">off (CPU timing)</span>'}
                  · buffer: <b style="color:#a78bfa">${VRAM_MB} MB</b> (5M max)
                </p>
              </div>
              <button id="bench-close" style="background:#1e293b;border:1px solid #334155;
                color:#94a3b8;padding:6px 16px;border-radius:8px;cursor:pointer;font-size:13px">
                ✕ закрыть
              </button>
            </div>
            <table style="width:100%;border-collapse:collapse;font-size:13px">
              <thead>
                <tr style="color:#64748b;font-size:11px;letter-spacing:.06em;text-transform:uppercase;
                  border-bottom:1px solid #1e293b">
                  <th style="padding:6px 14px;text-align:left">particles</th>
                  <th style="padding:6px 14px;text-align:left">avg fps</th>
                  <th style="padding:6px 14px;text-align:left">1% low fps</th>
                  <th style="padding:6px 14px;text-align:left">avg ms</th>
                  <th style="padding:6px 14px;text-align:left">max ms</th>
                  <th style="padding:6px 14px;text-align:left">p99 ms</th>
                  <th style="padding:6px 14px;text-align:left">bottleneck</th>
                </tr>
              </thead>
              <tbody style="border-top:1px solid #0f172a">${rows}</tbody>
            </table>
            ${cliffNote}
            <p style="margin-top:8px;font-size:11px;color:#334155">
              ms = CPU rAF→submit · 1% low = средний худший 1% кадров · p99 = 99-й перцентиль задержки
            </p>
          </div>`;
        document.body.appendChild(overlay);
        document.getElementById('bench-close').onclick = () => overlay.remove();
      }

      // ── 7. Render loop ──────────────────────────────────────────
      const t0 = performance.now();
      let frameCount = 0;
      let lastFpsTime = t0, fpsAcc = 0, fps = 0;
      let lastFrameTime = t0;
      const ubo = new Float32Array(32); // 8 × vec4

      function frame() {
        const now = performance.now();
        const t   = (now - t0) / 1000.0;
        const dt  = (now - lastFrameTime) / 1000.0;
        const cpuFrameMs = now - lastFrameTime;
        lastFrameTime = now;
        if (frameCount === 0) {
          log('🚀 render loop запущен!', '#67e8f9');
          log('🖱  drag · wheel · 1-5 · C/V/N form · [/] shape · R · B', '#a78bfa');
          setTimeout(() => { if (diag) diag.style.display = 'none'; }, 4000);
        }
        frameCount++;
        fpsAcc++;
        if (now - lastFpsTime >= 500) {
          fps = fpsAcc * 1000 / (now - lastFpsTime);
          lastFpsTime = now; fpsAcc = 0;
          if (!bench.running) updateHud(fps);
        }

        // benchmark tick (collect frame times)
        if (bench.running) benchTick(cpuFrameMs);

        // auto-rotate
        if (cam.autoRotate) cam.yaw += dt * 0.25;

        // build camera basis from spherical coords
        const cy = Math.cos(cam.yaw),   sy = Math.sin(cam.yaw);
        const cp = Math.cos(cam.pitch), sp = Math.sin(cam.pitch);
        // forward from camera → target
        const fwdX = -sy * cp, fwdY = -sp, fwdZ = cy * cp;
        // ro = target - fwd * dist
        const roX = cam.target[0] - fwdX * cam.dist;
        const roY = cam.target[1] - fwdY * cam.dist;
        const roZ = cam.target[2] - fwdZ * cam.dist;
        // right = fwd × worldUp
        const wuX = 0, wuY = 1, wuZ = 0;
        let rX = fwdY * wuZ - fwdZ * wuY;
        let rY = fwdZ * wuX - fwdX * wuZ;
        let rZ = fwdX * wuY - fwdY * wuX;
        const rL = Math.hypot(rX, rY, rZ) || 1;
        rX /= rL; rY /= rL; rZ /= rL;
        // up = right × fwd
        const uX = rY * fwdZ - rZ * fwdY;
        const uY = rZ * fwdX - rX * fwdZ;
        const uZ = rX * fwdY - rY * fwdX;

        // u0: time, w, h, pushStrength
        ubo[ 0] = t;
        ubo[ 1] = canvas.width;
        ubo[ 2] = canvas.height;
        ubo[ 3] = 0.45;            // push strength
        // u1: ro
        ubo[ 4] = roX; ubo[ 5] = roY; ubo[ 6] = roZ; ubo[ 7] = 0;
        // u2: right
        ubo[ 8] = rX;  ubo[ 9] = rY;  ubo[10] = rZ;  ubo[11] = 0;
        // u3: up
        ubo[12] = uX;  ubo[13] = uY;  ubo[14] = uZ;  ubo[15] = 0;
        // u4: fwd
        ubo[16] = fwdX; ubo[17] = fwdY; ubo[18] = fwdZ; ubo[19] = 0;
        // u5: mouse + shape exponent
        ubo[20] = mouse.ndcX;
        ubo[21] = mouse.ndcY;
        ubo[22] = mouse.active ? 1.0 : 0.0;
        ubo[23] = shapeExponent(shape.roundness);
        // ── ease formation.mix toward target (≈0.4 s settle) ─────
        {
          const k = 1.0 - Math.exp(-dt * 5.5);
          formation.mix += (formation.target - formation.mix) * k;
          // SNAP to exact endpoint once close enough — otherwise residual phase
          // (mix never reaches 1.0 via exp lerp) keeps every cube rotated by a
          // few degrees, breaking face-to-face alignment under tilted views.
          if (Math.abs(formation.target - formation.mix) < 0.002) {
            formation.mix = formation.target;
          }
        }
        // u6: formMix, formMode, paramA, paramB
        //   cube: paramA = grid side (full side³ solid),     paramB = scale
        //         interior cells hidden in shader → only 6s²−12s+8 visible.
        //   wall: paramA = cols,                              paramB = rows  (cols·rows ≤ N)
        let formModeId = 0, paramA = 1, paramB = 1.6;
        if (formation.mode === 'cube') {
          formModeId = 1;
          // side³ ≤ N → every cell of the solid grid has a particle.
          // Surface = 6s²−12s+8 visible; rest culled as interior.
          paramA = Math.max(2, Math.floor(Math.cbrt(NUM_SPHERES)));
          paramB = 1.8; // scale
        } else if (formation.mode === 'wall') {
          formModeId = 2;
          // pick cols so that cols·rows ≤ N with aspect ≈ 1.6:1
          const cols = Math.max(1, Math.floor(Math.sqrt(NUM_SPHERES * 1.6)));
          const rows = Math.max(1, Math.floor(NUM_SPHERES / cols));
          paramA = cols;
          paramB = rows;
        }
        ubo[24] = formation.mix;
        ubo[25] = formModeId;
        ubo[26] = paramA;
        ubo[27] = paramB;
        // u7: cellSdfOn, cellRadius, colorMode, hideLow
        ubo[28] = cellSdf.on ? 1.0 : 0.0;
        ubo[29] = cellSdf.radius;
        ubo[30] = cellSdf.colorMode;
        ubo[31] = cellSdf.hideLow ? 1.0 : 0.0;
        device.queue.writeBuffer(uniformBuf, 0, ubo);

        const enc  = device.createCommandEncoder();
        ensureDepth();
        const view      = gpuCtx.getCurrentTexture().createView();
        const depthView = depthTex.createView();

        {
          const pass = enc.beginRenderPass({
            colorAttachments: [{
              view, clearValue: { r: 0.02, g: 0.04, b: 0.09, a: 1 },
              loadOp: 'clear', storeOp: 'store',
            }],
            depthStencilAttachment: {
              view: depthView,
              depthClearValue: 1.0,
              depthLoadOp: 'clear',
              depthStoreOp: 'store',
            },
          });
          pass.setPipeline(bgPipeline);
          pass.setBindGroup(0, bindGroup);
          pass.draw(3);
          pass.end();
        }
        {
          const pass = enc.beginRenderPass({
            colorAttachments: [{
              view, loadOp: 'load', storeOp: 'store',
            }],
            depthStencilAttachment: {
              view: depthView,
              depthLoadOp: 'load',
              depthStoreOp: 'store',
            },
          });
          pass.setPipeline(spherePipeline);
          pass.setBindGroup(0, bindGroup);
          // 36 verts per instance: enough for either billboard quad (uses first 6,
          // discards 6..35) or cube mesh (uses all 36 for 6 axis-aligned faces).
          pass.draw(36, NUM_SPHERES);
          pass.end();
        }

        device.queue.submit([enc.finish()]);
        gpuRafId = requestAnimationFrame(frame);
      }

      gpuRafId = requestAnimationFrame(frame);
      // expose bench controls so HUD inline onclick can reach them
      window.__gpuBench      = bench;
      window.__gpuRunBench   = runBenchmark;
      console.log('%c[WebGPU] 🚀 v1.4 5M particles + benchmark', 'color:#67e8f9;font-weight:bold');
      setBadge(`✓ WebGPU · ${fmtN(NUM_SPHERES)} particles`, '#34d399');

      // ── 8. Cleanup ──────────────────────────────────────────────
      document.getElementById('close-chefos')?.addEventListener('click', () => {
        if (gpuRafId) { cancelAnimationFrame(gpuRafId); gpuRafId = null; }
        document.body.classList.remove('gpu-active');
        if (grad) grad.style.visibility = '';
        if (grid) grid.style.visibility = '';
        gpuCtx.unconfigure?.();
        window.removeEventListener('resize', resizeCanvas);
        window.removeEventListener('keydown', onKey);
        document.getElementById('bench-overlay')?.remove();
        hud?.remove();
        delete window.__gpuBench;
        delete window.__gpuRunBench;
      }, { once: true });
    }
"##
}
