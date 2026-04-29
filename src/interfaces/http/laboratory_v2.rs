//! HTTP handlers for Laboratory **v2** — Photo → 3D Model.
//!
//! Routes (mounted under `/api/laboratory/...`):
//!   * `POST /laboratory/images` — accepts **either**
//!       - `application/json`        → registers a pre-hosted URL
//!       - `multipart/form-data`     → uploads a file (`file` part)
//!   * `POST /laboratory/images/:image_id/generate-model`  (501 — PR #3-4)
//!   * `GET  /laboratory/assets/:asset_id`

use axum::{
    extract::{FromRequest, Multipart, Path, Query, Request, State},
    http::{header, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::application::laboratory_v2::{
    Laboratory3DAsset, LaboratoryImage, LaboratoryV2Service, RegisterImagePayload,
    SurfaceTuneInfo,
};
use crate::infrastructure::geometry::kernel::GeometryQuality;
use crate::infrastructure::gemini::GeminiVision3D;
use crate::interfaces::http::middleware::AuthUser;
use crate::shared::AppError;

// ─────────────────────────────────────────────────────────────────────────────
// JSON body for the non-multipart variant
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RegisterImageBody {
    #[serde(flatten)]
    pub payload: RegisterImagePayload,
}

// ─────────────────────────────────────────────────────────────────────────────
// `POST /laboratory/images` — content-type dispatcher
// ─────────────────────────────────────────────────────────────────────────────

pub async fn register_image(
    auth: AuthUser,
    State(svc): State<LaboratoryV2Service>,
    request: Request,
) -> Result<(StatusCode, Json<LaboratoryImage>), AppError> {
    let content_type = request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let user_id = *auth.user_id.as_uuid();

    let image = if content_type.starts_with("multipart/form-data") {
        register_image_multipart(&svc, user_id, request).await?
    } else {
        let Json(body) = Json::<RegisterImageBody>::from_request(request, &())
            .await
            .map_err(|e| AppError::validation(format!("invalid JSON body: {e}")))?;
        svc.register_image(user_id, body.payload).await?
    };

    Ok((StatusCode::CREATED, Json(image)))
}

async fn register_image_multipart(
    svc: &LaboratoryV2Service,
    user_id: Uuid,
    request: Request,
) -> Result<LaboratoryImage, AppError> {
    let mut multipart = Multipart::from_request(request, &())
        .await
        .map_err(|e| AppError::validation(format!("invalid multipart body: {e}")))?;

    let mut bytes: Option<Vec<u8>> = None;
    let mut mime: Option<String> = None;
    let mut filename: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::validation(format!("multipart field: {e}")))?
    {
        if field.name() == Some("file") {
            filename = field.file_name().map(str::to_owned);
            mime = field.content_type().map(str::to_owned);
            let data = field
                .bytes()
                .await
                .map_err(|e| AppError::validation(format!("multipart read: {e}")))?;
            bytes = Some(data.to_vec());
            break;
        }
    }

    let bytes = bytes.ok_or_else(|| AppError::validation("multipart: missing `file` part"))?;
    let mime = mime
        .ok_or_else(|| AppError::validation("multipart: `file` part missing Content-Type"))?;

    svc.upload_and_register(user_id, bytes, mime, filename).await
}

// ─────────────────────────────────────────────────────────────────────────────
// `POST /laboratory/images/:image_id/generate-model`
//
// Accepts an optional `?quality=draft|standard|high|ultra` query parameter.
// Defaults to `high` (the Studio preset). Quality drives Rust-side geometry
// resolution: radial segments + heightfield rings. Unrelated to the
// frontend `RenderQuality` switch (DPR / shadow maps / AA).
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, Deserialize)]
pub struct GenerateModelQuery {
    /// `draft` | `standard` | `high` | `ultra`. Unknown / missing → `high`.
    #[serde(default)]
    pub quality: Option<String>,
}

pub async fn generate_model(
    auth: AuthUser,
    State(svc): State<LaboratoryV2Service>,
    Path(image_id): Path<Uuid>,
    Query(query): Query<GenerateModelQuery>,
) -> Result<(StatusCode, Json<Laboratory3DAsset>), AppError> {
    let quality = GeometryQuality::from_opt(query.quality.as_deref());
    let asset = svc
        .generate_model_from_image_with_quality(image_id, *auth.user_id.as_uuid(), quality)
        .await?;
    Ok((StatusCode::CREATED, Json(asset)))
}

// ─────────────────────────────────────────────────────────────────────────────
// `GET /laboratory/assets/:asset_id`
// ─────────────────────────────────────────────────────────────────────────────

pub async fn get_asset(
    auth: AuthUser,
    State(svc): State<LaboratoryV2Service>,
    Path(asset_id): Path<Uuid>,
) -> Result<Json<Laboratory3DAsset>, AppError> {
    let asset = svc.get_asset(asset_id, *auth.user_id.as_uuid()).await?;
    Ok(Json(asset))
}

// ─────────────────────────────────────────────────────────────────────────────
// `POST /laboratory/debug-vision`
//
// Development / QA endpoint. Does NOT touch the DB.
// Accepts a multipart `file` part, sends it straight to Gemini Vision,
// and returns:
//   {
//     "image_size_bytes": 12345,
//     "mime_type": "image/jpeg",
//     "gemini_model": "gemini-2.5-flash",
//     "tokens": { "prompt": 1234, "output": 56, "total": 1290 },
//     "raw_response": "{ … }", // exactly what Gemini returned
//     "parsed_spec": { … }     // the decoded Product3DSpec
//   }
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct DebugVisionResponse {
    pub image_size_bytes: usize,
    pub mime_type: String,
    pub gemini_model: String,
    pub tokens: DebugVisionTokens,
    pub raw_response: serde_json::Value,
    pub parsed_spec: serde_json::Value,
}

#[derive(Serialize)]
pub struct DebugVisionTokens {
    pub prompt: u64,
    pub output: u64,
    pub total: u64,
}

pub async fn debug_vision(
    _auth: AuthUser,
    State(vision): State<std::sync::Arc<GeminiVision3D>>,
    request: Request,
) -> Result<Json<DebugVisionResponse>, AppError> {
    let mut multipart = Multipart::from_request(request, &())
        .await
        .map_err(|e| AppError::validation(format!("invalid multipart: {e}")))?;

    let mut bytes: Option<Vec<u8>> = None;
    let mut mime: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::validation(format!("multipart field: {e}")))?
    {
        if field.name() == Some("file") {
            mime = field.content_type().map(str::to_owned);
            let data = field
                .bytes()
                .await
                .map_err(|e| AppError::validation(format!("multipart read: {e}")))?;
            bytes = Some(data.to_vec());
            break;
        }
    }

    let bytes = bytes.ok_or_else(|| AppError::validation("missing `file` part"))?;
    let mime = mime.unwrap_or_else(|| "image/jpeg".to_string());
    let image_size = bytes.len();

    let result = vision
        .analyze_image_for_3d_with_usage(bytes, &mime)
        .await?;

    let raw_json: serde_json::Value = serde_json::to_value(&result.spec)
        .unwrap_or(serde_json::Value::Null);

    Ok(Json(DebugVisionResponse {
        image_size_bytes: image_size,
        mime_type: mime,
        gemini_model: "gemini-2.5-flash".to_string(),
        tokens: DebugVisionTokens {
            prompt: result.usage.prompt_tokens,
            output: result.usage.output_tokens,
            total: result.usage.total_tokens,
        },
        raw_response: raw_json.clone(),
        parsed_spec: raw_json,
    }))
}

// ─────────────────────────────────────────────────────────────────────────────
// `POST /laboratory/assets/:asset_id/tune-surface`
//
// PR #31 — Smoothness Slider.
//
// Regenerates the geometry for an existing asset using a single
// `smoothness` value (0.0 = very textured/chunky, 1.0 = perfectly smooth)
// without re-running Gemini Vision. The stored `product_spec` is reused
// for object_type / colours / container; only the surface params are
// overridden by the smoothness mapping.
//
// Body:
//   {
//     "smoothness": 0.72,          // 0.0..1.0  (front-end sends 0..100 / 100)
//     "quality": "draft"            // optional — default "draft" for preview
//   }
//
// Typical frontend flow:
//   • slider drag (debounce 300 ms) → POST tune-surface?quality=draft
//   • "Apply High" button           → POST tune-surface?quality=high
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct TuneSurfaceBody {
    /// Smoothness in [0.0, 1.0]. Frontend may send the integer 0–100 and
    /// divide by 100 before posting, or send the float directly.
    pub smoothness: f32,
    /// `"draft"` | `"standard"` | `"high"` | `"ultra"`. Default: `"draft"`.
    #[serde(default)]
    pub quality: Option<String>,
}

/// Derived surface parameters returned alongside the new GLB URL so the
/// frontend can display what the slider actually produced.
pub type TuneSurfaceInfo = SurfaceTuneInfo;

#[derive(Debug, Serialize)]
pub struct TuneSurfaceResponse {
    pub asset: Laboratory3DAsset,
    pub surface_info: SurfaceTuneInfo,
}

pub async fn tune_surface(
    auth: AuthUser,
    State(svc): State<LaboratoryV2Service>,
    Path(asset_id): Path<Uuid>,
    Json(body): Json<TuneSurfaceBody>,
) -> Result<Json<TuneSurfaceResponse>, AppError> {
    let smoothness = body.smoothness.clamp(0.0, 1.0);
    let quality = GeometryQuality::from_opt(body.quality.as_deref());

    let (asset, info) = svc
        .tune_surface(asset_id, *auth.user_id.as_uuid(), smoothness, quality)
        .await?;

    Ok(Json(TuneSurfaceResponse { asset, surface_info: info }))
}
