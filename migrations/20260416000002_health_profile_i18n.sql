-- ══════════════════════════════════════════════════════════════════════════════
-- HEALTH PROFILE i18n — add multilingual columns (EN/RU/PL/UK)
-- Migration: 20260416000002_health_profile_i18n.sql
--
-- Existing single-language TEXT/JSONB columns become _en (English),
-- plus new _ru, _pl, _uk columns for Russian, Polish, Ukrainian.
-- ══════════════════════════════════════════════════════════════════════════════

-- ── 1. product_health_profile: rename existing → _en, add _ru/_pl/_uk ─────

-- bioactive_compounds → bioactive_compounds_en
ALTER TABLE product_health_profile RENAME COLUMN bioactive_compounds TO bioactive_compounds_en;
ALTER TABLE product_health_profile ADD COLUMN IF NOT EXISTS bioactive_compounds_ru JSONB DEFAULT '[]'::jsonb;
ALTER TABLE product_health_profile ADD COLUMN IF NOT EXISTS bioactive_compounds_pl JSONB DEFAULT '[]'::jsonb;
ALTER TABLE product_health_profile ADD COLUMN IF NOT EXISTS bioactive_compounds_uk JSONB DEFAULT '[]'::jsonb;

-- health_effects → health_effects_en
ALTER TABLE product_health_profile RENAME COLUMN health_effects TO health_effects_en;
ALTER TABLE product_health_profile ADD COLUMN IF NOT EXISTS health_effects_ru JSONB DEFAULT '[]'::jsonb;
ALTER TABLE product_health_profile ADD COLUMN IF NOT EXISTS health_effects_pl JSONB DEFAULT '[]'::jsonb;
ALTER TABLE product_health_profile ADD COLUMN IF NOT EXISTS health_effects_uk JSONB DEFAULT '[]'::jsonb;

-- contraindications → contraindications_en
ALTER TABLE product_health_profile RENAME COLUMN contraindications TO contraindications_en;
ALTER TABLE product_health_profile ADD COLUMN IF NOT EXISTS contraindications_ru JSONB DEFAULT '[]'::jsonb;
ALTER TABLE product_health_profile ADD COLUMN IF NOT EXISTS contraindications_pl JSONB DEFAULT '[]'::jsonb;
ALTER TABLE product_health_profile ADD COLUMN IF NOT EXISTS contraindications_uk JSONB DEFAULT '[]'::jsonb;

-- absorption_notes → absorption_notes_en
ALTER TABLE product_health_profile RENAME COLUMN absorption_notes TO absorption_notes_en;
ALTER TABLE product_health_profile ADD COLUMN IF NOT EXISTS absorption_notes_ru TEXT;
ALTER TABLE product_health_profile ADD COLUMN IF NOT EXISTS absorption_notes_pl TEXT;
ALTER TABLE product_health_profile ADD COLUMN IF NOT EXISTS absorption_notes_uk TEXT;

-- ── 2. product_processing_effects: rename existing → _en, add _ru/_pl/_uk ──

-- best_cooking_method → best_cooking_method_en
ALTER TABLE product_processing_effects RENAME COLUMN best_cooking_method TO best_cooking_method_en;
ALTER TABLE product_processing_effects ADD COLUMN IF NOT EXISTS best_cooking_method_ru TEXT;
ALTER TABLE product_processing_effects ADD COLUMN IF NOT EXISTS best_cooking_method_pl TEXT;
ALTER TABLE product_processing_effects ADD COLUMN IF NOT EXISTS best_cooking_method_uk TEXT;

-- processing_notes → processing_notes_en
ALTER TABLE product_processing_effects RENAME COLUMN processing_notes TO processing_notes_en;
ALTER TABLE product_processing_effects ADD COLUMN IF NOT EXISTS processing_notes_ru TEXT;
ALTER TABLE product_processing_effects ADD COLUMN IF NOT EXISTS processing_notes_pl TEXT;
ALTER TABLE product_processing_effects ADD COLUMN IF NOT EXISTS processing_notes_uk TEXT;
