-- Add 'measure' to the allowed intent_type values for GEO layer
-- (weight/volume conversion pages: tablespoon flour grams, etc.)

ALTER TABLE intent_pages DROP CONSTRAINT IF EXISTS intent_pages_intent_type_check;
ALTER TABLE intent_pages ADD CONSTRAINT intent_pages_intent_type_check
    CHECK (intent_type IN ('question', 'comparison', 'goal', 'combo', 'recipe_intent', 'measure'));
