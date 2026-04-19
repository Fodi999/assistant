use serde::{Deserialize, Serialize};

/// User preferences — health, diet, lifestyle for ChefOS AI personalization
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserPreferences {
    pub age: Option<i32>,
    pub weight: Option<f64>,
    pub target_weight: Option<f64>,

    pub goal: String,
    pub calorie_target: i32,
    pub protein_target: i32,
    pub meals_per_day: i32,

    pub diet: String,
    pub preferred_cuisine: String,

    pub cooking_level: String,
    pub cooking_time: String,

    pub likes: Vec<String>,
    pub dislikes: Vec<String>,
    pub allergies: Vec<String>,
    pub intolerances: Vec<String>,
    pub medical_conditions: Vec<String>,
}
