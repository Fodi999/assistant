//! Domain types and pure logic for nutrition calculations.
//! No I/O, no DB, no HTTP — only data structures and pure functions.

use crate::domain::tools::unit_converter as uc;
use serde::{Deserialize, Serialize};

// ── Core types ────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NutritionBreakdown {
    pub calories:  f64,
    pub protein_g: f64,
    pub fat_g:     f64,
    pub carbs_g:   f64,
    pub fiber_g:   f64,
    pub sugar_g:   f64,
    pub salt_g:    f64,
    /// Sodium derived from salt: sodium_mg = salt_g × 393
    pub sodium_mg: f64,
}

impl NutritionBreakdown {
    pub fn zero() -> Self {
        Self { calories: 0.0, protein_g: 0.0, fat_g: 0.0, carbs_g: 0.0,
               fiber_g: 0.0, sugar_g: 0.0, salt_g: 0.0, sodium_mg: 0.0 }
    }

    /// Scale all values by a factor (amount_g / 100.0)
    pub fn scale(&self, factor: f64) -> Self {
        let r = |x: f64| uc::round_to(x * factor, 1);
        Self {
            calories:  r(self.calories),
            protein_g: r(self.protein_g),
            fat_g:     r(self.fat_g),
            carbs_g:   r(self.carbs_g),
            fiber_g:   r(self.fiber_g),
            sugar_g:   r(self.sugar_g),
            salt_g:    uc::round_to(self.salt_g * factor, 2),
            sodium_mg: uc::round_to(self.sodium_mg * factor, 1),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MacrosRatio {
    pub protein_pct: f64,
    pub fat_pct:     f64,
    pub carbs_pct:   f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VitaminData {
    /// µg per 100g
    pub vitamin_b12_mcg: Option<f64>,
    /// µg per 100g (IU × 0.025)
    pub vitamin_d_mcg:   Option<f64>,
    /// mg per 100g
    pub iron_mg:         Option<f64>,
    /// mg per 100g
    pub magnesium_mg:    Option<f64>,
}

impl VitaminData {
    pub fn unknown() -> Self {
        Self { vitamin_b12_mcg: None, vitamin_d_mcg: None, iron_mg: None, magnesium_mg: None }
    }
}

// ── Pure calculation functions ────────────────────────────────────────────────

/// Nutrition score (0–100).
/// Protein and fiber add points; sugar, sat-fat-proxy (fat×0.4), salt subtract.
/// Designed: lean fish ~85-95, white meat ~75-85, grains ~40-60, sweets ~10-30.
pub fn nutrition_score(
    cal: f64, prot: f64, fat: f64, _carbs: f64,
    fiber: f64, sugar: f64, salt: f64,
) -> u8 {
    if cal == 0.0 { return 0; }
    let prot_ratio  = (prot * 4.0) / cal.max(1.0);
    let fiber_bonus = (fiber * 2.5).min(15.0);
    let sugar_pen   = (sugar * 0.5).min(20.0);
    let sat_pen     = (fat * 0.4 * 0.3).min(10.0);
    let salt_pen    = (salt * 10.0).min(15.0);
    let raw = prot_ratio * 80.0 + fiber_bonus - sugar_pen - sat_pen - salt_pen;
    raw.clamp(0.0, 100.0).round() as u8
}

/// % of kcal from each macro
pub fn macros_ratio(prot: f64, fat: f64, carbs: f64) -> MacrosRatio {
    let p_kcal = prot  * 4.0;
    let f_kcal = fat   * 9.0;
    let c_kcal = carbs * 4.0;
    let total  = (p_kcal + f_kcal + c_kcal).max(1.0);
    let r = |x: f64| uc::round_to(x / total * 100.0, 1);
    MacrosRatio { protein_pct: r(p_kcal), fat_pct: r(f_kcal), carbs_pct: r(c_kcal) }
}

/// Build per-100g NutritionBreakdown from raw values
pub fn breakdown_per_100g(
    cal: f64, prot: f64, fat: f64, carbs: f64,
    fiber: f64, sugar: f64, salt: f64,
) -> NutritionBreakdown {
    let r = |x: f64| uc::round_to(x, 1);
    NutritionBreakdown {
        calories:  r(cal),
        protein_g: r(prot),
        fat_g:     r(fat),
        carbs_g:   r(carbs),
        fiber_g:   r(fiber),
        sugar_g:   r(sugar),
        salt_g:    uc::round_to(salt, 2),
        sodium_mg: uc::round_to(salt * 393.0, 1),
    }
}

// ── Nullable Nutrition Breakdown (for public API — null ≠ 0) ─────────────────

/// API-safe nutrition breakdown: null means "no data", NOT "zero".
/// Used in public API responses to avoid showing 0 for unfilled products.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NutritionBreakdownNullable {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calories:  Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protein_g: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fat_g:     Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub carbs_g:   Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fiber_g:   Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sugar_g:   Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub salt_g:    Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sodium_mg: Option<f64>,
}

impl NutritionBreakdownNullable {
    /// All null — no data available
    pub fn empty() -> Self {
        Self {
            calories: None, protein_g: None, fat_g: None, carbs_g: None,
            fiber_g: None, sugar_g: None, salt_g: None, sodium_mg: None,
        }
    }

    /// Scale all present values by factor (amount_g / 100.0)
    pub fn scale(&self, factor: f64) -> Self {
        let r = |x: Option<f64>| x.map(|v| uc::round_to(v * factor, 1));
        Self {
            calories:  r(self.calories),
            protein_g: r(self.protein_g),
            fat_g:     r(self.fat_g),
            carbs_g:   r(self.carbs_g),
            fiber_g:   r(self.fiber_g),
            sugar_g:   r(self.sugar_g),
            salt_g:    self.salt_g.map(|v| uc::round_to(v * factor, 2)),
            sodium_mg: r(self.sodium_mg),
        }
    }
}

/// Build nullable per-100g breakdown from Option<f64> values.
/// None in → None out. No silent conversion to 0.
pub fn breakdown_per_100g_nullable(
    cal: Option<f64>, prot: Option<f64>, fat: Option<f64>, carbs: Option<f64>,
    fiber: Option<f64>, sugar: Option<f64>, salt: Option<f64>,
) -> NutritionBreakdownNullable {
    let r = |x: Option<f64>| x.map(|v| uc::round_to(v, 1));
    let sodium = salt.map(|s| uc::round_to(s * 393.0, 1));
    NutritionBreakdownNullable {
        calories:  r(cal),
        protein_g: r(prot),
        fat_g:     r(fat),
        carbs_g:   r(carbs),
        fiber_g:   r(fiber),
        sugar_g:   r(sugar),
        salt_g:    salt.map(|v| uc::round_to(v, 2)),
        sodium_mg: sodium,
    }
}

// ── Vitamin static lookup — USDA averages (per 100g) ─────────────────────────

pub fn vitamins_for(slug: &str) -> VitaminData {
    let (b12, vd, fe, mg) = match slug {
        "salmon"      => (Some(3.2),  Some(11.1), Some(0.3),  Some(29.0)),
        "tuna"        => (Some(2.5),  Some(5.7),  Some(1.0),  Some(35.0)),
        "cod"         => (Some(0.9),  Some(0.9),  Some(0.4),  Some(32.0)),
        "herring"     => (Some(13.7), Some(4.2),  Some(1.1),  Some(32.0)),
        "mackerel"    => (Some(8.7),  Some(16.1), Some(1.6),  Some(60.0)),
        "trout"       => (Some(3.5),  Some(9.0),  Some(0.4),  Some(27.0)),
        "carp"        => (Some(1.5),  Some(12.5), Some(1.0),  Some(25.0)),
        "pike"        => (Some(1.6),  Some(0.5),  Some(0.7),  Some(26.0)),
        "sea-bass"    => (Some(1.1),  Some(1.0),  Some(0.3),  Some(29.0)),
        "shrimp"      => (Some(1.1),  Some(0.0),  Some(2.4),  Some(37.0)),
        "canned-tuna" => (Some(2.2),  Some(3.2),  Some(1.3),  Some(30.0)),
        "egg"         => (Some(1.1),  Some(2.0),  Some(1.8),  Some(12.0)),
        "chicken"     => (Some(0.3),  Some(0.1),  Some(1.3),  Some(25.0)),
        "beef"        => (Some(2.6),  Some(0.1),  Some(2.7),  Some(21.0)),
        "pork"        => (Some(0.7),  Some(0.6),  Some(0.9),  Some(25.0)),
        "milk"        => (Some(0.4),  Some(0.1),  Some(0.0),  Some(11.0)),
        "cheese"      => (Some(1.7),  Some(0.6),  Some(0.2),  Some(26.0)),
        "spinach"     => (Some(0.0),  Some(0.0),  Some(2.7),  Some(79.0)),
        "broccoli"    => (Some(0.0),  Some(0.0),  Some(0.7),  Some(21.0)),
        "tomato"      => (Some(0.0),  Some(0.0),  Some(0.3),  Some(11.0)),
        "potato"      => (Some(0.0),  Some(0.0),  Some(0.8),  Some(23.0)),
        "carrot"      => (Some(0.0),  Some(0.0),  Some(0.3),  Some(12.0)),
        "onion"       => (Some(0.0),  Some(0.0),  Some(0.2),  Some(10.0)),
        "garlic"      => (Some(0.0),  Some(0.0),  Some(1.7),  Some(25.0)),
        "lemon"       => (Some(0.0),  Some(0.0),  Some(0.6),  Some(8.0)),
        "apple"       => (Some(0.0),  Some(0.0),  Some(0.1),  Some(5.0)),
        "banana"      => (Some(0.0),  Some(0.0),  Some(0.3),  Some(27.0)),
        "rice"        => (Some(0.0),  Some(0.0),  Some(0.8),  Some(25.0)),
        "wheat-flour" => (Some(0.0),  Some(0.0),  Some(1.2),  Some(22.0)),
        "oats"        => (Some(0.0),  Some(0.0),  Some(4.7),  Some(138.0)),
        "butter"      => (Some(0.2),  Some(1.5),  Some(0.0),  Some(2.0)),
        "olive-oil"   => (Some(0.0),  Some(0.0),  Some(0.6),  Some(0.0)),
        _             => (None, None, None, None),
    };
    VitaminData { vitamin_b12_mcg: b12, vitamin_d_mcg: vd, iron_mg: fe, magnesium_mg: mg }
}
