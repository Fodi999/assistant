use serde::Serialize;

/// Executive summary for a restaurant tenant.
/// One endpoint, one glance — the owner sees everything.
#[derive(Debug, Clone, Serialize)]
pub struct TenantSummary {
    pub period_days: u32,

    // Revenue & Profit
    pub total_revenue_cents: i64,
    pub total_profit_cents: i64,
    pub total_orders: u32,
    pub avg_order_profit_cents: i64,

    // Menu health
    pub total_dishes: u32,
    pub dishes_with_cost: u32,
    pub avg_food_cost_percent: f64,
    pub avg_profit_margin_percent: f64,
    pub best_dish: Option<DishHighlight>,
    pub worst_dish: Option<DishHighlight>,

    // Inventory health
    pub inventory_health_score: i32,
    pub expired_products: usize,
    pub critical_products: usize,
    pub waste_cents: i64,
    pub waste_percent: f64,

    // Menu Engineering (BCG)
    pub stars: u32,
    pub plowhorses: u32,
    pub puzzles: u32,
    pub dogs: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct DishHighlight {
    pub name: String,
    pub profit_margin_percent: f64,
}

impl TenantSummary {
    /// Build summary from pre-aggregated parts.
    pub fn build(
        period_days: u32,
        sales: SalesAgg,
        menu: MenuAgg,
        inventory: InventoryAgg,
        engineering: EngineeringAgg,
    ) -> Self {
        let avg_order_profit = if sales.total_orders > 0 {
            sales.total_profit_cents / sales.total_orders as i64
        } else {
            0
        };

        Self {
            period_days,
            total_revenue_cents: sales.total_revenue_cents,
            total_profit_cents: sales.total_profit_cents,
            total_orders: sales.total_orders,
            avg_order_profit_cents: avg_order_profit,
            total_dishes: menu.total_dishes,
            dishes_with_cost: menu.dishes_with_cost,
            avg_food_cost_percent: menu.avg_food_cost_percent,
            avg_profit_margin_percent: menu.avg_profit_margin_percent,
            best_dish: menu.best_dish,
            worst_dish: menu.worst_dish,
            inventory_health_score: inventory.health_score,
            expired_products: inventory.expired,
            critical_products: inventory.critical,
            waste_cents: inventory.waste_cents,
            waste_percent: inventory.waste_percent,
            stars: engineering.stars,
            plowhorses: engineering.plowhorses,
            puzzles: engineering.puzzles,
            dogs: engineering.dogs,
        }
    }
}

// Input structs — ReportService fills these from existing services

#[derive(Debug, Clone, Default)]
pub struct SalesAgg {
    pub total_revenue_cents: i64,
    pub total_profit_cents: i64,
    pub total_orders: u32,
}

#[derive(Debug, Clone, Default)]
pub struct MenuAgg {
    pub total_dishes: u32,
    pub dishes_with_cost: u32,
    pub avg_food_cost_percent: f64,
    pub avg_profit_margin_percent: f64,
    pub best_dish: Option<DishHighlight>,
    pub worst_dish: Option<DishHighlight>,
}

#[derive(Debug, Clone, Default)]
pub struct InventoryAgg {
    pub health_score: i32,
    pub expired: usize,
    pub critical: usize,
    pub waste_cents: i64,
    pub waste_percent: f64,
}

#[derive(Debug, Clone, Default)]
pub struct EngineeringAgg {
    pub stars: u32,
    pub plowhorses: u32,
    pub puzzles: u32,
    pub dogs: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tenant_summary_build() {
        let summary = TenantSummary::build(
            30,
            SalesAgg {
                total_revenue_cents: 500_000, // 5000 PLN
                total_profit_cents: 350_000,  // 3500 PLN
                total_orders: 200,
            },
            MenuAgg {
                total_dishes: 12,
                dishes_with_cost: 10,
                avg_food_cost_percent: 28.5,
                avg_profit_margin_percent: 71.5,
                best_dish: Some(DishHighlight {
                    name: "Borsch".to_string(),
                    profit_margin_percent: 82.0,
                }),
                worst_dish: Some(DishHighlight {
                    name: "Steak".to_string(),
                    profit_margin_percent: 45.0,
                }),
            },
            InventoryAgg {
                health_score: 85,
                expired: 2,
                critical: 3,
                waste_cents: 15_000,
                waste_percent: 3.0,
            },
            EngineeringAgg {
                stars: 4,
                plowhorses: 3,
                puzzles: 3,
                dogs: 2,
            },
        );

        assert_eq!(summary.period_days, 30);
        assert_eq!(summary.total_revenue_cents, 500_000);
        assert_eq!(summary.total_profit_cents, 350_000);
        assert_eq!(summary.total_orders, 200);
        assert_eq!(summary.avg_order_profit_cents, 1750); // 350000 / 200
        assert_eq!(summary.total_dishes, 12);
        assert_eq!(summary.dishes_with_cost, 10);
        assert!((summary.avg_food_cost_percent - 28.5).abs() < 0.01);
        assert_eq!(summary.inventory_health_score, 85);
        assert_eq!(summary.expired_products, 2);
        assert_eq!(summary.waste_cents, 15_000);
        assert_eq!(summary.stars, 4);
        assert_eq!(summary.dogs, 2);
        assert_eq!(summary.best_dish.as_ref().unwrap().name, "Borsch");
        assert_eq!(summary.worst_dish.as_ref().unwrap().name, "Steak");
    }

    #[test]
    fn test_tenant_summary_empty_sales() {
        let summary = TenantSummary::build(
            7,
            SalesAgg::default(),
            MenuAgg::default(),
            InventoryAgg::default(),
            EngineeringAgg::default(),
        );

        assert_eq!(summary.avg_order_profit_cents, 0);
        assert_eq!(summary.total_orders, 0);
        assert!(summary.best_dish.is_none());
    }
}
