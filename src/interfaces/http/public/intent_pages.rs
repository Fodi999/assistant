//! Public Intent Pages handler
//!
//! GET /public/intent-pages?locale=en              → list published
//! GET /public/intent-pages/:slug?locale=en        → single published page

use axum::{
    extract::{Path, Query, State},
    Json,
};
use std::sync::Arc;

use crate::application::intent_pages::{IntentPage, IntentPagesService, PublicListQuery, PublicSlugQuery};
use crate::shared::AppError;

pub type IntentPagesPublicState = Arc<IntentPagesService>;

/// GET /public/intent-pages
pub async fn list_published_intent_pages(
    State(service): State<IntentPagesPublicState>,
    Query(q): Query<PublicListQuery>,
) -> Result<Json<Vec<IntentPage>>, AppError> {
    let pages = service.list_published(&q).await?;
    Ok(Json(pages))
}

/// GET /public/intent-pages/:slug
pub async fn get_published_intent_page(
    State(service): State<IntentPagesPublicState>,
    Path(slug): Path<String>,
    Query(q): Query<PublicSlugQuery>,
) -> Result<Json<IntentPage>, AppError> {
    let locale = q.locale.as_deref().unwrap_or("en");
    let page = service.get_by_slug(&slug, locale).await?;
    Ok(Json(page))
}
