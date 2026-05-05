pub fn webgpu_js() -> &'static str {
    r##"
    // ══════════════════════════════════════════════════════════════
    // WebGPU Scene v1.4 — Up to 5M Particles + Benchmark Suite
    //   • drag / shift+drag  — orbit / pan camera
    //   • wheel              — zoom 0.5…80
    //   • mouse move         — sand-cursor push
    //   • 1-5                — 1K / 10K / 100K / 500K / 1M
    //   • + / -              — ×1.5 fine
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
      // shape parameter: 0 = cube · 0.5 = rounded cube · 1 = sphere
      const shape = { roundness: 1.0 };
      // map slider [0..1] → exponent n for superellipse |x|ⁿ + |y|ⁿ ≤ 1
      // r=1.0 → n=2 (circle) · r=0.5 → n≈7 (squircle) · r=0.0 → n=22 (square)
      function shapeExponent(r) { return 2.0 + 20.0 * Math.pow(1.0 - r, 2.0); }

      // ── Formation state ────────────────────────────────────────
      // mode: 'cloud' | 'cube' | 'wall'
      // mix:  0 (cloud) → 1 (fully formed); animated each frame
      const formation = { mode: 'cloud', target: 0.0, mix: 0.0 };
      function setFormation(mode) {
        formation.mode   = mode;
        formation.target = (mode === 'cloud') ? 0.0 : 1.0;
        log(`◇ formation = ${mode}`, '#f0abfc');
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
      // Uniform layout (7 × vec4 = 112 bytes):
      //   u0: time, w, h, pushStrength
      //   u1: roX, roY, roZ, _
      //   u2: rightX, rightY, rightZ, _
      //   u3: upX, upY, upZ, _
      //   u4: fwdX, fwdY, fwdZ, _
      //   u5: mouseX, mouseY, mouseActive, shapeExponent
      //   u6: formMix(0..1), formMode(0=cloud,1=cube,2=wall), formA, formScale
      const uniformBuf = device.createBuffer({
        size: 112,
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
  let n         = max(u.u5.w, 2.0);
  let cubeness  = clamp((n - 2.0) / 20.0, 0.0, 1.0); // 0 sphere → 1 cube
  let coverK    = mix(1.45, 1.02, cubeness);

  var formed     = vec3f(0.0);
  var targetR    = sp.posR.w;     // target radius when fully formed
  var aliveForm  = true;          // false → particle hidden (extra over grid capacity)

  if formMode > 0.5 && formMode < 1.5 {
    // CUBE: 6 faces of side·side cells → totalCells = 6·side²
    let side       = u32(formA);            // formA is floor(√(N/6)) from JS
    let totalCells = 6u * side * side;
    if inst < totalCells {
      let face = inst % 6u;
      let k    = inst / 6u;
      let r    = k % side;
      let c    = (k / side) % side;
      let uu   = (f32(r) + 0.5) / formA * 2.0 - 1.0;
      let vv   = (f32(c) + 0.5) / formA * 2.0 - 1.0;
      var p: vec3f;
      if      face == 0u { p = vec3f( 1.0,  vv,   uu  ); }
      else if face == 1u { p = vec3f(-1.0,  vv,  -uu  ); }
      else if face == 2u { p = vec3f( uu,   1.0,  vv  ); }
      else if face == 3u { p = vec3f( uu,  -1.0, -vv  ); }
      else if face == 4u { p = vec3f(-uu,   vv,   1.0 ); }
      else               { p = vec3f( uu,   vv,  -1.0 ); }
      formed  = p * formScale;
      // half-cell on a face = formScale / side
      targetR = formScale / formA * coverK;
    } else {
      // overflow → hide inside the cube (size 0)
      aliveForm = false;
      formed    = vec3f(0.0);
      targetR   = 0.0;
    }
  } else if formMode > 1.5 {
    // WALL: cols × rows  (cols·rows ≤ N is guaranteed from JS)
    let cols       = u32(formA);
    let rows       = max(formScale, 1.0);
    let rowsU      = u32(rows);
    let totalCells = cols * rowsU;
    if inst < totalCells {
      let r      = inst / cols;
      let c      = inst % cols;
      let uu     = (f32(c) + 0.5) / formA * 2.0 - 1.0;
      let vv     = (f32(r) + 0.5) / rows  * 2.0 - 1.0;
      let scale  = 2.4;
      let aspect = formA / rows;
      formed     = vec3f(uu * aspect * scale, vv * scale, 0.0);
      targetR    = scale / rows * coverK;
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
      center += dir * falloff * u.u0.w * (1.0 - 0.7 * formMix);
    }
  }

  // view-space transform
  let rel = center - ro;
  let vx  = dot(rel, right);
  let vy  = dot(rel, upv);
  let vz  = dot(rel, fwd);

  if vz < 0.05 {
    var dead: Pv;
    dead.pos     = vec4f(0.0, 0.0, -2.0, 1.0);
    dead.quadUV  = vec2f(0.0);
    dead.color   = vec3f(0.0);
    dead.depth   = 0.0;
    dead.phase   = 0.0;
    dead.wCenter = vec3f(0.0);
    dead.size    = 0.0;
    return dead;
  }

  // billboard quad scaled to particle radius (world units)
  // size already lerped above between spawn radius and target cell radius
  let lx   = qx[vi] * size;
  let ly   = qy[vi] * size;

  // perspective project
  let focal = 1.5;
  let cx = (vx + lx) * focal / vz / asp;
  let cy = (vy + ly) * focal / vz;

  // map view-space z (≥0.05) to NDC depth [0..1) monotonically — used
  // only as a coarse sort key; the fragment shader writes the precise
  // per-pixel depth based on the actual ray-sphere hit point.
  let zNdc = clamp(vz / (vz + 8.0), 0.0, 0.9999);

  var o: Pv;
  o.pos     = vec4f(cx, cy, zNdc, 1.0);
  o.quadUV  = vec2f(qx[vi], qy[vi]);
  o.color   = sp.colorP.xyz;
  o.depth   = vz;
  o.phase   = ph;
  o.wCenter = center;
  o.size    = size;
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
fn sdSuperq(p: vec3f, n: f32) -> f32 {
  let q = abs(p);
  let v = pow(q.x, n) + pow(q.y, n) + pow(q.z, n);
  return pow(v, 1.0 / n) - 1.0;
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

  let pixelW = p.wCenter + right * p.quadUV.x * p.size + upv * p.quadUV.y * p.size;
  let rd     = normalize(pixelW - ro);

  let n = max(u.u5.w, 2.0);
  let sphereLikeness = clamp((22.0 - n) / 20.0, 0.0, 1.0); // 1 at n=2, 0 at n=22

  var nrm:   vec3f;
  var hitW:  vec3f;
  let R                  = p.size;

  if n < 3.5 {
    // ── analytical sphere intersection (sphere mode) ──
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
    let rot = rotMat(p.phase);
    // transform ray to local space: local = rot · (world - center) / R
    let roL = rot * (ro - p.wCenter) / R;
    let rdL = rot * rd;

    // bounding-sphere entry to skip empty space
    let bL = dot(roL, rdL);
    let cL = dot(roL, roL) - 1.732 * 1.732;   // bounding sphere radius √3 for unit cube
    let hL = bL * bL - cL;
    if hL < 0.0 { discard; }
    var tCur = max(0.0, -bL - sqrt(hL));
    let tEnd = -bL + sqrt(hL);

    var marched = false;
    var tFinal: f32 = tCur;
    for (var i = 0; i < 14; i++) {
      let pL = roL + rdL * tCur;
      let d  = sdSuperq(pL, n);
      if d < 0.0008 { marched = true; tFinal = tCur; break; }
      tCur += max(d * 0.85, 0.003);
      if tCur > tEnd { break; }
    }
    if !marched { discard; }

    // local-space normal from SDF gradient (central diff)
    let pL  = roL + rdL * tFinal;
    let eps = 0.002;
    let gx  = sdSuperq(pL + vec3f(eps,0,0), n) - sdSuperq(pL - vec3f(eps,0,0), n);
    let gy  = sdSuperq(pL + vec3f(0,eps,0), n) - sdSuperq(pL - vec3f(0,eps,0), n);
    let gz  = sdSuperq(pL + vec3f(0,0,eps), n) - sdSuperq(pL - vec3f(0,0,eps), n);
    let nL  = normalize(vec3f(gx, gy, gz));
    // back to world space: transpose(rot) = inverse for rotation matrices
    nrm  = transpose(rot) * nL;
    // world hit = ro + rd * (tFinal in world units = tFinal * R) — because rdL = rot*rd is unit-length
    // and we scaled positions by 1/R. The parameter in world space therefore equals tFinal * R.
    hitW = ro + rd * (tFinal * R);
  }

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

  let hitVz = max(dot(hitW - ro, fwd), 0.05);
  let t     = u.u0.x;
  let pulse = 0.85 + 0.15 * sin(t * 2.0 + p.phase);
  let fog   = exp(-hitVz * 0.045);

  var col = p.color * 0.14;                                   // ambient
  col    += p.color * (dA + dB) * (0.65 + 0.20 * pulse);      // diffuse
  col    += vec3f(0.95, 0.97, 1.0) * sp * 0.55;               // specular
  col    += vec3f(0.10, 0.65, 1.00) * fr * (0.30 + 0.20 * sphereLikeness);

  // subtle emissive bias so far particles do not turn pitch black
  col    += p.color * 0.05 * pulse;

  // distance darkening (atmospheric, NOT alpha)
  col *= 0.55 + 0.45 * fog;

  // tone map + gamma
  col = col / (col + vec3f(1.0));
  col = pow(col, vec3f(0.4545));

  // ── per-pixel depth from real hit point (matches vertex z mapping) ──
  let zNdc = clamp(hitVz / (hitVz + 8.0), 0.0, 0.9999);

  var out: FragOut;
  out.color = vec4f(col, 1.0);   // fully opaque
  out.depth = zNdc;
  return out;
}
`;

      const module = device.createShaderModule({ code: shaderSrc });
      log('✓ WGSL скомпилирован', '#34d399');

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
        const shapeLabel =
          r >= 0.85 ? 'sphere'    :
          r >= 0.65 ? 'capsule'   :
          r >= 0.40 ? 'squircle'  :
          r >= 0.20 ? 'rounded'   : 'cube';
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
          `1·2·3·4·5 = 1K/10K/100K/500K/1M · C/V/N · [/] shape · R rotate</div>`;
        // wire the slider after each rebuild
        const slider = document.getElementById('gpu-shape-slider');
        if (slider) slider.oninput = (e) => { shape.roundness = parseFloat(e.target.value); };
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
          case '[': shape.roundness = Math.max(0, shape.roundness - 0.1); break;
          case ']': shape.roundness = Math.min(1, shape.roundness + 0.1); break;
          case 'c': case 'C': setFormation('cloud'); break;
          case 'v': case 'V': setFormation('cube');  break;
          case 'n': case 'N': setFormation('wall');  break;
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
      const ubo = new Float32Array(28); // 7 × vec4

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
        }
        // u6: formMix, formMode, paramA, paramB
        //   cube: paramA = grid side per face (floor √(N/6)), paramB = scale
        //   wall: paramA = cols,                              paramB = rows  (cols·rows ≤ N)
        let formModeId = 0, paramA = 1, paramB = 1.6;
        if (formation.mode === 'cube') {
          formModeId = 1;
          // floor: 6·side² ≤ N → every cell on every face is occupied,
          // leftover (≤ 6·side² − 6·(side−1)²) particles are hidden in shader.
          paramA = Math.max(1, Math.floor(Math.sqrt(NUM_SPHERES / 6)));
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
          pass.draw(6, NUM_SPHERES);
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
