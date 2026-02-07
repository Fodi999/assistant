use crate::domain::{
    Recipe, RecipeId, RecipeName, RecipeIngredient, RecipeComponent, RecipeType,
    Servings, CatalogIngredientId, Quantity,
};
use crate::shared::{AppError, AppResult, UserId, TenantId};
use async_trait::async_trait;
use sqlx::PgPool;
use uuid;

#[async_trait]
pub trait RecipeRepositoryTrait: Send + Sync {
    async fn create(&self, recipe: &Recipe, user_id: UserId, tenant_id: TenantId) -> AppResult<()>;
    async fn find_by_id(&self, id: RecipeId, user_id: UserId) -> AppResult<Option<Recipe>>;
    async fn list_by_user(&self, user_id: UserId) -> AppResult<Vec<Recipe>>;
    async fn delete(&self, id: RecipeId, user_id: UserId) -> AppResult<bool>;
    async fn update_ingredients(&self, recipe_id: RecipeId, ingredients: Vec<RecipeIngredient>, user_id: UserId) -> AppResult<()>;
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
            "#
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
                "#
            )
            .bind(recipe.id().as_uuid())
            .bind(ingredient.catalog_ingredient_id().as_uuid())
            .bind(ingredient.quantity().value())
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
                "#
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

    async fn find_by_id(&self, id: RecipeId, user_id: UserId) -> AppResult<Option<Recipe>> {
        // Fetch recipe
        let recipe_row = sqlx::query_as::<_, (uuid::Uuid, uuid::Uuid, uuid::Uuid, String, String, i32, Option<String>, time::OffsetDateTime, time::OffsetDateTime)>(
            r#"
            SELECT id, user_id, tenant_id, name, recipe_type, servings, instructions, created_at, updated_at
            FROM recipes
            WHERE id = $1 AND user_id = $2
            "#
        )
        .bind(id.as_uuid())
        .bind(user_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let Some((row_id, row_user_id, row_tenant_id, row_name, row_recipe_type, row_servings, row_instructions, row_created_at, row_updated_at)) = recipe_row else {
            return Ok(None);
        };

        // Fetch ingredients
        let ingredients_rows = sqlx::query_as::<_, (uuid::Uuid, f64)>(
            r#"
            SELECT catalog_ingredient_id, quantity
            FROM recipe_ingredients
            WHERE recipe_id = $1
            "#
        )
        .bind(id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let ingredients: Vec<RecipeIngredient> = ingredients_rows
            .into_iter()
            .map(|(catalog_id, qty)| {
                let quantity = Quantity::new(qty)
                    .map_err(|_| AppError::Internal("Corrupted quantity data in database".to_string()))?;
                Ok(RecipeIngredient::new(
                    CatalogIngredientId::from_uuid(catalog_id),
                    quantity
                ))
            })
            .collect::<AppResult<Vec<_>>>()?;

        // Fetch components (recipes used in this recipe)
        let components_rows = sqlx::query_as::<_, (uuid::Uuid, f64)>(
            r#"
            SELECT component_recipe_id, quantity
            FROM recipe_components
            WHERE recipe_id = $1
            "#
        )
        .bind(id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let components: Vec<RecipeComponent> = components_rows
            .into_iter()
            .map(|(component_id, qty)| {
                RecipeComponent::new(RecipeId::from_uuid(component_id), qty)
            })
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
            row_updated_at
        );

        Ok(Some(recipe))
    }

    async fn list_by_user(&self, user_id: UserId) -> AppResult<Vec<Recipe>> {
        // TODO: Optimize N+1 query problem - consider using JOIN or WHERE recipe_id = ANY($1)
        // Current implementation: 1 query for recipes + N queries for ingredients + N queries for components
        // This is acceptable for small datasets but will be a bottleneck with many recipes
        
        // Fetch all recipes for user
        let recipe_rows = sqlx::query_as::<_, (uuid::Uuid, uuid::Uuid, uuid::Uuid, String, String, i32, Option<String>, time::OffsetDateTime, time::OffsetDateTime)>(
            r#"
            SELECT id, user_id, tenant_id, name, recipe_type, servings, instructions, created_at, updated_at
            FROM recipes
            WHERE user_id = $1
            ORDER BY created_at DESC
            "#
        )
        .bind(user_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let mut recipes = Vec::new();

        for (row_id, row_user_id, row_tenant_id, row_name, row_recipe_type, row_servings, row_instructions, row_created_at, row_updated_at) in recipe_rows {
            let recipe_id = RecipeId::from_uuid(row_id);

            // Fetch ingredients for this recipe (N+1 issue)
            let ingredients_rows = sqlx::query_as::<_, (uuid::Uuid, f64)>(
                r#"
                SELECT catalog_ingredient_id, quantity
                FROM recipe_ingredients
                WHERE recipe_id = $1
                "#
            )
            .bind(row_id)
            .fetch_all(&self.pool)
            .await
            .map_err(AppError::Database)?;

            let ingredients: Vec<RecipeIngredient> = ingredients_rows
                .into_iter()
                .map(|(catalog_id, qty)| {
                    let quantity = Quantity::new(qty)
                        .map_err(|_| AppError::Internal("Corrupted quantity data in database".to_string()))?;
                    Ok(RecipeIngredient::new(
                        CatalogIngredientId::from_uuid(catalog_id),
                        quantity
                    ))
                })
                .collect::<AppResult<Vec<_>>>()?;

            // Fetch components for this recipe (N+1 issue)
            let components_rows = sqlx::query_as::<_, (uuid::Uuid, f64)>(
                r#"
                SELECT component_recipe_id, quantity
                FROM recipe_components
                WHERE recipe_id = $1
                "#
            )
            .bind(row_id)
            .fetch_all(&self.pool)
            .await
            .map_err(AppError::Database)?;

            let components: Vec<RecipeComponent> = components_rows
                .into_iter()
                .map(|(component_id, qty)| {
                    RecipeComponent::new(RecipeId::from_uuid(component_id), qty)
                })
                .collect::<AppResult<Vec<_>>>()?;

            let recipe = Recipe::from_parts(
                recipe_id,
                UserId::from_uuid(row_user_id),
                TenantId::from_uuid(row_tenant_id),
                RecipeName::new(row_name)?,
                RecipeType::from_str(&row_recipe_type)?,
                Servings::new(row_servings as u32)?,
                ingredients,
                components,
                row_instructions,
                row_created_at,
                row_updated_at
            );

            recipes.push(recipe);
        }

        Ok(recipes)
    }

    async fn delete(&self, id: RecipeId, user_id: UserId) -> AppResult<bool> {
        let result = sqlx::query(
            r#"
            DELETE FROM recipes
            WHERE id = $1 AND user_id = $2
            "#
        )
        .bind(id.as_uuid())
        .bind(user_id.as_uuid())
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(result.rows_affected() > 0)
    }

    async fn update_ingredients(
        &self,
        recipe_id: RecipeId,
        ingredients: Vec<RecipeIngredient>,
        user_id: UserId
    ) -> AppResult<()> {
        let mut tx = self.pool.begin().await.map_err(AppError::Database)?;

        // Verify recipe belongs to user
        let exists_row = sqlx::query_as::<_, (bool,)>(
            r#"
            SELECT EXISTS(SELECT 1 FROM recipes WHERE id = $1 AND user_id = $2)
            "#
        )
        .bind(recipe_id.as_uuid())
        .bind(user_id.as_uuid())
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
            "#
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
                "#
            )
            .bind(recipe_id.as_uuid())
            .bind(ingredient.catalog_ingredient_id().as_uuid())
            .bind(ingredient.quantity().value())
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
        // This test ensures RecipeRepositoryTrait is object-safe
        // (can be used as Arc<dyn RecipeRepositoryTrait>)
        let _: Option<Box<dyn RecipeRepositoryTrait>> = None;
    }
}
