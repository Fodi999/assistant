pub const WGSL: &str = r##"
// ── CAD Solid pipeline vertex shader ──
// Renders a single clean solid primitive geometry representing the container 
// (e.g. perfect Blender default cube) without grid subdivisions.

// Helper mapping UI Euler Degrees to Rotation Matrix (XYZ)
fn euler_to_matrix(deg: vec3f) -> mat3x3f {
  let rad = deg * (3.14159265 / 180.0);
  
  let cx = cos(rad.x); let sx = sin(rad.x);
  let cy = cos(rad.y); let sy = sin(rad.y);
  let cz = cos(rad.z); let sz = sin(rad.z);

  let mX = mat3x3f(
    1.0, 0.0, 0.0,
    0.0, cx,  sx,
    0.0, -sx, cx
  );

  let mY = mat3x3f(
    cy,  0.0, -sy,
    0.0, 1.0, 0.0,
    sy,  0.0, cy
  );

  let mZ = mat3x3f(
    cz,  sz,  0.0,
    -sz, cz,  0.0,
    0.0, 0.0, 1.0
  );

  return mZ * mY * mX;
}

@vertex fn vs_cad(
  @location(0) position: vec3f,
  @location(1) normal: vec3f,
  @location(2) face_id: u32,
) -> Pv {
  let sp          = spheres.data[0];
  let ro          = u.u1.xyz;
  let right       = u.u2.xyz;
  let upv         = u.u3.xyz;
  let fwd         = u.u4.xyz;
  let isOrtho     = u.u9.y > 0.5;
  let orthoHeight = distance(ro, u.u8.xyz) * 0.45;
  let asp         = u.u0.y / u.u0.z;

  let objPos      = u.u8.xyz;
  let objScale    = max(vec3f(0.001), u.u11.xyz);
  let objDim      = max(vec3f(0.001), u.u12.xyz);
  let objRot      = u.u10.xyz;
  
  let halfCell    = (objDim / 2.0) * objScale;
  let center      = objPos + vec3f(0.0, halfCell.y, 0.0);
  
  let rotMat      = euler_to_matrix(objRot);
  // position comes in absolute dimensions from backend, assuming it's around origin
  // truck backend centers it at origin, we just apply rotation and translation
  // position from truck-modeling / sketch extrude is in world metres — no mm conversion needed.
  // (Previously the cube endpoint used mm, but the sketch extrude pipeline uses metres directly.)
  let world_scale = 1.0;
  
  // Also we want it scaled by objScale
  let scaledPos   = position * world_scale * objScale;
  let localPos    = rotMat * scaledPos;
  
  let wp          = center + localPos;
  let relV        = wp - ro;
  let mvx         = dot(relV, right);
  let mvy         = dot(relV, upv);
  let mvz         = dot(relV, fwd);
  let smvz        = max(mvz, 0.001);
  let focal       = 2.414;
  let cx          = select(mvx * focal / smvz / asp, mvx / orthoHeight / asp, isOrtho);
  let cy          = select(mvy * focal / smvz,       mvy / orthoHeight,       isOrtho);
  let zNdc        = clamp(mvz / (mvz + 8.0), 0.0, 0.9999);

  var o: Pv;
  o.pos      = vec4f(cx, cy, zNdc, 1.0);
  o.quadUV   = vec2f(0.0); // Not used in poly
  o.color    = sp.colorP.xyz; 
  o.depth    = mvz;
  o.phase    = 0.0;
  o.wCenter  = center;
  o.size     = halfCell.x;
  o.cellMask = face_id; // Using cellMask property to forward face_id
  o.halfCell = halfCell.x;
  o.meshMode = 1u;
  o.meshN    = normalize(rotMat * normal);
  return o;
}
"##;
