//! Sous-Chef Planner — single-shot meal plan generator.
//!
//! Architecture:
//!   80% Rust (parsing, normalization, nutrition math, recipe selection)
//!   20% Gemini Flash (intro text, explanation, motivation — only on cache miss)
//!
//! Flow:
//!   user input → normalize → cache key → cache? → YES: instant
//!                                                → NO:  Rust + Gemini → cache → respond
//!
//! Cache: DB-backed (ai_cache table, 90-day TTL) + in-memory AppCache (5 min).

use crate::infrastructure::llm_adapter::LlmAdapter;
use crate::infrastructure::persistence::AiCacheRepository;
use crate::shared::AppError;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;

// ── Public types ──────────────────────────────────────────────────────────────

/// Incoming request from frontend.
#[derive(Debug, Deserialize)]
pub struct PlanRequest {
    /// Free-text user goal, e.g. "Хочу похудеть — меню на 1 день"
    pub query: String,
    /// UI language: pl, en, ru, uk
    pub lang: Option<String>,
}

/// One recipe variant in the plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MealVariant {
    pub level: String,          // "light" | "balanced" | "rich"
    pub emoji: String,          // 🟢 🟡 🔴
    pub title: String,          // "Лёгкий"
    pub short_description: String, // "овсянка + банан + чай"
    pub calories: u32,
    pub protein_g: u32,
    pub fat_g: u32,
    pub carbs_g: u32,
    pub ingredients: Vec<MealIngredient>,
}

/// Single ingredient in a recipe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MealIngredient {
    pub name: String,   // localized
    pub amount: String,  // "150г", "1 cup", etc.
    pub calories: u32,
}

/// Full plan response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MealPlan {
    /// Normalized cache key (for debugging / analytics)
    pub cache_key: String,
    /// Was this served from cache?
    pub cached: bool,
    /// Chef intro text (Gemini)
    pub chef_intro: String,
    /// 3 recipe variants
    pub variants: Vec<MealVariant>,
    /// Why this works (Gemini)
    pub explanation: String,
    /// Motivation paragraph (Gemini)
    pub motivation: String,
    /// Detected goal category
    pub goal: String,
    /// Language
    pub lang: String,
}

// ── Goal detection ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Goal {
    WeightLoss,
    HighProtein,
    QuickBreakfast,
    FromIngredients,
    HealthyDay,
    Generic,
}

impl Goal {
    fn slug(&self) -> &'static str {
        match self {
            Goal::WeightLoss => "weight_loss",
            Goal::HighProtein => "high_protein",
            Goal::QuickBreakfast => "quick_breakfast",
            Goal::FromIngredients => "from_ingredients",
            Goal::HealthyDay => "healthy_day",
            Goal::Generic => "generic",
        }
    }

    fn detect(q: &str) -> Self {
        let q = q.to_lowercase();
        // Weight loss keywords (ru, pl, en, uk)
        if q.contains("похуд") || q.contains("дефицит") || q.contains("калори")
            || q.contains("odchudz") || q.contains("schudnąć") || q.contains("weight loss")
            || q.contains("lose weight") || q.contains("схуднути") || q.contains("калорій")
        {
            return Goal::WeightLoss;
        }
        // High protein
        if q.contains("белок") || q.contains("белк") || q.contains("протеин")
            || q.contains("protein") || q.contains("białko") || q.contains("білок")
        {
            return Goal::HighProtein;
        }
        // Quick breakfast
        if q.contains("завтрак") || q.contains("śniadani") || q.contains("breakfast")
            || q.contains("сніданок") || q.contains("быстр") || q.contains("szybk")
            || q.contains("quick")
        {
            return Goal::QuickBreakfast;
        }
        // From ingredients ("что приготовить из...")
        if q.contains("приготовить из") || q.contains("з чого") || q.contains("what to cook")
            || q.contains("co ugotować") || q.contains("из:") || q.contains("from:")
        {
            return Goal::FromIngredients;
        }
        // Healthy day
        if q.contains("здоров") || q.contains("zdrowy") || q.contains("healthy")
            || q.contains("здоров")
        {
            return Goal::HealthyDay;
        }
        Goal::Generic
    }
}

// ── Normalization (query → cache key) ─────────────────────────────────────────

fn normalize_query(q: &str) -> String {
    let q = q.trim().to_lowercase();
    // Remove punctuation, extra spaces
    let q: String = q
        .chars()
        .map(|c| if c.is_alphanumeric() || c == ' ' || c == ':' || c == ',' { c } else { ' ' })
        .collect();
    // Collapse multiple spaces
    q.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Public helper for in-memory cache key generation by handlers.
pub fn normalize_for_cache(q: &str) -> String {
    let norm = normalize_query(q);
    let goal = Goal::detect(&norm);
    goal.slug().to_string()
}

fn build_cache_key(goal: Goal, lang: &str) -> String {
    format!("sous_chef:{}:{}", goal.slug(), lang)
}

// ── Predefined meal templates (Rust — 0 cost) ────────────────────────────────
// These are nutritionist-validated templates. Gemini only adds personality text.

fn weight_loss_variants(lang: &str) -> Vec<MealVariant> {
    match lang {
        "ru" => vec![
            MealVariant {
                level: "light".into(), emoji: "🟢".into(),
                title: "Лёгкий".into(),
                short_description: "овсянка + банан + зелёный чай".into(),
                calories: 320, protein_g: 10, fat_g: 6, carbs_g: 58,
                ingredients: vec![
                    MealIngredient { name: "Овсянка".into(), amount: "60г".into(), calories: 230 },
                    MealIngredient { name: "Банан".into(), amount: "1 шт".into(), calories: 70 },
                    MealIngredient { name: "Зелёный чай".into(), amount: "250мл".into(), calories: 2 },
                ],
            },
            MealVariant {
                level: "balanced".into(), emoji: "🟡".into(),
                title: "Баланс".into(),
                short_description: "курица + рис + овощи".into(),
                calories: 480, protein_g: 35, fat_g: 10, carbs_g: 55,
                ingredients: vec![
                    MealIngredient { name: "Куриная грудка".into(), amount: "150г".into(), calories: 230 },
                    MealIngredient { name: "Рис бурый".into(), amount: "80г".into(), calories: 180 },
                    MealIngredient { name: "Овощи на пару".into(), amount: "150г".into(), calories: 50 },
                ],
            },
            MealVariant {
                level: "rich".into(), emoji: "🔴".into(),
                title: "Сытный".into(),
                short_description: "лосось + киноа + авокадо".into(),
                calories: 650, protein_g: 38, fat_g: 30, carbs_g: 52,
                ingredients: vec![
                    MealIngredient { name: "Лосось".into(), amount: "150г".into(), calories: 300 },
                    MealIngredient { name: "Киноа".into(), amount: "80г".into(), calories: 200 },
                    MealIngredient { name: "Авокадо".into(), amount: "½ шт".into(), calories: 120 },
                ],
            },
        ],
        "pl" => vec![
            MealVariant {
                level: "light".into(), emoji: "🟢".into(),
                title: "Lekki".into(),
                short_description: "owsianka + banan + zielona herbata".into(),
                calories: 320, protein_g: 10, fat_g: 6, carbs_g: 58,
                ingredients: vec![
                    MealIngredient { name: "Płatki owsiane".into(), amount: "60g".into(), calories: 230 },
                    MealIngredient { name: "Banan".into(), amount: "1 szt".into(), calories: 70 },
                    MealIngredient { name: "Herbata zielona".into(), amount: "250ml".into(), calories: 2 },
                ],
            },
            MealVariant {
                level: "balanced".into(), emoji: "🟡".into(),
                title: "Balans".into(),
                short_description: "kurczak + ryż + warzywa".into(),
                calories: 480, protein_g: 35, fat_g: 10, carbs_g: 55,
                ingredients: vec![
                    MealIngredient { name: "Pierś z kurczaka".into(), amount: "150g".into(), calories: 230 },
                    MealIngredient { name: "Ryż brązowy".into(), amount: "80g".into(), calories: 180 },
                    MealIngredient { name: "Warzywa na parze".into(), amount: "150g".into(), calories: 50 },
                ],
            },
            MealVariant {
                level: "rich".into(), emoji: "🔴".into(),
                title: "Sycący".into(),
                short_description: "łosoś + quinoa + awokado".into(),
                calories: 650, protein_g: 38, fat_g: 30, carbs_g: 52,
                ingredients: vec![
                    MealIngredient { name: "Łosoś".into(), amount: "150g".into(), calories: 300 },
                    MealIngredient { name: "Quinoa".into(), amount: "80g".into(), calories: 200 },
                    MealIngredient { name: "Awokado".into(), amount: "½ szt".into(), calories: 120 },
                ],
            },
        ],
        "uk" => vec![
            MealVariant {
                level: "light".into(), emoji: "🟢".into(),
                title: "Легкий".into(),
                short_description: "вівсянка + банан + зелений чай".into(),
                calories: 320, protein_g: 10, fat_g: 6, carbs_g: 58,
                ingredients: vec![
                    MealIngredient { name: "Вівсянка".into(), amount: "60г".into(), calories: 230 },
                    MealIngredient { name: "Банан".into(), amount: "1 шт".into(), calories: 70 },
                    MealIngredient { name: "Зелений чай".into(), amount: "250мл".into(), calories: 2 },
                ],
            },
            MealVariant {
                level: "balanced".into(), emoji: "🟡".into(),
                title: "Баланс".into(),
                short_description: "курка + рис + овочі".into(),
                calories: 480, protein_g: 35, fat_g: 10, carbs_g: 55,
                ingredients: vec![
                    MealIngredient { name: "Куряче філе".into(), amount: "150г".into(), calories: 230 },
                    MealIngredient { name: "Рис бурий".into(), amount: "80г".into(), calories: 180 },
                    MealIngredient { name: "Овочі на парі".into(), amount: "150г".into(), calories: 50 },
                ],
            },
            MealVariant {
                level: "rich".into(), emoji: "🔴".into(),
                title: "Ситний".into(),
                short_description: "лосось + кіноа + авокадо".into(),
                calories: 650, protein_g: 38, fat_g: 30, carbs_g: 52,
                ingredients: vec![
                    MealIngredient { name: "Лосось".into(), amount: "150г".into(), calories: 300 },
                    MealIngredient { name: "Кіноа".into(), amount: "80г".into(), calories: 200 },
                    MealIngredient { name: "Авокадо".into(), amount: "½ шт".into(), calories: 120 },
                ],
            },
        ],
        _ => vec![ // English default
            MealVariant {
                level: "light".into(), emoji: "🟢".into(),
                title: "Light".into(),
                short_description: "oatmeal + banana + green tea".into(),
                calories: 320, protein_g: 10, fat_g: 6, carbs_g: 58,
                ingredients: vec![
                    MealIngredient { name: "Oatmeal".into(), amount: "60g".into(), calories: 230 },
                    MealIngredient { name: "Banana".into(), amount: "1 pc".into(), calories: 70 },
                    MealIngredient { name: "Green tea".into(), amount: "250ml".into(), calories: 2 },
                ],
            },
            MealVariant {
                level: "balanced".into(), emoji: "🟡".into(),
                title: "Balanced".into(),
                short_description: "chicken + rice + vegetables".into(),
                calories: 480, protein_g: 35, fat_g: 10, carbs_g: 55,
                ingredients: vec![
                    MealIngredient { name: "Chicken breast".into(), amount: "150g".into(), calories: 230 },
                    MealIngredient { name: "Brown rice".into(), amount: "80g".into(), calories: 180 },
                    MealIngredient { name: "Steamed vegetables".into(), amount: "150g".into(), calories: 50 },
                ],
            },
            MealVariant {
                level: "rich".into(), emoji: "🔴".into(),
                title: "Hearty".into(),
                short_description: "salmon + quinoa + avocado".into(),
                calories: 650, protein_g: 38, fat_g: 30, carbs_g: 52,
                ingredients: vec![
                    MealIngredient { name: "Salmon".into(), amount: "150g".into(), calories: 300 },
                    MealIngredient { name: "Quinoa".into(), amount: "80g".into(), calories: 200 },
                    MealIngredient { name: "Avocado".into(), amount: "½ pc".into(), calories: 120 },
                ],
            },
        ],
    }
}

fn high_protein_variants(lang: &str) -> Vec<MealVariant> {
    match lang {
        "ru" => vec![
            MealVariant {
                level: "light".into(), emoji: "🟢".into(),
                title: "Лёгкий".into(),
                short_description: "творог + ягоды + орехи".into(),
                calories: 350, protein_g: 30, fat_g: 12, carbs_g: 25,
                ingredients: vec![
                    MealIngredient { name: "Творог 5%".into(), amount: "200г".into(), calories: 220 },
                    MealIngredient { name: "Ягоды".into(), amount: "100г".into(), calories: 50 },
                    MealIngredient { name: "Грецкие орехи".into(), amount: "20г".into(), calories: 130 },
                ],
            },
            MealVariant {
                level: "balanced".into(), emoji: "🟡".into(),
                title: "Баланс".into(),
                short_description: "индейка + гречка + брокколи".into(),
                calories: 520, protein_g: 45, fat_g: 12, carbs_g: 48,
                ingredients: vec![
                    MealIngredient { name: "Индейка".into(), amount: "180г".into(), calories: 250 },
                    MealIngredient { name: "Гречка".into(), amount: "80г".into(), calories: 200 },
                    MealIngredient { name: "Брокколи".into(), amount: "120г".into(), calories: 40 },
                ],
            },
            MealVariant {
                level: "rich".into(), emoji: "🔴".into(),
                title: "Сытный".into(),
                short_description: "стейк + батат + шпинат".into(),
                calories: 720, protein_g: 50, fat_g: 28, carbs_g: 55,
                ingredients: vec![
                    MealIngredient { name: "Говяжий стейк".into(), amount: "200г".into(), calories: 400 },
                    MealIngredient { name: "Батат".into(), amount: "150г".into(), calories: 180 },
                    MealIngredient { name: "Шпинат".into(), amount: "100г".into(), calories: 25 },
                ],
            },
        ],
        "pl" => vec![
            MealVariant {
                level: "light".into(), emoji: "🟢".into(),
                title: "Lekki".into(),
                short_description: "twaróg + jagody + orzechy".into(),
                calories: 350, protein_g: 30, fat_g: 12, carbs_g: 25,
                ingredients: vec![
                    MealIngredient { name: "Twaróg".into(), amount: "200g".into(), calories: 220 },
                    MealIngredient { name: "Jagody".into(), amount: "100g".into(), calories: 50 },
                    MealIngredient { name: "Orzechy włoskie".into(), amount: "20g".into(), calories: 130 },
                ],
            },
            MealVariant {
                level: "balanced".into(), emoji: "🟡".into(),
                title: "Balans".into(),
                short_description: "indyk + kasza gryczana + brokuły".into(),
                calories: 520, protein_g: 45, fat_g: 12, carbs_g: 48,
                ingredients: vec![
                    MealIngredient { name: "Filet z indyka".into(), amount: "180g".into(), calories: 250 },
                    MealIngredient { name: "Kasza gryczana".into(), amount: "80g".into(), calories: 200 },
                    MealIngredient { name: "Brokuły".into(), amount: "120g".into(), calories: 40 },
                ],
            },
            MealVariant {
                level: "rich".into(), emoji: "🔴".into(),
                title: "Sycący".into(),
                short_description: "stek wołowy + batat + szpinak".into(),
                calories: 720, protein_g: 50, fat_g: 28, carbs_g: 55,
                ingredients: vec![
                    MealIngredient { name: "Stek wołowy".into(), amount: "200g".into(), calories: 400 },
                    MealIngredient { name: "Batat".into(), amount: "150g".into(), calories: 180 },
                    MealIngredient { name: "Szpinak".into(), amount: "100g".into(), calories: 25 },
                ],
            },
        ],
        _ => vec![ // English + Ukrainian (same structure)
            MealVariant {
                level: "light".into(), emoji: "🟢".into(),
                title: "Light".into(),
                short_description: "cottage cheese + berries + nuts".into(),
                calories: 350, protein_g: 30, fat_g: 12, carbs_g: 25,
                ingredients: vec![
                    MealIngredient { name: "Cottage cheese".into(), amount: "200g".into(), calories: 220 },
                    MealIngredient { name: "Mixed berries".into(), amount: "100g".into(), calories: 50 },
                    MealIngredient { name: "Walnuts".into(), amount: "20g".into(), calories: 130 },
                ],
            },
            MealVariant {
                level: "balanced".into(), emoji: "🟡".into(),
                title: "Balanced".into(),
                short_description: "turkey + buckwheat + broccoli".into(),
                calories: 520, protein_g: 45, fat_g: 12, carbs_g: 48,
                ingredients: vec![
                    MealIngredient { name: "Turkey breast".into(), amount: "180g".into(), calories: 250 },
                    MealIngredient { name: "Buckwheat".into(), amount: "80g".into(), calories: 200 },
                    MealIngredient { name: "Broccoli".into(), amount: "120g".into(), calories: 40 },
                ],
            },
            MealVariant {
                level: "rich".into(), emoji: "🔴".into(),
                title: "Hearty".into(),
                short_description: "beef steak + sweet potato + spinach".into(),
                calories: 720, protein_g: 50, fat_g: 28, carbs_g: 55,
                ingredients: vec![
                    MealIngredient { name: "Beef steak".into(), amount: "200g".into(), calories: 400 },
                    MealIngredient { name: "Sweet potato".into(), amount: "150g".into(), calories: 180 },
                    MealIngredient { name: "Spinach".into(), amount: "100g".into(), calories: 25 },
                ],
            },
        ],
    }
}

fn quick_breakfast_variants(lang: &str) -> Vec<MealVariant> {
    match lang {
        "ru" => vec![
            MealVariant {
                level: "light".into(), emoji: "🟢".into(),
                title: "Экспресс".into(),
                short_description: "тост + арахисовая паста + банан".into(),
                calories: 350, protein_g: 12, fat_g: 14, carbs_g: 44,
                ingredients: vec![
                    MealIngredient { name: "Цельнозерновой тост".into(), amount: "2 шт".into(), calories: 160 },
                    MealIngredient { name: "Арахисовая паста".into(), amount: "20г".into(), calories: 120 },
                    MealIngredient { name: "Банан".into(), amount: "1 шт".into(), calories: 70 },
                ],
            },
            MealVariant {
                level: "balanced".into(), emoji: "🟡".into(),
                title: "Классика".into(),
                short_description: "яичница + хлеб + помидор".into(),
                calories: 420, protein_g: 22, fat_g: 20, carbs_g: 35,
                ingredients: vec![
                    MealIngredient { name: "Яйца".into(), amount: "2 шт".into(), calories: 180 },
                    MealIngredient { name: "Хлеб ржаной".into(), amount: "2 ломтика".into(), calories: 160 },
                    MealIngredient { name: "Помидор".into(), amount: "1 шт".into(), calories: 25 },
                ],
            },
            MealVariant {
                level: "rich".into(), emoji: "🔴".into(),
                title: "Сытный".into(),
                short_description: "сырники + сметана + мёд".into(),
                calories: 550, protein_g: 25, fat_g: 20, carbs_g: 60,
                ingredients: vec![
                    MealIngredient { name: "Сырники".into(), amount: "3 шт".into(), calories: 350 },
                    MealIngredient { name: "Сметана 15%".into(), amount: "30г".into(), calories: 50 },
                    MealIngredient { name: "Мёд".into(), amount: "1 ч.л.".into(), calories: 30 },
                ],
            },
        ],
        "pl" => vec![
            MealVariant {
                level: "light".into(), emoji: "🟢".into(),
                title: "Ekspres".into(),
                short_description: "tost + masło orzechowe + banan".into(),
                calories: 350, protein_g: 12, fat_g: 14, carbs_g: 44,
                ingredients: vec![
                    MealIngredient { name: "Tost pełnoziarnisty".into(), amount: "2 szt".into(), calories: 160 },
                    MealIngredient { name: "Masło orzechowe".into(), amount: "20g".into(), calories: 120 },
                    MealIngredient { name: "Banan".into(), amount: "1 szt".into(), calories: 70 },
                ],
            },
            MealVariant {
                level: "balanced".into(), emoji: "🟡".into(),
                title: "Klasyk".into(),
                short_description: "jajecznica + chleb + pomidor".into(),
                calories: 420, protein_g: 22, fat_g: 20, carbs_g: 35,
                ingredients: vec![
                    MealIngredient { name: "Jajka".into(), amount: "2 szt".into(), calories: 180 },
                    MealIngredient { name: "Chleb żytni".into(), amount: "2 kromki".into(), calories: 160 },
                    MealIngredient { name: "Pomidor".into(), amount: "1 szt".into(), calories: 25 },
                ],
            },
            MealVariant {
                level: "rich".into(), emoji: "🔴".into(),
                title: "Sycący".into(),
                short_description: "serniczki + śmietana + miód".into(),
                calories: 550, protein_g: 25, fat_g: 20, carbs_g: 60,
                ingredients: vec![
                    MealIngredient { name: "Serniczki".into(), amount: "3 szt".into(), calories: 350 },
                    MealIngredient { name: "Śmietana 15%".into(), amount: "30g".into(), calories: 50 },
                    MealIngredient { name: "Miód".into(), amount: "1 łyżeczka".into(), calories: 30 },
                ],
            },
        ],
        _ => vec![
            MealVariant {
                level: "light".into(), emoji: "🟢".into(),
                title: "Express".into(),
                short_description: "toast + peanut butter + banana".into(),
                calories: 350, protein_g: 12, fat_g: 14, carbs_g: 44,
                ingredients: vec![
                    MealIngredient { name: "Whole grain toast".into(), amount: "2 pcs".into(), calories: 160 },
                    MealIngredient { name: "Peanut butter".into(), amount: "20g".into(), calories: 120 },
                    MealIngredient { name: "Banana".into(), amount: "1 pc".into(), calories: 70 },
                ],
            },
            MealVariant {
                level: "balanced".into(), emoji: "🟡".into(),
                title: "Classic".into(),
                short_description: "scrambled eggs + bread + tomato".into(),
                calories: 420, protein_g: 22, fat_g: 20, carbs_g: 35,
                ingredients: vec![
                    MealIngredient { name: "Eggs".into(), amount: "2 pcs".into(), calories: 180 },
                    MealIngredient { name: "Rye bread".into(), amount: "2 slices".into(), calories: 160 },
                    MealIngredient { name: "Tomato".into(), amount: "1 pc".into(), calories: 25 },
                ],
            },
            MealVariant {
                level: "rich".into(), emoji: "🔴".into(),
                title: "Hearty".into(),
                short_description: "pancakes + sour cream + honey".into(),
                calories: 550, protein_g: 25, fat_g: 20, carbs_g: 60,
                ingredients: vec![
                    MealIngredient { name: "Cottage cheese pancakes".into(), amount: "3 pcs".into(), calories: 350 },
                    MealIngredient { name: "Sour cream".into(), amount: "30g".into(), calories: 50 },
                    MealIngredient { name: "Honey".into(), amount: "1 tsp".into(), calories: 30 },
                ],
            },
        ],
    }
}

fn generic_variants(lang: &str) -> Vec<MealVariant> {
    // Same as healthy_day — balanced day menu
    match lang {
        "ru" => vec![
            MealVariant {
                level: "light".into(), emoji: "🟢".into(),
                title: "Лёгкий".into(),
                short_description: "салат + тунец + оливковое масло".into(),
                calories: 380, protein_g: 28, fat_g: 18, carbs_g: 22,
                ingredients: vec![
                    MealIngredient { name: "Тунец консервированный".into(), amount: "120г".into(), calories: 180 },
                    MealIngredient { name: "Микс салат".into(), amount: "100г".into(), calories: 20 },
                    MealIngredient { name: "Оливковое масло".into(), amount: "1 ст.л.".into(), calories: 120 },
                    MealIngredient { name: "Помидоры черри".into(), amount: "100г".into(), calories: 30 },
                ],
            },
            MealVariant {
                level: "balanced".into(), emoji: "🟡".into(),
                title: "Баланс".into(),
                short_description: "паста + куриный фарш + томатный соус".into(),
                calories: 520, protein_g: 30, fat_g: 14, carbs_g: 65,
                ingredients: vec![
                    MealIngredient { name: "Паста".into(), amount: "100г".into(), calories: 250 },
                    MealIngredient { name: "Куриный фарш".into(), amount: "120г".into(), calories: 180 },
                    MealIngredient { name: "Томатный соус".into(), amount: "80г".into(), calories: 40 },
                ],
            },
            MealVariant {
                level: "rich".into(), emoji: "🔴".into(),
                title: "Праздничный".into(),
                short_description: "стейк + картофель + грибной соус".into(),
                calories: 750, protein_g: 42, fat_g: 32, carbs_g: 60,
                ingredients: vec![
                    MealIngredient { name: "Стейк рибай".into(), amount: "200г".into(), calories: 420 },
                    MealIngredient { name: "Картофель".into(), amount: "200г".into(), calories: 200 },
                    MealIngredient { name: "Грибной соус".into(), amount: "50г".into(), calories: 80 },
                ],
            },
        ],
        "pl" => vec![
            MealVariant {
                level: "light".into(), emoji: "🟢".into(),
                title: "Lekki".into(),
                short_description: "sałatka + tuńczyk + oliwa".into(),
                calories: 380, protein_g: 28, fat_g: 18, carbs_g: 22,
                ingredients: vec![
                    MealIngredient { name: "Tuńczyk w puszce".into(), amount: "120g".into(), calories: 180 },
                    MealIngredient { name: "Mix sałat".into(), amount: "100g".into(), calories: 20 },
                    MealIngredient { name: "Oliwa z oliwek".into(), amount: "1 łyżka".into(), calories: 120 },
                    MealIngredient { name: "Pomidorki koktajlowe".into(), amount: "100g".into(), calories: 30 },
                ],
            },
            MealVariant {
                level: "balanced".into(), emoji: "🟡".into(),
                title: "Balans".into(),
                short_description: "makaron + mielony kurczak + sos pomidorowy".into(),
                calories: 520, protein_g: 30, fat_g: 14, carbs_g: 65,
                ingredients: vec![
                    MealIngredient { name: "Makaron".into(), amount: "100g".into(), calories: 250 },
                    MealIngredient { name: "Mielony kurczak".into(), amount: "120g".into(), calories: 180 },
                    MealIngredient { name: "Sos pomidorowy".into(), amount: "80g".into(), calories: 40 },
                ],
            },
            MealVariant {
                level: "rich".into(), emoji: "🔴".into(),
                title: "Na specjalną okazję".into(),
                short_description: "stek + ziemniaki + sos grzybowy".into(),
                calories: 750, protein_g: 42, fat_g: 32, carbs_g: 60,
                ingredients: vec![
                    MealIngredient { name: "Stek wołowy".into(), amount: "200g".into(), calories: 420 },
                    MealIngredient { name: "Ziemniaki".into(), amount: "200g".into(), calories: 200 },
                    MealIngredient { name: "Sos grzybowy".into(), amount: "50g".into(), calories: 80 },
                ],
            },
        ],
        _ => vec![
            MealVariant {
                level: "light".into(), emoji: "🟢".into(),
                title: "Light".into(),
                short_description: "salad + tuna + olive oil".into(),
                calories: 380, protein_g: 28, fat_g: 18, carbs_g: 22,
                ingredients: vec![
                    MealIngredient { name: "Canned tuna".into(), amount: "120g".into(), calories: 180 },
                    MealIngredient { name: "Mixed greens".into(), amount: "100g".into(), calories: 20 },
                    MealIngredient { name: "Olive oil".into(), amount: "1 tbsp".into(), calories: 120 },
                    MealIngredient { name: "Cherry tomatoes".into(), amount: "100g".into(), calories: 30 },
                ],
            },
            MealVariant {
                level: "balanced".into(), emoji: "🟡".into(),
                title: "Balanced".into(),
                short_description: "pasta + ground chicken + tomato sauce".into(),
                calories: 520, protein_g: 30, fat_g: 14, carbs_g: 65,
                ingredients: vec![
                    MealIngredient { name: "Pasta".into(), amount: "100g".into(), calories: 250 },
                    MealIngredient { name: "Ground chicken".into(), amount: "120g".into(), calories: 180 },
                    MealIngredient { name: "Tomato sauce".into(), amount: "80g".into(), calories: 40 },
                ],
            },
            MealVariant {
                level: "rich".into(), emoji: "🔴".into(),
                title: "Celebration".into(),
                short_description: "steak + potatoes + mushroom sauce".into(),
                calories: 750, protein_g: 42, fat_g: 32, carbs_g: 60,
                ingredients: vec![
                    MealIngredient { name: "Ribeye steak".into(), amount: "200g".into(), calories: 420 },
                    MealIngredient { name: "Potatoes".into(), amount: "200g".into(), calories: 200 },
                    MealIngredient { name: "Mushroom sauce".into(), amount: "50g".into(), calories: 80 },
                ],
            },
        ],
    }
}

fn get_variants(goal: Goal, lang: &str) -> Vec<MealVariant> {
    match goal {
        Goal::WeightLoss => weight_loss_variants(lang),
        Goal::HighProtein => high_protein_variants(lang),
        Goal::QuickBreakfast => quick_breakfast_variants(lang),
        Goal::FromIngredients | Goal::HealthyDay | Goal::Generic => generic_variants(lang),
    }
}

// ── Gemini prompts (minimal — just personality text) ──────────────────────────

fn build_gemini_prompt(goal: Goal, lang: &str, variants: &[MealVariant]) -> String {
    let lang_instruction = match lang {
        "ru" => "Отвечай на русском языке. Пиши как живой шеф — коротко, тепло, по делу. Не используй слова 'чатбот', 'ИИ', 'алгоритм'.",
        "pl" => "Odpowiadaj po polsku. Pisz jak prawdziwy szef kuchni — krótko, ciepło, konkretnie.",
        "uk" => "Відповідай українською. Пиши як живий шеф — коротко, тепло, по суті.",
        _ => "Answer in English. Write like a real chef — short, warm, to the point.",
    };

    let (goal_desc, personalization_hint) = match goal {
        Goal::WeightLoss => (
            "calorie deficit / weight loss for 1 day",
            "Tell them: take the light variant for deficit, balanced if they need energy for the day.",
        ),
        Goal::HighProtein => (
            "high protein day — muscle building or active recovery",
            "Tell them: rich variant post-workout, balanced on rest days.",
        ),
        Goal::QuickBreakfast => (
            "quick breakfast in 5-15 minutes",
            "Tell them: light if not hungry in the morning, balanced for an active day ahead.",
        ),
        Goal::FromIngredients => (
            "meal from specific ingredients they have at home",
            "Tell them: balanced is the safest bet, light if they want to avoid heaviness.",
        ),
        Goal::HealthyDay => (
            "healthy balanced day — stable energy and nutrients",
            "Tell them: balanced is the default hero here, light if they sit most of the day.",
        ),
        Goal::Generic => (
            "balanced meal plan for the day",
            "Tell them: balanced is the recommended choice, light for deficit, rich for an active day.",
        ),
    };

    let variants_summary: String = variants.iter().map(|v| {
        format!("- {} {} ({}): {} — {} kcal, {}g protein, {}g fat, {}g carbs",
            v.emoji, v.title, v.level, v.short_description, v.calories, v.protein_g, v.fat_g, v.carbs_g)
    }).collect::<Vec<_>>().join("\n");

    format!(
        r#"You are a sous-chef who thinks, chooses and explains. You already looked at the task and built the plan.
{lang_instruction}

User goal: {goal_desc}

You prepared these 3 variants:
{variants_summary}

Personalization hint: {personalization_hint}

Generate EXACTLY this JSON (no markdown, no extra text):
{{
  "chef_intro": "2-3 sentences max. Start with 'Посмотрел задачу.' or equivalent. Briefly say what you made and WHY — light for deficit, balanced for energy, rich for taste/satiety. Sound like a person who thought it through, not a bot.",
  "explanation": "2-3 sentences. Explain WHY this works in human terms — e.g. stable energy, no sugar spikes, protein keeps you full. Avoid 'scientifically proven'. Be conversational.",
  "motivation": "1-2 sentences. Realistic, forward-looking. E.g. 'Придерживайся такого дня — и к вечеру почувствуешь лёгкость.' Never promise specific kg loss. No fake enthusiasm."
}}

Rules:
- chef_intro MUST mention the 3 options with logic (why each exists)
- explanation must feel human, not clinical
- motivation must feel like a real person speaking, not a slogan
- max 3 sentences per field"#,
        lang_instruction = lang_instruction,
        goal_desc = goal_desc,
        variants_summary = variants_summary,
        personalization_hint = personalization_hint,
    )
}

// ── Gemini response parsing ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct GeminiChefResponse {
    chef_intro: String,
    explanation: String,
    motivation: String,
}

fn parse_gemini_response(raw: &str) -> Option<GeminiChefResponse> {
    // Try direct JSON parse
    if let Ok(r) = serde_json::from_str::<GeminiChefResponse>(raw) {
        return Some(r);
    }
    // Try extracting JSON from markdown code block
    if let Some(start) = raw.find('{') {
        if let Some(end) = raw.rfind('}') {
            let json_str = &raw[start..=end];
            if let Ok(r) = serde_json::from_str::<GeminiChefResponse>(json_str) {
                return Some(r);
            }
        }
    }
    None
}

// ── Fallback texts (if Gemini fails — still show results) ─────────────────────

fn fallback_intro(goal: Goal, lang: &str) -> String {
    match (goal, lang) {
        (Goal::WeightLoss, "ru") => "Посмотрел задачу. Сделал тебе день с дефицитом: лёгкий — если хочешь минус, сбалансированный — если нужна энергия, и сытный — если важен вкус. Выбирай по ощущениям.".into(),
        (Goal::WeightLoss, "pl") => "Przejrzałem zadanie. Ułożyłem dzień z deficytem: lekki — jeśli chcesz chudnąć, zbilansowany — jeśli potrzebujesz energii, sycący — jeśli liczy się smak.".into(),
        (Goal::WeightLoss, "uk") => "Подивився на задачу. Зробив тобі день з дефіцитом: легкий — якщо хочеш мінус, збалансований — якщо потрібна енергія, ситний — якщо важливий смак.".into(),
        (Goal::WeightLoss, _) => "Looked at the task. Made you a deficit day: light for the minus, balanced if you need energy, rich if taste matters. Pick by feel.".into(),

        (Goal::HighProtein, "ru") => "Понял — нужен белок. Лёгкий вариант — в дни отдыха, сбалансированный — рабочий день, сытный — после тренировки. Всё с высоким протеином.".into(),
        (Goal::HighProtein, "pl") => "Rozumiem — potrzebujesz białka. Lekki — na dni odpoczynku, zbilansowany — dzień roboczy, sycący — po treningu. Wszystko z wysokim białkiem.".into(),
        (Goal::HighProtein, "uk") => "Зрозумів — потрібен білок. Легкий — у дні відпочинку, збалансований — робочий день, ситний — після тренування.".into(),
        (Goal::HighProtein, _) => "Got it — you need protein. Light on rest days, balanced for a work day, rich after training. All high protein.".into(),

        (Goal::QuickBreakfast, "ru") => "Быстрый завтрак — не проблема. Лёгкий если не голоден с утра, сбалансированный если впереди активный день, сытный если пропустил ужин.".into(),
        (Goal::QuickBreakfast, "pl") => "Szybkie śniadanie — żaden problem. Lekkie jeśli rano nie jesteś głodny, zbilansowane przed aktywnym dniem, sycące jeśli pominąłeś kolację.".into(),
        (Goal::QuickBreakfast, "uk") => "Швидкий сніданок — не проблема. Легкий якщо зранку не голодний, збалансований перед активним днем, ситний якщо пропустив вечерю.".into(),
        (Goal::QuickBreakfast, _) => "Quick breakfast — no problem. Light if you're not hungry in the morning, balanced before an active day, rich if you skipped dinner.".into(),

        (_, "ru") => "Посмотрел задачу. Сделал три варианта: лёгкий — для дефицита, сбалансированный — для стабильной энергии, сытный — для вкуса. Выбирай что ближе.".into(),
        (_, "pl") => "Przejrzałem zadanie. Przygotowałem trzy opcje: lekką — dla deficytu, zbilansowaną — dla stabilnej energii, sycącą — dla smaku.".into(),
        (_, "uk") => "Подивився на задачу. Зробив три варіанти: легкий — для дефіциту, збалансований — для стабільної енергії, ситний — для смаку.".into(),
        (_, _) => "Looked at the task. Made three options: light for deficit, balanced for stable energy, rich for taste. Pick what fits.".into(),
    }
}

fn fallback_explanation(goal: Goal, lang: &str) -> String {
    match (goal, lang) {
        (Goal::WeightLoss, "ru") => "Здесь баланс белка, жиров и углеводов — энергия будет стабильной, без скачков и переедания. Белок сохраняет мышцы даже при дефиците. Без лишнего сахара — никаких провалов к обеду.".into(),
        (Goal::WeightLoss, "pl") => "Tutaj balans białka, tłuszczów i węglowodanów — energia stabilna, bez napadów głodu. Białko chroni mięśnie przy deficycie. Bez nadmiaru cukru — żadnych spadków energii.".into(),
        (Goal::WeightLoss, "uk") => "Тут баланс білка, жирів і вуглеводів — енергія стабільна, без стрибків. Білок зберігає м'язи навіть при дефіциті. Без зайвого цукру — ніяких провалів до обіду.".into(),
        (Goal::WeightLoss, _) => "Protein, fat and carbs are balanced here — stable energy, no hunger spikes. Protein preserves muscle even at deficit. No sugar overload means no afternoon crash.".into(),

        (_, "ru") => "Здесь сочетание белков, жиров и углеводов подобрано так, чтобы энергия была стабильной без скачков. Ни тяжести, ни голода через час.".into(),
        (_, "pl") => "Połączenie białek, tłuszczów i węglowodanów dobrane tak, by energia była stabilna. Ani ciężkości, ani głodu po godzinie.".into(),
        (_, "uk") => "Поєднання білків, жирів і вуглеводів підібране так, щоб енергія була стабільною. Ні важкості, ні голоду через годину.".into(),
        (_, _) => "Proteins, fats and carbs are balanced here for stable energy — no heaviness, no hunger an hour later.".into(),
    }
}

fn fallback_motivation(lang: &str) -> String {
    match lang {
        "ru" => "Придерживайся такого дня — и к вечеру почувствуешь лёгкость. Не нужно ждать недели, чтобы понять, что это работает.".into(),
        "pl" => "Trzymaj się takiego dnia — wieczorem poczujesz lekkość. Nie trzeba czekać tygodnia, żeby zobaczyć, że to działa.".into(),
        "uk" => "Дотримуйся такого дня — і ввечері відчуєш легкість. Не потрібно чекати тижня, щоб зрозуміти, що це працює.".into(),
        _ => "Stick to this day — by evening you'll feel lighter. You don't need to wait a week to know it's working.".into(),
    }
}

// ── Service ───────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct SousChefPlannerService {
    llm: Arc<LlmAdapter>,
    ai_cache: Arc<AiCacheRepository>,
}

impl SousChefPlannerService {
    pub fn new(llm: Arc<LlmAdapter>, ai_cache: AiCacheRepository) -> Self {
        Self {
            llm,
            ai_cache: Arc::new(ai_cache),
        }
    }

    /// Generate a meal plan. Checks DB cache first, then Rust + optional Gemini.
    pub async fn generate_plan(&self, req: PlanRequest) -> Result<MealPlan, AppError> {
        let lang = req.lang.as_deref().unwrap_or("en");
        let normalized = normalize_query(&req.query);
        let goal = Goal::detect(&normalized);
        let cache_key = build_cache_key(goal, lang);

        // 1. Check DB cache (ai_cache table — 90-day TTL)
        if let Ok(Some(cached)) = self.ai_cache.get(&cache_key).await {
            if let Ok(mut plan) = serde_json::from_value::<MealPlan>(cached) {
                plan.cached = true;
                tracing::info!("⚡ Sous-chef plan cache HIT: {}", cache_key);
                return Ok(plan);
            }
        }

        // 2. Build variants (Rust — 0 AI cost)
        let variants = get_variants(goal, lang);

        // 3. Get personality text from Gemini Flash (cheap, async)
        let prompt = build_gemini_prompt(goal, lang, &variants);
        let (chef_intro, explanation, motivation) = match self.llm
            .groq_raw_request_with_model(&prompt, 400, "gemini-3-flash-preview")
            .await
        {
            Ok(raw) => {
                match parse_gemini_response(&raw) {
                    Some(r) => (r.chef_intro, r.explanation, r.motivation),
                    None => {
                        tracing::warn!("⚠️ Gemini response parse failed, using fallback");
                        (
                            fallback_intro(goal, lang),
                            fallback_explanation(goal, lang),
                            fallback_motivation(lang),
                        )
                    }
                }
            }
            Err(e) => {
                tracing::warn!("⚠️ Gemini call failed: {}, using fallback", e);
                (
                    fallback_intro(goal, lang),
                    fallback_explanation(goal, lang),
                    fallback_motivation(lang),
                )
            }
        };

        // 4. Build response
        let plan = MealPlan {
            cache_key: cache_key.clone(),
            cached: false,
            chef_intro,
            variants,
            explanation,
            motivation,
            goal: goal.slug().into(),
            lang: lang.into(),
        };

        // 5. Store in DB cache (90 days — same query = instant next time)
        if let Ok(val) = serde_json::to_value(&plan) {
            let _ = self.ai_cache
                .set(&cache_key, val, "gemini", "gemini-3-flash-preview", 90)
                .await;
            tracing::info!("💾 Sous-chef plan cached: {}", cache_key);
        }

        Ok(plan)
    }
}
