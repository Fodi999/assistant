use crate::domain::catalog::CatalogIngredientId;
use crate::shared::{AppError, AppResult, TenantId, UserId};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Inventory product ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InventoryProductId(Uuid);

impl InventoryProductId {
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

impl Default for InventoryProductId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for InventoryProductId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Money amount in smallest currency unit (e.g., cents for USD/EUR, grosze for PLN)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Money(i64);

impl Money {
    /// Create money from smallest unit (e.g., cents)
    pub fn from_cents(cents: i64) -> AppResult<Self> {
        if cents < 0 {
            return Err(AppError::validation("Money amount cannot be negative"));
        }
        Ok(Self(cents))
    }

    /// Create money from major unit (e.g., dollars) with 2 decimal places
    pub fn from_major(amount: f64) -> AppResult<Self> {
        if amount < 0.0 {
            return Err(AppError::validation("Money amount cannot be negative"));
        }
        let cents = (amount * 100.0).round() as i64;
        Ok(Self(cents))
    }

    pub fn as_cents(&self) -> i64 {
        self.0
    }

    pub fn as_major(&self) -> f64 {
        self.0 as f64 / 100.0
    }

    /// Add two money amounts
    pub fn add(&self, other: Money) -> AppResult<Money> {
        self.0
            .checked_add(other.0)
            .map(Money)
            .ok_or_else(|| AppError::validation("Money overflow"))
    }

    /// Multiply money by quantity
    pub fn multiply(&self, quantity: f64) -> AppResult<Money> {
        if quantity < 0.0 {
            return Err(AppError::validation("Quantity cannot be negative"));
        }
        let result = (self.0 as f64 * quantity).round() as i64;
        Ok(Money(result))
    }
}

/// Quantity with unit
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Quantity(f64);

impl Quantity {
    pub fn new(value: f64) -> AppResult<Self> {
        if value < 0.0 {
            return Err(AppError::validation("Quantity cannot be negative"));
        }
        if !value.is_finite() {
            return Err(AppError::validation("Quantity must be finite"));
        }
        Ok(Self(value))
    }

    pub fn value(&self) -> f64 {
        self.0
    }

    pub fn is_zero(&self) -> bool {
        self.0 == 0.0
    }
}

/// Inventory product - a concrete instance of a catalog ingredient purchased by user
#[derive(Debug, Clone)]
pub struct InventoryProduct {
    pub id: InventoryProductId,
    pub user_id: UserId,
    pub tenant_id: TenantId,
    
    /// Reference to catalog ingredient
    pub catalog_ingredient_id: CatalogIngredientId,
    
    /// Purchase price per unit (in smallest currency unit)
    pub price_per_unit: Money,
    
    /// Quantity purchased
    pub quantity: Quantity,
    
    /// Expiration date (optional)
    pub expires_at: Option<OffsetDateTime>,
    
    /// Timestamps
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl InventoryProduct {
    /// Create new inventory product
    pub fn new(
        user_id: UserId,
        tenant_id: TenantId,
        catalog_ingredient_id: CatalogIngredientId,
        price_per_unit: Money,
        quantity: Quantity,
        expires_at: Option<OffsetDateTime>,
    ) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            id: InventoryProductId::new(),
            user_id,
            tenant_id,
            catalog_ingredient_id,
            price_per_unit,
            quantity,
            expires_at,
            created_at: now,
            updated_at: now,
        }
    }

    /// Reconstruct from database
    pub fn from_parts(
        id: InventoryProductId,
        user_id: UserId,
        tenant_id: TenantId,
        catalog_ingredient_id: CatalogIngredientId,
        price_per_unit: Money,
        quantity: Quantity,
        expires_at: Option<OffsetDateTime>,
        created_at: OffsetDateTime,
        updated_at: OffsetDateTime,
    ) -> Self {
        Self {
            id,
            user_id,
            tenant_id,
            catalog_ingredient_id,
            price_per_unit,
            quantity,
            expires_at,
            created_at,
            updated_at,
        }
    }

    /// Calculate total cost (price_per_unit * quantity)
    pub fn total_cost(&self) -> AppResult<Money> {
        self.price_per_unit.multiply(self.quantity.value())
    }

    /// Check if product is expired (date is in the past)
    pub fn is_expired(&self) -> bool {
        self.expiration_status() == ExpirationStatus::Expired
    }

    /// Get detailed expiration status
    pub fn expiration_status(&self) -> ExpirationStatus {
        if let Some(expires_at) = self.expires_at {
            let today = OffsetDateTime::now_utc().date();
            let expiry_date = expires_at.date();
            
            if expiry_date < today {
                ExpirationStatus::Expired
            } else if expiry_date == today {
                ExpirationStatus::ExpiresToday
            } else if expiry_date <= today + time::Duration::days(2) {
                ExpirationStatus::ExpiringSoon
            } else {
                ExpirationStatus::Fresh
            }
        } else {
            // No expiration date = never expires
            ExpirationStatus::NoExpiration
        }
    }

    /// Update quantity
    pub fn update_quantity(&mut self, new_quantity: Quantity) {
        self.quantity = new_quantity;
        self.updated_at = OffsetDateTime::now_utc();
    }

    /// Update price
    pub fn update_price(&mut self, new_price: Money) {
        self.price_per_unit = new_price;
        self.updated_at = OffsetDateTime::now_utc();
    }
}

/// Expiration status of inventory product
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpirationStatus {
    /// Product is expired (date < today)
    Expired,
    /// Product expires today
    ExpiresToday,
    /// Product expires within 2 days
    ExpiringSoon,
    /// Product is fresh (expires in 3+ days)
    Fresh,
    /// No expiration date set
    NoExpiration,
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_money_from_cents() {
        let money = Money::from_cents(1500).unwrap();
        assert_eq!(money.as_cents(), 1500);
        assert_eq!(money.as_major(), 15.0);
    }

    #[test]
    fn test_money_from_major() {
        let money = Money::from_major(15.99).unwrap();
        assert_eq!(money.as_cents(), 1599);
        assert_eq!(money.as_major(), 15.99);
    }

    #[test]
    fn test_money_negative_rejected() {
        assert!(Money::from_cents(-100).is_err());
        assert!(Money::from_major(-10.0).is_err());
    }

    #[test]
    fn test_quantity_validation() {
        assert!(Quantity::new(10.5).is_ok());
        assert!(Quantity::new(0.0).is_ok());
        assert!(Quantity::new(-1.0).is_err());
        assert!(Quantity::new(f64::INFINITY).is_err());
    }

    #[test]
    fn test_total_cost() {
        let product = InventoryProduct::new(
            UserId::new(),
            TenantId::new(),
            CatalogIngredientId::new(),
            Money::from_major(10.0).unwrap(),
            Quantity::new(2.5).unwrap(),
            None,
        );
        
        let total = product.total_cost().unwrap();
        assert_eq!(total.as_major(), 25.0);
    }
}
