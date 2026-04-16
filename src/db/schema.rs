use sqlx::PgPool;

use crate::shared::{AppError, AppResult};

/// Initialise all nutrition & product detail tables.
/// Safe to call on every startup — all statements use `CREATE TABLE IF NOT EXISTS`.
pub async fn init_schema(pool: &PgPool) -> AppResult<()> {
    init_nutrition_macros(pool).await?;
    init_nutrition_vitamins(pool).await?;
    init_nutrition_minerals(pool).await?;
    init_nutrition_amino_acids(pool).await?;
    init_nutrition_fatty_acids(pool).await?;
    init_nutrition_carbohydrates(pool).await?;
    init_nutrition_phytonutrients(pool).await?;
    init_food_properties(pool).await?;
    init_nutrition_antinutrients(pool).await?;
    init_nutrition_bioavailability(pool).await?;
    init_diet_flags(pool).await?;
    init_product_allergens(pool).await?;
    init_food_culinary_properties(pool).await?;
    init_product_health_profile(pool).await?;
    init_nutrition_sugar_profile(pool).await?;
    init_product_processing_effects(pool).await?;
    init_product_culinary_behavior(pool).await?;
    init_food_pairing(pool).await?;
    init_recipes_catalog(pool).await?;
    init_recipe_catalog_ingredients(pool).await?;
    init_indexes(pool).await?;

    tracing::info!("nutrition schema initialised ✓");
    Ok(())
}

// ── 1. nutrition_macros ───────────────────────────────────────────────────────

async fn init_nutrition_macros(pool: &PgPool) -> AppResult<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS nutrition_macros (
            product_id      UUID PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,
            calories_kcal   REAL,
            protein_g       REAL,
            fat_g           REAL,
            carbs_g         REAL,
            fiber_g         REAL,
            sugar_g         REAL,
            starch_g        REAL,
            water_g         REAL,
            alcohol_g       REAL
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("init nutrition_macros: {e}");
        AppError::internal("DB schema error")
    })?;
    Ok(())
}

// ── 2. nutrition_vitamins ─────────────────────────────────────────────────────

async fn init_nutrition_vitamins(pool: &PgPool) -> AppResult<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS nutrition_vitamins (
            product_id  UUID PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,
            vitamin_a   REAL,
            vitamin_c   REAL,
            vitamin_d   REAL,
            vitamin_e   REAL,
            vitamin_k   REAL,
            vitamin_b1  REAL,
            vitamin_b2  REAL,
            vitamin_b3  REAL,
            vitamin_b5  REAL,
            vitamin_b6  REAL,
            vitamin_b7  REAL,
            vitamin_b9  REAL,
            vitamin_b12 REAL
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("init nutrition_vitamins: {e}");
        AppError::internal("DB schema error")
    })?;
    Ok(())
}

// ── 3. nutrition_minerals ─────────────────────────────────────────────────────

async fn init_nutrition_minerals(pool: &PgPool) -> AppResult<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS nutrition_minerals (
            product_id  UUID PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,
            calcium     REAL,
            iron        REAL,
            magnesium   REAL,
            phosphorus  REAL,
            potassium   REAL,
            sodium      REAL,
            zinc        REAL,
            copper      REAL,
            manganese   REAL,
            selenium    REAL
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("init nutrition_minerals: {e}");
        AppError::internal("DB schema error")
    })?;
    Ok(())
}

// ── 4. nutrition_amino_acids ──────────────────────────────────────────────────

async fn init_nutrition_amino_acids(pool: &PgPool) -> AppResult<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS nutrition_amino_acids (
            product_id      UUID PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,
            leucine         REAL,
            lysine          REAL,
            valine          REAL,
            isoleucine      REAL,
            methionine      REAL,
            tryptophan      REAL,
            phenylalanine   REAL,
            alanine         REAL,
            arginine        REAL,
            glycine         REAL,
            proline         REAL,
            serine          REAL,
            tyrosine        REAL
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("init nutrition_amino_acids: {e}");
        AppError::internal("DB schema error")
    })?;
    Ok(())
}

// ── 5. nutrition_fatty_acids ──────────────────────────────────────────────────

async fn init_nutrition_fatty_acids(pool: &PgPool) -> AppResult<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS nutrition_fatty_acids (
            product_id              UUID PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,
            saturated_fat           REAL,
            monounsaturated_fat     REAL,
            polyunsaturated_fat     REAL,
            omega3                  REAL,
            omega6                  REAL,
            epa                     REAL,
            dha                     REAL
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("init nutrition_fatty_acids: {e}");
        AppError::internal("DB schema error")
    })?;
    Ok(())
}

// ── 6. nutrition_carbohydrates ────────────────────────────────────────────────

async fn init_nutrition_carbohydrates(pool: &PgPool) -> AppResult<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS nutrition_carbohydrates (
            product_id          UUID PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,
            glucose             REAL,
            fructose            REAL,
            sucrose             REAL,
            lactose             REAL,
            maltose             REAL,
            starch              REAL,
            resistant_starch    REAL
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("init nutrition_carbohydrates: {e}");
        AppError::internal("DB schema error")
    })?;
    Ok(())
}

// ── 7. nutrition_phytonutrients ───────────────────────────────────────────────

async fn init_nutrition_phytonutrients(pool: &PgPool) -> AppResult<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS nutrition_phytonutrients (
            product_id      UUID PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,
            beta_carotene   REAL,
            lutein          REAL,
            lycopene        REAL,
            polyphenols     REAL,
            flavonoids      REAL
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("init nutrition_phytonutrients: {e}");
        AppError::internal("DB schema error")
    })?;
    Ok(())
}

// ── 8. food_properties ────────────────────────────────────────────────────────

async fn init_food_properties(pool: &PgPool) -> AppResult<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS food_properties (
            product_id      UUID PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,
            glycemic_index  REAL,
            glycemic_load   REAL,
            ph              REAL,
            smoke_point     REAL,
            water_activity  REAL
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("init food_properties: {e}");
        AppError::internal("DB schema error")
    })?;
    Ok(())
}

// ── 9. nutrition_antinutrients ────────────────────────────────────────────────

async fn init_nutrition_antinutrients(pool: &PgPool) -> AppResult<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS nutrition_antinutrients (
            product_id  UUID PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,
            oxalates    REAL,
            phytates    REAL,
            lectins     REAL,
            tannins     REAL
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("init nutrition_antinutrients: {e}");
        AppError::internal("DB schema error")
    })?;
    Ok(())
}

// ── 10. nutrition_bioavailability ─────────────────────────────────────────────

async fn init_nutrition_bioavailability(pool: &PgPool) -> AppResult<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS nutrition_bioavailability (
            product_id                  UUID PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,
            iron_absorption_rate        REAL,
            calcium_absorption_rate     REAL,
            protein_digestibility       REAL
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("init nutrition_bioavailability: {e}");
        AppError::internal("DB schema error")
    })?;
    Ok(())
}

// ── 11. diet_flags ────────────────────────────────────────────────────────────

async fn init_diet_flags(pool: &PgPool) -> AppResult<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS diet_flags (
            product_id      UUID PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,
            vegan           BOOLEAN NOT NULL DEFAULT FALSE,
            vegetarian      BOOLEAN NOT NULL DEFAULT FALSE,
            keto            BOOLEAN NOT NULL DEFAULT FALSE,
            paleo           BOOLEAN NOT NULL DEFAULT FALSE,
            gluten_free     BOOLEAN NOT NULL DEFAULT FALSE,
            mediterranean   BOOLEAN NOT NULL DEFAULT FALSE,
            low_carb        BOOLEAN NOT NULL DEFAULT FALSE
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("init diet_flags: {e}");
        AppError::internal("DB schema error")
    })?;
    Ok(())
}

// ── 12. product_allergens ─────────────────────────────────────────────────────

async fn init_product_allergens(pool: &PgPool) -> AppResult<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS product_allergens (
            product_id  UUID PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,
            milk        BOOLEAN NOT NULL DEFAULT FALSE,
            fish        BOOLEAN NOT NULL DEFAULT FALSE,
            shellfish   BOOLEAN NOT NULL DEFAULT FALSE,
            nuts        BOOLEAN NOT NULL DEFAULT FALSE,
            soy         BOOLEAN NOT NULL DEFAULT FALSE,
            gluten      BOOLEAN NOT NULL DEFAULT FALSE,
            eggs        BOOLEAN NOT NULL DEFAULT FALSE
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("init product_allergens: {e}");
        AppError::internal("DB schema error")
    })?;
    Ok(())
}

// ── 13. food_culinary_properties ──────────────────────────────────────────────

async fn init_food_culinary_properties(pool: &PgPool) -> AppResult<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS food_culinary_properties (
            product_id  UUID PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,
            sweetness   REAL,
            acidity     REAL,
            bitterness  REAL,
            umami       REAL,
            aroma       REAL,
            texture     TEXT
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("init food_culinary_properties: {e}");
        AppError::internal("DB schema error")
    })?;
    Ok(())
}

// ── 13b. product_health_profile ──────────────────────────────────────────────

async fn init_product_health_profile(pool: &PgPool) -> AppResult<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS product_health_profile (
            product_id              UUID PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,
            bioactive_compounds_en  JSONB DEFAULT '[]'::jsonb,
            bioactive_compounds_ru  JSONB DEFAULT '[]'::jsonb,
            bioactive_compounds_pl  JSONB DEFAULT '[]'::jsonb,
            bioactive_compounds_uk  JSONB DEFAULT '[]'::jsonb,
            health_effects_en       JSONB DEFAULT '[]'::jsonb,
            health_effects_ru       JSONB DEFAULT '[]'::jsonb,
            health_effects_pl       JSONB DEFAULT '[]'::jsonb,
            health_effects_uk       JSONB DEFAULT '[]'::jsonb,
            contraindications_en    JSONB DEFAULT '[]'::jsonb,
            contraindications_ru    JSONB DEFAULT '[]'::jsonb,
            contraindications_pl    JSONB DEFAULT '[]'::jsonb,
            contraindications_uk    JSONB DEFAULT '[]'::jsonb,
            food_role               TEXT,
            orac_score              REAL,
            absorption_notes_en     TEXT,
            absorption_notes_ru     TEXT,
            absorption_notes_pl     TEXT,
            absorption_notes_uk     TEXT
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("init product_health_profile: {e}");
        AppError::internal("DB schema error")
    })?;
    Ok(())
}

// ── 13c. nutrition_sugar_profile ─────────────────────────────────────────────

async fn init_nutrition_sugar_profile(pool: &PgPool) -> AppResult<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS nutrition_sugar_profile (
            product_id           UUID PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,
            glucose              REAL,
            fructose             REAL,
            sucrose              REAL,
            lactose              REAL,
            maltose              REAL,
            total_sugars         REAL,
            added_sugars         REAL,
            sweetness_perception REAL,
            sugar_alcohols       REAL
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("init nutrition_sugar_profile: {e}");
        AppError::internal("DB schema error")
    })?;
    Ok(())
}

// ── 13d. product_processing_effects ──────────────────────────────────────────

async fn init_product_processing_effects(pool: &PgPool) -> AppResult<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS product_processing_effects (
            product_id              UUID PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,
            vitamin_retention_pct   REAL,
            protein_denature_temp   REAL,
            mineral_leaching_risk   TEXT,
            best_cooking_method_en  TEXT,
            best_cooking_method_ru  TEXT,
            best_cooking_method_pl  TEXT,
            best_cooking_method_uk  TEXT,
            maillard_temp           REAL,
            processing_notes_en     TEXT,
            processing_notes_ru     TEXT,
            processing_notes_pl     TEXT,
            processing_notes_uk     TEXT
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("init product_processing_effects: {e}");
        AppError::internal("DB schema error")
    })?;
    Ok(())
}

// ── 13e. product_culinary_behavior ───────────────────────────────────────────

async fn init_product_culinary_behavior(pool: &PgPool) -> AppResult<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS product_culinary_behavior (
            product_id    UUID PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,
            behaviors_en  JSONB DEFAULT '[]'::jsonb,
            behaviors_ru  JSONB DEFAULT '[]'::jsonb,
            behaviors_pl  JSONB DEFAULT '[]'::jsonb,
            behaviors_uk  JSONB DEFAULT '[]'::jsonb
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("init product_culinary_behavior: {e}");
        AppError::internal("DB schema error")
    })?;
    Ok(())
}

// ── 14. food_pairing ─────────────────────────────────────────────────────────

async fn init_food_pairing(pool: &PgPool) -> AppResult<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS food_pairing (
            ingredient_a    UUID REFERENCES products(id) ON DELETE CASCADE,
            ingredient_b    UUID REFERENCES products(id) ON DELETE CASCADE,
            flavor_score    REAL,
            nutrition_score REAL,
            culinary_score  REAL,
            pair_score      REAL,
            PRIMARY KEY (ingredient_a, ingredient_b)
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("init food_pairing: {e}");
        AppError::internal("DB schema error")
    })?;
    Ok(())
}

// ── 15. recipes_catalog ───────────────────────────────────────────────────────

async fn init_recipes_catalog(pool: &PgPool) -> AppResult<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS recipes_catalog (
            id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name        TEXT NOT NULL,
            description TEXT,
            cuisine     TEXT,
            created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
            updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("init recipes_catalog: {e}");
        AppError::internal("DB schema error")
    })?;
    Ok(())
}

// ── 16. recipe_catalog_ingredients ───────────────────────────────────────────

async fn init_recipe_catalog_ingredients(pool: &PgPool) -> AppResult<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS recipe_catalog_ingredients (
            recipe_id       UUID REFERENCES recipes_catalog(id) ON DELETE CASCADE,
            ingredient_id   UUID REFERENCES products(id) ON DELETE CASCADE,
            amount_g        REAL,
            PRIMARY KEY (recipe_id, ingredient_id)
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("init recipe_catalog_ingredients: {e}");
        AppError::internal("DB schema error")
    })?;
    Ok(())
}

// ── Indexes ───────────────────────────────────────────────────────────────────

async fn init_indexes(pool: &PgPool) -> AppResult<()> {
    let indexes = [
        "CREATE INDEX IF NOT EXISTS idx_food_pairing_a ON food_pairing(ingredient_a)",
        "CREATE INDEX IF NOT EXISTS idx_food_pairing_b ON food_pairing(ingredient_b)",
        "CREATE INDEX IF NOT EXISTS idx_recipe_catalog_ingredients_recipe  ON recipe_catalog_ingredients(recipe_id)",
        "CREATE INDEX IF NOT EXISTS idx_recipe_catalog_ingredients_product ON recipe_catalog_ingredients(ingredient_id)",
    ];

    for sql in &indexes {
        sqlx::query(sql).execute(pool).await.map_err(|e| {
            tracing::error!("init index: {e}");
            AppError::internal("DB schema error")
        })?;
    }
    Ok(())
}
