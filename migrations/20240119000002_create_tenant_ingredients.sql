-- Create tenant-specific ingredient catalog
-- Migration: 20240119000002

-- 🎯 Tenant Ingredients: User-specific pricing, suppliers, and settings
CREATE TABLE IF NOT EXISTS tenant_ingredients (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    catalog_ingredient_id UUID NOT NULL REFERENCES catalog_ingredients(id) ON DELETE RESTRICT,
    
    -- Tenant-specific data
    price DECIMAL(10,2),  -- User's price from their supplier
    supplier VARCHAR(255),  -- Where they buy it
    custom_unit unit_type,  -- Override default unit if needed
    custom_expiration_days INTEGER,  -- Override default shelf life
    notes TEXT,  -- User's personal notes
    
    -- Metadata
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    
    -- Ensure tenant can only add ingredient once
    CONSTRAINT tenant_ingredient_unique UNIQUE (tenant_id, catalog_ingredient_id)
);

-- Indexes for performance
CREATE INDEX idx_tenant_ingredients_tenant ON tenant_ingredients(tenant_id) WHERE is_active = true;
CREATE INDEX idx_tenant_ingredients_catalog ON tenant_ingredients(catalog_ingredient_id);

-- Comments
COMMENT ON TABLE tenant_ingredients IS 'Tenant-specific ingredient data: prices, suppliers, custom settings. Links master catalog to tenant inventory.';
COMMENT ON COLUMN tenant_ingredients.price IS 'Price from tenant''s supplier (can be NULL if not set yet)';
COMMENT ON COLUMN tenant_ingredients.supplier IS 'Supplier name (Metro, Selgros, local market, etc.)';
COMMENT ON COLUMN tenant_ingredients.custom_unit IS 'Overrides catalog default_unit if tenant buys in different unit';
COMMENT ON COLUMN tenant_ingredients.custom_expiration_days IS 'Overrides catalog default_shelf_life_days for this tenant';

-- Trigger for updated_at
CREATE OR REPLACE FUNCTION update_tenant_ingredients_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER tenant_ingredients_updated_at
    BEFORE UPDATE ON tenant_ingredients
    FOR EACH ROW
    EXECUTE FUNCTION update_tenant_ingredients_updated_at();
