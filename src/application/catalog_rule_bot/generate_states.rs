use crate::application::catalog_rule_bot::{
    nutrition_transform::{transform_nutrition, BaseNutrition},
    storage_rules::{get_storage_rule, override_shelf_life},
    translation_rules::{get_state_notes, get_state_suffix},
};
use crate::domain::ProcessingState;
use crate::shared::{AppError, AppResult};
use sqlx::PgPool;
use uuid::Uuid;

/// Row from catalog_ingredients with base nutrition
#[derive(Debug, sqlx::FromRow)]
struct IngredientBaseRow {
    id: Uuid,
    name_en: String,
    product_type: String,
    calories_per_100g: Option<i32>,
    protein_per_100g: Option<f64>,
    fat_per_100g: Option<f64>,
    carbs_per_100g: Option<f64>,
    fiber_per_100g: Option<f64>,
}

/// Generate all missing states for a single ingredient.
/// Returns the number of states created.
pub async fn generate_states_for_ingredient(
    pool: &PgPool,
    ingredient_id: Uuid,
) -> AppResult<Vec<ProcessingState>> {
    // 1. Load base ingredient
    let row = sqlx::query_as::<_, IngredientBaseRow>(
        r#"SELECT id, name_en,
                  COALESCE(product_type, 'other') as product_type,
                  calories_per_100g,
                  protein_per_100g::float8 as protein_per_100g,
                  fat_per_100g::float8 as fat_per_100g,
                  carbs_per_100g::float8 as carbs_per_100g,
                  fiber_per_100g::float8 as fiber_per_100g
           FROM catalog_ingredients
           WHERE id = $1 AND is_active = true"#,
    )
    .bind(ingredient_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::not_found("Ingredient not found"))?;

    // 2. Check which states already exist
    let existing: Vec<String> = sqlx::query_scalar(
        "SELECT state::text FROM ingredient_states WHERE ingredient_id = $1",
    )
    .bind(ingredient_id)
    .fetch_all(pool)
    .await?;

    // 3. Build base nutrition (use defaults if missing)
    let base = BaseNutrition {
        calories: row.calories_per_100g.unwrap_or(0) as f64,
        protein: row.protein_per_100g.unwrap_or(0.0),
        fat: row.fat_per_100g.unwrap_or(0.0),
        carbs: row.carbs_per_100g.unwrap_or(0.0),
        fiber: row.fiber_per_100g.unwrap_or(0.0),
        water_percent: 70.0, // default — will be improved later
    };

    let has_nutrition = row.calories_per_100g.is_some();

    // 4. Generate missing states
    let mut created = Vec::new();

    for &state in ProcessingState::ALL {
        let state_str = state.as_str();
        if existing.contains(&state_str.to_string()) {
            continue; // already exists
        }

        // Transform nutrition
        let nutrition = transform_nutrition(&base, state);

        // Get storage rules
        let storage = get_storage_rule(state);
        let shelf_life = override_shelf_life(&row.product_type, state)
            .unwrap_or(storage.shelf_life_hours);

        // Get translations
        let suffix = get_state_suffix(state);
        let notes = get_state_notes(state);

        // Calculate data score
        let data_score = calculate_data_score(has_nutrition, true, true);

        // Insert
        sqlx::query(
            r#"INSERT INTO ingredient_states (
                ingredient_id, state,
                calories_per_100g, protein_per_100g, fat_per_100g,
                carbs_per_100g, fiber_per_100g, water_percent,
                shelf_life_hours, storage_temp_c, texture,
                name_suffix_en, name_suffix_pl, name_suffix_ru, name_suffix_uk,
                notes_en, notes_pl, notes_ru, notes_uk,
                notes, generated_by, data_score
            ) VALUES (
                $1, $2::processing_state,
                $3, $4, $5, $6, $7, $8,
                $9, $10, $11,
                $12, $13, $14, $15,
                $16, $17, $18, $19,
                $20, 'rule_bot', $21
            )
            ON CONFLICT (ingredient_id, state) DO NOTHING"#,
        )
        .bind(ingredient_id)
        .bind(state_str)
        .bind(nutrition.calories_per_100g)
        .bind(nutrition.protein_per_100g)
        .bind(nutrition.fat_per_100g)
        .bind(nutrition.carbs_per_100g)
        .bind(nutrition.fiber_per_100g)
        .bind(nutrition.water_percent)
        .bind(shelf_life)
        .bind(storage.storage_temp_c)
        .bind(storage.texture)
        .bind(suffix.en)
        .bind(suffix.pl)
        .bind(suffix.ru)
        .bind(suffix.uk)
        .bind(notes.en)
        .bind(notes.pl)
        .bind(notes.ru)
        .bind(notes.uk)
        .bind(format!("{} — {}", row.name_en, suffix.en))
        .bind(data_score)
        .execute(pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to insert state {} for {}: {}", state_str, row.name_en, e);
            AppError::internal("Failed to generate state")
        })?;

        created.push(state);
    }

    if !created.is_empty() {
        tracing::info!(
            "✅ Generated {} states for {} ({})",
            created.len(),
            row.name_en,
            ingredient_id,
        );
    }

    Ok(created)
}

/// Calculate data completeness score (0-100)
fn calculate_data_score(has_nutrition: bool, has_storage: bool, has_translations: bool) -> f64 {
    let mut score = 0.0;
    let mut total = 0.0;

    // Nutrition fields (50% weight)
    total += 50.0;
    if has_nutrition { score += 50.0; }

    // Storage fields (25% weight)
    total += 25.0;
    if has_storage { score += 25.0; }

    // Translation fields (25% weight)
    total += 25.0;
    if has_translations { score += 25.0; }

    if total > 0.0 { (score / total) * 100.0 } else { 0.0 }
}
