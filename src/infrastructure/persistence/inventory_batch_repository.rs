use crate::domain::{
    catalog::CatalogIngredientId,
    inventory::{InventoryBatch, InventoryBatchId, Money, Quantity, BatchStatus, MovementType, InventoryMovement},
};
use crate::shared::{AppError, AppResult, TenantId, UserId};
use async_trait::async_trait;
use sqlx::{PgPool, Row, Postgres, Transaction};
use time::OffsetDateTime;
use rust_decimal::Decimal;

#[async_trait]
pub trait InventoryBatchRepositoryTrait: Send + Sync {
    /// Add new batch to inventory
    async fn create(&self, batch: &InventoryBatch) -> AppResult<()>;

    /// Add new batch within a transaction
    async fn create_in_transaction(&self, tx: &mut Transaction<'static, Postgres>, batch: &InventoryBatch) -> AppResult<()>;
    
    /// Find batch by ID
    async fn find_by_id(&self, id: InventoryBatchId, tenant_id: TenantId) -> AppResult<Option<InventoryBatch>>;
    
    /// List all batches for tenant
    async fn list_by_tenant(&self, tenant_id: TenantId) -> AppResult<Vec<InventoryBatch>>;
    
    /// Count batches for tenant
    async fn count_by_tenant(&self, tenant_id: TenantId) -> AppResult<i64>;

    /// List active batches for specific ingredient with LOCK (FIFO order)
    async fn list_active_by_ingredient_for_update(&self, tx: &mut Transaction<'static, Postgres>, tenant_id: TenantId, catalog_id: CatalogIngredientId) -> AppResult<Vec<InventoryBatch>>;

    /// Update batch quantity and status (simple)
    async fn update(&self, batch: &InventoryBatch) -> AppResult<()>;

    /// Update batch quantity and status within a transaction
    async fn update_in_transaction(&self, tx: &mut Transaction<'static, Postgres>, batch: &InventoryBatch) -> AppResult<()>;
    
    /// Delete batch from inventory
    async fn delete(&self, id: InventoryBatchId, tenant_id: TenantId) -> AppResult<()>;

    /// Record a movement (audit log)
    async fn record_movement(&self, tx: &mut Transaction<'static, Postgres>, movement: &InventoryMovement) -> AppResult<()>;
}

#[derive(Clone)]
pub struct InventoryBatchRepository {
    pool: PgPool,
}

impl InventoryBatchRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn row_to_batch(row: &sqlx::postgres::PgRow) -> AppResult<InventoryBatch> {
        let id: uuid::Uuid = row.try_get("id").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?;
        let user_id: uuid::Uuid = row.try_get("user_id").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?;
        let tenant_id: uuid::Uuid = row.try_get("tenant_id").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?;
        let catalog_ingredient_id: uuid::Uuid = row.try_get("catalog_ingredient_id").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?;
        let price_cents: i64 = row.try_get("price_per_unit_cents").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?;
        let quantity: Decimal = row.try_get("quantity").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?;
        let remaining_quantity: Decimal = row.try_get("remaining_quantity").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?;
        let supplier: Option<String> = row.try_get("supplier").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?;
        let invoice: Option<String> = row.try_get("invoice_number").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?;
        let status_str: String = row.try_get("status").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?;
        let received_at: OffsetDateTime = row.try_get("received_at").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?;
        let expires_at: OffsetDateTime = row.try_get("expires_at").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?;
        let created_at: OffsetDateTime = row.try_get("created_at").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?;
        let updated_at: OffsetDateTime = row.try_get("updated_at").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?;

        let status = match status_str.as_str() {
            "active" => BatchStatus::Active,
            "exhausted" => BatchStatus::Exhausted,
            "archived" => BatchStatus::Archived,
            _ => BatchStatus::Active,
        };

        Ok(InventoryBatch::from_parts(
            InventoryBatchId::from_uuid(id),
            UserId::from_uuid(user_id),
            TenantId::from_uuid(tenant_id),
            CatalogIngredientId::from_uuid(catalog_ingredient_id),
            Money::from_cents(price_cents)?,
            Quantity::from_decimal(quantity)?,
            Quantity::from_decimal(remaining_quantity)?,
            supplier,
            invoice,
            status,
            received_at,
            expires_at,
            created_at,
            updated_at,
        ))
    }
}

// Simple type alias for backward compatibility
pub type InventoryProductRepository = InventoryBatchRepository;

#[async_trait]
impl InventoryBatchRepositoryTrait for InventoryBatchRepository {
    async fn create(&self, batch: &InventoryBatch) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO inventory_batches 
                (id, user_id, tenant_id, catalog_ingredient_id, price_per_unit_cents, 
                 quantity, remaining_quantity, supplier, invoice_number, status,
                 received_at, expires_at, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            "#
        )
        .bind(batch.id.as_uuid())
        .bind(batch.user_id.as_uuid())
        .bind(batch.tenant_id.as_uuid())
        .bind(batch.catalog_ingredient_id.as_uuid())
        .bind(batch.price_per_unit.as_cents())
        .bind(batch.quantity.decimal())
        .bind(batch.remaining_quantity.decimal())
        .bind(&batch.supplier)
        .bind(&batch.invoice_number)
        .bind(match batch.status {
            BatchStatus::Active => "active",
            BatchStatus::Exhausted => "exhausted",
            BatchStatus::Archived => "archived",
        })
        .bind(batch.received_at)
        .bind(batch.expires_at)
        .bind(batch.created_at)
        .bind(batch.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn find_by_id(&self, id: InventoryBatchId, tenant_id: TenantId) -> AppResult<Option<InventoryBatch>> {
        let row = sqlx::query(
            r#"
            SELECT id, user_id, tenant_id, catalog_ingredient_id, price_per_unit_cents, 
                   quantity, remaining_quantity, supplier, invoice_number, status,
                   received_at, expires_at, created_at, updated_at
            FROM inventory_batches
            WHERE id = $1 AND tenant_id = $2
            "#
        )
        .bind(id.as_uuid())
        .bind(tenant_id.as_uuid())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(ref r) => Ok(Some(Self::row_to_batch(r)?)),
            None => Ok(None),
        }
    }

    async fn list_by_tenant(&self, tenant_id: TenantId) -> AppResult<Vec<InventoryBatch>> {
        let rows = sqlx::query(
            r#"
            SELECT id, user_id, tenant_id, catalog_ingredient_id, price_per_unit_cents, 
                   quantity, remaining_quantity, supplier, invoice_number, status,
                   received_at, expires_at, created_at, updated_at
            FROM inventory_batches
            WHERE tenant_id = $1
            ORDER BY received_at DESC
            "#
        )
        .bind(tenant_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;

        let mut batches = Vec::with_capacity(rows.len());
        for row in rows {
            batches.push(Self::row_to_batch(&row)?);
        }

        Ok(batches)
    }

    async fn count_by_tenant(&self, tenant_id: TenantId) -> AppResult<i64> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM inventory_batches WHERE tenant_id = $1")
            .bind(tenant_id.as_uuid())
            .fetch_one(&self.pool)
            .await?;
        Ok(count)
    }

    async fn list_active_by_ingredient_for_update(
        &self,
        tx: &mut Transaction<'static, Postgres>,
        tenant_id: TenantId,
        catalog_id: CatalogIngredientId,
    ) -> AppResult<Vec<InventoryBatch>> {
        let rows = sqlx::query(
            r#"
            SELECT id, user_id, tenant_id, catalog_ingredient_id, price_per_unit_cents, 
                   quantity, remaining_quantity, supplier, invoice_number, status,
                   received_at, expires_at, created_at, updated_at
            FROM inventory_batches
            WHERE tenant_id = $1 AND catalog_ingredient_id = $2 AND status = 'active' AND remaining_quantity > 0
            ORDER BY expires_at NULLS LAST, received_at ASC
            FOR UPDATE
            "#
        )
        .bind(tenant_id.as_uuid())
        .bind(catalog_id.as_uuid())
        .fetch_all(&mut **tx)
        .await?;

        let mut batches = Vec::with_capacity(rows.len());
        for row in rows {
            batches.push(Self::row_to_batch(&row)?);
        }

        Ok(batches)
    }

    async fn update(&self, batch: &InventoryBatch) -> AppResult<()> {
        sqlx::query(
            r#"
            UPDATE inventory_batches 
            SET price_per_unit_cents = $1, quantity = $2, remaining_quantity = $3, 
                status = $4, expires_at = $5, updated_at = $6
            WHERE id = $7 AND tenant_id = $8
            "#
        )
        .bind(batch.price_per_unit.as_cents())
        .bind(batch.quantity.decimal())
        .bind(batch.remaining_quantity.decimal())
        .bind(match batch.status {
            BatchStatus::Active => "active",
            BatchStatus::Exhausted => "exhausted",
            BatchStatus::Archived => "archived",
        })
        .bind(batch.expires_at)
        .bind(batch.updated_at)
        .bind(batch.id.as_uuid())
        .bind(batch.tenant_id.as_uuid())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn update_in_transaction(&self, tx: &mut Transaction<'static, Postgres>, batch: &InventoryBatch) -> AppResult<()> {
        sqlx::query(
            r#"
            UPDATE inventory_batches 
            SET price_per_unit_cents = $1, quantity = $2, remaining_quantity = $3, 
                status = $4, expires_at = $5, updated_at = $6
            WHERE id = $7 AND tenant_id = $8
            "#
        )
        .bind(batch.price_per_unit.as_cents())
        .bind(batch.quantity.decimal())
        .bind(batch.remaining_quantity.decimal())
        .bind(match batch.status {
            BatchStatus::Active => "active",
            BatchStatus::Exhausted => "exhausted",
            BatchStatus::Archived => "archived",
        })
        .bind(batch.expires_at)
        .bind(batch.updated_at)
        .bind(batch.id.as_uuid())
        .bind(batch.tenant_id.as_uuid())
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    async fn delete(&self, id: InventoryBatchId, tenant_id: TenantId) -> AppResult<()> {
        sqlx::query("DELETE FROM inventory_batches WHERE id = $1 AND tenant_id = $2")
            .bind(id.as_uuid())
            .bind(tenant_id.as_uuid())
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn record_movement(&self, tx: &mut Transaction<'static, Postgres>, movement: &InventoryMovement) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO inventory_movements 
                (id, tenant_id, batch_id, type, quantity, unit_cost_cents, total_cost_cents, 
                 reference_id, reference_type, reason, notes, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#
        )
        .bind(movement.id)
        .bind(movement.tenant_id.as_uuid())
        .bind(movement.batch_id.as_uuid())
        .bind(match movement.movement_type {
            MovementType::In => "IN",
            MovementType::OutSale => "OUT_SALE",
            MovementType::OutExpire => "OUT_EXPIRE",
            MovementType::Adjustment => "ADJUSTMENT",
        })
        .bind(movement.quantity)
        .bind(movement.unit_cost_cents)
        .bind(movement.total_cost_cents)
        .bind(movement.reference_id)
        .bind(&movement.reference_type)
        .bind(&movement.reason)
        .bind(&movement.notes)
        .bind(movement.created_at)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    async fn create_in_transaction(&self, tx: &mut Transaction<'static, Postgres>, batch: &InventoryBatch) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO inventory_batches 
                (id, user_id, tenant_id, catalog_ingredient_id, price_per_unit_cents, 
                 quantity, remaining_quantity, supplier, invoice_number, status,
                 received_at, expires_at, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            "#
        )
        .bind(batch.id.as_uuid())
        .bind(batch.user_id.as_uuid())
        .bind(batch.tenant_id.as_uuid())
        .bind(batch.catalog_ingredient_id.as_uuid())
        .bind(batch.price_per_unit.as_cents())
        .bind(batch.quantity.decimal())
        .bind(batch.remaining_quantity.decimal())
        .bind(&batch.supplier)
        .bind(&batch.invoice_number)
        .bind(match batch.status {
            BatchStatus::Active => "active",
            BatchStatus::Exhausted => "exhausted",
            BatchStatus::Archived => "archived",
        })
        .bind(batch.received_at)
        .bind(batch.expires_at)
        .bind(batch.created_at)
        .bind(batch.updated_at)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }
}
