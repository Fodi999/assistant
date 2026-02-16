-- Enforce mandatory received_at and expires_at for inventory batches
-- Date: 2026-02-16

DO $$
BEGIN
    -- 1. For existing rows without expires_at, set a default (e.g., 1 year from received_at)
    UPDATE inventory_batches 
    SET expires_at = received_at + INTERVAL '1 year'
    WHERE expires_at IS NULL;

    -- 2. Make expires_at MANDATORY
    ALTER TABLE inventory_batches ALTER COLUMN expires_at SET NOT NULL;

    -- 3. Just in case, ensure received_at is also NOT NULL (it should be already)
    ALTER TABLE inventory_batches ALTER COLUMN received_at SET NOT NULL;

    RAISE NOTICE 'Mandatory dates for inventory batches enforced successfully ✅';
END $$;
