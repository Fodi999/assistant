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

// ── MealType enum ────────────────────────────────────────────────────────────

/// Meal occasion — affects which ingredients/portions/styles are preferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MealType {
    Breakfast,
    Lunch,
    Dinner,
    Snack,
    Dessert,
}

impl MealType {
    /// Parse from free-text (tolerant).
    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "breakfast" | "morning"               => Some(MealType::Breakfast),
            "lunch" | "midday"                    => Some(MealType::Lunch),
            "dinner" | "evening" | "supper"       => Some(MealType::Dinner),
            "snack" | "appetizer" | "starter"     => Some(MealType::Snack),
            "dessert" | "sweet"                   => Some(MealType::Dessert),
            _                                     => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            MealType::Breakfast => "breakfast",
            MealType::Lunch     => "lunch",
            MealType::Dinner    => "dinner",
            MealType::Snack     => "snack",
            MealType::Dessert   => "dessert",
        }
    }
}

// ── Diet enum ────────────────────────────────────────────────────────────────

/// Dietary restriction — hard constraints on ingredient selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Diet {
    None,
    Vegetarian,
    Vegan,
    Pescatarian,
    GlutenFree,
    DairyFree,
    Paleo,
    Mediterranean,
}

impl Diet {
    /// Parse from free-text (tolerant).
    pub fn from_str_loose(s: &str) -> Self {
        match s.to_lowercase().replace('-', "_").replace(' ', "_").as_str() {
            "vegetarian" | "veggie"                      => Diet::Vegetarian,
            "vegan" | "plant_based"                      => Diet::Vegan,
            "pescatarian" | "pescetarian"                 => Diet::Pescatarian,
            "gluten_free" | "glutenfree" | "no_gluten"   => Diet::GlutenFree,
            "dairy_free" | "dairyfree" | "no_dairy" | "lactose_free" => Diet::DairyFree,
            "paleo" | "primal"                           => Diet::Paleo,
            "mediterranean" | "med"                      => Diet::Mediterranean,
            _                                            => Diet::None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Diet::None           => "none",
            Diet::Vegetarian     => "vegetarian",
            Diet::Vegan          => "vegan",
            Diet::Pescatarian    => "pescatarian",
            Diet::GlutenFree     => "gluten-free",
            Diet::DairyFree      => "dairy-free",
            Diet::Paleo          => "paleo",
            Diet::Mediterranean  => "mediterranean",
        }
    }
}

impl Default for Diet {
    fn default() -> Self {
        Diet::None
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

    /// Meal occasion: "breakfast", "lunch", "dinner", "snack", "dessert".
    /// Affects ingredient scoring, portion sizes, and variant titles.
    #[serde(default)]
    pub meal_type: Option<String>,

    /// Dietary restriction: "vegetarian", "vegan", "keto", "gluten_free", etc.
    /// Hard constraint — filters out incompatible candidates.
    #[serde(default)]
    pub diet: Option<String>,

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

    /// Resolve the MealType from the optional free-text field.
    pub fn resolved_meal_type(&self) -> Option<MealType> {
        self.meal_type
            .as_deref()
            .and_then(MealType::from_str_loose)
    }

    /// Resolve the Diet from the optional free-text field.
    pub fn resolved_diet(&self) -> Diet {
        self.diet
            .as_deref()
            .map(Diet::from_str_loose)
            .unwrap_or_default()
    }

    /// Build the cache key: "ingredient|state|extra1,extra2|goal|meal|diet|lang"
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
        let meal = self.resolved_meal_type().map(|m| m.label()).unwrap_or("_");
        let diet = self.resolved_diet().label();
        format!("{}|{}|{}|{}|{}|{}|{}", self.ingredient, state, extras, goal, meal, diet, self.lang)
    }
}
