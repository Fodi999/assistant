use crate::application::inventory::InventoryStatus;
use crate::domain::inventory::{AlertSeverity, InventoryAlert, InventoryAlertType};
use crate::shared::{AppResult, TenantId};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use sqlx::{PgPool, Row};
use time::OffsetDateTime;

#[derive(Clone)]
pub struct InventoryAlertService {
    pool: PgPool,
}

impl InventoryAlertService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Optimized status aggregation for the whole inventory
    /// Returns counts and health score
    pub async fn get_inventory_status(&self, tenant_id: TenantId) -> AppResult<InventoryStatus> {
        let alerts = self.get_alerts(tenant_id).await?;
        
        let mut has_expired = false;
        let mut has_expiring_today = false;
        let mut has_expiring_soon = false;
        let mut has_low_stock = false;
        let mut has_zero_stock = false;

        let mut expired_count = 0;
        let mut critical_count = 0;
        let mut warning_count = 0;
        let mut low_stock_count = 0;

        for a in &alerts {
            match a.alert_type {
                InventoryAlertType::LowStock => {
                    if a.severity == AlertSeverity::Critical {
                        has_zero_stock = true;
                        critical_count += 1; // Zero stock is a critical issue
                    } else {
                        has_low_stock = true;
                        warning_count += 1; // Low stock is a warning issue
                        low_stock_count += 1; // Count specifically items that are low but not empty
                    }
                },
                InventoryAlertType::ExpiringBatch => {
                    match a.severity {
                        AlertSeverity::Expired => {
                            has_expired = true;
                            expired_count += 1;
                        },
                        AlertSeverity::Critical => {
                            has_expiring_today = true;
                            critical_count += 1;
                        },
                        AlertSeverity::Warning => {
                            has_expiring_soon = true;
                            warning_count += 1;
                        },
                        _ => {}
                    }
                }
            }
        }

        // Professional categorical health score formula
        let mut score = 100;
        if has_expired        { score -= 40; }
        if has_expiring_today { score -= 20; }
        if has_expiring_soon  { score -= 10; }
        if has_low_stock      { score -= 15; }
        if has_zero_stock     { score -= 25; }
        
        let health_score = score.max(0);

        let status = match health_score {
            90..=100 => "Excellent",
            70..=89 => "Good",
            40..=69 => "Warning",
            _ => "Critical",
        }.to_string();

        // Badge count: total products requiring immediate attention (Expired + Critical)
        let badge_count = expired_count + critical_count;

        Ok(InventoryStatus {
            health_score,
            status,
            critical: critical_count,
            warning: warning_count,
            expired: expired_count,
            low_stock: low_stock_count,
            badge_count,
        })
    }

    /// Get all active alerts for a tenant
    pub async fn get_alerts(&self, tenant_id: TenantId) -> AppResult<Vec<InventoryAlert>> {
        let mut alerts = Vec::new();

        // 1. Get Expiration Alerts
        let expiration_alerts = self.get_expiration_alerts(tenant_id).await?;
        alerts.extend(expiration_alerts);

        // 2. Get Low Stock Alerts
        let low_stock_alerts = self.get_low_stock_alerts(tenant_id).await?;
        alerts.extend(low_stock_alerts);

        // Sort by severity (Expired -> Critical -> Warning -> Info)
        alerts.sort_by(|a, b| {
            let severity_score = |s: AlertSeverity| match s {
                AlertSeverity::Expired => 0,
                AlertSeverity::Critical => 1,
                AlertSeverity::Warning => 2,
                AlertSeverity::Info => 3,
            };
            severity_score(a.severity).cmp(&severity_score(b.severity))
        });

        Ok(alerts)
    }

    /// Fetch aggregate expiration status by ingredient
    /// If an ingredient has at least 1 expired batch -> Expired
    /// Else if at least 1 critical -> Critical
    /// Else if at least 1 warning -> Warning
    async fn get_expiration_alerts(&self, tenant_id: TenantId) -> AppResult<Vec<InventoryAlert>> {
        let now = OffsetDateTime::now_utc();
        
        let query = r#"
            WITH batch_statuses AS (
                SELECT 
                    ib.catalog_ingredient_id,
                    ci.name_en as ingredient_name,
                    ib.remaining_quantity,
                    CASE 
                        WHEN ib.expires_at < $1 THEN 'expired'
                        WHEN ib.expires_at <= $1 + INTERVAL '1 day' THEN 'critical'
                        WHEN ib.expires_at <= $1 + INTERVAL '3 days' THEN 'warning'
                        ELSE 'ok'
                    END as severity_str
                FROM inventory_batches ib
                JOIN catalog_ingredients ci ON ib.catalog_ingredient_id = ci.id
                WHERE ib.tenant_id = $2 
                  AND ib.remaining_quantity > 0 
                  AND ib.status = 'active'
                  AND ib.expires_at IS NOT NULL
            )
            SELECT 
                catalog_ingredient_id,
                ingredient_name,
                SUM(remaining_quantity) as total_qty,
                MIN(CASE 
                    WHEN severity_str = 'expired' THEN 1
                    WHEN severity_str = 'critical' THEN 2
                    WHEN severity_str = 'warning' THEN 3
                    ELSE 4
                END) as min_severity_rank
            FROM batch_statuses
            WHERE severity_str != 'ok'
            GROUP BY catalog_ingredient_id, ingredient_name
            ORDER BY min_severity_rank ASC
        "#;

        let rows = sqlx::query(query)
            .bind(now)
            .bind(tenant_id.as_uuid())
            .fetch_all(&self.pool)
            .await?;

        let mut alerts = Vec::new();
        for row in rows {
            let rank: i32 = row.try_get("min_severity_rank")?;
            let severity = match rank {
                1 => AlertSeverity::Expired,
                2 => AlertSeverity::Critical,
                3 => AlertSeverity::Warning,
                _ => continue,
            };

            let ingredient_name: String = row.try_get("ingredient_name")?;
            let message = match severity {
                AlertSeverity::Expired => format!("{} has EXPIRED batches!", ingredient_name),
                AlertSeverity::Critical => format!("{} has batches expiring SOON!", ingredient_name),
                AlertSeverity::Warning => format!("{} has batches approaching expiry", ingredient_name),
                _ => format!("{} has expiration issues", ingredient_name),
            };

            alerts.push(InventoryAlert {
                alert_type: InventoryAlertType::ExpiringBatch,
                severity,
                ingredient_id: row.try_get("catalog_ingredient_id")?,
                ingredient_name,
                batch_id: None, // Aggregated alert doesn't point to a single batch
                message,
                current_value: row.try_get::<Decimal, _>("total_qty")?.to_f64().unwrap_or(0.0),
                threshold_value: None,
            });
        }

        Ok(alerts)
    }

    /// Fetch ingredients where total stock is below their threshold
    async fn get_low_stock_alerts(&self, tenant_id: TenantId) -> AppResult<Vec<InventoryAlert>> {
        let query = r#"
            SELECT 
                ci.id as ingredient_id,
                ci.name_en as ingredient_name,
                ci.min_stock_threshold,
                COALESCE(SUM(ib.remaining_quantity), 0) as total_remaining
            FROM catalog_ingredients ci
            LEFT JOIN inventory_batches ib ON ci.id = ib.catalog_ingredient_id 
                AND ib.tenant_id = $1 
                AND ib.status = 'active'
            WHERE ci.is_active = true
            GROUP BY ci.id, ci.name_en, ci.min_stock_threshold
            HAVING COALESCE(SUM(ib.remaining_quantity), 0) <= ci.min_stock_threshold
               OR COALESCE(SUM(ib.remaining_quantity), 0) = 0
        "#;

        let rows = sqlx::query(query)
            .bind(tenant_id.as_uuid())
            .fetch_all(&self.pool)
            .await?;

        let mut alerts = Vec::new();
        for row in rows {
            let total_remaining = row.try_get::<Decimal, _>("total_remaining")?;
            let threshold = row.try_get::<Decimal, _>("min_stock_threshold")?;
            
            // Only alert if threshold is set or if totally empty and we want to track it
            // If threshold is 0, we only alert if it's 0 (Out of stock)
            if threshold == Decimal::ZERO && total_remaining > Decimal::ZERO {
                continue;
            }

            let severity = if total_remaining == Decimal::ZERO {
                AlertSeverity::Critical
            } else {
                AlertSeverity::Warning
            };

            let ingredient_name: String = row.try_get("ingredient_name")?;
            let message = if total_remaining == Decimal::ZERO {
                format!("{} is OUT OF STOCK!", ingredient_name)
            } else {
                format!("{} is low on stock: {} remaining (threshold: {})", 
                    ingredient_name, 
                    total_remaining.round_dp(2), 
                    threshold.round_dp(2)
                )
            };

            alerts.push(InventoryAlert {
                alert_type: InventoryAlertType::LowStock,
                severity,
                ingredient_id: row.try_get("ingredient_id")?,
                ingredient_name,
                batch_id: None,
                message,
                current_value: total_remaining.to_f64().unwrap_or(0.0),
                threshold_value: Some(threshold.to_f64().unwrap_or(0.0)),
            });
        }

        Ok(alerts)
    }
}
