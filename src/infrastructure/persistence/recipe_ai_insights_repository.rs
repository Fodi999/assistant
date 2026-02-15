use crate::domain::recipe_ai_insights::*;
use crate::shared::AppError;
use sqlx::{PgPool, types::Json};
use uuid::Uuid;

#[derive(Clone)]
pub struct RecipeAIInsightsRepository {
    pool: PgPool,
}

impl RecipeAIInsightsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create or update AI insights for a recipe
    pub async fn upsert(
        &self,
        recipe_id: Uuid,
        language: &str,
        steps: Vec<CookingStep>,
        validation: RecipeValidation,
        suggestions: Vec<RecipeSuggestion>,
        feasibility_score: i32,
        model: &str,
    ) -> Result<RecipeAIInsights, AppError> {
        let row = sqlx::query_as::<_, RecipeAIInsightsRow>(
            r#"
            INSERT INTO recipe_ai_insights (
                recipe_id, language, steps_json, validation_json, 
                suggestions_json, feasibility_score, model
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (recipe_id, language)
            DO UPDATE SET
                steps_json = EXCLUDED.steps_json,
                validation_json = EXCLUDED.validation_json,
                suggestions_json = EXCLUDED.suggestions_json,
                feasibility_score = EXCLUDED.feasibility_score,
                model = EXCLUDED.model,
                updated_at = CURRENT_TIMESTAMP
            RETURNING *
            "#
        )
        .bind(recipe_id)
        .bind(language)
        .bind(Json(&steps))
        .bind(Json(&validation))
        .bind(Json(&suggestions))
        .bind(feasibility_score)
        .bind(model)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("❌ Failed to upsert recipe AI insights: {:?}", e);
            AppError::internal("Failed to save AI insights")
        })?;

        tracing::info!("✅ Upserted AI insights for recipe {} (language: {})", recipe_id, language);
        Ok(row.into())
    }

    /// Get AI insights for a recipe by language
    pub async fn get_by_recipe_and_language(
        &self,
        recipe_id: Uuid,
        language: &str,
    ) -> Result<Option<RecipeAIInsights>, AppError> {
        let row = sqlx::query_as::<_, RecipeAIInsightsRow>(
            r#"
            SELECT * FROM recipe_ai_insights
            WHERE recipe_id = $1 AND language = $2
            "#
        )
        .bind(recipe_id)
        .bind(language)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("❌ Failed to fetch AI insights: {:?}", e);
            AppError::internal("Failed to fetch AI insights")
        })?;

        Ok(row.map(Into::into))
    }

    /// Get all AI insights for a recipe (all languages)
    pub async fn get_all_by_recipe(
        &self,
        recipe_id: Uuid,
    ) -> Result<Vec<RecipeAIInsights>, AppError> {
        let rows = sqlx::query_as::<_, RecipeAIInsightsRow>(
            r#"
            SELECT * FROM recipe_ai_insights
            WHERE recipe_id = $1
            ORDER BY language
            "#
        )
        .bind(recipe_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("❌ Failed to fetch all AI insights: {:?}", e);
            AppError::internal("Failed to fetch AI insights")
        })?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Delete AI insights for a recipe (cascade handled by DB)
    pub async fn delete_by_recipe(&self, recipe_id: Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM recipe_ai_insights WHERE recipe_id = $1")
            .bind(recipe_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("❌ Failed to delete AI insights: {:?}", e);
                AppError::internal("Failed to delete AI insights")
            })?;

        tracing::info!("🗑️ Deleted AI insights for recipe {}", recipe_id);
        Ok(())
    }

    /// Delete specific language insights
    pub async fn delete_by_recipe_and_language(
        &self,
        recipe_id: Uuid,
        language: &str,
    ) -> Result<(), AppError> {
        sqlx::query("DELETE FROM recipe_ai_insights WHERE recipe_id = $1 AND language = $2")
            .bind(recipe_id)
            .bind(language)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("❌ Failed to delete AI insights: {:?}", e);
                AppError::internal("Failed to delete AI insights")
            })?;

        tracing::info!("🗑️ Deleted AI insights for recipe {} (language: {})", recipe_id, language);
        Ok(())
    }

    /// Get recipes with high feasibility scores
    pub async fn get_high_quality_recipes(
        &self,
        min_score: i32,
        limit: i64,
    ) -> Result<Vec<Uuid>, AppError> {
        let recipe_ids = sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT DISTINCT recipe_id
            FROM recipe_ai_insights
            WHERE feasibility_score >= $1
            ORDER BY feasibility_score DESC
            LIMIT $2
            "#
        )
        .bind(min_score)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("❌ Failed to fetch high-quality recipes: {:?}", e);
            AppError::internal("Failed to fetch recipes")
        })?;

        Ok(recipe_ids)
    }
}
