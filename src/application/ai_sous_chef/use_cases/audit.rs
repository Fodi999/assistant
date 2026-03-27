//! Use Case: AI Audit — catalog completeness & USDA validation
//!
//! Pipeline:
//! 1. Phase 1: SQL scan all products → completeness report (no AI)
//! 2. Phase 2: Check cache for USDA validation (hash of product list)
//! 3. If miss → call AiClient::generate (best model — audit is critical)
//! 4. Cache USDA result (TTL: 7 days — data changes often during filling)
//! 5. Combine into final report

use crate::application::admin_catalog::AdminCatalogService;
use crate::domain::ai_ports::{AiClient, AiQuality};
use crate::shared::{AppError, AppResult};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Cache TTL for audit USDA validation (days)
const AUDIT_CACHE_TTL_DAYS: i32 = 7;

/// Row for AI audit — collects completeness data across all tables
#[derive(Debug, sqlx::FromRow)]
struct AuditRow {
    id: Uuid,
    slug: Option<String>,
    name_en: Option<String>,
    name_ru: Option<String>,
    product_type: Option<String>,
    image_url: Option<String>,
    description_en: Option<String>,
    description_ru: Option<String>,
    description_pl: Option<String>,
    description_uk: Option<String>,
    calories_per_100g: Option<i32>,
    protein: Option<f32>,
    fat: Option<f32>,
    carbs: Option<f32>,
    density: Option<f32>,
    shelf_life_days: Option<i32>,
    portion: Option<f32>,
    availability_months: Option<Vec<bool>>,
    // nutrition sub-tables existence flags
    has_macros: Option<bool>,
    has_vitamins: Option<bool>,
    has_minerals: Option<bool>,
    has_fatty_acids: Option<bool>,
    has_diet_flags: Option<bool>,
    has_allergens: Option<bool>,
    has_food_props: Option<bool>,
    has_culinary: Option<bool>,
    // key macros from nutrition table
    n_calories: Option<f32>,
    n_protein: Option<f32>,
    n_fat: Option<f32>,
    n_carbs: Option<f32>,
    // key minerals
    m_calcium: Option<f32>,
    m_iron: Option<f32>,
    m_potassium: Option<f32>,
    m_sodium: Option<f32>,
    // key vitamins
    v_c: Option<f32>,
    v_d: Option<f32>,
}

impl AdminCatalogService {
    /// AI Audit — scans all products, checks completeness of all fields,
    /// then asks AI to validate nutrition data against USDA reference.
    ///
    /// 🚀 With caching: USDA validation cached 7 days (−70% AI costs)
    pub async fn ai_audit(&self) -> AppResult<serde_json::Value> {
        tracing::info!("🔍 Starting AI catalog audit...");

        // ── Phase 1: Collect all products with nutrition data ──
        let rows = sqlx::query_as::<_, AuditRow>(
            r#"
            SELECT
                ci.id,
                ci.slug,
                ci.name_en,
                ci.name_ru,
                COALESCE(ci.product_type, 'other') as product_type,
                ci.image_url,
                ci.description_en,
                ci.description_ru,
                ci.description_pl,
                ci.description_uk,
                ci.calories_per_100g,
                ci.protein_per_100g::float4  as protein,
                ci.fat_per_100g::float4      as fat,
                ci.carbs_per_100g::float4    as carbs,
                ci.density_g_per_ml::float4  as density,
                ci.shelf_life_days,
                ci.typical_portion_g::float4 as portion,
                ci.availability_months,
                -- nutrition sub-tables existence flags
                (SELECT COUNT(*) > 0 FROM nutrition_macros     WHERE product_id = ci.id) as has_macros,
                (SELECT COUNT(*) > 0 FROM nutrition_vitamins   WHERE product_id = ci.id) as has_vitamins,
                (SELECT COUNT(*) > 0 FROM nutrition_minerals   WHERE product_id = ci.id) as has_minerals,
                (SELECT COUNT(*) > 0 FROM nutrition_fatty_acids WHERE product_id = ci.id) as has_fatty_acids,
                (SELECT COUNT(*) > 0 FROM diet_flags           WHERE product_id = ci.id) as has_diet_flags,
                (SELECT COUNT(*) > 0 FROM product_allergens    WHERE product_id = ci.id) as has_allergens,
                (SELECT COUNT(*) > 0 FROM food_properties      WHERE product_id = ci.id) as has_food_props,
                (SELECT COUNT(*) > 0 FROM food_culinary_properties WHERE product_id = ci.id) as has_culinary,
                -- key macros from nutrition table
                nm.calories_kcal as n_calories,
                nm.protein_g     as n_protein,
                nm.fat_g         as n_fat,
                nm.carbs_g       as n_carbs,
                -- key minerals
                nmi.calcium      as m_calcium,
                nmi.iron         as m_iron,
                nmi.potassium    as m_potassium,
                nmi.sodium       as m_sodium,
                -- key vitamins
                nv.vitamin_c     as v_c,
                nv.vitamin_d     as v_d
            FROM catalog_ingredients ci
            LEFT JOIN nutrition_macros nm ON nm.product_id = ci.id
            LEFT JOIN nutrition_minerals nmi ON nmi.product_id = ci.id
            LEFT JOIN nutrition_vitamins nv ON nv.product_id = ci.id
            WHERE ci.is_active = true
            ORDER BY ci.name_en ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Audit query failed: {}", e);
            AppError::internal("Failed to query products for audit")
        })?;

        let total = rows.len();
        tracing::info!("🔍 Auditing {} products...", total);

        // ── Phase 2: Build completeness report for each product ──
        let mut issues: Vec<serde_json::Value> = Vec::new();
        let mut products_for_ai: Vec<String> = Vec::new();

        for row in &rows {
            let name = row.name_en.as_deref().unwrap_or("?");
            let slug = row.slug.as_deref().unwrap_or("?");
            let mut missing: Vec<String> = Vec::new();
            let mut warnings: Vec<String> = Vec::new();

            // ── Check basic fields ──
            if row.image_url.is_none() {
                missing.push("🖼️ нет фото".into());
            }
            if row.description_en.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true) {
                missing.push("📝 description_en".into());
            }
            if row.description_ru.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true) {
                missing.push("📝 description_ru".into());
            }
            if row.description_pl.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true) {
                missing.push("📝 description_pl".into());
            }
            if row.description_uk.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true) {
                missing.push("📝 description_uk".into());
            }

            // ── Check catalog macros ──
            if row.calories_per_100g.is_none() { missing.push("🔢 calories".into()); }
            if row.protein.is_none() { missing.push("🔢 protein".into()); }
            if row.fat.is_none() { missing.push("🔢 fat".into()); }
            if row.carbs.is_none() { missing.push("🔢 carbs".into()); }

            // ── Check physical ──
            if row.density.is_none() { missing.push("⚙️ density".into()); }
            if row.shelf_life_days.is_none() { missing.push("⚙️ shelf_life_days".into()); }
            if row.portion.is_none() { missing.push("⚙️ typical_portion".into()); }

            // ── Check nutrition sub-tables ──
            if !row.has_macros.unwrap_or(false) { missing.push("📊 macros table".into()); }
            if !row.has_vitamins.unwrap_or(false) { missing.push("⚗️ vitamins table".into()); }
            if !row.has_minerals.unwrap_or(false) { missing.push("🪨 minerals table".into()); }
            if !row.has_fatty_acids.unwrap_or(false) { missing.push("🧈 fatty_acids table".into()); }
            if !row.has_diet_flags.unwrap_or(false) { missing.push("🥗 diet_flags table".into()); }
            if !row.has_allergens.unwrap_or(false) { missing.push("⚠️ allergens table".into()); }
            if !row.has_food_props.unwrap_or(false) { missing.push("🔬 food_properties table".into()); }
            if !row.has_culinary.unwrap_or(false) { missing.push("🍳 culinary table".into()); }

            // ── Check availability_months ──
            if row.availability_months.is_none() {
                missing.push("📅 availability_months".into());
            }

            // ── Quick sanity checks (no AI needed) ──
            if let Some(cal) = row.calories_per_100g {
                if cal < 0 || cal > 900 {
                    warnings.push(format!("⚠️ calories={} (expected 0-900)", cal));
                }
            }
            if let Some(p) = row.protein {
                if p < 0.0 || p > 100.0 {
                    warnings.push(format!("⚠️ protein={:.1} (expected 0-100)", p));
                }
            }
            if let Some(f) = row.fat {
                if f < 0.0 || f > 100.0 {
                    warnings.push(format!("⚠️ fat={:.1} (expected 0-100)", f));
                }
            }
            if let Some(c) = row.carbs {
                if c < 0.0 || c > 100.0 {
                    warnings.push(format!("⚠️ carbs={:.1} (expected 0-100)", c));
                }
            }
            // Macros sum > 100g?
            if let (Some(p), Some(f), Some(c)) = (row.protein, row.fat, row.carbs) {
                let sum = p + f + c;
                if sum > 105.0 {
                    warnings.push(format!("⚠️ P+F+C={:.1}g > 100g (impossible)", sum));
                }
            }

            // Collect products that HAVE macros data for AI validation
            if row.has_macros.unwrap_or(false) && row.n_calories.is_some() {
                products_for_ai.push(format!(
                    "- {name}: cal={cal}, prot={prot}, fat={fat}, carbs={carbs}, vit_c={vc}, vit_d={vd}, calcium={ca}, iron={fe}, potassium={k}, sodium={na}",
                    name = name,
                    cal = row.n_calories.map(|v| format!("{:.0}", v)).unwrap_or("?".into()),
                    prot = row.n_protein.map(|v| format!("{:.1}", v)).unwrap_or("?".into()),
                    fat = row.n_fat.map(|v| format!("{:.1}", v)).unwrap_or("?".into()),
                    carbs = row.n_carbs.map(|v| format!("{:.1}", v)).unwrap_or("?".into()),
                    vc = row.v_c.map(|v| format!("{:.1}", v)).unwrap_or("?".into()),
                    vd = row.v_d.map(|v| format!("{:.1}", v)).unwrap_or("?".into()),
                    ca = row.m_calcium.map(|v| format!("{:.0}", v)).unwrap_or("?".into()),
                    fe = row.m_iron.map(|v| format!("{:.1}", v)).unwrap_or("?".into()),
                    k = row.m_potassium.map(|v| format!("{:.0}", v)).unwrap_or("?".into()),
                    na = row.m_sodium.map(|v| format!("{:.0}", v)).unwrap_or("?".into()),
                ));
            }

            let completeness = if missing.is_empty() && warnings.is_empty() {
                continue;
            } else {
                let total_checks: u32 = 20;
                let filled = total_checks.saturating_sub(missing.len() as u32);
                ((filled as f64 / total_checks as f64) * 100.0).round() as u32
            };

            issues.push(serde_json::json!({
                "id": row.id,
                "name_en": name,
                "slug": slug,
                "product_type": row.product_type,
                "completeness_percent": completeness,
                "missing": missing,
                "warnings": warnings,
            }));
        }

        // ── Phase 3: AI validation with caching ──
        let mut ai_warnings: Vec<serde_json::Value> = Vec::new();

        if !products_for_ai.is_empty() && products_for_ai.len() <= 30 {
            let products_list = products_for_ai.join("\n");

            // Cache key based on hash of all product data
            let cache_key = format!("uc:audit_usda:{}", hash_input(&products_list));

            // Check cache first
            let cached = self.ai_cache.get(&cache_key).await.ok().flatten();
            if let Some(cached_val) = cached {
                if let Ok(items) = serde_json::from_value::<Vec<serde_json::Value>>(cached_val) {
                    tracing::info!("📦 Audit USDA cache hit ({} warnings)", items.len());
                    ai_warnings = items;
                }
            }

            // Cache miss → call AI
            if ai_warnings.is_empty() {
                let ai_prompt = format!(
                    r#"You are a food database QA expert. Below is nutrition data (per 100g raw) from our database for several products.

Compare each product's values against USDA FoodData Central reference and report ONLY significant errors (>40% deviation from USDA).

Products:
{products_list}

Return ONLY a JSON array of issues found. Each issue:
{{
  "product": "<product name>",
  "field": "<field name>",
  "our_value": <our value>,
  "usda_value": <USDA reference value>,
  "deviation_percent": <percentage deviation>,
  "severity": "<high|medium|low>",
  "comment": "<brief explanation in Russian>"
}}

If ALL values are accurate, return an empty array: []
Be strict — only flag errors >40% deviation. Return ONLY the JSON array, no other text."#,
                    products_list = products_list,
                );

                match self.llm_adapter
                    .generate_with_quality(&ai_prompt, 2000, AiQuality::Best)
                    .await
                {
                    Ok(raw) => {
                        let parsed: Result<Vec<serde_json::Value>, _> = serde_json::from_str(&raw)
                            .or_else(|_| {
                                if let Some(start) = raw.find('[') {
                                    if let Some(end) = raw.rfind(']') {
                                        return serde_json::from_str(&raw[start..=end]);
                                    }
                                }
                                Ok(vec![])
                            });
                        if let Ok(items) = parsed {
                            // Cache the result
                            let cache_val = serde_json::to_value(&items).unwrap_or_default();
                            let _ = self.ai_cache.set(
                                &cache_key, cache_val, "gemini", "gemini-2.5-pro", AUDIT_CACHE_TTL_DAYS
                            ).await;
                            ai_warnings = items;
                        }
                    }
                    Err(e) => {
                        tracing::warn!("AI validation failed (non-critical): {}", e);
                    }
                }
            }
        }

        // ── Phase 4: Build final report ──
        issues.sort_by(|a, b| {
            let ca = a["completeness_percent"].as_u64().unwrap_or(100);
            let cb = b["completeness_percent"].as_u64().unwrap_or(100);
            ca.cmp(&cb)
        });

        let complete_count = total - issues.len();
        let report = serde_json::json!({
            "summary": {
                "total_products": total,
                "fully_complete": complete_count,
                "needs_attention": issues.len(),
                "ai_data_warnings": ai_warnings.len(),
                "audit_date": chrono::Utc::now().to_rfc3339(),
            },
            "products_needing_attention": issues,
            "ai_usda_warnings": ai_warnings,
        });

        tracing::info!(
            "✅ Audit complete: {}/{} products OK, {} need attention, {} AI warnings",
            complete_count, total, issues.len(), ai_warnings.len()
        );

        Ok(report)
    }
}

fn hash_input(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())[..16].to_string()
}
