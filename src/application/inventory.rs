use crate::domain::{
    catalog::CatalogIngredientId,
    inventory::{
        BatchStatus, ExpirationSeverity, InventoryBatch, InventoryBatchId, InventoryMovement,
        Money, MovementType, Quantity, calculate_expiration_status,
    },
};
use crate::infrastructure::persistence::{
    InventoryBatchRepository, InventoryBatchRepositoryTrait,
};
use crate::shared::{AppError, AppResult, Language, TenantId, UserId};
use serde::Serialize;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use time::OffsetDateTime;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

/// Aggregated inventory status for dashboard
#[derive(Debug, Clone, Serialize)]
pub struct InventoryStatus {
    pub health_score: i32,
    pub status: String, // e.g. "Excellent", "Warning", "Critical"
    pub critical: usize,
    pub warning: usize,
    pub expired: usize,
    pub low_stock: usize,
    /// Total count for the red badge (expired + critical)
    pub badge_count: usize,
}

#[derive(Debug, Serialize)]
pub struct InventoryDashboard {
    pub total_stock_value_cents: i64,
    pub waste_30d_cents: i64,
    pub waste_percentage: f64,
    pub health_score: i32,
    pub stockout_risks: Vec<StockoutPrediction>,
    pub expired_risks: Vec<RiskProduct>,
}

#[derive(Debug, Serialize)]
pub struct StockoutPrediction {
    pub ingredient_id: uuid::Uuid,
    pub name: String,
    pub current_quantity: f64,
    pub avg_daily_consumption: f64,
    pub days_until_stockout: f64,
}

#[derive(Debug, Serialize)]
pub struct RiskProduct {
    pub ingredient_id: uuid::Uuid,
    pub name: String,
    pub status: String,
    pub batch_id: uuid::Uuid,
    pub remaining_quantity: f64,
}

impl Default for InventoryStatus {
    fn default() -> Self {
        Self {
            health_score: 100,
            status: "Excellent".to_string(),
            critical: 0,
            warning: 0,
            expired: 0,
            low_stock: 0,
            badge_count: 0,
        }
    }
}

/// Aggregated stock summary for a specific ingredient
#[derive(Debug, Clone, Serialize)]
pub struct StockSummary {
    pub ingredient_id: uuid::Uuid,
    pub name: String,
    pub total_quantity: f64,
    pub avg_price_cents: i64,
}

#[derive(Clone)]
pub struct InventoryService {
    inventory_repo: Arc<InventoryBatchRepository>,
    pool: PgPool,
    alert_service: crate::application::InventoryAlertService,
}

impl InventoryService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            inventory_repo: Arc::new(InventoryBatchRepository::new(pool.clone())),
            alert_service: crate::application::InventoryAlertService::new(pool.clone()),
            pool,
        }
    }

    /// Comprehensive Dashboard for the Owner
    pub async fn get_dashboard(&self, tenant_id: TenantId) -> AppResult<InventoryDashboard> {
        // 1. Get current stock value (cents)
        let total_stock_value_cents: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(remaining_quantity * price_per_unit_cents), 0)::BIGINT 
             FROM inventory_batches 
             WHERE tenant_id = $1 AND status = 'active'"
        )
        .bind(tenant_id.as_uuid())
        .fetch_one(&self.pool)
        .await?;

        // 2. Get Health Score
        let health = self.alert_service.get_inventory_status(tenant_id).await?;

        // 3. Get Waste Info (30 days)
        let loss_report = self.get_loss_report(tenant_id, 30).await?;

        // 4. Calculate Stockout Predictions
        let stockout_risks = self.calculate_stockout_predictions(tenant_id).await?;

        // 5. Identify Risk Products
        let expired_risks = self.identify_risk_products(tenant_id).await?;

        Ok(InventoryDashboard {
            total_stock_value_cents,
            waste_30d_cents: loss_report.total_loss_cents,
            waste_percentage: loss_report.waste_percentage,
            health_score: health.health_score,
            stockout_risks,
            expired_risks,
        })
    }

    async fn calculate_stockout_predictions(&self, tenant_id: TenantId) -> AppResult<Vec<StockoutPrediction>> {
        let query = r#"
            WITH consumption AS (
                SELECT 
                    ib.catalog_ingredient_id,
                    SUM(im.quantity) as total_consumed
                FROM inventory_movements im
                JOIN inventory_batches ib ON im.batch_id = ib.id
                WHERE ib.tenant_id = $1 
                  AND im.type = 'OUT_SALE'
                  AND im.created_at > NOW() - INTERVAL '14 days'
                GROUP BY ib.catalog_ingredient_id
            ),
            current_stock AS (
                SELECT 
                    ib.catalog_ingredient_id,
                    ci.name_en as ingredient_name,
                    SUM(ib.remaining_quantity) as total_qty
                FROM inventory_batches ib
                JOIN catalog_ingredients ci ON ib.catalog_ingredient_id = ci.id
                WHERE ib.tenant_id = $1 AND ib.status = 'active'
                GROUP BY ib.catalog_ingredient_id, ci.name_en
            )
            SELECT 
                cs.catalog_ingredient_id,
                cs.ingredient_name,
                cs.total_qty,
                COALESCE(c.total_consumed / 14.0, 0) as avg_daily
            FROM current_stock cs
            LEFT JOIN consumption c ON cs.catalog_ingredient_id = c.catalog_ingredient_id
            WHERE cs.total_qty > 0
            ORDER BY (cs.total_qty / NULLIF(COALESCE(c.total_consumed / 14.0, 0), 0)) ASC
            LIMIT 5
        "#;

        let rows = sqlx::query(query)
            .bind(tenant_id.as_uuid())
            .fetch_all(&self.pool)
            .await?;

        let mut results = Vec::new();
        for row in rows {
            let total_qty: Decimal = row.get("total_qty");
            let avg_daily: Decimal = row.get("avg_daily");
            
            let days_until = if avg_daily > Decimal::ZERO {
                (total_qty / avg_daily).to_f64().unwrap_or(f64::INFINITY)
            } else {
                f64::INFINITY
            };

            results.push(StockoutPrediction {
                ingredient_id: row.get("catalog_ingredient_id"),
                name: row.get("ingredient_name"),
                current_quantity: total_qty.to_f64().unwrap_or(0.0),
                avg_daily_consumption: avg_daily.to_f64().unwrap_or(0.0),
                days_until_stockout: days_until,
            });
        }

        Ok(results)
    }

    async fn identify_risk_products(&self, tenant_id: TenantId) -> AppResult<Vec<RiskProduct>> {
        let query = r#"
            SELECT 
                ib.id as batch_id,
                ib.catalog_ingredient_id,
                ci.name_en as ingredient_name,
                ib.remaining_quantity,
                CASE 
                    WHEN ib.expires_at < NOW() THEN 'Expired'
                    WHEN ib.expires_at < NOW() + INTERVAL '1 day' THEN 'Critical'
                    ELSE 'Warning'
                END as risk_status
            FROM inventory_batches ib
            JOIN catalog_ingredients ci ON ib.catalog_ingredient_id = ci.id
            WHERE ib.tenant_id = $1 
              AND ib.status = 'active'
              AND ib.expires_at < NOW() + INTERVAL '3 days'
              AND ib.remaining_quantity > 0
            ORDER BY ib.expires_at ASC
            LIMIT 10
        "#;

        let rows = sqlx::query(query)
            .bind(tenant_id.as_uuid())
            .fetch_all(&self.pool)
            .await?;

        let mut results = Vec::new();
        for row in rows {
            results.push(RiskProduct {
                ingredient_id: row.get("catalog_ingredient_id"),
                name: row.get("ingredient_name"),
                status: row.get("risk_status"),
                batch_id: row.get("batch_id"),
                remaining_quantity: row.get::<Decimal, _>("remaining_quantity").to_f64().unwrap_or(0.0),
            });
        }

        Ok(results)
    }

    /// Add batch to inventory (пополнение склада)
    pub async fn add_batch(
        &self,
        user_id: UserId,
        tenant_id: TenantId,
        catalog_ingredient_id: CatalogIngredientId,
        price_per_unit_cents: i64,
        quantity: f64,
        supplier: Option<String>,
        invoice_number: Option<String>,
        received_at: OffsetDateTime,
        expires_at: OffsetDateTime,
    ) -> AppResult<InventoryBatchId> {
        let price = Money::from_cents(price_per_unit_cents)?;
        let qty = Quantity::new(quantity)?;

        let mut batch = InventoryBatch::new(
            user_id,
            tenant_id,
            catalog_ingredient_id,
            price,
            qty,
            received_at,
            expires_at,
        );
        
        batch.supplier = supplier;
        batch.invoice_number = invoice_number;

        let batch_id = batch.id;

        // Use transaction to ensure both batch and movement are created
        let mut tx = self.pool.begin().await?;
        
        self.inventory_repo.create_in_transaction(&mut tx, &batch).await?;

        // 🎯 Record IN movement for audit
        let mut movement = InventoryMovement::new(
            tenant_id,
            batch_id,
            MovementType::In,
            qty.decimal(),
            price.as_cents(),
        );
        movement.notes = batch.invoice_number.clone();
        movement.reason = Some("Purchase/Inbound shipment".to_string());
        movement.reference_type = Some("purchase".to_string());

        self.inventory_repo.record_movement(&mut tx, &movement).await?;

        tx.commit().await?;

        Ok(batch_id)
    }

    /// Legacy method alias
    pub async fn add_product(
        &self,
        user_id: UserId,
        tenant_id: TenantId,
        catalog_ingredient_id: CatalogIngredientId,
        price_per_unit_cents: i64,
        quantity: f64,
        received_at: OffsetDateTime,
        expires_at: OffsetDateTime,
    ) -> AppResult<InventoryBatchId> {
        self.add_batch(user_id, tenant_id, catalog_ingredient_id, price_per_unit_cents, quantity, None, None, received_at, expires_at).await
    }

    /// Get stock summary (runtime calculation from batches)
    pub async fn get_stock_summary(&self, tenant_id: TenantId) -> AppResult<Vec<StockSummary>> {
        let query = r#"
            SELECT 
                ib.catalog_ingredient_id,
                ci.name_en as ingredient_name,
                SUM(ib.remaining_quantity) as total_quantity,
                CASE 
                    WHEN SUM(ib.remaining_quantity) > 0 
                    THEN SUM(ib.remaining_quantity * ib.price_per_unit_cents) / SUM(ib.remaining_quantity)
                    ELSE 0 
                END as avg_price_cents
            FROM inventory_batches ib
            JOIN catalog_ingredients ci ON ib.catalog_ingredient_id = ci.id
            WHERE ib.tenant_id = $1 AND ib.status = 'active'
            GROUP BY ib.catalog_ingredient_id, ci.name_en
            HAVING SUM(ib.remaining_quantity) > 0
        "#;

        let rows = sqlx::query(query)
            .bind(tenant_id.as_uuid())
            .fetch_all(&self.pool)
            .await?;

        let mut summaries = Vec::new();
        for row in rows {
            summaries.push(StockSummary {
                ingredient_id: row.try_get("catalog_ingredient_id").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?,
                name: row.try_get("ingredient_name").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?,
                total_quantity: row.try_get::<Decimal, _>("total_quantity").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?.to_f64().unwrap_or(0.0),
                avg_price_cents: row.try_get::<Decimal, _>("avg_price_cents").map_err(|e| AppError::internal(&format!("DB Error: {}", e)))?.to_i64().unwrap_or(0),
            });
        }

        Ok(summaries)
    }

    /// Get tenant's inventory list (all batches)
    pub async fn list_products(
        &self,
        _user_id: UserId,
        tenant_id: TenantId,
    ) -> AppResult<Vec<InventoryBatch>> {
        self.inventory_repo.list_by_tenant(tenant_id).await
    }

    /// Update batch/product details
    pub async fn update_product(
        &self,
        batch_id: InventoryBatchId,
        _user_id: UserId,
        tenant_id: TenantId,
        price_per_unit_cents: Option<i64>,
        quantity: Option<f64>,
    ) -> AppResult<()> {
        let mut batch = self
            .inventory_repo
            .find_by_id(batch_id, tenant_id)
            .await?
            .ok_or_else(|| AppError::not_found("Batch not found"))?;

        if let Some(price_cents) = price_per_unit_cents {
            batch.price_per_unit = Money::from_cents(price_cents)?;
        }

        if let Some(qty) = quantity {
            let q = Quantity::new(qty)?;
            batch.remaining_quantity = q;
            batch.quantity = q; // Adjust total if this is a manual corrections
        }

        self.inventory_repo.update(&batch).await?;
        Ok(())
    }

    /// Delete batch from inventory
    pub async fn delete_product(
        &self,
        batch_id: InventoryBatchId,
        _user_id: UserId,
        tenant_id: TenantId,
    ) -> AppResult<()> {
        self.inventory_repo.delete(batch_id, tenant_id).await
    }

    /// Remove product from inventory (legacy alias for delete)
    pub async fn remove_product(
        &self,
        user_id: UserId,
        tenant_id: TenantId,
        product_id: InventoryBatchId,
    ) -> AppResult<()> {
        self.delete_product(product_id, user_id, tenant_id).await
    }

    /// Get specific product/batch detail
    pub async fn get_product(
        &self,
        _user_id: UserId,
        tenant_id: TenantId,
        product_id: InventoryBatchId,
    ) -> AppResult<Option<InventoryBatch>> {
        self.inventory_repo.find_by_id(product_id, tenant_id).await
    }

    /// Check if tenant has any products in inventory
    pub async fn has_products(&self, _user_id: UserId, tenant_id: TenantId) -> AppResult<bool> {
        let count = self.inventory_repo.count_by_tenant(tenant_id).await?;
        Ok(count > 0)
    }

    /// Count products in inventory
    pub async fn count_products(&self, _user_id: UserId, tenant_id: TenantId) -> AppResult<i64> {
        self.inventory_repo.count_by_tenant(tenant_id).await
    }

    /// Get expiring batches report
    pub async fn get_expiring_products(
        &self,
        _user_id: UserId,
        tenant_id: TenantId,
        days_ahead: i64,
    ) -> AppResult<Vec<InventoryBatch>> {
        let batches = self.inventory_repo.list_by_tenant(tenant_id).await?;
        let now = OffsetDateTime::now_utc();
        let limit = now + time::Duration::days(days_ahead);

        Ok(batches
            .into_iter()
            .filter(|b| {
                b.expires_at <= limit && b.status == BatchStatus::Active
            })
            .collect())
    }

    /// Fetch inventory views with dynamic status/severity calculation
    pub async fn list_products_with_details(
        &self,
        _user_id: UserId,
        tenant_id: TenantId,
        language: Language,
    ) -> AppResult<Vec<InventoryView>> {
        let lang_code = language.code();

        // 🎯 FIX: Use base table columns for translations (name_en, name_ru, name_pl, name_uk)
        // catalog_ingredient_translations table is NOT used - all translations are in catalog_ingredients
        let query = r#"
            SELECT 
                ip.id,
                ip.catalog_ingredient_id,
                CASE 
                    WHEN $2 = 'ru' THEN COALESCE(ci.name_ru, ci.name_en, 'Unknown')
                    WHEN $2 = 'pl' THEN COALESCE(ci.name_pl, ci.name_en, 'Unknown')
                    WHEN $2 = 'uk' THEN COALESCE(ci.name_uk, ci.name_en, 'Unknown')
                    ELSE COALESCE(ci.name_en, 'Unknown')
                END as ingredient_name,
                COALESCE(cct_user.name, cct_en.name, 'Unknown') as category_name,
                ci.default_unit::TEXT as base_unit,
                ci.image_url,
                ci.min_stock_threshold,
                ip.quantity,
                ip.remaining_quantity,
                ip.price_per_unit_cents,
                ip.received_at,
                ip.expires_at,
                ip.created_at,
                ip.updated_at
            FROM inventory_batches ip
            INNER JOIN catalog_ingredients ci 
                ON ip.catalog_ingredient_id = ci.id
            LEFT JOIN catalog_categories cc 
                ON ci.category_id = cc.id
            LEFT JOIN catalog_category_translations cct_user 
                ON cct_user.category_id = cc.id AND cct_user.language = $2
            LEFT JOIN catalog_category_translations cct_en 
                ON cct_en.category_id = cc.id AND cct_en.language = 'en'
            WHERE ip.tenant_id = $1
            ORDER BY ip.received_at DESC
        "#;

        let rows = sqlx::query(query)
            .bind(tenant_id.as_uuid())
            .bind(lang_code)  // user.language ('en'|'pl'|'uk'|'ru')
            .fetch_all(&self.pool)
            .await?;

        let mut views = Vec::new();
        let now = OffsetDateTime::now_utc();
        
        for row in rows {
            let expires_at: OffsetDateTime = row.try_get("expires_at").map_err(|e| AppError::internal(&format!("DB: {}", e)))?;
            let status = calculate_expiration_status(Some(expires_at), now);

            views.push(InventoryView {
                id: row.try_get("id").map_err(|e| AppError::internal(&format!("DB: {}", e)))?,
                product: ProductInfo {
                    id: row.try_get("catalog_ingredient_id").map_err(|e| AppError::internal(&format!("DB: {}", e)))?,
                    name: row.try_get("ingredient_name").map_err(|e| AppError::internal(&format!("DB: {}", e)))?,
                    category: row.try_get("category_name").map_err(|e| AppError::internal(&format!("DB: {}", e)))?,
                    base_unit: row.try_get("base_unit").map_err(|e| AppError::internal(&format!("DB: {}", e)))?,
                    image_url: row.try_get("image_url").map_err(|e| AppError::internal(&format!("DB: {}", e)))?,
                    min_stock_threshold: row.try_get::<Decimal, _>("min_stock_threshold")
                        .map_err(|e| AppError::internal(&format!("DB: {}", e)))?
                        .to_f64()
                        .unwrap_or(0.0),
                },
                quantity: row.try_get::<Decimal, _>("quantity").map_err(|e| AppError::internal(&format!("DB Qty Error: {}", e)))?.to_f64().unwrap_or(0.0),
                remaining_quantity: row.try_get::<Decimal, _>("remaining_quantity").map_err(|e| AppError::internal(&format!("DB Rem Error: {}", e)))?.to_f64().unwrap_or(0.0),
                price_per_unit_cents: row.try_get("price_per_unit_cents").map_err(|e| AppError::internal(&format!("DB: {}", e)))?,
                severity: status,
                received_at: row.try_get("received_at").map_err(|e| AppError::internal(&format!("DB: {}", e)))?,
                expires_at,
                created_at: row.try_get("created_at").map_err(|e| AppError::internal(&format!("DB: {}", e)))?,
                updated_at: row.try_get("updated_at").map_err(|e| AppError::internal(&format!("DB: {}", e)))?,
            });
        }

        Ok(views)
    }

    /// Deduct quantity from inventory using FIFO (списание со склада)
    pub async fn deduct_fifo(
        &self,
        tenant_id: TenantId,
        catalog_ingredient_id: CatalogIngredientId,
        quantity_to_deduct: f64,
        reference_id: Option<uuid::Uuid>,
        reference_type: Option<String>,
        notes: Option<String>,
    ) -> AppResult<()> {
        let target_qty = Quantity::new(quantity_to_deduct)?.decimal();

        if target_qty <= Decimal::ZERO {
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;

        // 1. Get active batches with FOR UPDATE lock
        let batches = self.inventory_repo
            .list_active_by_ingredient_for_update(&mut tx, tenant_id, catalog_ingredient_id)
            .await?;

        let mut remaining_to_deduct = target_qty;

        for mut batch in batches {
            if remaining_to_deduct <= Decimal::ZERO {
                break;
            }

            let batch_available = batch.remaining_quantity.decimal();
            let deduction = if batch_available <= remaining_to_deduct {
                batch_available
            } else {
                remaining_to_deduct
            };

            // 2. Update batch
            let new_remaining = batch_available - deduction;
            batch.remaining_quantity = Quantity::from_decimal(new_remaining)?;
            
            if new_remaining <= Decimal::ZERO {
                batch.status = BatchStatus::Exhausted;
            }

            self.inventory_repo.update_in_transaction(&mut tx, &batch).await?;

            // 3. Record movement (OutSale)
            let mut movement = InventoryMovement::new(
                tenant_id,
                batch.id,
                MovementType::OutSale,
                deduction,
                batch.price_per_unit.as_cents(),
            );
            movement.reference_id = reference_id;
            movement.reference_type = reference_type.clone();
            movement.notes = notes.clone();

            self.inventory_repo.record_movement(&mut tx, &movement).await?;

            remaining_to_deduct -= deduction;
        }

        if remaining_to_deduct > Decimal::ZERO {
            return Err(AppError::validation(&format!(
                "Insufficient stock. Missing: {}",
                remaining_to_deduct
            )));
        }

        tx.commit().await?;
        Ok(())
    }

    /// Process expirations (automagically exhaust batches)
    pub async fn process_expirations(&self, tenant_id: TenantId) -> AppResult<usize> {
        let now = OffsetDateTime::now_utc();
        let mut tx = self.pool.begin().await?;

        // 1. Get all active batches that have expired and have stock > 0
        let query = r#"
            SELECT id, price_per_unit_cents, remaining_quantity, catalog_ingredient_id
            FROM inventory_batches
            WHERE tenant_id = $1 AND status = 'active' AND remaining_quantity > 0 AND expires_at < $2
            FOR UPDATE
        "#;

        let rows = sqlx::query(query)
            .bind(tenant_id.as_uuid())
            .bind(now)
            .fetch_all(&mut *tx)
            .await?;

        let mut processed_count = 0;
        for row in rows {
            let id: uuid::Uuid = row.try_get("id")?;
            let price_cents: i64 = row.try_get("price_per_unit_cents")?;
            let remaining: Decimal = row.try_get("remaining_quantity")?;

            // 2. Clear remaining quantity and mark as exhausted
            sqlx::query(
                "UPDATE inventory_batches SET remaining_quantity = 0, status = 'exhausted', updated_at = $1 WHERE id = $2"
            )
            .bind(now)
            .bind(id)
            .execute(&mut *tx)
            .await?;

            // 3. Record OutExpire movement
            let mut movement = InventoryMovement::new(
                tenant_id,
                InventoryBatchId::from_uuid(id),
                MovementType::OutExpire,
                remaining,
                price_cents,
            );
            movement.reason = Some("Auto-exhaustion due to expiration".to_string());
            movement.reference_type = Some("expiration".to_string());
            
            self.inventory_repo.record_movement(&mut tx, &movement).await?;
            processed_count += 1;
        }

        tx.commit().await?;
        Ok(processed_count)
    }

    /// Get loss report (expired products) for the last N days
    pub async fn get_loss_report(&self, tenant_id: TenantId, days: i32) -> AppResult<LossReport> {
        // 1. Get losses (OUT_EXPIRE)
        let loss_query = r#"
            SELECT 
                ci.id as ingredient_id,
                ci.name_en as ingredient_name,
                SUM(im.quantity) as lost_quantity,
                SUM(im.total_cost_cents)::BIGINT as total_loss_cents
            FROM inventory_movements im
            JOIN inventory_batches ib ON im.batch_id = ib.id
            JOIN catalog_ingredients ci ON ib.catalog_ingredient_id = ci.id
            WHERE im.tenant_id = $1 
              AND im.type = 'OUT_EXPIRE'
              AND im.created_at >= NOW() - ($2 * INTERVAL '1 day')
            GROUP BY ci.id, ci.name_en
            ORDER BY total_loss_cents DESC
        "#;

        let loss_rows = sqlx::query(loss_query)
            .bind(tenant_id.as_uuid())
            .bind(days)
            .fetch_all(&self.pool)
            .await?;

        let mut items = Vec::new();
        let mut total_loss_cents = 0;

        for row in loss_rows {
            let cents: i64 = row.try_get("total_loss_cents")?;
            total_loss_cents += cents;
            
            items.push(LossReportItem {
                ingredient_id: row.try_get("ingredient_id")?,
                ingredient_name: row.try_get("ingredient_name")?,
                lost_quantity: row.try_get::<Decimal, _>("lost_quantity")?.to_f64().unwrap_or(0.0),
                loss_value_cents: cents,
            });
        }

        // 2. Get total purchases (IN) to calculate KPI
        let purchase_query = r#"
            SELECT COALESCE(SUM(total_cost_cents), 0)::BIGINT as total_purchased_cents
            FROM inventory_movements
            WHERE tenant_id = $1 
              AND type = 'IN'
              AND created_at >= NOW() - ($2 * INTERVAL '1 day')
        "#;

        let total_purchased_cents: i64 = sqlx::query_scalar(purchase_query)
            .bind(tenant_id.as_uuid())
            .bind(days)
            .fetch_one(&self.pool)
            .await?;

        let waste_percentage = if total_purchased_cents > 0 {
            (total_loss_cents as f64 / total_purchased_cents as f64) * 100.0
        } else {
            0.0
        };

        Ok(LossReport {
            items,
            total_loss_cents,
            total_purchased_cents,
            waste_percentage,
            period_days: days,
        })
    }

    /// Get status summary (health score, etc.)
    pub async fn get_status(&self, tenant_id: TenantId) -> AppResult<InventoryStatus> {
        self.alert_service.get_inventory_status(tenant_id).await
    }

    /// Get all active alerts (delegated to alert service)
    pub async fn get_alerts(&self, tenant_id: TenantId) -> AppResult<Vec<crate::domain::inventory::InventoryAlert>> {
        self.alert_service.get_alerts(tenant_id).await
    }
}

/// Rich inventory view DTO (returned from query with JOINs)
#[derive(Debug, Clone, Serialize)]
pub struct InventoryView {
    pub id: uuid::Uuid,
    pub product: ProductInfo,
    pub quantity: f64,
    pub remaining_quantity: f64,
    pub price_per_unit_cents: i64,
    /// Expiration severity (for row highlighting on frontend)
    pub severity: ExpirationSeverity,
    #[serde(with = "time::serde::rfc3339")]
    pub received_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub expires_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProductInfo {
    pub id: uuid::Uuid,
    pub name: String,
    pub category: String,
    pub base_unit: String,
    pub image_url: Option<String>,
    pub min_stock_threshold: f64,
}

/// Loss report item (for manager/owner)
#[derive(Debug, Clone, Serialize)]
pub struct LossReportItem {
    pub ingredient_id: uuid::Uuid,
    pub ingredient_name: String,
    pub lost_quantity: f64,
    pub loss_value_cents: i64,
}

/// Loss report summary
#[derive(Debug, Clone, Serialize)]
pub struct LossReport {
    pub items: Vec<LossReportItem>,
    pub total_loss_cents: i64,
    pub total_purchased_cents: i64,
    pub waste_percentage: f64,
    pub period_days: i32,
}
