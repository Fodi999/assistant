-- Add status column to ingredient_dictionary
-- Supports: pending (AI suggestion) → active (admin approved)
-- Admin must approve AI translations before they become "truth"

-- Add status column with default 'active' (existing entries are trusted)
ALTER TABLE ingredient_dictionary 
ADD COLUMN IF NOT EXISTS status TEXT NOT NULL DEFAULT 'active';

-- Add source column (who created: manual, ai, import)
ALTER TABLE ingredient_dictionary 
ADD COLUMN IF NOT EXISTS source TEXT NOT NULL DEFAULT 'manual';

-- Add confidence column (AI confidence score 0.0–1.0, null for manual)
ALTER TABLE ingredient_dictionary 
ADD COLUMN IF NOT EXISTS confidence REAL;

-- Add reviewed_at timestamp (when admin approved/rejected)
ALTER TABLE ingredient_dictionary 
ADD COLUMN IF NOT EXISTS reviewed_at TIMESTAMPTZ;

-- Constraint: status must be one of: pending, active, rejected
ALTER TABLE ingredient_dictionary 
ADD CONSTRAINT check_dictionary_status 
CHECK (status IN ('pending', 'active', 'rejected'));

-- Constraint: source must be one of: manual, ai, import
ALTER TABLE ingredient_dictionary 
ADD CONSTRAINT check_dictionary_source 
CHECK (source IN ('manual', 'ai', 'import'));

-- Index for fast pending lookups (admin review page)
CREATE INDEX IF NOT EXISTS idx_dictionary_status 
ON ingredient_dictionary (status) WHERE status = 'pending';

-- Comment
COMMENT ON COLUMN ingredient_dictionary.status IS 'pending = AI suggestion awaiting review, active = approved, rejected = declined';
COMMENT ON COLUMN ingredient_dictionary.source IS 'Who created: manual (admin), ai (auto-translated), import (bulk)';
COMMENT ON COLUMN ingredient_dictionary.confidence IS 'AI confidence score 0.0-1.0, null for manual entries';
COMMENT ON COLUMN ingredient_dictionary.reviewed_at IS 'When admin approved or rejected this entry';
