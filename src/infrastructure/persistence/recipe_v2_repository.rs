// Recipe V2 Repository - CRUD for recipes and ingredients
use crate::domain::recipe_v2::{Recipe, RecipeId, RecipeIngredient, RecipeIngredientId};
use crate::shared::{AppError, AppResult, TenantId, UserId, Language};
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use time::OffsetDateTime;

// ========== Recipe Repository ==========

#[async_trait]
pub trait RecipeV2RepositoryTrait: Send + Sync {
    async fn save(&self, recipe: &Recipe) -> AppResult<()>;
    async fn find_by_id(&self, id: RecipeId, tenant_id: TenantId) -> AppResult<Option<Recipe>>;
    async fn find_by_user_id(&self, user_id: UserId, tenant_id: TenantId) -> AppResult<Vec<Recipe>>;
    async fn update(&self, recipe: &Recipe) -> AppResult<()>;
    async fn delete(&self, id: RecipeId, tenant_id: TenantId) -> AppResult<()>;
}

#[derive(Clone)]
pub struct RecipeRepositoryV2 {
    pool: PgPool,
}

impl RecipeRepositoryV2 {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn row_to_recipe(row: &sqlx::postgres::PgRow) -> AppResult<Recipe> {
        let language_str: String = row.try_get("language_default")
            .map_err(|e| AppError::internal(&format!("Failed to get language_default: {}", e)))?;
            
        Ok(Recipe {
            id: RecipeId(row.try_get("id")
                .map_err(|e| AppError::internal(&format!("Failed to get id: {}", e)))?),
            user_id: UserId::from_uuid(row.try_get("user_id")
                .map_err(|e| AppError::internal(&format!("Failed to get user_id: {}", e)))?),
            tenant_id: TenantId::from_uuid(row.try_get("tenant_id")
                .map_err(|e| AppError::internal(&format!("Failed to get tenant_id: {}", e)))?),
            name_default: row.try_get("name_default")
                .map_err(|e| AppError::internal(&format!("Failed to get name_default: {}", e)))?,
            instructions_default: row.try_get("instructions_default")
                .map_err(|e| AppError::internal(&format!("Failed to get instructions_default: {}", e)))?,
            language_default: Language::from_code(&language_str).unwrap_or(Language::En),
            servings: row.try_get("servings")
                .map_err(|e| AppError::internal(&format!("Failed to get servings: {}", e)))?,
            total_cost_cents: row.try_get::<Option<i64>, _>("total_cost_cents")
                .map_err(|e| AppError::internal(&format!("Failed to get total_cost_cents: {}", e)))?
                .map(|v| v as i32),
            cost_per_serving_cents: row.try_get::<Option<i64>, _>("cost_per_serving_cents")
                .map_err(|e| AppError::internal(&format!("Failed to get cost_per_serving_cents: {}", e)))?
                .map(|v| v as i32),
            status: row.try_get::<String, _>("status")
                .map_err(|e| AppError::internal(&format!("Failed to get status: {}", e)))?
                .parse().unwrap_or_default(),
            is_public: row.try_get("is_public")
                .map_err(|e| AppError::internal(&format!("Failed to get is_public: {}", e)))?,
            published_at: row.try_get("published_at")
                .map_err(|e| AppError::internal(&format!("Failed to get published_at: {}", e)))?,
            created_at: row.try_get("created_at")
                .map_err(|e| AppError::internal(&format!("Failed to get created_at: {}", e)))?,
            updated_at: row.try_get("updated_at")
                .map_err(|e| AppError::internal(&format!("Failed to get updated_at: {}", e)))?,
        })
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

    async fn find_by_id(&self, id: RecipeId, tenant_id: TenantId) -> AppResult<Option<Recipe>> {
        let row = sqlx::query(
            r#"
            SELECT id, user_id, tenant_id, name_default, instructions_default, language_default, 
                   servings, total_cost_cents, cost_per_serving_cents, status, is_public, 
                   published_at, created_at, updated_at
            FROM recipes
            WHERE id = $1 AND tenant_id = $2
            "#
        )
        .bind(id.as_uuid())
        .bind(tenant_id.as_uuid())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(Some(Self::row_to_recipe(&r)?)),
            None => Ok(None),
        }
    }

    async fn find_by_user_id(&self, user_id: UserId, tenant_id: TenantId) -> AppResult<Vec<Recipe>> {
        let rows = sqlx::query(
            r#"
            SELECT id, user_id, tenant_id, name_default, instructions_default, language_default, 
                   servings, total_cost_cents, cost_per_serving_cents, status, is_public, 
                   published_at, created_at, updated_at
            FROM recipes
            WHERE user_id = $1 AND tenant_id = $2
            ORDER BY created_at DESC
            "#
        )
        .bind(user_id.as_uuid())
        .bind(tenant_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;

        let mut recipes = Vec::with_capacity(rows.len());
        for row in rows {
            recipes.push(Self::row_to_recipe(&row)?);
        }

        Ok(recipes)
    }

    async fn update(&self, recipe: &Recipe) -> AppResult<()> {
        // 🎯 TENANT ISOLATION: Explicitly filter by tenant_id in WHERE clause
        sqlx::query(
            r#"
            UPDATE recipes
            SET name_default = $1, instructions_default = $2, language_default = $3, 
                servings = $4, total_cost_cents = $5, cost_per_serving_cents = $6, 
                status = $7, is_public = $8, published_at = $9, updated_at = $10
            WHERE id = $11 AND tenant_id = $12
            "#
        )
        .bind(&recipe.name_default)
        .bind(&recipe.instructions_default)
        .bind(recipe.language_default.code())
        .bind(recipe.servings)
        .bind(recipe.total_cost_cents)
        .bind(recipe.cost_per_serving_cents)
        .bind(recipe.status.as_str())
        .bind(recipe.is_public)
        .bind(recipe.published_at)
        .bind(OffsetDateTime::now_utc())
        .bind(recipe.id.as_uuid())
        .bind(recipe.tenant_id.as_uuid())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete(&self, id: RecipeId, tenant_id: TenantId) -> AppResult<()> {
        sqlx::query("DELETE FROM recipes WHERE id = $1 AND tenant_id = $2")
            .bind(id.as_uuid())
            .bind(tenant_id.as_uuid())
            .execute(&self.pool)
            .await?;
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
                id: RecipeIngredientId(row.try_get("id")
                    .map_err(|e| AppError::internal(&format!("Failed to get id: {}", e)))?),
                recipe_id: RecipeId(row.try_get("recipe_id")
                    .map_err(|e| AppError::internal(&format!("Failed to get recipe_id: {}", e)))?),
                catalog_ingredient_id: row.try_get("catalog_ingredient_id")
                    .map_err(|e| AppError::internal(&format!("Failed to get catalog_ingredient_id: {}", e)))?,
                quantity: row.try_get("quantity")
                    .map_err(|e| AppError::internal(&format!("Failed to decode quantity: {}. This often happens if the database column type (e.g. FLOAT8) does not match the Rust field type (Decimal).", e)))?,
                unit: row.try_get("unit")
                    .map_err(|e| AppError::internal(&format!("Failed to get unit: {}", e)))?,
                cost_at_use_cents: row.try_get::<Option<i64>, _>("cost_at_use_cents")
                    .map_err(|e| AppError::internal(&format!("Failed to get cost_at_use_cents: {}", e)))?
                    .map(|v| v as i32),
                catalog_ingredient_name_snapshot: row.try_get("catalog_ingredient_name_snapshot")
                    .map_err(|e| AppError::internal(&format!("Failed to get catalog_ingredient_name_snapshot: {}", e)))?,
                created_at: row.try_get("created_at")
                    .map_err(|e| AppError::internal(&format!("Failed to get created_at: {}", e)))?,
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
