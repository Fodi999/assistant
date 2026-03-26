-- Lab Combo Pages — prerendered SEO landing pages for ingredient combos.
--
-- These are the "clean URL" SEO versions of /lab?q=salmon,rice&goal=high_protein&meal=dinner
-- → /chef-tools/lab/combo/high-protein-dinner-salmon-rice
--
-- Each row stores:
--   • A deterministic slug built from sorted ingredients + optional goal/meal
--   • Precomputed SmartService response (cached JSON, no live call needed)
--   • SEO metadata (title, description, h1, intro text)
--   • Status pipeline: draft → published → archived

CREATE TABLE IF NOT EXISTS lab_combo_pages (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Canonical slug: "high-protein-dinner-salmon-rice-avocado"
    slug            TEXT NOT NULL,

    -- The locale this page is generated for
    locale          TEXT NOT NULL DEFAULT 'en',

    -- Ingredient slugs (sorted alphabetically for dedup)
    ingredients     TEXT[] NOT NULL,

    -- Optional 6D context (same as SmartService)
    goal            TEXT,            -- "high_protein", "keto", etc.
    meal_type       TEXT,            -- "breakfast", "lunch", "dinner"
    diet            TEXT,            -- "vegan", "vegetarian", etc.
    cooking_time    TEXT,            -- "quick", "medium", "long"
    budget          TEXT,            -- "cheap", "premium"
    cuisine         TEXT,            -- "italian", "asian", etc.

    -- SEO metadata
    title           TEXT NOT NULL DEFAULT '',
    description     TEXT NOT NULL DEFAULT '',
    h1              TEXT NOT NULL DEFAULT '',
    intro           TEXT NOT NULL DEFAULT '',

    -- Precomputed SmartService response (full JSON snapshot)
    smart_response  JSONB NOT NULL DEFAULT '{}'::jsonb,

    -- FAQ for structured data / rich snippets
    faq             JSONB NOT NULL DEFAULT '[]'::jsonb,

    -- Status: draft, published, archived
    status          TEXT NOT NULL DEFAULT 'draft',

    -- Quality score (0-5, same as intent_pages)
    quality_score   SMALLINT NOT NULL DEFAULT 0,

    published_at    TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),

    -- Unique per slug+locale (no duplicates)
    CONSTRAINT uq_lab_combo_slug_locale UNIQUE (slug, locale)
);

-- Fast lookups
CREATE INDEX idx_lab_combo_status_locale ON lab_combo_pages (status, locale);
CREATE INDEX idx_lab_combo_ingredients ON lab_combo_pages USING GIN (ingredients);
CREATE INDEX idx_lab_combo_published ON lab_combo_pages (published_at DESC)
    WHERE status = 'published';
