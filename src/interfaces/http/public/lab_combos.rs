//! Public HTTP handlers for Lab Combo SEO pages.

use crate::application::lab_combos::{LabComboPage, LabComboService, LabComboSitemapEntry, PublicComboSlugQuery, RelatedCombo, RelatedCombosQuery};
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

/// GET /public/lab-combos/:slug/related?locale=en&limit=6
///
/// Related combos sharing at least 1 ingredient. For internal linking section.
pub async fn get_related_combos(
    State(svc): State<Arc<LabComboService>>,
    Path(slug): Path<String>,
    Query(query): Query<RelatedCombosQuery>,
) -> Result<Json<Vec<RelatedCombo>>, AppError> {
    let locale = query.locale.as_deref().unwrap_or("en");
    let limit = query.limit.unwrap_or(6).min(12);
    let combos = svc.get_related_combos(&slug, locale, limit).await?;
    Ok(Json(combos))
}

/// GET /public/lab-combos/:slug/also-cook?locale=en&limit=4
///
/// "People also cook" — combos with the same goal/meal but different ingredients.
/// Provides discovery-based internal linking (complement to related combos).
pub async fn get_also_cook(
    State(svc): State<Arc<LabComboService>>,
    Path(slug): Path<String>,
    Query(query): Query<RelatedCombosQuery>,
) -> Result<Json<Vec<RelatedCombo>>, AppError> {
    let locale = query.locale.as_deref().unwrap_or("en");
    let limit = query.limit.unwrap_or(4).min(8);
    let combos = svc.get_also_cook(&slug, locale, limit).await?;
    Ok(Json(combos))
}
