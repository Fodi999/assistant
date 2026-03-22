//! Public SEO Content handler
//!
//! GET /public/seo-content?intent_type=question&entity_a=salmon&locale=en
//! GET /public/seo-content?intent_type=comparison&entity_a=salmon&entity_b=tuna&locale=pl

use axum::{
    extract::{Query, State},
    Json,
};
use std::sync::Arc;

use crate::application::public_seo_content::{
    PublicSeoContentService, SeoContentRequest, SeoContentResponse,
};
use crate::shared::AppError;

pub type SeoContentState = Arc<PublicSeoContentService>;

/// GET /public/seo-content
pub async fn get_seo_content(
    State(service): State<SeoContentState>,
    Query(params): Query<SeoContentRequest>,
) -> Result<Json<SeoContentResponse>, AppError> {
    let result = service.generate(&params).await?;
    Ok(Json(result))
}
