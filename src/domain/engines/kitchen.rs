//! KitchenEngine — trait definition.
//!
//! Handles yield, cost, equivalents, suggestions, popular conversions.

use async_trait::async_trait;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ScaleRecipeRequest {
    pub value:         f64,
    pub from_portions: f64,
    pub to_portions:   f64,
}

#[derive(Debug, Deserialize)]
pub struct YieldRequest {
    pub raw_weight:    f64,
    pub usable_weight: f64,
}

#[derive(Debug, Deserialize)]
pub struct FoodCostRequest {
    pub price:      f64,
    pub amount:     f64,
    pub portions:   Option<f64>,
    pub sell_price: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct EquivalentsRequest {
    pub ingredient: String,
    pub value:      Option<f64>,
    pub unit:       Option<String>,
    pub lang:       Option<String>,
}

/// KitchenEngine: professional kitchen calculators.
#[async_trait]
pub trait KitchenEngine: Send + Sync {
    /// Scale recipe portions.
    async fn scale(&self, req: ScaleRecipeRequest) -> serde_json::Value;

    /// Cooking yield / waste calculator.
    async fn yield_calc(&self, req: YieldRequest) -> serde_json::Value;

    /// Ingredient equivalents across all units.
    async fn ingredient_equivalents(&self, req: EquivalentsRequest) -> serde_json::Value;

    /// Food cost, margin & markup.
    async fn food_cost(&self, req: FoodCostRequest) -> serde_json::Value;

    /// Suggest ingredients by volume unit.
    async fn ingredient_suggestions(&self, req: EquivalentsRequest) -> serde_json::Value;

    /// Popular conversions (SEO).
    async fn popular_conversions(&self, lang: Option<String>) -> serde_json::Value;
}
