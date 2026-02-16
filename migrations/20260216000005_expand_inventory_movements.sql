-- Expand inventory_movements with more granular types and costs
-- Date: 2026-02-16

DO $$
BEGIN
    -- 1. Update CHECK constraint for movement type
    ALTER TABLE inventory_movements DROP CONSTRAINT IF EXISTS inventory_movements_type_check;
    ALTER TABLE inventory_movements ADD CONSTRAINT inventory_movements_type_check 
        CHECK (type IN ('IN', 'OUT_SALE', 'OUT_EXPIRE', 'ADJUSTMENT', 'OUT'));

    -- 2. Add cost columns (to track financial loss)
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'inventory_movements' AND column_name = 'unit_cost_cents') THEN
        ALTER TABLE inventory_movements ADD COLUMN unit_cost_cents BIGINT NOT NULL DEFAULT 0;
    END IF;

    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'inventory_movements' AND column_name = 'total_cost_cents') THEN
        ALTER TABLE inventory_movements ADD COLUMN total_cost_cents BIGINT NOT NULL DEFAULT 0;
    END IF;

    -- 3. Add reason column
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'inventory_movements' AND column_name = 'reason') THEN
        ALTER TABLE inventory_movements ADD COLUMN reason TEXT;
    END IF;

    -- 4. Migrate old 'OUT' to 'OUT_SALE' (most likely)
    UPDATE inventory_movements SET type = 'OUT_SALE' WHERE type = 'OUT';
    
    -- Now remove 'OUT' from check constraint
    ALTER TABLE inventory_movements DROP CONSTRAINT IF EXISTS inventory_movements_type_check;
    ALTER TABLE inventory_movements ADD CONSTRAINT inventory_movements_type_check 
        CHECK (type IN ('IN', 'OUT_SALE', 'OUT_EXPIRE', 'ADJUSTMENT'));

    RAISE NOTICE 'Expanded inventory_movements schema ✅';
END $$;
