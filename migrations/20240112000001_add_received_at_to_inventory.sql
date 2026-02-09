-- Add received_at (дата поступления) to inventory_products
-- This allows tracking when product was received/purchased

ALTER TABLE inventory_products 
ADD COLUMN received_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

-- Index for queries by received date
CREATE INDEX idx_inventory_products_received ON inventory_products(received_at);

-- Comment for documentation
COMMENT ON COLUMN inventory_products.received_at IS 'Product receipt/purchase date (дата поступления)';
