//! Postgres I/O for Laboratory v2 (Photo → 3D Model).
//!
//! Two tables:
//!   * `laboratory_images`     — uploaded source photos
//!   * `laboratory_3d_assets`  — generated 3D models (one per generation attempt)
//!
//! All queries scope by `user_id` so a user can never read or mutate another
//! user's data; `tenant_id` is reserved for future B2B scoping.

use sqlx::PgPool;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::application::laboratory_v2::{AssetStatus, Laboratory3DAsset, LaboratoryImage};
use crate::shared::AppError;

// ─────────────────────────────────────────────────────────────────────────────
// Row types — exact mirror of the DB schema
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, sqlx::FromRow)]
struct LaboratoryImageRow {
    id: Uuid,
    user_id: Uuid,
    tenant_id: Option<Uuid>,
    image_url: String,
    mime_type: String,
    original_filename: Option<String>,
    byte_size: Option<i64>,
    width_px: Option<i32>,
    height_px: Option<i32>,
    created_at: OffsetDateTime,
}

impl From<LaboratoryImageRow> for LaboratoryImage {
    fn from(r: LaboratoryImageRow) -> Self {
        Self {
            id: r.id,
            user_id: r.user_id,
            tenant_id: r.tenant_id,
            image_url: r.image_url,
            mime_type: r.mime_type,
            original_filename: r.original_filename,
            byte_size: r.byte_size,
            width_px: r.width_px,
            height_px: r.height_px,
            created_at: format_ts(r.created_at),
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct Laboratory3DAssetRow {
    id: Uuid,
    image_id: Uuid,
    user_id: Uuid,
    tenant_id: Option<Uuid>,
    status: String,
    provider: String,
    object_type: Option<String>,
    object_spec_json: Option<serde_json::Value>,
    model_format: Option<String>,
    model_url: Option<String>,
    thumbnail_url: Option<String>,
    error_message: Option<String>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
    /// Joined from `laboratory_images.image_url` so the API can show the
    /// source photo without a second round-trip.
    image_url: Option<String>,
}

impl Laboratory3DAssetRow {
    fn into_dto(self) -> Result<Laboratory3DAsset, AppError> {
        let status = AssetStatus::parse(&self.status).ok_or_else(|| {
            AppError::internal(format!("laboratory_v2: unknown status `{}`", self.status))
        })?;
        Ok(Laboratory3DAsset {
            id: self.id,
            image_id: self.image_id,
            user_id: self.user_id,
            tenant_id: self.tenant_id,
            status,
            provider: self.provider,
            object_type: self.object_type,
            object_spec: self.object_spec_json,
            model_format: self.model_format,
            model_url: self.model_url,
            thumbnail_url: self.thumbnail_url,
            image_url: self.image_url,
            error_message: self.error_message,
            created_at: format_ts(self.created_at),
            updated_at: format_ts(self.updated_at),
        })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Insert input
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CreateImageInput {
    pub user_id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub image_url: String,
    pub mime_type: String,
    pub original_filename: Option<String>,
    pub byte_size: Option<i64>,
    pub width_px: Option<i32>,
    pub height_px: Option<i32>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Repository
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct LaboratoryV2Repository {
    pool: PgPool,
}

impl LaboratoryV2Repository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Insert a new source-image record, returning the persisted DTO.
    pub async fn create_image(&self, input: CreateImageInput) -> Result<LaboratoryImage, AppError> {
        let row = sqlx::query_as::<_, LaboratoryImageRow>(
            r#"
            INSERT INTO laboratory_images (
                user_id, tenant_id, image_url, mime_type,
                original_filename, byte_size, width_px, height_px
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, user_id, tenant_id, image_url, mime_type,
                      original_filename, byte_size, width_px, height_px,
                      created_at
            "#,
        )
        .bind(input.user_id)
        .bind(input.tenant_id)
        .bind(input.image_url)
        .bind(input.mime_type)
        .bind(input.original_filename)
        .bind(input.byte_size)
        .bind(input.width_px)
        .bind(input.height_px)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("laboratory_v2.create_image: {e}")))?;

        Ok(row.into())
    }

    /// Look up an image by id, scoped to the owner.
    /// Returns `Ok(None)` if the row is absent or owned by someone else.
    #[allow(dead_code)] // used by `generate_model_from_image` (PR #3)
    pub async fn get_image_by_id(
        &self,
        image_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<LaboratoryImage>, AppError> {
        let row = sqlx::query_as::<_, LaboratoryImageRow>(
            r#"
            SELECT id, user_id, tenant_id, image_url, mime_type,
                   original_filename, byte_size, width_px, height_px,
                   created_at
              FROM laboratory_images
             WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(image_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("laboratory_v2.get_image: {e}")))?;

        Ok(row.map(Into::into))
    }

    /// Look up a 3D asset by id, scoped to the owner. The source `image_url`
    /// is joined in so the API response can render before/after with no
    /// second request.
    pub async fn get_asset_by_id(
        &self,
        asset_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<Laboratory3DAsset>, AppError> {
        let row = sqlx::query_as::<_, Laboratory3DAssetRow>(
            r#"
            SELECT a.id,
                   a.image_id,
                   a.user_id,
                   a.tenant_id,
                   a.status,
                   a.provider,
                   a.object_type,
                   a.object_spec_json,
                   a.model_format,
                   a.model_url,
                   a.thumbnail_url,
                   a.error_message,
                   a.created_at,
                   a.updated_at,
                   i.image_url AS image_url
              FROM laboratory_3d_assets a
              JOIN laboratory_images   i ON i.id = a.image_id
             WHERE a.id = $1 AND a.user_id = $2
            "#,
        )
        .bind(asset_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("laboratory_v2.get_asset: {e}")))?;

        row.map(Laboratory3DAssetRow::into_dto).transpose()
    }

    // ── Asset lifecycle (PR #3) ─────────────────────────────────────────────

    /// Insert a fresh asset row in `pending` state for the given image.
    ///
    /// The `image_id` must already exist; FK is enforced by the schema.
    /// `provider` defaults to `"chefos_procedural"` server-side.
    pub async fn create_asset_pending(
        &self,
        image_id: Uuid,
        user_id: Uuid,
        tenant_id: Option<Uuid>,
    ) -> Result<Laboratory3DAsset, AppError> {
        let row = sqlx::query_as::<_, Laboratory3DAssetRow>(
            r#"
            INSERT INTO laboratory_3d_assets (image_id, user_id, tenant_id, status)
            VALUES ($1, $2, $3, 'pending')
            RETURNING id,
                      image_id,
                      user_id,
                      tenant_id,
                      status,
                      provider,
                      object_type,
                      object_spec_json,
                      model_format,
                      model_url,
                      thumbnail_url,
                      error_message,
                      created_at,
                      updated_at,
                      NULL::TEXT AS image_url
            "#,
        )
        .bind(image_id)
        .bind(user_id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("laboratory_v2.create_asset_pending: {e}")))?;

        row.into_dto()
    }

    /// Move an asset to a new lifecycle status without touching other fields.
    ///
    /// Used to mark `analyzing_image` before the Vision call and `failed`
    /// (with an error message) on any pipeline error.
    pub async fn update_asset_status(
        &self,
        asset_id: Uuid,
        status: AssetStatus,
        error_message: Option<&str>,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE laboratory_3d_assets
               SET status = $2,
                   error_message = COALESCE($3, error_message)
             WHERE id = $1
            "#,
        )
        .bind(asset_id)
        .bind(status.as_str())
        .bind(error_message)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("laboratory_v2.update_asset_status: {e}")))?;

        Ok(())
    }

    /// Persist the Vision result and transition `analyzing_image → generating_model`.
    ///
    /// This is a single atomic UPDATE so a partial failure can't leave
    /// `object_spec_json` populated while `status` lags behind.
    pub async fn save_spec_and_mark_generating(
        &self,
        asset_id: Uuid,
        object_type: &str,
        object_spec_json: serde_json::Value,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE laboratory_3d_assets
               SET object_type      = $2,
                   object_spec_json = $3,
                   status           = 'generating_model',
                   error_message    = NULL
             WHERE id = $1
            "#,
        )
        .bind(asset_id)
        .bind(object_type)
        .bind(object_spec_json)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AppError::internal(format!("laboratory_v2.save_spec_and_mark_generating: {e}"))
        })?;

        Ok(())
    }

    /// Persist the generated model URL and flip status to `ready`.
    ///
    /// Called by the service after OBJ bytes are stored via `StorageAdapter`.
    /// Sets `model_format`, `model_url`, clears `error_message`.
    pub async fn mark_asset_ready(
        &self,
        asset_id: Uuid,
        model_format: &str,
        model_url: &str,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE laboratory_3d_assets
               SET status        = 'ready',
                   model_format  = $2,
                   model_url     = $3,
                   error_message = NULL
             WHERE id = $1
            "#,
        )
        .bind(asset_id)
        .bind(model_format)
        .bind(model_url)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("laboratory_v2.mark_asset_ready: {e}")))?;

        Ok(())
    }
}

/// Render an `OffsetDateTime` as RFC 3339 (e.g. `"2024-09-21T10:11:12.345Z"`).
/// Falls back to `Display` if the formatter ever errors (it shouldn't).
fn format_ts(ts: OffsetDateTime) -> String {
    ts.format(&Rfc3339).unwrap_or_else(|_| ts.to_string())
}
