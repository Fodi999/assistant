//! Catalog profile adapter — read-only, fault-tolerant gateway from the
//! `products` catalog (and its 6+ side tables) to a single, flat
//! `LaboratoryIngredientProfile` DTO.
//!
//! Design principles
//! -----------------
//! 1. **Read-only.** No business logic, no scoring, no engine work — that
//!    lives in `process_engine` / `shelf_life_engine` (Step 4).
//! 2. **Fault-tolerant.** Missing side rows / malformed JSONB / dropped
//!    tables must NOT break the profile. Worst case: a field stays `None`.
//! 3. **Batched.** Always loads N slugs in O(8) queries, not O(8·N).
//! 4. **Soft-ref to slug.** `slug` is the contract with
//!    `lab_project_ingredients.ingredient_slug`. Unknown slugs are silently
//!    dropped from the result.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use uuid::Uuid;

use crate::shared::{AppError, AppResult, Language};

// ─────────────────────────────────────────────────────────────────────────────
// DTOs
// ─────────────────────────────────────────────────────────────────────────────

/// One culinary behavior parsed from a single element of
/// `product_culinary_behavior.behaviors` (JSONB array). All fields are
/// optional so a malformed row degrades gracefully — we drop the element
/// only if it has *no* useful information at all.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LaboratoryCulinaryBehavior {
    /// Stable i18n key from DB (`softens_quickly`, `caramelizes`, …).
    /// Mapped from `key` for symmetry with the rest of the DTO.
    pub title: Option<String>,
    /// `texture` | `flavor` | `aroma` | `color` | …  (DB column: `type`).
    pub category: Option<String>,
    /// `heat` | `acid` | `shear` | `time` | …
    pub trigger: Option<String>,
    /// `softening` | `moisture_release` | `caramelization` | …
    pub effect: Option<String>,
    /// 0.0 .. 1.0 — strength of the effect.
    pub intensity: Option<f64>,
    /// Temperature where the behavior kicks in (°C). DB column:
    /// `temp_threshold`. `None` = behavior is unconditional.
    pub temperature_c: Option<f64>,
    /// 0.0 .. 1.0 — how confident the source is.
    pub confidence: Option<f64>,
    /// Other slugs this behavior interacts with.
    pub targets: Vec<String>,
}

/// Flat, engine-friendly ingredient profile. Mirrors source DB column names
/// (e.g. `calories_kcal`, not `calories_per_100g`) so engine code stays
/// honest about the unit. Every field is optional except `slug` and `name`.
#[derive(Debug, Clone, Serialize, Default)]
pub struct LaboratoryIngredientProfile {
    pub slug: String,
    pub name: String,
    /// Source: `products.product_type` (e.g. `fruit`, `dairy`, `meat`, …).
    pub category: Option<String>,

    // ── products ─────────────────────────────────────────────────────────
    pub density_g_per_ml: Option<f64>,
    pub shelf_life_days: Option<i32>,

    // ── nutrition_macros (per 100 g) ─────────────────────────────────────
    pub calories_kcal: Option<f64>,
    pub protein_g: Option<f64>,
    pub fat_g: Option<f64>,
    pub carbs_g: Option<f64>,
    pub water_g: Option<f64>,

    // ── food_properties ──────────────────────────────────────────────────
    pub ph: Option<f64>,
    pub water_activity: Option<f64>,
    pub smoke_point: Option<f64>,
    pub glycemic_index: Option<f64>,

    // ── food_culinary_properties (sensory profile) ───────────────────────
    pub sweetness: Option<f64>,
    pub acidity: Option<f64>,
    pub bitterness: Option<f64>,
    pub umami: Option<f64>,
    pub aroma: Option<f64>,
    pub texture: Option<String>,

    // ── product_culinary_behavior ────────────────────────────────────────
    pub culinary_behaviors: Vec<LaboratoryCulinaryBehavior>,

    // ── product_processing_effects ───────────────────────────────────────
    pub maillard_temp: Option<f64>,
    pub protein_denature_temp: Option<f64>,
    pub vitamin_retention_pct: Option<f64>,
    pub best_cooking_method: Option<String>,

    // ── product_health_profile ───────────────────────────────────────────
    pub food_role: Option<String>,
    pub bioactive_compounds: Vec<String>,
    pub contraindications: Vec<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Adapter
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct CatalogProfileAdapter {
    pool: PgPool,
}

impl CatalogProfileAdapter {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Convenience for single-slug lookup.
    /// (Future endpoint: `GET /api/laboratory/catalog/:slug/profile`.)
    pub async fn get_profile_by_slug(
        &self,
        slug: &str,
        language: Language,
    ) -> AppResult<Option<LaboratoryIngredientProfile>> {
        let mut v = self
            .get_profiles_by_slugs(&[slug.to_string()], language)
            .await?;
        Ok(v.pop())
    }

    /// Batch fetch. Returns one profile per slug found in `products`. Slugs
    /// missing from the catalog are silently skipped.
    pub async fn get_profiles_by_slugs(
        &self,
        slugs: &[String],
        language: Language,
    ) -> AppResult<Vec<LaboratoryIngredientProfile>> {
        if slugs.is_empty() {
            return Ok(Vec::new());
        }

        // Normalize: trim + lowercase, dedupe (preserve first occurrence).
        let normalized: Vec<String> = {
            let mut seen = std::collections::HashSet::new();
            slugs
                .iter()
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty() && seen.insert(s.clone()))
                .collect()
        };
        if normalized.is_empty() {
            return Ok(Vec::new());
        }

        // Anchor query — must succeed.
        let products = self.fetch_products(&normalized, language).await?;
        if products.is_empty() {
            return Ok(Vec::new());
        }
        let ids: Vec<Uuid> = products.iter().map(|p| p.id).collect();

        // Side tables — all best-effort, run concurrently.
        let (macros, food_props, culinary, behaviors, processing, health) = tokio::join!(
            self.fetch_macros(&ids),
            self.fetch_food_properties(&ids),
            self.fetch_culinary_properties(&ids),
            self.fetch_culinary_behaviors(&ids),
            self.fetch_processing_effects(&ids),
            self.fetch_health_profile(&ids),
        );

        let mut by_slug: HashMap<String, LaboratoryIngredientProfile> =
            HashMap::with_capacity(products.len());
        for p in products {
            let mut profile = LaboratoryIngredientProfile {
                slug: p.slug.clone(),
                name: p.name,
                category: p.product_type,
                density_g_per_ml: p.density_g_per_ml,
                shelf_life_days: p.shelf_life_days,
                ..Default::default()
            };

            if let Some(m) = macros.get(&p.id) {
                profile.calories_kcal = m.calories_kcal;
                profile.protein_g = m.protein_g;
                profile.fat_g = m.fat_g;
                profile.carbs_g = m.carbs_g;
                profile.water_g = m.water_g;
            }
            if let Some(fp) = food_props.get(&p.id) {
                profile.ph = fp.ph;
                profile.water_activity = fp.water_activity;
                profile.smoke_point = fp.smoke_point;
                profile.glycemic_index = fp.glycemic_index;
            }
            if let Some(cp) = culinary.get(&p.id) {
                profile.sweetness = cp.sweetness;
                profile.acidity = cp.acidity;
                profile.bitterness = cp.bitterness;
                profile.umami = cp.umami;
                profile.aroma = cp.aroma;
                profile.texture = cp.texture.clone();
            }
            if let Some(b) = behaviors.get(&p.id) {
                profile.culinary_behaviors = b.clone();
            }
            if let Some(pe) = processing.get(&p.id) {
                profile.maillard_temp = pe.maillard_temp;
                profile.protein_denature_temp = pe.protein_denature_temp;
                profile.vitamin_retention_pct = pe.vitamin_retention_pct;
                profile.best_cooking_method = pe.best_cooking_method.clone();
            }
            if let Some(h) = health.get(&p.id) {
                profile.food_role = h.food_role.clone();
                profile.bioactive_compounds = h.bioactive_compounds.clone();
                profile.contraindications = h.contraindications.clone();
            }

            by_slug.insert(p.slug, profile);
        }

        // Reorder to match input.
        Ok(normalized
            .iter()
            .filter_map(|s| by_slug.remove(s))
            .collect())
    }

    // ── helpers ──────────────────────────────────────────────────────────────

    async fn fetch_products(
        &self,
        slugs: &[String],
        language: Language,
    ) -> AppResult<Vec<ProductHead>> {
        let name_col = match language {
            Language::Pl => "name_pl",
            Language::En => "name_en",
            Language::Uk => "name_uk",
            Language::Ru => "name_ru",
        };
        let sql = format!(
            r#"
            SELECT id,
                   slug,
                   COALESCE({name_col}, name_en, slug) AS name,
                   product_type,
                   density_g_per_ml,
                   shelf_life_days
            FROM products
            WHERE slug = ANY($1)
            "#,
        );
        let rows = sqlx::query(&sql)
            .bind(slugs)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("fetch_products: {e}")))?;

        Ok(rows
            .into_iter()
            .map(|r| ProductHead {
                id: r.get("id"),
                slug: r.get("slug"),
                name: r.get("name"),
                product_type: r.try_get("product_type").ok().flatten(),
                density_g_per_ml: opt_real(&r, "density_g_per_ml"),
                shelf_life_days: r.try_get("shelf_life_days").ok().flatten(),
            })
            .collect())
    }

    async fn fetch_macros(&self, ids: &[Uuid]) -> HashMap<Uuid, MacrosRow> {
        let sql = r#"
            SELECT product_id, calories_kcal, protein_g, fat_g, carbs_g, water_g
            FROM nutrition_macros
            WHERE product_id = ANY($1)
        "#;
        match sqlx::query(sql).bind(ids).fetch_all(&self.pool).await {
            Ok(rows) => rows
                .into_iter()
                .map(|r| {
                    (
                        r.get("product_id"),
                        MacrosRow {
                            calories_kcal: opt_real(&r, "calories_kcal"),
                            protein_g: opt_real(&r, "protein_g"),
                            fat_g: opt_real(&r, "fat_g"),
                            carbs_g: opt_real(&r, "carbs_g"),
                            water_g: opt_real(&r, "water_g"),
                        },
                    )
                })
                .collect(),
            Err(e) => {
                tracing::warn!("fetch_macros failed (non-fatal): {e}");
                HashMap::new()
            }
        }
    }

    async fn fetch_food_properties(&self, ids: &[Uuid]) -> HashMap<Uuid, FoodPropsRow> {
        let sql = r#"
            SELECT product_id, ph, water_activity, smoke_point, glycemic_index
            FROM food_properties
            WHERE product_id = ANY($1)
        "#;
        match sqlx::query(sql).bind(ids).fetch_all(&self.pool).await {
            Ok(rows) => rows
                .into_iter()
                .map(|r| {
                    (
                        r.get("product_id"),
                        FoodPropsRow {
                            ph: opt_real(&r, "ph"),
                            water_activity: opt_real(&r, "water_activity"),
                            smoke_point: opt_real(&r, "smoke_point"),
                            glycemic_index: opt_real(&r, "glycemic_index"),
                        },
                    )
                })
                .collect(),
            Err(e) => {
                tracing::warn!("fetch_food_properties failed (non-fatal): {e}");
                HashMap::new()
            }
        }
    }

    async fn fetch_culinary_properties(&self, ids: &[Uuid]) -> HashMap<Uuid, CulinaryPropsRow> {
        let sql = r#"
            SELECT product_id, sweetness, acidity, bitterness, umami, aroma, texture
            FROM food_culinary_properties
            WHERE product_id = ANY($1)
        "#;
        match sqlx::query(sql).bind(ids).fetch_all(&self.pool).await {
            Ok(rows) => rows
                .into_iter()
                .map(|r| {
                    (
                        r.get("product_id"),
                        CulinaryPropsRow {
                            sweetness: opt_real(&r, "sweetness"),
                            acidity: opt_real(&r, "acidity"),
                            bitterness: opt_real(&r, "bitterness"),
                            umami: opt_real(&r, "umami"),
                            aroma: opt_real(&r, "aroma"),
                            texture: r.try_get("texture").ok().flatten(),
                        },
                    )
                })
                .collect(),
            Err(e) => {
                tracing::warn!("fetch_culinary_properties failed (non-fatal): {e}");
                HashMap::new()
            }
        }
    }

    async fn fetch_culinary_behaviors(
        &self,
        ids: &[Uuid],
    ) -> HashMap<Uuid, Vec<LaboratoryCulinaryBehavior>> {
        let sql = r#"
            SELECT product_id, behaviors
            FROM product_culinary_behavior
            WHERE product_id = ANY($1)
        "#;
        match sqlx::query(sql).bind(ids).fetch_all(&self.pool).await {
            Ok(rows) => rows
                .into_iter()
                .map(|r| {
                    let id: Uuid = r.get("product_id");
                    let raw: Option<JsonValue> = r.try_get("behaviors").ok().flatten();
                    (id, parse_behaviors(raw))
                })
                .collect(),
            Err(e) => {
                tracing::warn!("fetch_culinary_behaviors failed (non-fatal): {e}");
                HashMap::new()
            }
        }
    }

    async fn fetch_processing_effects(&self, ids: &[Uuid]) -> HashMap<Uuid, ProcessingRow> {
        let sql = r#"
            SELECT product_id, vitamin_retention_pct, protein_denature_temp,
                   best_cooking_method_en, maillard_temp
            FROM product_processing_effects
            WHERE product_id = ANY($1)
        "#;
        match sqlx::query(sql).bind(ids).fetch_all(&self.pool).await {
            Ok(rows) => rows
                .into_iter()
                .map(|r| {
                    (
                        r.get("product_id"),
                        ProcessingRow {
                            maillard_temp: opt_real(&r, "maillard_temp"),
                            protein_denature_temp: opt_real(&r, "protein_denature_temp"),
                            vitamin_retention_pct: opt_real(&r, "vitamin_retention_pct"),
                            best_cooking_method: r
                                .try_get("best_cooking_method_en")
                                .ok()
                                .flatten(),
                        },
                    )
                })
                .collect(),
            Err(e) => {
                tracing::warn!("fetch_processing_effects failed (non-fatal): {e}");
                HashMap::new()
            }
        }
    }

    async fn fetch_health_profile(&self, ids: &[Uuid]) -> HashMap<Uuid, HealthRow> {
        let sql = r#"
            SELECT product_id, food_role, bioactive_compounds_en, contraindications_en
            FROM product_health_profile
            WHERE product_id = ANY($1)
        "#;
        match sqlx::query(sql).bind(ids).fetch_all(&self.pool).await {
            Ok(rows) => rows
                .into_iter()
                .map(|r| {
                    (
                        r.get("product_id"),
                        HealthRow {
                            food_role: r.try_get("food_role").ok().flatten(),
                            bioactive_compounds: parse_string_array(
                                r.try_get("bioactive_compounds_en").ok().flatten(),
                            ),
                            contraindications: parse_string_array(
                                r.try_get("contraindications_en").ok().flatten(),
                            ),
                        },
                    )
                })
                .collect(),
            Err(e) => {
                tracing::warn!("fetch_health_profile failed (non-fatal): {e}");
                HashMap::new()
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal row types (private)
// ─────────────────────────────────────────────────────────────────────────────

struct ProductHead {
    id: Uuid,
    slug: String,
    name: String,
    product_type: Option<String>,
    density_g_per_ml: Option<f64>,
    shelf_life_days: Option<i32>,
}

struct MacrosRow {
    calories_kcal: Option<f64>,
    protein_g: Option<f64>,
    fat_g: Option<f64>,
    carbs_g: Option<f64>,
    water_g: Option<f64>,
}

struct FoodPropsRow {
    ph: Option<f64>,
    water_activity: Option<f64>,
    smoke_point: Option<f64>,
    glycemic_index: Option<f64>,
}

struct CulinaryPropsRow {
    sweetness: Option<f64>,
    acidity: Option<f64>,
    bitterness: Option<f64>,
    umami: Option<f64>,
    aroma: Option<f64>,
    texture: Option<String>,
}

struct ProcessingRow {
    maillard_temp: Option<f64>,
    protein_denature_temp: Option<f64>,
    vitamin_retention_pct: Option<f64>,
    best_cooking_method: Option<String>,
}

struct HealthRow {
    food_role: Option<String>,
    bioactive_compounds: Vec<String>,
    contraindications: Vec<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Soft parsing helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Decode an optional `REAL` Postgres column into `Option<f64>`. sqlx maps
/// `REAL` to `f32`, so we widen here and treat any decode error as `None`.
fn opt_real(row: &sqlx::postgres::PgRow, col: &str) -> Option<f64> {
    row.try_get::<Option<f32>, _>(col)
        .ok()
        .flatten()
        .map(|v| v as f64)
}

/// Parse `behaviors` JSONB into our DTO.
///
/// Defensive rules:
///   • non-array → `[]`
///   • element with no extractable info → drop
///   • partial element → keep with `None` elsewhere
///   • `temp_threshold` may be number OR numeric string — both accepted
fn parse_behaviors(raw: Option<JsonValue>) -> Vec<LaboratoryCulinaryBehavior> {
    let arr = match raw {
        Some(JsonValue::Array(a)) => a,
        _ => return Vec::new(),
    };
    arr.into_iter()
        .filter_map(|item| {
            let obj = item.as_object()?;

            let title = obj
                .get("key")
                .or_else(|| obj.get("title"))
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());

            let category = obj
                .get("type")
                .or_else(|| obj.get("category"))
                .and_then(|v| v.as_str())
                .map(String::from);
            let trigger = obj.get("trigger").and_then(|v| v.as_str()).map(String::from);
            let effect = obj.get("effect").and_then(|v| v.as_str()).map(String::from);
            let intensity = obj.get("intensity").and_then(json_as_f64);
            let temperature_c = obj
                .get("temp_threshold")
                .or_else(|| obj.get("temperature_c"))
                .and_then(json_as_f64);
            let confidence = obj.get("confidence").and_then(json_as_f64);
            let targets = obj
                .get("targets")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|t| t.as_str().map(|s| s.to_string()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            let any_data = title.is_some()
                || category.is_some()
                || trigger.is_some()
                || effect.is_some()
                || intensity.is_some()
                || temperature_c.is_some()
                || confidence.is_some()
                || !targets.is_empty();
            if !any_data {
                return None;
            }

            Some(LaboratoryCulinaryBehavior {
                title,
                category,
                trigger,
                effect,
                intensity,
                temperature_c,
                confidence,
                targets,
            })
        })
        .collect()
}

/// Parse a JSONB column expected to hold `["str", "str", ...]`.
fn parse_string_array(raw: Option<JsonValue>) -> Vec<String> {
    match raw {
        Some(JsonValue::Array(a)) => a
            .into_iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        _ => Vec::new(),
    }
}

/// Accept JSON number OR numeric string ("70", "0.8") — old data sometimes
/// stores thresholds as strings.
fn json_as_f64(v: &JsonValue) -> Option<f64> {
    v.as_f64()
        .or_else(|| v.as_i64().map(|i| i as f64))
        .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()))
}
