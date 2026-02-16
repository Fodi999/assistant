// Recipe Translation Repository - CRUD for translations
use crate::domain::recipe_v2::{RecipeId, RecipeTranslation, TranslationSource};
use crate::shared::{AppError, AppResult, Language};
use async_trait::async_trait;
use sqlx::{PgPool, Row};

#[async_trait]
pub trait RecipeTranslationRepositoryTrait: Send + Sync {
    async fn save(&self, translation: &RecipeTranslation) -> AppResult<()>;
    async fn find_by_recipe_id(&self, recipe_id: RecipeId) -> AppResult<Vec<RecipeTranslation>>;
    async fn find_by_recipe_and_language(
        &self,
        recipe_id: RecipeId,
        language: Language,
    ) -> AppResult<Option<RecipeTranslation>>;
    async fn delete_by_recipe_id(&self, recipe_id: RecipeId) -> AppResult<()>;
}

#[derive(Clone)]
pub struct RecipeTranslationRepository {
    pool: PgPool,
}

impl RecipeTranslationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RecipeTranslationRepositoryTrait for RecipeTranslationRepository {
    async fn save(&self, translation: &RecipeTranslation) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO recipe_translations (
                id, recipe_id, language,
                name, instructions,
                translated_by,
                translated_at
            ) VALUES (
                $1, $2, $3,
                $4, $5,
                $6,
                $7
            )
            ON CONFLICT (recipe_id, language)
            DO UPDATE SET
                name = EXCLUDED.name,
                instructions = EXCLUDED.instructions,
                translated_by = EXCLUDED.translated_by
            "#,
        )
        .bind(translation.id)
        .bind(translation.recipe_id.0)
        .bind(translation.language.code())
        .bind(&translation.name)
        .bind(&translation.instructions)
        .bind(translation.translated_by.as_str())
        .bind(translation.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(&format!("Failed to save recipe translation: {}", e)))?;

        Ok(())
    }

    async fn find_by_recipe_id(&self, recipe_id: RecipeId) -> AppResult<Vec<RecipeTranslation>> {
        let rows = sqlx::query(
            r#"
            SELECT
                id, recipe_id, language,
                name, instructions,
                translated_by,
                translated_at
            FROM recipe_translations
            WHERE recipe_id = $1
            ORDER BY translated_at ASC
            "#,
        )
        .bind(recipe_id.0)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::internal(&format!("Failed to find recipe translations: {}", e)))?;

        let mut translations = Vec::new();
        for row in rows {
            translations.push(RecipeTranslation {
                id: row.try_get("id").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?,
                recipe_id: RecipeId(row.try_get("recipe_id").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?),
                language: Language::from_str(row.try_get("language").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?)
                    .map_err(|e| AppError::internal(&format!("Invalid language: {}", e)))?,
                name: row.try_get("name").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?,
                instructions: row.try_get("instructions").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?,
                translated_by: TranslationSource::from_str(row.try_get("translated_by").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?)?,
                created_at: row.try_get("translated_at").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?,
            });
        }

        Ok(translations)
    }

    async fn find_by_recipe_and_language(
        &self,
        recipe_id: RecipeId,
        language: Language,
    ) -> AppResult<Option<RecipeTranslation>> {
        let row = sqlx::query(
            r#"
            SELECT
                id, recipe_id, language,
                name, instructions,
                translated_by,
                translated_at
            FROM recipe_translations
            WHERE recipe_id = $1 AND language = $2
            "#,
        )
        .bind(recipe_id.0)
        .bind(language.code())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::internal(&format!("Failed to find recipe translation: {}", e)))?;

        match row {
            Some(row) => {
                let translation = RecipeTranslation {
                    id: row.try_get("id").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?,
                    recipe_id: RecipeId(row.try_get("recipe_id").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?),
                    language: Language::from_str(row.try_get("language").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?)
                        .map_err(|e| AppError::internal(&format!("Invalid language: {}", e)))?,
                    name: row.try_get("name").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?,
                    instructions: row.try_get("instructions").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?,
                    translated_by: TranslationSource::from_str(row.try_get("translated_by").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?)?,
                    created_at: row.try_get("translated_at").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?,
                };
                Ok(Some(translation))
            }
            None => Ok(None),
        }
    }

    async fn delete_by_recipe_id(&self, recipe_id: RecipeId) -> AppResult<()> {
        sqlx::query("DELETE FROM recipe_translations WHERE recipe_id = $1")
            .bind(recipe_id.0)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(&format!("Failed to delete recipe translations: {}", e)))?;

        Ok(())
    }
}
