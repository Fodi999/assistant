-- ========================================
-- STEP 1: Extend recipes with recipe_type and instructions
-- ========================================
-- Recipe type: 'preparation' (полуфабрикат) or 'final' (готовое блюдо)
ALTER TABLE recipes
ADD COLUMN recipe_type TEXT NOT NULL DEFAULT 'final'
CHECK (recipe_type IN ('preparation', 'final'));

-- Preparation instructions (technology description)
ALTER TABLE recipes
ADD COLUMN instructions TEXT;

CREATE INDEX idx_recipes_type ON recipes(recipe_type);

COMMENT ON COLUMN recipes.recipe_type IS 'Type: preparation (полуфабрикат) or final (готовое блюдо)';
COMMENT ON COLUMN recipes.instructions IS 'Preparation instructions / technology description (steps, notes, HACCP)';

-- ========================================
-- STEP 2: Recipe components (рецепт может включать другие рецепты)
-- ========================================
-- This is the KEY table for semi-finished products support
CREATE TABLE recipe_components (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    -- Recipe that includes another recipe
    recipe_id UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    
    -- Recipe being included (полуфабрикат)
    component_recipe_id UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    
    -- Quantity of the component recipe needed
    quantity NUMERIC(10,3) NOT NULL CHECK (quantity > 0),
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    
    -- Business constraints
    CONSTRAINT recipe_cannot_include_itself
        CHECK (recipe_id <> component_recipe_id),
    
    CONSTRAINT unique_component_per_recipe
        UNIQUE (recipe_id, component_recipe_id)
);

-- Indexes for recursive cost calculation
CREATE INDEX idx_recipe_components_recipe_id ON recipe_components(recipe_id);
CREATE INDEX idx_recipe_components_component_id ON recipe_components(component_recipe_id);

COMMENT ON TABLE recipe_components IS 'Recipes can include other recipes (полуфабрикаты)';
COMMENT ON COLUMN recipe_components.quantity IS 'Fraction of full recipe batch used (1.0 = 100% of recipe yield)';

-- ========================================
-- STEP 3: Dishes table (menu items with selling prices)
-- ========================================
-- This is where "владелец ресторана видит деньги" - profit and margins!
CREATE TABLE dishes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    
    -- Only 'final' recipes can be dishes
    recipe_id UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    
    name VARCHAR(255) NOT NULL,
    description TEXT,
    
    -- Selling price (цена продажи)
    selling_price_cents INTEGER NOT NULL CHECK (selling_price_cents > 0),
    
    active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    
    -- Business constraints
    CONSTRAINT unique_dish_name_per_tenant UNIQUE (tenant_id, name)
);

-- Indexes for queries
CREATE INDEX idx_dishes_tenant_id ON dishes(tenant_id);
CREATE INDEX idx_dishes_recipe_id ON dishes(recipe_id);
CREATE INDEX idx_dishes_active ON dishes(active) WHERE active = true;

-- Updated_at trigger
CREATE TRIGGER set_dishes_updated_at
    BEFORE UPDATE ON dishes
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Comments for documentation
COMMENT ON TABLE dishes IS 'Menu dishes with selling prices for profit/margin analysis';
COMMENT ON COLUMN dishes.selling_price_cents IS 'Selling price in cents (e.g., 1500 = 15.00 PLN)';
COMMENT ON COLUMN dishes.active IS 'Whether dish is currently available in menu';
COMMENT ON COLUMN dishes.recipe_id IS 'Links to final recipe for cost calculation';
