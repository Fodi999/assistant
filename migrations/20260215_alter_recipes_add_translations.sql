-- Add fields for recipe translations and publishing
-- Migration: 20260215_add_recipe_translations_and_publishing
-- Date: 2026-02-15

-- ============================================================
-- 1️⃣ ALTER RECIPES TABLE - Add new fields
-- ============================================================

-- Add default language fields
ALTER TABLE recipes 
ADD COLUMN IF NOT EXISTS name_default TEXT,
ADD COLUMN IF NOT EXISTS instructions_default TEXT,
ADD COLUMN IF NOT EXISTS language_default VARCHAR(5) CHECK (language_default IN ('ru', 'en', 'pl', 'uk'));

-- Add time fields
ALTER TABLE recipes
ADD COLUMN IF NOT EXISTS prep_time_minutes INTEGER CHECK (prep_time_minutes >= 0),
ADD COLUMN IF NOT EXISTS cook_time_minutes INTEGER CHECK (cook_time_minutes >= 0);

-- Add cost fields
ALTER TABLE recipes
ADD COLUMN IF NOT EXISTS total_cost_cents BIGINT DEFAULT 0 CHECK (total_cost_cents >= 0),
ADD COLUMN IF NOT EXISTS cost_per_serving_cents BIGINT DEFAULT 0 CHECK (cost_per_serving_cents >= 0);

-- Add publishing fields
ALTER TABLE recipes
ADD COLUMN IF NOT EXISTS is_public BOOLEAN DEFAULT FALSE,
ADD COLUMN IF NOT EXISTS published_at TIMESTAMPTZ;

-- Migrate existing data: copy 'name' to 'name_default' and set default language
UPDATE recipes 
SET name_default = name,
    instructions_default = '',
    language_default = 'en',
    total_cost_cents = 0,
    cost_per_serving_cents = 0,
    is_public = FALSE
WHERE name_default IS NULL;

-- Now make name_default NOT NULL
ALTER TABLE recipes
ALTER COLUMN name_default SET NOT NULL,
ALTER COLUMN instructions_default SET NOT NULL,
ALTER COLUMN language_default SET NOT NULL,
ALTER COLUMN total_cost_cents SET NOT NULL,
ALTER COLUMN cost_per_serving_cents SET NOT NULL;

-- Add indexes for new fields
CREATE INDEX IF NOT EXISTS idx_recipes_is_public ON recipes(is_public) WHERE is_public = true;
CREATE INDEX IF NOT EXISTS idx_recipes_published_at ON recipes(published_at) WHERE is_public = true;
CREATE INDEX IF NOT EXISTS idx_recipes_language ON recipes(language_default);

-- ============================================================
-- 2️⃣ CREATE RECIPE_TRANSLATIONS TABLE
-- ============================================================

CREATE TABLE IF NOT EXISTS recipe_translations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    recipe_id UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    
    language VARCHAR(5) NOT NULL CHECK (language IN ('ru', 'en', 'pl', 'uk')),
    name TEXT NOT NULL,
    instructions TEXT NOT NULL,
    
    -- Metadata
    translated_by VARCHAR(20) DEFAULT 'ai' CHECK (translated_by IN ('ai', 'human')),
    translated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Unique constraint: one translation per language per recipe
    CONSTRAINT recipe_translations_unique UNIQUE (recipe_id, language)
);

-- Indexes for fast lookup
CREATE INDEX IF NOT EXISTS idx_recipe_translations_recipe_id ON recipe_translations(recipe_id);
CREATE INDEX IF NOT EXISTS idx_recipe_translations_language ON recipe_translations(recipe_id, language);

-- ============================================================
-- 3️⃣ ALTER RECIPE_INGREDIENTS - Add cost snapshot
-- ============================================================

ALTER TABLE recipe_ingredients
ADD COLUMN IF NOT EXISTS cost_per_unit_cents BIGINT DEFAULT 0 CHECK (cost_per_unit_cents >= 0),
ADD COLUMN IF NOT EXISTS total_cost_cents BIGINT DEFAULT 0 CHECK (total_cost_cents >= 0),
ADD COLUMN IF NOT EXISTS display_order INTEGER DEFAULT 0;

-- Set default values for existing records
UPDATE recipe_ingredients
SET cost_per_unit_cents = 0,
    total_cost_cents = 0,
    display_order = 0
WHERE cost_per_unit_cents IS NULL;

-- Make columns NOT NULL
ALTER TABLE recipe_ingredients
ALTER COLUMN cost_per_unit_cents SET NOT NULL,
ALTER COLUMN total_cost_cents SET NOT NULL,
ALTER COLUMN display_order SET NOT NULL;

-- ============================================================
-- 4️⃣ COMMENTS
-- ============================================================

COMMENT ON COLUMN recipes.name_default IS 'Recipe name in user default language';
COMMENT ON COLUMN recipes.instructions_default IS 'Recipe instructions in user default language';
COMMENT ON COLUMN recipes.language_default IS 'User primary language (ru/en/pl/uk)';
COMMENT ON COLUMN recipes.total_cost_cents IS 'Total recipe cost in cents';
COMMENT ON COLUMN recipes.cost_per_serving_cents IS 'Cost per serving in cents';
COMMENT ON COLUMN recipes.is_public IS 'Is recipe published to public feed';
COMMENT ON COLUMN recipes.published_at IS 'When recipe was published';

COMMENT ON TABLE recipe_translations IS 'AI-generated translations for recipes';
COMMENT ON COLUMN recipe_translations.translated_by IS 'Translation source: ai or human';

COMMENT ON COLUMN recipe_ingredients.cost_per_unit_cents IS 'Snapshot of price at time of adding';
COMMENT ON COLUMN recipe_ingredients.total_cost_cents IS 'Calculated cost for this ingredient';
COMMENT ON COLUMN recipe_ingredients.display_order IS 'Order of ingredient in recipe';
