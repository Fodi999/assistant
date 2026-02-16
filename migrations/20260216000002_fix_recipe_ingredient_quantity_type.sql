-- Fix recipe_ingredients quantity type
-- Change from DOUBLE PRECISION to NUMERIC for better precision and compatibility with Rust Decimal
-- Date: 2026-02-16

DO $$
BEGIN
    -- 1. Alter quantity column
    ALTER TABLE recipe_ingredients 
    ALTER COLUMN quantity TYPE NUMERIC USING quantity::numeric;

    -- 2. Add missing columns if they don't exist (safety check for V2)
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'recipe_ingredients' AND column_name = 'unit') THEN
        ALTER TABLE recipe_ingredients ADD COLUMN unit VARCHAR(20) DEFAULT 'piece';
    END IF;

    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'recipe_ingredients' AND column_name = 'cost_at_use_cents') THEN
        ALTER TABLE recipe_ingredients ADD COLUMN cost_at_use_cents BIGINT;
    END IF;

    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'recipe_ingredients' AND column_name = 'catalog_ingredient_name_snapshot') THEN
        ALTER TABLE recipe_ingredients ADD COLUMN catalog_ingredient_name_snapshot TEXT;
    END IF;

    RAISE NOTICE 'recipe_ingredients type fix completed successfully ✅';
END $$;
