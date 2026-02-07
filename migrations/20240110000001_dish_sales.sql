-- Create dish_sales table for tracking sales performance
-- Used for Menu Engineering analysis (Star/Plowhorse/Puzzle/Dog classification)

CREATE TABLE IF NOT EXISTS dish_sales (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    dish_id UUID NOT NULL,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    
    -- Sale details
    quantity INT NOT NULL DEFAULT 1,
    selling_price_cents INT NOT NULL,
    recipe_cost_cents INT NOT NULL,
    profit_cents INT NOT NULL,
    
    -- Timestamps
    sold_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    CONSTRAINT dish_sales_quantity_positive CHECK (quantity > 0),
    CONSTRAINT dish_sales_selling_price_positive CHECK (selling_price_cents > 0),
    CONSTRAINT dish_sales_recipe_cost_non_negative CHECK (recipe_cost_cents >= 0)
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_dish_sales_tenant_id ON dish_sales(tenant_id);
CREATE INDEX IF NOT EXISTS idx_dish_sales_dish_id ON dish_sales(dish_id);
CREATE INDEX IF NOT EXISTS idx_dish_sales_sold_at ON dish_sales(sold_at DESC);
CREATE INDEX IF NOT EXISTS idx_dish_sales_tenant_dish ON dish_sales(tenant_id, dish_id);

-- Composite index for analytics queries
CREATE INDEX IF NOT EXISTS idx_dish_sales_analytics 
    ON dish_sales(tenant_id, sold_at DESC, dish_id);

COMMENT ON TABLE dish_sales IS 'Sales tracking for Menu Engineering analysis';
COMMENT ON COLUMN dish_sales.quantity IS 'Number of servings sold in this transaction';
COMMENT ON COLUMN dish_sales.profit_cents IS 'Calculated as (selling_price - recipe_cost) * quantity';
