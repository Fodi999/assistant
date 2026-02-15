use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;
use time::OffsetDateTime;

/// AI-generated cooking step with detailed information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookingStep {
    pub step_number: i32,
    pub action: String,                    // "Нарезать", "Варить", "Жарить"
    pub description: String,               // Full step description
    pub duration_minutes: Option<i32>,     // Estimated time
    pub temperature: Option<String>,       // "180°C", "medium heat"
    pub technique: Option<String>,         // "dice", "julienne", "blanch"
    pub ingredients_used: Vec<String>,     // Ingredient IDs used in this step
}

/// Validation issue (warning or error)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub severity: String,                  // "warning" | "error"
    pub code: String,                      // "MISSING_INGREDIENT", "UNSAFE_TEMP"
    pub message: String,                   // Human-readable message
    pub field: Option<String>,             // Field that caused the issue
}

/// Validation results for a recipe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeValidation {
    pub is_valid: bool,
    pub warnings: Vec<ValidationIssue>,
    pub errors: Vec<ValidationIssue>,
    pub missing_ingredients: Vec<String>, // Ingredients mentioned but not in list
    pub safety_checks: Vec<String>,       // Safety-related notes
}

/// AI suggestion for recipe improvement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeSuggestion {
    pub suggestion_type: String,           // "improvement" | "substitution" | "technique"
    pub title: String,                     // "Use fresh herbs instead of dried"
    pub description: String,               // Detailed explanation
    pub impact: String,                    // "taste" | "texture" | "nutrition" | "cost"
    pub confidence: f32,                   // 0.0 - 1.0
}

/// Complete AI insights for a recipe in specific language
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeAIInsights {
    pub id: Uuid,
    pub recipe_id: Uuid,
    pub language: String,                  // ru/en/pl/uk
    
    // AI-generated data
    pub steps: Vec<CookingStep>,
    pub validation: RecipeValidation,
    pub suggestions: Vec<RecipeSuggestion>,
    pub feasibility_score: i32,            // 0-100
    
    // Metadata
    pub model: String,                     // "llama-3.1-8b-instant"
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// Database row structure (with JSONB fields)
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RecipeAIInsightsRow {
    pub id: Uuid,
    pub recipe_id: Uuid,
    pub language: String,
    pub steps_json: sqlx::types::Json<Vec<CookingStep>>,
    pub validation_json: sqlx::types::Json<RecipeValidation>,
    pub suggestions_json: sqlx::types::Json<Vec<RecipeSuggestion>>,
    pub feasibility_score: i32,
    pub model: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl From<RecipeAIInsightsRow> for RecipeAIInsights {
    fn from(row: RecipeAIInsightsRow) -> Self {
        Self {
            id: row.id,
            recipe_id: row.recipe_id,
            language: row.language,
            steps: row.steps_json.0,
            validation: row.validation_json.0,
            suggestions: row.suggestions_json.0,
            feasibility_score: row.feasibility_score,
            model: row.model,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Request to create AI insights
#[derive(Debug, Clone, Deserialize)]
pub struct CreateRecipeAIInsightsRequest {
    pub recipe_id: Uuid,
    pub language: String,
}

/// Response with AI insights
#[derive(Debug, Clone, Serialize)]
pub struct RecipeAIInsightsResponse {
    pub insights: RecipeAIInsights,
    pub generated_in_ms: u64,              // Time taken to generate
}
