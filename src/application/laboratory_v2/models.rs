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
    /// "ceramic_bowl" | "glass_bottle" | "glass_jar" | "white_plate" | …
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_hex: Option<String>,
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
}

/// Optional scene hints (lighting, surface). Generators may ignore.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SceneSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lighting: Option<String>,
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
