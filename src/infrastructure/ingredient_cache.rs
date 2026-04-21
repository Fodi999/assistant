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
    /// Processing states (raw, boiled, fried, …) with weight/water/fat changes.
    /// Loaded from `ingredient_states`. Keyed by state enum value ("boiled", etc.).
    pub states: Vec<CachedState>,
}

/// Processing state with cooking-loss data (from `ingredient_states`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedState {
    /// 'raw' | 'boiled' | 'fried' | 'baked' | 'grilled' | 'steamed' | 'smoked' | 'frozen' | 'dried' | 'pickled'
    pub state: String,
    /// Weight change % during cooking (negative = loss).
    pub weight_change_percent: Option<f32>,
    /// % of water lost.
    pub water_loss_percent: Option<f32>,
    /// Grams of oil absorbed per 100 g raw.
    pub oil_absorption_g: Option<f32>,
    /// Re-calculated kcal / 100 g in this state (may differ from raw).
    pub calories_per_100g: Option<f32>,
    pub protein_per_100g: Option<f32>,
    pub fat_per_100g: Option<f32>,
    pub carbs_per_100g: Option<f32>,
    /// Localized suffixes ("варёный" / "boiled").
    pub name_suffix_en: Option<String>,
    pub name_suffix_ru: Option<String>,
    pub name_suffix_pl: Option<String>,
    pub name_suffix_uk: Option<String>,
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

    /// Find a specific processing state (e.g. "boiled", "fried").
    pub fn state(&self, name: &str) -> Option<&CachedState> {
        self.states.iter().find(|s| s.state == name)
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

        // Load processing-state rows once and bucket them by ingredient slug.
        // Joined via `catalog_ingredients.id` which is referenced in
        // `ingredient_states.ingredient_id`. We select only slugs so the
        // merge below is O(n) in a HashMap.
        let state_rows = sqlx::query_as::<_, StateRow>(
            r#"
            SELECT
                ci.slug,
                s.state::TEXT                            as state,
                s.weight_change_percent::REAL            as weight_change_percent,
                s.water_loss_percent::REAL               as water_loss_percent,
                s.oil_absorption_g::REAL                 as oil_absorption_g,
                s.calories_per_100g::REAL                as calories_per_100g,
                s.protein_per_100g::REAL                 as protein_per_100g,
                s.fat_per_100g::REAL                     as fat_per_100g,
                s.carbs_per_100g::REAL                   as carbs_per_100g,
                s.name_suffix_en, s.name_suffix_ru,
                s.name_suffix_pl, s.name_suffix_uk
            FROM ingredient_states s
            JOIN catalog_ingredients ci ON ci.id = s.ingredient_id
            WHERE ci.slug IS NOT NULL
            "#,
        )
        .fetch_all(pool)
        .await?;

        let mut states_by_slug: HashMap<String, Vec<CachedState>> = HashMap::new();
        for sr in state_rows {
            if let Some(slug) = sr.slug {
                states_by_slug.entry(slug).or_default().push(CachedState {
                    state: sr.state,
                    weight_change_percent: sr.weight_change_percent,
                    water_loss_percent: sr.water_loss_percent,
                    oil_absorption_g: sr.oil_absorption_g,
                    calories_per_100g: sr.calories_per_100g,
                    protein_per_100g: sr.protein_per_100g,
                    fat_per_100g: sr.fat_per_100g,
                    carbs_per_100g: sr.carbs_per_100g,
                    name_suffix_en: sr.name_suffix_en,
                    name_suffix_ru: sr.name_suffix_ru,
                    name_suffix_pl: sr.name_suffix_pl,
                    name_suffix_uk: sr.name_suffix_uk,
                });
            }
        }

        let mut map = HashMap::with_capacity(rows.len());
        for row in rows {
            if let Some(slug) = row.slug {
                let behaviors: Vec<CachedBehavior> = row.behaviors_json
                    .and_then(|v| serde_json::from_value(v).ok())
                    .unwrap_or_default();
                let states = states_by_slug.remove(&slug).unwrap_or_default();
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
                        states,
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

#[derive(sqlx::FromRow)]
struct StateRow {
    slug: Option<String>,
    state: String,
    weight_change_percent: Option<f32>,
    water_loss_percent: Option<f32>,
    oil_absorption_g: Option<f32>,
    calories_per_100g: Option<f32>,
    protein_per_100g: Option<f32>,
    fat_per_100g: Option<f32>,
    carbs_per_100g: Option<f32>,
    name_suffix_en: Option<String>,
    name_suffix_ru: Option<String>,
    name_suffix_pl: Option<String>,
    name_suffix_uk: Option<String>,
}
