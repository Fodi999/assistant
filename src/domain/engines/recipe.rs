//! RecipeEngine — trait definition.
//!
//! Handles recipe analysis, nutrition aggregation, cost, and sharing.

use async_trait::async_trait;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RecipeAnalyzeRequest {
    pub ingredients: Vec<RecipeIngredientItem>,
    pub portions:    Option<u32>,
    pub lang:        Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RecipeIngredientItem {
    pub slug:  String,
    pub grams: f64,
}

#[derive(Debug, Deserialize)]
pub struct ShareRecipeRequest {
    pub title:       String,
    pub ingredients: Vec<RecipeIngredientItem>,
    pub portions:    Option<u32>,
    pub lang:        Option<String>,
}

/// RecipeEngine: analysis, nutrition aggregation, cost, sharing.
#[async_trait]
pub trait RecipeEngine: Send + Sync {
    /// Full recipe analysis: nutrition + flavor + diet + rules.
    async fn analyze(&self, req: RecipeAnalyzeRequest) -> serde_json::Value;

    /// Calculate total nutrition for a list of ingredients.
    async fn recipe_nutrition(&self, req: RecipeAnalyzeRequest) -> serde_json::Value;

    /// Calculate recipe cost from ingredient weights.
    async fn recipe_cost(&self, req: RecipeAnalyzeRequest) -> serde_json::Value;

    /// Share a recipe (create shareable slug).
    async fn share(&self, req: ShareRecipeRequest) -> serde_json::Value;

    /// Retrieve shared recipe by slug.
    async fn get_shared(&self, slug: &str) -> serde_json::Value;
}
