// ── WGSL: background pass — neutral viewport + axis lines ───────────────────
// Domain: Scene background — follows Blender/Maya conventions:
//   • flat grey gradient
//   • X-axis red, Z-axis blue (Y-up) — thin world-space axes only
//   • The grid is drawn by Canvas 2D overlay (controlled via CAD panel)

pub const WGSL: &str = r##"
@vertex fn vs_full(@builtin(vertex_index) vi: u32) -> Vert {
  var p = array<vec2f, 3>(
    vec2f(-1.0, -1.0),
    vec2f( 3.0, -1.0),
    vec2f(-1.0,  3.0)
  );
  var o: Vert;
  o.pos = vec4f(p[vi], 0.0, 1.0);
  o.uv  = p[vi] * 0.5 + 0.5;
  return o;
}

struct BgFragOut {
  @location(0) col: vec4f,
  @builtin(frag_depth) depth: f32,
}

@fragment fn fs_bg(in: Vert) -> BgFragOut {
  let uv  = vec2f(in.uv.x, 1.0 - in.uv.y);
  let asp = u.u0.y / max(u.u0.z, 1.0);

  // ── Neutral viewport gradient (Blender 4.x default) ─────────
  let topCol = vec3f(0.235, 0.235, 0.245);
  let botCol = vec3f(0.105, 0.105, 0.115);
  var col    = mix(botCol, topCol, uv.y);

  // ── Camera basis from uniforms ──────────────────────────────
  let ro    = u.u1.xyz;
  let right = u.u2.xyz;
  let upv   = u.u3.xyz;
  let fwd   = u.u4.xyz;

  // Pixel ray (NDC, aspect-corrected, 45° fov → focal length 2.414)
  let isOrtho = u.u9.y > 0.5;

  let ndcX = (uv.x * 2.0 - 1.0) * asp;
  let ndcY = -(uv.y * 2.0 - 1.0);
  let fl   = 2.414;
  
  var ro_eff = ro;
  var rd     = normalize(fwd * fl + right * ndcX + upv * ndcY);

  if isOrtho {
    // Determine the viewport scale based on camera distance.
    // To match perspective visually, orthogonal size scales with distance.
    let orthoHeight = distance(ro, u.u8.xyz) * 0.45; 
    
    // Shift the ray origin offset by NDC to simulate parallel rays.
    ro_eff = ro + right * ndcX * orthoHeight + upv * ndcY * orthoHeight;
    rd     = normalize(fwd);
  }

  var out: BgFragOut;
  out.col = vec4f(col, 1.0);
  // Записываем глубину 0.99999, чтобы пройти тест depthCompare: 'less' (где фон очищен в 1.0)
  out.depth = 0.99999;

  // ── Grid Plane Logic (Dynamic axes) ─────────────────────────
  let draw_grid = u.u14.x > 0.5;
  let plane_id = u.u14.y; // 0.0 = XZ (floor), 1.0 = XY (front), 2.0 = YZ (side)

  // Calcs for all planes unconditionally to ensure fwidth works properly
  let t_xz = -ro_eff.y / rd.y;
  let p_xz = ro_eff + rd * t_xz;
  let dp_xz = fwidth(p_xz.xz);

  let t_xy = -ro_eff.z / rd.z;
  let p_xy = ro_eff + rd * t_xy;
  let dp_xy = fwidth(p_xy.xy);

  let t_yz = -ro_eff.x / rd.x;
  let p_yz = ro_eff + rd * t_yz;
  let dp_yz = fwidth(p_yz.yz);

  // Select variables based on active plane
  let t_hit = select(t_xz, select(t_xy, t_yz, plane_id > 1.5), plane_id > 0.5);
  let p     = select(p_xz, select(p_xy, p_yz, plane_id > 1.5), plane_id > 0.5);
  let dp    = select(dp_xz, select(dp_xy, dp_yz, plane_id > 1.5), plane_id > 0.5);
  let rd_param=select(rd.y, select(rd.z, rd.x, plane_id > 1.5), plane_id > 0.5);
  
  let grid_coords = select(p.xz, select(p.xy, p.yz, plane_id > 1.5), plane_id > 0.5);

  if (draw_grid && abs(rd_param) > 0.0005) {
    if ((t_hit > 0.05) && (t_hit < 1500.0)) {

      // ── Тонкие оси координат (X=красный, Z/Y=синий/зелёный) ──
      let axis_a = 1.0 - smoothstep(0.0, 1.2, abs(grid_coords.y) / dp.y);
      let axis_b = 1.0 - smoothstep(0.0, 1.2, abs(grid_coords.x) / dp.x);

      var color_a = vec3f(0.85, 0.25, 0.25);
      var color_b = vec3f(0.30, 0.53, 0.84);

      if (plane_id < 0.5) { // XZ
          color_a = vec3f(0.85, 0.25, 0.25); // X red
          color_b = vec3f(0.30, 0.53, 0.84); // Z blue
      } else if (plane_id < 1.5) { // XY
          color_a = vec3f(0.85, 0.25, 0.25); // X red
          color_b = vec3f(0.35, 0.75, 0.35); // Y green
      } else { // YZ
          color_a = vec3f(0.35, 0.75, 0.35); // Y green
          color_b = vec3f(0.30, 0.53, 0.84); // Z blue
      }

      let fade = smoothstep(40.0, 3.0, t_hit);
      col = mix(col, color_a, axis_a * 0.85 * fade);
      col = mix(col, color_b, axis_b * 0.85 * fade);

      out.col = vec4f(col, 1.0);
    }
  }

  return out;
}

fn hitSphere(ro: vec3f, rd: vec3f, c: vec3f, r: f32) -> f32 {
  let oc = ro - c;
  let b  = dot(oc, rd);
  let cc = dot(oc, oc) - r * r;
  let h  = b * b - cc;
  if (h < 0.0) { return -1.0; }
  return -b - sqrt(h);
}
"##;
