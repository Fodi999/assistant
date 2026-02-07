use std::collections::HashMap;

use crate::domain::{DishId, DishPerformance, MenuCategory, MenuEngineeringMatrix};
use crate::shared::{AppResult, Language, TenantId, UserId};

use sqlx::PgPool;
use uuid::Uuid;

/// Service for Menu Engineering analysis
/// Classifies dishes into Star/Plowhorse/Puzzle/Dog categories
#[derive(Clone)]
pub struct MenuEngineeringService {
    pool: PgPool,
}

impl MenuEngineeringService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Analyze menu performance for a tenant
    /// 
    /// Returns MenuEngineeringMatrix with all dishes classified by profitability and popularity
    pub async fn analyze_menu(
        &self,
        user_id: UserId,
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

        let rows = sqlx::query!(
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
            tenant_uuid,
            period_days_str
        )
        .fetch_all(&self.pool)
        .await?;

        let mut map = HashMap::new();

        for row in rows {
            let dish_id = DishId::from_uuid(row.dish_id);
            map.insert(
                dish_id,
                AggregatedDishData {
                    dish_name: row.dish_name,
                    sales_volume: row.total_quantity.unwrap_or(0) as u32,
                    total_revenue_cents: row.total_revenue_cents.unwrap_or(0),
                    total_profit_cents: row.total_profit_cents.unwrap_or(0),
                    avg_profit_margin_percent: row.avg_profit_margin_percent.unwrap_or(0.0),
                },
            );
        }

        Ok(map)
    }

    /// Record a dish sale (called after successful order/payment)
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

        sqlx::query!(
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
            tenant_uuid,
            dish_id,
            user_uuid,
            quantity as i32,
            selling_price_cents,
            recipe_cost_cents,
            profit_cents
        )
        .execute(&self.pool)
        .await?;

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
