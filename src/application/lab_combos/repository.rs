// ─── Repository — pure DB CRUD for Lab Combo Pages ──────────────────────────
//
// Single responsibility: read/write lab_combo_pages table.
// No business logic, no AI, no SEO generation.

use crate::shared::{AppError, AppResult};
use sqlx::PgPool;
use uuid::Uuid;

use super::types::*;

// ── Full SELECT columns (reused across all queries) ─────────────────────────

const FULL_COLUMNS: &str = r#"
    id, slug, locale, ingredients,
    goal, meal_type, diet, cooking_time, budget, cuisine,
    title, description, h1, intro,
    why_it_works, how_to_cook, optimization_tips, image_url, process_image_url, detail_image_url,
    smart_response, faq,
    total_weight_g, servings_count,
    calories_total, protein_total, fat_total, carbs_total, fiber_total,
    calories_per_serving, protein_per_serving, fat_per_serving, carbs_per_serving, fiber_per_serving,
    structured_ingredients,
    status, quality_score,
    published_at::text, created_at::text, updated_at::text
"#;

pub struct ComboRepository {
    pool: PgPool,
}

impl ComboRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    // ── Check if slug+locale exists ─────────────────────────────────────

    pub async fn exists(&self, slug: &str, locale: &str) -> AppResult<bool> {
        let row: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM lab_combo_pages WHERE slug = $1 AND locale = $2")
                .bind(slug)
                .bind(locale)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.is_some())
    }

    // ── Insert ──────────────────────────────────────────────────────────

    #[allow(clippy::too_many_arguments)]
    pub async fn insert(
        &self,
        id: Uuid,
        slug: &str,
        locale: &str,
        ingredients: &[String],
        goal: Option<&str>,
        meal_type: Option<&str>,
        diet: Option<&str>,
        cooking_time: Option<&str>,
        budget: Option<&str>,
        cuisine: Option<&str>,
        title: &str,
        description: &str,
        h1: &str,
        intro: &str,
        why_it_works: &str,
        how_to_cook: &serde_json::Value,
        optimization_tips: &serde_json::Value,
        smart_response: &serde_json::Value,
        faq: &serde_json::Value,
        total_weight_g: f32,
        servings_count: i16,
        calories_total: f32,
        protein_total: f32,
        fat_total: f32,
        carbs_total: f32,
        fiber_total: f32,
        calories_per_serving: f32,
        protein_per_serving: f32,
        fat_per_serving: f32,
        carbs_per_serving: f32,
        fiber_per_serving: f32,
        structured_ingredients: &serde_json::Value,
    ) -> AppResult<LabComboPage> {
        let sql = format!(
            r#"
            INSERT INTO lab_combo_pages (
                id, slug, locale, ingredients,
                goal, meal_type, diet, cooking_time, budget, cuisine,
                title, description, h1, intro,
                why_it_works, how_to_cook, optimization_tips,
                smart_response, faq,
                total_weight_g, servings_count,
                calories_total, protein_total, fat_total, carbs_total, fiber_total,
                calories_per_serving, protein_per_serving, fat_per_serving, carbs_per_serving, fiber_per_serving,
                structured_ingredients,
                status
            ) VALUES (
                $1, $2, $3, $4,
                $5, $6, $7, $8, $9, $10,
                $11, $12, $13, $14,
                $15, $16, $17,
                $18, $19,
                $20, $21,
                $22, $23, $24, $25, $26,
                $27, $28, $29, $30, $31,
                $32,
                'draft'
            )
            RETURNING {FULL_COLUMNS}
            "#
        );

        let page = sqlx::query_as::<_, LabComboPage>(&sql)
            .bind(id)
            .bind(slug)
            .bind(locale)
            .bind(ingredients)
            .bind(goal)
            .bind(meal_type)
            .bind(diet)
            .bind(cooking_time)
            .bind(budget)
            .bind(cuisine)
            .bind(title)
            .bind(description)
            .bind(h1)
            .bind(intro)
            .bind(why_it_works)
            .bind(how_to_cook)
            .bind(optimization_tips)
            .bind(smart_response)
            .bind(faq)
            .bind(total_weight_g)
            .bind(servings_count)
            .bind(calories_total)
            .bind(protein_total)
            .bind(fat_total)
            .bind(carbs_total)
            .bind(fiber_total)
            .bind(calories_per_serving)
            .bind(protein_per_serving)
            .bind(fat_per_serving)
            .bind(carbs_per_serving)
            .bind(fiber_per_serving)
            .bind(structured_ingredients)
            .fetch_one(&self.pool)
            .await?;

        Ok(page)
    }

    // ── Update quality score ────────────────────────────────────────────

    pub async fn update_quality_score(&self, id: Uuid, qs: i16) -> AppResult<()> {
        sqlx::query("UPDATE lab_combo_pages SET quality_score = $1 WHERE id = $2")
            .bind(qs)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ── Publish ─────────────────────────────────────────────────────────

    pub async fn publish(&self, id: Uuid) -> AppResult<LabComboPage> {
        let sql = format!(
            r#"
            UPDATE lab_combo_pages
            SET status = 'published', published_at = now(), updated_at = now()
            WHERE id = $1
            RETURNING {FULL_COLUMNS}
            "#
        );
        let page = sqlx::query_as::<_, LabComboPage>(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::not_found("combo page not found"))?;
        Ok(page)
    }

    // ── Archive ─────────────────────────────────────────────────────────

    pub async fn archive(&self, id: Uuid) -> AppResult<LabComboPage> {
        let sql = format!(
            r#"
            UPDATE lab_combo_pages
            SET status = 'archived', updated_at = now()
            WHERE id = $1
            RETURNING {FULL_COLUMNS}
            "#
        );
        let page = sqlx::query_as::<_, LabComboPage>(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::not_found("combo page not found"))?;
        Ok(page)
    }

    // ── Delete ──────────────────────────────────────────────────────────

    pub async fn delete(&self, id: Uuid) -> AppResult<()> {
        let result = sqlx::query("DELETE FROM lab_combo_pages WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::not_found("combo page not found"));
        }
        Ok(())
    }

    // ── Update ──────────────────────────────────────────────────────────

    pub async fn update(&self, id: Uuid, req: &UpdateComboRequest) -> AppResult<LabComboPage> {
        let sql = format!(
            r#"
            UPDATE lab_combo_pages SET
                title = COALESCE($2, title),
                description = COALESCE($3, description),
                h1 = COALESCE($4, h1),
                intro = COALESCE($5, intro),
                why_it_works = COALESCE($6, why_it_works),
                image_url = COALESCE($7, image_url),
                process_image_url = COALESCE($8, process_image_url),
                detail_image_url = COALESCE($9, detail_image_url),
                updated_at = now()
            WHERE id = $1
            RETURNING {FULL_COLUMNS}
            "#
        );
        let page = sqlx::query_as::<_, LabComboPage>(&sql)
            .bind(id)
            .bind(&req.title)
            .bind(&req.description)
            .bind(&req.h1)
            .bind(&req.intro)
            .bind(&req.why_it_works)
            .bind(&req.image_url)
            .bind(&req.process_image_url)
            .bind(&req.detail_image_url)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::not_found("combo page not found"))?;
        Ok(page)
    }

    // ── List ────────────────────────────────────────────────────────────

    pub async fn list(&self, query: &ListCombosQuery) -> AppResult<Vec<LabComboPage>> {
        let limit = query.limit.unwrap_or(50).min(200);
        let offset = query.offset.unwrap_or(0);

        let sql = format!(
            r#"
            SELECT {FULL_COLUMNS}
            FROM lab_combo_pages
            WHERE ($1::text IS NULL OR status = $1)
              AND ($2::text IS NULL OR locale = $2)
            ORDER BY updated_at DESC
            LIMIT $3 OFFSET $4
            "#
        );

        let rows = sqlx::query_as::<_, LabComboPage>(&sql)
            .bind(&query.status)
            .bind(&query.locale)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows)
    }

    // ── Get published page by slug ──────────────────────────────────────

    pub async fn get_published(&self, slug: &str, locale: &str) -> AppResult<Option<LabComboPage>> {
        let sql = format!(
            r#"
            SELECT {FULL_COLUMNS}
            FROM lab_combo_pages
            WHERE slug = $1 AND locale = $2 AND status = 'published'
            "#
        );
        let page = sqlx::query_as::<_, LabComboPage>(&sql)
            .bind(slug)
            .bind(locale)
            .fetch_optional(&self.pool)
            .await?;
        Ok(page)
    }

    // ── Related combos ──────────────────────────────────────────────────

    pub async fn get_related_combos(
        &self,
        slug: &str,
        locale: &str,
        limit: i64,
    ) -> AppResult<Vec<RelatedCombo>> {
        let rows = sqlx::query_as::<_, RelatedCombo>(
            r#"
            WITH current AS (
                SELECT id, ingredients FROM lab_combo_pages
                WHERE slug = $1 AND locale = $2 AND status = 'published'
                LIMIT 1
            )
            SELECT p.slug, p.title, p.ingredients, p.goal, p.meal_type, p.image_url
            FROM lab_combo_pages p, current c
            WHERE p.status = 'published'
              AND p.locale = $2
              AND p.id != c.id
              AND p.ingredients && c.ingredients
            ORDER BY array_length(
                ARRAY(SELECT unnest(p.ingredients) INTERSECT SELECT unnest(c.ingredients)), 1
            ) DESC, p.quality_score DESC
            LIMIT $3
            "#,
        )
        .bind(slug)
        .bind(locale)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    // ── "People also cook" ──────────────────────────────────────────────

    pub async fn get_also_cook(
        &self,
        slug: &str,
        locale: &str,
        limit: i64,
    ) -> AppResult<Vec<RelatedCombo>> {
        let rows = sqlx::query_as::<_, RelatedCombo>(
            r#"
            WITH current AS (
                SELECT id, ingredients, goal, meal_type
                FROM lab_combo_pages
                WHERE slug = $1 AND locale = $2 AND status = 'published'
                LIMIT 1
            )
            SELECT p.slug, p.title, p.ingredients, p.goal, p.meal_type, p.image_url
            FROM lab_combo_pages p, current c
            WHERE p.status = 'published'
              AND p.locale = $2
              AND p.id != c.id
              AND (
                  (p.goal IS NOT NULL AND p.goal = c.goal)
                  OR (p.meal_type IS NOT NULL AND p.meal_type = c.meal_type)
              )
              AND NOT (p.ingredients && c.ingredients)
            ORDER BY p.quality_score DESC, p.published_at DESC
            LIMIT $3
            "#,
        )
        .bind(slug)
        .bind(locale)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    // ── Sitemap ─────────────────────────────────────────────────────────

    pub async fn sitemap(&self) -> AppResult<Vec<LabComboSitemapEntry>> {
        let rows = sqlx::query_as::<_, LabComboSitemapEntry>(
            r#"
            SELECT slug, locale, updated_at::text, ingredients, goal, meal_type
            FROM lab_combo_pages
            WHERE status = 'published'
            ORDER BY published_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    // ── Update SEO + steps (for AI enrichment) ──────────────────────────

    pub async fn update_seo_with_steps(
        &self,
        id: Uuid,
        title: &str,
        description: &str,
        h1: &str,
        intro: &str,
        why_it_works: &str,
        steps: &serde_json::Value,
    ) -> AppResult<()> {
        sqlx::query(
            r#"UPDATE lab_combo_pages SET
                title = CASE WHEN $1 != '' THEN $1 ELSE title END,
                description = CASE WHEN $2 != '' THEN $2 ELSE description END,
                h1 = CASE WHEN $3 != '' THEN $3 ELSE h1 END,
                intro = CASE WHEN $4 != '' THEN $4 ELSE intro END,
                why_it_works = CASE WHEN $5 != '' THEN $5 ELSE why_it_works END,
                how_to_cook = $7,
                updated_at = NOW()
            WHERE id = $6"#,
        )
        .bind(title)
        .bind(description)
        .bind(h1)
        .bind(intro)
        .bind(why_it_works)
        .bind(id)
        .bind(steps)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Update only text fields (no steps change).
    pub async fn update_seo_text(
        &self,
        id: Uuid,
        title: &str,
        description: &str,
        h1: &str,
        intro: &str,
        why_it_works: &str,
    ) -> AppResult<()> {
        sqlx::query(
            r#"UPDATE lab_combo_pages SET
                title = CASE WHEN $1 != '' THEN $1 ELSE title END,
                description = CASE WHEN $2 != '' THEN $2 ELSE description END,
                h1 = CASE WHEN $3 != '' THEN $3 ELSE h1 END,
                intro = CASE WHEN $4 != '' THEN $4 ELSE intro END,
                why_it_works = CASE WHEN $5 != '' THEN $5 ELSE why_it_works END,
                updated_at = NOW()
            WHERE id = $6"#,
        )
        .bind(title)
        .bind(description)
        .bind(h1)
        .bind(intro)
        .bind(why_it_works)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ── Backfill structured_ingredients ──────────────────────────────────

    pub async fn get_empty_structured(&self) -> AppResult<Vec<(Uuid, Vec<String>, String)>> {
        let rows: Vec<(Uuid, Vec<String>, String)> = sqlx::query_as(
            r#"SELECT id, ingredients, locale
               FROM lab_combo_pages
               WHERE structured_ingredients = '[]'::jsonb
               ORDER BY created_at DESC"#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn update_structured_ingredients(
        &self,
        id: Uuid,
        structured: &serde_json::Value,
    ) -> AppResult<()> {
        sqlx::query(
            "UPDATE lab_combo_pages SET structured_ingredients = $1, updated_at = NOW() WHERE id = $2",
        )
        .bind(structured)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ── Slug count (for bulk generation) ────────────────────────────────

    pub async fn count_by_slug(&self, slug: &str) -> AppResult<i64> {
        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM lab_combo_pages WHERE slug = $1 AND locale IN ('en','pl','ru','uk')",
        )
        .bind(slug)
        .fetch_one(&self.pool)
        .await?;
        Ok(count)
    }

    // ── Image helpers ───────────────────────────────────────────────────

    pub async fn combo_exists(&self, id: Uuid) -> AppResult<bool> {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM lab_combo_pages WHERE id = $1)")
                .bind(id)
                .fetch_one(&self.pool)
                .await?;
        Ok(exists)
    }

    pub async fn save_typed_image_url(
        &self,
        id: Uuid,
        kind: &str,
        url: &str,
    ) -> AppResult<LabComboPage> {
        let column = match kind {
            "process" => "process_image_url",
            "detail" => "detail_image_url",
            _ => "image_url",
        };

        let sql = format!(
            r#"
            UPDATE lab_combo_pages SET {} = $1, updated_at = NOW()
            WHERE id = $2
            RETURNING {FULL_COLUMNS}
            "#,
            column,
        );

        let page = sqlx::query_as::<_, LabComboPage>(&sql)
            .bind(url)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::not_found("Lab combo not found"))?;

        Ok(page)
    }

    // ── Ingredient slug resolution ──────────────────────────────────────

    pub async fn resolve_ingredient_slug(&self, input: &str) -> AppResult<Option<String>> {
        let normalized = input.trim().to_lowercase();

        // Try exact slug match first
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT slug FROM catalog_ingredients WHERE slug = $1 AND is_active = true LIMIT 1",
        )
        .bind(&normalized)
        .fetch_optional(&self.pool)
        .await?;

        if let Some((slug,)) = row {
            return Ok(Some(slug));
        }

        // Try match by name in any language
        let row: Option<(String,)> = sqlx::query_as(
            r#"SELECT slug FROM catalog_ingredients
               WHERE is_active = true
                 AND (
                   LOWER(name_en) = $1
                   OR LOWER(name_ru) = $1
                   OR LOWER(name_pl) = $1
                   OR LOWER(name_uk) = $1
                 )
               LIMIT 1"#,
        )
        .bind(&normalized)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|(slug,)| slug))
    }

    /// Get catalog ingredient data for structured ingredients
    pub async fn get_catalog_ingredient(
        &self,
        slug: &str,
    ) -> AppResult<
        Option<(
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<f32>,
            Option<f32>,
            Option<f32>,
            Option<f32>,
        )>,
    > {
        let row = sqlx::query_as(
            r#"SELECT slug, name_en, name_ru, name_pl, name_uk,
                      image_url, product_type,
                      calories_per_100g::REAL, protein_per_100g::REAL,
                      fat_per_100g::REAL, carbs_per_100g::REAL
               FROM catalog_ingredients
               WHERE slug = $1 AND is_active = true
               LIMIT 1"#,
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }
}
