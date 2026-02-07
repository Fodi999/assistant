-- Create inventory_products table
CREATE TABLE inventory_products (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    
    -- Reference to catalog ingredient
    catalog_ingredient_id UUID NOT NULL REFERENCES catalog_ingredients(id) ON DELETE RESTRICT,
    
    -- Financial data (price in smallest currency unit, e.g., cents)
    price_per_unit_cents BIGINT NOT NULL CHECK (price_per_unit_cents >= 0),
    
    -- Quantity (can be decimal for weights/volumes)
    quantity DOUBLE PRECISION NOT NULL CHECK (quantity >= 0),
    
    -- Optional expiration date
    expires_at TIMESTAMPTZ,
    
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Indexes for fast queries
    CONSTRAINT inventory_products_user_tenant CHECK (true)
);

-- Indexes for performance
CREATE INDEX idx_inventory_products_user ON inventory_products(user_id, tenant_id);
CREATE INDEX idx_inventory_products_catalog ON inventory_products(catalog_ingredient_id);
CREATE INDEX idx_inventory_products_expires ON inventory_products(expires_at) WHERE expires_at IS NOT NULL;

-- Trigger for auto-updating updated_at
CREATE TRIGGER set_inventory_products_updated_at
    BEFORE UPDATE ON inventory_products
    FOR EACH ROW
    EXECUTE FUNCTION set_updated_at();

-- Comments for documentation
COMMENT ON TABLE inventory_products IS 'User inventory - purchased ingredients with price and quantity';
COMMENT ON COLUMN inventory_products.price_per_unit_cents IS 'Price per unit in smallest currency unit (cents/grosze)';
COMMENT ON COLUMN inventory_products.quantity IS 'Quantity purchased (can be decimal for kg/liters)';
COMMENT ON COLUMN inventory_products.expires_at IS 'Product expiration date (optional)';
