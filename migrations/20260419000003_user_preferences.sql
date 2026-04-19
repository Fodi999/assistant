-- ══════════════════════════════════════════════════════════════════════════════
-- USER PREFERENCES — personal health/diet/lifestyle data for ChefOS AI
-- Migration: 20260419000003_user_preferences.sql
-- ══════════════════════════════════════════════════════════════════════════════

CREATE TABLE IF NOT EXISTS user_preferences (
    user_id             UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,

    -- Personal
    age                 INT,
    weight              REAL,              -- kg
    target_weight       REAL,              -- kg

    -- Goals
    goal                TEXT DEFAULT 'eat_healthier',  -- lose_fat, gain_muscle, maintain_weight, eat_healthier, medical_diet
    calorie_target      INT  DEFAULT 2200,
    protein_target      INT  DEFAULT 120,
    meals_per_day       INT  DEFAULT 3,

    -- Diet
    diet                TEXT DEFAULT 'no_restrictions', -- no_restrictions, vegetarian, vegan, keto, paleo, gluten_free, dairy_free
    preferred_cuisine   TEXT DEFAULT 'any',             -- any, asian, mediterranean, american, mexican, italian, middle_eastern

    -- Lifestyle
    cooking_level       TEXT DEFAULT 'home_cook',       -- beginner, home_cook, advanced, chef
    cooking_time        TEXT DEFAULT 'medium',           -- quick, medium, long, any

    -- Arrays stored as JSONB
    likes               JSONB DEFAULT '[]'::jsonb,
    dislikes            JSONB DEFAULT '[]'::jsonb,
    allergies           JSONB DEFAULT '[]'::jsonb,
    intolerances        JSONB DEFAULT '[]'::jsonb,
    medical_conditions  JSONB DEFAULT '[]'::jsonb,

    -- Timestamps
    updated_at          TIMESTAMPTZ DEFAULT now()
);
