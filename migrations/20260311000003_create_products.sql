-- ══════════════════════════════════════════════════════════════════════════════
-- Products table — unified food product catalog
-- Migration: 20260311000003_create_products.sql
-- Must run BEFORE 20260312000001_nutrition_schema.sql
-- ══════════════════════════════════════════════════════════════════════════════

CREATE TABLE IF NOT EXISTS products (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    slug                    TEXT UNIQUE NOT NULL,

    name_en                 TEXT,
    name_pl                 TEXT,
    name_ru                 TEXT,
    name_uk                 TEXT,

    category_id             UUID,

    product_type            TEXT CHECK (product_type IN (
                                'seafood','vegetable','fruit','meat','grain',
                                'dairy','spice','herb','legume','nut','mushroom','other'
                            )),

    unit                    TEXT,

    image_url               TEXT,
    description             TEXT,

    density_g_per_ml        REAL,

    typical_portion_g       REAL,
    edible_yield_percent    REAL,

    shelf_life_days         INTEGER,

    wild_farmed             TEXT CHECK (wild_farmed IN ('wild','farmed','both')),
    water_type              TEXT CHECK (water_type IN ('fresh','salt','both')),

    sushi_grade             BOOLEAN DEFAULT FALSE,

    substitution_group      TEXT,

    availability_months     BOOLEAN[],

    created_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_products_slug        ON products(slug);
CREATE INDEX IF NOT EXISTS idx_products_category_id ON products(category_id);
CREATE INDEX IF NOT EXISTS idx_products_product_type ON products(product_type);
