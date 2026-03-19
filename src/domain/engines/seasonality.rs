//! SeasonalityEngine — trait definition.
//!
//! Handles all seasonality / calendar / fish operations.

use async_trait::async_trait;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SeasonalCalendarRequest {
    pub product_type: Option<String>,
    pub lang:         Option<String>,
    pub region:       Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BestInSeasonRequest {
    pub month:        Option<u32>,
    pub product_type: Option<String>,
    pub lang:         Option<String>,
    pub region:       Option<String>,
    pub limit:        Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ProductSeasonalityRequest {
    pub slug:   String,
    pub lang:   Option<String>,
    pub region: Option<String>,
}

/// SeasonalityEngine: calendar, fish, regions.
#[async_trait]
pub trait SeasonalityEngine: Send + Sync {
    /// Full seasonal calendar by product type.
    async fn seasonal_calendar(&self, req: SeasonalCalendarRequest) -> serde_json::Value;

    /// What's in season right now.
    async fn in_season_now(&self, req: SeasonalCalendarRequest) -> serde_json::Value;

    /// Single product seasonality.
    async fn product_seasonality(&self, req: ProductSeasonalityRequest) -> serde_json::Value;

    /// Best products for a given month.
    async fn best_in_season(&self, req: BestInSeasonRequest) -> serde_json::Value;

    /// All products with status for a given month.
    async fn products_by_month(&self, req: BestInSeasonRequest) -> serde_json::Value;

    /// Best products in season right now.
    async fn best_right_now(&self, req: BestInSeasonRequest) -> serde_json::Value;

    /// Fish seasonality (single fish).
    async fn fish_season(&self, req: SeasonalCalendarRequest) -> serde_json::Value;

    /// Full fish season table.
    async fn fish_season_table(&self, req: SeasonalCalendarRequest) -> serde_json::Value;

    /// Available regions.
    async fn list_regions(&self) -> serde_json::Value;

    /// Product search with seasonality.
    async fn product_search(&self, req: SeasonalCalendarRequest) -> serde_json::Value;
}
