//! `CatalogNutritionRow` — sqlx FromRow для `catalog_ingredients`.
//! Shared между всеми handlers в `interfaces/http/public/tools/`.

use crate::shared::Language;

pub fn dec_f64(d: Option<rust_decimal::Decimal>) -> f64 {
    d.and_then(|v| rust_decimal::prelude::ToPrimitive::to_f64(&v)).unwrap_or(0.0)
}

/// Full nutrition + metadata row from catalog_ingredients.
#[derive(sqlx::FromRow, Clone)]
pub struct CatalogNutritionRow {
    pub name_en: String,
    pub name_ru: String,
    pub name_pl: String,
    pub name_uk: String,
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
    pub fn cal(&self)   -> f64 { self.calories_per_100g.unwrap_or(0) as f64 }
    pub fn prot(&self)  -> f64 { dec_f64(self.protein_per_100g) }
    pub fn fat(&self)   -> f64 { dec_f64(self.fat_per_100g) }
    pub fn carbs(&self) -> f64 { dec_f64(self.carbs_per_100g) }
    pub fn fiber(&self) -> f64 { dec_f64(self.fiber_per_100g) }
    pub fn sugar(&self) -> f64 { dec_f64(self.sugar_per_100g) }
    pub fn salt(&self)  -> f64 { dec_f64(self.salt_per_100g) }

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
}

/// Reusable SELECT columns string for all queries using CatalogNutritionRow
pub const CATALOG_NUTRITION_COLS: &str = r#"
    name_en, name_ru, name_pl, name_uk,
    image_url, slug, product_type,
    calories_per_100g,
    protein_per_100g, fat_per_100g, carbs_per_100g,
    fiber_per_100g, sugar_per_100g, salt_per_100g,
    density_g_per_ml, typical_portion_g,
    water_type, wild_farmed, sushi_grade
"#;
