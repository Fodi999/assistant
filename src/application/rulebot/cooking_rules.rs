//! Cooking Rules — DDD rules-as-data for recipe generation.
//!
//! Each `DishRule` defines HOW to cook a dish type:
//!   - Which roles are required (protein, vegetable, aromatic…)
//!   - Which cooking method per role
//!   - Step sequence (pure logic, no LLM)
//!   - Constraints (max_fat, required_liquid, min_protein…)
//!
//! 9 dish types × 4 rule types ≈ 50–60 rules → covers 95% of dishes.
//!
//! Constraint Engine:
//!   After Gemini returns ingredients, `apply_constraints()` CORRECTS
//!   the dish to obey culinary laws (liquid in soup, oil limits, etc.)

use super::meal_builder::CookMethod;
use super::recipe_engine::DishType;

// ── Ingredient Roles ─────────────────────────────────────────────────────────

/// Semantic role of an ingredient in a dish.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IngredientRole {
    Protein,    // meat, fish, eggs, tofu
    Vegetable,  // potato, cabbage, beet, tomato…
    Aromatic,   // onion, carrot (зажарка in soups)
    Base,       // grain, pasta, legume
    Spice,      // garlic, pepper, herbs
    Oil,        // olive oil, butter, ghee
    Condiment,  // soy sauce, tomato paste, vinegar
    Liquid,     // broth, water, wine (future)
}

impl IngredientRole {
    /// Convert from the old string-based role system.
    pub fn from_str_role(role: &str, slug: &str) -> Self {
        // Aromatics: onion & carrot are special in soups/stews
        if slug.contains("onion") || slug.contains("carrot") || slug.contains("celery")
            || slug.contains("leek") || slug.contains("shallot") {
            return IngredientRole::Aromatic;
        }
        match role {
            "protein" => IngredientRole::Protein,
            "base" => IngredientRole::Base,
            "side" => IngredientRole::Vegetable,
            "spice" => IngredientRole::Spice,
            "oil" => IngredientRole::Oil,
            "condiment" => IngredientRole::Condiment,
            "liquid" => IngredientRole::Liquid,
            _ => IngredientRole::Vegetable,
        }
    }

    /// Back to string for JSON serialization.
    pub fn as_str(&self) -> &'static str {
        match self {
            IngredientRole::Protein => "protein",
            IngredientRole::Vegetable => "side",
            IngredientRole::Aromatic => "aromatic",
            IngredientRole::Base => "base",
            IngredientRole::Spice => "spice",
            IngredientRole::Oil => "oil",
            IngredientRole::Condiment => "condiment",
            IngredientRole::Liquid => "liquid",
        }
    }
}

// ── Step Types ───────────────────────────────────────────────────────────────

/// Named step type — the engine generates text from this.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepType {
    BoilProtein,      // "Отварить говядину до готовности"
    BraiseProtein,    // "Потушить мясо до мягкости"
    SearProtein,      // "Обжарить до корочки"
    GrillProtein,     // "Обжарить на гриле"
    MarinateProtein,  // "Замариновать"
    SauteAromatics,   // "Сделать зажарку: спассеровать лук и морковь"
    AddRoots,         // "Добавить картофель, свёклу, варить"
    AddVegetables,    // "Добавить капусту, помидор"
    AddAromatics,     // "Добавить зажарку в суп"
    BoilBase,         // "Отварить рис/пасту"
    AddBase,          // "Добавить крупу"
    AddLiquid,        // "Залить водой, довести до кипения"
    AddSpices,        // "Добавить специи, довести до вкуса"
    Combine,          // "Соединить всё" / "Смешать"
    Rest,             // "Дать настояться 5 минут, подавать"
    PreheatOven,      // "Разогреть духовку до 180°C"
    BakeAll,          // "Запекать до готовности"
    PreheatWok,       // "Разогреть масло в воке"
    PreheatGrill,     // "Разогреть гриль"
    ChopAll,          // "Нарезать" (salad)
    Dress,            // "Заправить" (salad)
    ServeFresh,       // "Подать свежим"
}

// ── Rule Structs ─────────────────────────────────────────────────────────────

/// Which roles a dish requires and in what amounts.
#[derive(Debug, Clone)]
pub struct RoleRule {
    pub role: IngredientRole,
    pub min: usize,
    pub max: usize,
    pub required: bool,
}

/// Which cooking method to use for a given role.
#[derive(Debug, Clone)]
pub struct MethodRule {
    pub role: IngredientRole,
    pub method: CookMethod,
}

/// A cooking step in the recipe sequence.
#[derive(Debug, Clone)]
pub struct StepRule {
    pub step: StepType,
    pub roles: Vec<IngredientRole>,
    pub time_min: Option<u16>,
    /// Cooking temperature in °C (sear=200, bake=180, simmer=90, etc.)
    pub temp_c: Option<u16>,
    /// Short chef tip key (resolved to localized text in step_text)
    pub tip: Option<&'static str>,
}

/// Shorthand to create a StepRule without temp/tip (most steps).
fn sr(step: StepType, roles: Vec<IngredientRole>, time_min: Option<u16>) -> StepRule {
    StepRule { step, roles, time_min, temp_c: None, tip: None }
}

/// Shorthand to create a StepRule with temperature.
fn sr_t(step: StepType, roles: Vec<IngredientRole>, time_min: Option<u16>, temp_c: u16, tip: Option<&'static str>) -> StepRule {
    StepRule { step, roles, time_min, temp_c: Some(temp_c), tip }
}

/// Constraint key for dish-level limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintKey {
    /// Dish MUST contain a liquid ingredient (soup, stew).
    RequiresLiquid,
    /// Max fat percentage of total dish weight (e.g. 15% for soup).
    MaxFatPercent,
    /// Max total kcal per serving.
    MaxKcalPerServing,
    /// Min protein grams per serving.
    MinProteinPerServing,
    /// Max oil grams total (hard cap).
    MaxOilGrams,
    /// Requires an aromatic (onion/carrot for зажарка).
    RequiresAromatic,
}

/// A numeric constraint on the dish.
#[derive(Debug, Clone)]
pub struct ConstraintRule {
    pub key: ConstraintKey,
    pub value: f32,
}

/// Complete rule set for one dish type.
/// This is the SINGLE source of truth for "how to cook a Soup/Stew/Salad/etc."
#[derive(Debug, Clone)]
pub struct DishRule {
    pub dish_type: DishType,
    pub roles: Vec<RoleRule>,
    pub methods: Vec<MethodRule>,
    pub steps: Vec<StepRule>,
    pub constraints: Vec<ConstraintRule>,
}

// ── Rule Definitions: 7 dish types ───────────────────────────────────────────

/// Load the rule set for a given dish type.
pub fn load_rule(dish_type: DishType) -> DishRule {
    match dish_type {
        DishType::Soup => soup_rule(),
        DishType::Stew => stew_rule(),
        DishType::Salad => salad_rule(),
        DishType::StirFry => stir_fry_rule(),
        DishType::Grill => grill_rule(),
        DishType::Bake => bake_rule(),
        DishType::Pasta => pasta_rule(),
        DishType::Raw => raw_rule(),
        DishType::Default => default_rule(),
    }
}

/// Resolve cooking method for a role using the dish rule.
/// Falls back to Raw if no rule matches.
pub fn method_for_role(rule: &DishRule, role: IngredientRole) -> CookMethod {
    rule.methods.iter()
        .find(|m| m.role == role)
        .map(|m| m.method)
        .unwrap_or(CookMethod::Raw)
}

// ── Soup ─────────────────────────────────────────────────────────────────────

fn soup_rule() -> DishRule {
    use IngredientRole::*;
    use StepType::*;
    DishRule {
        dish_type: DishType::Soup,
        roles: vec![
            RoleRule { role: Protein,   min: 0, max: 1, required: false },
            RoleRule { role: Vegetable, min: 2, max: 6, required: true },
            RoleRule { role: Aromatic,  min: 1, max: 3, required: true },
            RoleRule { role: Base,      min: 0, max: 1, required: false },
            RoleRule { role: Liquid,    min: 1, max: 1, required: true },
            RoleRule { role: Spice,     min: 0, max: 4, required: false },
            RoleRule { role: Oil,       min: 0, max: 1, required: false },
            RoleRule { role: Condiment, min: 0, max: 2, required: false },
        ],
        methods: vec![
            MethodRule { role: Protein,   method: CookMethod::Boil },
            MethodRule { role: Vegetable, method: CookMethod::Boil },
            MethodRule { role: Aromatic,  method: CookMethod::Saute },
            MethodRule { role: Base,      method: CookMethod::Boil },
            MethodRule { role: Liquid,    method: CookMethod::Boil },
            MethodRule { role: Spice,     method: CookMethod::Raw },
            MethodRule { role: Oil,       method: CookMethod::Raw },
            MethodRule { role: Condiment, method: CookMethod::Raw },
        ],
        steps: vec![
            // CORRECT order: protein → aromatics → liquid → roots → leafy → base → spice
            sr_t(BoilProtein,     vec![Protein],   Some(40), 100, Some("foam")),
            sr_t(SauteAromatics,  vec![Aromatic],  Some(7),  160, Some("golden")),
            sr(AddLiquid,       vec![Liquid],    Some(5)),
            sr(AddRoots,        vec![Vegetable], Some(15)),
            sr(AddVegetables,   vec![Vegetable], Some(10)),
            sr(AddAromatics,    vec![Aromatic],  Some(2)),
            sr(AddBase,         vec![Base],      Some(10)),
            sr(AddSpices,       vec![Spice, Condiment], Some(5)),
            sr(Rest,            vec![],          Some(5)),
        ],
        constraints: vec![
            ConstraintRule { key: ConstraintKey::RequiresLiquid,  value: 1.0 },
            ConstraintRule { key: ConstraintKey::RequiresAromatic, value: 1.0 },
            ConstraintRule { key: ConstraintKey::MaxFatPercent,   value: 15.0 },
            ConstraintRule { key: ConstraintKey::MaxOilGrams,     value: 15.0 },
        ],
    }
}

// ── Stew ─────────────────────────────────────────────────────────────────────

fn stew_rule() -> DishRule {
    use IngredientRole::*;
    use StepType::*;
    DishRule {
        dish_type: DishType::Stew,
        roles: vec![
            RoleRule { role: Protein,   min: 0, max: 2, required: false },
            RoleRule { role: Vegetable, min: 2, max: 6, required: true },
            RoleRule { role: Aromatic,  min: 1, max: 3, required: true },
            RoleRule { role: Base,      min: 0, max: 1, required: false },
            RoleRule { role: Liquid,    min: 1, max: 1, required: true },
            RoleRule { role: Spice,     min: 0, max: 4, required: false },
            RoleRule { role: Oil,       min: 0, max: 1, required: false },
            RoleRule { role: Condiment, min: 0, max: 2, required: false },
        ],
        methods: vec![
            MethodRule { role: Protein,   method: CookMethod::Boil },
            MethodRule { role: Vegetable, method: CookMethod::Boil },
            MethodRule { role: Aromatic,  method: CookMethod::Saute },
            MethodRule { role: Base,      method: CookMethod::Boil },
            MethodRule { role: Liquid,    method: CookMethod::Boil },
            MethodRule { role: Spice,     method: CookMethod::Raw },
            MethodRule { role: Oil,       method: CookMethod::Raw },
            MethodRule { role: Condiment, method: CookMethod::Raw },
        ],
        steps: vec![
            sr_t(BraiseProtein,   vec![Protein],   Some(45), 160, Some("sear_first")),
            sr_t(SauteAromatics,  vec![Aromatic],  Some(7),  150, Some("golden")),
            sr(AddLiquid,       vec![Liquid],    Some(5)),
            sr(AddVegetables,   vec![Vegetable], Some(20)),
            sr(AddAromatics,    vec![Aromatic],  Some(2)),
            sr(AddBase,         vec![Base],      Some(10)),
            sr(AddSpices,       vec![Spice, Condiment], Some(5)),
            sr(Rest,            vec![],          Some(5)),
        ],
        constraints: vec![
            ConstraintRule { key: ConstraintKey::RequiresLiquid,   value: 1.0 },
            ConstraintRule { key: ConstraintKey::RequiresAromatic, value: 1.0 },
            ConstraintRule { key: ConstraintKey::MaxFatPercent,    value: 20.0 },
            ConstraintRule { key: ConstraintKey::MaxOilGrams,      value: 20.0 },
        ],
    }
}

// ── Salad ────────────────────────────────────────────────────────────────────

fn salad_rule() -> DishRule {
    use IngredientRole::*;
    use StepType::*;
    DishRule {
        dish_type: DishType::Salad,
        roles: vec![
            RoleRule { role: Protein,   min: 0, max: 1, required: false },
            RoleRule { role: Vegetable, min: 2, max: 8, required: true },
            RoleRule { role: Spice,     min: 0, max: 3, required: false },
            RoleRule { role: Oil,       min: 0, max: 1, required: false },
            RoleRule { role: Condiment, min: 0, max: 2, required: false },
        ],
        methods: vec![
            MethodRule { role: Protein,   method: CookMethod::Raw },
            MethodRule { role: Vegetable, method: CookMethod::Raw },
            MethodRule { role: Spice,     method: CookMethod::Raw },
            MethodRule { role: Oil,       method: CookMethod::Raw },
            MethodRule { role: Condiment, method: CookMethod::Raw },
        ],
        steps: vec![
            sr(ChopAll,    vec![Vegetable, Protein], None),
            sr(Combine,    vec![],                   None),
            sr(Dress,      vec![Spice, Condiment, Oil], None),
        ],
        constraints: vec![
            ConstraintRule { key: ConstraintKey::MaxOilGrams,        value: 20.0 },
            ConstraintRule { key: ConstraintKey::MaxKcalPerServing,  value: 350.0 },
        ],
    }
}

// ── StirFry ──────────────────────────────────────────────────────────────────

fn stir_fry_rule() -> DishRule {
    use IngredientRole::*;
    use StepType::*;
    DishRule {
        dish_type: DishType::StirFry,
        roles: vec![
            RoleRule { role: Protein,   min: 1, max: 2, required: true },
            RoleRule { role: Vegetable, min: 2, max: 6, required: true },
            RoleRule { role: Base,      min: 0, max: 1, required: false },
            RoleRule { role: Spice,     min: 0, max: 3, required: false },
            RoleRule { role: Oil,       min: 1, max: 1, required: true },
            RoleRule { role: Condiment, min: 0, max: 2, required: false },
        ],
        methods: vec![
            MethodRule { role: Protein,   method: CookMethod::Fry },
            MethodRule { role: Vegetable, method: CookMethod::Fry },
            MethodRule { role: Base,      method: CookMethod::Boil },
            MethodRule { role: Spice,     method: CookMethod::Raw },
            MethodRule { role: Oil,       method: CookMethod::Raw },
            MethodRule { role: Condiment, method: CookMethod::Raw },
        ],
        steps: vec![
            sr_t(PreheatWok,    vec![Oil],       Some(2), 230, Some("smoking")),
            sr_t(SearProtein,   vec![Protein],   Some(5), 200, Some("no_move")),
            sr(AddVegetables, vec![Vegetable], Some(5)),
            sr(AddSpices,     vec![Spice, Condiment], Some(2)),
            sr(AddBase,       vec![Base],      None),
        ],
        constraints: vec![
            ConstraintRule { key: ConstraintKey::MaxFatPercent, value: 25.0 },
            ConstraintRule { key: ConstraintKey::MaxOilGrams,   value: 20.0 },
        ],
    }
}

// ── Grill ────────────────────────────────────────────────────────────────────

fn grill_rule() -> DishRule {
    use IngredientRole::*;
    use StepType::*;
    DishRule {
        dish_type: DishType::Grill,
        roles: vec![
            RoleRule { role: Protein,   min: 1, max: 2, required: true },
            RoleRule { role: Vegetable, min: 0, max: 4, required: false },
            RoleRule { role: Base,      min: 0, max: 1, required: false },
            RoleRule { role: Spice,     min: 0, max: 3, required: false },
            RoleRule { role: Oil,       min: 0, max: 1, required: false },
        ],
        methods: vec![
            MethodRule { role: Protein,   method: CookMethod::Grill },
            MethodRule { role: Vegetable, method: CookMethod::Grill },
            MethodRule { role: Base,      method: CookMethod::Boil },
            MethodRule { role: Spice,     method: CookMethod::Raw },
            MethodRule { role: Oil,       method: CookMethod::Raw },
        ],
        steps: vec![
            sr(MarinateProtein, vec![Protein],   Some(30)),
            sr_t(PreheatGrill,    vec![],          Some(5), 220, None),
            sr_t(GrillProtein,    vec![Protein],   Some(10), 220, Some("rest_after")),
            sr(AddVegetables,   vec![Vegetable], Some(8)),
            sr(BoilBase,        vec![Base],      Some(10)),
        ],
        constraints: vec![
            ConstraintRule { key: ConstraintKey::MinProteinPerServing, value: 25.0 },
            ConstraintRule { key: ConstraintKey::MaxOilGrams,          value: 10.0 },
        ],
    }
}

// ── Bake ─────────────────────────────────────────────────────────────────────

fn bake_rule() -> DishRule {
    use IngredientRole::*;
    use StepType::*;
    DishRule {
        dish_type: DishType::Bake,
        roles: vec![
            RoleRule { role: Protein,   min: 0, max: 2, required: false },
            RoleRule { role: Vegetable, min: 0, max: 6, required: false },
            RoleRule { role: Base,      min: 0, max: 1, required: false },
            RoleRule { role: Spice,     min: 0, max: 3, required: false },
            RoleRule { role: Oil,       min: 0, max: 1, required: false },
        ],
        methods: vec![
            MethodRule { role: Protein,   method: CookMethod::Bake },
            MethodRule { role: Vegetable, method: CookMethod::Bake },
            MethodRule { role: Base,      method: CookMethod::Bake },
            MethodRule { role: Spice,     method: CookMethod::Raw },
            MethodRule { role: Oil,       method: CookMethod::Raw },
        ],
        steps: vec![
            sr_t(PreheatOven,     vec![],                 Some(10), 180, None),
            sr(ChopAll,         vec![Protein, Vegetable], None),
            sr_t(BakeAll,         vec![],                 Some(30), 180, Some("check_color")),
        ],
        constraints: vec![
            ConstraintRule { key: ConstraintKey::MaxOilGrams, value: 15.0 },
        ],
    }
}

// ── Pasta ────────────────────────────────────────────────────────────────────

fn pasta_rule() -> DishRule {
    use IngredientRole::*;
    use StepType::*;
    DishRule {
        dish_type: DishType::Pasta,
        roles: vec![
            RoleRule { role: Protein,   min: 0, max: 1, required: false },
            RoleRule { role: Vegetable, min: 0, max: 4, required: false },
            RoleRule { role: Base,      min: 1, max: 1, required: true },
            RoleRule { role: Spice,     min: 0, max: 3, required: false },
            RoleRule { role: Oil,       min: 0, max: 1, required: false },
            RoleRule { role: Condiment, min: 0, max: 2, required: false },
        ],
        methods: vec![
            MethodRule { role: Protein,   method: CookMethod::Fry },
            MethodRule { role: Vegetable, method: CookMethod::Fry },
            MethodRule { role: Base,      method: CookMethod::Boil },
            MethodRule { role: Spice,     method: CookMethod::Raw },
            MethodRule { role: Oil,       method: CookMethod::Raw },
            MethodRule { role: Condiment, method: CookMethod::Raw },
        ],
        steps: vec![
            sr_t(BoilBase,        vec![Base],      Some(10), 100, Some("al_dente")),
            sr_t(SearProtein,     vec![Protein],   Some(8),  180, None),
            sr(AddVegetables,   vec![Vegetable], Some(5)),
            sr(Combine,         vec![],          Some(2)),
            sr(AddSpices,       vec![Spice, Condiment], None),
        ],
        constraints: vec![
            ConstraintRule { key: ConstraintKey::MinProteinPerServing, value: 15.0 },
            ConstraintRule { key: ConstraintKey::MaxOilGrams,          value: 20.0 },
        ],
    }
}

// ── Raw ──────────────────────────────────────────────────────────────────────

fn raw_rule() -> DishRule {
    use IngredientRole::*;
    use StepType::*;
    DishRule {
        dish_type: DishType::Raw,
        roles: vec![
            RoleRule { role: Protein,   min: 1, max: 1, required: true },
            RoleRule { role: Vegetable, min: 0, max: 4, required: false },
            RoleRule { role: Spice,     min: 0, max: 3, required: false },
            RoleRule { role: Oil,       min: 0, max: 1, required: false },
        ],
        methods: vec![
            MethodRule { role: Protein,   method: CookMethod::Raw },
            MethodRule { role: Vegetable, method: CookMethod::Raw },
            MethodRule { role: Spice,     method: CookMethod::Raw },
            MethodRule { role: Oil,       method: CookMethod::Raw },
        ],
        steps: vec![
            sr(ChopAll,    vec![Protein, Vegetable], None),
            sr(Dress,      vec![Spice, Oil],         None),
            sr(ServeFresh,  vec![],                   None),
        ],
        constraints: vec![
            ConstraintRule { key: ConstraintKey::MaxOilGrams,       value: 10.0 },
            ConstraintRule { key: ConstraintKey::MaxKcalPerServing, value: 300.0 },
        ],
    }
}

// ── Default (fallback) ───────────────────────────────────────────────────────

fn default_rule() -> DishRule {
    use IngredientRole::*;
    use StepType::*;
    DishRule {
        dish_type: DishType::Default,
        roles: vec![
            RoleRule { role: Protein,   min: 0, max: 2, required: false },
            RoleRule { role: Vegetable, min: 0, max: 6, required: false },
            RoleRule { role: Base,      min: 0, max: 1, required: false },
            RoleRule { role: Spice,     min: 0, max: 4, required: false },
            RoleRule { role: Oil,       min: 0, max: 1, required: false },
            RoleRule { role: Condiment, min: 0, max: 2, required: false },
        ],
        methods: vec![
            MethodRule { role: Protein,   method: CookMethod::Grill },
            MethodRule { role: Vegetable, method: CookMethod::Steam },
            MethodRule { role: Base,      method: CookMethod::Boil },
            MethodRule { role: Spice,     method: CookMethod::Raw },
            MethodRule { role: Oil,       method: CookMethod::Raw },
            MethodRule { role: Condiment, method: CookMethod::Raw },
        ],
        steps: vec![
            sr_t(SearProtein,     vec![Protein],   Some(15), 180, None),
            sr(BoilBase,        vec![Base],      Some(10)),
            sr(AddVegetables,   vec![Vegetable], Some(10)),
            sr(AddSpices,       vec![Spice, Condiment], None),
        ],
        constraints: vec![],
    }
}

// ── Constraint Engine ────────────────────────────────────────────────────────
//
// The Constraint Engine CORRECTS a dish AFTER ingredients are resolved.
// It enforces culinary laws that Gemini can't know about:
//   - soup MUST have water
//   - oil can't exceed N grams
//   - fat % can't exceed N% of total weight
//   - protein must hit N grams per serving

/// Result of constraint validation — list of violations (if any).
#[derive(Debug, Clone)]
pub struct ConstraintViolation {
    pub key: ConstraintKey,
    pub message: String,
    pub auto_fixed: bool,
}

/// Apply constraints to a mutable ingredient list.
/// Returns a list of violations (some auto-fixed, some just logged).
///
/// This is called from `recipe_engine::resolve_dish()` AFTER ingredient resolution
/// and `auto_insert_implicit()`.
pub fn apply_constraints(
    rule: &DishRule,
    ingredients: &mut Vec<IngredientSnapshot>,
    servings: u8,
) -> Vec<ConstraintViolation> {
    let mut violations = Vec::new();

    for c in &rule.constraints {
        match c.key {
            // ── RequiresLiquid: soup/stew MUST have water ────────────
            ConstraintKey::RequiresLiquid => {
                let has_liquid = ingredients.iter().any(|i| i.role == IngredientRole::Liquid);
                if !has_liquid {
                    // Auto-fix: add 300ml water
                    ingredients.push(IngredientSnapshot {
                        slug: "water".into(),
                        role: IngredientRole::Liquid,
                        gross_g: 300.0,
                        fat_g: 0.0,
                        protein_g: 0.0,
                        kcal: 0,
                    });
                    violations.push(ConstraintViolation {
                        key: c.key,
                        message: "Added 300ml water (dish requires liquid)".into(),
                        auto_fixed: true,
                    });
                }
            }

            // ── RequiresAromatic: зажарка is mandatory ───────────────
            ConstraintKey::RequiresAromatic => {
                let has_aromatic = ingredients.iter().any(|i| i.role == IngredientRole::Aromatic);
                if !has_aromatic {
                    violations.push(ConstraintViolation {
                        key: c.key,
                        message: "Dish should have aromatics (onion/carrot) for зажарка".into(),
                        auto_fixed: false,
                    });
                }
            }

            // ── MaxOilGrams: hard cap on oil ─────────────────────────
            ConstraintKey::MaxOilGrams => {
                let max_oil = c.value;
                for ing in ingredients.iter_mut() {
                    if ing.role == IngredientRole::Oil && ing.gross_g > max_oil {
                        let old = ing.gross_g;
                        ing.gross_g = max_oil;
                        ing.fat_g = max_oil; // oil ≈ 100% fat
                        ing.kcal = (max_oil * 9.0) as u32;
                        violations.push(ConstraintViolation {
                            key: c.key,
                            message: format!("Oil capped: {:.0}g → {:.0}g", old, max_oil),
                            auto_fixed: true,
                        });
                    }
                }
            }

            // ── MaxFatPercent: total fat < N% of total weight ────────
            ConstraintKey::MaxFatPercent => {
                let total_weight: f32 = ingredients.iter().map(|i| i.gross_g).sum();
                let total_fat: f32 = ingredients.iter().map(|i| i.fat_g).sum();
                if total_weight > 0.0 {
                    let fat_pct = (total_fat / total_weight) * 100.0;
                    if fat_pct > c.value {
                        // Reduce oil ingredients proportionally
                        let excess_ratio = c.value / fat_pct;
                        for ing in ingredients.iter_mut() {
                            if ing.role == IngredientRole::Oil {
                                ing.gross_g *= excess_ratio;
                                ing.fat_g *= excess_ratio;
                                ing.kcal = (ing.fat_g * 9.0) as u32;
                            }
                        }
                        violations.push(ConstraintViolation {
                            key: c.key,
                            message: format!("Fat reduced from {:.1}% to ≤{:.0}%", fat_pct, c.value),
                            auto_fixed: true,
                        });
                    }
                }
            }

            // ── MaxKcalPerServing ────────────────────────────────────
            ConstraintKey::MaxKcalPerServing => {
                let total_kcal: u32 = ingredients.iter().map(|i| i.kcal).sum();
                let per_serving = total_kcal as f32 / servings.max(1) as f32;
                if per_serving > c.value {
                    violations.push(ConstraintViolation {
                        key: c.key,
                        message: format!("Per-serving kcal ({:.0}) exceeds max ({:.0})", per_serving, c.value),
                        auto_fixed: false,
                    });
                }
            }

            // ── MinProteinPerServing ─────────────────────────────────
            ConstraintKey::MinProteinPerServing => {
                let total_protein: f32 = ingredients.iter().map(|i| i.protein_g).sum();
                let per_serving = total_protein / servings.max(1) as f32;
                if per_serving < c.value {
                    violations.push(ConstraintViolation {
                        key: c.key,
                        message: format!("Per-serving protein ({:.1}g) below min ({:.0}g)", per_serving, c.value),
                        auto_fixed: false,
                    });
                }
            }
        }
    }

    violations
}

/// Lightweight snapshot of an ingredient for constraint checking.
/// We don't need the full ResolvedIngredient — just role + amounts.
#[derive(Debug, Clone)]
pub struct IngredientSnapshot {
    pub slug: String,
    pub role: IngredientRole,
    pub gross_g: f32,
    pub fat_g: f32,
    pub protein_g: f32,
    pub kcal: u32,
}

// ── Vegetable Splitter ───────────────────────────────────────────────────────

/// Split vegetables into root (long cook time) vs soft (short cook time).
///
/// Root vegetables (potato, beet, turnip…) go in first: 15 min.
/// Soft vegetables (cabbage, tomato, pepper…) go in later: 10 min.
///
/// This prevents the classic mistake of overcooking cabbage alongside potato.
pub fn split_vegetables<'a>(slugs: &'a [&str]) -> (Vec<&'a str>, Vec<&'a str>) {
    let mut roots = Vec::new();
    let mut soft = Vec::new();
    for slug in slugs {
        if is_root_vegetable(slug) {
            roots.push(*slug);
        } else {
            soft.push(*slug);
        }
    }
    (roots, soft)
}

// ── Step Text Generator ──────────────────────────────────────────────────────

/// Generate the human-readable text for a step.
/// `lang` — ISO 639-1 code: "ru", "en", "pl", "uk".
pub fn step_text(step: StepType, names: &str, lang: &str) -> String {
    match lang {
        "en" => step_text_en(step, names),
        "pl" => step_text_pl(step, names),
        "uk" => step_text_uk(step, names),
        _    => step_text_ru(step, names),
    }
}

fn step_text_ru(step: StepType, n: &str) -> String {
    match step {
        StepType::BoilProtein     => format!("Отварить {} до готовности", n),
        StepType::BraiseProtein   => format!("Потушить {} до мягкости", n),
        StepType::SearProtein     => format!("Обжарить {} до корочки", n),
        StepType::GrillProtein    => format!("Обжарить {} на гриле", n),
        StepType::MarinateProtein => format!("Замариновать {}", n),
        StepType::SauteAromatics  => format!("Сделать зажарку: спассеровать {} на масле до золотистости", n),
        StepType::AddRoots        => format!("Добавить {}, варить", n),
        StepType::AddVegetables   => format!("Добавить {}", n),
        StepType::AddAromatics    => "Добавить обжаренные овощи, перемешать".into(),
        StepType::BoilBase        => format!("Отварить {} до готовности", n),
        StepType::AddBase         => format!("Добавить {}", n),
        StepType::AddLiquid       => "Залить водой, довести до кипения".into(),
        StepType::AddSpices       => format!("Добавить {}, довести до вкуса", n),
        StepType::Combine         => "Соединить все ингредиенты, перемешать".into(),
        StepType::Rest            => "Дать настояться 5 минут, подавать".into(),
        StepType::PreheatOven     => "Разогреть духовку до 180 °C".into(),
        StepType::BakeAll         => "Запекать до готовности".into(),
        StepType::PreheatWok      => "Разогреть масло в воке на сильном огне".into(),
        StepType::PreheatGrill    => "Разогреть гриль до высокой температуры".into(),
        StepType::ChopAll         => format!("Нарезать {}", n),
        StepType::Dress           => format!("Заправить {}", n),
        StepType::ServeFresh      => "Подать свежим".into(),
    }
}

fn step_text_en(step: StepType, n: &str) -> String {
    match step {
        StepType::BoilProtein     => format!("Boil {} until done", n),
        StepType::BraiseProtein   => format!("Braise {} until tender", n),
        StepType::SearProtein     => format!("Sear {} until golden", n),
        StepType::GrillProtein    => format!("Grill {} until done", n),
        StepType::MarinateProtein => format!("Marinate {}", n),
        StepType::SauteAromatics  => format!("Sauté {} in oil until golden", n),
        StepType::AddRoots        => format!("Add {}, cook", n),
        StepType::AddVegetables   => format!("Add {}", n),
        StepType::AddAromatics    => "Add the sautéed aromatics, stir".into(),
        StepType::BoilBase        => format!("Boil {} until done", n),
        StepType::AddBase         => format!("Add {}", n),
        StepType::AddLiquid       => "Cover with water, bring to a boil".into(),
        StepType::AddSpices       => format!("Add {}, season to taste", n),
        StepType::Combine         => "Combine all ingredients, mix well".into(),
        StepType::Rest            => "Let rest for 5 minutes, serve".into(),
        StepType::PreheatOven     => "Preheat oven to 180 °C".into(),
        StepType::BakeAll         => "Bake until done".into(),
        StepType::PreheatWok      => "Heat oil in wok over high heat".into(),
        StepType::PreheatGrill    => "Preheat grill to high".into(),
        StepType::ChopAll         => format!("Chop {}", n),
        StepType::Dress           => format!("Dress with {}", n),
        StepType::ServeFresh      => "Serve fresh".into(),
    }
}

fn step_text_pl(step: StepType, n: &str) -> String {
    match step {
        StepType::BoilProtein     => format!("Gotować {} do miękkości", n),
        StepType::BraiseProtein   => format!("Dusić {} do miękkości", n),
        StepType::SearProtein     => format!("Obsmażyć {} do złotego koloru", n),
        StepType::GrillProtein    => format!("Grillować {} do gotowości", n),
        StepType::MarinateProtein => format!("Zamarynować {}", n),
        StepType::SauteAromatics  => format!("Zeszklić {} na oleju do złotego koloru", n),
        StepType::AddRoots        => format!("Dodać {}, gotować", n),
        StepType::AddVegetables   => format!("Dodać {}", n),
        StepType::AddAromatics    => "Dodać podsmażone warzywa, wymieszać".into(),
        StepType::BoilBase        => format!("Ugotować {} do miękkości", n),
        StepType::AddBase         => format!("Dodać {}", n),
        StepType::AddLiquid       => "Zalać wodą, doprowadzić do wrzenia".into(),
        StepType::AddSpices       => format!("Dodać {}, doprawić do smaku", n),
        StepType::Combine         => "Połączyć wszystkie składniki, wymieszać".into(),
        StepType::Rest            => "Odstawić na 5 minut, podawać".into(),
        StepType::PreheatOven     => "Rozgrzać piekarnik do 180 °C".into(),
        StepType::BakeAll         => "Piec do gotowości".into(),
        StepType::PreheatWok      => "Rozgrzać olej w woku na dużym ogniu".into(),
        StepType::PreheatGrill    => "Rozgrzać grill do wysokiej temperatury".into(),
        StepType::ChopAll         => format!("Pokroić {}", n),
        StepType::Dress           => format!("Polać {}", n),
        StepType::ServeFresh      => "Podawać świeże".into(),
    }
}

fn step_text_uk(step: StepType, n: &str) -> String {
    match step {
        StepType::BoilProtein     => format!("Відварити {} до готовності", n),
        StepType::BraiseProtein   => format!("Потушкувати {} до м'якості", n),
        StepType::SearProtein     => format!("Обсмажити {} до скоринки", n),
        StepType::GrillProtein    => format!("Обсмажити {} на грилі", n),
        StepType::MarinateProtein => format!("Замаринувати {}", n),
        StepType::SauteAromatics  => format!("Зробити зажарку: спасерувати {} на олії до золотистості", n),
        StepType::AddRoots        => format!("Додати {}, варити", n),
        StepType::AddVegetables   => format!("Додати {}", n),
        StepType::AddAromatics    => "Додати обсмажені овочі, перемішати".into(),
        StepType::BoilBase        => format!("Відварити {} до готовності", n),
        StepType::AddBase         => format!("Додати {}", n),
        StepType::AddLiquid       => "Залити водою, довести до кипіння".into(),
        StepType::AddSpices       => format!("Додати {}, довести до смаку", n),
        StepType::Combine         => "З'єднати всі інгредієнти, перемішати".into(),
        StepType::Rest            => "Дати настоятися 5 хвилин, подавати".into(),
        StepType::PreheatOven     => "Розігріти духовку до 180 °C".into(),
        StepType::BakeAll         => "Запікати до готовності".into(),
        StepType::PreheatWok      => "Розігріти олію у воку на сильному вогні".into(),
        StepType::PreheatGrill    => "Розігріти гриль до високої температури".into(),
        StepType::ChopAll         => format!("Нарізати {}", n),
        StepType::Dress           => format!("Заправити {}", n),
        StepType::ServeFresh      => "Подавати свіжим".into(),
    }
}

// ── Root Vegetable Detection ─────────────────────────────────────────────────

/// Is this slug a root vegetable? (used for Soup step splitting: roots vs leafy)
pub fn is_root_vegetable(slug: &str) -> bool {
    slug.contains("potato") || slug.contains("beet")
        || slug.contains("turnip") || slug.contains("parsnip")
        || slug.contains("rutabaga") || slug.contains("yam")
        || slug.contains("sweet-potato")
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_dish_types_have_rules() {
        let types = [
            DishType::Soup, DishType::Stew, DishType::Salad,
            DishType::StirFry, DishType::Grill, DishType::Bake,
            DishType::Pasta, DishType::Raw, DishType::Default,
        ];
        for dt in &types {
            let rule = load_rule(*dt);
            assert!(!rule.methods.is_empty(), "{:?} has no methods", dt);
            assert!(!rule.steps.is_empty(), "{:?} has no steps", dt);
        }
    }

    #[test]
    fn soup_aromatics_are_sauteed() {
        let rule = load_rule(DishType::Soup);
        let m = method_for_role(&rule, IngredientRole::Aromatic);
        assert_eq!(m, CookMethod::Saute);
    }

    #[test]
    fn soup_protein_is_boiled() {
        let rule = load_rule(DishType::Soup);
        let m = method_for_role(&rule, IngredientRole::Protein);
        assert_eq!(m, CookMethod::Boil);
    }

    #[test]
    fn salad_everything_raw() {
        let rule = load_rule(DishType::Salad);
        for mr in &rule.methods {
            assert_eq!(mr.method, CookMethod::Raw, "salad {:?} should be raw", mr.role);
        }
    }

    #[test]
    fn stir_fry_protein_is_fried() {
        let rule = load_rule(DishType::StirFry);
        let m = method_for_role(&rule, IngredientRole::Protein);
        assert_eq!(m, CookMethod::Fry);
    }

    #[test]
    fn grill_protein_is_grilled() {
        let rule = load_rule(DishType::Grill);
        let m = method_for_role(&rule, IngredientRole::Protein);
        assert_eq!(m, CookMethod::Grill);
    }

    #[test]
    fn pasta_base_is_boiled() {
        let rule = load_rule(DishType::Pasta);
        let m = method_for_role(&rule, IngredientRole::Base);
        assert_eq!(m, CookMethod::Boil);
    }

    #[test]
    fn role_detection_onion_is_aromatic() {
        assert_eq!(IngredientRole::from_str_role("side", "onion"), IngredientRole::Aromatic);
    }

    #[test]
    fn role_detection_carrot_is_aromatic() {
        assert_eq!(IngredientRole::from_str_role("side", "carrot"), IngredientRole::Aromatic);
    }

    #[test]
    fn role_detection_potato_is_vegetable() {
        assert_eq!(IngredientRole::from_str_role("side", "potato"), IngredientRole::Vegetable);
    }

    #[test]
    fn root_vegetable_detection() {
        assert!(is_root_vegetable("potato"));
        assert!(is_root_vegetable("beetroot"));
        assert!(!is_root_vegetable("cabbage"));
        assert!(!is_root_vegetable("tomato"));
    }

    #[test]
    fn step_text_ru() {
        let text = step_text(StepType::BoilProtein, "говядину", "ru");
        assert_eq!(text, "Отварить говядину до готовности");
    }

    #[test]
    fn step_text_en() {
        let text = step_text(StepType::BoilProtein, "beef", "en");
        assert_eq!(text, "Boil beef until done");
    }

    #[test]
    fn step_text_pl() {
        let text = step_text(StepType::BoilProtein, "wołowinę", "pl");
        assert_eq!(text, "Gotować wołowinę do miękkości");
    }

    #[test]
    fn step_text_uk() {
        let text = step_text(StepType::BoilProtein, "яловичину", "uk");
        assert_eq!(text, "Відварити яловичину до готовності");
    }

    #[test]
    fn step_text_all_langs_sear() {
        assert!(step_text(StepType::SearProtein, "X", "ru").contains("Обжарить"));
        assert!(step_text(StepType::SearProtein, "X", "en").contains("Sear"));
        assert!(step_text(StepType::SearProtein, "X", "pl").contains("Obsmażyć"));
        assert!(step_text(StepType::SearProtein, "X", "uk").contains("Обсмажити"));
    }

    #[test]
    fn step_text_static_steps_all_langs() {
        // Steps without ingredient names (static text) — check they are non-empty
        let statics = [
            StepType::AddAromatics, StepType::AddLiquid, StepType::Combine,
            StepType::Rest, StepType::PreheatOven, StepType::BakeAll,
            StepType::PreheatWok, StepType::PreheatGrill, StepType::ServeFresh,
        ];
        for st in statics {
            for lang in &["ru", "en", "pl", "uk"] {
                let text = step_text(st, "", lang);
                assert!(!text.is_empty(), "step_text({st:?}, \"\", {lang}) must be non-empty");
            }
        }
    }

    // ═══ Constraint Engine Tests ═════════════════════════════════════════

    fn make_snapshot(slug: &str, role: IngredientRole, gross_g: f32, fat_g: f32, protein_g: f32) -> IngredientSnapshot {
        IngredientSnapshot {
            slug: slug.into(),
            role,
            gross_g,
            fat_g,
            protein_g,
            kcal: ((fat_g * 9.0) + (protein_g * 4.0)) as u32,
        }
    }

    #[test]
    fn constraint_soup_requires_liquid() {
        let rule = load_rule(DishType::Soup);
        // Soup with NO water → auto-fix adds water
        let mut ings = vec![
            make_snapshot("beef", IngredientRole::Protein, 100.0, 10.0, 25.0),
            make_snapshot("potato", IngredientRole::Vegetable, 80.0, 0.1, 2.0),
            make_snapshot("onion", IngredientRole::Aromatic, 30.0, 0.0, 1.0),
        ];
        let violations = apply_constraints(&rule, &mut ings, 1);
        assert!(violations.iter().any(|v| v.key == ConstraintKey::RequiresLiquid && v.auto_fixed));
        assert!(ings.iter().any(|i| i.role == IngredientRole::Liquid));
    }

    #[test]
    fn constraint_soup_liquid_already_present() {
        let rule = load_rule(DishType::Soup);
        let mut ings = vec![
            make_snapshot("beef", IngredientRole::Protein, 100.0, 10.0, 25.0),
            make_snapshot("water", IngredientRole::Liquid, 300.0, 0.0, 0.0),
            make_snapshot("onion", IngredientRole::Aromatic, 30.0, 0.0, 1.0),
        ];
        let violations = apply_constraints(&rule, &mut ings, 1);
        assert!(!violations.iter().any(|v| v.key == ConstraintKey::RequiresLiquid));
    }

    #[test]
    fn constraint_max_oil_caps_excess() {
        let rule = load_rule(DishType::Soup); // MaxOilGrams = 15
        let mut ings = vec![
            make_snapshot("beef", IngredientRole::Protein, 100.0, 10.0, 25.0),
            make_snapshot("water", IngredientRole::Liquid, 300.0, 0.0, 0.0),
            make_snapshot("onion", IngredientRole::Aromatic, 30.0, 0.0, 1.0),
            make_snapshot("sunflower-oil", IngredientRole::Oil, 40.0, 40.0, 0.0), // 40g → should cap to 15g
        ];
        let violations = apply_constraints(&rule, &mut ings, 1);
        let oil = ings.iter().find(|i| i.role == IngredientRole::Oil).unwrap();
        assert_eq!(oil.gross_g, 15.0);
        assert!(violations.iter().any(|v| v.key == ConstraintKey::MaxOilGrams && v.auto_fixed));
    }

    #[test]
    fn constraint_min_protein_warns() {
        let rule = load_rule(DishType::Pasta); // MinProteinPerServing = 15
        let mut ings = vec![
            make_snapshot("spaghetti", IngredientRole::Base, 100.0, 1.0, 5.0), // only 5g protein
        ];
        let violations = apply_constraints(&rule, &mut ings, 1);
        assert!(violations.iter().any(|v| v.key == ConstraintKey::MinProteinPerServing && !v.auto_fixed));
    }

    #[test]
    fn constraint_fat_percent_reduces_oil() {
        let rule = load_rule(DishType::Soup); // MaxFatPercent = 15%
        let mut ings = vec![
            make_snapshot("water", IngredientRole::Liquid, 300.0, 0.0, 0.0),
            make_snapshot("potato", IngredientRole::Vegetable, 80.0, 0.1, 2.0),
            make_snapshot("onion", IngredientRole::Aromatic, 30.0, 0.0, 1.0),
            // Oil already at 15g (from MaxOilGrams cap), but total weight = 425g
            // fat% = 15/425 = 3.5% → fine, no reduction
            make_snapshot("sunflower-oil", IngredientRole::Oil, 15.0, 15.0, 0.0),
        ];
        let violations = apply_constraints(&rule, &mut ings, 1);
        // Should NOT trigger fat% reduction (3.5% < 15%)
        assert!(!violations.iter().any(|v| v.key == ConstraintKey::MaxFatPercent));
    }

    #[test]
    fn constraint_salad_no_constraints_on_liquid() {
        let rule = load_rule(DishType::Salad);
        let mut ings = vec![
            make_snapshot("tomato", IngredientRole::Vegetable, 80.0, 0.2, 1.0),
            make_snapshot("cucumber", IngredientRole::Vegetable, 80.0, 0.1, 0.7),
        ];
        let violations = apply_constraints(&rule, &mut ings, 1);
        // Salad has no RequiresLiquid → no liquid added
        assert!(!ings.iter().any(|i| i.role == IngredientRole::Liquid));
        // But salad HAS MaxOilGrams and MaxKcalPerServing
        assert!(!violations.iter().any(|v| v.key == ConstraintKey::RequiresLiquid));
    }

    #[test]
    fn constraint_default_has_none() {
        let rule = load_rule(DishType::Default);
        assert!(rule.constraints.is_empty());
    }

    // ═══ Vegetable Splitter Tests ════════════════════════════════════════

    #[test]
    fn split_vegs_roots_vs_soft() {
        let slugs = ["potato", "cabbage", "beetroot", "tomato", "sweet-potato"];
        let (roots, soft) = split_vegetables(&slugs);
        assert_eq!(roots, vec!["potato", "beetroot", "sweet-potato"]);
        assert_eq!(soft, vec!["cabbage", "tomato"]);
    }

    #[test]
    fn split_vegs_all_soft() {
        let slugs = ["cabbage", "tomato", "pepper"];
        let (roots, soft) = split_vegetables(&slugs);
        assert!(roots.is_empty());
        assert_eq!(soft.len(), 3);
    }

    #[test]
    fn split_vegs_all_roots() {
        let slugs = ["potato", "beet"];
        let (roots, soft) = split_vegetables(&slugs);
        assert_eq!(roots.len(), 2);
        assert!(soft.is_empty());
    }

    // ═══ Soup step order ═════════════════════════════════════════════════

    #[test]
    fn soup_step_order_aromatics_before_liquid() {
        let rule = load_rule(DishType::Soup);
        let saute_idx = rule.steps.iter().position(|s| s.step == StepType::SauteAromatics).unwrap();
        let liquid_idx = rule.steps.iter().position(|s| s.step == StepType::AddLiquid).unwrap();
        assert!(saute_idx < liquid_idx, "SauteAromatics should come BEFORE AddLiquid in soup");
    }

    #[test]
    fn stew_step_order_aromatics_before_liquid() {
        let rule = load_rule(DishType::Stew);
        let saute_idx = rule.steps.iter().position(|s| s.step == StepType::SauteAromatics).unwrap();
        let liquid_idx = rule.steps.iter().position(|s| s.step == StepType::AddLiquid).unwrap();
        assert!(saute_idx < liquid_idx, "SauteAromatics should come BEFORE AddLiquid in stew");
    }

    // ═══ Constraint presence tests ═══════════════════════════════════════

    #[test]
    fn soup_has_liquid_constraint() {
        let rule = load_rule(DishType::Soup);
        assert!(rule.constraints.iter().any(|c| c.key == ConstraintKey::RequiresLiquid));
    }

    #[test]
    fn soup_has_liquid_role_required() {
        let rule = load_rule(DishType::Soup);
        let liquid_role = rule.roles.iter().find(|r| r.role == IngredientRole::Liquid);
        assert!(liquid_role.is_some(), "Soup must have Liquid role");
        assert!(liquid_role.unwrap().required, "Liquid must be required in soup");
    }

    #[test]
    fn stew_has_liquid_role_required() {
        let rule = load_rule(DishType::Stew);
        let liquid_role = rule.roles.iter().find(|r| r.role == IngredientRole::Liquid);
        assert!(liquid_role.is_some());
        assert!(liquid_role.unwrap().required);
    }

    #[test]
    fn stir_fry_has_fat_constraint() {
        let rule = load_rule(DishType::StirFry);
        assert!(rule.constraints.iter().any(|c| c.key == ConstraintKey::MaxFatPercent));
    }

    #[test]
    fn grill_has_protein_constraint() {
        let rule = load_rule(DishType::Grill);
        assert!(rule.constraints.iter().any(|c| c.key == ConstraintKey::MinProteinPerServing));
    }

    #[test]
    fn pasta_has_protein_constraint() {
        let rule = load_rule(DishType::Pasta);
        assert!(rule.constraints.iter().any(|c| c.key == ConstraintKey::MinProteinPerServing));
    }
}
