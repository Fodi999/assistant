use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::application::prayer_visualizer;
use crate::infrastructure::R2Client;

use super::church_content::{db_error, effective_visualizer_image, ChurchContentQuery, ChurchPrayerDto, PRAYER_COLUMNS};
use super::site_context::CHURCH_SITE_ID;

const ASSET_COLUMNS: &str = "id, prayer_id, source_image_url, desktop_map_url, mobile_map_url, low_power_map_url, \
    fallback_image_url, thumbnail_url, desktop_particle_count, mobile_particle_count, low_power_particle_count, \
    processing_status, processing_error, processing_version, created_at::text AS created_at, updated_at::text AS updated_at";

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ChurchPrayerVisualizerAssetDto {
    pub id: Uuid,
    pub prayer_id: Uuid,
    pub source_image_url: String,
    pub desktop_map_url: String,
    pub mobile_map_url: String,
    pub low_power_map_url: String,
    pub fallback_image_url: String,
    pub thumbnail_url: String,
    pub desktop_particle_count: i32,
    pub mobile_particle_count: i32,
    pub low_power_particle_count: i32,
    pub processing_status: String,
    pub processing_error: String,
    pub processing_version: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// Admin: fetch the current processing status/URLs for a prayer's visualizer
/// asset (or `null` if nothing has ever been queued for it).
pub async fn get_prayer_visualizer_asset(
    Path(prayer_id): Path<Uuid>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let row = fetch_asset_by_prayer_id(&pool, prayer_id).await?;
    Ok(Json(row))
}

/// Admin: force a re-run of the processing pipeline (e.g. after a `failed`
/// status, or to pick up a new `PROCESSING_VERSION` without re-uploading the
/// image). Uses the prayer's current effective source image.
pub async fn reprocess_prayer_visualizer(
    Path(id): Path<Uuid>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
    Extension(r2): Extension<R2Client>,
) -> Result<impl IntoResponse, StatusCode> {
    let site_id = query.site_id();
    let prayer: ChurchPrayerDto = sqlx::query_as(&format!(
        "SELECT {PRAYER_COLUMNS} FROM church_prayers WHERE id = $1 AND site_id = $2"
    ))
    .bind(id)
    .bind(site_id)
    .fetch_optional(&pool)
    .await
    .map_err(db_error)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let source_image = effective_visualizer_image(&prayer);
    if source_image.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    prayer_visualizer::mark_pending(&pool, prayer.id, &source_image)
        .await
        .map_err(db_error)?;

    let pool_bg = pool.clone();
    tokio::spawn(async move {
        prayer_visualizer::run_processing_job(pool_bg, r2, prayer.id, source_image).await;
    });

    let row = fetch_asset_by_prayer_id(&pool, prayer.id).await?;
    Ok(Json(row))
}

/// Public: the prepared particle-map asset URLs for a published prayer,
/// looked up by slug (mirrors `public_prayer_by_slug`'s scoping rules).
/// Returns `null` (200) rather than 404 when nothing has been processed yet
/// — the frontend treats that as "fall back to client-side sampling".
pub async fn public_prayer_visualizer_asset(
    Path(slug): Path<String>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let prayer_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM church_prayers WHERE slug = $1 AND (site_id = $2 OR is_global = true) AND status = 'published' LIMIT 1",
    )
    .bind(&slug)
    .bind(CHURCH_SITE_ID)
    .fetch_optional(&pool)
    .await
    .map_err(db_error)?;

    let Some(prayer_id) = prayer_id else {
        return Ok(Json(None::<ChurchPrayerVisualizerAssetDto>));
    };

    let row = fetch_asset_by_prayer_id(&pool, prayer_id).await?;
    Ok(Json(row))
}

async fn fetch_asset_by_prayer_id(
    pool: &PgPool,
    prayer_id: Uuid,
) -> Result<Option<ChurchPrayerVisualizerAssetDto>, StatusCode> {
    sqlx::query_as::<_, ChurchPrayerVisualizerAssetDto>(&format!(
        "SELECT {ASSET_COLUMNS} FROM church_prayer_visualizer_assets WHERE prayer_id = $1"
    ))
    .bind(prayer_id)
    .fetch_optional(pool)
    .await
    .map_err(db_error)
}
