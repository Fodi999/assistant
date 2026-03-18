//! `CatalogNutritionRow` — sqlx FromRow для `catalog_ingredients`.
//! Shared между всеми handlers в `interfaces/http/public/tools/`.

use crate::shared::Language;

pub fn dec_f64(d: Option<rust_decimal::Decimal>) -> f64 {
    d.and_then(|v| rust_decimal::prelude::ToPrimitive::to_f64(&v)).unwrap_or(0.0)
}

/// Convert Option<Decimal> to Option<f64> — preserves None (null).
pub fn dec_f64_opt(d: Option<rust_decimal::Decimal>) -> Option<f64> {
    d.and_then(|v| rust_decimal::prelude::ToPrimitive::to_f64(&v))
}

/// Full nutrition + metadata row from catalog_ingredients.
#[derive(sqlx::FromRow, Clone)]
pub struct CatalogNutritionRow {
    pub name_en: String,
    pub name_ru: String,
    pub name_pl: String,
    pub name_uk: String,
    pub name_en_gen: Option<String>,
    pub name_ru_gen: Option<String>,
    pub name_pl_gen: Option<String>,
    pub name_uk_gen: Option<String>,

    pub name_en_loc: Option<String>,
    pub name_ru_loc: Option<String>,
    pub name_pl_loc: Option<String>,
    pub name_uk_loc: Option<String>,

    pub name_en_dat: Option<String>,
    pub name_ru_dat: Option<String>,
    pub name_pl_dat: Option<String>,
    pub name_uk_dat: Option<String>,

    pub image_url:         Option<String>,
    pub slug:              Option<String>,
    pub product_type:      Option<String>,
    pub calories_per_100g: Option<i32>,
    pub protein_per_100g:  Option<rust_decimal::Decimal>,
    pub fat_per_100g:      Option<rust_decimal::Decimal>,
    pub carbs_per_100g:    Option<rust_decimal::Decimal>,
    pub fiber_per_100g:    Option<rust_decimal::Decimal>,
    pub sugar_per_100g:    Option<rust_decimal::Decimal>,
    pub salt_per_100g:     Option<rust_decimal::Decimal>,
    pub density_g_per_ml:  Option<rust_decimal::Decimal>,
    pub typical_portion_g: Option<rust_decimal::Decimal>,
    pub water_type:        Option<String>,
    pub wild_farmed:       Option<String>,
    pub sushi_grade:       Option<bool>,
}

impl CatalogNutritionRow {
    // ── f64 helpers (for calculations — NULL → 0.0) ──
    pub fn cal(&self)   -> f64 { self.calories_per_100g.unwrap_or(0) as f64 }
    pub fn prot(&self)  -> f64 { dec_f64(self.protein_per_100g) }
    pub fn fat(&self)   -> f64 { dec_f64(self.fat_per_100g) }
    pub fn carbs(&self) -> f64 { dec_f64(self.carbs_per_100g) }
    pub fn fiber(&self) -> f64 { dec_f64(self.fiber_per_100g) }
    pub fn sugar(&self) -> f64 { dec_f64(self.sugar_per_100g) }
    pub fn salt(&self)  -> f64 { dec_f64(self.salt_per_100g) }

    // ── Option<f64> helpers (for API responses — NULL → null, NOT 0) ──
    pub fn cal_opt(&self)   -> Option<f64> { self.calories_per_100g.map(|v| v as f64) }
    pub fn prot_opt(&self)  -> Option<f64> { dec_f64_opt(self.protein_per_100g) }
    pub fn fat_opt(&self)   -> Option<f64> { dec_f64_opt(self.fat_per_100g) }
    pub fn carbs_opt(&self) -> Option<f64> { dec_f64_opt(self.carbs_per_100g) }
    pub fn fiber_opt(&self) -> Option<f64> { dec_f64_opt(self.fiber_per_100g) }
    pub fn sugar_opt(&self) -> Option<f64> { dec_f64_opt(self.sugar_per_100g) }
    pub fn salt_opt(&self)  -> Option<f64> { dec_f64_opt(self.salt_per_100g) }

    /// Returns true if the product has ANY nutrition data filled
    pub fn has_nutrition(&self) -> bool {
        self.calories_per_100g.is_some()
            || self.protein_per_100g.is_some()
            || self.fat_per_100g.is_some()
            || self.carbs_per_100g.is_some()
    }

    pub fn density(&self) -> f64 {
        self.density_g_per_ml
            .and_then(|d| rust_decimal::prelude::ToPrimitive::to_f64(&d))
            .unwrap_or(1.0)
    }

    pub fn typical_g(&self) -> Option<f64> {
        self.typical_portion_g
            .and_then(|d| rust_decimal::prelude::ToPrimitive::to_f64(&d))
    }

    pub fn localized_name(&self, lang: Language) -> &str {
        match lang {
            Language::Ru => &self.name_ru,
            Language::Pl => &self.name_pl,
            Language::Uk => &self.name_uk,
            Language::En => &self.name_en,
        }
    }

    pub fn localized_genitive(&self, lang: Language) -> &str {
        let gen = match lang {
            Language::Ru => self.name_ru_gen.as_deref(),
            Language::Pl => self.name_pl_gen.as_deref(),
            Language::Uk => self.name_uk_gen.as_deref(),
            Language::En => self.name_en_gen.as_deref(),
        };
        gen.unwrap_or_else(|| self.localized_name(lang))
    }

    pub fn localized_locative(&self, lang: Language) -> &str {
        let loc = match lang {
            Language::Ru => self.name_ru_loc.as_deref(),
            Language::Pl => self.name_pl_loc.as_deref(),
            Language::Uk => self.name_uk_loc.as_deref(),
            Language::En => self.name_en_loc.as_deref(),
        };
        loc.unwrap_or_else(|| self.localized_name(lang))
    }

    pub fn localized_dative(&self, lang: Language) -> &str {
        let dat = match lang {
            Language::Ru => self.name_ru_dat.as_deref(),
            Language::Pl => self.name_pl_dat.as_deref(),
            Language::Uk => self.name_uk_dat.as_deref(),
            Language::En => self.name_en_dat.as_deref(),
        };
        dat.unwrap_or_else(|| self.localized_name(lang))
    }
}

/// Reusable SELECT columns string for all queries using CatalogNutritionRow
pub const CATALOG_NUTRITION_COLS: &str = r#"
    name_en, name_ru, name_pl, name_uk,
    name_en_gen, name_ru_gen, name_pl_gen, name_uk_gen,
    name_en_loc, name_ru_loc, name_pl_loc, name_uk_loc,
    name_en_dat, name_ru_dat, name_pl_dat, name_uk_dat,
    image_url, slug, product_type,
    calories_per_100g,
    protein_per_100g, fat_per_100g, carbs_per_100g,
    fiber_per_100g, sugar_per_100g, salt_per_100g,
    density_g_per_ml, typical_portion_g,
    water_type, wild_farmed, sushi_grade
"#;
