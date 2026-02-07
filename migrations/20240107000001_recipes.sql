-- Recipe tables migration
-- Stores recipes with ingredients and allows cost calculation

-- Helper function for auto-updating updated_at column
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Recipes table
CREATE TABLE recipes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    servings INTEGER NOT NULL CHECK (servings > 0 AND servings <= 1000),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Indexes for recipes
CREATE INDEX idx_recipes_user ON recipes(user_id, tenant_id);
CREATE INDEX idx_recipes_name ON recipes(user_id, name);

-- Recipe ingredients (junction table)
CREATE TABLE recipe_ingredients (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    recipe_id UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    catalog_ingredient_id UUID NOT NULL REFERENCES catalog_ingredients(id) ON DELETE RESTRICT,
    quantity DOUBLE PRECISION NOT NULL CHECK (quantity > 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    
    -- Ensure no duplicate ingredients in same recipe
    UNIQUE(recipe_id, catalog_ingredient_id)
);

-- Indexes for recipe_ingredients
CREATE INDEX idx_recipe_ingredients_recipe ON recipe_ingredients(recipe_id);
CREATE INDEX idx_recipe_ingredients_catalog ON recipe_ingredients(catalog_ingredient_id);

-- Trigger to update recipes.updated_at when ingredients change
CREATE OR REPLACE FUNCTION update_recipe_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE recipes 
    SET updated_at = now() 
    WHERE id = COALESCE(NEW.recipe_id, OLD.recipe_id);
    RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_recipe_timestamp_on_ingredient_change
AFTER INSERT OR UPDATE OR DELETE ON recipe_ingredients
FOR EACH ROW
EXECUTE FUNCTION update_recipe_timestamp();

-- Trigger to auto-update recipes.updated_at
CREATE TRIGGER trigger_update_recipes_timestamp
BEFORE UPDATE ON recipes
FOR EACH ROW
EXECUTE FUNCTION update_updated_at_column();
