use crate::application::{DishService, InventoryService, MenuEngineeringService};
use crate::domain::report::{
    DishHighlight, EngineeringAgg, InventoryAgg, MenuAgg, SalesAgg, TenantSummary,
};
use crate::domain::MenuCategory;
use crate::shared::{AppResult, Language, PaginationParams, TenantId, UserId};
use sqlx::PgPool;

#[derive(Clone)]
pub struct ReportService {
    pool: PgPool,
    dish_service: DishService,
    inventory_service: InventoryService,
    menu_engineering_service: MenuEngineeringService,
}

impl ReportService {
    pub fn new(
        pool: PgPool,
        dish_service: DishService,
        inventory_service: InventoryService,
        menu_engineering_service: MenuEngineeringService,
    ) -> Self {
        Self {
            pool,
            dish_service,
            inventory_service,
            menu_engineering_service,
        }
    }

    /// GET /api/reports/summary — the "one glance" executive view.
    pub async fn get_summary(
        &self,
        user_id: UserId,
        tenant_id: TenantId,
        language: Language,
        period_days: u32,
    ) -> AppResult<TenantSummary> {
        // 1. Sales aggregation (direct SQL — fast single query)
        let sales = self.aggregate_sales(tenant_id, period_days).await?;

        // 2. Menu health from materialized dish costs
        let menu = self.aggregate_menu(tenant_id).await?;

        // 3. Inventory health
        let inventory = self
            .aggregate_inventory(tenant_id, period_days, &language)
            .await?;

        // 4. Menu Engineering (BCG matrix)
        let engineering = self
            .aggregate_engineering(user_id, tenant_id, language, period_days)
            .await?;

        Ok(TenantSummary::build(
            period_days,
            sales,
            menu,
            inventory,
            engineering,
        ))
    }

    /// Aggregate revenue/profit from dish_sales table
    async fn aggregate_sales(
        &self,
        tenant_id: TenantId,
        period_days: u32,
    ) -> AppResult<SalesAgg> {
        let row = sqlx::query_as::<_, (Option<i64>, Option<i64>, Option<i64>)>(
            r#"
            SELECT
                COALESCE(SUM(selling_price_cents::BIGINT * quantity), 0)::BIGINT,
                COALESCE(SUM(profit_cents::BIGINT), 0)::BIGINT,
                COALESCE(SUM(quantity::BIGINT), 0)::BIGINT
            FROM dish_sales
            WHERE tenant_id = $1
              AND sold_at >= NOW() - ($2 * INTERVAL '1 day')
            "#,
        )
        .bind(tenant_id.as_uuid())
        .bind(period_days as i32)
        .fetch_one(&self.pool)
        .await
        .map_err(crate::shared::AppError::Database)?;

        Ok(SalesAgg {
            total_revenue_cents: row.0.unwrap_or(0),
            total_profit_cents: row.1.unwrap_or(0),
            total_orders: row.2.unwrap_or(0) as u32,
        })
    }

    /// Aggregate menu KPIs from materialized dish costs
    async fn aggregate_menu(&self, tenant_id: TenantId) -> AppResult<MenuAgg> {
        let all_pages = PaginationParams {
            page: Some(1),
            per_page: Some(100),
        };

        let (dishes, _total) = self
            .dish_service
            .list_dishes(tenant_id, false, &all_pages)
            .await?;

        let total_dishes = dishes.len() as u32;
        let mut with_cost = 0u32;
        let mut food_cost_sum = 0.0f64;
        let mut margin_sum = 0.0f64;
        let mut best: Option<DishHighlight> = None;
        let mut worst: Option<DishHighlight> = None;

        for dish in &dishes {
            if let (Some(fc), Some(pm)) = (dish.food_cost_percent(), dish.profit_margin_percent()) {
                with_cost += 1;
                food_cost_sum += fc;
                margin_sum += pm;

                let highlight = DishHighlight {
                    name: dish.name().as_str().to_string(),
                    profit_margin_percent: pm,
                };

                match &best {
                    None => best = Some(highlight.clone()),
                    Some(b) if pm > b.profit_margin_percent => best = Some(highlight.clone()),
                    _ => {}
                }
                match &worst {
                    None => worst = Some(highlight),
                    Some(w) if pm < w.profit_margin_percent => worst = Some(highlight),
                    _ => {}
                }
            }
        }

        let avg_food_cost = if with_cost > 0 {
            food_cost_sum / with_cost as f64
        } else {
            0.0
        };
        let avg_margin = if with_cost > 0 {
            margin_sum / with_cost as f64
        } else {
            0.0
        };

        Ok(MenuAgg {
            total_dishes,
            dishes_with_cost: with_cost,
            avg_food_cost_percent: (avg_food_cost * 10.0).round() / 10.0,
            avg_profit_margin_percent: (avg_margin * 10.0).round() / 10.0,
            best_dish: best,
            worst_dish: worst,
        })
    }

    /// Aggregate inventory health
    async fn aggregate_inventory(
        &self,
        tenant_id: TenantId,
        period_days: u32,
        language: &Language,
    ) -> AppResult<InventoryAgg> {
        let status = self.inventory_service.get_status(tenant_id).await?;

        let loss = self
            .inventory_service
            .get_loss_report(tenant_id, period_days as i32, language.code())
            .await?;

        Ok(InventoryAgg {
            health_score: status.health_score,
            expired: status.expired,
            critical: status.critical,
            waste_cents: loss.total_loss_cents,
            waste_percent: (loss.waste_percentage * 10.0).round() / 10.0,
        })
    }

    /// Aggregate Menu Engineering (BCG matrix counts)
    async fn aggregate_engineering(
        &self,
        user_id: UserId,
        tenant_id: TenantId,
        language: Language,
        period_days: u32,
    ) -> AppResult<EngineeringAgg> {
        match self
            .menu_engineering_service
            .analyze_menu(user_id, tenant_id, language, period_days)
            .await
        {
            Ok(matrix) => {
                let mut stars = 0u32;
                let mut plowhorses = 0u32;
                let mut puzzles = 0u32;
                let mut dogs = 0u32;

                for dish in &matrix.dishes {
                    match dish.category {
                        MenuCategory::Star => stars += 1,
                        MenuCategory::Plowhorse => plowhorses += 1,
                        MenuCategory::Puzzle => puzzles += 1,
                        MenuCategory::Dog => dogs += 1,
                    }
                }

                Ok(EngineeringAgg {
                    stars,
                    plowhorses,
                    puzzles,
                    dogs,
                })
            }
            Err(e) => {
                tracing::warn!("⚠️ Menu engineering analysis failed: {}. Using defaults.", e);
                Ok(EngineeringAgg::default())
            }
        }
    }
}
