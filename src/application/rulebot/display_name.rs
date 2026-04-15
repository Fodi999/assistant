//! Display Name — multilingual display names, grammar (ru/pl/uk), state labels, text formatting.
//!
//! Responsibilities:
//!   - `build_display_name`: "Борщ с говядиной", "Lekki barszcz z wołowiną"
//!   - Russian grammar: accusative (Додать муку), instrumental (с говядиной)
//!   - Polish grammar: accusative (Dodać cebulę), instrumental (z kurczakiem)
//!   - Ukrainian grammar: accusative (Додати цибулю), instrumental (з яловичиною)
//!   - `state_label`: localized cooking states (raw/boiled/fried/… → 4 languages)
//!   - `format_recipe_text`: short intro text for chat response
//!
//! Pure text transforms — no IO, no LLM.

use super::intent_router::ChatLang;
use super::response_builder::HealthGoal;
use super::recipe_engine::{ResolvedIngredient, DishType, TechCard};
use super::dish_schema::DishSchema;

// ── Display Name Builder ─────────────────────────────────────────────────────

/// Build an improved display name with goal prefix (multilingual).
pub fn build_display_name(
    schema: &DishSchema,
    ingredients: &[ResolvedIngredient],
    _dish_type: DishType,
    goal: HealthGoal,
    lang: ChatLang,
) -> String {
    let dish_local = schema.dish_local.as_deref().unwrap_or(&schema.dish);

    let has_with = {
        let d = dish_local.to_lowercase();
        d.contains(" с ") || d.contains(" with ") || d.contains(" z ") || d.contains(" з ")
    };

    let base_name = if has_with {
        dish_local.to_string()
    } else {
        let protein_name = ingredients.iter()
            .find(|i| i.role == "protein")
            .and_then(|i| i.product.as_ref())
            .map(|p| match lang {
                ChatLang::Ru => instrumental_case(&p.name_ru),
                ChatLang::En => p.name_en.to_lowercase(),
                ChatLang::Pl => instrumental_case_pl(&p.name_pl),
                ChatLang::Uk => instrumental_case_uk(&p.name_uk),
            });

        if let Some(protein) = protein_name {
            let prep = match lang {
                ChatLang::Ru => "с",
                ChatLang::En => "with",
                ChatLang::Pl => "z",
                ChatLang::Uk => "з",
            };
            format!("{} {} {}", dish_local, prep, protein)
        } else {
            dish_local.to_string()
        }
    };

    match (goal, lang) {
        (HealthGoal::LowCalorie, ChatLang::Ru) => format!("Лёгкий {}", lowercase_first(&base_name)),
        (HealthGoal::LowCalorie, ChatLang::En) => format!("Light {}", lowercase_first(&base_name)),
        (HealthGoal::LowCalorie, ChatLang::Pl) => format!("Lekki {}", lowercase_first(&base_name)),
        (HealthGoal::LowCalorie, ChatLang::Uk) => format!("Легкий {}", lowercase_first(&base_name)),

        (HealthGoal::HighProtein, ChatLang::Ru) => format!("Высокобелковый {}", lowercase_first(&base_name)),
        (HealthGoal::HighProtein, ChatLang::En) => format!("High-protein {}", lowercase_first(&base_name)),
        (HealthGoal::HighProtein, ChatLang::Pl) => format!("Wysokobiałkowy {}", lowercase_first(&base_name)),
        (HealthGoal::HighProtein, ChatLang::Uk) => format!("Високобілковий {}", lowercase_first(&base_name)),

        (HealthGoal::Balanced, _) => base_name,
    }
}

// ── Text Formatting ──────────────────────────────────────────────────────────

pub fn format_recipe_text(card: &TechCard, lang: ChatLang) -> String {
    let dish = card.display_name.as_deref()
        .unwrap_or_else(|| card.dish_name_local.as_deref().unwrap_or(&card.dish_name));

    let total_time: u16 = card.steps.iter().filter_map(|s| s.time_min).sum();
    let time_str = fmt_time(total_time);

    let intro = match lang {
        ChatLang::Ru => format!(
            "🍽 **{}** — {} порц. • ~{:.0}г •{} {} ккал на порцию",
            dish, card.servings, card.total_output_g / card.servings as f32,
            time_str, card.per_serving_kcal,
        ),
        ChatLang::En => format!(
            "🍽 **{}** — {} serv. • ~{:.0}g •{} {} kcal/serv",
            dish, card.servings, card.total_output_g / card.servings as f32,
            time_str, card.per_serving_kcal,
        ),
        ChatLang::Pl => format!(
            "🍽 **{}** — {} porcji • ~{:.0}g •{} {} kcal/porcja",
            dish, card.servings, card.total_output_g / card.servings as f32,
            time_str, card.per_serving_kcal,
        ),
        ChatLang::Uk => format!(
            "🍽 **{}** — {} порц. • ~{:.0}г •{} {} ккал/порція",
            dish, card.servings, card.total_output_g / card.servings as f32,
            time_str, card.per_serving_kcal,
        ),
    };

    let mut out = vec![intro];

    if !card.unresolved.is_empty() {
        let warn = match lang {
            ChatLang::Ru => format!("⚠️ Не в базе: {}", card.unresolved.join(", ")),
            ChatLang::En => format!("⚠️ Not in DB: {}", card.unresolved.join(", ")),
            ChatLang::Pl => format!("⚠️ Brak w bazie: {}", card.unresolved.join(", ")),
            ChatLang::Uk => format!("⚠️ Нема в базі: {}", card.unresolved.join(", ")),
        };
        out.push(warn);
    }

    out.join("\n")
}

fn fmt_time(min: u16) -> String {
    if min == 0 { return String::new(); }
    let rounded = ((min as f32 / 5.0).round() as u16).max(5);
    if rounded < 60 {
        format!(" ⏱ ~{} мин", rounded)
    } else {
        let h = rounded / 60;
        let m = rounded % 60;
        if m == 0 { format!(" ⏱ ~{} ч", h) }
        else { format!(" ⏱ ~{} ч {} мин", h, m) }
    }
}

// ── State Labels ─────────────────────────────────────────────────────────────

pub fn state_label<'a>(state: &'a str, lang: ChatLang) -> &'a str {
    match (state, lang) {
        ("raw", ChatLang::Ru) => "сырой", ("raw", ChatLang::En) => "raw",
        ("raw", ChatLang::Pl) => "surowy", ("raw", ChatLang::Uk) => "сирий",
        ("boiled", ChatLang::Ru) => "варёный", ("boiled", ChatLang::En) => "boiled",
        ("boiled", ChatLang::Pl) => "gotowany", ("boiled", ChatLang::Uk) => "варений",
        ("fried", ChatLang::Ru) => "жареный", ("fried", ChatLang::En) => "fried",
        ("fried", ChatLang::Pl) => "smażony", ("fried", ChatLang::Uk) => "смажений",
        ("sauteed", ChatLang::Ru) => "пассерованный", ("sauteed", ChatLang::En) => "sautéed",
        ("sauteed", ChatLang::Pl) => "podsmażony", ("sauteed", ChatLang::Uk) => "спасерований",
        ("baked", ChatLang::Ru) => "запечённый", ("baked", ChatLang::En) => "baked",
        ("baked", ChatLang::Pl) => "pieczony", ("baked", ChatLang::Uk) => "запечений",
        ("grilled", ChatLang::Ru) => "гриль", ("grilled", ChatLang::En) => "grilled",
        ("grilled", ChatLang::Pl) => "grillowany", ("grilled", ChatLang::Uk) => "гриль",
        ("steamed", ChatLang::Ru) => "на пару", ("steamed", ChatLang::En) => "steamed",
        ("steamed", ChatLang::Pl) => "na parze", ("steamed", ChatLang::Uk) => "на парі",
        ("smoked", ChatLang::Ru) => "копчёный", ("smoked", ChatLang::En) => "smoked",
        ("smoked", ChatLang::Pl) => "wędzony", ("smoked", ChatLang::Uk) => "копчений",
        _ => state,
    }
}

pub fn state_label_ru(state: &str, name_ru: &str) -> String {
    let gender = ru_gender(name_ru);
    match state {
        "raw" => match gender { 'f' => "сырая", 'n' => "сырое", _ => "сырой" }.into(),
        "boiled" => match gender { 'f' => "варёная", 'n' => "варёное", _ => "варёный" }.into(),
        "fried" => match gender { 'f' => "жареная", 'n' => "жареное", _ => "жареный" }.into(),
        "sauteed" => match gender { 'f' => "пассерованная", 'n' => "пассерованное", _ => "пассерованный" }.into(),
        "baked" => match gender { 'f' => "запечённая", 'n' => "запечённое", _ => "запечённый" }.into(),
        "grilled" => "гриль".into(),
        "steamed" => "на пару".into(),
        "smoked" => match gender { 'f' => "копчёная", 'n' => "копчёное", _ => "копчёный" }.into(),
        _ => state.into(),
    }
}

pub fn ru_gender(name: &str) -> char {
    let lower = name.to_lowercase();
    let lower = lower.trim();

    if lower.ends_with('ь') {
        const FEM_SOFT: &[&str] = &[
            "морковь", "фасоль", "соль", "ваниль", "зелень",
            "форель", "печень", "стручковая фасоль",
        ];
        for w in FEM_SOFT {
            if lower == *w { return 'f'; }
        }
        return 'm';
    }

    if lower.ends_with('а') || lower.ends_with('я') { 'f' }
    else if lower.ends_with('о') || lower.ends_with('е') || lower.ends_with('ё') { 'n' }
    else { 'm' }
}

fn lowercase_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_lowercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

// ── Russian Grammar ──────────────────────────────────────────────────────────

pub fn instrumental_word(word: &str) -> String {
    let lower = word.to_lowercase();
    if lower.ends_with("ая") { return format!("{}ой", &lower[..lower.len() - "ая".len()]); }
    if lower.ends_with("яя") { return format!("{}ей", &lower[..lower.len() - "яя".len()]); }
    if lower.ends_with("ое") || lower.ends_with("ее") { return format!("{}ым", &lower[..lower.len() - "ое".len()]); }
    if lower.ends_with("ые") { return format!("{}ыми", &lower[..lower.len() - "ые".len()]); }
    if lower.ends_with("ие") { return format!("{}ими", &lower[..lower.len() - "ие".len()]); }
    if lower.ends_with("ый") || lower.ends_with("ой") { return format!("{}ым", &lower[..lower.len() - "ый".len()]); }
    if lower.ends_with("ий") { return format!("{}им", &lower[..lower.len() - "ий".len()]); }
    if is_neuter_plural_a(&lower) { return format!("{}ми", lower); }
    if lower.ends_with('а') { return format!("{}ой", &lower[..lower.len() - 'а'.len_utf8()]); }
    if lower.ends_with('я') { return format!("{}ей", &lower[..lower.len() - 'я'.len_utf8()]); }
    if lower.ends_with('о') { return format!("{}ом", &lower[..lower.len() - 'о'.len_utf8()]); }
    if lower.ends_with('ь') { return format!("{}ью", &lower[..lower.len() - 'ь'.len_utf8()]); }
    if lower.ends_with("ец") { return format!("{}цем", &lower[..lower.len() - "ец".len()]); }
    format!("{}ом", lower)
}

pub fn accusative_word(word: &str) -> String {
    let lower = word.to_lowercase();
    if lower.ends_with("ая") { return format!("{}ую", &lower[..lower.len() - "ая".len()]); }
    if lower.ends_with("яя") { return format!("{}юю", &lower[..lower.len() - "яя".len()]); }
    if is_neuter_plural_a(&lower) { return lower; }
    if lower.ends_with('а') { return format!("{}у", &lower[..lower.len() - 'а'.len_utf8()]); }
    if lower.ends_with('я') { return format!("{}ю", &lower[..lower.len() - 'я'.len_utf8()]); }
    lower
}

fn is_neuter_plural_a(word: &str) -> bool {
    matches!(word, "яйца" | "яблока" | "молока" | "масла")
}

pub fn accusative_phrase(name: &str) -> String {
    name.split_whitespace().map(|w| accusative_word(w)).collect::<Vec<_>>().join(" ")
}

pub fn instrumental_phrase(name: &str) -> String {
    name.split_whitespace().map(|w| instrumental_word(w)).collect::<Vec<_>>().join(" ")
}

pub fn instrumental_case(name: &str) -> String {
    instrumental_phrase(name)
}

// ── Ukrainian Grammar ────────────────────────────────────────────────────────

pub fn instrumental_case_uk(name: &str) -> String {
    let lower = name.to_lowercase();
    if lower.ends_with('а') { return format!("{}ою", &lower[..lower.len() - 'а'.len_utf8()]); }
    if lower.ends_with('я') { return format!("{}ею", &lower[..lower.len() - 'я'.len_utf8()]); }
    if lower.ends_with('ь') { return format!("{}ю", &lower[..lower.len() - 'ь'.len_utf8()]); }
    format!("{}ом", lower)
}

pub fn accusative_word_uk(word: &str) -> String {
    let lower = word.to_lowercase();
    if lower.ends_with('а') { return format!("{}у", &lower[..lower.len() - 'а'.len_utf8()]); }
    if lower.ends_with('я') { return format!("{}ю", &lower[..lower.len() - 'я'.len_utf8()]); }
    lower
}

pub fn accusative_phrase_uk(name: &str) -> String {
    name.split_whitespace().map(|w| accusative_word_uk(w)).collect::<Vec<_>>().join(" ")
}

pub fn instrumental_phrase_uk(name: &str) -> String {
    instrumental_case_uk(name)
}

// ── Polish Grammar ───────────────────────────────────────────────────────────

pub fn instrumental_case_pl(name: &str) -> String {
    let lower = name.to_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();

    if words.len() >= 3 && words.contains(&"z") {
        let z_pos = words.iter().position(|w| *w == "z").unwrap();
        if z_pos + 1 < words.len() {
            let gen_noun = words[z_pos + 1];
            return genitive_to_instrumental_pl(gen_noun);
        }
    }

    if words.len() > 1 {
        let first = instrumental_word_pl(words[0]);
        return format!("{} {}", first, words[1..].join(" "));
    }

    instrumental_word_pl(&lower)
}

fn genitive_to_instrumental_pl(gen: &str) -> String {
    let w = gen.to_lowercase();
    if w.ends_with('a') {
        let stem = &w[..w.len() - 'a'.len_utf8()];
        return instrumental_word_pl(stem);
    }
    if w.ends_with('y') {
        let stem = &w[..w.len() - 'y'.len_utf8()];
        return format!("{}ą", stem);
    }
    if w.ends_with('i') {
        let stem = &w[..w.len() - 'i'.len_utf8()];
        return format!("{}ią", stem);
    }
    instrumental_word_pl(&w)
}

pub fn instrumental_word_pl(word: &str) -> String {
    let w = word.to_lowercase();
    if w == "kurczak" { return "kurczakiem".into(); }
    if w == "łosoś"  { return "łososiem".into(); }
    if w == "dorsz"   { return "dorszem".into(); }
    if w.ends_with('a') { return format!("{}ą", &w[..w.len() - 'a'.len_utf8()]); }
    if w.ends_with("ś") { return format!("{}ią", w); }
    if w.ends_with('o') { return format!("{}em", &w[..w.len() - 'o'.len_utf8()]); }
    if w.ends_with("ek") { return format!("{}kiem", &w[..w.len() - "ek".len()]); }
    if w.ends_with("ak") { return format!("{}iem", w); }
    format!("{}em", w)
}

pub fn accusative_word_pl(word: &str) -> String {
    let w = word.to_lowercase();
    if w.ends_with('a') { return format!("{}ę", &w[..w.len() - 'a'.len_utf8()]); }
    w
}

pub fn accusative_phrase_pl(name: &str) -> String {
    let lower = name.to_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();

    if words.len() == 1 {
        return accusative_word_pl(words[0]);
    }

    if words.len() >= 3 && words[1] == "z" {
        let first = accusative_word_pl(words[0]);
        return format!("{} {}", first, words[1..].join(" "));
    }

    words.iter().map(|w| accusative_word_pl(w)).collect::<Vec<_>>().join(" ")
}

pub fn instrumental_phrase_pl(name: &str) -> String {
    instrumental_case_pl(name)
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ru_gender_feminine_soft_sign() {
        assert_eq!(ru_gender("Морковь"), 'f');
        assert_eq!(ru_gender("Фасоль"), 'f');
        assert_eq!(ru_gender("Форель"), 'f');
        assert_eq!(ru_gender("Печень"), 'f');
        assert_eq!(ru_gender("Картофель"), 'm');
        assert_eq!(ru_gender("Имбирь"), 'm');
    }

    #[test]
    fn state_label_morkov_feminine() {
        let label = state_label_ru("sauteed", "Морковь");
        assert_eq!(label, "пассерованная");
    }

    #[test]
    fn state_label_kartoshka_feminine() {
        let label = state_label_ru("boiled", "Картошка");
        assert_eq!(label, "варёная");
    }

    #[test]
    fn accusative_simple_nouns() {
        assert_eq!(accusative_word("Говядина"), "говядину");
        assert_eq!(accusative_word("Свинина"), "свинину");
        assert_eq!(accusative_word("Мука"), "муку");
        assert_eq!(accusative_word("Морковь"), "морковь");
        assert_eq!(accusative_word("Лук"), "лук");
        assert_eq!(accusative_word("Чеснок"), "чеснок");
    }

    #[test]
    fn accusative_adjective_aya() {
        assert_eq!(accusative_word("Пшеничная"), "пшеничную");
        assert_eq!(accusative_word("Каменная"), "каменную");
    }

    #[test]
    fn accusative_compound_names() {
        assert_eq!(accusative_phrase("Пшеничная мука"), "пшеничную муку");
        assert_eq!(accusative_phrase("Соль каменная"), "соль каменную");
        assert_eq!(accusative_phrase("Куриные яйца"), "куриные яйца");
        assert_eq!(accusative_phrase("Чёрный перец"), "чёрный перец");
        assert_eq!(accusative_phrase("Говядина"), "говядину");
    }

    #[test]
    fn instrumental_simple_nouns() {
        assert_eq!(instrumental_word("Говядина"), "говядиной");
        assert_eq!(instrumental_word("Майонез"), "майонезом");
        assert_eq!(instrumental_word("Масло"), "маслом");
        assert_eq!(instrumental_word("Морковь"), "морковью");
        assert_eq!(instrumental_word("Перец"), "перцем");
    }

    #[test]
    fn instrumental_adjectives() {
        assert_eq!(instrumental_word("Подсолнечное"), "подсолнечным");
        assert_eq!(instrumental_word("Куриные"), "куриными");
        assert_eq!(instrumental_word("Чёрный"), "чёрным");
        assert_eq!(instrumental_word("Пшеничная"), "пшеничной");
    }

    #[test]
    fn instrumental_compound_names() {
        assert_eq!(instrumental_phrase("Подсолнечное масло"), "подсолнечным маслом");
        assert_eq!(instrumental_phrase("Майонез"), "майонезом");
        assert_eq!(instrumental_phrase("Говядина"), "говядиной");
        assert_eq!(instrumental_phrase("Чёрный перец"), "чёрным перцем");
        assert_eq!(instrumental_phrase("Куриные яйца"), "куриными яйцами");
    }
}
