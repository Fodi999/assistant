//! Shared helpers for all public tool handlers.
//! Language parsing, unit labels, guard functions.

use crate::shared::Language;

// ── Language ──────────────────────────────────────────────────────────────────

pub fn parse_lang(lang: &Option<String>) -> Language {
    lang.as_deref()
        .and_then(Language::from_code)
        .unwrap_or_default()
}

// ── Unit labels ───────────────────────────────────────────────────────────────
//
// Each unit has 4 forms:
//   nom  — nominative / base form    ("gram", "szklanka", "грамм")
//   gen  — genitive  (PL: "gramów", RU: "граммов", UK: "грамів")
//   in_  — "in one X" phrase         (PL: "w jednej szklance", RU: "в одном стакане")

pub struct UnitLabel {
    pub en:  &'static str,   // nominative EN
    pub pl:  &'static str,   // nominative PL
    pub ru:  &'static str,   // nominative RU
    pub uk:  &'static str,   // nominative UK
    // genitive (how many ___ = x gramów / граммов / грамів / grams)
    pub en_gen: &'static str,
    pub pl_gen: &'static str,
    pub ru_gen: &'static str,
    pub uk_gen: &'static str,
    // "in one ___" locative phrase for question
    pub en_in: &'static str,
    pub pl_in: &'static str,
    pub ru_in: &'static str,
    pub uk_in: &'static str,
}

impl UnitLabel {
    pub fn for_lang(&self, lang: Language) -> &'static str {
        match lang {
            Language::En => self.en,
            Language::Pl => self.pl,
            Language::Ru => self.ru,
            Language::Uk => self.uk,
        }
    }
    pub fn genitive(&self, lang: Language) -> &'static str {
        match lang {
            Language::En => self.en_gen,
            Language::Pl => self.pl_gen,
            Language::Ru => self.ru_gen,
            Language::Uk => self.uk_gen,
        }
    }
    /// "in one <unit>" phrase, e.g. "w jednej szklance", "в одном стакане"
    pub fn in_one(&self, lang: Language) -> &'static str {
        match lang {
            Language::En => self.en_in,
            Language::Pl => self.pl_in,
            Language::Ru => self.ru_in,
            Language::Uk => self.uk_in,
        }
    }
}

macro_rules! ul {
    ($en:expr, $pl:expr, $ru:expr, $uk:expr,
     $en_g:expr, $pl_g:expr, $ru_g:expr, $uk_g:expr,
     $en_i:expr, $pl_i:expr, $ru_i:expr, $uk_i:expr) => {
        UnitLabel {
            en: $en, pl: $pl, ru: $ru, uk: $uk,
            en_gen: $en_g, pl_gen: $pl_g, ru_gen: $ru_g, uk_gen: $uk_g,
            en_in:  $en_i, pl_in:  $pl_i, ru_in:  $ru_i, uk_in:  $uk_i,
        }
    };
}

pub static UNIT_LABELS: &[(&str, UnitLabel)] = &[
    //           nom-EN          nom-PL          nom-RU              nom-UK
    //           gen-EN          gen-PL          gen-RU              gen-UK
    //           in-EN           in-PL           in-RU               in-UK
    ("g", ul!(
        "gram",         "gram",         "грамм",            "грам",
        "grams",        "gramów",       "граммов",          "грамів",
        "in 1 gram",    "w 1 gramie",   "в 1 грамме",       "в 1 грамі"
    )),
    ("mg", ul!(
        "milligram",    "miligram",     "миллиграмм",       "міліграм",
        "milligrams",   "miligramów",   "миллиграммов",     "міліграмів",
        "in 1 mg",      "w 1 mg",       "в 1 мг",           "в 1 мг"
    )),
    ("kg", ul!(
        "kilogram",     "kilogram",     "килограмм",        "кілограм",
        "kilograms",    "kilogramów",   "килограммов",      "кілограмів",
        "in 1 kg",      "w 1 kg",       "в 1 кг",           "в 1 кг"
    )),
    ("oz", ul!(
        "ounce",        "uncja",        "унция",            "унція",
        "ounces",       "uncji",        "унций",            "унцій",
        "in 1 oz",      "w 1 uncji",    "в 1 унции",        "в 1 унції"
    )),
    ("lb", ul!(
        "pound",        "funt",         "фунт",             "фунт",
        "pounds",       "funtów",       "фунтов",           "фунтів",
        "in 1 lb",      "w 1 funcie",   "в 1 фунте",        "в 1 фунті"
    )),
    ("ml", ul!(
        "milliliter",   "mililitr",     "миллилитр",        "мілілітр",
        "milliliters",  "mililitrów",   "миллилитров",      "мілілітрів",
        "in 1 ml",      "w 1 ml",       "в 1 мл",           "в 1 мл"
    )),
    ("l", ul!(
        "liter",        "litr",         "литр",             "літр",
        "liters",       "litrów",       "литров",           "літрів",
        "in 1 liter",   "w 1 litrze",   "в 1 литре",        "в 1 літрі"
    )),
    ("fl_oz", ul!(
        "fl. ounce",    "fl. uncja",    "жидк. унция",      "рід. унція",
        "fl. ounces",   "fl. uncji",    "жидк. унций",      "рід. унцій",
        "in 1 fl oz",   "w 1 fl oz",    "в 1 жидк. унции",  "в 1 рід. унції"
    )),
    ("tsp", ul!(
        "teaspoon",     "łyżeczka",     "чайная ложка",     "чайна ложка",
        "teaspoons",    "łyżeczek",     "чайных ложек",     "чайних ложок",
        "in 1 tsp",     "w 1 łyżeczce", "в 1 ч. ложке",     "в 1 ч. ложці"
    )),
    ("tbsp", ul!(
        "tablespoon",   "łyżka",        "столовая ложка",   "столова ложка",
        "tablespoons",  "łyżek",        "столовых ложек",   "столових ложок",
        "in 1 tbsp",    "w 1 łyżce",    "в 1 ст. ложке",    "в 1 ст. ложці"
    )),
    ("cup", ul!(
        "cup",          "szklanka",     "стакан",           "склянка",
        "cups",         "szklanek",     "стаканов",         "склянок",
        "in 1 cup",     "w 1 szklance", "в 1 стакане",      "в 1 склянці"
    )),
    ("pint", ul!(
        "pint",         "pinta",        "пинта",            "пінта",
        "pints",        "pint",         "пинт",             "пінт",
        "in 1 pint",    "w 1 pincie",   "в 1 пинте",        "в 1 пінті"
    )),
    ("quart", ul!(
        "quart",        "kwarta",       "кварта",           "кварта",
        "quarts",       "kwart",        "кварт",            "кварт",
        "in 1 quart",   "w 1 kwarcie",  "в 1 кварте",       "в 1 кварті"
    )),
    ("gallon", ul!(
        "gallon",       "galon",        "галлон",           "галон",
        "gallons",      "galonów",      "галлонов",         "галонів",
        "in 1 gallon",  "w 1 galonie",  "в 1 галлоне",      "в 1 галоні"
    )),
    ("dash", ul!(
        "dash",         "odrobina",     "щепотка",          "дрібка",
        "dashes",       "odrobiny",     "щепоток",          "дрібок",
        "in 1 dash",    "w 1 odrobinie","в 1 щепотке",      "в 1 дрібці"
    )),
    ("pinch", ul!(
        "pinch",        "szczypta",     "щепотка",          "щіпка",
        "pinches",      "szczypty",     "щепоток",          "щіпок",
        "in 1 pinch",   "w 1 szczypty", "в 1 щепотке",      "в 1 щіпці"
    )),
    ("drop", ul!(
        "drop",         "kropla",       "капля",            "крапля",
        "drops",        "kropli",       "капель",           "крапель",
        "in 1 drop",    "w 1 kropli",   "в 1 капле",        "в 1 краплі"
    )),
    ("stick_butter", ul!(
        "stick butter", "kostka masła", "палочка масла",    "паличка масла",
        "sticks butter","kostek masła", "палочек масла",    "паличок масла",
        "in 1 stick",   "w 1 kostce",   "в 1 пачке",        "в 1 паличці"
    )),
];

/// Nominative label (for display, buttons, dropdowns)
pub fn label(unit: &str, lang: Language) -> String {
    UNIT_LABELS
        .iter()
        .find(|(code, _)| *code == unit)
        .map(|(_, l)| l.for_lang(lang).to_string())
        .unwrap_or_else(|| unit.to_string())
}

/// Short/abbreviated label for use in converted results ("g", "ml", "cup", "tbsp", "tsp")
/// Falls back to the unit code itself when no short form is defined.
pub fn label_short(unit: &str) -> &'static str {
    match unit {
        "g"           => "g",
        "mg"          => "mg",
        "kg"          => "kg",
        "oz"          => "oz",
        "lb"          => "lb",
        "ml"          => "ml",
        "l"           => "l",
        "fl_oz"       => "fl oz",
        "tsp"         => "tsp",
        "tbsp"        => "tbsp",
        "cup"         => "cup",
        "pint"        => "pint",
        "quart"       => "qt",
        "gallon"      => "gal",
        "dash"        => "dash",
        "pinch"       => "pinch",
        "drop"        => "drop",
        "stick_butter"=> "stick",
        other         => {
            // static leak is fine — unit codes are a closed set from URL params
            // but to avoid unsafe we just return the code as-is (it's &str not &'static str)
            // We convert to 'static by looking up in the full labels table
            UNIT_LABELS
                .iter()
                .find(|(code, _)| *code == other)
                .map(|(code, _)| *code)
                .unwrap_or("?")
        }
    }
}

/// Genitive label — "how many ___ " (gramów, граммов, grамів, grams)
pub fn label_gen(unit: &str, lang: Language) -> String {
    UNIT_LABELS
        .iter()
        .find(|(code, _)| *code == unit)
        .map(|(_, l)| l.genitive(lang).to_string())
        .unwrap_or_else(|| unit.to_string())
}

/// "in one <unit>" phrase for natural-language questions
pub fn label_in(unit: &str, lang: Language) -> String {
    UNIT_LABELS
        .iter()
        .find(|(code, _)| *code == unit)
        .map(|(_, l)| l.in_one(lang).to_string())
        .unwrap_or_else(|| format!("in 1 {unit}"))
}

// ── Smart unit result (shared by units + nutrition handlers) ──────────────────

use serde::Serialize;

#[derive(Serialize)]
pub struct SmartUnit {
    pub value: f64,
    pub unit:  String,
    pub label: String,
}

// ── Guard helpers ─────────────────────────────────────────────────────────────

pub fn sanitize_value(v: f64) -> Option<f64> {
    if v.is_nan() || v.is_infinite() {
        None
    } else {
        Some(v.clamp(-1_000_000.0, 1_000_000.0))
    }
}
