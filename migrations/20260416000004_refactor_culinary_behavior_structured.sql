-- ══════════════════════════════════════════════════════════════════════════════
-- REFACTOR: culinary_behavior → structured JSONB
-- Migration: 20260416000004_refactor_culinary_behavior_structured.sql
--
-- Instead of simple text arrays per language, store a SINGLE structured array:
-- [
--   { "key": "softens_quickly", "type": "texture", "effect": "softening",
--     "trigger": "heat", "intensity": 0.8, "temp_threshold": null, "targets": [] },
--   ...
-- ]
--
-- Frontend renders labels via i18n dictionary keyed by `key`.
-- Recipe engine can query: ingredient.has("softens_quickly")
-- ══════════════════════════════════════════════════════════════════════════════

-- Drop old text-based columns if they exist
ALTER TABLE product_culinary_behavior
    DROP COLUMN IF EXISTS behaviors_en,
    DROP COLUMN IF EXISTS behaviors_ru,
    DROP COLUMN IF EXISTS behaviors_pl,
    DROP COLUMN IF EXISTS behaviors_uk;

-- Add single structured column
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'product_culinary_behavior' AND column_name = 'behaviors'
    ) THEN
        ALTER TABLE product_culinary_behavior
            ADD COLUMN behaviors JSONB DEFAULT '[]'::jsonb;
    END IF;
END $$;
