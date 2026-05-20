pub const WGSL: &str = r##"
// ── CAD Solid pipeline vertex shader ──
// Positions arrive in world-space metres from the geometry kernel.
// We project them through the scene camera without any extra object transform.

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

  var o: Pv;
  o.pos      = vec4f(cx, cy, zNdc, 1.0);
  o.quadUV   = vec2f(0.0);
  o.color    = vec3f(0.72, 0.78, 0.85);   // neutral CAD grey
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
