use crate::shared::AppResult;
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

/// Summary for a single ingredient's state coverage
#[derive(Debug, Serialize)]
pub struct IngredientStateAudit {
    pub id: Uuid,
    pub name_en: String,
    pub product_type: String,
    pub has_base_nutrition: bool,
    pub total_states: i64,
    pub missing_states: Vec<String>,
    pub missing_translations: Vec<String>,
    pub data_score_avg: f64,
}

/// Full catalog state audit result
#[derive(Debug, Serialize)]
pub struct CatalogStateAudit {
    pub total_ingredients: usize,
    pub ingredients_with_all_states: usize,
    pub ingredients_missing_states: usize,
    pub total_state_records: i64,
    pub expected_state_records: usize,
    pub coverage_percent: f64,
    pub details: Vec<IngredientStateAudit>,
}

/// Row from scanning query
#[derive(Debug, sqlx::FromRow)]
struct ScanRow {
    id: Uuid,
    name_en: String,
    product_type: String,
    has_nutrition: bool,
    state_count: i64,
    existing_states: Vec<String>,
}

/// Scan entire catalog and return audit of state coverage
pub async fn scan_catalog_states(pool: &PgPool) -> AppResult<CatalogStateAudit> {
    let rows = sqlx::query_as::<_, ScanRow>(
        r#"
        SELECT
            ci.id,
            ci.name_en,
            COALESCE(ci.product_type, 'other') as product_type,
            (ci.calories_per_100g IS NOT NULL) as has_nutrition,
            COALESCE(s.cnt, 0) as state_count,
            COALESCE(s.states, ARRAY[]::text[]) as existing_states
        FROM catalog_ingredients ci
        LEFT JOIN LATERAL (
            SELECT
                COUNT(*) as cnt,
                array_agg(state::text) as states
            FROM ingredient_states
            WHERE ingredient_id = ci.id
        ) s ON true
        WHERE ci.is_active = true
        ORDER BY s.cnt ASC, ci.name_en ASC
        "#,
    )
    .fetch_all(pool)
    .await?;

    let total = rows.len();
    let all_states: Vec<&str> = crate::domain::ProcessingState::ALL
        .iter()
        .map(|s| s.as_str())
        .collect();
    let num_states = all_states.len();

    let mut details = Vec::new();
    let mut total_records: i64 = 0;
    let mut complete = 0usize;

    for row in &rows {
        total_records += row.state_count;

        let missing: Vec<String> = all_states
            .iter()
            .filter(|s| !row.existing_states.contains(&s.to_string()))
            .map(|s| s.to_string())
            .collect();

        if missing.is_empty() {
            complete += 1;
        }

        // Only include incomplete products in details
        if !missing.is_empty() {
            details.push(IngredientStateAudit {
                id: row.id,
                name_en: row.name_en.clone(),
                product_type: row.product_type.clone(),
                has_base_nutrition: row.has_nutrition,
                total_states: row.state_count,
                missing_states: missing,
                missing_translations: vec![], // filled below if needed
                data_score_avg: if row.state_count > 0 {
                    50.0 + (row.state_count as f64 / num_states as f64) * 50.0
                } else {
                    0.0
                },
            });
        }
    }

    let expected = total * num_states;
    let coverage = if expected > 0 {
        (total_records as f64 / expected as f64) * 100.0
    } else {
        0.0
    };

    Ok(CatalogStateAudit {
        total_ingredients: total,
        ingredients_with_all_states: complete,
        ingredients_missing_states: total - complete,
        total_state_records: total_records,
        expected_state_records: expected,
        coverage_percent: (coverage * 100.0).round() / 100.0,
        details,
    })
}

/// Get list of ingredient IDs that are missing at least one state
pub async fn get_ingredients_missing_states(pool: &PgPool) -> AppResult<Vec<Uuid>> {
    let ids = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT ci.id
        FROM catalog_ingredients ci
        WHERE ci.is_active = true
          AND (
              SELECT COUNT(DISTINCT state)
              FROM ingredient_states
              WHERE ingredient_id = ci.id
          ) < 10
        ORDER BY ci.name_en
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(ids)
}
