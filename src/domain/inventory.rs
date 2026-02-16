use crate::domain::catalog::CatalogIngredientId;
use crate::shared::{AppError, AppResult, TenantId, UserId};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

/// Inventory batch ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InventoryBatchId(Uuid);

impl InventoryBatchId {
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

impl Default for InventoryBatchId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for InventoryBatchId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Helper for backward compatibility or direct ID usage
pub type InventoryProductId = InventoryBatchId;

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
    pub fn multiply(&self, quantity: Decimal) -> AppResult<Money> {
        if quantity < Decimal::ZERO {
            return Err(AppError::validation("Quantity cannot be negative"));
        }
        
        // Convert money to decimal for precise calculation
        let money_dec = Decimal::from(self.0);
        let result_dec = money_dec * quantity;
        
        // Round to nearest integer (cents)
        let result = result_dec.round().to_i64()
            .ok_or_else(|| AppError::validation("Money calculation overflow"))?;
            
        Ok(Money(result))
    }
    
    /// Legacy multiply for f64 (internal use or migration)
    pub fn multiply_f64(&self, quantity: f64) -> AppResult<Money> {
        if quantity < 0.0 {
            return Err(AppError::validation("Quantity cannot be negative"));
        }
        let result = (self.0 as f64 * quantity).round() as i64;
        Ok(Money(result))
    }
}

/// Quantity with unit
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Quantity(Decimal);

impl Quantity {
    pub fn new(value: f64) -> AppResult<Self> {
        if value < 0.0 {
            return Err(AppError::validation("Quantity cannot be negative"));
        }
        let mut dec = Decimal::from_f64_retain(value)
            .ok_or_else(|| AppError::validation("Invalid quantity value"))?;
        
        // Round to 12 decimal places to eliminate f64 conversion noise
        dec = dec.round_dp(12);
        
        Ok(Self(dec))
    }
    
    pub fn from_decimal(value: Decimal) -> AppResult<Self> {
        if value < Decimal::ZERO {
            return Err(AppError::validation("Quantity cannot be negative"));
        }
        Ok(Self(value))
    }

    pub fn value(&self) -> f64 {
        self.0.to_f64().unwrap_or(0.0)
    }
    
    pub fn decimal(&self) -> Decimal {
        self.0
    }

    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
}

/// Batch status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BatchStatus {
    Active,
    Exhausted,
    Archived,
}

impl Default for BatchStatus {
    fn default() -> Self {
        Self::Active
    }
}

/// Inventory batch - a concrete delivery of a catalog ingredient
#[derive(Debug, Clone)]
pub struct InventoryBatch {
    pub id: InventoryBatchId,
    pub user_id: UserId,
    pub tenant_id: TenantId,
    
    /// Reference to catalog ingredient
    pub catalog_ingredient_id: CatalogIngredientId,
    
    /// Purchase price per unit (in smallest currency unit)
    pub price_per_unit: Money,
    
    /// Original quantity purchased
    pub quantity: Quantity,
    
    /// Current remaining quantity
    pub remaining_quantity: Quantity,
    
    /// Supplier information
    pub supplier: Option<String>,
    
    /// Invoice/Document reference
    pub invoice_number: Option<String>,
    
    /// Batch status
    pub status: BatchStatus,
    
    /// Product receipt/purchase date (дата поступления)
    pub received_at: OffsetDateTime,
    
    /// Expiration date (дата просрочки, optional)
    pub expires_at: Option<OffsetDateTime>,
    
    /// Timestamps
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// Backward compatibility
pub type InventoryProduct = InventoryBatch;

impl InventoryBatch {
    /// Create new inventory batch
    pub fn new(
        user_id: UserId,
        tenant_id: TenantId,
        catalog_ingredient_id: CatalogIngredientId,
        price_per_unit: Money,
        quantity: Quantity,
        received_at: OffsetDateTime,
        expires_at: Option<OffsetDateTime>,
    ) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            id: InventoryBatchId::new(),
            user_id,
            tenant_id,
            catalog_ingredient_id,
            price_per_unit,
            quantity,
            remaining_quantity: quantity,
            supplier: None,
            invoice_number: None,
            status: BatchStatus::Active,
            received_at,
            expires_at,
            created_at: now,
            updated_at: now,
        }
    }

    /// Reconstruct from database
    pub fn from_parts(
        id: InventoryBatchId,
        user_id: UserId,
        tenant_id: TenantId,
        catalog_ingredient_id: CatalogIngredientId,
        price_per_unit: Money,
        quantity: Quantity,
        remaining_quantity: Quantity,
        supplier: Option<String>,
        invoice_number: Option<String>,
        status: BatchStatus,
        received_at: OffsetDateTime,
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
            remaining_quantity,
            supplier,
            invoice_number,
            status,
            received_at,
            expires_at,
            created_at,
            updated_at,
        }
    }

    /// Calculate total cost of original batch
    pub fn total_cost(&self) -> AppResult<Money> {
        self.price_per_unit.multiply(self.quantity.decimal())
    }
    
    /// Calculate current value of remaining stock in this batch
    pub fn current_value(&self) -> AppResult<Money> {
        self.price_per_unit.multiply(self.remaining_quantity.decimal())
    }

    /// Check if product is expired
    pub fn is_expired(&self) -> bool {
        self.expiration_status() == ExpirationSeverity::Expired
    }

    /// Update expiration status calculation to match new severity rules
    pub fn expiration_status(&self) -> ExpirationSeverity {
        calculate_expiration_status(self.expires_at, OffsetDateTime::now_utc())
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

/// Movement type (IN/OUT_SALE/OUT_EXPIRE/ADJUSTMENT)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MovementType {
    In,
    OutSale,
    OutExpire,
    Adjustment,
}

impl std::fmt::Display for MovementType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::In => write!(f, "IN"),
            Self::OutSale => write!(f, "OUT_SALE"),
            Self::OutExpire => write!(f, "OUT_EXPIRE"),
            Self::Adjustment => write!(f, "ADJUSTMENT"),
        }
    }
}

/// Alert types for inventory
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InventoryAlertType {
    ExpiringBatch,
    LowStock,
}

/// Alert severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertSeverity {
    Expired,
    Critical,
    Warning,
    Info,
}

/// Inventory alert DTO
#[derive(Debug, Clone, Serialize)]
pub struct InventoryAlert {
    pub alert_type: InventoryAlertType,
    pub severity: AlertSeverity,
    pub ingredient_id: Uuid,
    pub ingredient_name: String,
    pub batch_id: Option<Uuid>,
    pub message: String,
    pub current_value: f64,
    pub threshold_value: Option<f64>,
}

/// Inventory movement record (Audit Log)
#[derive(Debug, Clone, Serialize)]
pub struct InventoryMovement {
    pub id: Uuid,
    pub tenant_id: TenantId,
    pub batch_id: InventoryBatchId,
    pub movement_type: MovementType,
    pub quantity: Decimal,
    pub unit_cost_cents: i64,
    pub total_cost_cents: i64,
    pub reference_id: Option<Uuid>,
    pub reference_type: Option<String>,
    pub reason: Option<String>,
    pub notes: Option<String>,
    pub created_at: OffsetDateTime,
}

impl InventoryMovement {
    pub fn new(
        tenant_id: TenantId,
        batch_id: InventoryBatchId,
        movement_type: MovementType,
        quantity: Decimal,
        unit_cost_cents: i64,
    ) -> Self {
        let total_cost = (quantity * Decimal::from(unit_cost_cents)).round().to_i64().unwrap_or(0);
        
        Self {
            id: Uuid::new_v4(),
            tenant_id,
            batch_id,
            movement_type,
            quantity,
            unit_cost_cents,
            total_cost_cents: total_cost,
            reference_id: None,
            reference_type: None,
            reason: None,
            notes: None,
            created_at: OffsetDateTime::now_utc(),
        }
    }
}

/// Expiration severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExpirationSeverity {
    Expired,
    Critical,   // 0-1 days
    Warning,    // 2-3 days
    Ok,
    NoExpiration,
}

/// Logical calculation for expiration severity
pub fn calculate_expiration_status(
    expires_at: Option<OffsetDateTime>,
    now: OffsetDateTime,
) -> ExpirationSeverity {
    match expires_at {
        None => ExpirationSeverity::NoExpiration,
        Some(date) => {
            if date < now {
                ExpirationSeverity::Expired
            } else {
                let diff = date - now;
                let days_left = diff.whole_days();

                if days_left <= 1 {
                    ExpirationSeverity::Critical
                } else if days_left <= 3 {
                    ExpirationSeverity::Warning
                } else {
                    ExpirationSeverity::Ok
                }
            }
        }
    }
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
            OffsetDateTime::now_utc(), // received_at
            None,
        );
        
        let total = product.total_cost().unwrap();
        assert_eq!(total.as_major(), 25.0);
    }
}
