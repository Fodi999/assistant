//! CulinaryContext — input contract for SmartService v3.

use serde::{Deserialize, Serialize};

// ── Goal enum ────────────────────────────────────────────────────────────────

/// High-level cooking/nutrition goal that affects scoring, diagnostics, and explanations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Goal {
    Balanced,
    HighProtein,
    LowCalorie,
    Keto,
    MuscleGain,
    Diet,
    FlavorBoost,
}

impl Goal {
    /// Parse from free-text string (backward-compatible with v2 strings).
    pub fn from_str_loose(s: &str) -> Self {
        match s.to_lowercase().replace('-', "_").as_str() {
            "high_protein" | "highprotein" | "protein" => Goal::HighProtein,
            "low_calorie" | "lowcalorie" | "lowcal"    => Goal::LowCalorie,
            "keto" | "low_carb" | "lowcarb"             => Goal::Keto,
            "muscle_gain" | "musclegain" | "muscle"     => Goal::MuscleGain,
            "diet" | "weight_loss"                      => Goal::Diet,
            "flavor_boost" | "flavorboost" | "flavor"   => Goal::FlavorBoost,
            _                                           => Goal::Balanced,
        }
    }

    /// Human-readable label for the goal.
    pub fn label(&self) -> &'static str {
        match self {
            Goal::Balanced    => "balanced",
            Goal::HighProtein => "high-protein",
            Goal::LowCalorie  => "low-calorie",
            Goal::Keto        => "keto",
            Goal::MuscleGain  => "muscle-gain",
            Goal::Diet        => "diet",
            Goal::FlavorBoost => "flavor-boost",
        }
    }
}

impl Default for Goal {
    fn default() -> Self {
        Goal::Balanced
    }
}

// ── Context ──────────────────────────────────────────────────────────────────

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

    /// High-level goal — typed enum (v3). Also accepts free-text for backward compat.
    #[serde(default)]
    pub goal: Option<String>,

    /// Desired response language: "en", "ru", "pl", "uk". Defaults to "en".
    #[serde(default = "default_lang")]
    pub lang: String,

    /// Session ID for continuity (v3). If absent, server generates one.
    #[serde(default)]
    pub session_id: Option<String>,
}

fn default_lang() -> String {
    "en".to_string()
}

impl CulinaryContext {
    /// Resolve the Goal enum from the optional free-text field.
    pub fn resolved_goal(&self) -> Goal {
        self.goal
            .as_deref()
            .map(Goal::from_str_loose)
            .unwrap_or_default()
    }

    /// Build the cache key: "ingredient|state|extra1,extra2|goal|lang"
    pub fn cache_key(&self) -> String {
        let state = self.state.as_deref().unwrap_or("raw");
        let extras = if self.additional_ingredients.is_empty() {
            "_".to_string()
        } else {
            let mut sorted = self.additional_ingredients.clone();
            sorted.sort();
            sorted.join(",")
        };
        let goal = self.resolved_goal().label();
        format!("{}|{}|{}|{}|{}", self.ingredient, state, extras, goal, self.lang)
    }
}
