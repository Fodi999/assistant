//! `LaboratoryV2Service` — use cases for the Photo → 3D Model pipeline.
//!
//! PR #2 ships:
//!   * `register_image`        — JSON path: client uploaded file elsewhere, we just persist URL
//!   * `upload_and_register`   — multipart path: receive raw bytes, write via [`StorageAdapter`], persist
//!   * `get_asset`             — fetch a 3D asset (with embedded source image URL)
//!
//! PR #3-4 will fill `generate_model_from_image` (Gemini Vision + geometry generators).

use std::sync::Arc;

use sqlx::PgPool;
use uuid::Uuid;

use super::models::{Laboratory3DAsset, LaboratoryImage, RegisterImagePayload};
use crate::infrastructure::persistence::laboratory_v2_repository::{
    CreateImageInput, LaboratoryV2Repository,
};
use crate::infrastructure::storage::StorageAdapter;
use crate::shared::AppError;

/// Mime types accepted by the multipart uploader.
const ALLOWED_MIME_TYPES: &[&str] = &["image/png", "image/jpeg", "image/webp"];

/// Hard cap on a single uploaded file (10 MB).
const MAX_UPLOAD_BYTES: usize = 10 * 1024 * 1024;

#[derive(Clone)]
pub struct LaboratoryV2Service {
    repo: LaboratoryV2Repository,
    storage: Arc<dyn StorageAdapter>,
}

impl LaboratoryV2Service {
    pub fn new(pool: PgPool, storage: Arc<dyn StorageAdapter>) -> Self {
        Self {
            repo: LaboratoryV2Repository::new(pool),
            storage,
        }
    }

    // ── POST /api/laboratory/images (JSON variant) ──────────────────────────
    //
    // Used when the client already has a hosted URL (e.g. existing CDN asset)
    // and just wants the backend to register it.

    pub async fn register_image(
        &self,
        user_id: Uuid,
        payload: RegisterImagePayload,
    ) -> Result<LaboratoryImage, AppError> {
        validate_mime(&payload.mime_type)?;
        if payload.image_url.trim().is_empty() {
            return Err(AppError::validation("image_url must not be empty"));
        }

        self.repo
            .create_image(CreateImageInput {
                user_id,
                tenant_id: None,
                image_url: payload.image_url,
                mime_type: payload.mime_type,
                original_filename: payload.original_filename,
                byte_size: payload.byte_size,
                width_px: payload.width_px,
                height_px: payload.height_px,
            })
            .await
    }

    // ── POST /api/laboratory/images (multipart variant) ─────────────────────

    /// Upload raw image bytes via the [`StorageAdapter`], then persist the row.
    pub async fn upload_and_register(
        &self,
        user_id: Uuid,
        bytes: Vec<u8>,
        mime_type: String,
        original_filename: Option<String>,
    ) -> Result<LaboratoryImage, AppError> {
        if bytes.is_empty() {
            return Err(AppError::validation("uploaded file is empty"));
        }
        if bytes.len() > MAX_UPLOAD_BYTES {
            return Err(AppError::validation(format!(
                "uploaded file exceeds {} bytes",
                MAX_UPLOAD_BYTES
            )));
        }
        validate_mime(&mime_type)?;

        let ext = extension_for_mime(&mime_type);
        let id = Uuid::new_v4();
        let key = format!("laboratory/images/{id}.{ext}");
        let byte_size = bytes.len() as i64;

        let public_url = self.storage.put_bytes(&key, bytes, &mime_type).await?;

        self.repo
            .create_image(CreateImageInput {
                user_id,
                tenant_id: None,
                image_url: public_url,
                mime_type,
                original_filename,
                byte_size: Some(byte_size),
                width_px: None,
                height_px: None,
            })
            .await
    }

    /// `POST /api/laboratory/images/:image_id/generate-model`
    ///
    /// Lands in PR #3-4 (Gemini Vision + geometry generators).
    pub async fn generate_model_from_image(
        &self,
        _image_id: Uuid,
        _user_id: Uuid,
    ) -> Result<Laboratory3DAsset, AppError> {
        Err(AppError::internal(
            "not_implemented: generate_model_from_image lands in PR #3-4",
        ))
    }

    /// `GET /api/laboratory/assets/:asset_id`
    pub async fn get_asset(
        &self,
        asset_id: Uuid,
        user_id: Uuid,
    ) -> Result<Laboratory3DAsset, AppError> {
        self.repo
            .get_asset_by_id(asset_id, user_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("asset {asset_id} not found")))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// helpers
// ─────────────────────────────────────────────────────────────────────────────

fn validate_mime(mime: &str) -> Result<(), AppError> {
    if ALLOWED_MIME_TYPES
        .iter()
        .any(|m| m.eq_ignore_ascii_case(mime))
    {
        Ok(())
    } else {
        Err(AppError::validation(format!(
            "unsupported mime_type `{mime}` — allowed: {}",
            ALLOWED_MIME_TYPES.join(", ")
        )))
    }
}

fn extension_for_mime(mime: &str) -> &'static str {
    match mime.to_ascii_lowercase().as_str() {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/webp" => "webp",
        _ => "bin",
    }
}
