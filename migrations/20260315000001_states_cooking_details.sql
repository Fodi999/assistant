-- ============================================================
-- Migration: Add cooking detail columns to ingredient_states
-- 1. weight_change_percent — mass change during cooking (e.g. -20% for grilled)
-- 2. state_type — classification (raw / heat / preserved)
-- 3. oil_absorption_g — grams of oil absorbed per 100g (mainly for fried)
-- 4. water_loss_percent — water lost during cooking (e.g. 18% for grilled)
-- ============================================================

ALTER TABLE ingredient_states
    ADD COLUMN IF NOT EXISTS weight_change_percent DECIMAL(6,2),
    ADD COLUMN IF NOT EXISTS state_type VARCHAR(20) DEFAULT 'raw',
    ADD COLUMN IF NOT EXISTS oil_absorption_g DECIMAL(6,2),
    ADD COLUMN IF NOT EXISTS water_loss_percent DECIMAL(6,2);

-- Add comments for documentation
COMMENT ON COLUMN ingredient_states.weight_change_percent IS 'Weight change during processing: negative = loss (e.g. -20 for grilled), positive = gain (e.g. +10 for boiled absorbing water)';
COMMENT ON COLUMN ingredient_states.state_type IS 'Processing category: raw, heat, preserved';
COMMENT ON COLUMN ingredient_states.oil_absorption_g IS 'Grams of oil absorbed per 100g of raw product (mainly for fried/deep-fried)';
COMMENT ON COLUMN ingredient_states.water_loss_percent IS 'Percentage of water lost during processing (0-100)';
