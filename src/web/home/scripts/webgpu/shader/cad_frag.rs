pub const WGSL: &str = r##"
// ── CAD / Solid mode fragment shader ── Visual Style v1 ──────────────────────
// Raymarching с точными миллиметровыми размерами (Dimensions) и фасками (Bevel)
//
// Lighting model: matte CAD (Blinn-Phong, tuned for engineering aesthetics)
//   - soft 3-light rig  (key + fill + rim)
//   - moderate specular (not plastic, not metal)
//   - subtle Fresnel rim
//   - gamma-correct output (sRGB)
//
// Selection/Hover via UBO (unchanged):
//   ubo[38] isSelected   ubo[39] selectionMode
//   ubo[60] selected_face_id   ubo[61] hovered_face_id
//
// TODO(v2):
//   - full material system (roughness/metalness from GPU buffer)
//   - real GGX/PBR when geometry_engine MeshPart has Material
//   - per-object logical face material ids
//   - edge overlay / silhouette pass
//   - ambient occlusion bake

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
  let d2 = sdRoundBox2D(p.xy, b.xy, bevel);
  let dz = abs(p.z) - b.z;
  let w = vec2f(max(d2, 0.0), max(dz, 0.0));
  let d_ext = min(max(d2, dz), 0.0) + length(w);
  let chamfer = (d2 + dz + bevel) * 0.70710678;
  return max(d_ext, chamfer);
}

@fragment fn fs_cad(p: Pv, @builtin(front_facing) is_front: bool) -> FragOut {
  // Edge detection по face_id градиенту.
  // Порог 0.5 — только реальная граница двух разных граней.
  let is_edge = fwidth(f32(p.cellMask)) > 0.5;

  let ro   = u.u1.xyz;
  let dims = max(vec3f(0.001), u.u12.xyz);
  let bevel = min(u.u13.x, min(dims.x, min(dims.y, dims.z)) * 0.5);

  let fwd   = u.u4.xyz;
  let right = u.u2.xyz;
  let upv   = u.u3.xyz;

  let asp  = u.u0.y / u.u0.z;
  let ndcX = (p.pos.x / u.u0.y * 2.0 - 1.0) * asp;
  let ndcY = -(p.pos.y / u.u0.z * 2.0 - 1.0);

  let isOrtho     = u.u9.y > 0.5;
  let orthoHeight = distance(ro, u.u8.xyz) * 0.45;
  let fl = 2.414;

  var rd = normalize(fwd * fl + right * ndcX + upv * ndcY);
  if isOrtho {
    rd = normalize(fwd);
  }

  // ── Normal ───────────────────────────────────────────────────────────────
  var nrm = p.meshN;
  if length(nrm) < 0.1 {
    nrm = vec3f(0.0, 1.0, 0.0);
  }
  // Flip normals on back faces for consistent lighting
  if !is_front { nrm = -nrm; }

  // ── CAD 3-light rig ──────────────────────────────────────────────────────
  // Key light: upper-right-front (warm, main)
  let L_key  = normalize(vec3f( 0.55,  0.75,  0.45));
  // Fill light: lower-left-back (cool, soft)
  let L_fill = normalize(vec3f(-0.70, -0.25, -0.30));
  // Rim light: lower-front (very soft)
  let L_rim  = normalize(vec3f( 0.15, -0.45, -0.70));

  let v = -rd;   // view direction

  // Diffuse contributions — kept low so total ≤ 1.0 before gamma
  let d_key  = max(dot(nrm, L_key),  0.0) * 0.52;
  let d_fill = max(dot(nrm, L_fill), 0.0) * 0.15;
  let d_rim  = max(dot(nrm, L_rim),  0.0) * 0.08;

  // Blinn-Phong specular on key light only (matte CAD look)
  let h_key     = normalize(L_key + v);
  let spec_pow  = select(60.0, 12.0 + (1.0 / (bevel + 0.02)), bevel > 0.0);
  let spec      = pow(max(dot(nrm, h_key), 0.0), spec_pow) * 0.06;

  // Subtle Fresnel rim (silhouette catch only)
  let fresnel   = pow(1.0 - clamp(dot(nrm, v), 0.0, 1.0), 4.0) * 0.04;

  // Ambient: soft constant + very slight sky gradient
  let ambient   = 0.24 + max(nrm.y, 0.0) * 0.06;

  // Total max ≈ 0.24 + 0.52 + 0.15 + 0.08 = 0.99 → base_col(0.80) * 0.99 = 0.79 linear
  // After gamma pow(0.79, 1/2.2) ≈ 0.89 — solid opaque mid-grey, not blown out
  var col = p.color * (ambient + d_key + d_fill + d_rim);
  col    += vec3f(1.0) * spec;
  col    += p.color * fresnel;

  // Subtle edge darkening (AO approximation on face boundaries)
  if is_edge {
    col *= 0.70;
  }

  // Gamma correction (linear → sRGB)
  col = pow(max(col, vec3f(0.0)), vec3f(1.0 / 2.2));

  // ── Selection / Hover UBO ────────────────────────────────────────────────
  let isSelected      = u.u9.z > 0.5;
  let selectionMode   = u32(u.u9.w);   // 0=Object 1=Face 2=Edge 3=Vertex
  let selected_face_id = u32(u.u15.x);
  let hovered_face_id  = u32(u.u15.y);

  // Hover: very soft cyan tint only on the hovered face — does NOT wash out the surface
  if hovered_face_id != 0u && p.cellMask == hovered_face_id {
    col = mix(col, vec3f(0.35, 0.88, 1.0), 0.14);
  }

  if isSelected {
    let yellow = vec3f(1.0, 0.85, 0.20);
    // Rim factor — highlights silhouette, not flat fill
    let rim = pow(clamp(1.0 - max(dot(nrm, -rd), 0.0), 0.0, 1.0), 5.0);

    if selectionMode == 0u {
      // Object Mode: subtle yellow rim on silhouette only
      col += yellow * rim * 0.25;
    } else if selectionMode == 1u {
      // Face Mode: selected face gets a warm overlay (not acid), others stay normal
      if selected_face_id != 0u && p.cellMask == selected_face_id {
        col = mix(col, yellow, 0.28);
      }
    } else if selectionMode == 2u {
      // Edge Mode: edges glow yellow, faces stay normal
      if is_edge { col = mix(col, yellow, 0.80); }
    } else if selectionMode == 3u {
      // Vertex Mode: same as edge for now (vertex instancing in v2)
      if is_edge { col = mix(col, yellow, 0.60); }
    }
  }

  var out: FragOut;
  out.color = vec4f(col, 1.0);
  out.depth = p.pos.z;
  return out;
}

// ── CAD Edge fragment shader — near-black edge lines ──────────────────────
@fragment fn fs_cad_edge(p: EdgeVOut) -> @location(0) vec4f {
  return vec4f(0.08, 0.09, 0.10, 1.0);
}
"##;
