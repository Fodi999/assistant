use crate::domain::{
    CatalogIngredientId, Quantity, Recipe, RecipeComponent, RecipeId, RecipeIngredient, RecipeName,
    RecipeType, Servings,
};
use crate::shared::{AppError, AppResult, PaginatedResponse, PaginationParams, TenantId, UserId};
use async_trait::async_trait;
use sqlx::PgPool;
use std::collections::HashMap;
use uuid;

#[async_trait]
pub trait RecipeRepositoryTrait: Send + Sync {
    async fn create(&self, recipe: &Recipe, user_id: UserId, tenant_id: TenantId) -> AppResult<()>;
    async fn find_by_id(&self, id: RecipeId, tenant_id: TenantId) -> AppResult<Option<Recipe>>;
    async fn list_by_tenant(
        &self,
        tenant_id: TenantId,
        pagination: &PaginationParams,
    ) -> AppResult<PaginatedResponse<Recipe>>;
    async fn delete(&self, id: RecipeId, tenant_id: TenantId) -> AppResult<bool>;
    async fn update_ingredients(
        &self,
        recipe_id: RecipeId,
        ingredients: Vec<RecipeIngredient>,
        tenant_id: TenantId,
    ) -> AppResult<()>;
}

#[derive(Clone)]
pub struct RecipeRepository {
    pool: PgPool,
}

impl RecipeRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RecipeRepositoryTrait for RecipeRepository {
    async fn create(&self, recipe: &Recipe, user_id: UserId, tenant_id: TenantId) -> AppResult<()> {
        let mut tx = self.pool.begin().await.map_err(AppError::Database)?;

        // Insert recipe
        sqlx::query(
            r#"
            INSERT INTO recipes (id, user_id, tenant_id, name, recipe_type, servings, instructions)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(recipe.id().as_uuid())
        .bind(user_id.as_uuid())
        .bind(tenant_id.as_uuid())
        .bind(recipe.name().as_str())
        .bind(recipe.recipe_type().as_str())
        .bind(recipe.servings().count() as i32)
        .bind(recipe.instructions())
        .execute(&mut *tx)
        .await
        .map_err(AppError::Database)?;

        // Insert ingredients
        for ingredient in recipe.ingredients() {
            sqlx::query(
                r#"
                INSERT INTO recipe_ingredients (recipe_id, catalog_ingredient_id, quantity)
                VALUES ($1, $2, $3)
                "#,
            )
            .bind(recipe.id().as_uuid())
            .bind(ingredient.catalog_ingredient_id().as_uuid())
            .bind(ingredient.quantity().decimal())
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;
        }

        // Insert components (other recipes used in this recipe)
        for component in recipe.components() {
            sqlx::query(
                r#"
                INSERT INTO recipe_components (recipe_id, component_recipe_id, quantity)
                VALUES ($1, $2, $3)
                "#,
            )
            .bind(recipe.id().as_uuid())
            .bind(component.component_recipe_id().as_uuid())
            .bind(component.quantity())
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;
        }

        tx.commit().await.map_err(AppError::Database)?;
        Ok(())
    }

    /// 🔒 TENANT ISOLATION: find_by_id filters by tenant_id (not user_id)
    async fn find_by_id(&self, id: RecipeId, tenant_id: TenantId) -> AppResult<Option<Recipe>> {
        // Fetch recipe — filtered by tenant_id
        let recipe_row = sqlx::query_as::<_, (uuid::Uuid, uuid::Uuid, uuid::Uuid, String, String, i32, Option<String>, time::OffsetDateTime, time::OffsetDateTime)>(
            r#"
            SELECT id, user_id, tenant_id, name, recipe_type, servings, instructions, created_at, updated_at
            FROM recipes
            WHERE id = $1 AND tenant_id = $2
            "#
        )
        .bind(id.as_uuid())
        .bind(tenant_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let Some((
            row_id,
            row_user_id,
            row_tenant_id,
            row_name,
            row_recipe_type,
            row_servings,
            row_instructions,
            row_created_at,
            row_updated_at,
        )) = recipe_row
        else {
            return Ok(None);
        };

        // Fetch ingredients
        let ingredients_rows = sqlx::query_as::<_, (uuid::Uuid, rust_decimal::Decimal)>(
            r#"
            SELECT catalog_ingredient_id, quantity
            FROM recipe_ingredients
            WHERE recipe_id = $1
            "#,
        )
        .bind(id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let ingredients: Vec<RecipeIngredient> = ingredients_rows
            .into_iter()
            .map(|(catalog_id, qty)| {
                let quantity = Quantity::from_decimal(qty).map_err(|_| {
                    AppError::Internal("Corrupted quantity data in database".to_string())
                })?;
                Ok(RecipeIngredient::new(
                    CatalogIngredientId::from_uuid(catalog_id),
                    quantity,
                ))
            })
            .collect::<AppResult<Vec<_>>>()?;

        // Fetch components
        let components_rows = sqlx::query_as::<_, (uuid::Uuid, rust_decimal::Decimal)>(
            r#"
            SELECT component_recipe_id, quantity
            FROM recipe_components
            WHERE recipe_id = $1
            "#,
        )
        .bind(id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let components: Vec<RecipeComponent> = components_rows
            .into_iter()
            .map(|(component_id, qty)| RecipeComponent::new(RecipeId::from_uuid(component_id), qty))
            .collect::<AppResult<Vec<_>>>()?;

        let recipe = Recipe::from_parts(
            RecipeId::from_uuid(row_id),
            UserId::from_uuid(row_user_id),
            TenantId::from_uuid(row_tenant_id),
            RecipeName::new(row_name)?,
            RecipeType::from_str(&row_recipe_type)?,
            Servings::new(row_servings as u32)?,
            ingredients,
            components,
            row_instructions,
            row_created_at,
            row_updated_at,
        );

        Ok(Some(recipe))
    }

    /// 🔒 TENANT ISOLATION + Pagination + Batch loading (no N+1)
    async fn list_by_tenant(
        &self,
        tenant_id: TenantId,
        pagination: &PaginationParams,
    ) -> AppResult<PaginatedResponse<Recipe>> {
        // 1. Count total recipes for this tenant
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM recipes WHERE tenant_id = $1",
        )
        .bind(tenant_id.as_uuid())
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;

        // 2. Fetch paginated recipes
        let recipe_rows = sqlx::query_as::<_, (uuid::Uuid, uuid::Uuid, uuid::Uuid, String, String, i32, Option<String>, time::OffsetDateTime, time::OffsetDateTime)>(
            r#"
            SELECT id, user_id, tenant_id, name, recipe_type, servings, instructions, created_at, updated_at
            FROM recipes
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(tenant_id.as_uuid())
        .bind(pagination.limit())
        .bind(pagination.offset())
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        if recipe_rows.is_empty() {
            return Ok(PaginatedResponse::new(vec![], total, pagination));
        }

        // 3. Collect recipe IDs for batch loading
        let recipe_ids: Vec<uuid::Uuid> = recipe_rows.iter().map(|r| r.0).collect();

        // 4. Batch load ALL ingredients for all recipes in one query (fixes N+1)
        let all_ingredients = sqlx::query_as::<_, (uuid::Uuid, uuid::Uuid, rust_decimal::Decimal)>(
            r#"
            SELECT recipe_id, catalog_ingredient_id, quantity
            FROM recipe_ingredients
            WHERE recipe_id = ANY($1)
            "#,
        )
        .bind(&recipe_ids)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        // Group ingredients by recipe_id
        let mut ingredients_map: HashMap<uuid::Uuid, Vec<RecipeIngredient>> = HashMap::new();
        for (recipe_id, catalog_id, qty) in all_ingredients {
            let quantity = Quantity::from_decimal(qty).map_err(|_| {
                AppError::Internal("Corrupted quantity data in database".to_string())
            })?;
            ingredients_map
                .entry(recipe_id)
                .or_default()
                .push(RecipeIngredient::new(
                    CatalogIngredientId::from_uuid(catalog_id),
                    quantity,
                ));
        }

        // 5. Batch load ALL components for all recipes in one query (fixes N+1)
        let all_components = sqlx::query_as::<_, (uuid::Uuid, uuid::Uuid, rust_decimal::Decimal)>(
            r#"
            SELECT recipe_id, component_recipe_id, quantity
            FROM recipe_components
            WHERE recipe_id = ANY($1)
            "#,
        )
        .bind(&recipe_ids)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        // Group components by recipe_id
        let mut components_map: HashMap<uuid::Uuid, Vec<RecipeComponent>> = HashMap::new();
        for (recipe_id, component_id, qty) in all_components {
            let component = RecipeComponent::new(RecipeId::from_uuid(component_id), qty)?;
            components_map
                .entry(recipe_id)
                .or_default()
                .push(component);
        }

        // 6. Assemble Recipe objects
        let mut recipes = Vec::with_capacity(recipe_rows.len());
        for (
            row_id,
            row_user_id,
            row_tenant_id,
            row_name,
            row_recipe_type,
            row_servings,
            row_instructions,
            row_created_at,
            row_updated_at,
        ) in recipe_rows
        {
            let ingredients = ingredients_map.remove(&row_id).unwrap_or_default();
            let components = components_map.remove(&row_id).unwrap_or_default();

            let recipe = Recipe::from_parts(
                RecipeId::from_uuid(row_id),
                UserId::from_uuid(row_user_id),
                TenantId::from_uuid(row_tenant_id),
                RecipeName::new(row_name)?,
                RecipeType::from_str(&row_recipe_type)?,
                Servings::new(row_servings as u32)?,
                ingredients,
                components,
                row_instructions,
                row_created_at,
                row_updated_at,
            );

            recipes.push(recipe);
        }

        Ok(PaginatedResponse::new(recipes, total, pagination))
    }

    /// 🔒 TENANT ISOLATION: delete filters by tenant_id
    async fn delete(&self, id: RecipeId, tenant_id: TenantId) -> AppResult<bool> {
        let result = sqlx::query(
            r#"
            DELETE FROM recipes
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id.as_uuid())
        .bind(tenant_id.as_uuid())
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(result.rows_affected() > 0)
    }

    /// 🔒 TENANT ISOLATION: update_ingredients verifies by tenant_id
    async fn update_ingredients(
        &self,
        recipe_id: RecipeId,
        ingredients: Vec<RecipeIngredient>,
        tenant_id: TenantId,
    ) -> AppResult<()> {
        let mut tx = self.pool.begin().await.map_err(AppError::Database)?;

        // Verify recipe belongs to tenant
        let exists_row = sqlx::query_as::<_, (bool,)>(
            r#"
            SELECT EXISTS(SELECT 1 FROM recipes WHERE id = $1 AND tenant_id = $2)
            "#,
        )
        .bind(recipe_id.as_uuid())
        .bind(tenant_id.as_uuid())
        .fetch_one(&mut *tx)
        .await
        .map_err(AppError::Database)?;

        if !exists_row.0 {
            return Err(AppError::NotFound("Recipe not found".to_string()));
        }

        // Delete old ingredients
        sqlx::query(
            r#"
            DELETE FROM recipe_ingredients
            WHERE recipe_id = $1
            "#,
        )
        .bind(recipe_id.as_uuid())
        .execute(&mut *tx)
        .await
        .map_err(AppError::Database)?;

        // Insert new ingredients
        for ingredient in ingredients {
            sqlx::query(
                r#"
                INSERT INTO recipe_ingredients (recipe_id, catalog_ingredient_id, quantity)
                VALUES ($1, $2, $3)
                "#,
            )
            .bind(recipe_id.as_uuid())
            .bind(ingredient.catalog_ingredient_id().as_uuid())
            .bind(ingredient.quantity().decimal())
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;
        }

        tx.commit().await.map_err(AppError::Database)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recipe_repository_trait_is_object_safe() {
        let _: Option<Box<dyn RecipeRepositoryTrait>> = None;
    }
}
