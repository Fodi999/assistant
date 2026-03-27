-- Add process and detail image URLs to lab_combo_pages.
-- Enables 2-3 photo strategy for Google Recipe Rich Results:
--   image_url         = Hero (finished dish, top-down beauty shot)
--   process_image_url = Process (cooking action: searing, plating)
--   detail_image_url  = Detail (raw ingredients, texture close-up)

ALTER TABLE lab_combo_pages
  ADD COLUMN IF NOT EXISTS process_image_url TEXT,
  ADD COLUMN IF NOT EXISTS detail_image_url  TEXT;
