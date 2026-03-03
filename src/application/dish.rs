use crate::application::RecipeService;
use crate::domain::{Dish, DishFinancials, DishId, DishName, Money, RecipeId};
use crate::infrastructure::persistence::DishRepositoryTrait;
use crate::shared::{AppError, AppResult, PaginationParams, TenantId};
use std::sync::Arc;

#[derive(Clone)]
pub struct DishService {
    dish_repo: Arc<dyn DishRepositoryTrait>,
    recipe_service: RecipeService,
}

impl DishService {
    pub fn new(dish_repo: Arc<dyn DishRepositoryTrait>, recipe_service: RecipeService) -> Self {
        Self {
            dish_repo,
            recipe_service,
        }
    }

    /// Create new dish — materializes cost immediately if recipe cost is available
    pub async fn create_dish(
        &self,
        tenant_id: TenantId,
        recipe_id: RecipeId,
        name: DishName,
        description: Option<String>,
        selling_price: Money,
        image_url: Option<String>,
    ) -> AppResult<Dish> {
        let mut dish = Dish::new(tenant_id, recipe_id, name, description, selling_price, image_url)?;

        // Try to materialize cost at creation time
        match self.recipe_service.calculate_cost(recipe_id, tenant_id).await {
            Ok(recipe_cost) => {
                let cost = Money::from_cents(recipe_cost.cost_per_serving.as_cents())?;
                dish.recalculate_cost(cost);
                tracing::info!(
                    "💰 Dish '{}' cost materialized: recipe={} food_cost={:.1}% margin={:.1}%",
                    dish.name().as_str(),
                    recipe_cost.cost_per_serving.as_cents(),
                    dish.food_cost_percent().unwrap_or(0.0),
                    dish.profit_margin_percent().unwrap_or(0.0),
                );
            }
            Err(e) => {
                tracing::warn!(
                    "⚠️ Could not materialize cost for dish '{}': {}. Will be calculated later.",
                    dish.name().as_str(),
                    e
                );
            }
        }

        self.dish_repo.create(&dish).await?;
        Ok(dish)
    }

    /// Get dish by ID
    pub async fn get_dish(&self, id: DishId, tenant_id: TenantId) -> AppResult<Option<Dish>> {
        self.dish_repo.find_by_id(id, tenant_id).await
    }

    /// List dishes for tenant (paginated)
    pub async fn list_dishes(
        &self,
        tenant_id: TenantId,
        active_only: bool,
        pagination: &PaginationParams,
    ) -> AppResult<(Vec<Dish>, i64)> {
        self.dish_repo
            .list_by_tenant(tenant_id, active_only, pagination)
            .await
    }

    /// Calculate dish financials (on-the-fly, does NOT persist)
    /// 🔒 TENANT ISOLATION: uses tenant_id for recipe cost lookup
    pub async fn calculate_financials(
        &self,
        dish_id: DishId,
        tenant_id: TenantId,
    ) -> AppResult<DishFinancials> {
        let dish = self
            .get_dish(dish_id, tenant_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Dish not found".to_string()))?;

        let recipe_cost = self
            .recipe_service
            .calculate_cost(dish.recipe_id(), tenant_id)
            .await?;

        let financials = DishFinancials::calculate(
            dish.id(),
            dish.name().as_str().to_string(),
            dish.selling_price(),
            Money::from_cents(recipe_cost.cost_per_serving.as_cents())?,
        );

        Ok(financials)
    }

    /// Recalculate materialized costs for ALL dishes of a tenant.
    /// Called when ingredient prices change or manually by owner.
    /// Returns (updated_count, error_count).
    pub async fn recalculate_all_costs(
        &self,
        tenant_id: TenantId,
    ) -> AppResult<RecalculateResult> {
        // Load all dishes (no pagination — we need all of them)
        let all_pagination = PaginationParams {
            page: Some(1),
            per_page: Some(100),
        };

        let mut updated = 0u32;
        let mut errors = 0u32;
        let mut page = 1u32;

        loop {
            let pagination = PaginationParams {
                page: Some(page),
                per_page: Some(all_pagination.per_page()),
            };

            let (dishes, total) = self
                .dish_repo
                .list_by_tenant(tenant_id, false, &pagination)
                .await?;

            if dishes.is_empty() {
                break;
            }

            for dish in dishes {
                match self
                    .recipe_service
                    .calculate_cost(dish.recipe_id(), tenant_id)
                    .await
                {
                    Ok(recipe_cost) => {
                        let cost = Money::from_cents(recipe_cost.cost_per_serving.as_cents())?;
                        let mut dish = dish;
                        dish.recalculate_cost(cost);
                        self.dish_repo.update(&dish).await?;
                        updated += 1;
                    }
                    Err(e) => {
                        tracing::warn!(
                            "⚠️ Failed to recalculate cost for dish '{}' ({}): {}",
                            dish.name().as_str(),
                            dish.id().as_uuid(),
                            e
                        );
                        errors += 1;
                    }
                }
            }

            if (page as i64 * all_pagination.per_page() as i64) >= total {
                break;
            }
            page += 1;
        }

        tracing::info!(
            "✅ Cost recalculation complete: {} updated, {} errors",
            updated,
            errors
        );

        Ok(RecalculateResult { updated, errors })
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
        let mut dish = self
            .get_dish(id, tenant_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Dish not found".to_string()))?;

        if let Some(new_name) = name {
            dish.update_name(new_name);
        }

        if let Some(new_description) = description {
            dish.update_description(new_description);
        }

        let price_changed = selling_price.is_some();
        if let Some(new_price) = selling_price {
            dish.update_selling_price(new_price)?;
        }

        if let Some(true) = active {
            dish.activate();
        } else if let Some(false) = active {
            dish.deactivate();
        }

        // Re-materialize cost if selling price changed
        if price_changed {
            if let Ok(recipe_cost) = self
                .recipe_service
                .calculate_cost(dish.recipe_id(), tenant_id)
                .await
            {
                let cost = Money::from_cents(recipe_cost.cost_per_serving.as_cents())?;
                dish.recalculate_cost(cost);
            }
        }

        self.dish_repo.update(&dish).await?;
        Ok(dish)
    }

    /// Delete dish
    pub async fn delete_dish(&self, id: DishId, tenant_id: TenantId) -> AppResult<bool> {
        self.dish_repo.delete(id, tenant_id).await
    }
}

/// Result of batch recalculation
#[derive(Debug, Clone, serde::Serialize)]
pub struct RecalculateResult {
    pub updated: u32,
    pub errors: u32,
}

#[cfg(test)]
mod tests {
    // TODO: Add integration tests with mock repositories
}
