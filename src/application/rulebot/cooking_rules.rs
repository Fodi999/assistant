//! Cooking Rules — DDD rules-as-data for recipe generation.
//!
//! Each `DishRule` defines HOW to cook a dish type:
//!   - Which roles are required (protein, vegetable, aromatic…)
//!   - Which cooking method per role
//!   - Step sequence (pure logic, no LLM)
//!   - Constraints (max_fat, required_liquid…)
//!
//! 7 dish types × 4 rule types ≈ 40–50 rules → covers 95% of dishes.

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
}

/// Constraint key for dish-level limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintKey {
    MaxFatPercent,
    MaxKcal,
    RequiresLiquid,
    MinProtein,
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
            RoleRule { role: Spice,     min: 0, max: 4, required: false },
            RoleRule { role: Oil,       min: 0, max: 1, required: false },
            RoleRule { role: Condiment, min: 0, max: 2, required: false },
        ],
        methods: vec![
            MethodRule { role: Protein,   method: CookMethod::Boil },
            MethodRule { role: Vegetable, method: CookMethod::Boil },
            MethodRule { role: Aromatic,  method: CookMethod::Saute },
            MethodRule { role: Base,      method: CookMethod::Boil },
            MethodRule { role: Spice,     method: CookMethod::Raw },
            MethodRule { role: Oil,       method: CookMethod::Raw },
            MethodRule { role: Condiment, method: CookMethod::Raw },
        ],
        steps: vec![
            StepRule { step: BoilProtein,     roles: vec![Protein],   time_min: Some(40) },
            StepRule { step: AddLiquid,       roles: vec![Liquid],    time_min: Some(5) },
            StepRule { step: SauteAromatics,  roles: vec![Aromatic],  time_min: Some(7) },
            StepRule { step: AddRoots,        roles: vec![Vegetable], time_min: Some(15) },
            StepRule { step: AddVegetables,   roles: vec![Vegetable], time_min: Some(10) },
            StepRule { step: AddAromatics,    roles: vec![Aromatic],  time_min: Some(2) },
            StepRule { step: AddBase,         roles: vec![Base],      time_min: Some(10) },
            StepRule { step: AddSpices,       roles: vec![Spice, Condiment], time_min: Some(5) },
            StepRule { step: Rest,            roles: vec![],          time_min: Some(5) },
        ],
        constraints: vec![],
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
            RoleRule { role: Spice,     min: 0, max: 4, required: false },
            RoleRule { role: Oil,       min: 0, max: 1, required: false },
            RoleRule { role: Condiment, min: 0, max: 2, required: false },
        ],
        methods: vec![
            MethodRule { role: Protein,   method: CookMethod::Boil },
            MethodRule { role: Vegetable, method: CookMethod::Boil },
            MethodRule { role: Aromatic,  method: CookMethod::Saute },
            MethodRule { role: Base,      method: CookMethod::Boil },
            MethodRule { role: Spice,     method: CookMethod::Raw },
            MethodRule { role: Oil,       method: CookMethod::Raw },
            MethodRule { role: Condiment, method: CookMethod::Raw },
        ],
        steps: vec![
            StepRule { step: BraiseProtein,   roles: vec![Protein],   time_min: Some(45) },
            StepRule { step: AddLiquid,       roles: vec![Liquid],    time_min: Some(5) },
            StepRule { step: SauteAromatics,  roles: vec![Aromatic],  time_min: Some(7) },
            StepRule { step: AddVegetables,   roles: vec![Vegetable], time_min: Some(20) },
            StepRule { step: AddAromatics,    roles: vec![Aromatic],  time_min: Some(2) },
            StepRule { step: AddBase,         roles: vec![Base],      time_min: Some(10) },
            StepRule { step: AddSpices,       roles: vec![Spice, Condiment], time_min: Some(5) },
            StepRule { step: Rest,            roles: vec![],          time_min: Some(5) },
        ],
        constraints: vec![],
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
            StepRule { step: ChopAll,    roles: vec![Vegetable, Protein], time_min: None },
            StepRule { step: Combine,    roles: vec![],                   time_min: None },
            StepRule { step: Dress,      roles: vec![Spice, Condiment, Oil], time_min: None },
        ],
        constraints: vec![],
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
            StepRule { step: PreheatWok,    roles: vec![Oil],       time_min: Some(2) },
            StepRule { step: SearProtein,   roles: vec![Protein],   time_min: Some(5) },
            StepRule { step: AddVegetables, roles: vec![Vegetable], time_min: Some(5) },
            StepRule { step: AddSpices,     roles: vec![Spice, Condiment], time_min: Some(2) },
            StepRule { step: AddBase,       roles: vec![Base],      time_min: None },
        ],
        constraints: vec![],
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
            StepRule { step: MarinateProtein, roles: vec![Protein],   time_min: Some(30) },
            StepRule { step: PreheatGrill,    roles: vec![],          time_min: Some(5) },
            StepRule { step: GrillProtein,    roles: vec![Protein],   time_min: Some(10) },
            StepRule { step: AddVegetables,   roles: vec![Vegetable], time_min: Some(8) },
            StepRule { step: BoilBase,        roles: vec![Base],      time_min: Some(10) },
        ],
        constraints: vec![],
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
            StepRule { step: PreheatOven,     roles: vec![],                 time_min: Some(10) },
            StepRule { step: ChopAll,         roles: vec![Protein, Vegetable], time_min: None },
            StepRule { step: BakeAll,         roles: vec![],                 time_min: Some(30) },
        ],
        constraints: vec![],
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
            StepRule { step: BoilBase,        roles: vec![Base],      time_min: Some(10) },
            StepRule { step: SearProtein,     roles: vec![Protein],   time_min: Some(8) },
            StepRule { step: AddVegetables,   roles: vec![Vegetable], time_min: Some(5) },
            StepRule { step: Combine,         roles: vec![],          time_min: Some(2) },
            StepRule { step: AddSpices,       roles: vec![Spice, Condiment], time_min: None },
        ],
        constraints: vec![],
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
            StepRule { step: ChopAll,    roles: vec![Protein, Vegetable], time_min: None },
            StepRule { step: Dress,      roles: vec![Spice, Oil],         time_min: None },
            StepRule { step: ServeFresh,  roles: vec![],                   time_min: None },
        ],
        constraints: vec![],
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
            StepRule { step: SearProtein,     roles: vec![Protein],   time_min: Some(15) },
            StepRule { step: BoilBase,        roles: vec![Base],      time_min: Some(10) },
            StepRule { step: AddVegetables,   roles: vec![Vegetable], time_min: Some(10) },
            StepRule { step: AddSpices,       roles: vec![Spice, Condiment], time_min: None },
        ],
        constraints: vec![],
    }
}

// ── Step Text Generator ──────────────────────────────────────────────────────

/// Generate the human-readable text for a step.
/// Takes the step type and the ingredient names for that step.
pub fn step_text(step: StepType, names: &str) -> String {
    match step {
        StepType::BoilProtein     => format!("Отварить {} до готовности", names),
        StepType::BraiseProtein   => format!("Потушить {} до мягкости", names),
        StepType::SearProtein     => format!("Обжарить {} до корочки", names),
        StepType::GrillProtein    => format!("Обжарить {} на гриле", names),
        StepType::MarinateProtein => format!("Замариновать {}", names),
        StepType::SauteAromatics  => format!("Сделать зажарку: спассеровать {} на масле до золотистости", names),
        StepType::AddRoots        => format!("Добавить {}, варить", names),
        StepType::AddVegetables   => format!("Добавить {}", names),
        StepType::AddAromatics    => "Добавить зажарку в суп, перемешать".to_string(),
        StepType::BoilBase        => format!("Отварить {} до готовности", names),
        StepType::AddBase         => format!("Добавить {}", names),
        StepType::AddLiquid       => "Залить водой, довести до кипения".to_string(),
        StepType::AddSpices       => format!("Добавить {}, довести до вкуса", names),
        StepType::Combine         => "Соединить все ингредиенты, перемешать".to_string(),
        StepType::Rest            => "Дать настояться 5 минут, подавать".to_string(),
        StepType::PreheatOven     => "Разогреть духовку до 180°C".to_string(),
        StepType::BakeAll         => "Запекать до готовности".to_string(),
        StepType::PreheatWok      => "Разогреть масло в воке на сильном огне".to_string(),
        StepType::PreheatGrill    => "Разогреть гриль до высокой температуры".to_string(),
        StepType::ChopAll         => format!("Нарезать {}", names),
        StepType::Dress           => format!("Заправить {}", names),
        StepType::ServeFresh      => "Подать свежим".to_string(),
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
    fn step_text_generation() {
        let text = step_text(StepType::BoilProtein, "говядину");
        assert_eq!(text, "Отварить говядину до готовности");
    }
}
