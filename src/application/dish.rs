use crate::domain::{Dish, DishId, DishName, DishFinancials, RecipeId, Money};
use crate::infrastructure::persistence::DishRepositoryTrait;
use crate::application::RecipeService;
use crate::shared::{AppError, AppResult, TenantId, UserId};
use std::sync::Arc;

#[derive(Clone)]
pub struct DishService {
    dish_repo: Arc<dyn DishRepositoryTrait>,
    recipe_service: RecipeService,
}

impl DishService {
    pub fn new(
        dish_repo: Arc<dyn DishRepositoryTrait>,
        recipe_service: RecipeService,
    ) -> Self {
        Self {
            dish_repo,
            recipe_service,
        }
    }

    /// Create new dish
    /// Only 'final' recipes can be dishes (DDD validation)
    pub async fn create_dish(
        &self,
        tenant_id: TenantId,
        recipe_id: RecipeId,
        name: DishName,
        description: Option<String>,
        selling_price: Money,
    ) -> AppResult<Dish> {
        // TODO: Validate that recipe exists and is 'final' type
        // This requires RecipeService API to accept tenant_id or return recipe type
        // For now, we trust the caller to provide valid recipe_id
        
        let dish = Dish::new(tenant_id, recipe_id, name, description, selling_price)?;
        self.dish_repo.create(&dish).await?;
        Ok(dish)
    }

    /// Get dish by ID
    pub async fn get_dish(&self, id: DishId, tenant_id: TenantId) -> AppResult<Option<Dish>> {
        self.dish_repo.find_by_id(id, tenant_id).await
    }

    /// List dishes for tenant
    pub async fn list_dishes(&self, tenant_id: TenantId, active_only: bool) -> AppResult<Vec<Dish>> {
        self.dish_repo.list_by_tenant(tenant_id, active_only).await
    }

    /// Calculate dish financials
    /// This is where "владелец ресторана видит деньги"!
    pub async fn calculate_financials(
        &self,
        dish_id: DishId,
        user_id: UserId,
        tenant_id: TenantId,
    ) -> AppResult<DishFinancials> {
        // Get dish
        let dish = self.get_dish(dish_id, tenant_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Dish not found".to_string()))?;

        // Calculate recipe cost (recursive, handles components)
        let recipe_cost = self.recipe_service
            .calculate_cost(dish.recipe_id(), user_id)
            .await?;

        // Calculate financials
        let financials = DishFinancials::calculate(
            dish.id(),
            dish.name().as_str().to_string(),
            dish.selling_price(),
            Money::from_cents(recipe_cost.total_cost.as_cents())?,
        );

        Ok(financials)
    }

    /// Update dish
    pub async fn update_dish(
        &self,
        id: DishId,
        tenant_id: TenantId,
        name: Option<DishName>,
        description: Option<Option<String>>,
        selling_price: Option<Money>,
        active: Option<bool>,
    ) -> AppResult<Dish> {
        let mut dish = self.get_dish(id, tenant_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Dish not found".to_string()))?;

        if let Some(new_name) = name {
            dish.update_name(new_name);
        }

        if let Some(new_description) = description {
            dish.update_description(new_description);
        }

        if let Some(new_price) = selling_price {
            dish.update_selling_price(new_price)?;
        }

        if let Some(true) = active {
            dish.activate();
        } else if let Some(false) = active {
            dish.deactivate();
        }

        self.dish_repo.update(&dish).await?;
        Ok(dish)
    }

    /// Delete dish
    pub async fn delete_dish(&self, id: DishId, tenant_id: TenantId) -> AppResult<bool> {
        self.dish_repo.delete(id, tenant_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: Add integration tests with mock repositories
}
