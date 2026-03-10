-- Add category to gallery table
ALTER TABLE gallery ADD COLUMN IF NOT EXISTS category TEXT NOT NULL DEFAULT '';

-- Create index for fast filtering
CREATE INDEX IF NOT EXISTS idx_gallery_category ON gallery (category);
