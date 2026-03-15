-- ============================================================
-- Migration: Add glycemic_index + cooking_method to ingredient_states
-- Closes the architecture gap:
--   ingredient_states.glycemic_index  — GI per state (e.g. banana raw=51, dried=70)
--   ingredient_states.cooking_method  — specific cooking method enum
-- ============================================================

-- 1. Create cooking_method enum
DO $$ BEGIN
    CREATE TYPE cooking_method_enum AS ENUM (
        'raw',
        'boiled',
        'steamed',
        'baked',
        'grilled',
        'pan_fried',
        'deep_fried',
        'air_fried',
        'smoked',
        'dried',
        'fermented',
        'frozen',
        'pickled'
    );
EXCEPTION
    WHEN duplicate_object THEN NULL;
END $$;

-- 2. Add columns
ALTER TABLE ingredient_states
    ADD COLUMN IF NOT EXISTS glycemic_index SMALLINT,
    ADD COLUMN IF NOT EXISTS cooking_method cooking_method_enum;

-- 3. Comments
COMMENT ON COLUMN ingredient_states.glycemic_index IS 'Glycemic index for this state (0-110). Falls back to food_properties.glycemic_index if NULL';
COMMENT ON COLUMN ingredient_states.cooking_method IS 'Specific cooking method: raw, boiled, steamed, baked, grilled, pan_fried, deep_fried, air_fried, smoked, dried, fermented, frozen, pickled';

-- 4. Backfill cooking_method from state (1:1 mapping for existing states)
UPDATE ingredient_states SET cooking_method = 'raw'     WHERE state = 'raw'     AND cooking_method IS NULL;
UPDATE ingredient_states SET cooking_method = 'boiled'  WHERE state = 'boiled'  AND cooking_method IS NULL;
UPDATE ingredient_states SET cooking_method = 'steamed' WHERE state = 'steamed' AND cooking_method IS NULL;
UPDATE ingredient_states SET cooking_method = 'baked'   WHERE state = 'baked'   AND cooking_method IS NULL;
UPDATE ingredient_states SET cooking_method = 'grilled' WHERE state = 'grilled' AND cooking_method IS NULL;
UPDATE ingredient_states SET cooking_method = 'pan_fried' WHERE state = 'fried' AND cooking_method IS NULL;
UPDATE ingredient_states SET cooking_method = 'smoked'  WHERE state = 'smoked'  AND cooking_method IS NULL;
UPDATE ingredient_states SET cooking_method = 'frozen'  WHERE state = 'frozen'  AND cooking_method IS NULL;
UPDATE ingredient_states SET cooking_method = 'dried'   WHERE state = 'dried'   AND cooking_method IS NULL;
UPDATE ingredient_states SET cooking_method = 'pickled' WHERE state = 'pickled' AND cooking_method IS NULL;
