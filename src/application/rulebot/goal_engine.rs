//! Goal Engine — nutritional target profiles for recipe adaptation.
//!
//! Maps `HealthModifier` (8 variants) → `GoalProfile` with precise
//! per-serving macro targets, preferred/forbidden cooking methods,
//! and a rebalancing strategy.
//!
//! Used by `adaptation_engine` to rebalance ingredient quantities
//! and by `auto_fix` to compensate for removed ingredients.

use std::ops::Range;
use serde::Serialize;
use super::meal_builder::CookMethod;
use super::goal_modifier::HealthModifier;

// ── Goal Strategy ────────────────────────────────────────────────────────────

/// High-level rebalancing strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GoalStrategy {
    /// Cut calories: reduce oil, reduce carb portions, keep protein
    ReduceCalories,
    /// Maximize protein: increase protein portion, moderate carbs
    IncreaseProtein,
    /// Surplus for muscle gain: increase protein + carbs + total portion
    IncreaseCalories,
    /// Keto-adjacent: cut carbs, increase fat/protein
    ReduceCarbs,
    /// No special rebalancing
    Balanced,
}

// ── Protein Level ────────────────────────────────────────────────────────────

/// Protein adequacy classification per serving.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProteinLevel {
    /// 0–10g: dangerously low, needs auto-fix
    None,
    /// 10–25g: suboptimal
    Low,
    /// 25–45g: ideal range
    Optimal,
    /// 45g+: very high (fine for muscle gain)
    High,
}

/// Classify protein grams per serving.
pub fn classify_protein(g: f32) -> ProteinLevel {
    if g < 10.0 { ProteinLevel::None }
    else if g < 25.0 { ProteinLevel::Low }
    else if g < 45.0 { ProteinLevel::Optimal }
    else { ProteinLevel::High }
}

// ── Goal Profile ─────────────────────────────────────────────────────────────

/// Nutritional target profile for one serving.
#[derive(Debug, Clone)]
pub struct GoalProfile {
    pub name: &'static str,
    pub protein_g: Range<f32>,
    pub fat_g: Range<f32>,
    pub carbs_g: Range<f32>,
    pub kcal: Range<f32>,
    pub preferred_methods: Vec<CookMethod>,
    pub forbidden_methods: Vec<CookMethod>,
    pub strategy: GoalStrategy,
    /// Portion multiplier relative to Balanced (1.0)
    pub portion_factor: f32,
    /// Max oil grams for this goal
    pub max_oil_g: f32,
}

impl GoalProfile {
    /// Check if per-serving protein is within target.
    pub fn protein_ok(&self, g: f32) -> bool {
        g >= self.protein_g.start && g < self.protein_g.end
    }

    /// Check if per-serving kcal is within target.
    pub fn kcal_ok(&self, kcal: f32) -> bool {
        kcal >= self.kcal.start && kcal < self.kcal.end
    }

    /// How much protein deficit per serving (0 if sufficient).
    pub fn protein_deficit(&self, g: f32) -> f32 {
        if g >= self.protein_g.start { 0.0 }
        else { self.protein_g.start - g }
    }

    /// How much kcal surplus per serving (0 if within range).
    pub fn kcal_surplus(&self, kcal: f32) -> f32 {
        if kcal < self.kcal.end { 0.0 }
        else { kcal - self.kcal.end }
    }

    /// How much fat surplus per serving.
    pub fn fat_surplus(&self, g: f32) -> f32 {
        if g < self.fat_g.end { 0.0 }
        else { g - self.fat_g.end }
    }

    /// How much carbs surplus per serving.
    pub fn carbs_surplus(&self, g: f32) -> f32 {
        if g < self.carbs_g.end { 0.0 }
        else { g - self.carbs_g.end }
    }
}

// ── Profile Constructors ─────────────────────────────────────────────────────

/// Map HealthModifier → GoalProfile with nutritional targets.
pub fn profile_for(modifier: HealthModifier) -> GoalProfile {
    match modifier {
        HealthModifier::LowCalorie => GoalProfile {
            name: "weight_loss",
            protein_g: 25.0..40.0,
            fat_g: 5.0..15.0,
            carbs_g: 20.0..50.0,
            kcal: 300.0..500.0,
            preferred_methods: vec![CookMethod::Steam, CookMethod::Grill, CookMethod::Bake],
            forbidden_methods: vec![CookMethod::Fry],
            strategy: GoalStrategy::ReduceCalories,
            portion_factor: 0.85,
            max_oil_g: 5.0,
        },
        HealthModifier::HighProtein => GoalProfile {
            name: "high_protein",
            protein_g: 40.0..70.0,
            fat_g: 10.0..25.0,
            carbs_g: 30.0..80.0,
            kcal: 500.0..800.0,
            preferred_methods: vec![CookMethod::Grill, CookMethod::Bake, CookMethod::Boil],
            forbidden_methods: vec![],
            strategy: GoalStrategy::IncreaseProtein,
            portion_factor: 1.2,
            max_oil_g: 15.0,
        },
        HealthModifier::ComfortFood => GoalProfile {
            name: "muscle_gain",
            protein_g: 35.0..60.0,
            fat_g: 10.0..25.0,
            carbs_g: 50.0..100.0,
            kcal: 600.0..900.0,
            preferred_methods: vec![CookMethod::Bake, CookMethod::Boil, CookMethod::Saute],
            forbidden_methods: vec![],
            strategy: GoalStrategy::IncreaseCalories,
            portion_factor: 1.3,
            max_oil_g: 20.0,
        },
        HealthModifier::LowCarb => GoalProfile {
            name: "low_carb",
            protein_g: 30.0..55.0,
            fat_g: 15.0..35.0,
            carbs_g: 10.0..30.0,
            kcal: 400.0..650.0,
            preferred_methods: vec![CookMethod::Grill, CookMethod::Bake, CookMethod::Steam],
            forbidden_methods: vec![CookMethod::Fry],
            strategy: GoalStrategy::ReduceCarbs,
            portion_factor: 1.0,
            max_oil_g: 15.0,
        },
        HealthModifier::HighFiber => GoalProfile {
            name: "high_fiber",
            protein_g: 20.0..40.0,
            fat_g: 5.0..20.0,
            carbs_g: 30.0..70.0,
            kcal: 350.0..550.0,
            preferred_methods: vec![CookMethod::Boil, CookMethod::Steam, CookMethod::Bake],
            forbidden_methods: vec![CookMethod::Fry],
            strategy: GoalStrategy::ReduceCalories,
            portion_factor: 1.0,
            max_oil_g: 10.0,
        },
        HealthModifier::Quick => GoalProfile {
            name: "balanced",
            protein_g: 20.0..40.0,
            fat_g: 10.0..25.0,
            carbs_g: 30.0..80.0,
            kcal: 400.0..700.0,
            preferred_methods: vec![CookMethod::Saute, CookMethod::Fry, CookMethod::Boil],
            forbidden_methods: vec![],
            strategy: GoalStrategy::Balanced,
            portion_factor: 1.0,
            max_oil_g: 15.0,
        },
        HealthModifier::Budget | HealthModifier::None => GoalProfile {
            name: "balanced",
            protein_g: 20.0..40.0,
            fat_g: 10.0..25.0,
            carbs_g: 30.0..80.0,
            kcal: 400.0..700.0,
            preferred_methods: vec![],
            forbidden_methods: vec![],
            strategy: GoalStrategy::Balanced,
            portion_factor: 1.0,
            max_oil_g: 15.0,
        },
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_protein_levels() {
        assert_eq!(classify_protein(0.0), ProteinLevel::None);
        assert_eq!(classify_protein(5.0), ProteinLevel::None);
        assert_eq!(classify_protein(10.0), ProteinLevel::Low);
        assert_eq!(classify_protein(20.0), ProteinLevel::Low);
        assert_eq!(classify_protein(25.0), ProteinLevel::Optimal);
        assert_eq!(classify_protein(40.0), ProteinLevel::Optimal);
        assert_eq!(classify_protein(45.0), ProteinLevel::High);
        assert_eq!(classify_protein(70.0), ProteinLevel::High);
    }

    #[test]
    fn weight_loss_profile() {
        let p = profile_for(HealthModifier::LowCalorie);
        assert_eq!(p.name, "weight_loss");
        assert_eq!(p.strategy, GoalStrategy::ReduceCalories);
        assert!(p.protein_g.start >= 25.0);
        assert!(p.kcal.end <= 500.0);
        assert!(p.forbidden_methods.contains(&CookMethod::Fry));
        assert!(p.max_oil_g <= 5.0);
    }

    #[test]
    fn high_protein_profile() {
        let p = profile_for(HealthModifier::HighProtein);
        assert_eq!(p.name, "high_protein");
        assert_eq!(p.strategy, GoalStrategy::IncreaseProtein);
        assert!(p.protein_g.start >= 40.0);
        assert!(p.portion_factor > 1.0);
    }

    #[test]
    fn comfort_food_is_muscle_gain() {
        let p = profile_for(HealthModifier::ComfortFood);
        assert_eq!(p.strategy, GoalStrategy::IncreaseCalories);
        assert!(p.portion_factor >= 1.3);
    }

    #[test]
    fn low_carb_profile() {
        let p = profile_for(HealthModifier::LowCarb);
        assert_eq!(p.strategy, GoalStrategy::ReduceCarbs);
        assert!(p.carbs_g.end <= 30.0);
    }

    #[test]
    fn protein_deficit_calculation() {
        let p = profile_for(HealthModifier::LowCalorie);
        assert!(p.protein_deficit(10.0) > 0.0); // below range
        assert_eq!(p.protein_deficit(30.0), 0.0); // within range
    }

    #[test]
    fn kcal_surplus_calculation() {
        let p = profile_for(HealthModifier::LowCalorie);
        assert_eq!(p.kcal_surplus(400.0), 0.0);    // within range
        assert!(p.kcal_surplus(600.0) > 0.0);       // above range
    }

    #[test]
    fn balanced_has_no_forbidden_methods() {
        let p = profile_for(HealthModifier::None);
        assert!(p.forbidden_methods.is_empty());
        assert_eq!(p.strategy, GoalStrategy::Balanced);
    }
}
