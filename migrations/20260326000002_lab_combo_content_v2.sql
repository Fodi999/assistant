-- Add rich content fields to lab_combo_pages for SEO depth:
--   why_it_works: explanation of why this ingredient combo works
--   how_to_cook:  JSON array of cooking steps
--   optimization_tips: JSON array of tips for adjusting the recipe
--   image_url:    optional hero image

ALTER TABLE lab_combo_pages
  ADD COLUMN IF NOT EXISTS why_it_works TEXT NOT NULL DEFAULT '',
  ADD COLUMN IF NOT EXISTS how_to_cook  JSONB NOT NULL DEFAULT '[]'::jsonb,
  ADD COLUMN IF NOT EXISTS optimization_tips JSONB NOT NULL DEFAULT '[]'::jsonb,
  ADD COLUMN IF NOT EXISTS image_url TEXT;
