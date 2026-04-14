//! Goal Modifier — DDD context layer for user goals.
//!
//! Expanded modifiers detected from user text.
//! Maps to HealthGoal (3 core variants) for business logic.
//!
//! ```text
//! User says         → Modifier        → HealthGoal (business)
//! "лёгкое блюдо"   → LowCalorie      → LowCalorie
//! "быстро"          → Quick           → Balanced
//! "дёшево"          → Budget          → Balanced
//! "сытное"          → ComfortFood     → Balanced
//! "богато клетчаткой"→ HighFiber      → LowCalorie (fiber = diet-adjacent)
//! "кето / без углев"→ LowCarb         → LowCalorie (low-carb = calorie-aware)
//! "на массу"        → HighProtein     → HighProtein
//! ```

use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════════════════
//  EXPANDED HEALTH MODIFIER (user-facing, many variants)
// ═══════════════════════════════════════════════════════════════════════════════

/// Additional context modifier extracted alongside the primary intent.
/// Influences product selection, recipe style, and response text.
///
/// This is the **user-facing** enum — can have many nuanced variants.
/// Maps to `HealthGoal` (3 variants) for core business logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthModifier {
    /// "на массу", "набрать мышцы", "protein", "muscle"
    HighProtein,
    /// "похудеть", "сушиться", "диета", "лёгкое", "light"
    LowCalorie,
    /// "быстрое", "quick", "fast", "за 15 минут"
    Quick,
    /// "бюджетное", "дешевле", "cheap", "budget"
    Budget,
    /// "сытное", "comfort food", "домашнее"
    ComfortFood,
    /// "клетчатка", "fiber", "high fiber"
    HighFiber,
    /// "кето", "без углеводов", "low carb"
    LowCarb,
    /// No specific modifier detected
    None,
}

impl HealthModifier {
    pub fn label(&self) -> &'static str {
        match self {
            Self::HighProtein  => "high_protein",
            Self::LowCalorie   => "low_calorie",
            Self::Quick        => "quick",
            Self::Budget       => "budget",
            Self::ComfortFood  => "comfort_food",
            Self::HighFiber    => "high_fiber",
            Self::LowCarb      => "low_carb",
            Self::None         => "none",
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
//  MODIFIER DETECTION — keyword-based
// ═══════════════════════════════════════════════════════════════════════════════

/// Keyword groups for each modifier. Order = priority (first match wins).
struct ModifierRule {
    modifier: HealthModifier,
    keywords: &'static [&'static str],
}

const MODIFIER_RULES: &[ModifierRule] = &[
    // ── HighProtein (check first — explicit muscle/protein intent) ────────
    ModifierRule {
        modifier: HealthModifier::HighProtein,
        keywords: &[
            "на массу", "набрать мышц", "набрать мус", "для мышц", "мышечн",
            "on bulk", "bulk", "muscle", "high protein", "для белка", "протеин",
            "na masę", "na mase", "budowanie mięśni",
            "набрати масу", "для м'язів", "на білок",
        ],
    },
    // ── LowCalorie / diet / light ────────────────────────────────────────
    ModifierRule {
        modifier: HealthModifier::LowCalorie,
        keywords: &[
            "сушиться", "сушка", "сушк", "похудеть", "похудения", "для похудения",
            "диет", "сбросить вес", "lose weight", "weight loss", "cutting",
            "low calorie", "low cal", "mało kalorii", "odchudzanie", "schudnąć",
            "лёгк", "легк", "light",
            // Ukrainian
            "схуднути", "схуднення", "для схуднення", "скинути вагу",
            "худнути", "худіти", "на дієті",
        ],
    },
    // ── LowCarb / keto ──────────────────────────────────────────────────
    ModifierRule {
        modifier: HealthModifier::LowCarb,
        keywords: &[
            "без углевод", "low carb", "lowcarb", "кето", "keto",
            "bez węglowodan", "без вуглевод", "niskougle",
        ],
    },
    // ── HighFiber ────────────────────────────────────────────────────────
    ModifierRule {
        modifier: HealthModifier::HighFiber,
        keywords: &[
            "клетчатк", "fiber", "high fiber", "błonnik",
            "клітковин", "богат клетчатк", "rich in fiber",
        ],
    },
    // ── ComfortFood / hearty / сытное ────────────────────────────────────
    ModifierRule {
        modifier: HealthModifier::ComfortFood,
        keywords: &[
            "сытн", "comfort food", "comfort", "домашн", "hearty",
            "наваристый", "наваристое", "густой", "густое",
            "pożywn", "domow", "ситн",
        ],
    },
    // ── Quick ────────────────────────────────────────────────────────────
    ModifierRule {
        modifier: HealthModifier::Quick,
        keywords: &[
            "быстр", "за 15 минут", "за 20 минут", "за 10 минут", "за 5 минут",
            "quick", "fast", "szybk", "szybkie", "на скорую", "скорый",
            "швидк", "за 15 хвилин",
        ],
    },
    // ── Budget ───────────────────────────────────────────────────────────
    ModifierRule {
        modifier: HealthModifier::Budget,
        keywords: &[
            "бюджетн", "дешевле", "дешево", "дёшево", "cheap", "budget",
            "tanie", "tani", "недорого", "економ", "дешев",
        ],
    },
];

/// Detect goal/context modifier from lowercased text.
pub fn detect_modifier(text: &str) -> HealthModifier {
    for rule in MODIFIER_RULES {
        if rule.keywords.iter().any(|kw| text.contains(kw)) {
            return rule.modifier;
        }
    }
    HealthModifier::None
}

// ═══════════════════════════════════════════════════════════════════════════════
//  TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── LowCalorie ──
    #[test] fn mod_diet_ru()    { assert_eq!(detect_modifier("хочу похудеть"),                   HealthModifier::LowCalorie); }
    #[test] fn mod_light_ru()   { assert_eq!(detect_modifier("приготовь лёгкое блюдо с треска"), HealthModifier::LowCalorie); }
    #[test] fn mod_light_en()   { assert_eq!(detect_modifier("cook a light dish with cod"),      HealthModifier::LowCalorie); }
    #[test] fn mod_light_uk()   { assert_eq!(detect_modifier("легку страву з тріски"),           HealthModifier::LowCalorie); }
    #[test] fn mod_sushka()     { assert_eq!(detect_modifier("питание на сушку"),                HealthModifier::LowCalorie); }

    // ── HighProtein ──
    #[test] fn mod_mass_ru()    { assert_eq!(detect_modifier("что есть на массу"),               HealthModifier::HighProtein); }
    #[test] fn mod_muscle_en()  { assert_eq!(detect_modifier("high protein foods for muscle"),   HealthModifier::HighProtein); }
    #[test] fn mod_bulk()       { assert_eq!(detect_modifier("i'm on bulk"),                     HealthModifier::HighProtein); }

    // ── Quick ──
    #[test] fn mod_quick_ru()   { assert_eq!(detect_modifier("быстрый завтрак"),                 HealthModifier::Quick); }
    #[test] fn mod_quick_en()   { assert_eq!(detect_modifier("quick dinner idea"),               HealthModifier::Quick); }
    #[test] fn mod_fast_pl()    { assert_eq!(detect_modifier("szybkie danie"),                   HealthModifier::Quick); }
    #[test] fn mod_15min()      { assert_eq!(detect_modifier("рецепт за 15 минут"),              HealthModifier::Quick); }

    // ── Budget ──
    #[test] fn mod_cheap_ru()   { assert_eq!(detect_modifier("что-то дешевле"),                  HealthModifier::Budget); }
    #[test] fn mod_budget_en()  { assert_eq!(detect_modifier("budget meal ideas"),               HealthModifier::Budget); }
    #[test] fn mod_tanie_pl()   { assert_eq!(detect_modifier("tanie danie"),                     HealthModifier::Budget); }

    // ── ComfortFood ──
    #[test] fn mod_comfort_ru() { assert_eq!(detect_modifier("что-нибудь сытное"),               HealthModifier::ComfortFood); }
    #[test] fn mod_comfort_en() { assert_eq!(detect_modifier("comfort food please"),             HealthModifier::ComfortFood); }
    #[test] fn mod_hearty()     { assert_eq!(detect_modifier("hearty soup recipe"),              HealthModifier::ComfortFood); }
    #[test] fn mod_domashn()    { assert_eq!(detect_modifier("домашнее блюдо"),                  HealthModifier::ComfortFood); }

    // ── HighFiber ──
    #[test] fn mod_fiber_ru()   { assert_eq!(detect_modifier("богато клетчаткой"),               HealthModifier::HighFiber); }
    #[test] fn mod_fiber_en()   { assert_eq!(detect_modifier("high fiber breakfast"),            HealthModifier::HighFiber); }
    #[test] fn mod_fiber_pl()   { assert_eq!(detect_modifier("dużo błonnika"),                   HealthModifier::HighFiber); }

    // ── LowCarb ──
    #[test] fn mod_keto_ru()    { assert_eq!(detect_modifier("кето рецепт"),                     HealthModifier::LowCarb); }
    #[test] fn mod_keto_en()    { assert_eq!(detect_modifier("keto dinner"),                     HealthModifier::LowCarb); }
    #[test] fn mod_lowcarb()    { assert_eq!(detect_modifier("low carb meal"),                   HealthModifier::LowCarb); }
    #[test] fn mod_no_carbs()   { assert_eq!(detect_modifier("блюдо без углеводов"),             HealthModifier::LowCarb); }

    // ── None ──
    #[test] fn mod_none()       { assert_eq!(detect_modifier("приготовь борщ"),                  HealthModifier::None); }
    #[test] fn mod_none_en()    { assert_eq!(detect_modifier("recipe for borscht"),              HealthModifier::None); }
}
