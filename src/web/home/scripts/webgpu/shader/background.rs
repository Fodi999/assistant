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

// ── Фрактальная сетка с авто-сокрытием ───────────────────────
// Плавно скрывает линии, если они становятся "слишком густыми"
fn grid_intensity(p: vec3f, scale: f32, world_lw: f32) -> f32 {
  let cell_lw = world_lw * scale; 
  let opacity = smoothstep(1.5, 0.05, cell_lw); // expanded fade range
  let clamped_lw = min(cell_lw, 0.5);
  
  // fract(x + 0.5) - 0.5 centers the grid lines exactly on integer matching: 0.0, 1.0, 2.0
  let mx = abs(fract(p.x * scale + 0.5) - 0.5);
  let mz = abs(fract(p.z * scale + 0.5) - 0.5);
  
  return max(
    1.0 - smoothstep(0.0, clamped_lw, mx),
    1.0 - smoothstep(0.0, clamped_lw, mz)
  ) * opacity;
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
  let ndcX = (uv.x * 2.0 - 1.0) * asp;
  let ndcY = -(uv.y * 2.0 - 1.0);
  let fl   = 2.414;
  let rd   = normalize(fwd * fl + right * ndcX + upv * ndcY);

  var out: BgFragOut;
  out.col = vec4f(col, 1.0);
  out.depth = 1.0;

  // ── Floor at Y = 0 ──────────────────────────────────────────
  // Works from both sides — camera can be above OR below Y=0.
  if (abs(rd.y) > 0.0005) {
    let t_hit = -ro.y / rd.y;
    if ((t_hit > 0.05) && (t_hit < 200.0)) {
      let p = ro + rd * t_hit;

      // Вычисляем реальную толщину линии в мировых координатах (~1.5px экрана)
      let world_lw = max((t_hit / 2.414) * (2.0 / 900.0) * 1.5, 0.0001);

      // Иерархия масштабов с учетом настройки UI (u.u9.x):
      // UI scale 1.0 (метры)      => u9.x = 1.0
      // UI scale 100.0 (сантиметры) => u9.x = 100.0
      // UI scale 1000.0 (миллиметры) => u9.x = 1000.0
      let s = u.u9.x;
      // Усиливаем видимость линий (были слишком слабые: 0.05, 0.12...)
      let mm  = grid_intensity(p, s * 1000.0, world_lw) * 0.15;
      let cm  = grid_intensity(p, s * 100.0,  world_lw) * 0.35;
      let dcm = grid_intensity(p, s * 10.0,   world_lw) * 0.50;
      let m   = grid_intensity(p, s * 1.0,    world_lw) * 0.80;

      let combined_grid = clamp(mm + cm + dcm + m, 0.0, 1.0);

      // smooth distance fade (gone by 70 units)
      let fade = clamp(1.0 - t_hit / 70.0, 0.0, 1.0);

      // Сначала рассчитываем яркое пятно под объектом
      let objPos = u.u8.xyz;
      let dObj = length(p.xz - objPos.xz);
      let platformGlow = exp(-dObj * dObj * 1.85);
      let platformCore = exp(-dObj * dObj * 8.0);

      var floorCol = col;
      floorCol += vec3f(0.00, 0.65, 1.00) * platformGlow * 0.45;
      floorCol += vec3f(0.40, 0.95, 1.00) * platformCore * 0.55;

      // Затем рисуем сетку ПОВЕРХ светящегося пола более ярким цветом
      let lineCol = vec3f(0.20, 0.45, 0.75); // чуть более светлый сине-серый
      var gridCol = mix(floorCol, floorCol + lineCol, combined_grid); // Additive смесь, чтобы линии не темнили глоу

      // Яркая подсветка центральных осей (Красный = X, Синий = Z)
      let axis_x = 1.0 - smoothstep(world_lw * 0.5, world_lw * 2.0, abs(p.z));
      let axis_z = 1.0 - smoothstep(world_lw * 0.5, world_lw * 2.0, abs(p.x));
      gridCol = mix(gridCol, vec3f(1.00, 0.25, 0.35), axis_x * 0.85);
      gridCol = mix(gridCol, vec3f(0.20, 0.55, 1.00), axis_z * 0.85);

      // Применяем сетку к фону
      col = mix(col, gridCol, fade);

      out.col = vec4f(col, 1.0);
      let hitVz = max(dot(p - ro, fwd), 0.05);
      out.depth = clamp(hitVz / (hitVz + 8.0), 0.0, 0.9999);
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
