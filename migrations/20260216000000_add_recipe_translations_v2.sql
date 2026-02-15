-- Recipe Translations Migration
-- Phase 1: Add translation support to recipes (NON-BREAKING)
-- Date: 2026-02-15

-- ============================================================
-- 1️⃣ ADD NEW COLUMNS (NON-BREAKING)
-- ============================================================

-- Add default language content fields
ALTER TABLE recipes 
ADD COLUMN IF NOT EXISTS name_default TEXT,
ADD COLUMN IF NOT EXISTS instructions_default TEXT,
ADD COLUMN IF NOT EXISTS language_default VARCHAR(5) DEFAULT 'en' CHECK (language_default IN ('en', 'ru', 'pl', 'uk'));

-- Add publishing fields
ALTER TABLE recipes
ADD COLUMN IF NOT EXISTS status VARCHAR(20) DEFAULT 'draft' CHECK (status IN ('draft', 'published', 'archived')),
ADD COLUMN IF NOT EXISTS is_public BOOLEAN DEFAULT FALSE,
ADD COLUMN IF NOT EXISTS published_at TIMESTAMPTZ;

-- Add cost tracking fields
ALTER TABLE recipes
ADD COLUMN IF NOT EXISTS total_cost_cents BIGINT DEFAULT 0 CHECK (total_cost_cents >= 0),
ADD COLUMN IF NOT EXISTS cost_per_serving_cents BIGINT DEFAULT 0 CHECK (cost_per_serving_cents >= 0);

-- ============================================================
-- 2️⃣ MIGRATE EXISTING DATA (SAFE)
-- ============================================================

-- Copy existing 'name' to 'name_default' for recipes that don't have it yet
UPDATE recipes 
SET 
    name_default = name,
    instructions_default = COALESCE(instructions, ''),
    language_default = 'en'
WHERE name_default IS NULL;

-- ============================================================
-- 3️⃣ CREATE TRANSLATIONS TABLE
-- ============================================================

CREATE TABLE IF NOT EXISTS recipe_translations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    recipe_id UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    language VARCHAR(5) NOT NULL CHECK (language IN ('en', 'ru', 'pl', 'uk')),
    name TEXT NOT NULL,
    instructions TEXT NOT NULL,
    translated_by VARCHAR(20) NOT NULL CHECK (translated_by IN ('ai', 'human')),
    translated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Ensure one translation per language per recipe
    UNIQUE(recipe_id, language),
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- 4️⃣ ADD INDEXES FOR PERFORMANCE
-- ============================================================

-- Index for finding translations by recipe
CREATE INDEX IF NOT EXISTS idx_recipe_translations_recipe_id 
ON recipe_translations(recipe_id);

-- Index for finding translations by language
CREATE INDEX IF NOT EXISTS idx_recipe_translations_language 
ON recipe_translations(language);

-- Index for public recipes
CREATE INDEX IF NOT EXISTS idx_recipes_is_public 
ON recipes(is_public) 
WHERE is_public = TRUE;

-- Index for published recipes
CREATE INDEX IF NOT EXISTS idx_recipes_published_at 
ON recipes(published_at) 
WHERE is_public = TRUE;

-- ============================================================
-- 5️⃣ COMMENTS FOR DOCUMENTATION
-- ============================================================

COMMENT ON COLUMN recipes.name_default IS 'Original recipe name in default language';
COMMENT ON COLUMN recipes.instructions_default IS 'Original recipe instructions in default language';
COMMENT ON COLUMN recipes.language_default IS 'Language of the original recipe (en/ru/pl/uk)';
COMMENT ON COLUMN recipes.is_public IS 'Whether recipe is published to public feed';
COMMENT ON COLUMN recipes.published_at IS 'When recipe was published (NULL = not published)';

COMMENT ON TABLE recipe_translations IS 'AI/human translations of recipes to other languages';
COMMENT ON COLUMN recipe_translations.translated_by IS 'Source of translation: ai (Groq) or human (manual)';

-- ============================================================
-- ✅ VERIFICATION
-- ============================================================

-- Check migration worked
DO $$
BEGIN
    -- Verify columns exist
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'recipes' AND column_name = 'name_default'
    ) THEN
        RAISE EXCEPTION 'Migration failed: name_default column not created';
    END IF;
    
    -- Verify table exists
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.tables 
        WHERE table_name = 'recipe_translations'
    ) THEN
        RAISE EXCEPTION 'Migration failed: recipe_translations table not created';
    END IF;
    
    RAISE NOTICE 'Recipe translation migration completed successfully ✅';
END $$;
