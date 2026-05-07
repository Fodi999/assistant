// ── Scene: Recipe — a recipe as a scene entity ───────────────────────────────────
// Domain: A recipe maps to a cube / wall formation where each particle
// represents one ingredient slot. Visual layout mirrors the recipe structure.

use super::ingredient::Ingredient;

/// A single step in a recipe, each becoming a layer in the cube formation.
#[derive(Debug, Clone)]
pub struct RecipeStep {
    pub description: String,
    pub duration_sec: u32,
}

/// A recipe that can be "played" as a scene formation sequence.
#[derive(Debug, Clone)]
pub struct Recipe {
    pub name:        String,
    pub ingredients: Vec<Ingredient>,
    pub steps:       Vec<RecipeStep>,
}

impl Recipe {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), ingredients: Vec::new(), steps: Vec::new() }
    }

    pub fn add_ingredient(&mut self, ingredient: Ingredient) -> &mut Self {
        self.ingredients.push(ingredient);
        self
    }

    pub fn add_step(&mut self, description: impl Into<String>, duration_sec: u32) -> &mut Self {
        self.steps.push(RecipeStep { description: description.into(), duration_sec });
        self
    }

    /// Ideal particle count to represent this recipe (one particle per ingredient gram).
    pub fn suggested_particle_count(&self) -> u32 {
        let total: f32 = self.ingredients.iter().map(|i| i.quantity).sum();
        (total as u32).clamp(100, 1_000_000)
    }
}
