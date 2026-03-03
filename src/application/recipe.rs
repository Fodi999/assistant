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
            let exists = self
                .catalog_repo
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

        // Validate that all components exist
        for component in &components {
            let exists = self
                .recipe_repo
                .find_by_id(component.component_recipe_id, tenant_id)
                .await?
                .is_some();

            if !exists {
                return Err(AppError::NotFound(format!(
                    "Component recipe {} not found",
                    component.component_recipe_id.as_uuid()
                )));
            }
        }

        let recipe = Recipe::new(
            user_id,
            tenant_id,
            name,
            RecipeType::Final, // All recipes created via UI are Final by default
            servings,
            ingredients,
            components,
            None, // Instructions not provided via simple V1 API
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

        // Calculate cost for each ingredient
        let mut ingredients_breakdown = Vec::new();

        for recipe_ingredient in recipe.ingredients() {
            let ingredient_id = recipe_ingredient.catalog_ingredient_id();

            // Get inventory price data
            let (_inventory_qty, inventory_price) = price_map
                .get(&ingredient_id)
                .ok_or_else(|| AppError::NotFound(format!(
                    "No inventory data for ingredient {}. Cannot calculate cost.",
                    ingredient_id.as_uuid()
                )))?;

            let unit_price_cents = inventory_price.as_cents() as f64;
            let unit_price = Money::from_cents(unit_price_cents.round() as i64)?;

            let ingredient_cost_cents = recipe_ingredient.quantity().value() * unit_price_cents;
            let ingredient_cost = Money::from_cents(ingredient_cost_cents.round() as i64)?;

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
            vec![], // components_breakdown: component recipes not supported in V1
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
