use std::collections::HashMap;

use crate::domain::{DishId, DishPerformance, MenuEngineeringMatrix, inventory::{BatchStatus, MovementType, InventoryMovement, Quantity}};
use crate::shared::{AppError, AppResult, Language, TenantId, UserId};
use crate::infrastructure::persistence::{
    InventoryBatchRepositoryTrait, DishRepositoryTrait, RecipeRepositoryTrait
};

use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;
use rust_decimal::Decimal;

/// Service for Menu Engineering analysis
/// Classifies dishes into Star/Plowhorse/Puzzle/Dog categories
#[derive(Clone)]
pub struct MenuEngineeringService {
    pool: PgPool,
    inventory_repo: Arc<dyn InventoryBatchRepositoryTrait>,
    dish_repo: Arc<dyn DishRepositoryTrait>,
    recipe_repo: Arc<dyn RecipeRepositoryTrait>,
}

impl MenuEngineeringService {
    pub fn new(
        pool: PgPool,
        inventory_repo: Arc<dyn InventoryBatchRepositoryTrait>,
        dish_repo: Arc<dyn DishRepositoryTrait>,
        recipe_repo: Arc<dyn RecipeRepositoryTrait>,
    ) -> Self {
        Self {
            pool,
            inventory_repo,
            dish_repo,
            recipe_repo,
        }
    }

    /// Analyze menu performance for a tenant
    /// 
    /// Returns MenuEngineeringMatrix with all dishes classified by profitability and popularity
    pub async fn analyze_menu(
        &self,
        _user_id: UserId,
        tenant_id: TenantId,
        language: Language,
        period_days: u32,
    ) -> AppResult<MenuEngineeringMatrix> {
        // 1. Get all dishes with their financials and sales data
        let sales_data = self.fetch_sales_data(tenant_id, period_days).await?;

        if sales_data.is_empty() {
            return Ok(MenuEngineeringMatrix::analyze(vec![]));
        }

        // 2. Calculate popularity scores (normalize by max sales volume)
        let max_volume = sales_data
            .values()
            .map(|d| d.sales_volume)
            .max()
            .unwrap_or(1) as f64;

        // 3. Build DishPerformance for each dish
        let mut performances: Vec<DishPerformance> = sales_data
            .into_iter()
            .map(|(dish_id, data)| {
                let popularity_score = if max_volume > 0.0 {
                    data.sales_volume as f64 / max_volume
                } else {
                    0.0
                };

                DishPerformance::new(
                    dish_id,
                    data.dish_name,
                    data.avg_profit_margin_percent,
                    popularity_score,
                    data.sales_volume,
                    data.total_revenue_cents,
                    data.total_profit_cents,
                    0.0, // cumulative_revenue_share placeholder (calculated below)
                    language,
                )
            })
            .collect();

        // 4. Sort by revenue descending (for ABC analysis)
        performances.sort_by(|a, b| {
            b.total_revenue_cents.cmp(&a.total_revenue_cents)
        });

        // 5. Calculate cumulative revenue share for ABC classification
        let total_revenue: i64 = performances.iter().map(|p| p.total_revenue_cents).sum();
        let mut cumulative_revenue: i64 = 0;
        
        for perf in &mut performances {
            cumulative_revenue += perf.total_revenue_cents;
            perf.cumulative_revenue_share = if total_revenue > 0 {
                cumulative_revenue as f64 / total_revenue as f64
            } else {
                0.0
            };
            // Recalculate ABC class with correct cumulative share
            perf.abc_class = crate::domain::AbcClass::classify(perf.cumulative_revenue_share);
        }

        // 6. Sort by contribution margin (profit × volume) descending for display
        performances.sort_by(|a, b| {
            b.contribution_margin_cents
                .cmp(&a.contribution_margin_cents)
        });

        // 7. Return matrix
        Ok(MenuEngineeringMatrix::analyze(performances))
    }

    /// Fetch aggregated sales data for all dishes in tenant
    async fn fetch_sales_data(
        &self,
        tenant_id: TenantId,
        period_days: u32,
    ) -> AppResult<HashMap<DishId, AggregatedDishData>> {
        let tenant_uuid = *tenant_id.as_uuid();
        let period_days_str = period_days.to_string();

        // 🎯 FIX: Use manual query instead of query! macro to avoid DB-at-build-time issues
        let rows = sqlx::query(
            r#"
            SELECT
                ds.dish_id,
                d.name as dish_name,
                COUNT(*) as sales_count,
                SUM(ds.quantity) as total_quantity,
                SUM(ds.selling_price_cents * ds.quantity) as total_revenue_cents,
                SUM(ds.profit_cents * ds.quantity) as total_profit_cents,
                AVG(
                    CAST(ds.profit_cents AS FLOAT) / NULLIF(ds.selling_price_cents, 0) * 100.0
                ) as avg_profit_margin_percent
            FROM dish_sales ds
            JOIN dishes d ON d.id = ds.dish_id AND d.tenant_id = ds.tenant_id
            WHERE
                ds.tenant_id = $1
                AND ds.sold_at >= NOW() - ($2 || ' days')::INTERVAL
            GROUP BY ds.dish_id, d.name
            "#,
        )
        .bind(tenant_uuid)
        .bind(period_days_str)
        .fetch_all(&self.pool)
        .await?;

        let mut map = HashMap::new();

        for row in rows {
            use sqlx::Row;
            let dish_id: Uuid = row.try_get("dish_id").map_err(|e| AppError::internal(&format!("DB: {}", e)))?;
            let dish_name: String = row.try_get("dish_name").map_err(|e| AppError::internal(&format!("DB: {}", e)))?;
            let total_quantity: i64 = row.try_get("total_quantity").map_err(|e| AppError::internal(&format!("DB: {}", e)))?;
            let total_revenue_cents: i64 = row.try_get("total_revenue_cents").map_err(|e| AppError::internal(&format!("DB: {}", e)))?;
            let total_profit_cents: i64 = row.try_get("total_profit_cents").map_err(|e| AppError::internal(&format!("DB: {}", e)))?;
            let avg_profit_margin_percent: f64 = row.try_get("avg_profit_margin_percent").map_err(|e| AppError::internal(&format!("DB: {}", e)))?;

            map.insert(
                DishId::from_uuid(dish_id),
                AggregatedDishData {
                    dish_name,
                    sales_volume: total_quantity as u32,
                    total_revenue_cents,
                    total_profit_cents,
                    avg_profit_margin_percent,
                },
            );
        }

        Ok(map)
    }

    /// Record a dish sale (called after successful order/payment)
    /// Also automatically deducts ingredients from inventory (FIFO)
    pub async fn record_sale(
        &self,
        tenant_id: TenantId,
        dish_id: Uuid,
        user_id: UserId,
        quantity: u32,
        selling_price_cents: i32,
        recipe_cost_cents: i32,
    ) -> AppResult<()> {
        let tenant_uuid = *tenant_id.as_uuid();
        let user_uuid = *user_id.as_uuid();
        let profit_cents = (selling_price_cents - recipe_cost_cents) * quantity as i32;

        // 1. Record the sale for analytics
        sqlx::query(
            r#"
            INSERT INTO dish_sales (
                tenant_id,
                dish_id,
                user_id,
                quantity,
                selling_price_cents,
                recipe_cost_cents,
                profit_cents
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(tenant_uuid)
        .bind(dish_id)
        .bind(user_uuid)
        .bind(quantity as i32)
        .bind(selling_price_cents)
        .bind(recipe_cost_cents)
        .bind(profit_cents)
        .execute(&self.pool)
        .await?;

        // 2. 🚀 AUTOMATIC INVENTORY DEDUCTION (The "Business Flow" Logic)
        tracing::info!("Starting automatic inventory deduction for dish {}", dish_id);

        // Find the dish to get the recipe
        let dish = self.dish_repo.find_by_id(DishId::from_uuid(dish_id), tenant_id)
            .await?
            .ok_or_else(|| AppError::not_found("Dish not found"))?;

        // Find the recipe
        let recipe = self.recipe_repo.find_by_id(dish.recipe_id, user_id)
            .await?
            .ok_or_else(|| AppError::not_found("Recipe not found"))?;

        tracing::info!("Found recipe {} with {} ingredients", recipe.id().as_uuid(), recipe.ingredients().len());

        // For each ingredient in the recipe, deduct from inventory using FIFO with locking
        for ingredient in recipe.ingredients() {
            let catalog_id = ingredient.catalog_ingredient_id();
            let quantity_per_dish = ingredient.quantity().value();
            let total_to_deduct_val = quantity_per_dish * (quantity as f64);
            
            let target_qty = Quantity::new(total_to_deduct_val)?.decimal();
            if target_qty <= Decimal::ZERO { continue; }

            tracing::info!("Deducting {} units of ingredient {} using FIFO", target_qty, catalog_id.as_uuid());

            let mut tx = self.pool.begin().await?;

            // 1. Get deliveries for this ingredient with FOR UPDATE lock
            let batches = self.inventory_repo
                .list_active_by_ingredient_for_update(&mut tx, tenant_id, catalog_id)
                .await?;

            let mut remaining_to_deduct = target_qty;
            for mut batch in batches {
                if remaining_to_deduct <= Decimal::ZERO { break; }

                let batch_available = batch.remaining_quantity.decimal();
                let deduction = if batch_available <= remaining_to_deduct {
                    batch_available
                } else {
                    remaining_to_deduct
                };

                // 2. Update batch stock
                let new_remaining = batch_available - deduction;
                batch.remaining_quantity = Quantity::from_decimal(new_remaining)?;
                
                if new_remaining <= Decimal::ZERO {
                    batch.status = BatchStatus::Exhausted;
                }

                self.inventory_repo.update_in_transaction(&mut tx, &batch).await?;

                // 3. Record movement for audit log (OutSale)
                let mut movement = InventoryMovement::new(
                    tenant_id,
                    batch.id,
                    MovementType::OutSale,
                    deduction,
                    batch.price_per_unit.as_cents(),
                );
                movement.reference_id = Some(dish_id);
                movement.reference_type = Some("DISH_SALE".to_string());
                movement.notes = Some(format!("Sold {} x dish", quantity));

                self.inventory_repo.record_movement(&mut tx, &movement).await?;

                remaining_to_deduct -= deduction;
            }

            if remaining_to_deduct > Decimal::ZERO {
                tracing::warn!("Insufficient stock for {} during sale. Missing: {}", catalog_id.as_uuid(), remaining_to_deduct);
                // In some systems you might want to allow negative stock or stop the sale. 
                // For now, we continue but warn.
            }

            tx.commit().await?;
        }

        Ok(())
    }
}

/// Aggregated sales data for a dish
struct AggregatedDishData {
    dish_name: String,
    sales_volume: u32,
    total_revenue_cents: i64,
    total_profit_cents: i64,
    avg_profit_margin_percent: f64,
}
