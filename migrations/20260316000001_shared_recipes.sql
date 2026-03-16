-- Shared recipe links: store recipe configs for shareable URLs
CREATE TABLE IF NOT EXISTS shared_recipes (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug            TEXT UNIQUE NOT NULL,
    ingredients_json JSONB NOT NULL,          -- [{slug, grams}, ...]
    portions        INT NOT NULL DEFAULT 1,
    lang            TEXT NOT NULL DEFAULT 'en',
    title           TEXT,                      -- optional user-given title
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_shared_recipes_slug ON shared_recipes (slug);
CREATE INDEX idx_shared_recipes_created ON shared_recipes (created_at DESC);
