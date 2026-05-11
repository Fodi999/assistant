pub const WGSL: &str = r##"
// ── CAD / Solid mode fragment shader ──
// Raymarching с точными миллиметровыми размерами (Dimensions) и фасками (Bevel)

struct FragOut {
  @location(0)         color: vec4f,
  @builtin(frag_depth) depth: f32,
}

// Математическая функция 2D профиля (скругленный прямоугольник по XY)
fn sdRoundBox2D(p: vec2f, b: vec2f, r: f32) -> f32 {
  let q = abs(p) - b + vec2f(r);
  return length(max(q, vec2f(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

// Точная математическая копия алгоритма Rust Backend (Extrude Profile + Bevel Chamfer)
fn sdBackendShape(p: vec3f, b: vec3f, bevel: f32) -> f32 {
  // 1. Двумерный скругленный профиль по XY (как делает rounded_rect_points)
  let d2 = sdRoundBox2D(p.xy, b.xy, bevel);
  
  // 2. Расстояние по Z (Глубина / extrusion depth)
  let dz = abs(p.z) - b.z;
  
  // 3. Базовая дистанция вытесненного 3D объекта (Extrude) 
  let w = vec2f(max(d2, 0.0), max(dz, 0.0));
  let d_ext = min(max(d2, dz), 0.0) + length(w);
  
  // 4. Плоскость 45-градусной фаски на торцах Z (Chamfer), как в extrude_polygon
  let chamfer = (d2 + dz + bevel) * 0.70710678;
  
  // Пересекаем прямые грани с плоскостью снятой фаски
  return max(d_ext, chamfer);
}

@fragment fn fs_cad(p: Pv, @builtin(front_facing) is_front: bool) -> FragOut {
  // Вычисляем производную от ID грани в screen-space (работает внутри 2x2 пиксельных блоков).
  // Там, где пиксели соседних полигонов с разным face_id встречаются, fwidth будет > 0.
  let is_edge = fwidth(f32(p.cellMask)) > 0.001;

  let ro  = u.u1.xyz;
  let dims = max(vec3f(0.001), u.u12.xyz); // Габариты в мм
  let bevel = min(u.u13.x, min(dims.x, min(dims.y, dims.z)) * 0.5); // Ограничим радиус половиной толщины
  
  let fwd = u.u4.xyz;
  let right = u.u2.xyz;
  let upv = u.u3.xyz;
  
  let asp = u.u0.y / u.u0.z;
  let ndcX = (p.pos.x / u.u0.y * 2.0 - 1.0) * asp;
  let ndcY = -(p.pos.y / u.u0.z * 2.0 - 1.0);
  
  let isOrtho = u.u9.y > 0.5;
  let orthoHeight = distance(ro, u.u8.xyz) * 0.45;
  let fl = 2.414;
  
  var ro_eff = ro;
  var rd     = normalize(fwd * fl + right * ndcX + upv * ndcY);

  if isOrtho {
    ro_eff = ro + right * ndcX * orthoHeight + upv * ndcY * orthoHeight;
    rd     = normalize(fwd);
  }

  // Используем переданную нормаль от меша, но если она нулевая (пока так), 
  // используем flat shading
  var nrm = p.meshN;
  if length(nrm) < 0.1 {
    nrm = vec3f(0.0, 1.0, 0.0);
  }
  
  let L1  = normalize(vec3f( 0.50,  0.80,  0.50));
  let L2  = normalize(vec3f(-0.80, -0.20, -0.20));
  let L3  = normalize(vec3f( 0.20, -0.50, -0.80));
  
  let v   = -rd;
  
  let dA  = max(dot(nrm, L1), 0.0) * 0.80;
  let dB  = max(dot(nrm, L2), 0.0) * 0.35;
  let dC  = max(dot(nrm, L3), 0.0) * 0.25;
  let h   = normalize(L1 + v);
  
  let specPower = select(50.0, 10.0 + (1.0 / (bevel + 0.01)), bevel > 0.0);
  let sp  = pow(max(dot(nrm, h), 0.0), specPower) * 0.15;
  let fr  = pow(1.0 - max(dot(nrm, v), 0.0), 3.0) * 0.10;
  
  var col = p.color * (0.15 + dA + dB + dC);
  col    += vec3f(1.0) * sp;
  col    += vec3f(1.0) * fr;
  col     = pow(max(col, vec3f(0.0)), vec3f(1.0 / 2.2));
  
  let isSelected = u.u9.z > 0.5;
  let selectionMode = u32(u.u9.w); // 0=Object, 1=Face, 2=Edge, 3=Vertex
  let face_id_to_highlight = 2u; // Например выделяем ID=2 для тестов, можно брать из Uniform
  
  if isSelected {
    let yellowLine = vec3f(1.0, 0.75, 0.0);
    let rim = pow(clamp(1.0 - max(dot(nrm, -rd), 0.0), 0.0, 1.0), 5.0);
    
    if selectionMode == 0u {
      // Object Mode: Подсветка всего объекта легким контуром
      col += yellowLine * rim * 0.3;
      if is_edge { col = mix(col, yellowLine, 0.5); }
    } else if selectionMode == 1u {
      // Face Mode: Подсвечивается только выбранная грань (сейчас для примера ID=2)
      if p.cellMask == face_id_to_highlight {
         col = mix(col, yellowLine, 0.5);
      }
      if is_edge { col = mix(col, vec3f(0.2), 0.5); } // Остальные линии тусклые
    } else if selectionMode == 2u {
      // Edge Mode: Подсвечиваются только линии (ребра)
      if is_edge {
         col = yellowLine;
      }
    } else if selectionMode == 3u {
      // Vertex Mode: Точечный режим (пока показываем ребра + уголки)
      if is_edge { col = yellowLine; }
      // Точки для CAD можно вычислять аналитически или рисовать инстансами, 
      // пока просто перекрашиваем все в wireframe.
    }
  }

  var out: FragOut;
  out.color = vec4f(col, 1.0);
  out.depth = p.pos.z;
  return out;
}
"##;
