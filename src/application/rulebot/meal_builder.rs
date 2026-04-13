//! Meal Builder — smart combo assembler.
//!
//! Given a user goal (HighProtein / LowCalorie / Balanced), dynamically picks:
//!   - 1 protein source  (meat / fish / high-protein dairy / legume)
//!   - 1 side            (vegetable / mushroom / fruit)
//!   - 1 base (optional) (grain / legume)
//!
//! Then computes combined nutrition for a ~400g plate (150g protein + 150g side + 100g base)
//! and generates a structured MealCombo.
//!
//! No hardcoded recipes — every combo is built from the live ingredient cache.

use crate::infrastructure::ingredient_cache::IngredientData;
use super::response_builder::HealthGoal;

/// A dynamically assembled meal combo.
#[derive(Debug, Clone)]
pub struct MealCombo {
    pub protein: IngredientData,
    pub side: IngredientData,
    pub base: Option<IngredientData>,
    /// Portion sizes in grams.
    pub protein_g: f32,
    pub side_g: f32,
    pub base_g: f32,
    /// Total nutrition for the plate.
    pub total_kcal: u32,
    pub total_protein: f32,
    pub total_fat: f32,
    pub total_carbs: f32,
}

impl MealCombo {
    /// All ingredients in the combo (for card rendering).
    pub fn ingredients(&self) -> Vec<&IngredientData> {
        let mut v = vec![&self.protein, &self.side];
        if let Some(ref b) = self.base {
            v.push(b);
        }
        v
    }
}

/// Build a meal combo from a pool of ingredients.
///
/// Strategy:
///   1. Split pool into roles: protein / side / base
///   2. Score each within its role (goal-weighted)
///   3. Pick top protein, top side, optionally top base
///   4. Compute combined nutrition
///
/// `exclude_slugs` — products already shown in this session.
pub fn build_combo(
    all: &[IngredientData],
    goal: HealthGoal,
    exclude_slugs: &[String],
) -> Option<MealCombo> {
    // ── Split by role ──────────────────────────────────────────────────
    let mut proteins: Vec<&IngredientData> = Vec::new();
    let mut sides: Vec<&IngredientData> = Vec::new();
    let mut bases: Vec<&IngredientData> = Vec::new();

    for p in all {
        if exclude_slugs.contains(&p.slug) { continue; }
        if p.calories_per_100g <= 0.0 && p.protein_per_100g <= 0.0 { continue; }
        // Skip spices, condiments, oils, beverages — not standalone meal components
        match p.product_type.as_str() {
            "spice" | "herb" | "condiment" | "oil" | "beverage" | "other" => continue,
            _ => {}
        }
        match p.meal_role() {
            "protein" => proteins.push(p),
            "side"    => sides.push(p),
            "base"    => bases.push(p),
            _         => {}
        }
    }

    if proteins.is_empty() || sides.is_empty() {
        return None;
    }

    // ── Goal-aware scoring ────────────────────────────────────────────
    let score_protein = |p: &IngredientData| -> f64 {
        match goal {
            HealthGoal::HighProtein => p.protein_per_100g as f64 * 2.0 - p.fat_per_100g as f64 * 0.5,
            HealthGoal::LowCalorie  => p.protein_per_100g as f64 * 1.5 - p.calories_per_100g as f64 * 0.01,
            HealthGoal::Balanced    => p.protein_per_100g as f64 * 1.2 - p.fat_per_100g as f64 * 0.3,
        }
    };

    let score_side = |p: &IngredientData| -> f64 {
        match goal {
            HealthGoal::HighProtein => p.protein_per_100g as f64 + (100.0 - p.calories_per_100g as f64) * 0.02,
            HealthGoal::LowCalorie  => (100.0 - p.calories_per_100g as f64) * 0.05 + p.protein_per_100g as f64 * 0.5,
            HealthGoal::Balanced    => (80.0 - p.calories_per_100g as f64) * 0.03 + p.protein_per_100g as f64 * 0.8,
        }
    };

    let score_base = |p: &IngredientData| -> f64 {
        match goal {
            HealthGoal::HighProtein => p.protein_per_100g as f64 * 1.5 + p.carbs_per_100g as f64 * 0.3,
            HealthGoal::LowCalorie  => (400.0 - p.calories_per_100g as f64) * 0.02 + p.protein_per_100g as f64 * 0.5,
            HealthGoal::Balanced    => p.protein_per_100g as f64 + p.carbs_per_100g as f64 * 0.5 - p.fat_per_100g as f64 * 0.2,
        }
    };

    // ── 80/20 exploration: sometimes pick 2nd or 3rd instead of always top-1
    let explore_idx = {
        use std::time::{SystemTime, UNIX_EPOCH};
        let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        (secs % 3) as usize  // 0, 1, or 2
    };

    let pick_top = |pool: &mut Vec<&IngredientData>, scorer: &dyn Fn(&IngredientData) -> f64| -> Option<IngredientData> {
        if pool.is_empty() { return None; }
        pool.sort_by(|a, b| scorer(b).partial_cmp(&scorer(a)).unwrap_or(std::cmp::Ordering::Equal));
        let idx = explore_idx.min(pool.len() - 1);
        Some(pool.remove(idx).clone())
    };

    let protein = pick_top(&mut proteins, &score_protein)?;
    let side = pick_top(&mut sides, &score_side)?;
    let base = if goal == HealthGoal::LowCalorie {
        None // Low-calorie meals skip the carb base
    } else {
        pick_top(&mut bases, &score_base)
    };

    // ── Portion sizes ─────────────────────────────────────────────────
    let (protein_g, side_g, base_g) = match goal {
        HealthGoal::HighProtein => (200.0_f32, 150.0, 100.0),
        HealthGoal::LowCalorie  => (150.0, 200.0, 0.0),
        HealthGoal::Balanced    => (150.0, 150.0, 100.0),
    };

    let actual_base_g = if base.is_some() { base_g } else { 0.0 };

    // ── Compute totals ────────────────────────────────────────────────
    let mut total_kcal = 0u32;
    let mut total_protein = 0.0_f32;
    let mut total_fat = 0.0_f32;
    let mut total_carbs = 0.0_f32;

    let add = |ing: &IngredientData, grams: f32, kcal: &mut u32, pro: &mut f32, fat: &mut f32, carb: &mut f32| {
        *kcal += ing.kcal_for(grams);
        *pro  += ing.protein_for(grams);
        *fat  += ing.fat_for(grams);
        *carb += ing.carbs_for(grams);
    };

    add(&protein, protein_g, &mut total_kcal, &mut total_protein, &mut total_fat, &mut total_carbs);
    add(&side, side_g, &mut total_kcal, &mut total_protein, &mut total_fat, &mut total_carbs);
    if let Some(ref b) = base {
        add(b, actual_base_g, &mut total_kcal, &mut total_protein, &mut total_fat, &mut total_carbs);
    }

    Some(MealCombo {
        protein,
        side,
        base,
        protein_g,
        side_g,
        base_g: actual_base_g,
        total_kcal,
        total_protein,
        total_fat,
        total_carbs,
    })
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ingredient(slug: &str, ptype: &str, cal: f32, pro: f32, fat: f32, carbs: f32) -> IngredientData {
        IngredientData {
            slug: slug.into(),
            name_en: slug.into(),
            name_ru: slug.into(),
            name_pl: slug.into(),
            name_uk: slug.into(),
            calories_per_100g: cal,
            protein_per_100g: pro,
            fat_per_100g: fat,
            carbs_per_100g: carbs,
            image_url: None,
            product_type: ptype.into(),
        }
    }

    #[test]
    fn builds_high_protein_combo() {
        let pool = vec![
            make_ingredient("chicken-breast", "meat", 165.0, 31.0, 3.6, 0.0),
            make_ingredient("broccoli", "vegetable", 34.0, 2.8, 0.4, 7.0),
            make_ingredient("rice", "grain", 110.0, 7.0, 0.9, 74.0),
            make_ingredient("olive-oil", "oil", 884.0, 0.0, 100.0, 0.0),
            make_ingredient("basil", "spice", 23.0, 3.2, 0.6, 2.7),
        ];
        let combo = build_combo(&pool, HealthGoal::HighProtein, &[]).unwrap();
        assert_eq!(combo.protein.meal_role(), "protein");
        assert_eq!(combo.side.meal_role(), "side");
        assert!(combo.base.is_some());
        assert!(combo.total_protein > 50.0, "should have significant protein: {}", combo.total_protein);
        assert!(combo.total_kcal > 0);
    }

    #[test]
    fn low_calorie_skips_base() {
        let pool = vec![
            make_ingredient("cod", "fish", 82.0, 17.5, 0.7, 0.0),
            make_ingredient("cucumber", "vegetable", 15.0, 0.7, 0.2, 2.6),
            make_ingredient("rice", "grain", 110.0, 7.0, 0.9, 74.0),
        ];
        let combo = build_combo(&pool, HealthGoal::LowCalorie, &[]).unwrap();
        assert!(combo.base.is_none(), "low calorie should skip base grain");
        assert!(combo.total_kcal < 200, "should be very low cal: {}", combo.total_kcal);
    }

    #[test]
    fn respects_exclusions() {
        let pool = vec![
            make_ingredient("salmon", "fish", 208.0, 20.0, 13.0, 0.0),
            make_ingredient("spinach", "vegetable", 23.0, 2.9, 0.4, 3.6),
        ];
        let combo = build_combo(&pool, HealthGoal::Balanced, &["salmon".into()]);
        assert!(combo.is_none(), "only protein was salmon → excluded → no combo");
    }

    #[test]
    fn excludes_spices_and_oils() {
        let pool = vec![
            make_ingredient("basil", "spice", 23.0, 3.2, 0.6, 2.7),
            make_ingredient("olive-oil", "oil", 884.0, 0.0, 100.0, 0.0),
            make_ingredient("spinach", "vegetable", 23.0, 2.9, 0.4, 3.6),
        ];
        // No protein source → should return None
        let combo = build_combo(&pool, HealthGoal::Balanced, &[]);
        assert!(combo.is_none());
    }

    #[test]
    fn balanced_has_all_three() {
        let pool = vec![
            make_ingredient("salmon", "fish", 208.0, 20.0, 13.0, 0.0),
            make_ingredient("broccoli", "vegetable", 34.0, 2.8, 0.4, 7.0),
            make_ingredient("quinoa", "grain", 368.0, 14.0, 6.0, 64.0),
        ];
        let combo = build_combo(&pool, HealthGoal::Balanced, &[]).unwrap();
        assert!(combo.base.is_some());
        assert!(combo.ingredients().len() == 3);
    }

    #[test]
    fn nutrition_math_is_correct() {
        let pool = vec![
            make_ingredient("chicken-breast", "meat", 165.0, 31.0, 3.6, 0.0),
            make_ingredient("spinach", "vegetable", 23.0, 2.9, 0.4, 3.6),
        ];
        let combo = build_combo(&pool, HealthGoal::LowCalorie, &[]).unwrap();
        // chicken 150g = 247.5 kcal, spinach 200g = 46 kcal → ~293
        assert!(combo.total_kcal > 250 && combo.total_kcal < 350,
            "expected ~293 kcal, got {}", combo.total_kcal);
    }
}
