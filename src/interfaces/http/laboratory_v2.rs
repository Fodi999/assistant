//! HTTP handlers for Laboratory **v2** — Photo → 3D Model.
//!
//! Routes (mounted under `/api/laboratory/...`):
//!   * `POST /laboratory/images` — accepts **either**
//!       - `application/json`        → registers a pre-hosted URL
//!       - `multipart/form-data`     → uploads a file (`file` part)
//!   * `POST /laboratory/images/:image_id/generate-model`  (501 — PR #3-4)
//!   * `GET  /laboratory/assets/:asset_id`

use axum::{
    extract::{FromRequest, Multipart, Path, Request, State},
    http::{header, StatusCode},
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
// ─────────────────────────────────────────────────────────────────────────────

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
