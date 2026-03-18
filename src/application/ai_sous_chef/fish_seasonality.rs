//! Auto-generate seasonality rows for fish/seafood products.
//!
//! When a new fish or seafood product is created in catalog_ingredients,
//! this helper inserts 12 months × 4 regions (GLOBAL, PL, EU, UA) into
//! catalog_product_seasonality.
//!
//! Season status comes from availability_months (bool[12]):
//!   true  → 'good'
//!   false → 'off'
//!   NULL  → all months 'good' (available year-round)

use sqlx::PgPool;
use uuid::Uuid;

/// Regions to generate seasonality for
const REGIONS: &[&str] = &["GLOBAL", "PL", "EU", "UA"];

/// Product types eligible for auto-seasonality
const FISH_TYPES: &[&str] = &["fish", "seafood"];

/// Auto-generate seasonality rows for a newly created fish/seafood product.
/// Returns the number of rows inserted.
pub async fn generate_seasonality_for_product(
    pool: &PgPool,
    product_id: Uuid,
) -> Result<usize, String> {
    // 1. Load product_type and availability_months
    let row: Option<(String, Option<Vec<bool>>)> = sqlx::query_as(
        r#"SELECT COALESCE(product_type, 'other'), availability_months
           FROM catalog_ingredients
           WHERE id = $1 AND is_active = true"#,
    )
    .bind(product_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("DB error loading product {}: {}", product_id, e))?;

    let (product_type, availability_months) = match row {
        Some(r) => r,
        None => return Err(format!("Product {} not found or inactive", product_id)),
    };

    // Only for fish/seafood
    if !FISH_TYPES.contains(&product_type.as_str()) {
        tracing::debug!(
            "⏭️ Skipping seasonality for product {} (type={})",
            product_id, product_type
        );
        return Ok(0);
    }

    // 2. Determine status for each month
    let statuses: Vec<&str> = (0..12)
        .map(|i| {
            match &availability_months {
                Some(months) if months.len() > i => {
                    if months[i] { "good" } else { "off" }
                }
                None => "good", // No availability_months → assume all year
                _ => "off",
            }
        })
        .collect();

    // 3. Insert into catalog_product_seasonality for all regions
    let mut total_inserted = 0usize;

    for region in REGIONS {
        for month_idx in 0..12 {
            let month = (month_idx + 1) as i16;
            let status = statuses[month_idx];

            let result = sqlx::query(
                r#"INSERT INTO catalog_product_seasonality (product_id, region_code, month, status)
                   VALUES ($1, $2, $3, $4)
                   ON CONFLICT (product_id, region_code, month) DO NOTHING"#,
            )
            .bind(product_id)
            .bind(*region)
            .bind(month)
            .bind(status)
            .execute(pool)
            .await;

            match result {
                Ok(r) => total_inserted += r.rows_affected() as usize,
                Err(e) => {
                    tracing::warn!(
                        "⚠️ Failed to insert seasonality for {} region={} month={}: {}",
                        product_id, region, month, e
                    );
                }
            }
        }
    }

    // 4. Ensure availability_model = 'seasonal_calendar'
    let _ = sqlx::query(
        r#"UPDATE catalog_ingredients
           SET availability_model = 'seasonal_calendar'
           WHERE id = $1 AND (availability_model IS NULL OR availability_model = 'all_year')"#,
    )
    .bind(product_id)
    .execute(pool)
    .await;

    tracing::info!(
        "🗓️ Auto-generated {} seasonality rows for product {} (type={})",
        total_inserted, product_id, product_type
    );

    Ok(total_inserted)
}
