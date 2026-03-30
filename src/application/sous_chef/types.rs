//! Public types for Sous-Chef Planner.

use serde::{Deserialize, Serialize};

/// Incoming request from frontend.
#[derive(Debug, Deserialize)]
pub struct PlanRequest {
    pub query: String,
    pub lang: Option<String>,
}

/// One recipe variant in the plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MealVariant {
    pub level: String,
    pub emoji: String,
    pub title: String,
    pub short_description: String,
    pub calories: u32,
    pub protein_g: u32,
    pub fat_g: u32,
    pub carbs_g: u32,
    pub ingredients: Vec<MealIngredient>,
}

/// Single ingredient in a recipe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MealIngredient {
    pub name: String,
    pub amount: String,
    pub calories: u32,
    pub image_url: Option<String>,
}

/// Full plan response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MealPlan {
    pub cache_key: String,
    pub cached: bool,
    pub chef_intro: String,
    pub variants: Vec<MealVariant>,
    pub explanation: String,
    pub motivation: String,
    pub goal: String,
    pub lang: String,
}
