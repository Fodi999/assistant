// ── WGSL: particle fragment shader ───────────────────────────────────────────────
// Domain: Particle shading — rotation matrix, ray-march (cell/superquadric/sphere),
//         mesh-mode flat shading, lighting, seam suppression, debug overrides.

pub const WGSL: &str = r##"
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

struct FragOut {
  @location(0)              color: vec4f,
  @builtin(frag_depth)      depth: f32,
}

@fragment fn fs_particles(p: Pv, @builtin(front_facing) is_front: bool) -> FragOut {
  // Compute derivatives at the very top before any discards to preserve uniform control flow
  let vx = fwidth(p.quadUV.x);
  let vy = fwidth(p.quadUV.y);
  
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
  // CAD MATTE MATERIAL (Plasticity-style)
  // Soft, matte gradient lighting:
  let L1 = normalize(vec3f( 0.45,  0.85, -0.40));
  let L2 = normalize(vec3f(-0.35,  0.25,  0.65));
  let L3 = normalize(vec3f( 0.10, -0.50, -0.20)); // Soft bounce light

  let v  = -rd;
  
  // Soften lambert with wrapping (valve half-lambert style)
  let dA = pow(max(dot(nrm, L1) * 0.5 + 0.5, 0.0), 1.5) * 1.1; 
  let dB = max(dot(nrm, L2), 0.0) * 0.35;
  let dC = max(dot(nrm, L3), 0.0) * 0.20;

  // Reduced specular
  let h  = normalize(L1 + v);
  let sp = pow(max(dot(nrm, h), 0.0), 32.0) * 0.15; // low shine

  // Soft rim
  let fr = pow(1.0 - max(dot(nrm, v), 0.0), 2.5) * 0.15;

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
  let fog   = exp(-hitVz * 0.045);

  let cad_matte_base = vec3f(0.85, 0.88, 0.90); // light gray-blue CAD material

  var col = p.color * 0.25;                                    // ambient
  col    += cad_matte_base * (dA + dB + dC) * 0.85;                // matte diffuse
  col    += vec3f(0.95, 0.97, 1.0) * sp * seamMul;                 // subtle specular 
  col    += vec3f(0.80, 0.90, 1.00) * fr * (0.80 + 0.20 * sphereLikeness) * seamMul;

  // distance darkening (atmospheric, NOT alpha)
  col *= 0.65 + 0.35 * fog;

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

  // ── CAD/Plasticity Edge Overlay (mesh mode only) ──
  if p.meshMode == 1u {
    let fw = max(vx, vy);
    
    let dU = 1.0 - abs(p.quadUV.x);
    let dV = 1.0 - abs(p.quadUV.y);
    let dEdge = min(dU, dV);
    
    // 1.2 pixels wide edge, with anti-aliasing
    let edgeT = smoothstep(fw * 1.8, fw * 0.4, dEdge);
    let edgeColor = vec3f(0.12, 0.14, 0.17); // Softer edge color, less black
    
    col = mix(col, edgeColor, edgeT);
  }

  var out: FragOut;
  out.color = vec4f(col, 1.0);   // fully opaque
  out.depth = zNdc;
  return out;
}
"##;
