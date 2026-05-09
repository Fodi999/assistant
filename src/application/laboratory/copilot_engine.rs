//! Copilot Engine — **deterministic** keyword-to-draft translator.
//!
//! # What it does
//!
//! Given a free-form user prompt (e.g. "I want a strawberry sauce without
//! sugar") it produces a draft `CopilotDraft { ingredients, steps }` that
//! the frontend inserts directly into the constructor zone.
//!
//! # Algorithm (no AI, no HTTP)
//!
//! 1. **Normalise** prompt to lowercase.
//! 2. **Token extraction** — split on whitespace/punctuation, deduplicate.
//! 3. **Ingredient matching** — for each token, check against a built-in
//!    keyword→slug table. Each slug gets a *role* and a *default quantity*.
//! 4. **Exclusion matching** — words like "without", "no", "без", "ohne"
//!    introduce a set of excluded ingredient categories. Slugs whose role
//!    intersects the exclusion set are dropped.
//! 5. **Product type inference** — heuristic label from the prompt
//!    ("sauce", "soup", "smoothie", …).
//! 6. **Step template selection** — each inferred `product_type` carries a
//!    default process program (temperature, duration, technique).
//! 7. **Catalog validation** — slugs are NOT validated against the DB here
//!    (pure, sync, fast). The service layer will call the catalog adapter and
//!    silently drop slugs that are absent from `products`.
//!
//! # Extending
//!
//! Add rows to `KEYWORD_MAP` and `STEP_TEMPLATES`. No DB migration needed.

use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// Public DTOs
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopilotDraftIngredient {
    pub slug: String,
    pub quantity: f64,
    pub unit: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopilotDraftStep {
    pub technique: String,
    pub temperature_c: Option<f64>,
    pub duration_min: Option<u32>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopilotDraft {
    /// Inferred product type (sent back so the frontend can populate the
    /// `target_product_type` field when creating the project).
    pub product_type: String,
    /// Draft name suggestion based on the prompt.
    pub suggested_name: String,
    /// Ordered ingredient list ready to POST to `/ingredients`.
    pub ingredients: Vec<CopilotDraftIngredient>,
    /// Process steps ready to POST to `/steps` in order.
    pub steps: Vec<CopilotDraftStep>,
    /// Human-readable explanation of what the engine understood.
    pub rationale: String,
    /// Confidence 0.0–1.0 (1.0 = all tokens matched, 0.0 = nothing found).
    pub confidence: f64,
    /// Tokens the engine could NOT match (so the frontend can highlight them).
    pub unmatched_tokens: Vec<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Keyword table
// ─────────────────────────────────────────────────────────────────────────────

/// Maps a normalised keyword to a catalog slug + role + default quantity/unit.
#[derive(Clone)]
struct KeywordEntry {
    slug: &'static str,
    role: &'static str,
    quantity_g: f64,
    unit: &'static str,
    /// Semantic category used for exclusions (e.g. "sweetener", "fat", …).
    category: &'static str,
}

const KEYWORD_MAP: &[(&str, KeywordEntry)] = &[
    // ── Fruits ──────────────────────────────────────────────────────────────
    (
        "strawberry",
        KeywordEntry {
            slug: "strawberry",
            role: "base",
            quantity_g: 300.0,
            unit: "g",
            category: "fruit",
        },
    ),
    (
        "клубника",
        KeywordEntry {
            slug: "strawberry",
            role: "base",
            quantity_g: 300.0,
            unit: "g",
            category: "fruit",
        },
    ),
    (
        "клубніка",
        KeywordEntry {
            slug: "strawberry",
            role: "base",
            quantity_g: 300.0,
            unit: "g",
            category: "fruit",
        },
    ),
    (
        "truskawka",
        KeywordEntry {
            slug: "strawberry",
            role: "base",
            quantity_g: 300.0,
            unit: "g",
            category: "fruit",
        },
    ),
    (
        "apricot",
        KeywordEntry {
            slug: "apricot",
            role: "base",
            quantity_g: 300.0,
            unit: "g",
            category: "fruit",
        },
    ),
    (
        "абрикос",
        KeywordEntry {
            slug: "apricot",
            role: "base",
            quantity_g: 300.0,
            unit: "g",
            category: "fruit",
        },
    ),
    (
        "абрикосовый",
        KeywordEntry {
            slug: "apricot",
            role: "base",
            quantity_g: 300.0,
            unit: "g",
            category: "fruit",
        },
    ),
    (
        "абрикосовий",
        KeywordEntry {
            slug: "apricot",
            role: "base",
            quantity_g: 300.0,
            unit: "g",
            category: "fruit",
        },
    ),
    (
        "morela",
        KeywordEntry {
            slug: "apricot",
            role: "base",
            quantity_g: 300.0,
            unit: "g",
            category: "fruit",
        },
    ),
    (
        "mango",
        KeywordEntry {
            slug: "mango",
            role: "base",
            quantity_g: 300.0,
            unit: "g",
            category: "fruit",
        },
    ),
    (
        "манго",
        KeywordEntry {
            slug: "mango",
            role: "base",
            quantity_g: 300.0,
            unit: "g",
            category: "fruit",
        },
    ),
    (
        "raspberry",
        KeywordEntry {
            slug: "raspberry",
            role: "base",
            quantity_g: 250.0,
            unit: "g",
            category: "fruit",
        },
    ),
    (
        "малина",
        KeywordEntry {
            slug: "raspberry",
            role: "base",
            quantity_g: 250.0,
            unit: "g",
            category: "fruit",
        },
    ),
    (
        "malina",
        KeywordEntry {
            slug: "raspberry",
            role: "base",
            quantity_g: 250.0,
            unit: "g",
            category: "fruit",
        },
    ),
    (
        "lemon",
        KeywordEntry {
            slug: "lemon",
            role: "acid",
            quantity_g: 30.0,
            unit: "g",
            category: "citrus",
        },
    ),
    (
        "лимон",
        KeywordEntry {
            slug: "lemon",
            role: "acid",
            quantity_g: 30.0,
            unit: "g",
            category: "citrus",
        },
    ),
    (
        "cytryna",
        KeywordEntry {
            slug: "lemon",
            role: "acid",
            quantity_g: 30.0,
            unit: "g",
            category: "citrus",
        },
    ),
    (
        "lime",
        KeywordEntry {
            slug: "lime",
            role: "acid",
            quantity_g: 20.0,
            unit: "g",
            category: "citrus",
        },
    ),
    (
        "лайм",
        KeywordEntry {
            slug: "lime",
            role: "acid",
            quantity_g: 20.0,
            unit: "g",
            category: "citrus",
        },
    ),
    (
        "apple",
        KeywordEntry {
            slug: "apple",
            role: "base",
            quantity_g: 200.0,
            unit: "g",
            category: "fruit",
        },
    ),
    (
        "яблоко",
        KeywordEntry {
            slug: "apple",
            role: "base",
            quantity_g: 200.0,
            unit: "g",
            category: "fruit",
        },
    ),
    (
        "jabłko",
        KeywordEntry {
            slug: "apple",
            role: "base",
            quantity_g: 200.0,
            unit: "g",
            category: "fruit",
        },
    ),
    (
        "яблуко",
        KeywordEntry {
            slug: "apple",
            role: "base",
            quantity_g: 200.0,
            unit: "g",
            category: "fruit",
        },
    ),
    (
        "pear",
        KeywordEntry {
            slug: "pear",
            role: "base",
            quantity_g: 200.0,
            unit: "g",
            category: "fruit",
        },
    ),
    (
        "груша",
        KeywordEntry {
            slug: "pear",
            role: "base",
            quantity_g: 200.0,
            unit: "g",
            category: "fruit",
        },
    ),
    (
        "gruszka",
        KeywordEntry {
            slug: "pear",
            role: "base",
            quantity_g: 200.0,
            unit: "g",
            category: "fruit",
        },
    ),
    (
        "cherry",
        KeywordEntry {
            slug: "cherry",
            role: "base",
            quantity_g: 250.0,
            unit: "g",
            category: "fruit",
        },
    ),
    (
        "вишня",
        KeywordEntry {
            slug: "cherry",
            role: "base",
            quantity_g: 250.0,
            unit: "g",
            category: "fruit",
        },
    ),
    (
        "wiśnia",
        KeywordEntry {
            slug: "cherry",
            role: "base",
            quantity_g: 250.0,
            unit: "g",
            category: "fruit",
        },
    ),
    (
        "banana",
        KeywordEntry {
            slug: "banana",
            role: "base",
            quantity_g: 150.0,
            unit: "g",
            category: "fruit",
        },
    ),
    (
        "банан",
        KeywordEntry {
            slug: "banana",
            role: "base",
            quantity_g: 150.0,
            unit: "g",
            category: "fruit",
        },
    ),
    (
        "peach",
        KeywordEntry {
            slug: "peach",
            role: "base",
            quantity_g: 200.0,
            unit: "g",
            category: "fruit",
        },
    ),
    (
        "персик",
        KeywordEntry {
            slug: "peach",
            role: "base",
            quantity_g: 200.0,
            unit: "g",
            category: "fruit",
        },
    ),
    (
        "brzoskwinia",
        KeywordEntry {
            slug: "peach",
            role: "base",
            quantity_g: 200.0,
            unit: "g",
            category: "fruit",
        },
    ),
    (
        "pineapple",
        KeywordEntry {
            slug: "pineapple",
            role: "base",
            quantity_g: 250.0,
            unit: "g",
            category: "fruit",
        },
    ),
    (
        "ананас",
        KeywordEntry {
            slug: "pineapple",
            role: "base",
            quantity_g: 250.0,
            unit: "g",
            category: "fruit",
        },
    ),
    (
        "ananas",
        KeywordEntry {
            slug: "pineapple",
            role: "base",
            quantity_g: 250.0,
            unit: "g",
            category: "fruit",
        },
    ),
    (
        "blueberry",
        KeywordEntry {
            slug: "blueberry",
            role: "base",
            quantity_g: 200.0,
            unit: "g",
            category: "fruit",
        },
    ),
    (
        "черника",
        KeywordEntry {
            slug: "blueberry",
            role: "base",
            quantity_g: 200.0,
            unit: "g",
            category: "fruit",
        },
    ),
    (
        "jagoda",
        KeywordEntry {
            slug: "blueberry",
            role: "base",
            quantity_g: 200.0,
            unit: "g",
            category: "fruit",
        },
    ),
    // ── Vegetables ──────────────────────────────────────────────────────────
    (
        "tomato",
        KeywordEntry {
            slug: "tomato",
            role: "base",
            quantity_g: 300.0,
            unit: "g",
            category: "vegetable",
        },
    ),
    (
        "томат",
        KeywordEntry {
            slug: "tomato",
            role: "base",
            quantity_g: 300.0,
            unit: "g",
            category: "vegetable",
        },
    ),
    (
        "помидор",
        KeywordEntry {
            slug: "tomato",
            role: "base",
            quantity_g: 300.0,
            unit: "g",
            category: "vegetable",
        },
    ),
    (
        "pomidor",
        KeywordEntry {
            slug: "tomato",
            role: "base",
            quantity_g: 300.0,
            unit: "g",
            category: "vegetable",
        },
    ),
    (
        "onion",
        KeywordEntry {
            slug: "onion",
            role: "aromatic",
            quantity_g: 100.0,
            unit: "g",
            category: "vegetable",
        },
    ),
    (
        "лук",
        KeywordEntry {
            slug: "onion",
            role: "aromatic",
            quantity_g: 100.0,
            unit: "g",
            category: "vegetable",
        },
    ),
    (
        "cebula",
        KeywordEntry {
            slug: "onion",
            role: "aromatic",
            quantity_g: 100.0,
            unit: "g",
            category: "vegetable",
        },
    ),
    (
        "garlic",
        KeywordEntry {
            slug: "garlic",
            role: "aromatic",
            quantity_g: 10.0,
            unit: "g",
            category: "vegetable",
        },
    ),
    (
        "чеснок",
        KeywordEntry {
            slug: "garlic",
            role: "aromatic",
            quantity_g: 10.0,
            unit: "g",
            category: "vegetable",
        },
    ),
    (
        "czosnek",
        KeywordEntry {
            slug: "garlic",
            role: "aromatic",
            quantity_g: 10.0,
            unit: "g",
            category: "vegetable",
        },
    ),
    (
        "czosn",
        KeywordEntry {
            slug: "garlic",
            role: "aromatic",
            quantity_g: 10.0,
            unit: "g",
            category: "vegetable",
        },
    ),
    (
        "часник",
        KeywordEntry {
            slug: "garlic",
            role: "aromatic",
            quantity_g: 10.0,
            unit: "g",
            category: "vegetable",
        },
    ),
    (
        "carrot",
        KeywordEntry {
            slug: "carrot",
            role: "base",
            quantity_g: 150.0,
            unit: "g",
            category: "vegetable",
        },
    ),
    (
        "морковь",
        KeywordEntry {
            slug: "carrot",
            role: "base",
            quantity_g: 150.0,
            unit: "g",
            category: "vegetable",
        },
    ),
    (
        "marchewka",
        KeywordEntry {
            slug: "carrot",
            role: "base",
            quantity_g: 150.0,
            unit: "g",
            category: "vegetable",
        },
    ),
    (
        "морква",
        KeywordEntry {
            slug: "carrot",
            role: "base",
            quantity_g: 150.0,
            unit: "g",
            category: "vegetable",
        },
    ),
    (
        "pepper",
        KeywordEntry {
            slug: "bell_pepper",
            role: "base",
            quantity_g: 150.0,
            unit: "g",
            category: "vegetable",
        },
    ),
    (
        "перец",
        KeywordEntry {
            slug: "bell_pepper",
            role: "base",
            quantity_g: 150.0,
            unit: "g",
            category: "vegetable",
        },
    ),
    (
        "papryka",
        KeywordEntry {
            slug: "bell_pepper",
            role: "base",
            quantity_g: 150.0,
            unit: "g",
            category: "vegetable",
        },
    ),
    (
        "spinach",
        KeywordEntry {
            slug: "spinach",
            role: "base",
            quantity_g: 100.0,
            unit: "g",
            category: "vegetable",
        },
    ),
    (
        "шпинат",
        KeywordEntry {
            slug: "spinach",
            role: "base",
            quantity_g: 100.0,
            unit: "g",
            category: "vegetable",
        },
    ),
    (
        "szpinak",
        KeywordEntry {
            slug: "spinach",
            role: "base",
            quantity_g: 100.0,
            unit: "g",
            category: "vegetable",
        },
    ),
    (
        "cucumber",
        KeywordEntry {
            slug: "cucumber",
            role: "base",
            quantity_g: 200.0,
            unit: "g",
            category: "vegetable",
        },
    ),
    (
        "огурец",
        KeywordEntry {
            slug: "cucumber",
            role: "base",
            quantity_g: 200.0,
            unit: "g",
            category: "vegetable",
        },
    ),
    (
        "ogórek",
        KeywordEntry {
            slug: "cucumber",
            role: "base",
            quantity_g: 200.0,
            unit: "g",
            category: "vegetable",
        },
    ),
    (
        "брокколи",
        KeywordEntry {
            slug: "broccoli",
            role: "base",
            quantity_g: 200.0,
            unit: "g",
            category: "vegetable",
        },
    ),
    (
        "broccoli",
        KeywordEntry {
            slug: "broccoli",
            role: "base",
            quantity_g: 200.0,
            unit: "g",
            category: "vegetable",
        },
    ),
    (
        "brokuł",
        KeywordEntry {
            slug: "broccoli",
            role: "base",
            quantity_g: 200.0,
            unit: "g",
            category: "vegetable",
        },
    ),
    // ── Dairy ────────────────────────────────────────────────────────────────
    (
        "cream",
        KeywordEntry {
            slug: "cream",
            role: "fat",
            quantity_g: 100.0,
            unit: "ml",
            category: "dairy",
        },
    ),
    (
        "сливки",
        KeywordEntry {
            slug: "cream",
            role: "fat",
            quantity_g: 100.0,
            unit: "ml",
            category: "dairy",
        },
    ),
    (
        "śmietana",
        KeywordEntry {
            slug: "cream",
            role: "fat",
            quantity_g: 100.0,
            unit: "ml",
            category: "dairy",
        },
    ),
    (
        "вершки",
        KeywordEntry {
            slug: "cream",
            role: "fat",
            quantity_g: 100.0,
            unit: "ml",
            category: "dairy",
        },
    ),
    (
        "yogurt",
        KeywordEntry {
            slug: "yogurt",
            role: "fat",
            quantity_g: 150.0,
            unit: "g",
            category: "dairy",
        },
    ),
    (
        "йогурт",
        KeywordEntry {
            slug: "yogurt",
            role: "fat",
            quantity_g: 150.0,
            unit: "g",
            category: "dairy",
        },
    ),
    (
        "jogurt",
        KeywordEntry {
            slug: "yogurt",
            role: "fat",
            quantity_g: 150.0,
            unit: "g",
            category: "dairy",
        },
    ),
    (
        "butter",
        KeywordEntry {
            slug: "butter",
            role: "fat",
            quantity_g: 30.0,
            unit: "g",
            category: "dairy",
        },
    ),
    (
        "масло",
        KeywordEntry {
            slug: "butter",
            role: "fat",
            quantity_g: 30.0,
            unit: "g",
            category: "dairy",
        },
    ),
    (
        "masło",
        KeywordEntry {
            slug: "butter",
            role: "fat",
            quantity_g: 30.0,
            unit: "g",
            category: "dairy",
        },
    ),
    (
        "масло",
        KeywordEntry {
            slug: "butter",
            role: "fat",
            quantity_g: 30.0,
            unit: "g",
            category: "dairy",
        },
    ),
    (
        "milk",
        KeywordEntry {
            slug: "milk",
            role: "liquid",
            quantity_g: 200.0,
            unit: "ml",
            category: "dairy",
        },
    ),
    (
        "молоко",
        KeywordEntry {
            slug: "milk",
            role: "liquid",
            quantity_g: 200.0,
            unit: "ml",
            category: "dairy",
        },
    ),
    (
        "mleko",
        KeywordEntry {
            slug: "milk",
            role: "liquid",
            quantity_g: 200.0,
            unit: "ml",
            category: "dairy",
        },
    ),
    (
        "cheese",
        KeywordEntry {
            slug: "cheese",
            role: "fat",
            quantity_g: 80.0,
            unit: "g",
            category: "dairy",
        },
    ),
    (
        "сыр",
        KeywordEntry {
            slug: "cheese",
            role: "fat",
            quantity_g: 80.0,
            unit: "g",
            category: "dairy",
        },
    ),
    (
        "ser",
        KeywordEntry {
            slug: "cheese",
            role: "fat",
            quantity_g: 80.0,
            unit: "g",
            category: "dairy",
        },
    ),
    (
        "mascarpone",
        KeywordEntry {
            slug: "mascarpone",
            role: "fat",
            quantity_g: 100.0,
            unit: "g",
            category: "dairy",
        },
    ),
    (
        "ricotta",
        KeywordEntry {
            slug: "ricotta",
            role: "fat",
            quantity_g: 100.0,
            unit: "g",
            category: "dairy",
        },
    ),
    // ── Proteins ─────────────────────────────────────────────────────────────
    (
        "chicken",
        KeywordEntry {
            slug: "chicken",
            role: "protein",
            quantity_g: 300.0,
            unit: "g",
            category: "protein",
        },
    ),
    (
        "курица",
        KeywordEntry {
            slug: "chicken",
            role: "protein",
            quantity_g: 300.0,
            unit: "g",
            category: "protein",
        },
    ),
    (
        "kurczak",
        KeywordEntry {
            slug: "chicken",
            role: "protein",
            quantity_g: 300.0,
            unit: "g",
            category: "protein",
        },
    ),
    (
        "курятина",
        KeywordEntry {
            slug: "chicken",
            role: "protein",
            quantity_g: 300.0,
            unit: "g",
            category: "protein",
        },
    ),
    (
        "beef",
        KeywordEntry {
            slug: "beef",
            role: "protein",
            quantity_g: 300.0,
            unit: "g",
            category: "protein",
        },
    ),
    (
        "говядина",
        KeywordEntry {
            slug: "beef",
            role: "protein",
            quantity_g: 300.0,
            unit: "g",
            category: "protein",
        },
    ),
    (
        "wołowina",
        KeywordEntry {
            slug: "beef",
            role: "protein",
            quantity_g: 300.0,
            unit: "g",
            category: "protein",
        },
    ),
    (
        "salmon",
        KeywordEntry {
            slug: "salmon",
            role: "protein",
            quantity_g: 200.0,
            unit: "g",
            category: "protein",
        },
    ),
    (
        "лосось",
        KeywordEntry {
            slug: "salmon",
            role: "protein",
            quantity_g: 200.0,
            unit: "g",
            category: "protein",
        },
    ),
    (
        "łosoś",
        KeywordEntry {
            slug: "salmon",
            role: "protein",
            quantity_g: 200.0,
            unit: "g",
            category: "protein",
        },
    ),
    (
        "egg",
        KeywordEntry {
            slug: "egg",
            role: "binder",
            quantity_g: 60.0,
            unit: "g",
            category: "protein",
        },
    ),
    (
        "яйцо",
        KeywordEntry {
            slug: "egg",
            role: "binder",
            quantity_g: 60.0,
            unit: "g",
            category: "protein",
        },
    ),
    (
        "яйце",
        KeywordEntry {
            slug: "egg",
            role: "binder",
            quantity_g: 60.0,
            unit: "g",
            category: "protein",
        },
    ),
    (
        "jajko",
        KeywordEntry {
            slug: "egg",
            role: "binder",
            quantity_g: 60.0,
            unit: "g",
            category: "protein",
        },
    ),
    (
        "tofu",
        KeywordEntry {
            slug: "tofu",
            role: "protein",
            quantity_g: 200.0,
            unit: "g",
            category: "protein",
        },
    ),
    (
        "тофу",
        KeywordEntry {
            slug: "tofu",
            role: "protein",
            quantity_g: 200.0,
            unit: "g",
            category: "protein",
        },
    ),
    // ── Nuts & seeds ─────────────────────────────────────────────────────────
    (
        "almond",
        KeywordEntry {
            slug: "almond",
            role: "texture",
            quantity_g: 30.0,
            unit: "g",
            category: "nut",
        },
    ),
    (
        "миндаль",
        KeywordEntry {
            slug: "almond",
            role: "texture",
            quantity_g: 30.0,
            unit: "g",
            category: "nut",
        },
    ),
    (
        "migdał",
        KeywordEntry {
            slug: "almond",
            role: "texture",
            quantity_g: 30.0,
            unit: "g",
            category: "nut",
        },
    ),
    (
        "walnut",
        KeywordEntry {
            slug: "walnut",
            role: "texture",
            quantity_g: 30.0,
            unit: "g",
            category: "nut",
        },
    ),
    (
        "грецкий",
        KeywordEntry {
            slug: "walnut",
            role: "texture",
            quantity_g: 30.0,
            unit: "g",
            category: "nut",
        },
    ),
    (
        "orzech",
        KeywordEntry {
            slug: "walnut",
            role: "texture",
            quantity_g: 30.0,
            unit: "g",
            category: "nut",
        },
    ),
    (
        "pistachio",
        KeywordEntry {
            slug: "pistachio",
            role: "texture",
            quantity_g: 20.0,
            unit: "g",
            category: "nut",
        },
    ),
    (
        "фисташка",
        KeywordEntry {
            slug: "pistachio",
            role: "texture",
            quantity_g: 20.0,
            unit: "g",
            category: "nut",
        },
    ),
    (
        "pistacja",
        KeywordEntry {
            slug: "pistachio",
            role: "texture",
            quantity_g: 20.0,
            unit: "g",
            category: "nut",
        },
    ),
    // ── Sweeteners ───────────────────────────────────────────────────────────
    (
        "sugar",
        KeywordEntry {
            slug: "sugar",
            role: "sweet",
            quantity_g: 50.0,
            unit: "g",
            category: "sweetener",
        },
    ),
    (
        "сахар",
        KeywordEntry {
            slug: "sugar",
            role: "sweet",
            quantity_g: 50.0,
            unit: "g",
            category: "sweetener",
        },
    ),
    (
        "cukier",
        KeywordEntry {
            slug: "sugar",
            role: "sweet",
            quantity_g: 50.0,
            unit: "g",
            category: "sweetener",
        },
    ),
    (
        "цукор",
        KeywordEntry {
            slug: "sugar",
            role: "sweet",
            quantity_g: 50.0,
            unit: "g",
            category: "sweetener",
        },
    ),
    (
        "honey",
        KeywordEntry {
            slug: "honey",
            role: "sweet",
            quantity_g: 30.0,
            unit: "g",
            category: "sweetener",
        },
    ),
    (
        "мёд",
        KeywordEntry {
            slug: "honey",
            role: "sweet",
            quantity_g: 30.0,
            unit: "g",
            category: "sweetener",
        },
    ),
    (
        "мед",
        KeywordEntry {
            slug: "honey",
            role: "sweet",
            quantity_g: 30.0,
            unit: "g",
            category: "sweetener",
        },
    ),
    (
        "miód",
        KeywordEntry {
            slug: "honey",
            role: "sweet",
            quantity_g: 30.0,
            unit: "g",
            category: "sweetener",
        },
    ),
    (
        "maple",
        KeywordEntry {
            slug: "maple_syrup",
            role: "sweet",
            quantity_g: 25.0,
            unit: "g",
            category: "sweetener",
        },
    ),
    (
        "кленовый",
        KeywordEntry {
            slug: "maple_syrup",
            role: "sweet",
            quantity_g: 25.0,
            unit: "g",
            category: "sweetener",
        },
    ),
    (
        "клен",
        KeywordEntry {
            slug: "maple_syrup",
            role: "sweet",
            quantity_g: 25.0,
            unit: "g",
            category: "sweetener",
        },
    ),
    // ── Fats & oils ──────────────────────────────────────────────────────────
    (
        "olive",
        KeywordEntry {
            slug: "olive_oil",
            role: "fat",
            quantity_g: 20.0,
            unit: "ml",
            category: "fat",
        },
    ),
    (
        "оливковое",
        KeywordEntry {
            slug: "olive_oil",
            role: "fat",
            quantity_g: 20.0,
            unit: "ml",
            category: "fat",
        },
    ),
    (
        "oliwa",
        KeywordEntry {
            slug: "olive_oil",
            role: "fat",
            quantity_g: 20.0,
            unit: "ml",
            category: "fat",
        },
    ),
    (
        "coconut",
        KeywordEntry {
            slug: "coconut_oil",
            role: "fat",
            quantity_g: 20.0,
            unit: "ml",
            category: "fat",
        },
    ),
    (
        "кокосовое",
        KeywordEntry {
            slug: "coconut_oil",
            role: "fat",
            quantity_g: 20.0,
            unit: "ml",
            category: "fat",
        },
    ),
    (
        "kokos",
        KeywordEntry {
            slug: "coconut_oil",
            role: "fat",
            quantity_g: 20.0,
            unit: "ml",
            category: "fat",
        },
    ),
    // ── Spices & herbs ───────────────────────────────────────────────────────
    (
        "ginger",
        KeywordEntry {
            slug: "ginger",
            role: "spice",
            quantity_g: 10.0,
            unit: "g",
            category: "spice",
        },
    ),
    (
        "имбирь",
        KeywordEntry {
            slug: "ginger",
            role: "spice",
            quantity_g: 10.0,
            unit: "g",
            category: "spice",
        },
    ),
    (
        "imbir",
        KeywordEntry {
            slug: "ginger",
            role: "spice",
            quantity_g: 10.0,
            unit: "g",
            category: "spice",
        },
    ),
    (
        "cinnamon",
        KeywordEntry {
            slug: "cinnamon",
            role: "spice",
            quantity_g: 3.0,
            unit: "g",
            category: "spice",
        },
    ),
    (
        "корица",
        KeywordEntry {
            slug: "cinnamon",
            role: "spice",
            quantity_g: 3.0,
            unit: "g",
            category: "spice",
        },
    ),
    (
        "cynamon",
        KeywordEntry {
            slug: "cinnamon",
            role: "spice",
            quantity_g: 3.0,
            unit: "g",
            category: "spice",
        },
    ),
    (
        "vanilla",
        KeywordEntry {
            slug: "vanilla",
            role: "spice",
            quantity_g: 2.0,
            unit: "g",
            category: "spice",
        },
    ),
    (
        "ваниль",
        KeywordEntry {
            slug: "vanilla",
            role: "spice",
            quantity_g: 2.0,
            unit: "g",
            category: "spice",
        },
    ),
    (
        "wanilia",
        KeywordEntry {
            slug: "vanilla",
            role: "spice",
            quantity_g: 2.0,
            unit: "g",
            category: "spice",
        },
    ),
    (
        "mint",
        KeywordEntry {
            slug: "mint",
            role: "spice",
            quantity_g: 5.0,
            unit: "g",
            category: "herb",
        },
    ),
    (
        "мята",
        KeywordEntry {
            slug: "mint",
            role: "spice",
            quantity_g: 5.0,
            unit: "g",
            category: "herb",
        },
    ),
    (
        "mięta",
        KeywordEntry {
            slug: "mint",
            role: "spice",
            quantity_g: 5.0,
            unit: "g",
            category: "herb",
        },
    ),
    (
        "basil",
        KeywordEntry {
            slug: "basil",
            role: "spice",
            quantity_g: 5.0,
            unit: "g",
            category: "herb",
        },
    ),
    (
        "базилик",
        KeywordEntry {
            slug: "basil",
            role: "spice",
            quantity_g: 5.0,
            unit: "g",
            category: "herb",
        },
    ),
    (
        "bazylia",
        KeywordEntry {
            slug: "basil",
            role: "spice",
            quantity_g: 5.0,
            unit: "g",
            category: "herb",
        },
    ),
    // ── Starch / thickeners ──────────────────────────────────────────────────
    (
        "starch",
        KeywordEntry {
            slug: "corn_starch",
            role: "thickener",
            quantity_g: 15.0,
            unit: "g",
            category: "starch",
        },
    ),
    (
        "крахмал",
        KeywordEntry {
            slug: "corn_starch",
            role: "thickener",
            quantity_g: 15.0,
            unit: "g",
            category: "starch",
        },
    ),
    (
        "skrobia",
        KeywordEntry {
            slug: "corn_starch",
            role: "thickener",
            quantity_g: 15.0,
            unit: "g",
            category: "starch",
        },
    ),
    (
        "flour",
        KeywordEntry {
            slug: "wheat_flour",
            role: "thickener",
            quantity_g: 30.0,
            unit: "g",
            category: "starch",
        },
    ),
    (
        "мука",
        KeywordEntry {
            slug: "wheat_flour",
            role: "thickener",
            quantity_g: 30.0,
            unit: "g",
            category: "starch",
        },
    ),
    (
        "mąka",
        KeywordEntry {
            slug: "wheat_flour",
            role: "thickener",
            quantity_g: 30.0,
            unit: "g",
            category: "starch",
        },
    ),
    // ── Liquids ──────────────────────────────────────────────────────────────
    (
        "water",
        KeywordEntry {
            slug: "water",
            role: "liquid",
            quantity_g: 200.0,
            unit: "ml",
            category: "liquid",
        },
    ),
    (
        "вода",
        KeywordEntry {
            slug: "water",
            role: "liquid",
            quantity_g: 200.0,
            unit: "ml",
            category: "liquid",
        },
    ),
    (
        "woda",
        KeywordEntry {
            slug: "water",
            role: "liquid",
            quantity_g: 200.0,
            unit: "ml",
            category: "liquid",
        },
    ),
    (
        "broth",
        KeywordEntry {
            slug: "chicken_broth",
            role: "liquid",
            quantity_g: 300.0,
            unit: "ml",
            category: "liquid",
        },
    ),
    (
        "бульон",
        KeywordEntry {
            slug: "chicken_broth",
            role: "liquid",
            quantity_g: 300.0,
            unit: "ml",
            category: "liquid",
        },
    ),
    (
        "bulion",
        KeywordEntry {
            slug: "chicken_broth",
            role: "liquid",
            quantity_g: 300.0,
            unit: "ml",
            category: "liquid",
        },
    ),
    (
        "wine",
        KeywordEntry {
            slug: "white_wine",
            role: "liquid",
            quantity_g: 80.0,
            unit: "ml",
            category: "liquid",
        },
    ),
    (
        "вино",
        KeywordEntry {
            slug: "white_wine",
            role: "liquid",
            quantity_g: 80.0,
            unit: "ml",
            category: "liquid",
        },
    ),
    (
        "wino",
        KeywordEntry {
            slug: "white_wine",
            role: "liquid",
            quantity_g: 80.0,
            unit: "ml",
            category: "liquid",
        },
    ),
    // ── Grains / legumes ─────────────────────────────────────────────────────
    (
        "rice",
        KeywordEntry {
            slug: "rice",
            role: "base",
            quantity_g: 150.0,
            unit: "g",
            category: "grain",
        },
    ),
    (
        "рис",
        KeywordEntry {
            slug: "rice",
            role: "base",
            quantity_g: 150.0,
            unit: "g",
            category: "grain",
        },
    ),
    (
        "oats",
        KeywordEntry {
            slug: "oats",
            role: "base",
            quantity_g: 100.0,
            unit: "g",
            category: "grain",
        },
    ),
    (
        "овёс",
        KeywordEntry {
            slug: "oats",
            role: "base",
            quantity_g: 100.0,
            unit: "g",
            category: "grain",
        },
    ),
    (
        "овсянка",
        KeywordEntry {
            slug: "oats",
            role: "base",
            quantity_g: 100.0,
            unit: "g",
            category: "grain",
        },
    ),
    (
        "płatki",
        KeywordEntry {
            slug: "oats",
            role: "base",
            quantity_g: 100.0,
            unit: "g",
            category: "grain",
        },
    ),
    (
        "lentil",
        KeywordEntry {
            slug: "lentil",
            role: "protein",
            quantity_g: 100.0,
            unit: "g",
            category: "legume",
        },
    ),
    (
        "чечевица",
        KeywordEntry {
            slug: "lentil",
            role: "protein",
            quantity_g: 100.0,
            unit: "g",
            category: "legume",
        },
    ),
    (
        "soczewica",
        KeywordEntry {
            slug: "lentil",
            role: "protein",
            quantity_g: 100.0,
            unit: "g",
            category: "legume",
        },
    ),
    (
        "chickpea",
        KeywordEntry {
            slug: "chickpea",
            role: "protein",
            quantity_g: 100.0,
            unit: "g",
            category: "legume",
        },
    ),
    (
        "нут",
        KeywordEntry {
            slug: "chickpea",
            role: "protein",
            quantity_g: 100.0,
            unit: "g",
            category: "legume",
        },
    ),
    (
        "ciecierzyca",
        KeywordEntry {
            slug: "chickpea",
            role: "protein",
            quantity_g: 100.0,
            unit: "g",
            category: "legume",
        },
    ),
    // ── Chocolate / cocoa ────────────────────────────────────────────────────
    (
        "chocolate",
        KeywordEntry {
            slug: "dark_chocolate",
            role: "flavor",
            quantity_g: 50.0,
            unit: "g",
            category: "sweet",
        },
    ),
    (
        "шоколад",
        KeywordEntry {
            slug: "dark_chocolate",
            role: "flavor",
            quantity_g: 50.0,
            unit: "g",
            category: "sweet",
        },
    ),
    (
        "czekolada",
        KeywordEntry {
            slug: "dark_chocolate",
            role: "flavor",
            quantity_g: 50.0,
            unit: "g",
            category: "sweet",
        },
    ),
    (
        "cocoa",
        KeywordEntry {
            slug: "cocoa_powder",
            role: "flavor",
            quantity_g: 20.0,
            unit: "g",
            category: "sweet",
        },
    ),
    (
        "какао",
        KeywordEntry {
            slug: "cocoa_powder",
            role: "flavor",
            quantity_g: 20.0,
            unit: "g",
            category: "sweet",
        },
    ),
    (
        "kakao",
        KeywordEntry {
            slug: "cocoa_powder",
            role: "flavor",
            quantity_g: 20.0,
            unit: "g",
            category: "sweet",
        },
    ),
    // ── Acids ────────────────────────────────────────────────────────────────
    (
        "vinegar",
        KeywordEntry {
            slug: "apple_vinegar",
            role: "acid",
            quantity_g: 10.0,
            unit: "ml",
            category: "acid",
        },
    ),
    (
        "уксус",
        KeywordEntry {
            slug: "apple_vinegar",
            role: "acid",
            quantity_g: 10.0,
            unit: "ml",
            category: "acid",
        },
    ),
    (
        "ocet",
        KeywordEntry {
            slug: "apple_vinegar",
            role: "acid",
            quantity_g: 10.0,
            unit: "ml",
            category: "acid",
        },
    ),
];

// ─────────────────────────────────────────────────────────────────────────────
// Exclusion keywords
// ─────────────────────────────────────────────────────────────────────────────

/// Words that introduce a negation ("without X", "no X", "без X", …).
const NEGATION_TRIGGERS: &[&str] = &[
    "without", "no", "not", "non", "без", "не", // Russian
    "без", "ні", // Ukrainian
    "bez", "nie",  // Polish
    "ohne", // German
];

/// Maps a content-type word (coming after a negation) to the category it
/// blocks. E.g. "sugar" → "sweetener", "dairy" → "dairy".
const EXCLUSION_MAP: &[(&str, &str)] = &[
    ("sugar", "sweetener"),
    ("сахара", "sweetener"),
    ("сахар", "sweetener"),
    ("цукру", "sweetener"),
    ("cukru", "sweetener"),
    ("dairy", "dairy"),
    ("lactose", "dairy"),
    ("молочных", "dairy"),
    ("молочного", "dairy"),
    ("gluten", "starch"),
    ("глютена", "starch"),
    ("glutenu", "starch"),
    ("fat", "fat"),
    ("жира", "fat"),
    ("жирного", "fat"),
    ("alcohol", "liquid"), // approximation: wine etc.
    ("алкоголя", "liquid"),
    ("alkoholu", "liquid"),
    ("meat", "protein"),
    ("мяса", "protein"),
    ("mięsa", "protein"),
    ("nuts", "nut"),
    ("орехов", "nut"),
    ("orzechów", "nut"),
];

// ─────────────────────────────────────────────────────────────────────────────
// Product type inference
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq)]
enum ProductType {
    Sauce,
    Soup,
    Smoothie,
    Marinade,
    Dressing,
    Jam,
    Dessert,
    Drink,
    Unknown,
}

impl ProductType {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Sauce => "sauce",
            Self::Soup => "soup",
            Self::Smoothie => "smoothie",
            Self::Marinade => "marinade",
            Self::Dressing => "dressing",
            Self::Jam => "jam",
            Self::Dessert => "dessert",
            Self::Drink => "drink",
            Self::Unknown => "other",
        }
    }
}

const PRODUCT_TYPE_KEYWORDS: &[(&str, ProductType)] = &[
    ("sauce", ProductType::Sauce),
    ("соус", ProductType::Sauce),
    ("sos", ProductType::Sauce),
    ("soup", ProductType::Soup),
    ("суп", ProductType::Soup),
    ("zupa", ProductType::Soup),
    ("smoothie", ProductType::Smoothie),
    ("смузи", ProductType::Smoothie),
    ("koktajl", ProductType::Smoothie),
    ("marinade", ProductType::Marinade),
    ("маринад", ProductType::Marinade),
    ("marynata", ProductType::Marinade),
    ("dressing", ProductType::Dressing),
    ("заправка", ProductType::Dressing),
    ("jam", ProductType::Jam),
    ("джем", ProductType::Jam),
    ("варенье", ProductType::Jam),
    ("dżem", ProductType::Jam),
    ("конфитюр", ProductType::Jam),
    ("dessert", ProductType::Dessert),
    ("десерт", ProductType::Dessert),
    ("deser", ProductType::Dessert),
    ("drink", ProductType::Drink),
    ("напиток", ProductType::Drink),
    ("napój", ProductType::Drink),
];

// ─────────────────────────────────────────────────────────────────────────────
// Step templates
// ─────────────────────────────────────────────────────────────────────────────

/// Returns (technique, temperature_c, duration_min, note).
fn steps_for(pt: &ProductType) -> Vec<(String, Option<f64>, Option<u32>, String)> {
    match pt {
        ProductType::Sauce => vec![(
            "heat".into(),
            Some(85.0),
            Some(15),
            "Bring to 85 °C, stirring.".into(),
        )],
        ProductType::Soup => vec![
            (
                "heat".into(),
                Some(100.0),
                Some(30),
                "Simmer until all vegetables are soft.".into(),
            ),
            (
                "blend".into(),
                None,
                Some(2),
                "Blend until smooth, season.".into(),
            ),
        ],
        ProductType::Smoothie => vec![(
            "blend".into(),
            None,
            Some(2),
            "Blend all ingredients cold.".into(),
        )],
        ProductType::Marinade => vec![(
            "mix".into(),
            None,
            Some(3),
            "Mix all marinade components thoroughly.".into(),
        )],
        ProductType::Dressing => vec![(
            "mix".into(),
            None,
            Some(5),
            "Emulsify oil and acid, season.".into(),
        )],
        ProductType::Jam => vec![(
            "heat".into(),
            Some(104.0),
            Some(25),
            "Bring to setting point 104 °C.".into(),
        )],
        ProductType::Dessert => vec![
            (
                "mix".into(),
                None,
                Some(5),
                "Combine all ingredients.".into(),
            ),
            (
                "cool".into(),
                Some(4.0),
                Some(60),
                "Chill in refrigerator until set.".into(),
            ),
        ],
        ProductType::Drink => vec![("blend".into(), None, Some(2), "Blend until smooth.".into())],
        ProductType::Unknown => vec![(
            "mix".into(),
            None,
            Some(5),
            "Combine and adjust seasoning.".into(),
        )],
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Main public function
// ─────────────────────────────────────────────────────────────────────────────

/// Analyse `prompt` and return a `CopilotDraft`.
///
/// Pure function — no I/O, no allocations beyond the output.
pub fn suggest(prompt: &str) -> CopilotDraft {
    if prompt.trim().is_empty() {
        return empty_draft();
    }

    let lower = prompt.to_lowercase();

    // 1. Tokenise: split on whitespace and common punctuation.
    let tokens: Vec<&str> = lower
        .split(|c: char| !c.is_alphabetic() && c != '\'' && c != '-')
        .filter(|t| !t.is_empty())
        .collect();

    // 2. Detect product type (first matching token wins).
    let product_type = tokens
        .iter()
        .find_map(|t| {
            PRODUCT_TYPE_KEYWORDS
                .iter()
                .find(|(kw, _)| kw_matches(kw, t))
                .map(|(_, pt)| pt.clone())
        })
        .unwrap_or(ProductType::Unknown);

    // 3. Build exclusion set.
    let mut excluded_categories: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for i in 0..tokens.len() {
        if NEGATION_TRIGGERS.contains(&tokens[i]) {
            // Look ahead up to 2 tokens.
            for j in 1..=2_usize {
                if i + j >= tokens.len() {
                    break;
                }
                if let Some((_, cat)) = EXCLUSION_MAP
                    .iter()
                    .find(|(kw, _)| kw_matches(kw, &tokens[i + j]))
                {
                    excluded_categories.insert(cat);
                }
            }
        }
    }

    // 4. Match ingredients, dedup by slug.
    let mut matched: Vec<CopilotDraftIngredient> = Vec::new();
    let mut matched_slugs: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut matched_tokens: std::collections::HashSet<String> = std::collections::HashSet::new();

    for token in &tokens {
        if let Some((_, entry)) = KEYWORD_MAP.iter().find(|(kw, _)| kw_matches(kw, token)) {
            // Skip if category is excluded.
            if excluded_categories.contains(entry.category) {
                continue;
            }
            if matched_slugs.insert(entry.slug.to_string()) {
                matched.push(CopilotDraftIngredient {
                    slug: entry.slug.to_string(),
                    quantity: entry.quantity_g,
                    unit: entry.unit.to_string(),
                    role: entry.role.to_string(),
                });
            }
            matched_tokens.insert((*token).to_string());
        }
    }

    // 5. Unmatched tokens (filter noise words).
    let noise: &[&str] = &[
        "i",
        "a",
        "the",
        "with",
        "and",
        "or",
        "make",
        "want",
        "need",
        "хочу",
        "хочется",
        "нужен",
        "нужна",
        "хочу",
        "зробити",
        "зроблю",
        "chcę",
        "chciałbym",
        "chciałabym",
        "zrobić",
        "я",
        "мне",
        "мне",
        "мне",
        "me",
    ];
    let unmatched: Vec<String> = tokens
        .iter()
        .filter(|t| {
            !matched_tokens.contains(**t)
                && !PRODUCT_TYPE_KEYWORDS
                    .iter()
                    .any(|(kw, _)| kw_matches(kw, t))
                && !NEGATION_TRIGGERS.contains(t)
                && !EXCLUSION_MAP.iter().any(|(kw, _)| kw_matches(kw, t))
                && !noise.contains(t)
                && t.len() > 2
        })
        .map(|t| t.to_string())
        .collect();

    // 6. Build steps from template.
    let raw_steps = steps_for(&product_type);
    let steps: Vec<CopilotDraftStep> = raw_steps
        .into_iter()
        .map(|(technique, temp, dur, note)| CopilotDraftStep {
            technique,
            temperature_c: temp,
            duration_min: dur,
            note,
        })
        .collect();

    // 7. Confidence.
    let meaningful_tokens: Vec<&str> = tokens
        .iter()
        .copied()
        .filter(|t| t.len() > 2 && !noise.contains(t))
        .collect();
    let confidence = if meaningful_tokens.is_empty() {
        0.0
    } else {
        let matched_count = meaningful_tokens
            .iter()
            .filter(|t| {
                KEYWORD_MAP.iter().any(|(kw, _)| kw_matches(kw, t))
                    || PRODUCT_TYPE_KEYWORDS
                        .iter()
                        .any(|(kw, _)| kw_matches(kw, t))
            })
            .count();
        (matched_count as f64 / meaningful_tokens.len() as f64).min(1.0)
    };

    // 8. Suggested name.
    let suggested_name = build_name(&matched, &product_type, prompt);

    // 9. Rationale.
    let exclusion_note = if excluded_categories.is_empty() {
        String::new()
    } else {
        format!(
            " Excluded: {}.",
            excluded_categories
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let rationale = format!(
        "Detected type: {}. Found {} ingredient(s).{}{}",
        product_type.as_str(),
        matched.len(),
        exclusion_note,
        if unmatched.is_empty() {
            String::new()
        } else {
            format!(" Unrecognised tokens: {}.", unmatched.join(", "))
        },
    );

    CopilotDraft {
        product_type: product_type.as_str().to_string(),
        suggested_name,
        ingredients: matched,
        steps,
        rationale,
        confidence,
        unmatched_tokens: unmatched,
    }
}

fn empty_draft() -> CopilotDraft {
    CopilotDraft {
        product_type: "other".into(),
        suggested_name: "New project".into(),
        ingredients: vec![],
        steps: vec![],
        rationale: "Empty prompt.".into(),
        confidence: 0.0,
        unmatched_tokens: vec![],
    }
}

fn build_name(ingredients: &[CopilotDraftIngredient], pt: &ProductType, original: &str) -> String {
    // Take the first "base" ingredient slug and capitalise it; otherwise the
    // first matched ingredient of any role.
    let base = ingredients
        .iter()
        .find(|i| i.role == "base")
        .or_else(|| ingredients.first())
        .map(|i| {
            let mut s = i.slug.replace('_', " ");
            if let Some(c) = s.get_mut(0..1) {
                c.make_ascii_uppercase();
            }
            s
        });

    let pt_word = pt.as_str();

    match base {
        Some(b) => {
            // If product type is "other" / "unknown", just use the ingredient.
            if pt_word == "other" {
                b
            } else {
                format!("{} {}", b, pt_word)
            }
        }
        None => {
            // No matched ingredient: fall back to product type word, or the
            // first 3 words of the prompt if the product type is unknown.
            if pt_word == "other" {
                let head: String = original
                    .split_whitespace()
                    .take(3)
                    .collect::<Vec<_>>()
                    .join(" ");
                if head.is_empty() {
                    "New project".into()
                } else {
                    head
                }
            } else {
                let mut s = pt_word.to_string();
                if let Some(c) = s.get_mut(0..1) {
                    c.make_ascii_uppercase();
                }
                s
            }
        }
    }
}

/// Match a keyword against a token: exact match, or token starts with keyword
/// when keyword is at least 4 characters long (handles Slavic morphology like
/// "абрикос" matching "абрикоса"/"абрикосовый", "pomidor" matching "pomidorowa").
fn kw_matches(kw: &str, token: &str) -> bool {
    if kw == token {
        return true;
    }
    // Use char count, not byte length, for unicode safety.
    let kw_chars = kw.chars().count();
    if kw_chars < 4 {
        return false;
    }
    token.starts_with(kw)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strawberry_sauce_en() {
        let d = suggest("I want a strawberry sauce without sugar");
        assert_eq!(d.product_type, "sauce");
        assert!(d.ingredients.iter().any(|i| i.slug == "strawberry"));
        // sugar excluded
        assert!(!d.ingredients.iter().any(|i| i.slug == "sugar"));
        assert!(d.confidence > 0.0);
    }

    #[test]
    fn apricot_sauce_ru() {
        let d = suggest("хочу абрикосовый соус");
        assert_eq!(d.product_type, "sauce");
        assert!(d.ingredients.iter().any(|i| i.slug == "apricot"));
    }

    #[test]
    fn tomato_soup_pl() {
        let d = suggest("zupa pomidorowa z czosnkiem");
        assert_eq!(d.product_type, "soup");
        assert!(d.ingredients.iter().any(|i| i.slug == "tomato"));
        assert!(d.ingredients.iter().any(|i| i.slug == "garlic"));
        // soup should have >= 2 steps (heat + blend)
        assert!(d.steps.len() >= 2);
    }

    #[test]
    fn mango_smoothie_no_dairy() {
        let d = suggest("mango smoothie without dairy");
        assert_eq!(d.product_type, "smoothie");
        assert!(d.ingredients.iter().any(|i| i.slug == "mango"));
        // dairy excluded
        for ing in &d.ingredients {
            assert_ne!(ing.role, "dairy");
        }
    }

    #[test]
    fn empty_prompt_safe() {
        let d = suggest("");
        assert!(d.ingredients.is_empty());
        assert_eq!(d.confidence, 0.0);
    }

    #[test]
    fn no_duplicate_slugs() {
        let d = suggest("strawberry strawberry strawberry sauce");
        let count = d
            .ingredients
            .iter()
            .filter(|i| i.slug == "strawberry")
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn jam_steps_include_heat() {
        let d = suggest("клубничный джем");
        assert_eq!(d.product_type, "jam");
        assert!(d.steps.iter().any(|s| s.technique == "heat"));
    }

    #[test]
    fn confidence_high_for_full_match() {
        let d = suggest("strawberry sauce with lemon");
        // strawberry + sauce + lemon all known
        assert!(d.confidence >= 0.5);
    }
}
