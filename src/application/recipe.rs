use crate::domain::{
    Recipe, RecipeId, RecipeName, RecipeIngredient, RecipeCost, IngredientCost,
    RecipeType, Servings, CatalogIngredientId, Quantity, Money,
};
use crate::infrastructure::persistence::{
    RecipeRepositoryTrait, InventoryProductRepositoryTrait, CatalogIngredientRepositoryTrait,
};
use crate::shared::{AppError, AppResult, UserId, TenantId, Language};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct RecipeService {
    recipe_repo: Arc<dyn RecipeRepositoryTrait>,
    inventory_repo: Arc<dyn InventoryProductRepositoryTrait>,
    catalog_repo: Arc<dyn CatalogIngredientRepositoryTrait>,
}

impl RecipeService {
    pub fn new(
        recipe_repo: Arc<dyn RecipeRepositoryTrait>,
        inventory_repo: Arc<dyn InventoryProductRepositoryTrait>,
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
            RecipeType::Final, // Default to final recipe
            servings,
            ingredients,
            vec![], // No components yet
            None,   // No instructions yet
        )?;

        self.recipe_repo.create(&recipe, user_id, tenant_id).await?;
        Ok(recipe)
    }

    /// Get recipe by ID
    pub async fn get_recipe(&self, id: RecipeId, user_id: UserId) -> AppResult<Option<Recipe>> {
        self.recipe_repo.find_by_id(id, user_id).await
    }

    /// List all recipes for user
    pub async fn list_recipes(&self, user_id: UserId) -> AppResult<Vec<Recipe>> {
        self.recipe_repo.list_by_user(user_id).await
    }

    /// Delete recipe
    pub async fn delete_recipe(&self, id: RecipeId, user_id: UserId) -> AppResult<bool> {
        self.recipe_repo.delete(id, user_id).await
    }

    /// Calculate recipe cost based on current inventory prices
    /// 
    /// This method:
    /// 1. Loads the recipe with all ingredients
    /// 2. For each ingredient, finds the latest inventory product with that catalog_ingredient_id
    /// 3. Calculates ingredient cost = required_quantity * (inventory_unit_price / inventory_quantity)
    /// 4. Aggregates all costs into RecipeCost with breakdown
    /// 
    /// Returns AppError::NotFound if:
    /// - Recipe doesn't exist
    /// - Any ingredient has no inventory (no price data available)
    pub async fn calculate_cost(
        &self,
        recipe_id: RecipeId,
        user_id: UserId,
    ) -> AppResult<RecipeCost> {
        // Load recipe
        let recipe = self.recipe_repo
            .find_by_id(recipe_id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Recipe not found".to_string()))?;

        // Load all inventory products for this tenant to build price map
        let inventory_products = self.inventory_repo.list_by_tenant(recipe.tenant_id()).await?;

        // Build map: catalog_ingredient_id -> (quantity, price)
        // Use the latest added product for each ingredient
        let mut price_map: HashMap<CatalogIngredientId, (Quantity, Money)> = HashMap::new();
        
        for product in inventory_products {
            let key = product.catalog_ingredient_id;
            // Always update (last wins, assuming list_by_user returns newest first)
            price_map.insert(
                key,
                (product.quantity, product.price_per_unit)
            );
        }

        // Calculate cost for each ingredient
        let mut ingredients_breakdown = Vec::new();

        for recipe_ingredient in recipe.ingredients() {
            let ingredient_id = recipe_ingredient.catalog_ingredient_id();
            
            // Get inventory price data
            let (inventory_qty, inventory_price) = price_map
                .get(&ingredient_id)
                .ok_or_else(|| AppError::NotFound(format!(
                    "No inventory data for ingredient {}. Cannot calculate cost.",
                    ingredient_id.as_uuid()
                )))?;

            // Calculate unit price: inventory_price / inventory_quantity
            let unit_price_cents = (inventory_price.as_cents() as f64) / inventory_qty.value();
            let unit_price = Money::from_cents(unit_price_cents.round() as i64)?;
            
            // Calculate ingredient cost: required_quantity * unit_price
            let ingredient_cost_cents = recipe_ingredient.quantity().value() * unit_price_cents;
            let ingredient_cost = Money::from_cents(ingredient_cost_cents.round() as i64)?;

            // Load ingredient name from catalog for breakdown
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
                quantity: recipe_ingredient.quantity(),
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

    /// Update recipe ingredients
    pub async fn update_ingredients(
        &self,
        recipe_id: RecipeId,
        ingredients: Vec<RecipeIngredient>,
        user_id: UserId,
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

        self.recipe_repo.update_ingredients(recipe_id, ingredients, user_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recipe_service_can_be_created() {
        // This test ensures RecipeService compiles with Arc<dyn Trait>
        // Actual functionality requires database, so tested via integration tests
    }
}
