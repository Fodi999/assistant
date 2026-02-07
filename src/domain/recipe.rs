use crate::domain::{
    catalog::CatalogIngredientId,
    inventory::{Money, Quantity},
};
use crate::shared::{AppError, AppResult, TenantId, UserId};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Recipe ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RecipeId(Uuid);

impl RecipeId {
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

/// Recipe name with validation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeName(String);

impl RecipeName {
    pub fn new(name: impl Into<String>) -> AppResult<Self> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(AppError::validation("Recipe name cannot be empty"));
        }
        if name.len() > 255 {
            return Err(AppError::validation("Recipe name is too long (max 255 characters)"));
        }
        Ok(Self(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

/// Servings count with validation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Servings(u32);

impl Servings {
    pub fn new(count: u32) -> AppResult<Self> {
        if count == 0 {
            return Err(AppError::validation("Servings must be greater than 0"));
        }
        if count > 1000 {
            return Err(AppError::validation("Servings count is too large (max 1000)"));
        }
        Ok(Self(count))
    }

    pub fn count(&self) -> u32 {
        self.0
    }
}

/// Ingredient in a recipe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeIngredient {
    pub catalog_ingredient_id: CatalogIngredientId,
    pub quantity: Quantity,
}

impl RecipeIngredient {
    pub fn new(catalog_ingredient_id: CatalogIngredientId, quantity: Quantity) -> Self {
        Self {
            catalog_ingredient_id,
            quantity,
        }
    }

    pub fn catalog_ingredient_id(&self) -> CatalogIngredientId {
        self.catalog_ingredient_id
    }

    pub fn quantity(&self) -> Quantity {
        self.quantity
    }
}

/// Recipe - a formula for preparing a dish
#[derive(Debug, Clone)]
pub struct Recipe {
    pub id: RecipeId,
    pub user_id: UserId,
    pub tenant_id: TenantId,
    pub name: RecipeName,
    pub servings: Servings,
    pub ingredients: Vec<RecipeIngredient>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl Recipe {
    /// Create new recipe
    pub fn new(
        user_id: UserId,
        tenant_id: TenantId,
        name: RecipeName,
        servings: Servings,
        ingredients: Vec<RecipeIngredient>,
    ) -> AppResult<Self> {
        if ingredients.is_empty() {
            return Err(AppError::validation("Recipe must have at least one ingredient"));
        }

        let now = OffsetDateTime::now_utc();
        Ok(Self {
            id: RecipeId::new(),
            user_id,
            tenant_id,
            name,
            servings,
            ingredients,
            created_at: now,
            updated_at: now,
        })
    }

    /// Reconstruct from database
    pub fn from_parts(
        id: RecipeId,
        user_id: UserId,
        tenant_id: TenantId,
        name: RecipeName,
        servings: Servings,
        ingredients: Vec<RecipeIngredient>,
        created_at: OffsetDateTime,
        updated_at: OffsetDateTime,
    ) -> Self {
        Self {
            id,
            user_id,
            tenant_id,
            name,
            servings,
            ingredients,
            created_at,
            updated_at,
        }
    }

    // Getters
    pub fn id(&self) -> RecipeId {
        self.id
    }

    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    pub fn tenant_id(&self) -> TenantId {
        self.tenant_id
    }

    pub fn name(&self) -> &RecipeName {
        &self.name
    }

    pub fn servings(&self) -> Servings {
        self.servings
    }

    pub fn ingredients(&self) -> &[RecipeIngredient] {
        &self.ingredients
    }

    pub fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }

    pub fn updated_at(&self) -> OffsetDateTime {
        self.updated_at
    }

    /// Update recipe name
    pub fn update_name(&mut self, new_name: RecipeName) {
        self.name = new_name;
        self.updated_at = OffsetDateTime::now_utc();
    }

    /// Update servings count
    pub fn update_servings(&mut self, new_servings: Servings) {
        self.servings = new_servings;
        self.updated_at = OffsetDateTime::now_utc();
    }

    /// Update ingredients
    pub fn update_ingredients(&mut self, new_ingredients: Vec<RecipeIngredient>) -> AppResult<()> {
        if new_ingredients.is_empty() {
            return Err(AppError::validation("Recipe must have at least one ingredient"));
        }
        self.ingredients = new_ingredients;
        self.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }
}

/// Cost breakdown for a single ingredient in recipe
#[derive(Debug, Clone, Serialize)]
pub struct IngredientCost {
    pub ingredient_id: CatalogIngredientId,
    pub ingredient_name: String,
    pub quantity: Quantity,
    pub unit_price: Money,
    pub total_cost: Money,
}

/// Total recipe cost calculation
#[derive(Debug, Clone, Serialize)]
pub struct RecipeCost {
    pub recipe_id: RecipeId,
    pub recipe_name: String,
    pub total_cost: Money,
    pub cost_per_serving: Money,
    pub ingredients_breakdown: Vec<IngredientCost>,
}

impl RecipeCost {
    pub fn new(
        recipe_id: RecipeId,
        recipe_name: String,
        ingredients_breakdown: Vec<IngredientCost>,
        servings: u32,
    ) -> AppResult<Self> {
        let mut total_cents = 0i64;
        for ingredient in &ingredients_breakdown {
            total_cents += ingredient.total_cost.as_cents();
        }

        let total_cost = Money::from_cents(total_cents)?;
        let cost_per_serving = Money::from_cents(total_cents / servings as i64)?;

        Ok(Self {
            recipe_id,
            recipe_name,
            total_cost,
            cost_per_serving,
            ingredients_breakdown,
        })
    }

    /// Calculate food cost percentage
    pub fn food_cost_percentage(&self, selling_price: Money) -> f64 {
        if selling_price.as_cents() == 0 {
            return 0.0;
        }
        (self.cost_per_serving.as_cents() as f64 / selling_price.as_cents() as f64) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recipe_name_validation() {
        assert!(RecipeName::new("").is_err());
        assert!(RecipeName::new("   ").is_err());
        assert!(RecipeName::new("Valid Name").is_ok());
        
        let long_name = "a".repeat(256);
        assert!(RecipeName::new(long_name).is_err());
    }

    #[test]
    fn test_servings_validation() {
        assert!(Servings::new(0).is_err());
        assert!(Servings::new(1).is_ok());
        assert!(Servings::new(4).is_ok());
        assert!(Servings::new(1001).is_err());
    }

    #[test]
    fn test_recipe_cost_calculation() {
        let ingredient1 = IngredientCost {
            ingredient_id: CatalogIngredientId::from_uuid(Uuid::new_v4()),
            ingredient_name: "Milk".to_string(),
            quantity: Quantity::new(1.0).unwrap(),
            unit_price: Money::from_cents(450).unwrap(),
            total_cost: Money::from_cents(450).unwrap(),
        };

        let ingredient2 = IngredientCost {
            ingredient_id: CatalogIngredientId::from_uuid(Uuid::new_v4()),
            ingredient_name: "Eggs".to_string(),
            quantity: Quantity::new(3.0).unwrap(),
            unit_price: Money::from_cents(50).unwrap(),
            total_cost: Money::from_cents(150).unwrap(),
        };

        let recipe_cost = RecipeCost::new(
            RecipeId::new(),
            "Pancakes".to_string(),
            vec![ingredient1, ingredient2],
            4, // 4 servings
        ).unwrap();

        assert_eq!(recipe_cost.total_cost.as_cents(), 600); // 450 + 150
        assert_eq!(recipe_cost.cost_per_serving.as_cents(), 150); // 600 / 4
    }

    #[test]
    fn test_food_cost_percentage() {
        let recipe_cost = RecipeCost::new(
            RecipeId::new(),
            "Test".to_string(),
            vec![IngredientCost {
                ingredient_id: CatalogIngredientId::from_uuid(Uuid::new_v4()),
                ingredient_name: "Test".to_string(),
                quantity: Quantity::new(1.0).unwrap(),
                unit_price: Money::from_cents(300).unwrap(),
                total_cost: Money::from_cents(300).unwrap(),
            }],
            1,
        ).unwrap();

        // Cost: 3.00 PLN, Selling: 10.00 PLN => 30% food cost
        let selling_price = Money::from_cents(1000).unwrap();
        let food_cost_pct = recipe_cost.food_cost_percentage(selling_price);
        assert!((food_cost_pct - 30.0).abs() < 0.01);
    }
}
