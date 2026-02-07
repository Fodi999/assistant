use crate::domain::{inventory::Money, recipe::RecipeId};
use crate::shared::{AppError, AppResult, TenantId};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Dish ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DishId(Uuid);

impl DishId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

/// Dish name with validation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DishName(String);

impl DishName {
    pub fn new(name: impl Into<String>) -> AppResult<Self> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(AppError::validation("Dish name cannot be empty"));
        }
        if name.len() > 255 {
            return Err(AppError::validation("Dish name is too long (max 255 characters)"));
        }
        Ok(Self(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Dish - menu item with selling price
#[derive(Debug, Clone)]
pub struct Dish {
    pub id: DishId,
    pub tenant_id: TenantId,
    pub recipe_id: RecipeId,
    pub name: DishName,
    pub description: Option<String>,
    pub selling_price: Money,
    pub active: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl Dish {
    /// Create new dish
    pub fn new(
        tenant_id: TenantId,
        recipe_id: RecipeId,
        name: DishName,
        description: Option<String>,
        selling_price: Money,
    ) -> AppResult<Self> {
        if selling_price.as_cents() <= 0 {
            return Err(AppError::validation("Selling price must be greater than 0"));
        }

        let now = OffsetDateTime::now_utc();
        Ok(Self {
            id: DishId::new(),
            tenant_id,
            recipe_id,
            name,
            description,
            selling_price,
            active: true,
            created_at: now,
            updated_at: now,
        })
    }

    /// Reconstruct from database
    pub fn from_parts(
        id: DishId,
        tenant_id: TenantId,
        recipe_id: RecipeId,
        name: DishName,
        description: Option<String>,
        selling_price: Money,
        active: bool,
        created_at: OffsetDateTime,
        updated_at: OffsetDateTime,
    ) -> Self {
        Self {
            id,
            tenant_id,
            recipe_id,
            name,
            description,
            selling_price,
            active,
            created_at,
            updated_at,
        }
    }

    // Getters
    pub fn id(&self) -> DishId {
        self.id
    }

    pub fn tenant_id(&self) -> TenantId {
        self.tenant_id
    }

    pub fn recipe_id(&self) -> RecipeId {
        self.recipe_id
    }

    pub fn name(&self) -> &DishName {
        &self.name
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn selling_price(&self) -> Money {
        self.selling_price
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }

    pub fn updated_at(&self) -> OffsetDateTime {
        self.updated_at
    }

    // Setters
    pub fn update_name(&mut self, new_name: DishName) {
        self.name = new_name;
        self.updated_at = OffsetDateTime::now_utc();
    }

    pub fn update_selling_price(&mut self, new_price: Money) -> AppResult<()> {
        if new_price.as_cents() <= 0 {
            return Err(AppError::validation("Selling price must be greater than 0"));
        }
        self.selling_price = new_price;
        self.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }

    pub fn update_description(&mut self, new_description: Option<String>) {
        self.description = new_description;
        self.updated_at = OffsetDateTime::now_utc();
    }

    pub fn activate(&mut self) {
        self.active = true;
        self.updated_at = OffsetDateTime::now_utc();
    }

    pub fn deactivate(&mut self) {
        self.active = false;
        self.updated_at = OffsetDateTime::now_utc();
    }
}

/// Financial analysis of a dish
/// This is where "владелец ресторана видит деньги"!
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DishFinancials {
    pub dish_id: DishId,
    pub dish_name: String,
    pub selling_price_cents: i32,
    pub recipe_cost_cents: i32,
    pub profit_cents: i32,
    pub profit_margin_percent: f64,
    pub food_cost_percent: f64,
}

impl DishFinancials {
    pub fn calculate(
        dish_id: DishId,
        dish_name: String,
        selling_price: Money,
        recipe_cost: Money,
    ) -> Self {
        let selling_price_cents = selling_price.as_cents() as i32;
        let recipe_cost_cents = recipe_cost.as_cents() as i32;
        let profit_cents = selling_price_cents - recipe_cost_cents;
        
        let profit_margin_percent = if selling_price_cents > 0 {
            (profit_cents as f64 / selling_price_cents as f64) * 100.0
        } else {
            0.0
        };

        let food_cost_percent = if selling_price_cents > 0 {
            (recipe_cost_cents as f64 / selling_price_cents as f64) * 100.0
        } else {
            0.0
        };

        Self {
            dish_id,
            dish_name,
            selling_price_cents,
            recipe_cost_cents,
            profit_cents,
            profit_margin_percent,
            food_cost_percent,
        }
    }

    /// Check if profit margin is healthy (typically > 60% for restaurants)
    pub fn is_healthy_margin(&self) -> bool {
        self.profit_margin_percent >= 60.0
    }

    /// Check if food cost is acceptable (typically < 35% for restaurants)
    pub fn is_acceptable_food_cost(&self) -> bool {
        self.food_cost_percent <= 35.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dish_financials_calculation() {
        let dish_id = DishId::new();
        let selling_price = Money::from_cents(1500).unwrap(); // 15.00 PLN
        let recipe_cost = Money::from_cents(311).unwrap();    // 3.11 PLN (from recipe test)

        let financials = DishFinancials::calculate(
            dish_id,
            "Tomato Soup".to_string(),
            selling_price,
            recipe_cost,
        );

        assert_eq!(financials.selling_price_cents, 1500);
        assert_eq!(financials.recipe_cost_cents, 311);
        assert_eq!(financials.profit_cents, 1189); // 11.89 PLN profit
        assert_eq!(financials.profit_margin_percent, 79.26666666666667);
        assert_eq!(financials.food_cost_percent, 20.733333333333334);
        assert!(financials.is_healthy_margin());
        assert!(financials.is_acceptable_food_cost());
    }

    #[test]
    fn test_dish_name_validation() {
        assert!(DishName::new("Pizza").is_ok());
        assert!(DishName::new("").is_err());
        assert!(DishName::new("  ").is_err());
        assert!(DishName::new("a".repeat(256)).is_err());
    }

    #[test]
    fn test_dish_selling_price_validation() {
        let result = Dish::new(
            TenantId::new(),
            RecipeId::new(),
            DishName::new("Test").unwrap(),
            None,
            Money::from_cents(0).unwrap(),
        );
        assert!(result.is_err());
    }
}
