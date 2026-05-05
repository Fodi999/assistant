/// Economy snapshot — loads all live restaurant data needed by the city generator.
///
/// This is intentionally a flat struct. When you add delivery/competitors/sales
/// as separate data sources, add new fields here and extend `load()`.
/// The city generator only sees this struct — not the DB directly.

use crate::shared::{AppResult, TenantId, UserId};
use sqlx::PgPool;
use tracing::error;

// ─────────────────────────────────────────────────────────────────────────────
// Snapshot
// ─────────────────────────────────────────────────────────────────────────────

pub struct EconomySnapshot {
    pub restaurant_name: String,

    // Menu
    pub dish_count: i32,
    pub avg_profit_margin: f64,

    // Inventory / warehouse
    pub inventory_count: i32,
    pub inventory_value_cents: i64,
    pub expiring_soon: i32,

    // Onboarding
    pub assistant_progress: i32,

    // Sales
    pub revenue_cents: i64,
}

impl EconomySnapshot {
    /// Load all economy data for the tenant in parallel-friendly queries.
    pub async fn load(pool: &PgPool, user_id: UserId, tenant_id: TenantId) -> AppResult<Self> {
        // ── Restaurant name ───────────────────────────────────────────────
        let restaurant_name: Option<String> = sqlx::query_scalar(
            "SELECT COALESCE(display_name, name) FROM users WHERE id = $1",
        )
        .bind(user_id.0)
        .fetch_optional(pool)
        .await
        .map_err(|e| { error!("city/map: restaurant_name query failed: {e}"); e })?
        .flatten();

        // ── Dish stats ────────────────────────────────────────────────────
        let dish_row: (Option<i64>, Option<f64>) = sqlx::query_as(
            r#"SELECT COUNT(*)::BIGINT,
                      AVG(profit_margin_percent)
               FROM dishes
               WHERE tenant_id = $1 AND active = true"#,
        )
        .bind(tenant_id.0)
        .fetch_one(pool)
        .await
        .map_err(|e| { error!("city/map: dish_row query failed: {e}"); e })
        .unwrap_or((Some(0), Some(0.0)));

        // ── Inventory stats ───────────────────────────────────────────────
        let inv_row: (Option<i64>, Option<i64>, Option<i64>) = sqlx::query_as(
            r#"SELECT
                 COUNT(*)::BIGINT,
                 COALESCE(SUM(quantity * price_per_unit_cents), 0)::BIGINT,
                 COUNT(*) FILTER (
                   WHERE expires_at IS NOT NULL
                     AND expires_at <= NOW() + INTERVAL '3 days'
                     AND expires_at > NOW()
                 )::BIGINT
               FROM inventory_products
               WHERE tenant_id = $1"#,
        )
        .bind(tenant_id.0)
        .fetch_one(pool)
        .await
        .map_err(|e| { error!("city/map: inv_row query failed: {e}"); e })
        .unwrap_or((Some(0), Some(0), Some(0)));

        // ── Assistant progress (0-100) ────────────────────────────────────
        let step_name: Option<String> = sqlx::query_scalar(
            "SELECT current_step FROM assistant_states WHERE tenant_id = $1 ORDER BY updated_at DESC LIMIT 1",
        )
        .bind(tenant_id.0)
        .fetch_optional(pool)
        .await
        .map_err(|e| { error!("city/map: assistant_states query failed: {e}"); e })
        .unwrap_or(None);

        let assistant_progress = step_to_progress(step_name.as_deref());

        // ── Revenue last 30 days ──────────────────────────────────────────
        let revenue: Option<i64> = sqlx::query_scalar(
            r#"SELECT COALESCE(SUM(selling_price_cents::BIGINT * quantity), 0)::BIGINT
               FROM dish_sales
               WHERE tenant_id = $1 AND sold_at >= NOW() - INTERVAL '30 days'"#,
        )
        .bind(tenant_id.0)
        .fetch_optional(pool)
        .await
        .map_err(|e| { error!("city/map: revenue query failed: {e}"); e })
        .unwrap_or(None);

        Ok(EconomySnapshot {
            restaurant_name: restaurant_name
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "Food Empire".to_string()),
            dish_count: dish_row.0.unwrap_or(0) as i32,
            avg_profit_margin: dish_row.1.unwrap_or(0.0),
            inventory_count: inv_row.0.unwrap_or(0) as i32,
            inventory_value_cents: inv_row.1.unwrap_or(0),
            expiring_soon: inv_row.2.unwrap_or(0) as i32,
            assistant_progress,
            revenue_cents: revenue.unwrap_or(0),
        })
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Map assistant step name → progress percentage (0-100).
/// Steps are defined in `migrations/20240102000001_assistant_states.sql`.
fn step_to_progress(step: Option<&str>) -> i32 {
    match step {
        None                    => 0,
        Some("welcome")         => 5,
        Some("start_inventory") => 15,
        Some("add_product")     => 30,
        Some("finish_inventory")=> 45,
        Some("create_recipe")   => 60,
        Some("create_dish")     => 75,
        Some("view_report")     => 90,
        Some("done")            => 100,
        Some(_)                 => 10,  // unknown future step
    }
}
