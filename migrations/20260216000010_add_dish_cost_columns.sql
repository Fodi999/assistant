-- Phase 3: Cost Materialization — store calculated costs in dishes table
ALTER TABLE dishes
ADD COLUMN IF NOT EXISTS recipe_cost_cents BIGINT,
ADD COLUMN IF NOT EXISTS food_cost_percent DOUBLE PRECISION,
ADD COLUMN IF NOT EXISTS profit_margin_percent DOUBLE PRECISION,
ADD COLUMN IF NOT EXISTS cost_calculated_at TIMESTAMPTZ;

COMMENT ON COLUMN dishes.recipe_cost_cents IS 'Materialized recipe cost in cents';
COMMENT ON COLUMN dishes.food_cost_percent IS 'Food cost %: (recipe_cost / selling_price) * 100';
COMMENT ON COLUMN dishes.profit_margin_percent IS 'Profit margin %: ((selling_price - recipe_cost) / selling_price) * 100';
COMMENT ON COLUMN dishes.cost_calculated_at IS 'When cost was last calculated';

CREATE INDEX IF NOT EXISTS idx_dishes_profit_margin ON dishes(profit_margin_percent);
