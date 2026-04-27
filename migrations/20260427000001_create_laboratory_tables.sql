-- ══════════════════════════════════════════════════════════════════════════════
-- LABORATORY — food-technology analysis projects
-- Migration: 20260427000001_create_laboratory_tables.sql
--
-- The Laboratory turns the rich `products` catalog (with culinary_behavior,
-- health_profile, processing_effects, sugar_profile, …) into an analytical
-- engine. A project is a user-defined recipe-in-progress:
--
--   lab_projects             – the project itself (name, owner, status)
--   lab_project_ingredients  – which products & how much (slug refs `products.slug`)
--   lab_process_steps        – ordered technological process (heat / cool / blend …)
--   lab_project_analysis     – snapshot of the engines' output (shelf life,
--                              process_effects, flavor, nutrition, warnings, …)
--
-- We deliberately store ONLY user choices in lab_project_ingredients —
-- all rich properties (pH, water_activity, culinary_behaviors, pairings, …)
-- are pulled from the `products` catalog at analysis time via the
-- `catalog_profile_adapter`. The analysis snapshot makes the result stable
-- even if catalog data later changes.
-- ══════════════════════════════════════════════════════════════════════════════

-- ── 1. lab_projects ───────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS lab_projects (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id            UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    name                TEXT NOT NULL,
    description         TEXT,

    -- e.g. "sauce", "marinade", "drink", "dessert", "spread", "soup"
    target_product_type TEXT,

    -- draft → analyzing → analyzed → archived
    status              TEXT NOT NULL DEFAULT 'draft'
                        CHECK (status IN ('draft','analyzing','analyzed','archived')),

    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_lab_projects_owner_id
    ON lab_projects(owner_id);
CREATE INDEX IF NOT EXISTS idx_lab_projects_status
    ON lab_projects(status);

CREATE TRIGGER trg_lab_projects_updated
    BEFORE UPDATE ON lab_projects
    FOR EACH ROW
    EXECUTE FUNCTION set_updated_at();

-- ── 2. lab_project_ingredients ───────────────────────────────────────────────
-- Only the user's choice — everything else is fetched from `products` at runtime.
CREATE TABLE IF NOT EXISTS lab_project_ingredients (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id          UUID NOT NULL REFERENCES lab_projects(id) ON DELETE CASCADE,

    -- Soft reference to products.slug. We don't enforce FK because catalog
    -- can be reseeded; analyses are durable via `lab_project_analysis`.
    ingredient_slug     TEXT NOT NULL,

    quantity            NUMERIC(12,3) NOT NULL CHECK (quantity > 0),
    unit                TEXT NOT NULL,   -- 'g','kg','ml','l','piece'

    -- 'base' | 'flavor' | 'acid' | 'sweetener' | 'spice' | 'herb'
    -- | 'garnish' | 'sauce' | 'liquid' | 'fat' | 'thickener'
    role                TEXT,

    sort_order          INTEGER NOT NULL DEFAULT 0,
    notes               TEXT,

    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_lab_project_ingredients_project_id
    ON lab_project_ingredients(project_id);
CREATE INDEX IF NOT EXISTS idx_lab_project_ingredients_slug
    ON lab_project_ingredients(ingredient_slug);

-- ── 3. lab_process_steps ─────────────────────────────────────────────────────
-- Ordered technological steps applied to the whole project.
CREATE TABLE IF NOT EXISTS lab_process_steps (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id          UUID NOT NULL REFERENCES lab_projects(id) ON DELETE CASCADE,

    order_index         INTEGER NOT NULL,

    -- 'heat' | 'cool' | 'freeze' | 'blend' | 'mix' | 'marinate'
    -- | 'ferment' | 'dry' | 'smoke' | 'age' | 'pasteurize' | 'rest'
    technique           TEXT NOT NULL,

    temperature_c       NUMERIC(6,2),
    duration_min        INTEGER CHECK (duration_min IS NULL OR duration_min >= 0),

    -- Optional list of ingredient slugs this step targets (NULL = all)
    target_slugs        TEXT[],

    notes               TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),

    UNIQUE (project_id, order_index)
);

CREATE INDEX IF NOT EXISTS idx_lab_process_steps_project_id
    ON lab_process_steps(project_id);

-- ── 4. lab_project_analysis ──────────────────────────────────────────────────
-- Snapshot of the engines' output. One row per analysis run; latest by created_at.
CREATE TABLE IF NOT EXISTS lab_project_analysis (
    id                          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id                  UUID NOT NULL REFERENCES lab_projects(id) ON DELETE CASCADE,

    -- Top-level summary numbers
    shelf_life_days             INTEGER,
    estimated_cost              NUMERIC(12, 2),
    complexity_score            INTEGER CHECK (complexity_score IS NULL
                                               OR (complexity_score >= 0 AND complexity_score <= 100)),
    risk_level                  TEXT CHECK (risk_level IS NULL
                                            OR risk_level IN ('low','medium','high','critical')),

    -- Engine outputs
    texture_result              TEXT,
    flavor_result               JSONB NOT NULL DEFAULT '{}'::jsonb,
    nutrition_result            JSONB NOT NULL DEFAULT '{}'::jsonb,
    process_effects             JSONB NOT NULL DEFAULT '[]'::jsonb,
    storage_recommendations     JSONB NOT NULL DEFAULT '{}'::jsonb,
    pairing_suggestions         JSONB NOT NULL DEFAULT '[]'::jsonb,
    warnings                    JSONB NOT NULL DEFAULT '[]'::jsonb,

    -- Optional raw input snapshot for reproducibility
    input_snapshot              JSONB,

    created_at                  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_lab_project_analysis_project_id
    ON lab_project_analysis(project_id);
CREATE INDEX IF NOT EXISTS idx_lab_project_analysis_project_created
    ON lab_project_analysis(project_id, created_at DESC);
