pub const WGSL: &str = r##"
// ── CAD Solid pipeline vertex shader ──
// Positions arrive in world-space metres from the geometry kernel.
// We project them through the scene camera without any extra object transform.
//
// CAD Visual Style v1:
//   face_id=1  top cap    → slightly lighter
//   face_id=2  bottom cap → slightly darker
//   face_id=3  side walls → base CAD grey
//   face_id=0  unknown    → base CAD grey
//
// TODO(v2): receive per-vertex material color from GPU buffer once
//           geometry_engine MeshPart gains a Material field.

@vertex fn vs_cad(
  @location(0) position: vec3f,
  @location(1) normal:   vec3f,
  @location(2) face_id:  u32,
) -> Pv {
  let ro          = u.u1.xyz;
  let right       = u.u2.xyz;
  let upv         = u.u3.xyz;
  let fwd         = u.u4.xyz;
  let isOrtho     = u.u9.y > 0.5;
  let orthoHeight = distance(ro, u.u8.xyz) * 0.45;
  let asp         = u.u0.y / u.u0.z;   // w / h

  // --- world-space camera projection (identical to background / particles) ---
  let relV  = position - ro;
  let mvx   = dot(relV, right);
  let mvy   = dot(relV, upv);
  let mvz   = dot(relV, fwd);
  let smvz  = max(mvz, 0.001);
  let focal = 2.414;
  let cx    = select(mvx * focal / smvz / asp, mvx / orthoHeight / asp, isOrtho);
  let cy    = select(mvy * focal / smvz,       mvy / orthoHeight,       isOrtho);
  let zNdc  = clamp(mvz / (mvz + 8.0), 0.0, 0.9999);

  // ── CAD face tint by face_id (Visual Style v1) ───────────────────────────
  // Base: calm neutral CAD grey (per requirements: ~0.72-0.80)
  var base_col = vec3f(0.72, 0.76, 0.80);   // sides / unknown — base value
  if face_id == 1u {
    // top cap — +5% lighter (light from above)
    base_col = vec3f(0.756, 0.798, 0.840);
  } else if face_id == 2u {
    // bottom cap — -8% darker (shadowed underside)
    base_col = vec3f(0.662, 0.699, 0.736);
  }
  // face_id == 3u (sides) and 0u (unknown) keep the base value

  var o: Pv;
  o.pos      = vec4f(cx, cy, zNdc, 1.0);
  o.quadUV   = vec2f(0.0);
  o.color    = base_col;
  o.depth    = mvz;
  o.phase    = 0.0;
  o.wCenter  = position;
  o.size     = 0.0;
  o.cellMask = face_id;
  o.halfCell = 0.0;
  o.meshMode = 1u;
  o.meshN    = normalize(normal);
  return o;
}
"##;
