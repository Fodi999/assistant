-- Add Recipe AI Insights table
-- This table stores AI-generated analysis and suggestions for recipes
-- Separate from recipe_translations to keep concerns separated

CREATE TABLE IF NOT EXISTS recipe_ai_insights (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    recipe_id UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    language VARCHAR(5) NOT NULL,                 -- ru/en/pl/uk
    
    -- AI-generated structured data
    steps_json JSONB NOT NULL,                    -- Array of cooking steps with details
    validation_json JSONB NOT NULL,               -- Warnings, errors, missing ingredients
    suggestions_json JSONB NOT NULL,              -- Improvements, ingredient substitutions
    feasibility_score INTEGER NOT NULL CHECK (feasibility_score >= 0 AND feasibility_score <= 100),
    
    -- AI metadata
    model VARCHAR(100) NOT NULL,                  -- e.g., "llama-3.1-8b-instant"
    
    -- Timestamps
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    -- One insights record per recipe per language
    UNIQUE(recipe_id, language)
);

-- Index for faster lookups by recipe_id
CREATE INDEX IF NOT EXISTS idx_recipe_ai_insights_recipe_id
    ON recipe_ai_insights(recipe_id);

-- Index for filtering by language
CREATE INDEX IF NOT EXISTS idx_recipe_ai_insights_language
    ON recipe_ai_insights(language);

-- Index for filtering by feasibility score (e.g., show only high-quality recipes)
CREATE INDEX IF NOT EXISTS idx_recipe_ai_insights_feasibility
    ON recipe_ai_insights(feasibility_score);

-- Trigger to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_recipe_ai_insights_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER recipe_ai_insights_updated_at
    BEFORE UPDATE ON recipe_ai_insights
    FOR EACH ROW
    EXECUTE FUNCTION update_recipe_ai_insights_updated_at();

-- Comments for documentation
COMMENT ON TABLE recipe_ai_insights IS 'AI-generated insights for recipes including steps, validation, and suggestions';
COMMENT ON COLUMN recipe_ai_insights.steps_json IS 'Array of cooking steps with timing, temperature, techniques';
COMMENT ON COLUMN recipe_ai_insights.validation_json IS 'Validation results: warnings, errors, missing ingredients';
COMMENT ON COLUMN recipe_ai_insights.suggestions_json IS 'AI suggestions for improvements and substitutions';
COMMENT ON COLUMN recipe_ai_insights.feasibility_score IS 'Recipe feasibility score from 0 (impossible) to 100 (perfect)';
