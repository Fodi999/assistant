//! `LaboratoryV2Service` — use cases for the Photo → 3D Model pipeline.
//!
//! PR #1 ships only the **skeleton** with three methods that return
//! `AppError::internal("not_implemented")`. The endpoints are wired so the
//! frontend can already see HTTP 500 with a stable error shape and start
//! building against the contract.
//!
//! Real implementations land in subsequent PRs:
//!   * PR #2 — `register_image` + `get_asset` against a storage adapter
//!   * PR #3 — Gemini Vision integration inside `generate_model_from_image`
//!   * PR #4 — geometry generators (`flat_card`, `sauce_in_bowl`)

use sqlx::PgPool;
use uuid::Uuid;

use super::models::{Laboratory3DAsset, LaboratoryImage, RegisterImagePayload};
use crate::shared::AppError;

#[derive(Clone)]
pub struct LaboratoryV2Service {
    #[allow(dead_code)] // wired to PgPool in PR #2
    pool: PgPool,
}

impl LaboratoryV2Service {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// `POST /api/laboratory/images`
    ///
    /// Registers a new source image for the user. The actual file upload
    /// (multipart) is handled in the HTTP layer; this method takes the URL
    /// produced by the storage adapter and persists the row.
    pub async fn register_image(
        &self,
        _user_id: Uuid,
        _payload: RegisterImagePayload,
    ) -> Result<LaboratoryImage, AppError> {
        Err(AppError::internal(
            "not_implemented: register_image lands in PR #2",
        ))
    }

    /// `POST /api/laboratory/images/:image_id/generate-model`
    ///
    /// Synchronous (MVP): runs the full pipeline and returns the final asset.
    ///
    /// 1. Load image record (404 if missing or wrong owner).
    /// 2. Insert `laboratory_3d_assets` row with `status = pending`.
    /// 3. Update status → `analyzing_image`, fetch image bytes,
    ///    call Gemini Vision → `Product3DSpec`, persist `object_spec_json`.
    /// 4. Update status → `generating_model`, dispatch to a geometry
    ///    generator, upload OBJ/GLB to storage, save `model_url`.
    /// 5. Update status → `ready`, return.
    /// On any error: status → `failed`, populate `error_message`, return Err.
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
    ///
    /// Returns the asset with its source image URL embedded for convenience.
    pub async fn get_asset(
        &self,
        _asset_id: Uuid,
        _user_id: Uuid,
    ) -> Result<Laboratory3DAsset, AppError> {
        Err(AppError::internal(
            "not_implemented: get_asset lands in PR #2",
        ))
    }
}
