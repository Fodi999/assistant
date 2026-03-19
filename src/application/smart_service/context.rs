//! CulinaryContext — input contract for SmartService.

use serde::{Deserialize, Serialize};

/// What the client sends to `POST /api/smart/ingredient`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CulinaryContext {
    /// Main ingredient slug (e.g. "salmon", "tomato").
    pub ingredient: String,

    /// Cooking state (e.g. "raw", "grilled", "steamed"). Optional.
    #[serde(default)]
    pub state: Option<String>,

    /// Additional ingredients already in the recipe (slugs).
    #[serde(default)]
    pub additional_ingredients: Vec<String>,

    /// High-level goal: "balanced", "high-protein", "low-carb", "flavor-boost".
    #[serde(default)]
    pub goal: Option<String>,

    /// Desired response language: "en", "ru", "pl", "uk". Defaults to "en".
    #[serde(default = "default_lang")]
    pub lang: String,
}

fn default_lang() -> String {
    "en".to_string()
}

impl CulinaryContext {
    /// Build the cache key: "ingredient|state|extra1,extra2|goal"
    pub fn cache_key(&self) -> String {
        let state = self.state.as_deref().unwrap_or("raw");
        let extras = if self.additional_ingredients.is_empty() {
            "_".to_string()
        } else {
            let mut sorted = self.additional_ingredients.clone();
            sorted.sort();
            sorted.join(",")
        };
        let goal = self.goal.as_deref().unwrap_or("balanced");
        format!("{}|{}|{}|{}|{}", self.ingredient, state, extras, goal, self.lang)
    }
}
