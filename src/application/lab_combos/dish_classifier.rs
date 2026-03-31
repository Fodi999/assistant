// ─── DishClassifier — deterministic dish type classification ─────────────
//
// Extracts from dish name:
// 1. DishType (sticks, cutlets, soup, salad, bowl, pasta, baked, stir_fry, etc.)
// 2. Required steps (forming, liquid_base, etc.)
// 3. Allowed techniques (fry, bake, boil, raw_assembly, etc.)
// 4. Forbidden techniques per dish type
// 5. Expected texture descriptors
//
// This is the SINGLE SOURCE OF TRUTH for culinary logic.
// AI receives this as constraints — not as suggestions.

use serde::{Deserialize, Serialize};

// ── DishType enum ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DishType {
    /// Палочки, наггетсы, крокеты — formed + fried/baked
    Sticks,
    /// Котлеты, тефтели, фрикадельки — formed + fried/baked
    Cutlets,
    /// Блинчики, оладьи, панкейки — batter poured + fried
    Pancakes,
    /// Суп, бульон, крем-суп — liquid base required
    Soup,
    /// Салат — raw assembly + dressing
    Salad,
    /// Боул, поке — assembly, minimal cooking
    Bowl,
    /// Паста, ризотто — grain-based, sauce
    Pasta,
    /// Запеканка, гратен — layered + baked
    Casserole,
    /// Жареный рис, стир-фрай — wok/pan high heat
    StirFry,
    /// Запечённое блюдо — oven technique
    Baked,
    /// Рулет, шаурма, ролл — wrap/roll forming
    Wrap,
    /// Омлет, яичница — egg-based pan dish
    Omelette,
    /// Каша, овсянка — porridge
    Porridge,
    /// Смузи — blended
    Smoothie,
    /// Generic fallback — AI decides technique
    Generic,
}

// ── CookingTechnique ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CookingTechnique {
    Fry,
    DeepFry,
    Bake,
    Boil,
    Steam,
    Grill,
    StirFry,
    Braise,
    RawAssembly,
    Blend,
    Simmer,
}

// ── TextureDescriptor ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextureDescriptor {
    pub outside: &'static str,   // "хрустящая золотистая корочка"
    pub inside: &'static str,    // "мягкая тянущаяся начинка"
    pub temperature: &'static str, // "горячее" / "холодное" / "тёплое"
}

// ── DishProfile — complete classification result ────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DishProfile {
    pub dish_type: DishType,
    /// Human-readable type name for prompts
    pub type_label: &'static str,
    /// Forming step required (палочки, котлеты, шарики)
    pub requires_forming: bool,
    /// Liquid base required (суп, крем-суп)
    pub requires_liquid: bool,
    /// Oven required (запеканка, запечённый)
    pub requires_oven: bool,
    /// Allowed cooking techniques for this dish type
    pub allowed_techniques: Vec<CookingTechnique>,
    /// Forbidden techniques (e.g. boiling sticks is nonsense)
    pub forbidden_techniques: Vec<CookingTechnique>,
    /// Expected texture for validation
    pub expected_texture: TextureDescriptor,
    /// Minimum cooking steps
    pub min_steps: usize,
}

// ── Classification engine ───────────────────────────────────────────────────

/// Classify a dish name into a DishProfile.
/// Works with Russian, English, Polish, Ukrainian dish names.
pub fn classify_dish(dish_name: &str) -> DishProfile {
    let name = dish_name.to_lowercase();

    // ── Sticks / Палочки / Наггетсы ─────────────────────────────────
    if contains_any(&name, &[
        "палочк", "палички", "stick", "наггетс", "nugget", "крокет", "croquette",
        "палушк", "paluszkі", "paluszk",
    ]) {
        return DishProfile {
            dish_type: DishType::Sticks,
            type_label: "sticks/nuggets",
            requires_forming: true,
            requires_liquid: false,
            requires_oven: false,
            allowed_techniques: vec![CookingTechnique::Fry, CookingTechnique::DeepFry, CookingTechnique::Bake],
            forbidden_techniques: vec![CookingTechnique::Boil, CookingTechnique::Steam, CookingTechnique::RawAssembly],
            expected_texture: TextureDescriptor {
                outside: "golden crispy crust",
                inside: "soft, tender inside",
                temperature: "hot",
            },
            min_steps: 4,
        };
    }

    // ── Cutlets / Котлеты / Тефтели ─────────────────────────────────
    if contains_any(&name, &[
        "котлет", "тефтел", "фрикадельк", "cutlet", "patties", "meatball",
        "биточк", "кнел", "зраз", "kotlet", "klopsik",
    ]) {
        return DishProfile {
            dish_type: DishType::Cutlets,
            type_label: "cutlets/patties",
            requires_forming: true,
            requires_liquid: false,
            requires_oven: false,
            allowed_techniques: vec![CookingTechnique::Fry, CookingTechnique::Bake, CookingTechnique::Braise],
            forbidden_techniques: vec![CookingTechnique::RawAssembly, CookingTechnique::Blend],
            expected_texture: TextureDescriptor {
                outside: "golden-brown seared crust",
                inside: "juicy, tender",
                temperature: "hot",
            },
            min_steps: 4,
        };
    }

    // ── Pancakes / Блинчики / Оладьи ────────────────────────────────
    if contains_any(&name, &[
        "блинч", "блін", "оладь", "оладк", "панкейк", "pancake", "crepe",
        "naleśnik", "placki", "placuszk",
    ]) {
        return DishProfile {
            dish_type: DishType::Pancakes,
            type_label: "pancakes/crepes",
            requires_forming: false, // batter is poured, not manually formed
            requires_liquid: false,
            requires_oven: false,
            allowed_techniques: vec![CookingTechnique::Fry],
            forbidden_techniques: vec![CookingTechnique::Boil, CookingTechnique::Bake, CookingTechnique::RawAssembly],
            expected_texture: TextureDescriptor {
                outside: "golden, lightly crispy edges",
                inside: "soft, fluffy",
                temperature: "hot",
            },
            min_steps: 3,
        };
    }

    // ── Soup / Суп ──────────────────────────────────────────────────
    if contains_any(&name, &[
        "суп", "бульон", "крем-суп", "soup", "broth", "chowder", "bisque",
        "zupa", "rosół", "борщ", "borscht", "щи", "солянк", "гаспачо", "gazpacho",
        "похлёбк", "юшка",
    ]) {
        return DishProfile {
            dish_type: DishType::Soup,
            type_label: "soup",
            requires_forming: false,
            requires_liquid: true,
            requires_oven: false,
            allowed_techniques: vec![CookingTechnique::Boil, CookingTechnique::Simmer, CookingTechnique::Braise],
            forbidden_techniques: vec![CookingTechnique::Fry, CookingTechnique::DeepFry, CookingTechnique::RawAssembly, CookingTechnique::Bake],
            expected_texture: TextureDescriptor {
                outside: "rich aromatic broth",
                inside: "tender pieces in liquid",
                temperature: "hot",
            },
            min_steps: 4,
        };
    }

    // ── Salad / Салат ───────────────────────────────────────────────
    if contains_any(&name, &[
        "салат", "salad", "sałatk", "цезарь", "caesar", "вінегрет", "vinaigrette",
    ]) {
        return DishProfile {
            dish_type: DishType::Salad,
            type_label: "salad",
            requires_forming: false,
            requires_liquid: false,
            requires_oven: false,
            allowed_techniques: vec![CookingTechnique::RawAssembly],
            forbidden_techniques: vec![CookingTechnique::DeepFry, CookingTechnique::Braise],
            expected_texture: TextureDescriptor {
                outside: "fresh, vibrant colors",
                inside: "crisp, varied textures",
                temperature: "cold or room temperature",
            },
            min_steps: 3,
        };
    }

    // ── Bowl / Боул / Поке ──────────────────────────────────────────
    if contains_any(&name, &[
        "боул", "bowl", "поке", "poke", "будда", "buddha",
    ]) {
        return DishProfile {
            dish_type: DishType::Bowl,
            type_label: "bowl",
            requires_forming: false,
            requires_liquid: false,
            requires_oven: false,
            allowed_techniques: vec![CookingTechnique::RawAssembly, CookingTechnique::Boil, CookingTechnique::Fry],
            forbidden_techniques: vec![CookingTechnique::DeepFry],
            expected_texture: TextureDescriptor {
                outside: "colorful layered presentation",
                inside: "mix of textures — soft, crunchy, creamy",
                temperature: "warm or room temperature",
            },
            min_steps: 3,
        };
    }

    // ── Pasta / Паста / Ризотто ─────────────────────────────────────
    if contains_any(&name, &[
        "паста", "pasta", "ризотто", "risotto", "спагетти", "spaghetti", "пенне",
        "фетучіні", "fettuccine", "лазанья", "lasagna", "makaron",
    ]) {
        return DishProfile {
            dish_type: DishType::Pasta,
            type_label: "pasta/risotto",
            requires_forming: false,
            requires_liquid: false,
            requires_oven: false,
            allowed_techniques: vec![CookingTechnique::Boil, CookingTechnique::Fry, CookingTechnique::Simmer, CookingTechnique::Bake],
            forbidden_techniques: vec![CookingTechnique::RawAssembly, CookingTechnique::DeepFry],
            expected_texture: TextureDescriptor {
                outside: "al dente pasta coated in sauce",
                inside: "creamy/rich sauce",
                temperature: "hot",
            },
            min_steps: 3,
        };
    }

    // ── Casserole / Запеканка / Гратен ──────────────────────────────
    if contains_any(&name, &[
        "запеканк", "гратен", "casserole", "gratin", "запіканка",
    ]) {
        return DishProfile {
            dish_type: DishType::Casserole,
            type_label: "casserole/gratin",
            requires_forming: false,
            requires_liquid: false,
            requires_oven: true,
            allowed_techniques: vec![CookingTechnique::Bake],
            forbidden_techniques: vec![CookingTechnique::Fry, CookingTechnique::DeepFry, CookingTechnique::RawAssembly, CookingTechnique::Boil],
            expected_texture: TextureDescriptor {
                outside: "golden baked top crust",
                inside: "soft, layered filling",
                temperature: "hot",
            },
            min_steps: 4,
        };
    }

    // ── Stir-fry / Жареный рис / Вок ───────────────────────────────
    if contains_any(&name, &[
        "жареный рис", "стир-фрай", "stir-fry", "stir fry", "вок", "wok",
        "smażony ryż",
    ]) {
        return DishProfile {
            dish_type: DishType::StirFry,
            type_label: "stir-fry",
            requires_forming: false,
            requires_liquid: false,
            requires_oven: false,
            allowed_techniques: vec![CookingTechnique::StirFry, CookingTechnique::Fry],
            forbidden_techniques: vec![CookingTechnique::Boil, CookingTechnique::Bake, CookingTechnique::RawAssembly],
            expected_texture: TextureDescriptor {
                outside: "slightly charred, wok hei",
                inside: "tender with crispy bits",
                temperature: "hot",
            },
            min_steps: 4,
        };
    }

    // ── Baked / Запечённый ──────────────────────────────────────────
    if contains_any(&name, &[
        "запечён", "запечен", "запіч", "baked", "roast", "pieczony",
    ]) {
        return DishProfile {
            dish_type: DishType::Baked,
            type_label: "baked",
            requires_forming: false,
            requires_liquid: false,
            requires_oven: true,
            allowed_techniques: vec![CookingTechnique::Bake, CookingTechnique::Grill],
            forbidden_techniques: vec![CookingTechnique::DeepFry, CookingTechnique::RawAssembly, CookingTechnique::Boil],
            expected_texture: TextureDescriptor {
                outside: "golden roasted surface",
                inside: "tender, juicy",
                temperature: "hot",
            },
            min_steps: 3,
        };
    }

    // ── Wrap / Рулет / Шаурма ───────────────────────────────────────
    if contains_any(&name, &[
        "рулет", "шаурма", "ролл", "wrap", "roll", "burrito", "буріто",
    ]) {
        return DishProfile {
            dish_type: DishType::Wrap,
            type_label: "wrap/roll",
            requires_forming: true,
            requires_liquid: false,
            requires_oven: false,
            allowed_techniques: vec![CookingTechnique::RawAssembly, CookingTechnique::Fry, CookingTechnique::Bake],
            forbidden_techniques: vec![CookingTechnique::Boil, CookingTechnique::DeepFry],
            expected_texture: TextureDescriptor {
                outside: "soft tortilla/wrap",
                inside: "layered filling",
                temperature: "warm",
            },
            min_steps: 3,
        };
    }

    // ── Omelette / Омлет ────────────────────────────────────────────
    if contains_any(&name, &[
        "омлет", "omelette", "omlet", "яичниц", "scramble", "фритата", "frittata",
    ]) {
        return DishProfile {
            dish_type: DishType::Omelette,
            type_label: "omelette/eggs",
            requires_forming: false,
            requires_liquid: false,
            requires_oven: false,
            allowed_techniques: vec![CookingTechnique::Fry, CookingTechnique::Bake],
            forbidden_techniques: vec![CookingTechnique::Boil, CookingTechnique::DeepFry, CookingTechnique::RawAssembly],
            expected_texture: TextureDescriptor {
                outside: "lightly golden surface",
                inside: "soft, creamy, just-set",
                temperature: "hot",
            },
            min_steps: 3,
        };
    }

    // ── Porridge / Каша ─────────────────────────────────────────────
    if contains_any(&name, &[
        "каша", "овсянк", "porridge", "oatmeal", "owsiank",
    ]) {
        return DishProfile {
            dish_type: DishType::Porridge,
            type_label: "porridge",
            requires_forming: false,
            requires_liquid: true,
            requires_oven: false,
            allowed_techniques: vec![CookingTechnique::Boil, CookingTechnique::Simmer],
            forbidden_techniques: vec![CookingTechnique::Fry, CookingTechnique::DeepFry, CookingTechnique::Bake],
            expected_texture: TextureDescriptor {
                outside: "creamy smooth surface",
                inside: "thick, creamy consistency",
                temperature: "hot",
            },
            min_steps: 3,
        };
    }

    // ── Smoothie / Смузі ────────────────────────────────────────────
    if contains_any(&name, &[
        "смузі", "смузи", "smoothie", "шейк", "shake", "koktajl",
    ]) {
        return DishProfile {
            dish_type: DishType::Smoothie,
            type_label: "smoothie",
            requires_forming: false,
            requires_liquid: true,
            requires_oven: false,
            allowed_techniques: vec![CookingTechnique::Blend],
            forbidden_techniques: vec![CookingTechnique::Fry, CookingTechnique::Bake, CookingTechnique::Boil],
            expected_texture: TextureDescriptor {
                outside: "smooth, thick pour",
                inside: "creamy, uniform",
                temperature: "cold",
            },
            min_steps: 2,
        };
    }

    // ── Fried (generic) / Жареный ───────────────────────────────────
    if contains_any(&name, &[
        "жарен", "смажен", "fried", "smażon",
    ]) {
        return DishProfile {
            dish_type: DishType::StirFry,
            type_label: "fried dish",
            requires_forming: false,
            requires_liquid: false,
            requires_oven: false,
            allowed_techniques: vec![CookingTechnique::Fry, CookingTechnique::StirFry],
            forbidden_techniques: vec![CookingTechnique::Boil, CookingTechnique::RawAssembly],
            expected_texture: TextureDescriptor {
                outside: "golden, slightly crispy",
                inside: "tender, cooked through",
                temperature: "hot",
            },
            min_steps: 3,
        };
    }

    // ── Generic fallback ────────────────────────────────────────────
    DishProfile {
        dish_type: DishType::Generic,
        type_label: "dish",
        requires_forming: false,
        requires_liquid: false,
        requires_oven: false,
        allowed_techniques: vec![
            CookingTechnique::Fry, CookingTechnique::Bake, CookingTechnique::Boil,
            CookingTechnique::Grill, CookingTechnique::RawAssembly, CookingTechnique::StirFry,
        ],
        forbidden_techniques: vec![],
        expected_texture: TextureDescriptor {
            outside: "well-presented",
            inside: "properly cooked",
            temperature: "hot",
        },
        min_steps: 3,
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn contains_any(text: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|p| text.contains(p))
}

impl std::fmt::Display for DishType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DishType::Sticks => write!(f, "sticks"),
            DishType::Cutlets => write!(f, "cutlets"),
            DishType::Pancakes => write!(f, "pancakes"),
            DishType::Soup => write!(f, "soup"),
            DishType::Salad => write!(f, "salad"),
            DishType::Bowl => write!(f, "bowl"),
            DishType::Pasta => write!(f, "pasta"),
            DishType::Casserole => write!(f, "casserole"),
            DishType::StirFry => write!(f, "stir_fry"),
            DishType::Baked => write!(f, "baked"),
            DishType::Wrap => write!(f, "wrap"),
            DishType::Omelette => write!(f, "omelette"),
            DishType::Porridge => write!(f, "porridge"),
            DishType::Smoothie => write!(f, "smoothie"),
            DishType::Generic => write!(f, "generic"),
        }
    }
}

impl std::fmt::Display for CookingTechnique {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CookingTechnique::Fry => write!(f, "fry"),
            CookingTechnique::DeepFry => write!(f, "deep_fry"),
            CookingTechnique::Bake => write!(f, "bake"),
            CookingTechnique::Boil => write!(f, "boil"),
            CookingTechnique::Steam => write!(f, "steam"),
            CookingTechnique::Grill => write!(f, "grill"),
            CookingTechnique::StirFry => write!(f, "stir_fry"),
            CookingTechnique::Braise => write!(f, "braise"),
            CookingTechnique::RawAssembly => write!(f, "raw_assembly"),
            CookingTechnique::Blend => write!(f, "blend"),
            CookingTechnique::Simmer => write!(f, "simmer"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_sticks() {
        let p = classify_dish("Домашние сырные палочки");
        assert_eq!(p.dish_type, DishType::Sticks);
        assert!(p.requires_forming);
        assert!(!p.requires_liquid);
        assert!(p.forbidden_techniques.contains(&CookingTechnique::Boil));
    }

    #[test]
    fn test_classify_soup() {
        let p = classify_dish("Куриный суп с лапшой");
        assert_eq!(p.dish_type, DishType::Soup);
        assert!(!p.requires_forming);
        assert!(p.requires_liquid);
        assert!(p.forbidden_techniques.contains(&CookingTechnique::Fry));
    }

    #[test]
    fn test_classify_salad() {
        let p = classify_dish("Средиземноморский салат с тунцом");
        assert_eq!(p.dish_type, DishType::Salad);
        assert!(!p.requires_forming);
    }

    #[test]
    fn test_classify_cutlets() {
        let p = classify_dish("Куриные котлеты с сыром");
        assert_eq!(p.dish_type, DishType::Cutlets);
        assert!(p.requires_forming);
    }

    #[test]
    fn test_classify_risotto() {
        let p = classify_dish("Ризотто с грибами");
        assert_eq!(p.dish_type, DishType::Pasta);
        assert!(!p.requires_forming);
    }

    #[test]
    fn test_classify_generic() {
        let p = classify_dish("Лосось с рисом");
        assert_eq!(p.dish_type, DishType::Generic);
    }

    #[test]
    fn test_classify_english() {
        let p = classify_dish("Chicken Meatballs");
        assert_eq!(p.dish_type, DishType::Cutlets);
        assert!(p.requires_forming);
    }

    #[test]
    fn test_classify_baked() {
        let p = classify_dish("Запечённая курица с овощами");
        assert_eq!(p.dish_type, DishType::Baked);
        assert!(p.requires_oven);
    }
}
