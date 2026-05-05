/// City domain — authoritative wire types for the Food Empire city map.
///
/// Frontend receives this JSON and renders it — no geometry logic on the client.
/// All coordinates are world-space XZ (Y = up).
///
/// Real-city geometry model:
///   - CityDistrict  → polygon boundary (XZ contour)
///   - CityRoad      → polyline centerline + width (not a box)
///   - CityBuilding  → footprint polygon + height extrude (not x/z/w/d box)
///   - CityLot       → polygon ground tile within a district

use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// Top-level map
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CityMap {
    /// Deterministic seed (derived from tenant_id). Same tenant → same layout.
    pub seed: u64,
    pub bounds: CityBounds,
    /// Live economy snapshot merged from restaurant data.
    pub economy: CityEconomy,
    /// Road network — polyline-based.
    pub roads: Vec<CityRoad>,
    /// Districts — polygon-based.
    pub districts: Vec<CityDistrict>,
    /// Ground plane config.
    pub ground: CityGround,
}

// ─────────────────────────────────────────────────────────────────────────────
// Bounds
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CityBounds {
    pub width: f32,
    pub depth: f32,
}

// ─────────────────────────────────────────────────────────────────────────────
// Economy snapshot (wire — sent to frontend HUD)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CityEconomy {
    pub inventory_value_cents: i64,
    pub avg_profit_margin: f64,
    pub assistant_progress: i32,
    pub dish_count: i32,
    pub inventory_count: i32,
    pub expiring_soon: i32,
    pub revenue_cents: i64,
    pub restaurant_name: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Road — polyline centerline
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CityRoad {
    pub id: String,
    /// Centerline as XZ points: [[x0,z0],[x1,z1],…]
    pub polyline: Vec<[f32; 2]>,
    pub width: f32,
    pub lanes: u8,
    /// "primary" | "secondary" | "alley"
    pub road_type: String,
    pub color: String,
    /// Dashed lane markings (parametric offset along polyline)
    pub markings: Vec<RoadMarking>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoadMarking {
    /// Distance along polyline from start
    pub t: f32,
    pub length: f32,
    pub width: f32,
}

// ─────────────────────────────────────────────────────────────────────────────
// District — polygon boundary
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DistrictKind {
    Player,
    Office,
    Residential,
    Market,
    Shops,
    Competitor,
    Park,
    Industrial,
}

impl DistrictKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Player      => "player",
            Self::Office      => "office",
            Self::Residential => "residential",
            Self::Market      => "market",
            Self::Shops       => "shops",
            Self::Competitor  => "competitor",
            Self::Park        => "park",
            Self::Industrial  => "industrial",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CityDistrict {
    pub id: String,
    pub name: String,
    pub kind: String,
    /// District boundary polygon XZ: [[x0,z0],…] (closed — last connects to first)
    pub polygon: Vec<[f32; 2]>,
    /// Pre-computed centroid [x, z] for camera focus and labels
    pub centroid: [f32; 2],
    pub ground_color: String,
    pub accent_color: String,
    pub buildings: Vec<CityBuilding>,
    pub lots: Vec<CityLot>,
    pub unlocked: bool,
    pub badge: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Building mesh — pre-computed 3D geometry from backend geometry kernel
// ─────────────────────────────────────────────────────────────────────────────

/// Pre-computed indexed triangle mesh for a building.
///
/// The backend calls `extrude_polygon` from the geometry kernel, transforms
/// the result from kernel-space (XY-polygon extruded along Z) to city-space
/// (XZ-footprint extruded along Y), and serialises the flat buffers here.
///
/// Frontend uploads these directly to `BufferGeometry` — zero geometry math
/// on the client side.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CityMesh {
    /// Flat vertex positions: [x0,y0,z0, x1,y1,z1, …]
    pub positions: Vec<f32>,
    /// Flat vertex normals:   [nx0,ny0,nz0, …]
    pub normals: Vec<f32>,
    /// Flat UV coords:        [u0,v0, u1,v1, …]
    pub uvs: Vec<f32>,
    /// Triangle indices (every 3 = one face)
    pub indices: Vec<u32>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Building — footprint polygon + extrude
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CityBuilding {
    pub id: String,
    /// Footprint XZ contour (world space): [[x0,z0],[x1,z1],…]
    /// Rectangle = 4 points. L-shape = 6 points. Complex = N points.
    /// Frontend extrudes this polygon to `height`.
    pub footprint: Vec<[f32; 2]>,
    /// Ground level Y (0.0 = street level, >0 for elevated)
    pub base_y: f32,
    pub height: f32,
    /// Number of floors — drives window grid density on facades
    pub floors: u32,
    /// "office" | "residential" | "shop" | "market" | "player" | "competitor" | "industrial"
    pub kind: String,
    pub color: String,
    pub roof_color: Option<String>,
    pub emissive: Option<String>,
    pub emissive_intensity: f32,
    pub metalness: f32,
    pub roughness: f32,
    pub windows: bool,
    pub window_color: Option<String>,
    pub cast_shadow: bool,
    /// Pre-computed 3D mesh from the backend geometry kernel.
    /// When `Some`, the frontend uses BufferGeometry directly (no client-side extrude).
    /// When `None`, the frontend falls back to ExtrudeGeometry from `footprint`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mesh: Option<CityMesh>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Lot — ground polygon within a district
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CityLot {
    pub id: String,
    /// XZ polygon (world space)
    pub polygon: Vec<[f32; 2]>,
    /// "grass" | "parking" | "plaza" | "pavement" | "water"
    pub kind: String,
    pub color: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Ground
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CityGround {
    pub color: String,
    pub size: f32,
    pub fog_color: String,
    pub fog_near: f32,
    pub fog_far: f32,
}
