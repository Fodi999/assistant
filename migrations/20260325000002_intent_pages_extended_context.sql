-- Extend recipe_intent context with 3 new dimensions:
--   cooking_time  — "quick" / "medium" / "long"
--   budget        — "cheap" / "medium" / "premium"
--   cuisine       — "italian" / "asian" / "japanese" / etc.
--
-- This expands the SEO page matrix from 96 to thousands of combinations.
-- Frontend reads these fields and pre-fills SmartService widget context.

ALTER TABLE intent_pages ADD COLUMN IF NOT EXISTS cooking_time TEXT;
ALTER TABLE intent_pages ADD COLUMN IF NOT EXISTS budget       TEXT;
ALTER TABLE intent_pages ADD COLUMN IF NOT EXISTS cuisine      TEXT;

-- Expand the composite index for recipe_intent lookups
DROP INDEX IF EXISTS idx_intent_pages_recipe_intent;
CREATE INDEX idx_intent_pages_recipe_intent
    ON intent_pages (intent_type, goal, meal_type, diet, cooking_time, budget, cuisine)
    WHERE intent_type = 'recipe_intent';
