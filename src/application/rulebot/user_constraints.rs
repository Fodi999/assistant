//! User Constraints — parse dietary exclusions from user text.
//!
//! Detects:
//!   - Allergen exclusions: "без лактозы", "gluten-free", "bez glutenu"
//!   - Dietary modes: "для вегана", "vegan", "pescatarian"
//!   - Specific product bans: "без сахара", "no sugar"
//!
//! Returns `UserConstraints` — a structured set of filters applied
//! BEFORE ingredient resolution in the recipe pipeline.

use serde::Serialize;
use super::intent_router::ChatLang;

// ── Dietary Mode ─────────────────────────────────────────────────────────────

/// Broad dietary preference that removes entire categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DietaryMode {
    /// No animal products at all
    Vegan,
    /// No meat/fish, dairy & eggs OK
    Vegetarian,
    /// No meat, fish & seafood OK
    Pescatarian,
}

// ── UserConstraints ──────────────────────────────────────────────────────────

/// Structured set of user dietary constraints parsed from free text.
#[derive(Debug, Clone, Default, Serialize)]
pub struct UserConstraints {
    /// Allergens to exclude: "lactose", "gluten", "nuts", "eggs", "fish", "shellfish", "soy"
    pub exclude_allergens: Vec<String>,
    /// Product types to exclude: "dairy", "meat", "fish", etc.
    pub exclude_types: Vec<String>,
    /// Specific slugs to ban: "sugar", "butter", etc.
    pub exclude_slugs: Vec<String>,
    /// Dietary mode (overrides individual exclusions)
    pub dietary_mode: Option<DietaryMode>,
    /// Raw user exclusion phrases for logging/display
    pub raw_exclusions: Vec<String>,
}

impl UserConstraints {
    pub fn is_empty(&self) -> bool {
        self.exclude_allergens.is_empty()
            && self.exclude_types.is_empty()
            && self.exclude_slugs.is_empty()
            && self.dietary_mode.is_none()
    }
}

// ── Parser ───────────────────────────────────────────────────────────────────

/// Parse user text for dietary constraints.
/// Works on the SAME input text that goes to Gemini.
///
/// Examples:
///   "борщ без лактозы"          → exclude_allergens: ["lactose"]
///   "vegan stir-fry"            → dietary_mode: Vegan
///   "pasta gluten-free"         → exclude_allergens: ["gluten"]
///   "суп без молока и сахара"   → exclude_slugs: ["milk", "sugar"]
///   "веганский борщ без орехов" → dietary_mode: Vegan + exclude_allergens: ["nuts"]
pub fn parse_constraints(input: &str, _lang: ChatLang) -> UserConstraints {
    let t = input.to_lowercase();
    let mut c = UserConstraints::default();

    // ── Dietary modes (checked first — broadest filter) ──────────────────

    // Vegan
    if contains_any(&t, &[
        "веган", "vegan", "растительн",
        "wegańsk", "weganski",
        "веганськ", "рослинн",
    ]) {
        c.dietary_mode = Some(DietaryMode::Vegan);
        c.raw_exclusions.push("vegan".into());
    }
    // Vegetarian (only if vegan not set — vegan is stricter)
    else if contains_any(&t, &[
        "вегетариан", "vegetarian",
        "wegetariańsk", "wegetarianski",
        "вегетаріан",
    ]) {
        c.dietary_mode = Some(DietaryMode::Vegetarian);
        c.raw_exclusions.push("vegetarian".into());
    }
    // Pescatarian
    else if contains_any(&t, &[
        "пескетариан", "pescatarian", "peskatarian",
        "peskatariańsk",
        "пескетаріан",
    ]) {
        c.dietary_mode = Some(DietaryMode::Pescatarian);
        c.raw_exclusions.push("pescatarian".into());
    }

    // ── Allergen exclusions ──────────────────────────────────────────────

    // Lactose / Dairy
    if contains_any(&t, &[
        "без лактоз", "без молок", "без молочн", "без сливок",
        "lactose-free", "lactose free", "dairy-free", "dairy free", "no dairy", "no lactose",
        "bez laktozy", "bez mleka", "bez nabiału",
        "без лактоз", "без молок",
    ]) {
        c.exclude_allergens.push("lactose".into());
        c.raw_exclusions.push("lactose-free".into());
    }

    // Gluten
    if contains_any(&t, &[
        "без глютен", "безглютенов", "без клейковин",
        "gluten-free", "gluten free", "no gluten",
        "bez glutenu", "bezglutenow",
        "без глютен", "безглютенов",
    ]) {
        c.exclude_allergens.push("gluten".into());
        c.raw_exclusions.push("gluten-free".into());
    }

    // Nuts
    if contains_any(&t, &[
        "без орех", "без арахис",
        "nut-free", "nut free", "no nuts", "without nuts",
        "bez orzechów", "bez orzechow",
        "без горіх",
    ]) {
        c.exclude_allergens.push("nuts".into());
        c.raw_exclusions.push("nut-free".into());
    }

    // Eggs
    if contains_any(&t, &[
        "без яиц", "без яйц",
        "egg-free", "egg free", "no eggs",
        "bez jajek", "bez jaj",
        "без яєць",
    ]) {
        c.exclude_allergens.push("eggs".into());
        c.raw_exclusions.push("egg-free".into());
    }

    // Fish
    if contains_any(&t, &[
        "без рыб",
        "fish-free", "no fish",
        "bez ryb",
        "без риб",
    ]) {
        c.exclude_allergens.push("fish".into());
        c.raw_exclusions.push("fish-free".into());
    }

    // Shellfish / Seafood
    if contains_any(&t, &[
        "без морепродукт",
        "shellfish-free", "no shellfish", "no seafood",
        "bez owoców morza",
        "без морепродукт",
    ]) {
        c.exclude_allergens.push("shellfish".into());
        c.raw_exclusions.push("shellfish-free".into());
    }

    // Soy
    if contains_any(&t, &[
        "без сои", "без соев",
        "soy-free", "soy free", "no soy",
        "bez soi",
        "без сої",
    ]) {
        c.exclude_allergens.push("soy".into());
        c.raw_exclusions.push("soy-free".into());
    }

    // ── Specific product bans ────────────────────────────────────────────

    // Sugar
    if contains_any(&t, &[
        "без сахар", "без цукр",
        "no sugar", "sugar-free", "sugar free",
        "bez cukru",
    ]) {
        c.exclude_slugs.push("sugar".into());
        c.raw_exclusions.push("sugar-free".into());
    }

    // Butter (specific, not all dairy)
    if contains_any(&t, &[
        "без масл",  // careful: could also mean "без масла растительного"
        "no butter",
        "bez masła",
    ]) && !t.contains("подсолнечн") && !t.contains("растительн") && !t.contains("оливков") {
        c.exclude_slugs.push("butter".into());
        c.raw_exclusions.push("no butter".into());
    }

    c
}

fn contains_any(text: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|p| text.contains(p))
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_lactose_free_ru() {
        let c = parse_constraints("борщ без лактозы", ChatLang::Ru);
        assert!(c.exclude_allergens.contains(&"lactose".to_string()));
    }

    #[test]
    fn parse_gluten_free_en() {
        let c = parse_constraints("gluten-free pasta with chicken", ChatLang::En);
        assert!(c.exclude_allergens.contains(&"gluten".to_string()));
    }

    #[test]
    fn parse_vegan_mode() {
        let c = parse_constraints("веганский борщ", ChatLang::Ru);
        assert_eq!(c.dietary_mode, Some(DietaryMode::Vegan));
    }

    #[test]
    fn parse_vegetarian_pl() {
        let c = parse_constraints("wegetariańska zupa", ChatLang::Pl);
        assert_eq!(c.dietary_mode, Some(DietaryMode::Vegetarian));
    }

    #[test]
    fn parse_multiple_constraints() {
        let c = parse_constraints("vegan stir-fry without nuts and soy-free", ChatLang::En);
        assert_eq!(c.dietary_mode, Some(DietaryMode::Vegan));
        assert!(c.exclude_allergens.contains(&"nuts".to_string()));
        assert!(c.exclude_allergens.contains(&"soy".to_string()));
    }

    #[test]
    fn parse_no_constraints() {
        let c = parse_constraints("приготовь борщ", ChatLang::Ru);
        assert!(c.is_empty());
    }

    #[test]
    fn parse_sugar_free() {
        let c = parse_constraints("рецепт без сахара", ChatLang::Ru);
        assert!(c.exclude_slugs.contains(&"sugar".to_string()));
    }

    #[test]
    fn parse_egg_free_uk() {
        let c = parse_constraints("борщ без яєць", ChatLang::Uk);
        assert!(c.exclude_allergens.contains(&"eggs".to_string()));
    }

    #[test]
    fn parse_dairy_free_en() {
        let c = parse_constraints("dairy-free chicken soup", ChatLang::En);
        assert!(c.exclude_allergens.contains(&"lactose".to_string()));
    }

    #[test]
    fn parse_pescatarian() {
        let c = parse_constraints("pescatarian dinner", ChatLang::En);
        assert_eq!(c.dietary_mode, Some(DietaryMode::Pescatarian));
    }
}
