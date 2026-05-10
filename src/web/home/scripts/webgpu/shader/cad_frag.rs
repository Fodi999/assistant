pub const WGSL: &str = r##"
// ── CAD / Solid mode fragment shader ──
// Blender 3-point studio lighting, no raymarching.

struct FragOut {
  @location(0)         color: vec4f,
  @builtin(frag_depth) depth: f32,
}

@fragment fn fs_cad(p: Pv, @builtin(front_facing) is_front: bool) -> FragOut {
  let L1  = normalize(vec3f( 0.50,  0.80,  0.50));
  let L2  = normalize(vec3f(-0.80, -0.20, -0.20));
  let L3  = normalize(vec3f( 0.20, -0.50, -0.80));
  let nrm = select(-p.meshN, p.meshN, is_front);
  let ro  = u.u1.xyz;
  let fwd = u.u4.xyz;
  let rd  = normalize(p.wCenter - ro);
  let v   = -rd;
  let dA  = max(dot(nrm, L1), 0.0) * 0.80;
  let dB  = max(dot(nrm, L2), 0.0) * 0.35;
  let dC  = max(dot(nrm, L3), 0.0) * 0.25;
  let h   = normalize(L1 + v);
  let sp  = pow(max(dot(nrm, h), 0.0), 50.0) * 0.15;
  let fr  = pow(1.0 - max(dot(nrm, v), 0.0), 3.0) * 0.10;
  var col = p.color * (0.15 + dA + dB + dC);
  col    += vec3f(1.0) * sp;
  col    += vec3f(1.0) * fr;
  col     = pow(max(col, vec3f(0.0)), vec3f(1.0 / 2.2));
  let hitVz = max(dot(p.wCenter - ro, fwd), 0.05);
  let zNdc  = clamp(hitVz / (hitVz + 8.0), 0.0, 0.9999);
  
  // Добавляем Blender Solid подсветку при выделении
  let isSelected = u.u9.z > 0.5;
  if isSelected {
    // В Blender контур при выделении оранжевый (#FF9900)
    let dU = 1.0 - abs(p.quadUV.x);
    let dV = 1.0 - abs(p.quadUV.y);
    let pixelDist = min(dU, dV);
    if pixelDist < 0.05 {
      col = mix(col, vec3f(1.0, 0.6, 0.0), 0.8);
    }
  }

  var out: FragOut;
  out.color = vec4f(col, 1.0);
  out.depth = zNdc;
  return out;
}
"##;
