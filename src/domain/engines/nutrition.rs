//! NutritionEngine — trait definition.
//!
//! Handles nutrition lookup, comparison, scoring, and ingredient database.

use async_trait::async_trait;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct NutritionRequest {
    pub ingredient: String,
    pub amount:     Option<f64>,
    pub unit:       Option<String>,
    pub lang:       Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CompareRequest {
    pub a:    String,
    pub b:    String,
    pub lang: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct IngredientsSearchRequest {
    pub q:     Option<String>,
    pub lang:  Option<String>,
    pub limit: Option<i64>,
}

/// NutritionEngine: all nutrition data operations.
#[async_trait]
pub trait NutritionEngine: Send + Sync {
    /// Get nutrition for an ingredient + amount + unit.
    async fn nutrition(&self, req: NutritionRequest) -> serde_json::Value;

    /// Compare two ingredients side by side.
    async fn compare(&self, req: CompareRequest) -> serde_json::Value;

    /// Search ingredients database.
    async fn search_ingredients(&self, req: IngredientsSearchRequest) -> serde_json::Value;

    /// Resolve a name to canonical slug.
    async fn resolve_slug(&self, name: &str) -> serde_json::Value;
}
