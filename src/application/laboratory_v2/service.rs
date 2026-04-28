//! `LaboratoryV2Service` — use cases for the Photo → 3D Model pipeline.
//!
//! Shipped layers:
//!   * PR #2 — `register_image` / `upload_and_register` / `get_asset`
//!   * PR #3 — `generate_model_from_image`: pending → analyzing_image →
//!             Gemini Vision → save spec → generating_model.
//!   * PR #4 — geometry dispatch → OBJ export → StorageAdapter → ready.

use std::path::PathBuf;
use std::sync::Arc;

use sqlx::PgPool;
use uuid::Uuid;

use super::models::{
    AssetStatus, Laboratory3DAsset, LaboratoryImage, RegisterImagePayload,
};
use crate::infrastructure::gemini::GeminiVision3D;
use crate::infrastructure::geometry::kernel::GeometryQuality;
use crate::infrastructure::geometry::{dispatch_with_quality as geometry_dispatch, export_glb};
use crate::infrastructure::persistence::laboratory_v2_repository::{
    CreateImageInput, LaboratoryV2Repository,
};
use crate::infrastructure::storage::StorageAdapter;
use crate::shared::AppError;

/// Mime types accepted by the multipart uploader.
const ALLOWED_MIME_TYPES: &[&str] = &["image/png", "image/jpeg", "image/webp"];

/// Hard cap on a single uploaded file (10 MB).
const MAX_UPLOAD_BYTES: usize = 10 * 1024 * 1024;

/// Where local files live on disk. Must match `LocalStorageAdapter::root_dir`
/// in `routes.rs`. Stored here so the service can read bytes back for Vision.
const LOCAL_UPLOADS_ROOT: &str = "./uploads";

/// Public URL prefix that points at `LOCAL_UPLOADS_ROOT` (mounted via ServeDir).
const LOCAL_PUBLIC_PREFIX: &str = "/static/";

#[derive(Clone)]
pub struct LaboratoryV2Service {
    repo: LaboratoryV2Repository,
    storage: Arc<dyn StorageAdapter>,
    vision: Arc<GeminiVision3D>,
    /// Reqwest client reused for fetching remote images during `generate_model_from_image`.
    http: reqwest::Client,
}

impl LaboratoryV2Service {
    pub fn new(
        pool: PgPool,
        storage: Arc<dyn StorageAdapter>,
        vision: Arc<GeminiVision3D>,
    ) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("failed to build laboratory_v2 http client");
        Self {
            repo: LaboratoryV2Repository::new(pool),
            storage,
            vision,
            http,
        }
    }

    // ── POST /api/laboratory/images (JSON variant) ──────────────────────────

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

    // ── POST /api/laboratory/images/:image_id/generate-model ────────────────
    //
    // PR #3 flow (no OBJ yet):
    //   1. load image (404 if missing / wrong owner)
    //   2. INSERT asset row (status = pending)
    //   3. UPDATE asset.status = analyzing_image
    //   4. fetch image bytes (./uploads or remote)
    //   5. Gemini Vision → Product3DSpec
    //   6. UPDATE asset SET object_type, object_spec_json, status = generating_model
    //
    // On any error after the row exists we mark it `failed` with the message.
    pub async fn generate_model_from_image(
        &self,
        image_id: Uuid,
        user_id: Uuid,
    ) -> Result<Laboratory3DAsset, AppError> {
        self.generate_model_from_image_with_quality(
            image_id,
            user_id,
            GeometryQuality::default(),
        )
        .await
    }

    /// Same as [`generate_model_from_image`] but with explicit
    /// [`GeometryQuality`]. Studio default is `High`; final exports use
    /// `Ultra`. Frontend `RenderQuality` is independent and acts on the
    /// viewer canvas (DPR / shadows / AA), not on the GLB topology.
    pub async fn generate_model_from_image_with_quality(
        &self,
        image_id: Uuid,
        user_id: Uuid,
        quality: GeometryQuality,
    ) -> Result<Laboratory3DAsset, AppError> {
        // 1. Load image
        let image = self
            .repo
            .get_image_by_id(image_id, user_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("image {image_id} not found")))?;

        // 2. Create pending asset
        let asset = self
            .repo
            .create_asset_pending(image.id, user_id, image.tenant_id)
            .await?;
        let asset_id = asset.id;

        // From here on every error path must mark the row as failed.
        match self.run_vision_pipeline(&image, asset_id, quality).await {
            Ok(()) => {
                // 6. Re-read so the response carries the joined image_url + spec.
                self.repo
                    .get_asset_by_id(asset_id, user_id)
                    .await?
                    .ok_or_else(|| {
                        AppError::internal(format!(
                            "asset {asset_id} disappeared after generate-model"
                        ))
                    })
            }
            Err(err) => {
                let msg = err.to_string();
                if let Err(persist_err) = self
                    .repo
                    .update_asset_status(asset_id, AssetStatus::Failed, Some(&msg))
                    .await
                {
                    tracing::error!(
                        "laboratory_v2: failed to mark asset {asset_id} as failed: {persist_err}"
                    );
                }
                Err(err)
            }
        }
    }

    /// 3-6: status → analyzing_image → Vision → save spec → generating_model
    ///      → geometry dispatch → OBJ store → ready.
    async fn run_vision_pipeline(
        &self,
        image: &LaboratoryImage,
        asset_id: Uuid,
        quality: GeometryQuality,
    ) -> Result<(), AppError> {
        // 3. analyzing_image
        self.repo
            .update_asset_status(asset_id, AssetStatus::AnalyzingImage, None)
            .await?;

        // 4. Fetch image bytes
        let bytes = self.fetch_image_bytes(&image.image_url).await?;

        // 5. Gemini Vision → spec
        let spec = self
            .vision
            .analyze_image_for_3d(bytes, &image.mime_type)
            .await?;

        let effective = spec.effective_object_type();
        let spec_json = serde_json::to_value(&spec).map_err(|e| {
            AppError::internal(format!("laboratory_v2: spec serialize: {e}"))
        })?;

        // 6. save spec + status = generating_model
        self.repo
            .save_spec_and_mark_generating(asset_id, effective.as_str(), spec_json.clone())
            .await?;

        // 7. Geometry dispatch → Mesh → GLB (geometry quality drives segment
        // counts and ring counts on heightfield surfaces).
        let mesh = geometry_dispatch(effective.as_str(), Some(&spec_json), quality)?;
        let export = export_glb(&mesh)?;

        // 8. Store .glb (single self-contained file with embedded PBR materials)
        let glb_key = format!("laboratory/models/{asset_id}/model.glb");
        let model_url = self
            .storage
            .put_bytes(&glb_key, export.glb_bytes, "model/gltf-binary")
            .await?;

        // 9. Mark ready
        self.repo
            .mark_asset_ready(asset_id, "glb", &model_url)
            .await?;

        tracing::info!(
            "✅ laboratory_v2: asset {asset_id} ready — object_type={} quality={} model_url={model_url}",
            effective.as_str(),
            quality.as_str()
        );

        Ok(())
    }

    /// Read the image bytes referenced by `image_url`.
    ///
    ///   * `/static/<key>` (and any URL ending with `/static/<key>`) → read from `./uploads/<key>`
    ///   * `http(s)://…` → reqwest GET
    ///   * other relative paths → rejected
    async fn fetch_image_bytes(&self, image_url: &str) -> Result<Vec<u8>, AppError> {
        if let Some(rel_key) = local_static_key(image_url) {
            let path = PathBuf::from(LOCAL_UPLOADS_ROOT).join(rel_key);
            tracing::debug!("laboratory_v2: reading local image from {path:?}");
            return tokio::fs::read(&path).await.map_err(|e| {
                AppError::internal(format!(
                    "laboratory_v2: read local image {path:?}: {e}"
                ))
            });
        }

        if image_url.starts_with("http://") || image_url.starts_with("https://") {
            tracing::debug!("laboratory_v2: fetching remote image {image_url}");
            let res = self
                .http
                .get(image_url)
                .send()
                .await
                .map_err(|e| AppError::internal(format!(
                    "laboratory_v2: GET {image_url}: {e}"
                )))?;
            if !res.status().is_success() {
                return Err(AppError::internal(format!(
                    "laboratory_v2: GET {image_url} returned {}",
                    res.status()
                )));
            }
            return res
                .bytes()
                .await
                .map(|b| b.to_vec())
                .map_err(|e| AppError::internal(format!(
                    "laboratory_v2: read body {image_url}: {e}"
                )));
        }

        Err(AppError::internal(format!(
            "laboratory_v2: unsupported image_url scheme: {image_url}"
        )))
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

/// If `url` points at `/static/<key>` (either as a relative path or an
/// absolute URL whose path starts with `/static/`), return `<key>`. The
/// returned slice is guaranteed not to start with `/` and not to contain
/// `..` segments, so it can be safely joined with `LOCAL_UPLOADS_ROOT`.
fn local_static_key(url: &str) -> Option<&str> {
    // Absolute URL → take path part.
    let path = if let Some(idx) = url.find("://") {
        let after_scheme = &url[idx + 3..];
        match after_scheme.find('/') {
            Some(i) => &after_scheme[i..],
            None => return None,
        }
    } else {
        url
    };

    let rest = path.strip_prefix(LOCAL_PUBLIC_PREFIX)?;
    if rest.is_empty() || rest.contains("..") {
        return None;
    }
    Some(rest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_static_key_handles_relative_and_absolute() {
        assert_eq!(
            local_static_key("/static/laboratory/images/abc.png"),
            Some("laboratory/images/abc.png")
        );
        assert_eq!(
            local_static_key("https://api.example.com/static/laboratory/images/abc.png"),
            Some("laboratory/images/abc.png")
        );
        assert_eq!(local_static_key("/static/"), None);
        assert_eq!(local_static_key("/static/../etc/passwd"), None);
        assert_eq!(local_static_key("https://cdn.example.com/foo.png"), None);
    }
}
