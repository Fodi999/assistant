-- v2: Add structured_ingredients JSONB to lab_combo_pages.
-- Source of truth for the Ingredients UI block (blog + admin).
-- Each element: { slug, name, grams, kcal, protein, fat, carbs, image_url, product_type }
-- Populated by backend from catalog_ingredients DB. AI never touches this.

ALTER TABLE lab_combo_pages
  ADD COLUMN IF NOT EXISTS structured_ingredients JSONB NOT NULL DEFAULT '[]'::jsonb;
