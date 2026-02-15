// Recipe V2 Repository - CRUD for recipes and ingredients
use crate::domain::recipe_v2::{Recipe, RecipeId, RecipeIngredient, RecipeIngredientId, RecipeStatus};
use crate::shared::{AppError, AppResult, Language, TenantId, UserId};
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use time::OffsetDateTime;

// ========== Recipe Repository ==========

#[async_trait]
pub trait RecipeV2RepositoryTrait: Send + Sync {
    async fn save(&self, recipe: &Recipe) -> AppResult<()>;
    async fn find_by_id(&self, id: RecipeId) -> AppResult<Option<Recipe>>;
    async fn find_by_user_id(&self, user_id: UserId) -> AppResult<Vec<Recipe>>;
    async fn update(&self, recipe: &Recipe) -> AppResult<()>;
    async fn delete(&self, id: RecipeId) -> AppResult<()>;
}

#[derive(Clone)]
pub struct RecipeRepositoryV2 {
    pool: PgPool,
}

impl RecipeRepositoryV2 {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RecipeV2RepositoryTrait for RecipeRepositoryV2 {
    async fn save(&self, recipe: &Recipe) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO recipes (
                id, user_id, tenant_id,
                name_default, instructions_default, language_default,
                servings,
                total_cost_cents, cost_per_serving_cents,
                status, is_public, published_at,
                created_at, updated_at
            ) VALUES (
                $1, $2, $3,
                $4, $5, $6,
                $7,
                $8, $9,
                $10, $11, $12,
                $13, $14
            )
            "#,
        )
        .bind(recipe.id.0)
        .bind(recipe.user_id.0)
        .bind(recipe.tenant_id.0)
        .bind(&recipe.name_default)
        .bind(&recipe.instructions_default)
        .bind(recipe.language_default.code())
        .bind(recipe.servings)
        .bind(recipe.total_cost_cents.map(|c| c as i64))
        .bind(recipe.cost_per_serving_cents.map(|c| c as i64))
        .bind(recipe.status.as_str())
        .bind(recipe.is_public)
        .bind(recipe.published_at)
        .bind(recipe.created_at)
        .bind(recipe.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(&format!("Failed to save recipe: {}", e)))?;
        
        Ok(())
    }

    async fn find_by_id(&self, id: RecipeId) -> AppResult<Option<Recipe>> {
        let row = sqlx::query(
            r#"
            SELECT
                id, user_id, tenant_id,
                name_default, instructions_default, language_default,
                servings,
                total_cost_cents, cost_per_serving_cents,
                status, is_public, published_at,
                created_at, updated_at
            FROM recipes
            WHERE id = $1
            "#,
        )
        .bind(id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::internal(&format!("Failed to find recipe: {}", e)))?;

        match row {
            Some(row) => {
                let recipe = Recipe {
                    id: RecipeId(row.get("id")),
                    user_id: UserId(row.get("user_id")),
                    tenant_id: TenantId(row.get("tenant_id")),
                    name_default: row.get("name_default"),
                    instructions_default: row.get("instructions_default"),
                    language_default: Language::from_str(row.get("language_default"))
                        .map_err(|e| AppError::internal(&format!("Invalid language: {}", e)))?,
                    servings: row.get("servings"),
                    total_cost_cents: row.get::<Option<i64>, _>("total_cost_cents").map(|v| v as i32),
                    cost_per_serving_cents: row.get::<Option<i64>, _>("cost_per_serving_cents").map(|v| v as i32),
                    status: RecipeStatus::from_str(row.get("status"))?,
                    is_public: row.get("is_public"),
                    published_at: row.get("published_at"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                };
                Ok(Some(recipe))
            }
            None => Ok(None),
        }
    }

    async fn find_by_user_id(&self, user_id: UserId) -> AppResult<Vec<Recipe>> {
        let rows = sqlx::query(
            r#"
            SELECT
                id, user_id, tenant_id,
                name_default, instructions_default, language_default,
                servings,
                total_cost_cents, cost_per_serving_cents,
                status, is_public, published_at,
                created_at, updated_at
            FROM recipes
            WHERE user_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id.0)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::internal(&format!("Failed to find recipes: {}", e)))?;

        let mut recipes = Vec::new();
        for row in rows {
            recipes.push(Recipe {
                id: RecipeId(row.get("id")),
                user_id: UserId(row.get("user_id")),
                tenant_id: TenantId(row.get("tenant_id")),
                name_default: row.get("name_default"),
                instructions_default: row.get("instructions_default"),
                language_default: Language::from_str(row.get("language_default"))
                    .map_err(|e| AppError::internal(&format!("Invalid language: {}", e)))?,
                servings: row.get("servings"),
                total_cost_cents: row.get::<Option<i64>, _>("total_cost_cents").map(|v| v as i32),
                cost_per_serving_cents: row.get::<Option<i64>, _>("cost_per_serving_cents").map(|v| v as i32),
                status: RecipeStatus::from_str(row.get("status"))?,
                is_public: row.get("is_public"),
                published_at: row.get("published_at"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(recipes)
    }

    async fn update(&self, recipe: &Recipe) -> AppResult<()> {
        let result = sqlx::query(
            r#"
            UPDATE recipes SET
                name_default = $2,
                instructions_default = $3,
                language_default = $4,
                servings = $5,
                total_cost_cents = $6,
                cost_per_serving_cents = $7,
                status = $8,
                is_public = $9,
                published_at = $10,
                updated_at = $11
            WHERE id = $1
            "#,
        )
        .bind(recipe.id.0)
        .bind(&recipe.name_default)
        .bind(&recipe.instructions_default)
        .bind(recipe.language_default.code())
        .bind(recipe.servings)
        .bind(recipe.total_cost_cents.map(|c| c as i64))
        .bind(recipe.cost_per_serving_cents.map(|c| c as i64))
        .bind(recipe.status.as_str())
        .bind(recipe.is_public)
        .bind(recipe.published_at)
        .bind(OffsetDateTime::now_utc())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(&format!("Failed to update recipe: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found("Recipe"));
        }

        Ok(())
    }

    async fn delete(&self, id: RecipeId) -> AppResult<()> {
        let result = sqlx::query("DELETE FROM recipes WHERE id = $1")
            .bind(id.0)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(&format!("Failed to delete recipe: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found("Recipe"));
        }

        Ok(())
    }
}

// ========== Recipe Ingredient Repository ==========

#[async_trait]
pub trait RecipeIngredientRepositoryTrait: Send + Sync {
    async fn save(&self, ingredient: &RecipeIngredient) -> AppResult<()>;
    async fn find_by_recipe_id(&self, recipe_id: RecipeId) -> AppResult<Vec<RecipeIngredient>>;
    async fn delete_by_recipe_id(&self, recipe_id: RecipeId) -> AppResult<()>;
}

#[derive(Clone)]
pub struct RecipeIngredientRepository {
    pool: PgPool,
}

impl RecipeIngredientRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RecipeIngredientRepositoryTrait for RecipeIngredientRepository {
    async fn save(&self, ingredient: &RecipeIngredient) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO recipe_ingredients (
                id, recipe_id, catalog_ingredient_id,
                quantity, unit,
                cost_at_use_cents,
                catalog_ingredient_name_snapshot,
                created_at
            ) VALUES (
                $1, $2, $3,
                $4, $5,
                $6,
                $7,
                $8
            )
            "#,
        )
        .bind(ingredient.id.0)
        .bind(ingredient.recipe_id.0)
        .bind(ingredient.catalog_ingredient_id)
        .bind(ingredient.quantity)
        .bind(&ingredient.unit)
        .bind(ingredient.cost_at_use_cents.map(|c| c as i64))
        .bind(&ingredient.catalog_ingredient_name_snapshot)
        .bind(ingredient.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(&format!("Failed to save recipe ingredient: {}", e)))?;

        Ok(())
    }

    async fn find_by_recipe_id(&self, recipe_id: RecipeId) -> AppResult<Vec<RecipeIngredient>> {
        let rows = sqlx::query(
            r#"
            SELECT
                id, recipe_id, catalog_ingredient_id,
                quantity, unit,
                cost_at_use_cents,
                catalog_ingredient_name_snapshot,
                created_at
            FROM recipe_ingredients
            WHERE recipe_id = $1
            ORDER BY created_at ASC
            "#,
        )
        .bind(recipe_id.0)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::internal(&format!("Failed to find recipe ingredients: {}", e)))?;

        let mut ingredients = Vec::new();
        for row in rows {
            ingredients.push(RecipeIngredient {
                id: RecipeIngredientId(row.get("id")),
                recipe_id: RecipeId(row.get("recipe_id")),
                catalog_ingredient_id: row.get("catalog_ingredient_id"),
                quantity: row.get("quantity"),
                unit: row.get("unit"),
                cost_at_use_cents: row.get::<Option<i64>, _>("cost_at_use_cents").map(|v| v as i32),
                catalog_ingredient_name_snapshot: row.get("catalog_ingredient_name_snapshot"),
                created_at: row.get("created_at"),
            });
        }

        Ok(ingredients)
    }

    async fn delete_by_recipe_id(&self, recipe_id: RecipeId) -> AppResult<()> {
        sqlx::query("DELETE FROM recipe_ingredients WHERE recipe_id = $1")
            .bind(recipe_id.0)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::internal(&format!("Failed to delete recipe ingredients: {}", e)))?;

        Ok(())
    }
}
