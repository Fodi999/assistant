// ── WGSL: background pass — neutral viewport + clean floor grid ─────────────────
// Domain: Scene background — follows Blender/Maya/Three.js conventions:
//   • flat grey gradient (no nebulae, no horizon glow)
//   • infinite Y=0 floor plane with screen-space-derivative anti-aliasing
//   • minor grid every 1 unit, major every 10 units
//   • X-axis red, Z-axis blue (Y-up)
//   • smooth distance fade — no shimmering at the horizon

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

// ── Фрактальная сетка с аналитическим антиалиасингом (Evan Wallace) ──
fn grid_intensity(xz: vec2f, dp: vec2f, scale: f32) -> f32 {
  let coord = xz * scale;
  let deriv = dp * scale;
  
  // Расстояние до линии сетки с учетом ее размера на экране
  let grid = abs(fract(coord - 0.5) - 0.5) / deriv;
  
  // Толщина линии около 1.5 пикселя на экране
  let lw = 1.5;
  let l = min(grid.x, grid.y);
  let alpha = 1.0 - min(l / lw, 1.0);
  
  // Если масштаб приближается к размеру пикселя (deriv > 0.5),
  // линии сливаются в кашу. Чтобы не было Муара (ряби), плавно их прячем.
  let fade = 1.0 - smoothstep(0.1, 0.4, max(deriv.x, deriv.y));
  
  return alpha * fade;
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
      // Увеличили дальность видимости пола для чертежных метрик
    if ((t_hit > 0.05) && (t_hit < 1500.0)) {

      // Сетка: Для метров m = каждые 1 м, dcm = каждые 0.1 м.
      // u.u9.x это масштаб сетки (по умолчанию 1.0).
      let scaleMult = select(1.0, 0.1, u.u9.x > 10.0); 
      let m = grid_intensity(grid_coords, dp, scaleMult * 1.0); 
      let dcm = grid_intensity(grid_coords, dp, scaleMult * 10.0);

      let combined_grid = clamp(m * 0.4 + dcm * 0.6, 0.0, 1.0);

      // Идеально ровное затухание в туман (как в Blender) 
      // Изменяем дистанцию затухания под CAD метрики - дистанция камеры ~6.4
      let fade = smoothstep(40.0, 3.0, t_hit);

      // Светло-серая сетка
      let lineCol = vec3f(0.5, 0.5, 0.5); 
      var gridCol = mix(col, lineCol, combined_grid);

      // Тонкие минималистичные оси (динамически раскрашиваем в зависимости от плоскости)
      // В XZ: p.z (green, Z as X in grid?), p.x (red, X as Y in grid?) 
      // dp.x and dp.y roughly corresponds to fwidth on the axes.
      let axis_a = 1.0 - smoothstep(0.0, 1.2, abs(grid_coords.y) / dp.y);
      let axis_b = 1.0 - smoothstep(0.0, 1.2, abs(grid_coords.x) / dp.x);
      
      var color_a = vec3f(0.85, 0.25, 0.25); // Red (X by default)
      var color_b = vec3f(0.35, 0.75, 0.35); // Green (Y by default)
      
      if (plane_id < 0.5) { // XZ
          // X = Red, Z = Blue (#4d87d6)
          color_a = vec3f(0.85, 0.25, 0.25); // X
          color_b = vec3f(0.30, 0.53, 0.84); // Z
      } else if (plane_id < 1.5) { // XY
          // X = Red, Y = Green
          color_a = vec3f(0.85, 0.25, 0.25); // X
          color_b = vec3f(0.35, 0.75, 0.35); // Y
      } else { // YZ
          // Y = Green, Z = Blue
          color_a = vec3f(0.35, 0.75, 0.35); // Y
          color_b = vec3f(0.30, 0.53, 0.84); // Z
      }

      gridCol = mix(gridCol, color_a, axis_a * 0.85);
      gridCol = mix(gridCol, color_b, axis_b * 0.85);

      // Применяем сетку к фону
      col = mix(col, gridCol, fade);

      // --- 4. BLENDER FLOOR SHADOW (Падающая тень на пол) ---
      // Фейковая круглая тень прямо под центром мира (где стоит наш объект).
      // В Blender в Solid Mode полупрозрачная тень под объектом дает четкое ощущение плоскости.
      // Адаптируем размер тени под габариты (u.u12.x, u.u12.z).
      let objScaleX = max(0.01, u.u12.x);
      let objScaleZ = max(0.01, u.u12.z);
      let shadowRadius = max(objScaleX, objScaleZ) * 0.65;
      
      let shadowDist = length(vec2f(p.x - u.u8.x, p.z - u.u8.z)); // сдвиг за у8.xyz
      let shadowFade = smoothstep(shadowRadius, 0.0, shadowDist); 
      col = mix(col, col * 0.25, shadowFade * 0.65); // Мягко затемняем пол

      out.col = vec4f(col, 1.0);
      // Глубину оставляем 1.0 (глубина фона), чтобы сетка и пол не перекрывали объект снизу
      // out.depth = ... 
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
