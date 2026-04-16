//! In-memory ingredient catalog — loaded once at startup, zero SQL in runtime.
//!
//! Usage:
//!   let cache = IngredientCache::load(&pool).await?;
//!   let chicken = cache.get("chicken-breast");
//!   // chicken.calories_per_100g = 165, protein = 31, image_url = Some(...)
//!
//! Invalidation:
//!   cache.reload(&pool).await?;   // after admin edits

use serde::{Serialize, Deserialize};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Nutritional + display data for a single ingredient.
#[derive(Debug, Clone)]
pub struct IngredientData {
    pub slug: String,
    pub name_en: String,
    pub name_ru: String,
    pub name_pl: String,
    pub name_uk: String,
    pub calories_per_100g: f32,
    pub protein_per_100g: f32,
    pub fat_per_100g: f32,
    pub carbs_per_100g: f32,
    pub image_url: Option<String>,
    /// DB `product_type`: seafood, vegetable, fruit, meat, grain, dairy, spice, herb, legume, nut, mushroom, oil, condiment, beverage, fish, other
    pub product_type: String,
    /// Density for unit conversion: grams per 1 ml (e.g. water=1.0, honey=1.42, flour=0.55)
    pub density_g_per_ml: Option<f32>,
    /// Structured culinary behaviors from product_culinary_behavior table
    pub behaviors: Vec<CachedBehavior>,
}

/// Lightweight behavior struct for in-memory cache (subset of full CookingBehavior)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedBehavior {
    pub key: String,
    #[serde(rename = "type")]
    pub behavior_type: String,
    #[serde(default)]
    pub effect: Option<String>,
    #[serde(default)]
    pub trigger: Option<String>,
    #[serde(default)]
    pub intensity: Option<f32>,
    #[serde(default)]
    pub temp_threshold: Option<f32>,
    #[serde(default)]
    pub targets: Vec<String>,
    #[serde(default)]
    pub polarity: Option<String>,
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub pairing_score: Option<f32>,
}

impl IngredientData {
    /// Calculate calories for a given weight in grams.
    pub fn kcal_for(&self, grams: f32) -> u32 {
        (self.calories_per_100g * grams / 100.0).round() as u32
    }

    /// Calculate protein for a given weight in grams.
    pub fn protein_for(&self, grams: f32) -> f32 {
        (self.protein_per_100g * grams / 100.0 * 10.0).round() / 10.0
    }

    /// Calculate fat for a given weight in grams.
    pub fn fat_for(&self, grams: f32) -> f32 {
        (self.fat_per_100g * grams / 100.0 * 10.0).round() / 10.0
    }

    /// Calculate carbs for a given weight in grams.
    pub fn carbs_for(&self, grams: f32) -> f32 {
        (self.carbs_per_100g * grams / 100.0 * 10.0).round() / 10.0
    }

    /// Localized name by language code.
    pub fn name(&self, lang: &str) -> &str {
        match lang {
            "ru" => &self.name_ru,
            "pl" => &self.name_pl,
            "uk" => &self.name_uk,
            _ => &self.name_en,
        }
    }

    /// Meal role classification based on `product_type`.
    /// Returns: "protein" | "side" | "base" | "other"
    pub fn meal_role(&self) -> &'static str {
        match self.product_type.as_str() {
            "meat" | "fish" | "seafood" => "protein",
            "dairy" if self.protein_per_100g >= 10.0 => "protein",  // cottage cheese, eggs
            "legume" if self.protein_per_100g >= 15.0 => "protein", // chickpeas, lentils
            "vegetable" | "mushroom" | "fruit" => "side",
            "grain" | "legume" => "base",
            _ => "other",
        }
    }
}

/// Thread-safe in-memory ingredient catalog.
/// Inner map: slug → IngredientData
#[derive(Clone)]
pub struct IngredientCache {
    data: Arc<RwLock<HashMap<String, IngredientData>>>,
}

impl IngredientCache {
    /// Create an empty cache (fallback if DB load fails — server still starts).
    pub fn empty() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Load all active ingredients from DB into memory. Call once at startup.
    pub async fn load(pool: &PgPool) -> Result<Self, sqlx::Error> {
        let map = Self::fetch_all(pool).await?;
        let count = map.len();
        tracing::info!("🧊 IngredientCache loaded: {} ingredients in memory", count);
        Ok(Self {
            data: Arc::new(RwLock::new(map)),
        })
    }

    /// Reload all data (call after admin edits).
    pub async fn reload(&self, pool: &PgPool) -> Result<(), sqlx::Error> {
        let map = Self::fetch_all(pool).await?;
        let count = map.len();
        let mut w = self.data.write().await;
        *w = map;
        tracing::info!("🔄 IngredientCache reloaded: {} ingredients", count);
        Ok(())
    }

    /// Get ingredient by slug. Returns None if not found.
    pub async fn get(&self, slug: &str) -> Option<IngredientData> {
        let r = self.data.read().await;
        r.get(slug).cloned()
    }

    /// Get multiple ingredients by slugs. Returns a map slug → data.
    pub async fn get_many(&self, slugs: &[&str]) -> HashMap<String, IngredientData> {
        let r = self.data.read().await;
        slugs
            .iter()
            .filter_map(|s| r.get(*s).map(|d| (s.to_string(), d.clone())))
            .collect()
    }

    /// Get ALL loaded ingredients as a Vec. O(n) clone — use for ranking/filtering.
    pub async fn all(&self) -> Vec<IngredientData> {
        let r = self.data.read().await;
        r.values().cloned().collect()
    }

    /// Get all loaded ingredients (for debugging).
    pub async fn len(&self) -> usize {
        self.data.read().await.len()
    }

    // ── Internal ─────────────────────────────────────────────────────────

    async fn fetch_all(pool: &PgPool) -> Result<HashMap<String, IngredientData>, sqlx::Error> {
        let rows = sqlx::query_as::<_, IngredientRow>(
            r#"
            SELECT
                ci.slug,
                ci.name_en, ci.name_ru, ci.name_pl, ci.name_uk,
                COALESCE(ci.calories_per_100g, 0)::REAL as calories_per_100g,
                COALESCE(ci.protein_per_100g, 0)::REAL  as protein_per_100g,
                COALESCE(ci.fat_per_100g, 0)::REAL      as fat_per_100g,
                COALESCE(ci.carbs_per_100g, 0)::REAL    as carbs_per_100g,
                ci.image_url,
                COALESCE(ci.product_type, 'other')      as product_type,
                ci.density_g_per_ml::REAL               as density_g_per_ml,
                pcb.behaviors as behaviors_json
            FROM catalog_ingredients ci
            LEFT JOIN product_culinary_behavior pcb ON pcb.product_id = ci.id
            WHERE COALESCE(ci.is_active, true) = true
              AND ci.slug IS NOT NULL
            ORDER BY ci.slug
            "#,
        )
        .fetch_all(pool)
        .await?;

        let mut map = HashMap::with_capacity(rows.len());
        for row in rows {
            if let Some(slug) = row.slug {
                let behaviors: Vec<CachedBehavior> = row.behaviors_json
                    .and_then(|v| serde_json::from_value(v).ok())
                    .unwrap_or_default();
                map.insert(
                    slug.clone(),
                    IngredientData {
                        slug,
                        name_en: row.name_en,
                        name_ru: row.name_ru,
                        name_pl: row.name_pl,
                        name_uk: row.name_uk,
                        calories_per_100g: row.calories_per_100g,
                        protein_per_100g: row.protein_per_100g,
                        fat_per_100g: row.fat_per_100g,
                        carbs_per_100g: row.carbs_per_100g,
                        image_url: row.image_url,
                        product_type: row.product_type,
                        density_g_per_ml: row.density_g_per_ml,
                        behaviors,
                    },
                );
            }
        }
        Ok(map)
    }
}

#[derive(sqlx::FromRow)]
struct IngredientRow {
    slug: Option<String>,
    name_en: String,
    name_ru: String,
    name_pl: String,
    name_uk: String,
    calories_per_100g: f32,
    protein_per_100g: f32,
    fat_per_100g: f32,
    carbs_per_100g: f32,
    image_url: Option<String>,
    product_type: String,
    density_g_per_ml: Option<f32>,
    behaviors_json: Option<serde_json::Value>,
}
