-- ============================================================
-- Migration: ingredient_states
-- Product states system (raw, boiled, fried, etc.)
-- ============================================================

-- 1. Create enum type for processing states
DO $$ BEGIN
    CREATE TYPE processing_state AS ENUM (
        'raw',
        'boiled',
        'fried',
        'baked',
        'grilled',
        'steamed',
        'smoked',
        'frozen',
        'dried',
        'pickled'
    );
EXCEPTION
    WHEN duplicate_object THEN NULL;
END $$;

-- 2. Create ingredient_states table
CREATE TABLE IF NOT EXISTS ingredient_states (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    ingredient_id   UUID NOT NULL REFERENCES catalog_ingredients(id) ON DELETE CASCADE,
    state           processing_state NOT NULL,

    -- Nutrition per 100g (recalculated from base)
    calories_per_100g   DECIMAL(8,2),
    protein_per_100g    DECIMAL(8,2),
    fat_per_100g        DECIMAL(8,2),
    carbs_per_100g      DECIMAL(8,2),
    fiber_per_100g      DECIMAL(8,2),
    water_percent       DECIMAL(5,2),

    -- Storage
    shelf_life_hours    INT,
    storage_temp_c      INT,

    -- Culinary
    texture             VARCHAR(100),
    notes               TEXT,

    -- Translations (state-specific descriptions)
    name_suffix_en      VARCHAR(100),  -- e.g. "boiled", "fried"
    name_suffix_pl      VARCHAR(100),  -- e.g. "gotowany", "smażony"
    name_suffix_ru      VARCHAR(100),  -- e.g. "варёный", "жареный"
    name_suffix_uk      VARCHAR(100),  -- e.g. "варений", "смажений"

    notes_en            TEXT,
    notes_pl            TEXT,
    notes_ru            TEXT,
    notes_uk            TEXT,

    -- Metadata
    generated_by        VARCHAR(50) DEFAULT 'rule_bot',  -- 'rule_bot' | 'manual' | 'ai'
    data_score          DECIMAL(5,2) DEFAULT 0,          -- completeness 0-100%
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Unique: one state per ingredient
    CONSTRAINT uq_ingredient_state UNIQUE (ingredient_id, state)
);

-- 3. Indexes
CREATE INDEX IF NOT EXISTS idx_ingredient_states_ingredient ON ingredient_states(ingredient_id);
CREATE INDEX IF NOT EXISTS idx_ingredient_states_state ON ingredient_states(state);

-- 4. Updated_at trigger
CREATE OR REPLACE FUNCTION update_ingredient_states_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_ingredient_states_updated ON ingredient_states;
CREATE TRIGGER trg_ingredient_states_updated
    BEFORE UPDATE ON ingredient_states
    FOR EACH ROW
    EXECUTE FUNCTION update_ingredient_states_timestamp();
