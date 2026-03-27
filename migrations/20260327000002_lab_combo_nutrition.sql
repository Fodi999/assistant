-- Add pre-calculated nutrition columns to lab_combo_pages.
-- These are the SINGLE SOURCE OF TRUTH for all nutrition display.
-- Calculated by backend from known ingredient data (USDA-based).
-- AI never calculates numbers — only uses these values in text.
--
-- total_* = full recipe
-- *_per_serving = total / servings_count
-- serving_weight_g = total_weight_g / servings_count

ALTER TABLE lab_combo_pages
  ADD COLUMN IF NOT EXISTS total_weight_g        REAL NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS servings_count        SMALLINT NOT NULL DEFAULT 1,
  ADD COLUMN IF NOT EXISTS calories_total        REAL NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS protein_total         REAL NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS fat_total             REAL NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS carbs_total           REAL NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS fiber_total           REAL NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS calories_per_serving  REAL NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS protein_per_serving   REAL NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS fat_per_serving       REAL NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS carbs_per_serving     REAL NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS fiber_per_serving     REAL NOT NULL DEFAULT 0;
