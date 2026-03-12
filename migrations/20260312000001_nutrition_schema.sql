-- ══════════════════════════════════════════════════════════════════════════════
-- NUTRITION & PRODUCT DETAIL SCHEMA
-- Migration: 20260312000001_nutrition_schema.sql
-- ══════════════════════════════════════════════════════════════════════════════

-- ── 1. nutrition_macros ───────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS nutrition_macros (
    product_id          UUID PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,

    calories_kcal       REAL,
    protein_g           REAL,
    fat_g               REAL,
    carbs_g             REAL,

    fiber_g             REAL,
    sugar_g             REAL,
    starch_g            REAL,

    water_g             REAL,
    alcohol_g           REAL
);

-- ── 2. nutrition_vitamins ─────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS nutrition_vitamins (
    product_id          UUID PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,

    vitamin_a           REAL,
    vitamin_c           REAL,
    vitamin_d           REAL,
    vitamin_e           REAL,
    vitamin_k           REAL,

    vitamin_b1          REAL,
    vitamin_b2          REAL,
    vitamin_b3          REAL,
    vitamin_b5          REAL,
    vitamin_b6          REAL,
    vitamin_b7          REAL,
    vitamin_b9          REAL,
    vitamin_b12         REAL
);

-- ── 3. nutrition_minerals ─────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS nutrition_minerals (
    product_id          UUID PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,

    calcium             REAL,
    iron                REAL,
    magnesium           REAL,
    phosphorus          REAL,
    potassium           REAL,
    sodium              REAL,
    zinc                REAL,
    copper              REAL,
    manganese           REAL,
    selenium            REAL
);

-- ── 4. nutrition_amino_acids ──────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS nutrition_amino_acids (
    product_id          UUID PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,

    leucine             REAL,
    lysine              REAL,
    valine              REAL,
    isoleucine          REAL,
    methionine          REAL,
    tryptophan          REAL,
    phenylalanine       REAL,

    alanine             REAL,
    arginine            REAL,
    glycine             REAL,
    proline             REAL,
    serine              REAL,
    tyrosine            REAL
);

-- ── 5. nutrition_fatty_acids ──────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS nutrition_fatty_acids (
    product_id              UUID PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,

    saturated_fat           REAL,
    monounsaturated_fat     REAL,
    polyunsaturated_fat     REAL,

    omega3                  REAL,
    omega6                  REAL,

    epa                     REAL,
    dha                     REAL
);

-- ── 6. nutrition_carbohydrates ────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS nutrition_carbohydrates (
    product_id              UUID PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,

    glucose                 REAL,
    fructose                REAL,
    sucrose                 REAL,
    lactose                 REAL,
    maltose                 REAL,

    starch                  REAL,
    resistant_starch        REAL
);

-- ── 7. nutrition_phytonutrients ───────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS nutrition_phytonutrients (
    product_id              UUID PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,

    beta_carotene           REAL,
    lutein                  REAL,
    lycopene                REAL,

    polyphenols             REAL,
    flavonoids              REAL
);

-- ── 8. food_properties ───────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS food_properties (
    product_id              UUID PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,

    glycemic_index          REAL,
    glycemic_load           REAL,

    ph                      REAL,
    smoke_point             REAL,
    water_activity          REAL
);

-- ── 9. nutrition_antinutrients ────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS nutrition_antinutrients (
    product_id              UUID PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,

    oxalates                REAL,
    phytates                REAL,
    lectins                 REAL,
    tannins                 REAL
);

-- ── 10. nutrition_bioavailability ─────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS nutrition_bioavailability (
    product_id                  UUID PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,

    iron_absorption_rate        REAL,
    calcium_absorption_rate     REAL,
    protein_digestibility       REAL
);

-- ── 11. diet_flags ────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS diet_flags (
    product_id              UUID PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,

    vegan                   BOOLEAN DEFAULT FALSE,
    vegetarian              BOOLEAN DEFAULT FALSE,
    keto                    BOOLEAN DEFAULT FALSE,
    paleo                   BOOLEAN DEFAULT FALSE,
    gluten_free             BOOLEAN DEFAULT FALSE,
    mediterranean           BOOLEAN DEFAULT FALSE,
    low_carb                BOOLEAN DEFAULT FALSE
);

-- ── 12. product_allergens ─────────────────────────────────────────────────────
-- (renamed from allergens to avoid conflict with existing allergens array on products)
CREATE TABLE IF NOT EXISTS product_allergens (
    product_id              UUID REFERENCES products(id) ON DELETE CASCADE,

    milk                    BOOLEAN DEFAULT FALSE,
    fish                    BOOLEAN DEFAULT FALSE,
    shellfish               BOOLEAN DEFAULT FALSE,
    nuts                    BOOLEAN DEFAULT FALSE,
    soy                     BOOLEAN DEFAULT FALSE,
    gluten                  BOOLEAN DEFAULT FALSE,
    eggs                    BOOLEAN DEFAULT FALSE,

    PRIMARY KEY (product_id)
);

-- ── 13. food_culinary_properties ──────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS food_culinary_properties (
    product_id              UUID PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,

    sweetness               REAL,
    acidity                 REAL,
    bitterness              REAL,
    umami                   REAL,

    aroma                   REAL,
    texture                 TEXT
);

-- ── 14. food_pairing ─────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS food_pairing (
    ingredient_a            UUID REFERENCES products(id) ON DELETE CASCADE,
    ingredient_b            UUID REFERENCES products(id) ON DELETE CASCADE,

    flavor_score            REAL,
    nutrition_score         REAL,
    culinary_score          REAL,

    pair_score              REAL,

    PRIMARY KEY (ingredient_a, ingredient_b)
);

-- ── 15. recipes ───────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS recipes_catalog (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name                    TEXT NOT NULL,
    description             TEXT,
    cuisine                 TEXT,

    created_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ── 16. recipe_ingredients ────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS recipe_catalog_ingredients (
    recipe_id               UUID REFERENCES recipes_catalog(id) ON DELETE CASCADE,
    ingredient_id           UUID REFERENCES products(id) ON DELETE CASCADE,

    amount_g                REAL,

    PRIMARY KEY (recipe_id, ingredient_id)
);

-- ── Indexes ───────────────────────────────────────────────────────────────────
CREATE INDEX IF NOT EXISTS idx_food_pairing_a ON food_pairing(ingredient_a);
CREATE INDEX IF NOT EXISTS idx_food_pairing_b ON food_pairing(ingredient_b);
CREATE INDEX IF NOT EXISTS idx_recipe_catalog_ingredients_recipe ON recipe_catalog_ingredients(recipe_id);
CREATE INDEX IF NOT EXISTS idx_recipe_catalog_ingredients_product ON recipe_catalog_ingredients(ingredient_id);
