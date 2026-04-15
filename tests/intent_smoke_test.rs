//! Smoke test: 4 languages × 3 scenarios = 12 intent checks.
//! Run: cargo test --test intent_smoke_test -- --nocapture

use restaurant_backend::application::rulebot::intent_router::{
    detect_intent, detect_intent_scored, parse_input, detect_language, Intent, ChatLang,
};
use restaurant_backend::application::rulebot::goal_modifier::{detect_modifier, HealthModifier};

fn check(label: &str, input: &str, expected_intent: Intent) {
    let scored = detect_intent_scored(input);
    let parsed = parse_input(input);
    let lang = detect_language(input);
    let modifier = detect_modifier(&input.to_lowercase());

    let status = if scored.intent == expected_intent { "✅" } else { "❌" };

    println!(
        "{} {} | lang={:?} | intent={:?}(score={}) | modifier={:?} | intents={:?}\n   input: \"{}\"",
        status, label, lang, scored.intent, scored.score, modifier, parsed.intents, input
    );

    assert_eq!(
        scored.intent, expected_intent,
        "\n  FAIL {}: expected {:?}, got {:?} (score={})\n  input: \"{}\"",
        label, expected_intent, scored.intent, scored.score, input
    );
}

#[test]
fn test_weight_loss_4_languages() {
    println!("\n═══════════════════════════════════════════════════════");
    println!("  🎯 СЦЕНАРИЙ 1: ПОХУДЕНИЕ (Weight Loss)");
    println!("═══════════════════════════════════════════════════════\n");

    check("RU похудение", "хочу похудеть", Intent::HealthyProduct);
    check("EN weight loss", "I want to lose weight", Intent::HealthyProduct);
    check("PL odchudzanie", "chcę schudnąć", Intent::HealthyProduct);
    check("UK схуднення", "хочу схуднути", Intent::HealthyProduct);

    println!("\n--- С рецептом ---\n");

    check("RU похудение+рецепт", "приготовь лёгкий суп для похудения", Intent::RecipeHelp);
    check("EN weight loss+recipe", "make a light soup for weight loss", Intent::RecipeHelp);
    check("PL odchudzanie+przepis", "ugotuj lekką zupę na odchudzanie", Intent::RecipeHelp);
    check("UK схуднення+рецепт", "приготуй легкий суп для схуднення", Intent::RecipeHelp);

    println!("\n--- С потребностью (goal+need → MealIdea) ---\n");

    check("RU need", "хочу похудеть, но нужно много белка", Intent::MealIdea);
    check("EN need", "want to lose weight but need high protein", Intent::MealIdea);
    check("PL need", "chcę schudnąć, ale potrzebuję dużo białka", Intent::MealIdea);
    check("UK need", "хочу схуднути, але потрібно багато білка", Intent::MealIdea);
}

#[test]
fn test_muscle_gain_4_languages() {
    println!("\n═══════════════════════════════════════════════════════");
    println!("  💪 СЦЕНАРИЙ 2: НАБОР МАССЫ (Muscle Gain)");
    println!("═══════════════════════════════════════════════════════\n");

    check("RU масса", "хочу набрать массу", Intent::HealthyProduct);
    check("EN muscle", "I want to gain muscle mass", Intent::HealthyProduct);
    check("PL masa", "chcę nabrać masy mięśniowej", Intent::HealthyProduct);
    check("UK маса", "хочу набрати масу", Intent::HealthyProduct);

    println!("\n--- С рецептом ---\n");

    check("RU масса+рецепт", "приготовь высокобелковый салат на массу", Intent::RecipeHelp);
    check("EN muscle+recipe", "make a high protein salad for muscle gain", Intent::RecipeHelp);
    check("PL masa+przepis", "przygotuj sałatkę wysokobiałkową na masę", Intent::RecipeHelp);
    check("UK маса+рецепт", "приготуй високобілковий салат на масу", Intent::RecipeHelp);

    println!("\n--- Что приготовить? (goal+action → MealIdea) ---\n");

    check("RU масса+meal", "хочу на массу, что приготовить?", Intent::MealIdea);
    check("EN muscle+meal", "high protein meal for dinner", Intent::MealIdea);
    check("PL masa+meal", "co ugotować na masę na obiad?", Intent::MealIdea);
    check("UK маса+meal", "що приготувати на масу на вечерю?", Intent::MealIdea);
}

#[test]
fn test_gluten_free_4_languages() {
    println!("\n═══════════════════════════════════════════════════════");
    println!("  🌾 СЦЕНАРИЙ 3: БЕЗ ГЛЮТЕНА (Gluten Free)");
    println!("═══════════════════════════════════════════════════════\n");

    check("RU рецепт без глютена", "рецепт без глютена", Intent::RecipeHelp);
    check("EN gluten free recipe", "gluten free recipe", Intent::RecipeHelp);
    check("PL przepis bezglutenowy", "przepis bezglutenowy", Intent::RecipeHelp);
    check("UK рецепт без глютену", "рецепт без глютену", Intent::RecipeHelp);

    println!("\n--- Что приготовить без глютена? ---\n");

    check("RU что без глютена", "что приготовить без глютена на ужин?", Intent::MealIdea);
    check("EN what gluten free", "what to cook gluten free for dinner?", Intent::MealIdea);
    check("PL co bezglutenowe", "co ugotować bezglutenowe na kolację?", Intent::MealIdea);
    check("UK що без глютену", "що приготувати без глютену на вечерю?", Intent::MealIdea);
}

#[test]
fn test_modifier_detection_4_languages() {
    println!("\n═══════════════════════════════════════════════════════");
    println!("  🏷 МОДИФИКАТОРЫ (Modifier Detection)");
    println!("═══════════════════════════════════════════════════════\n");

    let cases: &[(&str, &str, HealthModifier)] = &[
        ("RU похудение", "хочу похудеть", HealthModifier::LowCalorie),
        ("EN weight loss", "lose weight", HealthModifier::LowCalorie),
        ("PL odchudzanie", "chcę schudnąć", HealthModifier::LowCalorie),
        ("UK схуднення", "хочу схуднути", HealthModifier::LowCalorie),

        ("RU на массу", "на массу", HealthModifier::HighProtein),
        ("EN muscle", "muscle gain", HealthModifier::HighProtein),
        ("PL masa", "nabrać masy", HealthModifier::HighProtein),
        ("UK маса", "набрати масу", HealthModifier::HighProtein),

        ("RU кето", "кето рецепт", HealthModifier::LowCarb),
        ("EN keto", "keto recipe", HealthModifier::LowCarb),
        ("RU клетчатка", "богат клетчаткой", HealthModifier::HighFiber),
        ("EN fiber", "high fiber dish", HealthModifier::HighFiber),
    ];

    for (label, input, expected) in cases {
        let modifier = detect_modifier(&input.to_lowercase());
        let status = if modifier == *expected { "✅" } else { "❌" };
        println!("{} {} | modifier={:?} (expected {:?}) | input: \"{}\"",
            status, label, modifier, expected, input);
        assert_eq!(modifier, *expected, "FAIL {}: input \"{}\"", label, input);
    }
}

#[test]
fn test_language_detection() {
    println!("\n═══════════════════════════════════════════════════════");
    println!("  🌍 ОПРЕДЕЛЕНИЕ ЯЗЫКА (Language Detection)");
    println!("═══════════════════════════════════════════════════════\n");

    let cases: &[(&str, ChatLang)] = &[
        ("Привет, хочу похудеть", ChatLang::Ru),
        ("Hello, I want to lose weight", ChatLang::En),
        ("Cześć, chcę schudnąć", ChatLang::Pl),
        ("Привіт, хочу схуднути", ChatLang::Uk),
        ("рецепт борща", ChatLang::Ru),
        ("tomato soup recipe", ChatLang::En),
        ("przepis na zupę pomidorową", ChatLang::Pl),
        ("рецепт борщу з буряком", ChatLang::Uk),
    ];

    for (input, expected) in cases {
        let lang = detect_language(input);
        let status = if lang == *expected { "✅" } else { "❌" };
        println!("{} lang={:?} (expected {:?}) | \"{}\"", status, lang, expected, input);
        assert_eq!(lang, *expected, "FAIL: input \"{}\"", input);
    }
}
