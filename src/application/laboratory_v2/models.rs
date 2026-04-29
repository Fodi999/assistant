//! Domain models for Laboratory v2.
//!
//! Three layers of types:
//!
//! 1. **Persistence rows** (`LaboratoryImageRow`, `Laboratory3DAssetRow`)
//!    — exact mirror of DB columns, used by the repository.
//! 2. **DTOs** (`LaboratoryImage`, `Laboratory3DAsset`) — what the HTTP layer
//!    serialises to JSON. `created_at`/`updated_at` are `String` (RFC3339)
//!    here because the frontend doesn't care about chrono types.
//! 3. **Vision spec** (`Product3DSpec`) — output of Gemini Vision; persisted
//!    inside `object_spec_json`. Stable schema across all generators.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─────────────────────────────────────────────────────────────────────────────
// Asset status — string-newtype style, stays in sync with the SQL CHECK.
// ─────────────────────────────────────────────────────────────────────────────

/// Lifecycle of a 3D-asset job. Stored as TEXT in `laboratory_3d_assets.status`.
///
/// `pending → analyzing_image → generating_model → ready`
/// Any stage may transition to `failed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetStatus {
    Pending,
    AnalyzingImage,
    GeneratingModel,
    Ready,
    Failed,
}

impl AssetStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::AnalyzingImage => "analyzing_image",
            Self::GeneratingModel => "generating_model",
            Self::Ready => "ready",
            Self::Failed => "failed",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "analyzing_image" => Some(Self::AnalyzingImage),
            "generating_model" => Some(Self::GeneratingModel),
            "ready" => Some(Self::Ready),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// API DTO — uploaded source image.
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaboratoryImage {
    pub id: Uuid,
    pub user_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<Uuid>,
    pub image_url: String,
    pub mime_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub byte_size: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width_px: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height_px: Option<i32>,
    pub created_at: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// API DTO — 3D asset (job + generated model).
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Laboratory3DAsset {
    pub id: Uuid,
    pub image_id: Uuid,
    pub user_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<Uuid>,

    pub status: AssetStatus,
    pub provider: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_type: Option<String>,
    /// Raw `Product3DSpec` JSON, stored as `serde_json::Value` so we don't
    /// have to re-deserialise it on every read; the frontend can render it
    /// directly or ignore it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_spec: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail_url: Option<String>,

    /// Source image URL is duplicated into the asset response so the frontend
    /// can render "before / after" without a second request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Vision output — `Product3DSpec`
// ─────────────────────────────────────────────────────────────────────────────
//
// Gemini Vision returns this shape (after JSON repair). It's the only
// contract between the vision adapter and the geometry generators.
//
// Generators receive a `Product3DSpec` and return a mesh. Therefore adding
// a new generator only requires a new `Product3DObjectType` variant + a new
// match arm in `geometry::dispatch`.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Product3DObjectType {
    SauceInBowl,
    BottledSauce,
    JarProduct,
    PlateFood,
    FlatCard,
    Unknown,
}

impl Product3DObjectType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SauceInBowl => "sauce_in_bowl",
            Self::BottledSauce => "bottled_sauce",
            Self::JarProduct => "jar_product",
            Self::PlateFood => "plate_food",
            Self::FlatCard => "flat_card",
            Self::Unknown => "unknown",
        }
    }
}

/// Container hint (bowl, bottle, jar, plate). Optional — for `flat_card` etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerSpec {
    /// "ceramic_bowl" | "glass_bowl" | "glass_bottle" | "glass_jar" | "white_plate" | …
    pub kind: String,
    /// Physical material: `"glass"` | `"ceramic"` | `"plastic"` | `"metal"` | `"unknown"`.
    /// Used by the frontend to choose the correct material shader.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material: Option<String>,
    /// Opaque base colour (for ceramic / plastic / metal).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_hex: Option<String>,
    /// Glass tint colour — the colour visible through the glass wall.
    /// Only meaningful when `material == "glass"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tint_hex: Option<String>,
    /// Glass transparency fraction (0.0 = opaque, 1.0 = fully transparent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transparency: Option<f32>,
    /// How much darker the rim appears compared to the body (0.0–1.0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rim_darkness: Option<f32>,
    /// Approximate diameter / width in mm (vision is rough — used as a hint).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diameter_mm: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height_mm: Option<f32>,
}

/// Visual properties of the product itself.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductVisualSpec {
    /// Dominant colour as hex, e.g. `"#B8321F"`.
    pub color_hex: String,
    /// 0.0 (water) … 1.0 (paste). For sauces / liquids only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub viscosity: Option<f32>,
    /// 0.0 (matte) … 1.0 (mirror). Used for material shading.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gloss: Option<f32>,
    /// Free-form short description ("thick red sauce with herb specks").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Visible surface geometry — filled by Vision for sauce_in_bowl / plate_food.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface: Option<ProductSurfaceSpec>,
}

/// Surface geometry parameters estimated by Gemini Vision.
///
/// All fields are optional so old Gemini responses (without `surface`) keep
/// deserializing correctly. Generators should call `.unwrap_or(default)`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProductSurfaceSpec {
    /// General surface pattern:
    /// `"flat"` | `"swirl"` | `"spiral_swirl"` | `"mound"` | `"waves"` | `"chunky"` | `"unknown"`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    /// Number of visible swirl arms (1–8). Relevant for swirl / spiral_swirl.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub swirl_arms: Option<u8>,
    /// Height of ridges relative to container radius (0.0 = flat, 1.0 = very tall).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ridge_height: Option<f32>,
    /// Depth of grooves between ridges (0.0 = none, 1.0 = deep).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub groove_depth: Option<f32>,
    /// Height of the centre peak relative to container radius (0.0 = flat, 1.0 = prominent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub center_peak: Option<f32>,
    /// Fraction of container radius the sauce fills (0.80–1.0, typical 0.92–0.96).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_radius_ratio: Option<f32>,
    /// Gap between sauce edge and container rim as a fraction of radius (0.0–0.20).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rim_gap_ratio: Option<f32>,
    /// Degree of random noise / organic imperfection (0.0 = perfect, 1.0 = very rough).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface_irregularity: Option<f32>,
    /// Strength of specular highlight in the centre (0.0 = none, 1.0 = very bright).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub highlight_strength: Option<f32>,
    /// Camera angle used to estimate the surface:
    /// `"top_down"` | `"three_quarter"` | `"side"` | `"unknown"`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view_angle: Option<String>,

    // ── PR #29: Fill Volume ──────────────────────────────────────────────────
    /// How full the container is, from 0 (empty) to 1 (filled to the brim).
    /// Drives the liquid side-wall height and the sauce floor Y position.
    /// Typical range 0.35–0.90.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_height_ratio: Option<f32>,
    /// Visible thickness / depth of the sauce layer relative to bowl height.
    /// 0 = thin film, 1 = very thick paste (affects side-wall geometry).
    /// Typical range 0.10–0.70.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface_thickness: Option<f32>,
    /// Height of the meniscus (raised edge at the container wall).
    /// 0 = flat edge, 1 = strongly curved up at the rim (like a full glass of water).
    /// Typical range 0.05–0.60.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meniscus_height: Option<f32>,
}

/// Optional scene hints (lighting, surface). Generators may ignore.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SceneSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lighting: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// PR #28 — Shape Recipe
// ─────────────────────────────────────────────────────────────────────────────

/// One constructive layer in the Shape Recipe — maps to a geometry generator
/// call or a material group in the final mesh.
///
/// Gemini populates this; the geometry dispatcher reads it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapeLayer {
    /// What this layer is: `"container"` | `"fill"` | `"surface"` |
    /// `"label"` | `"cap"` | `"decal"` | `"lighting_hint"`.
    pub layer: String,
    /// Sub-type hint used to select the correct generator:
    /// `"bowl"` | `"bottle"` | `"jar"` | `"plate"` | `"flat_card"` |
    /// `"sauce"` | `"swirl"` | `"mound"` | `"waves"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// Physical material class: `"glass"` | `"ceramic"` | `"plastic"` |
    /// `"metal"` | `"food"` | `"liquid"` | `"label"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material_class: Option<String>,
    /// Dominant colour for this layer as `"#RRGGBB"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_hex: Option<String>,
    /// PBR roughness 0.0–1.0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roughness: Option<f32>,
    /// PBR metalness 0.0–1.0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metalness: Option<f32>,
    /// Opacity / transparency 0.0–1.0 (1 = solid, 0 = invisible).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opacity: Option<f32>,
    /// Free-form notes the geometry generator may use.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Gemini-authored build plan for the 3D model.
///
/// Contains an ordered list of [`ShapeLayer`]s from bottom to top (container
/// first, fill/surface last, lighting hints last).  Generators walk this list
/// to decide which sub-generators to call and what PBR properties to assign.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShapeRecipe {
    /// Ordered layers, bottom → top.
    pub layers: Vec<ShapeLayer>,
    /// Optional overall lighting hint (e.g. `"soft overhead"`, `"HDRI studio"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lighting_hint: Option<String>,
    /// Camera angle most likely to show the product at its best.
    /// `"top_down"` | `"three_quarter"` | `"side"` | `"hero_45"`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hero_angle: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product3DSpec {
    pub object_type: Product3DObjectType,
    /// 0.0 .. 1.0 — Vision's confidence. Below 0.55 the service falls back to `flat_card`.
    pub confidence: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container: Option<ContainerSpec>,
    pub product: ProductVisualSpec,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene: Option<SceneSpec>,
    /// PR #28 — Gemini-authored build plan (layers, PBR hints, hero angle).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape_recipe: Option<ShapeRecipe>,
}

impl Product3DSpec {
    /// Confidence threshold below which the service should ignore Vision's
    /// `object_type` and fall back to `flat_card`.
    pub const MIN_CONFIDENCE: f32 = 0.55;

    /// Resolve the effective object type (with confidence + Unknown fallback).
    pub fn effective_object_type(&self) -> Product3DObjectType {
        if self.confidence < Self::MIN_CONFIDENCE
            || matches!(self.object_type, Product3DObjectType::Unknown)
        {
            Product3DObjectType::FlatCard
        } else {
            self.object_type
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Request payloads
// ─────────────────────────────────────────────────────────────────────────────

/// JSON body for `POST /laboratory/images` — used when the client has already
/// uploaded the image elsewhere and just wants to register the URL. The
/// multipart form path is handled separately in the HTTP layer.
#[derive(Debug, Clone, Deserialize)]
pub struct RegisterImagePayload {
    pub image_url: String,
    pub mime_type: String,
    #[serde(default)]
    pub original_filename: Option<String>,
    #[serde(default)]
    pub byte_size: Option<i64>,
    #[serde(default)]
    pub width_px: Option<i32>,
    #[serde(default)]
    pub height_px: Option<i32>,
}
