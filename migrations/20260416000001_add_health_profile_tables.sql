-- ══════════════════════════════════════════════════════════════════════════════
-- HEALTH PROFILE, SUGAR PROFILE & PROCESSING EFFECTS
-- Migration: 20260416000001_add_health_profile_tables.sql
--
-- Adds missing product data fields:
--   🔴 MUST HAVE: bioactive_compounds, health_effects, contraindications, food_role
--   🟠 IMPORTANT: sugar_profile, processing_effects
-- ══════════════════════════════════════════════════════════════════════════════

-- ── 1. product_health_profile ─────────────────────────────────────────────────
-- Bioactives, health effects, contraindications, food role
CREATE TABLE IF NOT EXISTS product_health_profile (
    product_id              UUID PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,

    -- 🔴 bioactive compounds (JSON array of strings, e.g. ["beta-carotene","lycopene"])
    bioactive_compounds     JSONB DEFAULT '[]'::jsonb,

    -- 🔴 health effects (JSON array, e.g. ["antioxidant","anti-inflammatory"])
    health_effects          JSONB DEFAULT '[]'::jsonb,

    -- 🔴 contraindications (JSON array, e.g. ["kidney stones","blood thinners"])
    contraindications       JSONB DEFAULT '[]'::jsonb,

    -- 🔴 food role (e.g. "protein_source", "flavor_base", "garnish", "binder")
    food_role               TEXT,

    -- 🟢 deeper chemistry: antioxidant capacity (ORAC score μmol TE/100g)
    orac_score              REAL,

    -- 🟢 deeper chemistry: anti-nutrient interactions
    absorption_notes        TEXT
);

-- ── 2. nutrition_sugar_profile ────────────────────────────────────────────────
-- Detailed sugar breakdown (extends nutrition_carbohydrates)
CREATE TABLE IF NOT EXISTS nutrition_sugar_profile (
    product_id              UUID PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,

    -- individual sugars (g per 100g)
    glucose                 REAL,
    fructose                REAL,
    sucrose                 REAL,
    lactose                 REAL,
    maltose                 REAL,

    -- total & added sugars
    total_sugars            REAL,
    added_sugars            REAL,

    -- sweetness perception (1–10 scale)
    sweetness_perception    REAL,

    -- sugar alcohol content (sorbitol, xylitol, etc.)
    sugar_alcohols          REAL
);

-- ── 3. product_processing_effects ─────────────────────────────────────────────
-- How processing affects the product
CREATE TABLE IF NOT EXISTS product_processing_effects (
    product_id              UUID PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,

    -- vitamin retention after cooking (% remaining, e.g. 60.0 = 60%)
    vitamin_retention_pct   REAL,

    -- protein denaturation temperature (°C)
    protein_denature_temp   REAL,

    -- mineral leaching risk (low / medium / high)
    mineral_leaching_risk   TEXT,

    -- best cooking method for nutrient preservation
    best_cooking_method     TEXT,

    -- Maillard reaction temperature (°C)
    maillard_temp           REAL,

    -- notes on processing effects (free text)
    processing_notes        TEXT
);

-- ── Indexes ───────────────────────────────────────────────────────────────────
CREATE INDEX IF NOT EXISTS idx_health_profile_food_role
    ON product_health_profile(food_role);
