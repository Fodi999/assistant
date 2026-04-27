//! Laboratory **shelf-life engine** — pure, deterministic estimator that
//! converts (ingredients × steps × catalog profiles) into:
//!
//!   • a numeric `shelf_life_days`,
//!   • a bucketed `risk_level`,
//!   • a small set of `storage_recommendations`,
//!   • a list of `warnings`.
//!
//! ⚠️ This is a *technological estimate*, not a food-safety guarantee. The
//! output should be presented as guidance, never as a medical / regulatory
//! claim.
//!
//! Design notes
//! ------------
//! * Pure: no I/O, no DB, no AI. Trivially testable.
//! * Defensive: missing `shelf_life_days` / `ph` / `water_activity` only
//!   downgrade confidence, they never crash the analysis.
//! * Same `LaboratoryWarning` type as `process_engine` so the HTTP layer can
//!   merge two warning lists without conversion.

use serde::Serialize;

use super::catalog_profile_adapter::LaboratoryIngredientProfile;
use super::process_engine::{LaboratoryProcessAnalysis, LaboratoryWarning};
use super::types::{LabProcessStepDto, LabProjectIngredientDto};
use rust_decimal::prelude::ToPrimitive;

// ─────────────────────────────────────────────────────────────────────────────
// Output DTOs
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Default)]
pub struct LaboratoryShelfLifeAnalysis {
    pub shelf_life_days: Option<i32>,
    pub risk_level: String,
    pub storage_recommendations: Vec<LaboratoryStorageRecommendation>,
    pub warnings: Vec<LaboratoryWarning>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LaboratoryStorageRecommendation {
    /// Stable key for the frontend (`refrigeration`, `freezing`, `pantry`, …).
    pub method: String,
    /// Localized human label.
    pub label: String,
    /// Estimated bonus shelf-life days from this storage method (vs. base).
    pub extra_days: Option<i32>,
    /// `low` | `medium` | `high` — informational, no guarantees.
    pub cost_impact: Option<String>,
    /// `low` | `medium` | `high` — informational, no guarantees.
    pub quality_impact: Option<String>,
    /// Short human explanation. i18n later.
    pub message: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Constants — kept tight and conservative.
// ─────────────────────────────────────────────────────────────────────────────

/// Categories that are pH/aw-sensitive and need controlled cold + heat.
const PROTEIN_CATEGORIES: &[&str] = &["dairy", "meat", "fish", "seafood", "egg", "poultry"];
/// pH above which microbial growth is more permissive (unless aw < 0.85).
const PH_DANGER: f64 = 4.6;
/// Water activity above which microbial growth is more permissive.
const AW_DANGER: f64 = 0.85;
/// Hard cap on the absolute shelf-life MVP can claim.
const ABSOLUTE_DAYS_CAP: i32 = 30;

// ─────────────────────────────────────────────────────────────────────────────
// Engine entry point
// ─────────────────────────────────────────────────────────────────────────────

pub fn analyze_shelf_life(
    ingredients: &[LabProjectIngredientDto],
    steps: &[LabProcessStepDto],
    profiles: &[LaboratoryIngredientProfile],
    _process_analysis: &LaboratoryProcessAnalysis,
) -> LaboratoryShelfLifeAnalysis {
    let mut warnings: Vec<LaboratoryWarning> = Vec::new();

    // 1) Base shelf life: min over profiles that actually have data.
    let base_days: Option<i32> = profiles
        .iter()
        .filter_map(|p| p.shelf_life_days)
        .filter(|d| *d > 0)
        .min();

    if base_days.is_none() {
        warnings.push(LaboratoryWarning {
            kind: "shelf_life_unknown".into(),
            severity: "info".into(),
            ingredient_slug: None,
            message: "Недостаточно данных для расчёта срока хранения.".into(),
        });
    }

    // 2) pH × aw safety check.
    let max_ph: Option<f64> = profiles.iter().filter_map(|p| p.ph).fold(None, max_opt);
    let max_aw: Option<f64> = profiles
        .iter()
        .filter_map(|p| p.water_activity)
        .fold(None, max_opt);

    // 3) Step-level signals.
    let max_step_temp: Option<f64> = steps
        .iter()
        .filter_map(|s| s.temperature_c.as_ref().and_then(|d| d.to_f64()))
        .fold(None, max_opt);

    let has_pasteurize = steps
        .iter()
        .any(|s| s.technique.eq_ignore_ascii_case("pasteurize"));
    let has_sterilize = steps
        .iter()
        .any(|s| s.technique.eq_ignore_ascii_case("sterilize"));
    let has_hot_fill = steps
        .iter()
        .any(|s| s.technique.eq_ignore_ascii_case("hot_fill"));
    let has_safe_heat = max_step_temp.map(|t| t >= 75.0).unwrap_or(false)
        || has_pasteurize
        || has_sterilize
        || has_hot_fill;

    // 4) Bonus days from heat treatment.
    let mut bonus = 0i32;
    if has_sterilize || max_step_temp.map(|t| t >= 100.0).unwrap_or(false) {
        bonus = bonus.max(7);
    } else if has_hot_fill || max_step_temp.map(|t| t >= 85.0).unwrap_or(false) {
        bonus = bonus.max(4);
    } else if has_pasteurize || max_step_temp.map(|t| t >= 75.0).unwrap_or(false) {
        bonus = bonus.max(2);
    }

    // 5) Combined shelf-life days (capped).
    let shelf_life_days = base_days.map(|base| {
        let raw = base.saturating_add(bonus);
        let cap_by_base = base.saturating_mul(3);
        raw.min(cap_by_base).min(ABSOLUTE_DAYS_CAP).max(1)
    });

    // 6) Risk level — start optimistic, escalate from evidence.
    let mut risk_rank: u8 = 0; // 0 low, 1 medium, 2 high, 3 critical

    // pH × aw danger zone without thermal kill step.
    let danger_chemistry = matches!((max_ph, max_aw), (Some(ph), Some(aw)) if ph > PH_DANGER && aw > AW_DANGER);
    if danger_chemistry && !(has_pasteurize || has_sterilize) {
        risk_rank = risk_rank.max(3);
        warnings.push(LaboratoryWarning {
            kind: "ph_aw_danger_zone".into(),
            severity: "critical".into(),
            ingredient_slug: None,
            message: format!(
                "Среда благоприятна для роста микроорганизмов (pH {:.1} > {:.1}, aw {:.2} > {:.2}) \
                 и нет шага пастеризации/стерилизации.",
                max_ph.unwrap_or(0.0), PH_DANGER, max_aw.unwrap_or(0.0), AW_DANGER
            ),
        });
    } else if danger_chemistry {
        risk_rank = risk_rank.max(2);
    } else if let Some(ph) = max_ph {
        if ph > PH_DANGER {
            risk_rank = risk_rank.max(1);
        }
    }

    // High-risk categories without safe heat.
    let mut risky_proteins: Vec<&str> = Vec::new();
    for ing in ingredients {
        if let Some(profile) = profiles
            .iter()
            .find(|p| p.slug == ing.ingredient_slug)
        {
            if let Some(cat) = profile.category.as_deref() {
                let cat_lc = cat.to_lowercase();
                if PROTEIN_CATEGORIES.contains(&cat_lc.as_str()) {
                    risky_proteins.push(profile.slug.as_str());
                }
            }
        }
    }
    if !risky_proteins.is_empty() && !has_safe_heat {
        risk_rank = risk_rank.max(2);
        warnings.push(LaboratoryWarning {
            kind: "protein_category_no_heat".into(),
            severity: "warning".into(),
            ingredient_slug: None,
            message: format!(
                "Белковые/молочные продукты ({}) требуют контролируемого охлаждения и безопасной термообработки.",
                risky_proteins.join(", ")
            ),
        });
    }

    // Unknown shelf life — at least medium risk.
    if shelf_life_days.is_none() {
        risk_rank = risk_rank.max(1);
    }

    let risk_level = match risk_rank {
        3 => "critical",
        2 => "high",
        1 => "medium",
        _ => "low",
    }
    .to_string();

    // 7) Storage recommendations.
    let storage_recommendations = build_recommendations(
        ingredients,
        profiles,
        max_ph,
        max_aw,
        has_safe_heat,
        &risk_level,
    );

    LaboratoryShelfLifeAnalysis {
        shelf_life_days,
        risk_level,
        storage_recommendations,
        warnings,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Storage recommendation builder
// ─────────────────────────────────────────────────────────────────────────────

fn build_recommendations(
    ingredients: &[LabProjectIngredientDto],
    profiles: &[LaboratoryIngredientProfile],
    max_ph: Option<f64>,
    max_aw: Option<f64>,
    has_safe_heat: bool,
    risk_level: &str,
) -> Vec<LaboratoryStorageRecommendation> {
    let mut recs: Vec<LaboratoryStorageRecommendation> = Vec::new();

    let any_protein = ingredients.iter().any(|ing| {
        profiles
            .iter()
            .find(|p| p.slug == ing.ingredient_slug)
            .and_then(|p| p.category.as_deref())
            .map(|c| PROTEIN_CATEGORIES.contains(&c.to_lowercase().as_str()))
            .unwrap_or(false)
    });

    let aw_low = matches!(max_aw, Some(aw) if aw < 0.6);
    let ph_acidic = matches!(max_ph, Some(ph) if ph <= 4.6);

    // Default: refrigeration is almost always a sane baseline.
    recs.push(LaboratoryStorageRecommendation {
        method: "refrigeration".into(),
        label: "Хранить при 0–4°C".into(),
        extra_days: None,
        cost_impact: Some("low".into()),
        quality_impact: Some("low".into()),
        message: "После обработки быстро охладить и хранить в холодильнике.".into(),
    });

    // Freezing — extends shelf life significantly, especially for proteins.
    if any_protein || risk_level == "high" || risk_level == "critical" {
        recs.push(LaboratoryStorageRecommendation {
            method: "freezing".into(),
            label: "Заморозка при −18°C".into(),
            extra_days: Some(60),
            cost_impact: Some("medium".into()),
            quality_impact: Some("medium".into()),
            message: "Для длительного хранения заморозить порционно; учесть потерю текстуры при размораживании.".into(),
        });
    }

    // Pantry — only if water activity is low (dry) AND pH is acidic OR shelf-stable.
    if aw_low && ph_acidic {
        recs.push(LaboratoryStorageRecommendation {
            method: "pantry".into(),
            label: "Хранение при комнатной температуре".into(),
            extra_days: Some(14),
            cost_impact: Some("low".into()),
            quality_impact: Some("low".into()),
            message: "Низкая активность воды и кислая среда позволяют хранение в сухом тёмном месте.".into(),
        });
    }

    // Pasteurization advisory if user hasn't included one and conditions need it.
    if !has_safe_heat && (matches!(max_ph, Some(ph) if ph > 4.6) || any_protein) {
        recs.push(LaboratoryStorageRecommendation {
            method: "pasteurization_advisory".into(),
            label: "Рекомендуется пастеризация".into(),
            extra_days: Some(2),
            cost_impact: Some("low".into()),
            quality_impact: Some("low".into()),
            message: "Добавление шага пастеризации (≥75°C) снизит риски и продлит срок хранения.".into(),
        });
    }

    recs
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn max_opt(acc: Option<f64>, v: f64) -> Option<f64> {
    Some(match acc {
        Some(a) => a.max(v),
        None => v,
    })
}

/// Merge two risk levels using the same ordering as the rest of the lab.
pub fn max_risk_level(a: &str, b: &str) -> String {
    let rank = |s: &str| -> u8 {
        match s {
            "critical" => 3,
            "high" => 2,
            "medium" => 1,
            _ => 0,
        }
    };
    let pick = if rank(a) >= rank(b) { a } else { b };
    pick.to_string()
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;
    use time::OffsetDateTime;
    use uuid::Uuid;

    fn ing(slug: &str) -> LabProjectIngredientDto {
        LabProjectIngredientDto {
            id: Uuid::new_v4(),
            ingredient_slug: slug.into(),
            quantity: Decimal::from_str("100").unwrap(),
            unit: "g".into(),
            role: None,
            sort_order: 0,
            notes: None,
            created_at: OffsetDateTime::now_utc(),
        }
    }

    fn step(tech: &str, temp: Option<f64>) -> LabProcessStepDto {
        LabProcessStepDto {
            id: Uuid::new_v4(),
            order_index: 0,
            technique: tech.into(),
            temperature_c: temp.map(|t| Decimal::from_f64_retain(t).unwrap()),
            duration_min: None,
            target_slugs: vec![],
            notes: None,
            created_at: OffsetDateTime::now_utc(),
        }
    }

    fn profile(slug: &str, days: Option<i32>, ph: Option<f64>, aw: Option<f64>, category: Option<&str>) -> LaboratoryIngredientProfile {
        LaboratoryIngredientProfile {
            slug: slug.into(),
            name: slug.into(),
            shelf_life_days: days,
            ph,
            water_activity: aw,
            category: category.map(String::from),
            ..Default::default()
        }
    }

    #[test]
    fn apricot_at_75c_gets_small_bonus() {
        let res = analyze_shelf_life(
            &[ing("apricot")],
            &[step("heat", Some(75.0))],
            &[profile("apricot", Some(5), Some(4.0), Some(1.0), Some("fruit"))],
            &LaboratoryProcessAnalysis::default(),
        );
        // base 5 + bonus 2 = 7, no protein, ph<4.6 -> low
        assert_eq!(res.shelf_life_days, Some(7));
        assert_eq!(res.risk_level, "low");
        assert!(res
            .storage_recommendations
            .iter()
            .any(|r| r.method == "refrigeration"));
    }

    #[test]
    fn protein_without_heat_warns_and_high_risk() {
        let res = analyze_shelf_life(
            &[ing("cream")],
            &[step("mix", None)],
            &[profile("cream", Some(3), Some(6.5), Some(0.95), Some("dairy"))],
            &LaboratoryProcessAnalysis::default(),
        );
        // ph>4.6 + aw>0.85 + no heat → critical
        assert_eq!(res.risk_level, "critical");
        assert!(res
            .warnings
            .iter()
            .any(|w| w.kind == "ph_aw_danger_zone"));
        // freezing + pasteurization advisory should appear
        assert!(res
            .storage_recommendations
            .iter()
            .any(|r| r.method == "freezing"));
        assert!(res
            .storage_recommendations
            .iter()
            .any(|r| r.method == "pasteurization_advisory"));
    }

    #[test]
    fn pasteurization_neutralizes_chemistry_danger_to_high() {
        let res = analyze_shelf_life(
            &[ing("cream")],
            &[step("pasteurize", Some(80.0))],
            &[profile("cream", Some(3), Some(6.5), Some(0.95), Some("dairy"))],
            &LaboratoryProcessAnalysis::default(),
        );
        // danger chemistry + pasteurize → not critical (max 2 = high possible from protein? has heat → not added)
        assert_ne!(res.risk_level, "critical");
        // Should NOT have ph_aw_danger_zone warning when pasteurize present.
        assert!(!res
            .warnings
            .iter()
            .any(|w| w.kind == "ph_aw_danger_zone"));
    }

    #[test]
    fn no_profile_data_yields_unknown() {
        let res = analyze_shelf_life(
            &[ing("mystery")],
            &[step("heat", Some(75.0))],
            &[],
            &LaboratoryProcessAnalysis::default(),
        );
        assert_eq!(res.shelf_life_days, None);
        assert_eq!(res.risk_level, "medium");
        assert!(res
            .warnings
            .iter()
            .any(|w| w.kind == "shelf_life_unknown"));
    }

    #[test]
    fn min_of_multiple_profiles_used_as_base() {
        let res = analyze_shelf_life(
            &[ing("apricot"), ing("cream")],
            &[step("pasteurize", Some(80.0))],
            &[
                profile("apricot", Some(5), Some(4.0), Some(0.95), Some("fruit")),
                profile("cream", Some(3), Some(6.5), Some(0.95), Some("dairy")),
            ],
            &LaboratoryProcessAnalysis::default(),
        );
        // base = min(5,3) = 3; bonus = 2 (pasteurize). Cap by base*3 = 9.
        // 3 + 2 = 5 → 5
        assert_eq!(res.shelf_life_days, Some(5));
    }

    #[test]
    fn absolute_cap_30_days() {
        let res = analyze_shelf_life(
            &[ing("dry_legume")],
            &[step("sterilize", Some(120.0))],
            &[profile("dry_legume", Some(20), Some(6.0), Some(0.4), Some("grain"))],
            &LaboratoryProcessAnalysis::default(),
        );
        // 20 + 7 = 27 ≤ 30 → 27
        assert_eq!(res.shelf_life_days, Some(27));

        let res2 = analyze_shelf_life(
            &[ing("dry_legume")],
            &[step("sterilize", Some(120.0))],
            &[profile("dry_legume", Some(60), Some(6.0), Some(0.4), Some("grain"))],
            &LaboratoryProcessAnalysis::default(),
        );
        // 60 + 7 = 67, capped to 30
        assert_eq!(res2.shelf_life_days, Some(30));
    }

    #[test]
    fn max_risk_level_picks_higher() {
        assert_eq!(max_risk_level("low", "medium"), "medium");
        assert_eq!(max_risk_level("high", "medium"), "high");
        assert_eq!(max_risk_level("critical", "high"), "critical");
        assert_eq!(max_risk_level("low", "low"), "low");
    }
}
