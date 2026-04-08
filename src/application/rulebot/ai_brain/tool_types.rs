//! Tool types — LLM action schema for AI Brain tool-calling.
//!
//! Defines the JSON contract between LLM and our tool executor:
//!   LLM returns AiAction → we match on ToolChoice → execute the right handler.

use serde::Deserialize;

/// The LLM's decision: which tool to call and with what parameters.
#[derive(Debug, Deserialize)]
pub(crate) struct AiAction {
    /// Which tool to use.
    pub tool: ToolChoice,
    /// The reasoning behind the choice (for logging/debugging).
    #[serde(default)]
    pub reasoning: String,
}

/// Available tools the AI Brain can invoke.
#[derive(Debug, Deserialize)]
#[serde(tag = "name", content = "params", rename_all = "snake_case")]
pub(crate) enum ToolChoice {
    /// Search the ingredient catalog by goal/keywords.
    SearchProducts {
        /// What to search for (e.g. "high protein fish").
        query: String,
        /// Health goal: "high_protein", "low_calorie", "balanced".
        #[serde(default = "default_goal")]
        goal: String,
        /// How many results to return (1-5).
        #[serde(default = "default_limit")]
        limit: usize,
    },
    /// Get nutrition info for a specific product.
    GetNutrition {
        /// Product name or slug (e.g. "salmon", "chicken breast").
        product: String,
    },
    /// Convert between units.
    ConvertUnits {
        /// The numeric value to convert.
        value: f64,
        /// Source unit (e.g. "g", "ml", "cups").
        from: String,
        /// Target unit (e.g. "tbsp", "oz").
        to: String,
    },
    /// Answer from LLM's own knowledge (no tool needed).
    GeneralAnswer {
        /// The answer text to return to the user.
        answer: String,
    },
    /// Create a meal plan / menu suggestion.
    MealPlan {
        /// The user's goal or constraint (e.g. "lose weight", "high protein dinner").
        goal: String,
        /// Number of meals to suggest.
        #[serde(default = "default_meals")]
        meals: usize,
    },
}

fn default_goal() -> String { "balanced".to_string() }
fn default_limit() -> usize { 3 }
fn default_meals() -> usize { 3 }

/// Human-readable tool name for logging.
pub(crate) fn action_name(tool: &ToolChoice) -> &'static str {
    match tool {
        ToolChoice::SearchProducts { .. } => "search_products",
        ToolChoice::GetNutrition { .. }   => "get_nutrition",
        ToolChoice::ConvertUnits { .. }   => "convert_units",
        ToolChoice::GeneralAnswer { .. }  => "general_answer",
        ToolChoice::MealPlan { .. }       => "meal_plan",
    }
}
