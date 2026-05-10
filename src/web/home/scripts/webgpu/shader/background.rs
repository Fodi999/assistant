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

  // Чтобы получить идеальный антиалиасинг, мы должны использовать функцию fwidth()
  // до любых if/discard веток (uniform control flow). Вычисляем лучевое попадание заранее:
  let t_hit_uncond = -ro_eff.y / rd.y;
  let p_uncond = ro_eff + rd * t_hit_uncond;
  let dp_uncond = fwidth(p_uncond.xz); // Сколько мировых едениц за 1 пиксель экрана!

  // ── Floor at Y = 0 ──────────────────────────────────────────
  if (abs(rd.y) > 0.0005) {
    let t_hit = t_hit_uncond;
    if ((t_hit > 0.05) && (t_hit < 200.0)) {
      let p = p_uncond;
      let dp = dp_uncond;

      // Усиливаем видимость базовых линий
      let m = grid_intensity(p.xz, dp, u.u9.x * 1.0);
      // Добавляем крупные клетки (по 10 метров) чтобы даже вдали пол читался
      let dcm = grid_intensity(p.xz, dp, u.u9.x * 0.1);

      let combined_grid = clamp(m * 0.5 + dcm * 0.3, 0.0, 1.0);

      // Идеально ровное затухание в туман (как в Blender, fadeDistance = 150)
      let fade = smoothstep(150.0, 30.0, t_hit);

      // Светло-серая сетка
      let lineCol = vec3f(0.5, 0.5, 0.5); 
      var gridCol = mix(col, lineCol, combined_grid);

      // Тонкие минималистичные оси (Красная = X, Зеленая = Z)
      // Толщина ровно ~1 пиксель экрана (антиалиасинг от 0.0 до 1.2)
      let axis_x = 1.0 - smoothstep(0.0, 1.2, abs(p.z) / dp.y);
      let axis_z = 1.0 - smoothstep(0.0, 1.2, abs(p.x) / dp.x);
      
      // Смешиваем на 85% для большей утонченности (не такие "ядовито-режущие")
      // X = Red 
      gridCol = mix(gridCol, vec3f(0.85, 0.25, 0.25), axis_x * 0.85);
      // Z = Green 
      gridCol = mix(gridCol, vec3f(0.35, 0.75, 0.35), axis_z * 0.85);


      // Применяем сетку к фону
      col = mix(col, gridCol, fade);

      // --- 4. BLENDER FLOOR SHADOW (Падающая тень на пол) ---
      // Фейковая круглая тень прямо под центром мира (где стоит наш объект).
      // В Blender в Solid Mode полупрозрачная тень под объектом дает четкое ощущение плоскости.
      let shadowDist = length(vec2f(p.x, p.z));
      let shadowFade = smoothstep(1.3, 0.0, shadowDist); 
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
