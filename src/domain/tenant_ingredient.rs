use crate::domain::catalog::{CatalogIngredientId, Unit};
use crate::shared::AppResult;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Tenant ID (from JWT claims)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TenantId(Uuid);

impl TenantId {
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

impl Default for TenantId {
    fn default() -> Self {
        Self::new()
    }
}

/// Tenant Ingredient ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TenantIngredientId(Uuid);

impl TenantIngredientId {
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

impl Default for TenantIngredientId {
    fn default() -> Self {
        Self::new()
    }
}

/// Tenant-specific ingredient data
/// Links master catalog ingredient to tenant with custom price, supplier, etc.
#[derive(Debug, Clone)]
pub struct TenantIngredient {
    pub id: TenantIngredientId,
    pub tenant_id: TenantId,
    pub catalog_ingredient_id: CatalogIngredientId,
    
    // Tenant-specific fields
    pub price: Option<Decimal>,
    pub supplier: Option<String>,
    pub custom_unit: Option<Unit>,
    pub custom_expiration_days: Option<i32>,
    pub notes: Option<String>,
    
    pub is_active: bool,
}

impl TenantIngredient {
    /// Create new tenant ingredient
    pub fn new(
        tenant_id: TenantId,
        catalog_ingredient_id: CatalogIngredientId,
        price: Option<Decimal>,
        supplier: Option<String>,
    ) -> Self {
        Self {
            id: TenantIngredientId::new(),
            tenant_id,
            catalog_ingredient_id,
            price,
            supplier,
            custom_unit: None,
            custom_expiration_days: None,
            notes: None,
            is_active: true,
        }
    }

    /// Reconstruct from database
    pub fn from_parts(
        id: TenantIngredientId,
        tenant_id: TenantId,
        catalog_ingredient_id: CatalogIngredientId,
        price: Option<Decimal>,
        supplier: Option<String>,
        custom_unit: Option<Unit>,
        custom_expiration_days: Option<i32>,
        notes: Option<String>,
        is_active: bool,
    ) -> Self {
        Self {
            id,
            tenant_id,
            catalog_ingredient_id,
            price,
            supplier,
            custom_unit,
            custom_expiration_days,
            notes,
            is_active,
        }
    }

    /// Get effective unit (custom or default from catalog)
    pub fn effective_unit<'a>(&'a self, catalog_unit: &'a Unit) -> &'a Unit {
        self.custom_unit.as_ref().unwrap_or(catalog_unit)
    }

    /// Get effective expiration days (custom or default from catalog)
    pub fn effective_expiration_days(&self, catalog_days: Option<i32>) -> Option<i32> {
        self.custom_expiration_days.or(catalog_days)
    }

    /// Update price
    pub fn set_price(&mut self, price: Option<Decimal>) {
        self.price = price;
    }

    /// Update supplier
    pub fn set_supplier(&mut self, supplier: Option<String>) {
        self.supplier = supplier;
    }

    /// Update notes
    pub fn set_notes(&mut self, notes: Option<String>) {
        self.notes = notes;
    }

    /// Soft delete
    pub fn deactivate(&mut self) {
        self.is_active = false;
    }
}
