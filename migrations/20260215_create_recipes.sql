-- 🍳 Recipes System - Private & Public Recipes with Translations
-- Date: 2026-02-15

-- ============================================================
-- 1️⃣ RECIPES TABLE (Main recipe data)
-- ============================================================
CREATE TABLE IF NOT EXISTS recipes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,

    -- Default language content (user's primary language)
    name_default TEXT NOT NULL,
    instructions_default TEXT NOT NULL,
    language_default VARCHAR(5) NOT NULL CHECK (language_default IN ('ru', 'en', 'pl', 'uk')),

    -- Recipe metadata
    servings INTEGER NOT NULL CHECK (servings >= 1),
    prep_time_minutes INTEGER CHECK (prep_time_minutes >= 0),
    cook_time_minutes INTEGER CHECK (cook_time_minutes >= 0),

    -- Cost calculation (in cents)
    total_cost_cents BIGINT NOT NULL CHECK (total_cost_cents >= 0),
    cost_per_serving_cents BIGINT NOT NULL CHECK (cost_per_serving_cents >= 0),

    -- Publishing
    is_public BOOLEAN DEFAULT FALSE,
    published_at TIMESTAMP,

    -- Timestamps
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),

    -- Indexes
    CONSTRAINT recipes_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id),
    CONSTRAINT recipes_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES tenants(id)
);

-- Indexes for performance
CREATE INDEX idx_recipes_tenant_id ON recipes(tenant_id);
CREATE INDEX idx_recipes_user_id ON recipes(user_id);
CREATE INDEX idx_recipes_is_public ON recipes(is_public) WHERE is_public = true;
CREATE INDEX idx_recipes_published_at ON recipes(published_at) WHERE is_public = true;

-- ============================================================
-- 2️⃣ RECIPE TRANSLATIONS (AI-generated translations)
-- ============================================================
CREATE TABLE IF NOT EXISTS recipe_translations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    recipe_id UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,

    language VARCHAR(5) NOT NULL CHECK (language IN ('ru', 'en', 'pl', 'uk')),
    name TEXT NOT NULL,
    instructions TEXT NOT NULL,

    -- Metadata
    translated_by VARCHAR(20) DEFAULT 'ai', -- 'ai' | 'human'
    translated_at TIMESTAMP NOT NULL DEFAULT NOW(),

    -- Unique constraint: one translation per language per recipe
    CONSTRAINT recipe_translations_unique UNIQUE (recipe_id, language)
);

-- Index for fast lookup
CREATE INDEX idx_recipe_translations_recipe_id ON recipe_translations(recipe_id);
CREATE INDEX idx_recipe_translations_language ON recipe_translations(recipe_id, language);

-- ============================================================
-- 3️⃣ RECIPE INGREDIENTS (Links to inventory products)
-- ============================================================
CREATE TABLE IF NOT EXISTS recipe_ingredients (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    recipe_id UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    inventory_product_id UUID NOT NULL REFERENCES inventory_products(id) ON DELETE RESTRICT,

    -- Quantity used in recipe
    quantity NUMERIC(10, 3) NOT NULL CHECK (quantity > 0),

    -- Cost snapshot (at time of adding to recipe)
    cost_per_unit_cents BIGINT NOT NULL CHECK (cost_per_unit_cents >= 0),
    total_cost_cents BIGINT NOT NULL CHECK (total_cost_cents >= 0),

    -- Order in recipe
    display_order INTEGER NOT NULL DEFAULT 0,

    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_recipe_ingredients_recipe_id ON recipe_ingredients(recipe_id);
CREATE INDEX idx_recipe_ingredients_inventory_product_id ON recipe_ingredients(inventory_product_id);

-- ============================================================
-- 4️⃣ TRIGGER: Auto-update updated_at
-- ============================================================
CREATE OR REPLACE FUNCTION update_recipes_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_recipes_updated_at
    BEFORE UPDATE ON recipes
    FOR EACH ROW
    EXECUTE FUNCTION update_recipes_updated_at();

-- ============================================================
-- 5️⃣ COMMENTS
-- ============================================================
COMMENT ON TABLE recipes IS 'User recipes with default language content';
COMMENT ON TABLE recipe_translations IS 'AI-generated translations for recipes';
COMMENT ON TABLE recipe_ingredients IS 'Ingredients used in recipes with cost snapshots';

COMMENT ON COLUMN recipes.language_default IS 'User primary language (ru/en/pl/uk)';
COMMENT ON COLUMN recipes.total_cost_cents IS 'Total recipe cost in cents';
COMMENT ON COLUMN recipes.cost_per_serving_cents IS 'Cost per serving in cents';
COMMENT ON COLUMN recipes.is_public IS 'Is recipe published to public feed';

COMMENT ON COLUMN recipe_translations.translated_by IS 'Translation source: ai or human';
COMMENT ON COLUMN recipe_ingredients.cost_per_unit_cents IS 'Snapshot of price at time of adding';
