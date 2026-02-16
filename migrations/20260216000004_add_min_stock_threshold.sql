-- Add min_stock_threshold to catalog_ingredients
-- This is used for Low Stock Alerts in the inventory system
-- Date: 2026-02-16

ALTER TABLE catalog_ingredients 
ADD COLUMN IF NOT EXISTS min_stock_threshold NUMERIC DEFAULT 0;

COMMENT ON COLUMN catalog_ingredients.min_stock_threshold IS 'Minimum stock quantity before a Low Stock Alert is triggered';
