//! Public HTTP handlers for Lab Combo SEO pages.

use crate::application::lab_combos::{LabComboPage, LabComboService, LabComboSitemapEntry, PublicComboSlugQuery};
use crate::shared::AppError;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use std::sync::Arc;

/// GET /public/lab-combos/sitemap
///
/// Lightweight list of all published combo pages for sitemap generation.
pub async fn lab_combos_sitemap(
    State(svc): State<Arc<LabComboService>>,
) -> Result<Json<Vec<LabComboSitemapEntry>>, AppError> {
    let entries = svc.sitemap().await?;
    Ok(Json(entries))
}

/// GET /public/lab-combos/:slug?locale=en
///
/// Single published combo page with full SmartResponse + SEO metadata.
pub async fn get_lab_combo(
    State(svc): State<Arc<LabComboService>>,
    Path(slug): Path<String>,
    Query(query): Query<PublicComboSlugQuery>,
) -> Result<Json<LabComboPage>, AppError> {
    let locale = query.locale.as_deref().unwrap_or("en");
    let page = svc
        .get_published(&slug, locale)
        .await?
        .ok_or_else(|| AppError::not_found("combo page not found"))?;
    Ok(Json(page))
}
