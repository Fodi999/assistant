// ── WGSL: uniform bindings, storage bindings, helper vertex struct ─────────────
// Domain: GPU interface contract — all shader stages share these declarations.

pub const WGSL: &str = r##"
struct Uniforms {
  u0: vec4f,   // time, w, h, pushStrength
  u1: vec4f,   // ro
  u2: vec4f,   // right
  u3: vec4f,   // up
  u4: vec4f,   // fwd
  u5: vec4f,   // mouseX, mouseY, mouseActive, shapeExponent
  u6: vec4f,   // formMix, formMode, formA, formScale
  u7: vec4f,   // cellSdfOn, cellRadius, colorMode, hideLow
  u8: vec4f,   // objectX, objectY, objectZ, _
  u9: vec4f,   // floorGridScale, ortho, isSelected, _
  u10: vec4f,  // objectRotX, objectRotY, objectRotZ, _
  u11: vec4f,  // objectScaleX, objectScaleY, objectScaleZ, _
};
@group(0) @binding(0) var<uniform> u: Uniforms;

struct Sphere { posR: vec4f, colorP: vec4f }
struct Spheres { data: array<Sphere> }
@group(0) @binding(1) var<storage, read> spheres: Spheres;

struct Vert { @builtin(position) pos: vec4f, @location(0) uv: vec2f }
"##;
