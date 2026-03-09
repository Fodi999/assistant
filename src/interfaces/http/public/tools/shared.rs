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
// Each unit has 4 inflection groups:
//   nom  — nominative / base form    ("gram", "szklanka", "грамм")
//   gen  — genitive                  ("gramów", "граммов", "грамів")
//   in_  — "in one X" full phrase    ("w 1 szklance", "в 1 стакане")
//   loc  — pure locative (no number) ("szklance", "стакане", "склянці")

pub struct UnitLabel {
    pub en:  &'static str,
    pub pl:  &'static str,
    pub ru:  &'static str,
    pub uk:  &'static str,
    pub en_gen: &'static str,
    pub pl_gen: &'static str,
    pub ru_gen: &'static str,
    pub uk_gen: &'static str,
    pub en_in: &'static str,
    pub pl_in: &'static str,
    pub ru_in: &'static str,
    pub uk_in: &'static str,
    pub en_loc: &'static str,
    pub pl_loc: &'static str,
    pub ru_loc: &'static str,
    pub uk_loc: &'static str,
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
    pub fn in_one(&self, lang: Language) -> &'static str {
        match lang {
            Language::En => self.en_in,
            Language::Pl => self.pl_in,
            Language::Ru => self.ru_in,
            Language::Uk => self.uk_in,
        }
    }
    /// Pure locative form — no number prefix.
    /// e.g. cup->"cup", cup->"szklance", cup->"стакане", cup->"склянці"
    pub fn locative(&self, lang: Language) -> &'static str {
        match lang {
            Language::En => self.en_loc,
            Language::Pl => self.pl_loc,
            Language::Ru => self.ru_loc,
            Language::Uk => self.uk_loc,
        }
    }
}

macro_rules! ul {
    ($en:expr, $pl:expr, $ru:expr, $uk:expr,
     $en_g:expr, $pl_g:expr, $ru_g:expr, $uk_g:expr,
     $en_i:expr, $pl_i:expr, $ru_i:expr, $uk_i:expr,
     $en_l:expr, $pl_l:expr, $ru_l:expr, $uk_l:expr) => {
        UnitLabel {
            en: $en, pl: $pl, ru: $ru, uk: $uk,
            en_gen: $en_g, pl_gen: $pl_g, ru_gen: $ru_g, uk_gen: $uk_g,
            en_in:  $en_i, pl_in:  $pl_i, ru_in:  $ru_i, uk_in:  $uk_i,
            en_loc: $en_l, pl_loc: $pl_l, ru_loc: $ru_l, uk_loc: $uk_l,
        }
    };
}

pub static UNIT_LABELS: &[(&str, UnitLabel)] = &[
    //        nom-EN        nom-PL          nom-RU            nom-UK
    //        gen-EN        gen-PL          gen-RU            gen-UK
    //        in-EN         in-PL           in-RU             in-UK
    //        loc-EN        loc-PL          loc-RU            loc-UK
    ("g", ul!(
        "gram",       "gram",         "грамм",          "грам",
        "grams",      "gramów",       "граммов",        "грамів",
        "in 1 gram",  "w 1 gramie",   "в 1 грамме",     "в 1 грамі",
        "gram",       "gramie",       "грамме",         "грамі"
    )),
    ("mg", ul!(
        "milligram",  "miligram",     "миллиграмм",     "міліграм",
        "milligrams", "miligramów",   "миллиграммов",   "міліграмів",
        "in 1 mg",    "w 1 mg",       "в 1 мг",         "в 1 мг",
        "mg",         "mg",           "мг",             "мг"
    )),
    ("kg", ul!(
        "kilogram",   "kilogram",     "килограмм",      "кілограм",
        "kilograms",  "kilogramów",   "килограммов",    "кілограмів",
        "in 1 kg",    "w 1 kg",       "в 1 кг",         "в 1 кг",
        "kg",         "kg",           "кг",             "кг"
    )),
    ("oz", ul!(
        "ounce",      "uncja",        "унция",          "унція",
        "ounces",     "uncji",        "унций",          "унцій",
        "in 1 oz",    "w 1 uncji",    "в 1 унции",      "в 1 унції",
        "oz",         "uncji",        "унции",          "унції"
    )),
    ("lb", ul!(
        "pound",      "funt",         "фунт",           "фунт",
        "pounds",     "funtów",       "фунтов",         "фунтів",
        "in 1 lb",    "w 1 funcie",   "в 1 фунте",      "в 1 фунті",
        "lb",         "funcie",       "фунте",          "фунті"
    )),
    ("ml", ul!(
        "milliliter", "mililitr",     "миллилитр",      "мілілітр",
        "milliliters","mililitrów",   "миллилитров",    "мілілітрів",
        "in 1 ml",    "w 1 ml",       "в 1 мл",         "в 1 мл",
        "ml",         "ml",           "мл",             "мл"
    )),
    ("l", ul!(
        "liter",      "litr",         "литр",           "літр",
        "liters",     "litrów",       "литров",         "літрів",
        "in 1 liter", "w 1 litrze",   "в 1 литре",      "в 1 літрі",
        "liter",      "litrze",       "литре",          "літрі"
    )),
    ("fl_oz", ul!(
        "fl. ounce",  "fl. uncja",    "жидк. унция",    "рід. унція",
        "fl. ounces", "fl. uncji",    "жидк. унций",    "рід. унцій",
        "in 1 fl oz", "w 1 fl oz",    "в 1 жидк. унции","в 1 рід. унції",
        "fl oz",      "fl oz",        "жидк. унции",    "рід. унції"
    )),
    ("tsp", ul!(
        "teaspoon",   "łyżeczka",     "чайная ложка",   "чайна ложка",
        "teaspoons",  "łyżeczek",     "чайных ложек",   "чайних ложок",
        "in 1 tsp",   "w 1 łyżeczce", "в 1 ч. ложке",   "в 1 ч. ложці",
        "teaspoon",   "łyżeczce",     "чайной ложке",   "чайній ложці"
    )),
    ("tbsp", ul!(
        "tablespoon", "łyżka",        "столовая ложка", "столова ложка",
        "tablespoons","łyżek",        "столовых ложек", "столових ложок",
        "in 1 tbsp",  "w 1 łyżce",    "в 1 ст. ложке",  "в 1 ст. ложці",
        "tablespoon", "łyżce",        "столовой ложке", "столовій ложці"
    )),
    ("cup", ul!(
        "cup",        "szklanka",     "стакан",         "склянка",
        "cups",       "szklanek",     "стаканов",       "склянок",
        "in 1 cup",   "w 1 szklance", "в 1 стакане",    "в 1 склянці",
        "cup",        "szklance",     "стакане",        "склянці"
    )),
    ("pint", ul!(
        "pint",       "pinta",        "пинта",          "пінта",
        "pints",      "pint",         "пинт",           "пінт",
        "in 1 pint",  "w 1 pincie",   "в 1 пинте",      "в 1 пінті",
        "pint",       "pincie",       "пинте",          "пінті"
    )),
    ("quart", ul!(
        "quart",      "kwarta",       "кварта",         "кварта",
        "quarts",     "kwart",        "кварт",          "кварт",
        "in 1 quart", "w 1 kwarcie",  "в 1 кварте",     "в 1 кварті",
        "quart",      "kwarcie",      "кварте",         "кварті"
    )),
    ("gallon", ul!(
        "gallon",     "galon",        "галлон",         "галон",
        "gallons",    "galonów",      "галлонов",       "галонів",
        "in 1 gallon","w 1 galonie",  "в 1 галлоне",    "в 1 галоні",
        "gallon",     "galonie",      "галлоне",        "галоні"
    )),
    ("dash", ul!(
        "dash",       "odrobina",     "щепотка",        "дрібка",
        "dashes",     "odrobiny",     "щепоток",        "дрібок",
        "in 1 dash",  "w 1 odrobinie","в 1 щепотке",    "в 1 дрібці",
        "dash",       "odrobinie",    "щепотке",        "дрібці"
    )),
    ("pinch", ul!(
        "pinch",      "szczypta",     "щепотка",        "щіпка",
        "pinches",    "szczypty",     "щепоток",        "щіпок",
        "in 1 pinch", "w 1 szczypty", "в 1 щепотке",    "в 1 щіпці",
        "pinch",      "szczypty",     "щепотке",        "щіпці"
    )),
    ("drop", ul!(
        "drop",       "kropla",       "капля",          "крапля",
        "drops",      "kropli",       "капель",         "крапель",
        "in 1 drop",  "w 1 kropli",   "в 1 капле",      "в 1 краплі",
        "drop",       "kropli",       "капле",          "краплі"
    )),
    ("stick_butter", ul!(
        "stick butter","kostka masła","палочка масла",  "паличка масла",
        "sticks butter","kostek masła","палочек масла", "паличок масла",
        "in 1 stick", "w 1 kostce",   "в 1 пачке",      "в 1 паличці",
        "stick",      "kostce",       "пачке",          "паличці"
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

/// Short/abbreviated label for converted results ("g", "ml", "cup", "tbsp", "tsp")
pub fn label_short(unit: &str) -> &'static str {
    match unit {
        "g"            => "g",
        "mg"           => "mg",
        "kg"           => "kg",
        "oz"           => "oz",
        "lb"           => "lb",
        "ml"           => "ml",
        "l"            => "l",
        "fl_oz"        => "fl oz",
        "tsp"          => "tsp",
        "tbsp"         => "tbsp",
        "cup"          => "cup",
        "pint"         => "pint",
        "quart"        => "qt",
        "gallon"       => "gal",
        "dash"         => "dash",
        "pinch"        => "pinch",
        "drop"         => "drop",
        "stick_butter" => "stick",
        other          => UNIT_LABELS
            .iter()
            .find(|(code, _)| *code == other)
            .map(|(code, _)| *code)
            .unwrap_or("?"),
    }
}

/// Genitive label — "how many ___" (gramów, граммов, грамів, grams)
pub fn label_gen(unit: &str, lang: Language) -> String {
    UNIT_LABELS
        .iter()
        .find(|(code, _)| *code == unit)
        .map(|(_, l)| l.genitive(lang).to_string())
        .unwrap_or_else(|| unit.to_string())
}

/// Pure locative form — no number prefix.
/// cup → "cup" / "szklance" / "стакане" / "склянці"
pub fn label_loc(unit: &str, lang: Language) -> String {
    UNIT_LABELS
        .iter()
        .find(|(code, _)| *code == unit)
        .map(|(_, l)| l.locative(lang).to_string())
        .unwrap_or_else(|| unit.to_string())
}

/// "in one <unit>" full phrase for natural-language questions
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
