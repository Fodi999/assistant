// ─── Types — shared data structures for Lab Combos ──────────────────────────

use deunicode::deunicode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── DB Types ─────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct LabComboPage {
    pub id: Uuid,
    pub slug: String,
    pub locale: String,
    pub ingredients: Vec<String>,
    pub goal: Option<String>,
    pub meal_type: Option<String>,
    pub diet: Option<String>,
    pub cooking_time: Option<String>,
    pub budget: Option<String>,
    pub cuisine: Option<String>,
    pub title: String,
    pub description: String,
    pub h1: String,
    pub intro: String,
    pub why_it_works: String,
    pub how_to_cook: serde_json::Value,
    pub optimization_tips: serde_json::Value,
    pub image_url: Option<String>,
    pub process_image_url: Option<String>,
    pub detail_image_url: Option<String>,
    pub smart_response: serde_json::Value,
    pub faq: serde_json::Value,
    // ── Pre-calculated nutrition ─────────────────────────────────────
    pub total_weight_g: f32,
    pub servings_count: i16,
    pub calories_total: f32,
    pub protein_total: f32,
    pub fat_total: f32,
    pub carbs_total: f32,
    pub fiber_total: f32,
    pub calories_per_serving: f32,
    pub protein_per_serving: f32,
    pub fat_per_serving: f32,
    pub carbs_per_serving: f32,
    pub fiber_per_serving: f32,
    // ── Structured ingredients (DB data, not AI) ─────────────────────
    pub structured_ingredients: serde_json::Value,
    // ─────────────────────────────────────────────────────────────────
    pub status: String,
    pub quality_score: i16,
    pub published_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Lightweight version for sitemap generation
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct LabComboSitemapEntry {
    pub slug: String,
    pub locale: String,
    pub updated_at: String,
    pub ingredients: Vec<String>,
    pub goal: Option<String>,
    pub meal_type: Option<String>,
}

/// Lightweight version for related combos (internal linking)
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct RelatedCombo {
    pub slug: String,
    pub title: String,
    pub ingredients: Vec<String>,
    pub goal: Option<String>,
    pub meal_type: Option<String>,
    pub image_url: Option<String>,
}

// ── Request/Query Types ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct GenerateComboRequest {
    pub ingredients: Vec<String>,
    pub locale: String,
    pub goal: Option<String>,
    pub meal_type: Option<String>,
    pub diet: Option<String>,
    pub cooking_time: Option<String>,
    pub budget: Option<String>,
    pub cuisine: Option<String>,
    /// 🧠 Dish name — the primary logic driver for recipe generation.
    #[serde(default)]
    pub dish_name: Option<String>,
    /// AI model override: "flash" or "pro"
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListCombosQuery {
    pub status: Option<String>,
    pub locale: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct PublicComboSlugQuery {
    pub locale: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RelatedCombosQuery {
    pub locale: Option<String>,
    pub limit: Option<i64>,
}

/// Partial update for admin editing.
#[derive(Debug, Deserialize)]
pub struct UpdateComboRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub h1: Option<String>,
    pub intro: Option<String>,
    pub why_it_works: Option<String>,
    pub image_url: Option<String>,
    pub process_image_url: Option<String>,
    pub detail_image_url: Option<String>,
}

// ── Slug Builder ─────────────────────────────────────────────────────────────

/// Build a deterministic, SEO-friendly slug from ingredients + context.
/// SHORT slug formula: `{ingredients}-{meal_type}` (max 60 chars)
pub fn combo_slug(
    ingredients: &[String],
    _goal: Option<&str>,
    meal_type: Option<&str>,
    _diet: Option<&str>,
    _cooking_time: Option<&str>,
    _budget: Option<&str>,
    _cuisine: Option<&str>,
) -> String {
    let mut sorted_ingredients: Vec<String> = ingredients
        .iter()
        .map(|s| {
            let clean = deunicode(s.trim())
                .to_lowercase()
                .replace(' ', "-")
                .replace('_', "-");
            clean
                .split('-')
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("-")
        })
        .collect();
    sorted_ingredients.sort();
    sorted_ingredients.dedup();

    let mut parts = sorted_ingredients;

    if let Some(m) = meal_type {
        parts.push(m.replace('_', "-"));
    }

    let slug = parts.join("-");
    if slug.len() > 60 {
        slug.chars()
            .take(60)
            .collect::<String>()
            .trim_end_matches('-')
            .to_string()
    } else {
        slug
    }
}
