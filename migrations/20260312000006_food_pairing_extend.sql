-- ════════════════════════════════════════════════════════════════════════
-- Migration: Extend food_pairing with pairing_type for admin CRUD
-- Adds: id (UUID PK), pairing_type enum column
-- Existing data gets pairing_type derived from pair_score
-- ════════════════════════════════════════════════════════════════════════

-- 1. Add id column (UUID) for individual row addressing
ALTER TABLE food_pairing ADD COLUMN IF NOT EXISTS id UUID DEFAULT gen_random_uuid();

-- 2. Add pairing_type column: primary / secondary / experimental / avoid
ALTER TABLE food_pairing ADD COLUMN IF NOT EXISTS pairing_type TEXT NOT NULL DEFAULT 'primary';

-- 3. Backfill pairing_type based on existing pair_score
--    >= 8 → primary, >= 5 → secondary, else → experimental
UPDATE food_pairing SET pairing_type = CASE
    WHEN pair_score >= 8.0 THEN 'primary'
    WHEN pair_score >= 5.0 THEN 'secondary'
    ELSE 'experimental'
END
WHERE pairing_type = 'primary'; -- only update rows that still have the default

-- 4. Index on id for DELETE by id
CREATE INDEX IF NOT EXISTS idx_food_pairing_id ON food_pairing(id);

-- 5. Index on pairing_type for filtered queries
CREATE INDEX IF NOT EXISTS idx_food_pairing_type ON food_pairing(pairing_type);

COMMENT ON COLUMN food_pairing.pairing_type IS 'primary | secondary | experimental | avoid';
COMMENT ON COLUMN food_pairing.id IS 'Unique row ID for admin CRUD delete';
