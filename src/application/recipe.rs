use crate::domain::{
    Recipe, RecipeId, RecipeName, RecipeIngredient, RecipeCost, IngredientCost,
    RecipeType, Servings, CatalogIngredientId, Quantity, Money,
};
use crate::infrastructure::persistence::{
    RecipeRepositoryTrait, InventoryBatchRepositoryTrait, CatalogIngredientRepositoryTrait,
};
use crate::shared::{AppError, AppResult, UserId, TenantId, Language, PaginatedResponse, PaginationParams};
use std::collections::HashMap;
use std::sync::Arc;
use rust_decimal::prelude::ToPrimitive;

#[derive(Clone)]
pub struct RecipeService {
    recipe_repo: Arc<dyn RecipeRepositoryTrait>,
    inventory_repo: Arc<dyn InventoryBatchRepositoryTrait>,
    catalog_repo: Arc<dyn CatalogIngredientRepositoryTrait>,
}

impl RecipeService {
    pub fn new(
        recipe_repo: Arc<dyn RecipeRepositoryTrait>,
        inventory_repo: Arc<dyn InventoryBatchRepositoryTrait>,
        catalog_repo: Arc<dyn CatalogIngredientRepositoryTrait>,
    ) -> Self {
        Self {
            recipe_repo,
            inventory_repo,
            catalog_repo,
        }
    }

    /// Create a new recipe
    pub async fn create_recipe(
        &self,
        name: RecipeName,
        servings: Servings,
        ingredients: Vec<RecipeIngredient>,
        components: Vec<crate::domain::RecipeComponent>,
        user_id: UserId,
        tenant_id: TenantId,
    ) -> AppResult<Recipe> {
        // Validate that all ingredients exist in catalog
        for ingredient in &ingredients {
            let exists = self.catalog_repo
                .find_by_id(ingredient.catalog_ingredient_id())
                .await?
                .is_some();

            if !exists {
                return Err(AppError::NotFound(format!(
                    "Catalog ingredient {} not found",
                    ingredient.catalog_ingredient_id().as_uuid()
                )));
            }
        }

        let recipe = Recipe::new(
            user_id,
            tenant_id,
            name,
            RecipeType::Final,
            servings,
            ingredients,
            components,
            None,
        )?;

        self.recipe_repo.create(&recipe, user_id, tenant_id).await?;
        Ok(recipe)
    }

    /// 🔒 TENANT ISOLATION: Get recipe by ID within tenant
    pub async fn get_recipe(&self, id: RecipeId, tenant_id: TenantId) -> AppResult<Option<Recipe>> {
        self.recipe_repo.find_by_id(id, tenant_id).await
    }

    /// 🔒 TENANT ISOLATION: List all recipes for tenant (paginated)
    pub async fn list_recipes(
        &self,
        tenant_id: TenantId,
        pagination: &PaginationParams,
    ) -> AppResult<PaginatedResponse<Recipe>> {
        self.recipe_repo.list_by_tenant(tenant_id, pagination).await
    }

    /// 🔒 TENANT ISOLATION: Delete recipe within tenant
    pub async fn delete_recipe(&self, id: RecipeId, tenant_id: TenantId) -> AppResult<bool> {
        self.recipe_repo.delete(id, tenant_id).await
    }

    /// Recursively flattens a recipe into its base ingredients.
    /// Returns a map of CatalogIngredientId -> total quantity needed.
    pub async fn flatten_recipe(
        &self,
        recipe_id: RecipeId,
        tenant_id: TenantId,
        multiplier: rust_decimal::Decimal,
        depth: u32,
    ) -> AppResult<HashMap<CatalogIngredientId, rust_decimal::Decimal>> {
        if depth > 10 {
            return Err(AppError::validation("Max recipe depth exceeded (possible circular dependency)"));
        }

        let recipe = self.recipe_repo
            .find_by_id(recipe_id, tenant_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Recipe {} not found", recipe_id.as_uuid())))?;

        let mut flattened = HashMap::new();

        // Add direct ingredients
        for ingredient in recipe.ingredients() {
            let qty = ingredient.quantity().decimal() * multiplier;
            *flattened.entry(ingredient.catalog_ingredient_id()).or_insert(rust_decimal::Decimal::ZERO) += qty;
        }

        // Add component ingredients recursively
        for component in recipe.components() {
            // Need Box::pin for recursive call in async
            let component_flattened = Box::pin(self.flatten_recipe(
                component.component_recipe_id(),
                tenant_id,
                multiplier * component.quantity(),
                depth + 1,
            ))
            .await?;

            for (id, qty) in component_flattened {
                *flattened.entry(id).or_insert(rust_decimal::Decimal::ZERO) += qty;
            }
        }

        Ok(flattened)
    }

    /// 🔒 TENANT ISOLATION: Calculate recipe cost within tenant
    pub async fn calculate_cost(
        &self,
        recipe_id: RecipeId,
        tenant_id: TenantId,
    ) -> AppResult<RecipeCost> {
        // Load recipe
        let recipe = self.recipe_repo
            .find_by_id(recipe_id, tenant_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Recipe not found".to_string()))?;

        // Load all inventory products for this tenant to build price map
        let inventory_products = self.inventory_repo.list_by_tenant(recipe.tenant_id()).await?;

        // Build map: catalog_ingredient_id -> (quantity, price)
        let mut price_map: HashMap<CatalogIngredientId, (Quantity, Money)> = HashMap::new();

        for product in inventory_products {
            let key = product.catalog_ingredient_id;
            price_map.insert(
                key,
                (product.quantity, product.price_per_unit)
            );
        }

        // Flatten recipe to include all base ingredients from components
        let flattened_ingredients = self.flatten_recipe(recipe_id, tenant_id, rust_decimal::Decimal::ONE, 0).await?;

        // Calculate cost for each baseline ingredient
        let mut ingredients_breakdown = Vec::new();

        for (ingredient_id, total_qty_dec) in flattened_ingredients {
            // Get inventory price data
            let (_inventory_qty, inventory_price) = price_map
                .get(&ingredient_id)
                .ok_or_else(|| AppError::NotFound(format!(
                    "No inventory data for ingredient {}. Cannot calculate cost.",
                    ingredient_id.as_uuid()
                )))?;

            let unit_price_cents = inventory_price.as_cents();
            let unit_price = Money::from_cents(unit_price_cents)?;

            // Use rust_decimal for precise total cost calculation
            let item_total_cents = (total_qty_dec * rust_decimal::Decimal::from(unit_price_cents))
                .round()
                .to_i64()
                .unwrap_or(0);
            let ingredient_cost = Money::from_cents(item_total_cents)?;

            // Load ingredient name from catalog
            let catalog_ingredient = self.catalog_repo
                .find_by_id(ingredient_id)
                .await?
                .ok_or_else(|| AppError::NotFound(format!(
                    "Catalog ingredient {} not found",
                    ingredient_id.as_uuid()
                )))?;

            ingredients_breakdown.push(IngredientCost {
                ingredient_id,
                ingredient_name: catalog_ingredient.name(Language::En).to_string(),
                quantity: Quantity::from_decimal(total_qty_dec)?,
                unit_price,
                total_cost: ingredient_cost,
            });
        }

        // Build RecipeCost
        let recipe_cost = RecipeCost::new(
            recipe.id(),
            recipe.name().as_str().to_string(),
            ingredients_breakdown,
            recipe.servings().count()
        )?;

        Ok(recipe_cost)
    }

    /// 🔒 TENANT ISOLATION: Update recipe ingredients within tenant
    pub async fn update_ingredients(
        &self,
        recipe_id: RecipeId,
        ingredients: Vec<RecipeIngredient>,
        tenant_id: TenantId,
    ) -> AppResult<()> {
        // Validate that all ingredients exist in catalog
        for ingredient in &ingredients {
            let exists = self.catalog_repo
                .find_by_id(ingredient.catalog_ingredient_id())
                .await?
                .is_some();

            if !exists {
                return Err(AppError::NotFound(format!(
                    "Catalog ingredient {} not found",
                    ingredient.catalog_ingredient_id().as_uuid()
                )));
            }
        }

        self.recipe_repo.update_ingredients(recipe_id, ingredients, tenant_id).await
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_recipe_service_can_be_created() {
        // Ensures RecipeService compiles with Arc<dyn Trait>
    }
}
