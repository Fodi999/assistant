-- Add structured ingredients JSONB to lab_combo_pages.
-- SINGLE SOURCE OF TRUTH for the "Ingredients" UI block on blog + admin.
-- Each element: { slug, name, grams, kcal, protein, fat, carbs, image_url, product_type }
-- Name is localized per page locale. Grams/macros come from catalog_ingredients DB.
-- AI never generates this — it's pure DB data.

ALTER TABLE lab_combo_pages
  ADD COLUMN IF NOT EXISTS structured_ingredients JSONB NOT NULL DEFAULT '[]'::jsonb;

COMMENT ON COLUMN lab_combo_pages.structured_ingredients IS
  'Structured ingredient list with localized names, grams, macros, images. Source: catalog_ingredients DB. AI never touches this.';
