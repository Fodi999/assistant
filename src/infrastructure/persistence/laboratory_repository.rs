//! Laboratory repository — PostgreSQL persistence for the food-tech Laboratory.
//!
//! Owns 4 tables (see migration `20260427000001_create_laboratory_tables.sql`):
//!   • `lab_projects`              – top-level project (per user)
//!   • `lab_project_ingredients`   – user-chosen products with qty / unit / role
//!   • `lab_process_steps`         – ordered technological steps
//!   • `lab_project_analysis`      – snapshot output of the engines (later step)
//!
//! All queries are scoped by `owner_id` so the application layer can defend
//! tenancy without having to remember to add a WHERE clause.

use rust_decimal::Decimal;
use serde_json::Value as JsonValue;
use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::shared::{AppError, AppResult};

// ─────────────────────────────────────────────────────────────────────────────
// Row structs (DB → Rust) — kept private to this module; the application layer
// converts them into its own DTOs via `From` impls in `application::laboratory`.
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LabProjectRow {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub target_product_type: Option<String>,
    pub status: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LabProjectIngredientRow {
    pub id: Uuid,
    pub project_id: Uuid,
    pub ingredient_slug: String,
    pub quantity: Decimal,
    pub unit: String,
    pub role: Option<String>,
    pub sort_order: i32,
    pub notes: Option<String>,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LabProcessStepRow {
    pub id: Uuid,
    pub project_id: Uuid,
    pub order_index: i32,
    pub technique: String,
    pub temperature_c: Option<Decimal>,
    pub duration_min: Option<i32>,
    pub target_slugs: Option<Vec<String>>,
    pub notes: Option<String>,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LabProjectAnalysisRow {
    pub id: Uuid,
    pub project_id: Uuid,
    pub shelf_life_days: Option<i32>,
    pub estimated_cost: Option<Decimal>,
    pub complexity_score: Option<i32>,
    pub risk_level: Option<String>,
    pub texture_result: Option<String>,
    pub flavor_result: JsonValue,
    pub nutrition_result: JsonValue,
    pub process_effects: JsonValue,
    pub storage_recommendations: JsonValue,
    pub pairing_suggestions: JsonValue,
    pub warnings: JsonValue,
    pub created_at: OffsetDateTime,
}

// ─────────────────────────────────────────────────────────────────────────────
// Insert / update DTOs — the repository deliberately accepts plain Rust values
// (no domain types yet) so the application layer can validate first.
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct NewLabProject {
    pub owner_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub target_product_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewLabProjectIngredient {
    pub project_id: Uuid,
    pub ingredient_slug: String,
    pub quantity: Decimal,
    pub unit: String,
    pub role: Option<String>,
    pub sort_order: Option<i32>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewLabProcessStep {
    pub project_id: Uuid,
    pub order_index: Option<i32>,
    pub technique: String,
    pub temperature_c: Option<Decimal>,
    pub duration_min: Option<i32>,
    pub target_slugs: Option<Vec<String>>,
    pub notes: Option<String>,
}

/// Snapshot insert for `lab_project_analysis`. Engines fill what they can;
/// JSON columns default to `'{}'` / `'[]'` if `None` is passed at the SQL
/// layer — but our schema column defaults already handle that, so we still
/// have to pass *something*. Callers should pass `serde_json::json!({})` or
/// `json!([])` for empty results.
#[derive(Debug, Clone)]
pub struct NewLabProjectAnalysis {
    pub project_id: Uuid,
    pub shelf_life_days: Option<i32>,
    pub estimated_cost: Option<Decimal>,
    pub complexity_score: Option<i32>,
    pub risk_level: Option<String>,
    pub texture_result: Option<String>,
    pub flavor_result: JsonValue,
    pub nutrition_result: JsonValue,
    pub process_effects: JsonValue,
    pub storage_recommendations: JsonValue,
    pub pairing_suggestions: JsonValue,
    pub warnings: JsonValue,
    pub input_snapshot: Option<JsonValue>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Repository
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct LaboratoryRepository {
    pool: PgPool,
}

impl LaboratoryRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ── lab_projects ─────────────────────────────────────────────────────────

    pub async fn insert_project(&self, p: NewLabProject) -> AppResult<LabProjectRow> {
        let row = sqlx::query_as::<_, LabProjectRow>(
            r#"
            INSERT INTO lab_projects (owner_id, name, description, target_product_type)
            VALUES ($1, $2, $3, $4)
            RETURNING id, owner_id, name, description, target_product_type, status,
                      created_at, updated_at
            "#,
        )
        .bind(p.owner_id)
        .bind(&p.name)
        .bind(&p.description)
        .bind(&p.target_product_type)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("insert_project: {e}")))?;
        Ok(row)
    }

    pub async fn list_projects_by_owner(&self, owner_id: Uuid) -> AppResult<Vec<LabProjectRow>> {
        let rows = sqlx::query_as::<_, LabProjectRow>(
            r#"
            SELECT id, owner_id, name, description, target_product_type, status,
                   created_at, updated_at
            FROM lab_projects
            WHERE owner_id = $1
            ORDER BY updated_at DESC
            "#,
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("list_projects_by_owner: {e}")))?;
        Ok(rows)
    }

    /// Fetches a project only if it belongs to `owner_id`; returns `Ok(None)`
    /// if the project doesn't exist OR isn't owned by this user (don't leak
    /// existence to non-owners).
    pub async fn get_project_for_owner(
        &self,
        project_id: Uuid,
        owner_id: Uuid,
    ) -> AppResult<Option<LabProjectRow>> {
        let row = sqlx::query_as::<_, LabProjectRow>(
            r#"
            SELECT id, owner_id, name, description, target_product_type, status,
                   created_at, updated_at
            FROM lab_projects
            WHERE id = $1 AND owner_id = $2
            "#,
        )
        .bind(project_id)
        .bind(owner_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("get_project_for_owner: {e}")))?;
        Ok(row)
    }

    /// Delete a project. Returns `true` if a row was actually deleted (i.e.
    /// the user owned it). Cascades clean ingredients / steps / analysis.
    pub async fn delete_project_for_owner(
        &self,
        project_id: Uuid,
        owner_id: Uuid,
    ) -> AppResult<bool> {
        let res = sqlx::query("DELETE FROM lab_projects WHERE id = $1 AND owner_id = $2")
            .bind(project_id)
            .bind(owner_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(format!("delete_project_for_owner: {e}")))?;
        Ok(res.rows_affected() > 0)
    }

    // ── lab_project_ingredients ──────────────────────────────────────────────

    pub async fn insert_ingredient(
        &self,
        ing: NewLabProjectIngredient,
    ) -> AppResult<LabProjectIngredientRow> {
        let (row, _merged) = self.upsert_ingredient(ing).await?;
        Ok(row)
    }

    /// Insert a new ingredient row OR merge into the existing one when
    /// `(project_id, ingredient_slug, unit)` already exists for this project.
    ///
    /// Merge behaviour (Laboratory v2):
    /// * `quantity`  ← old + new
    /// * `role`      ← keep existing if set, otherwise take new
    /// * `notes`     ← keep existing if set, otherwise take new
    /// * `sort_order`← unchanged on conflict (keeps the user's layout)
    ///
    /// Returns `(row, merged)` where `merged == true` when this call
    /// updated an existing row instead of inserting a fresh one.
    /// Detection uses Postgres' `xmax <> 0` system column trick on the
    /// returned row of an `INSERT ... ON CONFLICT DO UPDATE`.
    pub async fn upsert_ingredient(
        &self,
        ing: NewLabProjectIngredient,
    ) -> AppResult<(LabProjectIngredientRow, bool)> {
        // Auto-assign sort_order = max+1 only when this is a fresh insert.
        // (For an UPDATE path the column simply isn't touched.)
        let sort_order: i32 = match ing.sort_order {
            Some(v) => v,
            None => {
                let next: i32 = sqlx::query_scalar(
                    "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM lab_project_ingredients WHERE project_id = $1",
                )
                .bind(ing.project_id)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| AppError::internal(format!("next sort_order: {e}")))?;
                next
            }
        };

        use sqlx::Row;
        let rec = sqlx::query(
            r#"
            INSERT INTO lab_project_ingredients
                (project_id, ingredient_slug, quantity, unit, role, sort_order, notes)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (project_id, ingredient_slug, unit) DO UPDATE SET
                quantity = lab_project_ingredients.quantity + EXCLUDED.quantity,
                role     = COALESCE(lab_project_ingredients.role,  EXCLUDED.role),
                notes    = COALESCE(lab_project_ingredients.notes, EXCLUDED.notes)
            RETURNING id, project_id, ingredient_slug, quantity, unit, role, sort_order,
                      notes, created_at,
                      (xmax <> 0) AS merged
            "#,
        )
        .bind(ing.project_id)
        .bind(&ing.ingredient_slug)
        .bind(ing.quantity)
        .bind(&ing.unit)
        .bind(&ing.role)
        .bind(sort_order)
        .bind(&ing.notes)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("upsert_ingredient: {e}")))?;

        let merged: bool = rec.try_get("merged").unwrap_or(false);
        let row = LabProjectIngredientRow {
            id: rec
                .try_get("id")
                .map_err(|e| AppError::internal(format!("upsert_ingredient row id: {e}")))?,
            project_id: rec.try_get("project_id").map_err(|e| {
                AppError::internal(format!("upsert_ingredient row project_id: {e}"))
            })?,
            ingredient_slug: rec
                .try_get("ingredient_slug")
                .map_err(|e| AppError::internal(format!("upsert_ingredient row slug: {e}")))?,
            quantity: rec
                .try_get("quantity")
                .map_err(|e| AppError::internal(format!("upsert_ingredient row quantity: {e}")))?,
            unit: rec
                .try_get("unit")
                .map_err(|e| AppError::internal(format!("upsert_ingredient row unit: {e}")))?,
            role: rec.try_get("role").ok(),
            sort_order: rec.try_get("sort_order").map_err(|e| {
                AppError::internal(format!("upsert_ingredient row sort_order: {e}"))
            })?,
            notes: rec.try_get("notes").ok(),
            created_at: rec.try_get("created_at").map_err(|e| {
                AppError::internal(format!("upsert_ingredient row created_at: {e}"))
            })?,
        };
        Ok((row, merged))
    }

    pub async fn list_ingredients(
        &self,
        project_id: Uuid,
    ) -> AppResult<Vec<LabProjectIngredientRow>> {
        let rows = sqlx::query_as::<_, LabProjectIngredientRow>(
            r#"
            SELECT id, project_id, ingredient_slug, quantity, unit, role, sort_order,
                   notes, created_at
            FROM lab_project_ingredients
            WHERE project_id = $1
            ORDER BY sort_order ASC, created_at ASC
            "#,
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("list_ingredients: {e}")))?;
        Ok(rows)
    }

    /// Deletes an ingredient if it belongs to a project owned by `owner_id`.
    /// Returns `true` if a row was actually deleted.
    pub async fn delete_ingredient_for_owner(
        &self,
        ingredient_id: Uuid,
        project_id: Uuid,
        owner_id: Uuid,
    ) -> AppResult<bool> {
        let res = sqlx::query(
            r#"
            DELETE FROM lab_project_ingredients li
            USING lab_projects p
            WHERE li.id = $1
              AND li.project_id = $2
              AND li.project_id = p.id
              AND p.owner_id = $3
            "#,
        )
        .bind(ingredient_id)
        .bind(project_id)
        .bind(owner_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("delete_ingredient_for_owner: {e}")))?;
        Ok(res.rows_affected() > 0)
    }

    // ── lab_process_steps ────────────────────────────────────────────────────

    pub async fn insert_step(&self, s: NewLabProcessStep) -> AppResult<LabProcessStepRow> {
        let order_index: i32 = match s.order_index {
            Some(v) => v,
            None => {
                let next: i32 = sqlx::query_scalar(
                    "SELECT COALESCE(MAX(order_index), -1) + 1 FROM lab_process_steps WHERE project_id = $1",
                )
                .bind(s.project_id)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| AppError::internal(format!("next order_index: {e}")))?;
                next
            }
        };

        let row = sqlx::query_as::<_, LabProcessStepRow>(
            r#"
            INSERT INTO lab_process_steps
                (project_id, order_index, technique, temperature_c, duration_min,
                 target_slugs, notes)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, project_id, order_index, technique, temperature_c, duration_min,
                      target_slugs, notes, created_at
            "#,
        )
        .bind(s.project_id)
        .bind(order_index)
        .bind(&s.technique)
        .bind(s.temperature_c)
        .bind(s.duration_min)
        .bind(&s.target_slugs)
        .bind(&s.notes)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("insert_step: {e}")))?;
        Ok(row)
    }

    pub async fn list_steps(&self, project_id: Uuid) -> AppResult<Vec<LabProcessStepRow>> {
        let rows = sqlx::query_as::<_, LabProcessStepRow>(
            r#"
            SELECT id, project_id, order_index, technique, temperature_c, duration_min,
                   target_slugs, notes, created_at
            FROM lab_process_steps
            WHERE project_id = $1
            ORDER BY order_index ASC, created_at ASC
            "#,
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("list_steps: {e}")))?;
        Ok(rows)
    }

    /// Returns the highest-`order_index` step of a project, used by the
    /// service layer to detect "back-to-back identical step" duplicates.
    pub async fn latest_step(&self, project_id: Uuid) -> AppResult<Option<LabProcessStepRow>> {
        let row = sqlx::query_as::<_, LabProcessStepRow>(
            r#"
            SELECT id, project_id, order_index, technique, temperature_c, duration_min,
                   target_slugs, notes, created_at
            FROM lab_process_steps
            WHERE project_id = $1
            ORDER BY order_index DESC, created_at DESC
            LIMIT 1
            "#,
        )
        .bind(project_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("latest_step: {e}")))?;
        Ok(row)
    }

    pub async fn delete_step_for_owner(
        &self,
        step_id: Uuid,
        project_id: Uuid,
        owner_id: Uuid,
    ) -> AppResult<bool> {
        let res = sqlx::query(
            r#"
            DELETE FROM lab_process_steps s
            USING lab_projects p
            WHERE s.id = $1
              AND s.project_id = $2
              AND s.project_id = p.id
              AND p.owner_id = $3
            "#,
        )
        .bind(step_id)
        .bind(project_id)
        .bind(owner_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("delete_step_for_owner: {e}")))?;
        Ok(res.rows_affected() > 0)
    }

    // ── lab_project_analysis ─────────────────────────────────────────────────

    /// Insert a new analysis snapshot. We append (not upsert) so the user can
    /// see how the analysis evolves as they iterate on the recipe;
    /// `latest_analysis` always returns the most recent row.
    pub async fn insert_analysis(
        &self,
        snapshot: NewLabProjectAnalysis,
    ) -> AppResult<LabProjectAnalysisRow> {
        let row = sqlx::query_as::<_, LabProjectAnalysisRow>(
            r#"
            INSERT INTO lab_project_analysis (
                project_id, shelf_life_days, estimated_cost, complexity_score,
                risk_level, texture_result, flavor_result, nutrition_result,
                process_effects, storage_recommendations, pairing_suggestions,
                warnings, input_snapshot
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            RETURNING id, project_id, shelf_life_days, estimated_cost, complexity_score,
                      risk_level, texture_result, flavor_result, nutrition_result,
                      process_effects, storage_recommendations, pairing_suggestions,
                      warnings, created_at
            "#,
        )
        .bind(snapshot.project_id)
        .bind(snapshot.shelf_life_days)
        .bind(snapshot.estimated_cost)
        .bind(snapshot.complexity_score)
        .bind(snapshot.risk_level)
        .bind(snapshot.texture_result)
        .bind(snapshot.flavor_result)
        .bind(snapshot.nutrition_result)
        .bind(snapshot.process_effects)
        .bind(snapshot.storage_recommendations)
        .bind(snapshot.pairing_suggestions)
        .bind(snapshot.warnings)
        .bind(snapshot.input_snapshot)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("insert_analysis: {e}")))?;
        Ok(row)
    }

    pub async fn latest_analysis(
        &self,
        project_id: Uuid,
    ) -> AppResult<Option<LabProjectAnalysisRow>> {
        let row = sqlx::query_as::<_, LabProjectAnalysisRow>(
            r#"
            SELECT id, project_id, shelf_life_days, estimated_cost, complexity_score,
                   risk_level, texture_result, flavor_result, nutrition_result,
                   process_effects, storage_recommendations, pairing_suggestions,
                   warnings, created_at
            FROM lab_project_analysis
            WHERE project_id = $1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(project_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("latest_analysis: {e}")))?;
        Ok(row)
    }

    /// Tiny helper used by handlers to confirm child writes target a project
    /// the user actually owns, without fetching the whole row.
    pub async fn assert_owner(&self, project_id: Uuid, owner_id: Uuid) -> AppResult<()> {
        let exists: Option<i32> =
            sqlx::query("SELECT 1 FROM lab_projects WHERE id = $1 AND owner_id = $2")
                .bind(project_id)
                .bind(owner_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| AppError::internal(format!("assert_owner: {e}")))?
                .map(|r| r.get::<i32, _>(0));
        if exists.is_some() {
            Ok(())
        } else {
            Err(AppError::not_found("Laboratory project not found"))
        }
    }
}
