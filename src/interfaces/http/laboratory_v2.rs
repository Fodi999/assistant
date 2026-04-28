//! HTTP handlers for Laboratory **v2** — Photo → 3D Model.
//!
//! PR #1: thin wrappers around the service skeleton. Every handler compiles,
//! is wired to JWT auth via `AuthUser`, and currently returns HTTP 500 with
//! a stable error envelope (`AppError::internal("not_implemented: …")`).
//!
//! Routes (mounted under `/api/laboratory/...` next to the legacy v1):
//!   * `POST /laboratory/images`
//!   * `POST /laboratory/images/:image_id/generate-model`
//!   * `GET  /laboratory/assets/:asset_id`

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::application::laboratory_v2::{
    Laboratory3DAsset, LaboratoryImage, LaboratoryV2Service, RegisterImagePayload,
};
use crate::interfaces::http::middleware::AuthUser;
use crate::shared::AppError;

// ─────────────────────────────────────────────────────────────────────────────
// Wrapper request body
// ─────────────────────────────────────────────────────────────────────────────
//
// PR #1 ships only the JSON variant ("client uploaded the file elsewhere,
// here is the URL"). The multipart variant — `multipart/form-data` with a
// `file` part — will be added on top in PR #2 alongside the StorageAdapter.

#[derive(Debug, Deserialize)]
pub struct RegisterImageBody {
    #[serde(flatten)]
    pub payload: RegisterImagePayload,
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// `POST /api/laboratory/images`
pub async fn register_image(
    auth: AuthUser,
    State(svc): State<LaboratoryV2Service>,
    Json(body): Json<RegisterImageBody>,
) -> Result<(StatusCode, Json<LaboratoryImage>), AppError> {
    let image = svc
        .register_image(*auth.user_id.as_uuid(), body.payload)
        .await?;
    Ok((StatusCode::CREATED, Json(image)))
}

/// `POST /api/laboratory/images/:image_id/generate-model`
pub async fn generate_model(
    auth: AuthUser,
    State(svc): State<LaboratoryV2Service>,
    Path(image_id): Path<Uuid>,
) -> Result<(StatusCode, Json<Laboratory3DAsset>), AppError> {
    let asset = svc
        .generate_model_from_image(image_id, *auth.user_id.as_uuid())
        .await?;
    Ok((StatusCode::CREATED, Json(asset)))
}

/// `GET /api/laboratory/assets/:asset_id`
pub async fn get_asset(
    auth: AuthUser,
    State(svc): State<LaboratoryV2Service>,
    Path(asset_id): Path<Uuid>,
) -> Result<Json<Laboratory3DAsset>, AppError> {
    let asset = svc.get_asset(asset_id, *auth.user_id.as_uuid()).await?;
    Ok(Json(asset))
}
