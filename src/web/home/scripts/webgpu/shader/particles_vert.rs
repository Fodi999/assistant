// ── WGSL: particle vertex shader ─────────────────────────────────────────────────
// Domain: Particle placement — cloud drift, formation targets (cube/wall),
//         LOD decimation, billboard inflate, mesh-mode dispatch.

pub const WGSL: &str = r##"
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

  // ── Scene placement: scale & translate the entire formation ──
  // Cloud (formMix=0) keeps original position; the formed object is repositioned
  // and resized via uniform u8 = (objectPos, objectScale).
  let objPos   = u.u8.xyz;
  let objScale = max(0.001, u.u8.w);
  // blend toward the placed pose as the formation forms — keeps cloud anchored at origin
  let placeT   = smoothstep(0.0, 1.0, formMix);
  let placedC  = center * objScale + objPos;
  center       = mix(center, placedC, placeT);
  halfCell     = mix(halfCell, halfCell * objScale, placeT);

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
  // imposter radius also follows the scene scale once formed
  let sizeScaled = mix(size, size * objScale, placeT);
  let sizeLod    = sizeScaled * lodInflate * pxBoost;

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
"##;
