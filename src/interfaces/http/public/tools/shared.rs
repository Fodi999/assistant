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

pub struct UnitLabel {
    pub en: &'static str,
    pub pl: &'static str,
    pub ru: &'static str,
    pub uk: &'static str,
}

impl UnitLabel {
    pub fn for_lang(&self, lang: Language) -> &'static str {
        match lang {
            Language::Pl => self.pl,
            Language::Ru => self.ru,
            Language::Uk => self.uk,
            Language::En => self.en,
        }
    }
}

pub static UNIT_LABELS: &[(&str, UnitLabel)] = &[
    ("g",          UnitLabel { en: "gram",         pl: "gram",          ru: "грамм",           uk: "грам"          }),
    ("mg",         UnitLabel { en: "milligram",     pl: "miligram",      ru: "миллиграмм",      uk: "міліграм"      }),
    ("kg",         UnitLabel { en: "kilogram",      pl: "kilogram",      ru: "килограмм",       uk: "кілограм"      }),
    ("oz",         UnitLabel { en: "ounce",         pl: "uncja",         ru: "унция",           uk: "унція"         }),
    ("lb",         UnitLabel { en: "pound",         pl: "funt",          ru: "фунт",            uk: "фунт"          }),
    ("ml",         UnitLabel { en: "milliliter",    pl: "mililitr",      ru: "миллилитр",       uk: "мілілітр"      }),
    ("l",          UnitLabel { en: "liter",         pl: "litr",          ru: "литр",            uk: "літр"          }),
    ("fl_oz",      UnitLabel { en: "fl. ounce",     pl: "fl. uncja",     ru: "жидк. унция",     uk: "рід. унція"    }),
    ("tsp",        UnitLabel { en: "teaspoon",      pl: "łyżeczka",      ru: "чайная ложка",    uk: "чайна ложка"   }),
    ("tbsp",       UnitLabel { en: "tablespoon",    pl: "łyżka",         ru: "столовая ложка",  uk: "столова ложка" }),
    ("cup",        UnitLabel { en: "cup",           pl: "szklanka",      ru: "стакан",          uk: "склянка"       }),
    ("pint",       UnitLabel { en: "pint",          pl: "pinta",         ru: "пинта",           uk: "пінта"         }),
    ("quart",      UnitLabel { en: "quart",         pl: "kwarta",        ru: "кварта",          uk: "кварта"        }),
    ("gallon",     UnitLabel { en: "gallon",        pl: "galon",         ru: "галлон",          uk: "галон"         }),
    ("dash",       UnitLabel { en: "dash",          pl: "odrobina",      ru: "щепотка",         uk: "дрібка"        }),
    ("pinch",      UnitLabel { en: "pinch",         pl: "szczypta",      ru: "щепотка",         uk: "щіпка"         }),
    ("drop",       UnitLabel { en: "drop",          pl: "kropla",        ru: "капля",           uk: "крапля"        }),
    ("stick_butter", UnitLabel { en: "stick butter", pl: "kostka masła", ru: "палочка масла",   uk: "паличка масла" }),
];

pub fn label(unit: &str, lang: Language) -> String {
    UNIT_LABELS
        .iter()
        .find(|(code, _)| *code == unit)
        .map(|(_, l)| l.for_lang(lang).to_string())
        .unwrap_or_else(|| unit.to_string())
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
