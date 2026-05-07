// ── WGSL: signed-distance functions ─────────────────────────────────────────────
// Domain: Shape SDF library — cell SDF (particle_shape port) + superquadric family.

pub const WGSL: &str = r##"
// ─── kernel::particle_shape WGSL port ─────────────────────────
// Mirrors src/infrastructure/geometry/kernel/particle_shape.rs verbatim.
// Layout: bit 0 = +X, 1 = -X, 2 = +Y, 3 = -Y, 4 = +Z, 5 = -Z.
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
"##;
