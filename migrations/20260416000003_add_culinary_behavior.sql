-- ══════════════════════════════════════════════════════════════════════════════
-- CULINARY BEHAVIOR (i18n)
-- Migration: 20260416000003_add_culinary_behavior.sql
--
-- Describes HOW an ingredient behaves during cooking:
--   "softens quickly", "caramelizes well", "adds sweet-sour accent",
--   "good for sauces, jams, baking"
-- ══════════════════════════════════════════════════════════════════════════════

CREATE TABLE IF NOT EXISTS product_culinary_behavior (
    product_id              UUID PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,

    -- JSONB array of behavior strings in each language
    behaviors_en            JSONB DEFAULT '[]'::jsonb,
    behaviors_ru            JSONB DEFAULT '[]'::jsonb,
    behaviors_pl            JSONB DEFAULT '[]'::jsonb,
    behaviors_uk            JSONB DEFAULT '[]'::jsonb
);
