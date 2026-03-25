-- Add recipe_intent page type + context fields (goal, meal_type, diet)
--
-- recipe_intent pages are SEO landing pages that automatically wire
-- goal/meal_type/diet into the SmartService bot:
--
--   /recipes/high-protein-dinner  →  goal=high_protein, meal_type=dinner
--   /recipes/vegan-breakfast      →  goal=balanced, meal_type=breakfast, diet=vegan
--
-- One page = 1000+ recipe variations (user adds ingredients → bot generates).

-- 1. Expand intent_type CHECK to include recipe_intent
ALTER TABLE intent_pages DROP CONSTRAINT IF EXISTS intent_pages_intent_type_check;
ALTER TABLE intent_pages ADD CONSTRAINT intent_pages_intent_type_check
    CHECK (intent_type IN ('question', 'comparison', 'goal', 'combo', 'recipe_intent'));

-- 2. Add context fields (nullable — only used by recipe_intent pages)
ALTER TABLE intent_pages ADD COLUMN IF NOT EXISTS goal      TEXT;
ALTER TABLE intent_pages ADD COLUMN IF NOT EXISTS meal_type TEXT;
ALTER TABLE intent_pages ADD COLUMN IF NOT EXISTS diet      TEXT;

-- 3. Index for fast recipe_intent queries by context
CREATE INDEX IF NOT EXISTS idx_intent_pages_recipe_intent
    ON intent_pages (intent_type, goal, meal_type, diet)
    WHERE intent_type = 'recipe_intent';

COMMENT ON COLUMN intent_pages.goal IS 'SmartService goal: balanced, high_protein, low_calorie, keto, muscle_gain, diet, flavor_boost';
COMMENT ON COLUMN intent_pages.meal_type IS 'Meal occasion: breakfast, lunch, dinner, snack, dessert';
COMMENT ON COLUMN intent_pages.diet IS 'Dietary restriction: vegan, vegetarian, pescatarian, gluten_free, dairy_free, paleo, mediterranean';
