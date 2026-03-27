//! Use Case: AI Food Pairing — generate & manage ingredient pairings
//!
//! CRUD operations (no AI, no cache):
//! - get_pairings, add_pairing, delete_pairing, search_products
//!
//! AI generation (with cache):
//! - ai_generate_pairings: hash(product_name + catalog_list) → cached pairings
//! - Cache TTL: 30 days (catalog changes slowly)

use crate::application::admin_catalog::AdminCatalogService;
use crate::domain::ai_ports::{AiClient, AiQuality};
use crate::shared::{AppError, AppResult};
use serde::Serialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Cache TTL for AI-generated pairings (days)
const PAIRING_CACHE_TTL_DAYS: i32 = 30;

impl AdminCatalogService {
    /// Get all pairings for a product, grouped by type
    pub async fn get_pairings(&self, product_id: Uuid) -> AppResult<serde_json::Value> {
        #[derive(Debug, sqlx::FromRow, Serialize)]
        struct PairingRow {
            id: Option<Uuid>,
            paired_slug: Option<String>,
            paired_name_en: Option<String>,
            paired_name_ru: Option<String>,
            paired_image: Option<String>,
            paired_product_id: Option<Uuid>,
            pairing_type: Option<String>,
            pair_score: Option<f32>,
            flavor_score: Option<f32>,
            nutrition_score: Option<f32>,
            culinary_score: Option<f32>,
        }

        let rows: Vec<PairingRow> = sqlx::query_as(
            r#"SELECT fp.id,
                      b.slug    AS paired_slug,
                      b.name_en AS paired_name_en,
                      b.name_ru AS paired_name_ru,
                      b.image_url AS paired_image,
                      b.id      AS paired_product_id,
                      fp.pairing_type,
                      fp.pair_score,
                      fp.flavor_score,
                      fp.nutrition_score,
                      fp.culinary_score
               FROM food_pairing fp
               JOIN catalog_ingredients b ON b.id = fp.ingredient_b
               WHERE fp.ingredient_a = $1
               ORDER BY fp.pair_score DESC NULLS LAST"#,
        )
        .bind(product_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get pairings: {}", e);
            AppError::internal("Failed to get pairings")
        })?;

        // Group by pairing_type
        let mut primary = Vec::new();
        let mut secondary = Vec::new();
        let mut experimental = Vec::new();
        let mut avoid = Vec::new();

        for r in &rows {
            let item = serde_json::json!({
                "id": r.id,
                "paired_product_id": r.paired_product_id,
                "slug": r.paired_slug,
                "name_en": r.paired_name_en,
                "name_ru": r.paired_name_ru,
                "image_url": r.paired_image,
                "pair_score": r.pair_score,
                "flavor_score": r.flavor_score,
                "nutrition_score": r.nutrition_score,
                "culinary_score": r.culinary_score,
            });
            match r.pairing_type.as_deref().unwrap_or("primary") {
                "secondary" => secondary.push(item),
                "experimental" => experimental.push(item),
                "avoid" => avoid.push(item),
                _ => primary.push(item),
            }
        }

        Ok(serde_json::json!({
            "product_id": product_id,
            "total": rows.len(),
            "primary": primary,
            "secondary": secondary,
            "experimental": experimental,
            "avoid": avoid,
        }))
    }

    /// Add a single pairing
    pub async fn add_pairing(
        &self,
        product_id: Uuid,
        paired_product_id: Uuid,
        pairing_type: &str,
        strength: f32,
    ) -> AppResult<serde_json::Value> {
        let valid_types = ["primary", "secondary", "experimental", "avoid"];
        if !valid_types.contains(&pairing_type) {
            return Err(AppError::validation("Invalid pairing_type. Must be: primary, secondary, experimental, avoid"));
        }

        sqlx::query(
            r#"INSERT INTO food_pairing (ingredient_a, ingredient_b, pairing_type, pair_score, id)
               VALUES ($1, $2, $3, $4, gen_random_uuid())
               ON CONFLICT (ingredient_a, ingredient_b)
               DO UPDATE SET pairing_type = $3, pair_score = $4"#,
        )
        .bind(product_id)
        .bind(paired_product_id)
        .bind(pairing_type)
        .bind(strength)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to add pairing: {}", e);
            AppError::internal("Failed to add pairing")
        })?;

        tracing::info!("✅ Added pairing {} -> {} ({})", product_id, paired_product_id, pairing_type);
        self.get_pairings(product_id).await
    }

    /// Delete a pairing by its id
    pub async fn delete_pairing(&self, product_id: Uuid, pairing_id: Uuid) -> AppResult<()> {
        let result = sqlx::query(
            "DELETE FROM food_pairing WHERE id = $1 AND ingredient_a = $2",
        )
        .bind(pairing_id)
        .bind(product_id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete pairing: {}", e);
            AppError::internal("Failed to delete pairing")
        })?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found("Pairing not found"));
        }

        tracing::info!("✅ Deleted pairing {} from product {}", pairing_id, product_id);
        Ok(())
    }

    /// Search products by name (for pairing ingredient search)
    pub async fn search_products(&self, query: &str) -> AppResult<Vec<serde_json::Value>> {
        #[derive(Debug, sqlx::FromRow)]
        struct SearchRow {
            id: Uuid,
            slug: Option<String>,
            name_en: String,
            name_ru: Option<String>,
            image_url: Option<String>,
            product_type: Option<String>,
        }

        let pattern = format!("%{}%", query.to_lowercase());

        let rows: Vec<SearchRow> = sqlx::query_as(
            r#"SELECT ci.id, ci.slug, ci.name_en, ci.name_ru, ci.image_url, ci.product_type
               FROM catalog_ingredients ci
               WHERE ci.is_active = true
                 AND (LOWER(ci.name_en) LIKE $1
                      OR LOWER(COALESCE(ci.name_ru, '')) LIKE $1
                      OR LOWER(COALESCE(ci.name_pl, '')) LIKE $1
                      OR LOWER(COALESCE(ci.slug, '')) LIKE $1)
               ORDER BY ci.name_en
               LIMIT 15"#,
        )
        .bind(&pattern)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Product search failed: {}", e);
            AppError::internal("Search failed")
        })?;

        Ok(rows.into_iter().map(|r| {
            serde_json::json!({
                "id": r.id,
                "slug": r.slug,
                "name_en": r.name_en,
                "name_ru": r.name_ru,
                "image_url": r.image_url,
                "product_type": r.product_type,
            })
        }).collect())
    }

    /// AI Generate pairings for a product
    ///
    /// 🚀 With caching: hash(product + catalog_count) → cached AI suggestions
    pub async fn ai_generate_pairings(&self, product_id: Uuid) -> AppResult<serde_json::Value> {
        let product = self.get_product_by_id(product_id).await?;
        let name_en = &product.name_en;
        let product_type = &product.product_type;

        // Get catalog for matching
        #[derive(Debug, sqlx::FromRow)]
        struct SlugRow {
            id: Uuid,
            slug: Option<String>,
            name_en: String,
        }

        let catalog: Vec<SlugRow> = sqlx::query_as(
            "SELECT id, slug, name_en FROM catalog_ingredients WHERE is_active = true ORDER BY name_en",
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        let catalog_names: Vec<String> = catalog.iter().map(|r| r.name_en.to_lowercase()).collect();

        // ── Cache check ──
        // Key: product name + catalog size (catalog changes = new pairings)
        let fingerprint = format!("{}:{}:{}", name_en, product_type, catalog_names.len());
        let cache_key = format!("uc:pairings:{}", hash_input(&fingerprint));

        let ai_result: serde_json::Value;

        if let Ok(Some(cached)) = self.ai_cache.get(&cache_key).await {
            tracing::info!("📦 Pairings AI cache hit for {}", name_en);
            ai_result = cached;
        } else {
            // ── Call AI ──
            let prompt = format!(
                r#"You are a culinary expert and food scientist.

For the ingredient "{name_en}" (category: {product_type}), suggest food pairings.

AVAILABLE INGREDIENTS in our catalog:
{catalog_list}

Return ONLY a valid JSON object:
{{
  "primary": ["ingredient1", "ingredient2", ...],
  "secondary": ["ingredient3", "ingredient4", ...],
  "experimental": ["ingredient5"],
  "avoid": ["ingredient6"]
}}

Rules:
- primary: classic, well-known pairings (3-8 items)
- secondary: good but less common pairings (2-5 items)
- experimental: surprising/creative pairings (0-3 items)
- avoid: ingredients that clash (0-2 items)
- ONLY use ingredient names from the available list above
- Use lowercase English names exactly as shown
- Return ONLY the JSON, no other text"#,
                name_en = name_en,
                product_type = product_type,
                catalog_list = catalog_names.join(", "),
            );

            let raw = self.llm_adapter
                .generate_with_quality(&prompt, 1000, AiQuality::Balanced)
                .await?;

            ai_result = serde_json::from_str(&raw)
                .or_else(|_| {
                    if let Some(start) = raw.find('{') {
                        if let Some(end) = raw.rfind('}') {
                            return serde_json::from_str(&raw[start..=end]);
                        }
                    }
                    Err(serde_json::Error::io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "No JSON found",
                    )))
                })
                .map_err(|e| {
                    tracing::error!("Failed to parse AI pairings: {}", e);
                    AppError::internal("AI returned invalid JSON")
                })?;

            // Cache the AI result
            let _ = self.ai_cache.set(
                &cache_key, ai_result.clone(), "gemini", "gemini-3.1-pro-preview", PAIRING_CACHE_TTL_DAYS
            ).await;
        }

        // ── Match AI suggestions to catalog products and insert ──
        let mut inserted = 0u32;
        let mut not_found = Vec::new();

        for (ptype, score_base) in &[
            ("primary", 9.0f32),
            ("secondary", 6.5),
            ("experimental", 4.0),
            ("avoid", 1.0),
        ] {
            if let Some(arr) = ai_result.get(ptype).and_then(|v| v.as_array()) {
                for (i, item) in arr.iter().enumerate() {
                    if let Some(name) = item.as_str() {
                        let name_lower = name.to_lowercase();
                        let matched = catalog.iter().find(|r| {
                            r.name_en.to_lowercase() == name_lower
                                || r.slug.as_deref().unwrap_or("") == name_lower
                        });

                        if let Some(matched_product) = matched {
                            if matched_product.id == product_id {
                                continue;
                            }
                            let score = score_base - (i as f32 * 0.3);
                            match sqlx::query(
                                r#"INSERT INTO food_pairing (ingredient_a, ingredient_b, pairing_type, pair_score, id)
                                   VALUES ($1, $2, $3, $4, gen_random_uuid())
                                   ON CONFLICT (ingredient_a, ingredient_b)
                                   DO UPDATE SET pairing_type = $3, pair_score = $4"#,
                            )
                            .bind(product_id)
                            .bind(matched_product.id)
                            .bind(*ptype)
                            .bind(score)
                            .execute(&self.pool)
                            .await
                            {
                                Ok(_) => inserted += 1,
                                Err(e) => {
                                    tracing::error!("Failed to insert pairing {} -> {}: {}", product_id, matched_product.name_en, e);
                                    not_found.push(format!("{}(DB error)", name));
                                }
                            }
                        } else {
                            not_found.push(name.to_string());
                        }
                    }
                }
            }
        }

        tracing::info!(
            "✅ AI pairings for {}: {} inserted, {} not found in catalog",
            name_en, inserted, not_found.len()
        );

        let pairings = self.get_pairings(product_id).await?;
        Ok(serde_json::json!({
            "ai_suggestions": ai_result,
            "inserted": inserted,
            "not_found_in_catalog": not_found,
            "pairings": pairings,
        }))
    }
}

fn hash_input(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())[..16].to_string()
}
