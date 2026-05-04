//! Procedural Food Mesh generators.
//!
//! Food objects don't need engineering precision — they need:
//! shape, colour, volume, texture, viscosity, steam, bubbles,
//! animation, product state.
//!
//! All generators here use: lathe_profile / extrude_polygon /
//! MeshBuilder / decal / normals → direct GLB export.
//! NO GeometricShell involved.

pub mod bottled_sauce;
pub mod flat_card;
pub mod jar_product;
pub mod plate_food;
pub mod sauce_in_bowl;
