-- Professional Batch-based Inventory System
-- Implementation of 2-level model: Catalog -> Batches
-- Includes FIFO support and Audit Log (Movements)
-- Date: 2026-02-16

DO $$
BEGIN
    -- 1. Rename inventory_products to inventory_batches (if exists)
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'inventory_products') THEN
        ALTER TABLE inventory_products RENAME TO inventory_batches;
        
        -- Rename indexes for consistency
        EXECUTE 'ALTER INDEX IF EXISTS idx_inventory_products_user RENAME TO idx_inventory_batches_tenant';
        EXECUTE 'ALTER INDEX IF EXISTS idx_inventory_products_catalog RENAME TO idx_inventory_batches_catalog';
        EXECUTE 'ALTER INDEX IF EXISTS idx_inventory_products_expires RENAME TO idx_inventory_batches_expires';
        
        -- Rename trigger
        EXECUTE 'ALTER TRIGGER set_inventory_products_updated_at ON inventory_batches RENAME TO set_inventory_batches_updated_at';
    END IF;

    -- 2. Add professional fields to inventory_batches
    -- Add columns with safe ALTER TABLE
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'inventory_batches' AND column_name = 'remaining_quantity') THEN
        ALTER TABLE inventory_batches ADD COLUMN remaining_quantity NUMERIC NOT NULL DEFAULT 0;
        
        -- Migrate existing quantity to remaining_quantity
        UPDATE inventory_batches SET remaining_quantity = quantity;
    END IF;

    -- Ensure quantity is NUMERIC for precision
    ALTER TABLE inventory_batches ALTER COLUMN quantity TYPE NUMERIC USING quantity::numeric;
    ALTER TABLE inventory_batches ALTER COLUMN remaining_quantity TYPE NUMERIC USING remaining_quantity::numeric;

    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'inventory_batches' AND column_name = 'supplier') THEN
        ALTER TABLE inventory_batches ADD COLUMN supplier TEXT;
    END IF;

    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'inventory_batches' AND column_name = 'invoice_number') THEN
        ALTER TABLE inventory_batches ADD COLUMN invoice_number TEXT;
    END IF;

    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'inventory_batches' AND column_name = 'received_at') THEN
        -- inventory_products had received_at, but just in case
        ALTER TABLE inventory_batches ADD COLUMN received_at TIMESTAMPTZ NOT NULL DEFAULT NOW();
    END IF;
    
    -- Ensure status column for batches (active, exhausted, archived)
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'inventory_batches' AND column_name = 'status') THEN
        ALTER TABLE inventory_batches ADD COLUMN status VARCHAR(20) DEFAULT 'active' CHECK (status IN ('active', 'exhausted', 'archived'));
    END IF;

    -- 3. Create inventory_movements table for audit & history
    CREATE TABLE IF NOT EXISTS inventory_movements (
        id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
        tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
        batch_id UUID NOT NULL REFERENCES inventory_batches(id) ON DELETE CASCADE,
        
        type VARCHAR(20) NOT NULL CHECK (type IN ('IN', 'OUT', 'ADJUSTMENT')),
        quantity NUMERIC NOT NULL,
        
        -- Reference to sale_id, invoice_id, etc.
        reference_id UUID,
        reference_type VARCHAR(50), -- 'sale', 'purchase', 'waste', 'manual'
        
        notes TEXT,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    );

    CREATE INDEX IF NOT EXISTS idx_inventory_movements_tenant ON inventory_movements(tenant_id);
    CREATE INDEX IF NOT EXISTS idx_inventory_movements_batch ON inventory_movements(batch_id);
    CREATE INDEX IF NOT EXISTS idx_inventory_movements_created ON inventory_movements(created_at);

    RAISE NOTICE 'Inventory batch-based architecture migration completed successfully ✅';
END $$;
