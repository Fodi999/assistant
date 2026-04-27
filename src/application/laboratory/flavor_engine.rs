//! Laboratory **flavor engine** — pure, deterministic sensory analyzer.
//!
//! Given a list of ingredients (with quantities) and their catalog
//! profiles, this engine produces:
//!
//!   • a weighted sensory profile (sweetness/acidity/bitterness/umami/aroma)
//!   • a `dominant_profile` tag and a human label
//!   • pairing suggestions extracted from `culinary_behaviors[].targets`
//!     where `effect == "complements"`
//!   • lightweight warnings (e.g. "no profile data available")
//!
//! Design notes
//! ------------
//! * Pure: no I/O, no DB, no AI. Easy to unit-test.
//! * Defensive: missing sensory values just shrink the weight pool;
//!   missing profiles → warning, but never crash.
//! * Sensory scale matches `food_culinary_properties` columns: 0..10.
//! * Quantity is used as a relative weight. Unit conversion (g vs ml vs
//!   ea) is intentionally NOT done in MVP — the user typically uses g/ml
//!   so this is good enough; later we'll multiply by `density_g_per_ml`.

use rust_decimal::prelude::ToPrimitive;
use serde::Serialize;
use std::collections::HashSet;

use super::catalog_profile_adapter::LaboratoryIngredientProfile;
use super::process_engine::LaboratoryWarning;
use super::types::LabProjectIngredientDto;

// ─────────────────────────────────────────────────────────────────────────────
// Output DTOs
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Default)]
pub struct LaboratoryFlavorAnalysis {
    pub flavor_result: LaboratoryFlavorResult,
    pub pairing_suggestions: Vec<LaboratoryPairingSuggestion>,
    pub warnings: Vec<LaboratoryWarning>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct LaboratoryFlavorResult {
    pub sweetness: Option<f64>,
    pub acidity: Option<f64>,
    pub bitterness: Option<f64>,
    pub umami: Option<f64>,
    pub aroma: Option<f64>,
    /// Stable tag for the frontend (`sweet`, `sweet_sour`, `umami_rich`, …).
    pub dominant_profile: String,
    /// Human-readable label (RU for now; i18n later).
    pub balance_label: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LaboratoryPairingSuggestion {
    /// Slug from `culinary_behaviors[].targets`. May not exist as a product.
    pub ingredient_slug: Option<String>,
    /// For now equals `slug` if no name resolution; the service layer or
    /// frontend can enrich.
    pub ingredient_name: String,
    /// 0.0 .. 100.0 — derived from `pairing_score` × `intensity` (×100).
    pub score: f64,
    /// Optional human reason (e.g. "сочетается с сливками по поведению pcb").
    pub reason: Option<String>,
    /// `culinary_behavior` for now; future sources may include
    /// `flavor_graph`, `chef_db`, etc.
    pub source: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Thresholds
// ─────────────────────────────────────────────────────────────────────────────

const T_SWEET: f64 = 6.5;
const T_ACID: f64 = 4.5;
const T_ACID_STRONG: f64 = 6.0;
const T_UMAMI: f64 = 6.0;
const T_BITTER: f64 = 5.0;
const T_AROMA: f64 = 7.0;

// ─────────────────────────────────────────────────────────────────────────────
// Entry point
// ─────────────────────────────────────────────────────────────────────────────

pub fn analyze_flavor(
    ingredients: &[LabProjectIngredientDto],
    profiles: &[LaboratoryIngredientProfile],
) -> LaboratoryFlavorAnalysis {
    let mut warnings: Vec<LaboratoryWarning> = Vec::new();

    // Slugs already in the project — never recommend them again.
    let in_project: HashSet<String> = ingredients
        .iter()
        .map(|i| i.ingredient_slug.to_lowercase())
        .collect();

    // ── Weighted sensory averages ────────────────────────────────────────────
    let mut sw = WeightedAvg::default();
    let mut ac = WeightedAvg::default();
    let mut bt = WeightedAvg::default();
    let mut um = WeightedAvg::default();
    let mut ar = WeightedAvg::default();
    let mut profiled_count = 0usize;

    for ing in ingredients {
        let weight = ing.quantity.to_f64().unwrap_or(0.0).max(0.0);
        if weight <= 0.0 {
            continue;
        }
        let Some(p) = profiles
            .iter()
            .find(|p| p.slug == ing.ingredient_slug)
        else {
            continue;
        };
        profiled_count += 1;
        sw.push_opt(p.sweetness, weight);
        ac.push_opt(p.acidity, weight);
        bt.push_opt(p.bitterness, weight);
        um.push_opt(p.umami, weight);
        ar.push_opt(p.aroma, weight);
    }

    if profiled_count == 0 && !ingredients.is_empty() {
        warnings.push(LaboratoryWarning {
            kind: "flavor_no_profile_data".into(),
            severity: "info".into(),
            ingredient_slug: None,
            message: "Недостаточно данных о вкусе ингредиентов для расчёта профиля.".into(),
        });
    }

    let flavor_result = build_flavor_result(
        sw.value(),
        ac.value(),
        bt.value(),
        um.value(),
        ar.value(),
    );

    // ── Pairing suggestions from culinary_behaviors[].targets ────────────────
    let pairing_suggestions = collect_pairings(profiles, &in_project);

    LaboratoryFlavorAnalysis {
        flavor_result,
        pairing_suggestions,
        warnings,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Sensory aggregator
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Default)]
struct WeightedAvg {
    sum: f64,
    weight: f64,
}

impl WeightedAvg {
    fn push_opt(&mut self, value: Option<f64>, weight: f64) {
        if let Some(v) = value {
            self.sum += v * weight;
            self.weight += weight;
        }
    }
    fn value(&self) -> Option<f64> {
        if self.weight > 0.0 {
            Some((self.sum / self.weight * 100.0).round() / 100.0)
        } else {
            None
        }
    }
}

fn build_flavor_result(
    sweetness: Option<f64>,
    acidity: Option<f64>,
    bitterness: Option<f64>,
    umami: Option<f64>,
    aroma: Option<f64>,
) -> LaboratoryFlavorResult {
    let s = sweetness.unwrap_or(0.0);
    let a = acidity.unwrap_or(0.0);
    let b = bitterness.unwrap_or(0.0);
    let u = umami.unwrap_or(0.0);
    let r = aroma.unwrap_or(0.0);

    // Order of precedence: sweet_sour > sweet > acidic > umami_rich > bitter > aromatic > balanced.
    let (tag, label) = if s >= T_SWEET && a >= T_ACID {
        ("sweet_sour", "Сладко-кислый профиль")
    } else if s >= T_SWEET {
        ("sweet", "Сладкий профиль")
    } else if a >= T_ACID_STRONG {
        ("acidic", "Кислотный профиль")
    } else if u >= T_UMAMI {
        ("umami_rich", "Умами-насыщенный профиль")
    } else if b >= T_BITTER {
        ("bitter", "Горьковатый профиль")
    } else if r >= T_AROMA {
        ("aromatic", "Ароматный профиль")
    } else if sweetness.is_none()
        && acidity.is_none()
        && bitterness.is_none()
        && umami.is_none()
        && aroma.is_none()
    {
        ("unknown", "Профиль не определён")
    } else {
        ("balanced", "Сбалансированный профиль")
    };

    let message = match tag {
        "sweet_sour" => format!(
            "Доминирует сладко-кислый баланс (сладость {s:.1}, кислотность {a:.1}). Хорош для соусов и десертов."
        ),
        "sweet" => format!("Преобладает сладость ({s:.1}). Подойдёт для десертов и кондитерских изделий."),
        "acidic" => format!("Выражена кислотность ({a:.1}). Освежающий профиль, хорошо балансирует жирное."),
        "umami_rich" => format!("Высокий уровень умами ({u:.1}). Глубокий, насыщенный вкус."),
        "bitter" => format!("Заметна горечь ({b:.1}). Уравновесит сладкое или жирное."),
        "aromatic" => format!("Сильный аромат ({r:.1}). Ведущая нота — ароматика."),
        "balanced" => "Профиль сбалансирован, без явного доминирования.".to_string(),
        _ => "Недостаточно вкусовых данных для оценки.".to_string(),
    };

    LaboratoryFlavorResult {
        sweetness,
        acidity,
        bitterness,
        umami,
        aroma,
        dominant_profile: tag.to_string(),
        balance_label: label.to_string(),
        message,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Pairing extraction
// ─────────────────────────────────────────────────────────────────────────────

fn collect_pairings(
    profiles: &[LaboratoryIngredientProfile],
    in_project: &HashSet<String>,
) -> Vec<LaboratoryPairingSuggestion> {
    use std::collections::HashMap;

    // Aggregate scores per target slug across all profiles/behaviors.
    let mut best: HashMap<String, (f64, String, String)> = HashMap::new();
    // value tuple: (score, source_slug, reason)

    for profile in profiles {
        for behavior in &profile.culinary_behaviors {
            // Accept "complements" (canonical) or category=="pairing".
            let is_pairing = matches!(behavior.effect.as_deref(), Some("complements"))
                || matches!(behavior.category.as_deref(), Some("pairing"));
            if !is_pairing {
                continue;
            }
            // intensity is our score proxy: 0..1 → 0..100.
            let intensity = behavior.intensity.unwrap_or(0.7).clamp(0.0, 1.0);
            let score = (intensity * 100.0).round();

            for target in &behavior.targets {
                let slug = target.trim().to_lowercase();
                if slug.is_empty() || in_project.contains(&slug) {
                    continue;
                }
                let reason = format!(
                    "Хорошо сочетается с «{}» ({})",
                    profile.name,
                    behavior
                        .title
                        .clone()
                        .unwrap_or_else(|| "culinary behavior".into())
                );
                best.entry(slug)
                    .and_modify(|e| {
                        if score > e.0 {
                            *e = (score, profile.slug.clone(), reason.clone());
                        }
                    })
                    .or_insert((score, profile.slug.clone(), reason));
            }
        }
    }

    let mut out: Vec<LaboratoryPairingSuggestion> = best
        .into_iter()
        .map(|(slug, (score, _src, reason))| LaboratoryPairingSuggestion {
            ingredient_slug: Some(slug.clone()),
            ingredient_name: slug,
            score,
            reason: Some(reason),
            source: "culinary_behavior".into(),
        })
        .collect();

    // Stable order: highest score first, then alphabetic.
    out.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.ingredient_name.cmp(&b.ingredient_name))
    });

    // Cap to a reasonable number for the frontend.
    out.truncate(12);
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::catalog_profile_adapter::LaboratoryCulinaryBehavior;
    use rust_decimal::Decimal;
    use std::str::FromStr;
    use time::OffsetDateTime;
    use uuid::Uuid;

    fn ing(slug: &str, qty: &str) -> LabProjectIngredientDto {
        LabProjectIngredientDto {
            id: Uuid::new_v4(),
            ingredient_slug: slug.into(),
            quantity: Decimal::from_str(qty).unwrap(),
            unit: "g".into(),
            role: None,
            sort_order: 0,
            notes: None,
            created_at: OffsetDateTime::now_utc(),
        }
    }

    fn apricot_profile() -> LaboratoryIngredientProfile {
        LaboratoryIngredientProfile {
            slug: "apricot".into(),
            name: "Абрикос".into(),
            sweetness: Some(7.0),
            acidity: Some(5.0),
            bitterness: Some(1.0),
            umami: Some(1.0),
            aroma: Some(8.0),
            culinary_behaviors: vec![
                LaboratoryCulinaryBehavior {
                    title: Some("pairs_with_dairy".into()),
                    category: Some("pairing".into()),
                    effect: Some("complements".into()),
                    intensity: Some(0.9),
                    targets: vec!["cream".into(), "yogurt".into(), "mascarpone".into()],
                    ..Default::default()
                },
                LaboratoryCulinaryBehavior {
                    title: Some("pairs_with_nuts".into()),
                    effect: Some("complements".into()),
                    intensity: Some(0.9),
                    targets: vec!["almond".into(), "walnut".into(), "pistachio".into()],
                    ..Default::default()
                },
            ],
            ..Default::default()
        }
    }

    #[test]
    fn apricot_solo_is_sweet_sour() {
        let res = analyze_flavor(&[ing("apricot", "200")], &[apricot_profile()]);
        assert_eq!(res.flavor_result.dominant_profile, "sweet_sour");
        assert_eq!(res.flavor_result.sweetness, Some(7.0));
        assert_eq!(res.flavor_result.acidity, Some(5.0));
    }

    #[test]
    fn apricot_pairings_extracted() {
        let res = analyze_flavor(&[ing("apricot", "200")], &[apricot_profile()]);
        let names: Vec<_> = res
            .pairing_suggestions
            .iter()
            .map(|p| p.ingredient_name.clone())
            .collect();
        for expected in ["cream", "yogurt", "mascarpone", "almond", "walnut", "pistachio"] {
            assert!(names.iter().any(|n| n == expected), "missing {expected}");
        }
        // Score ≈ intensity 0.9 × 100 = 90
        let cream = res
            .pairing_suggestions
            .iter()
            .find(|p| p.ingredient_name == "cream")
            .unwrap();
        assert_eq!(cream.score, 90.0);
        assert_eq!(cream.source, "culinary_behavior");
    }

    #[test]
    fn pairings_skip_already_in_project() {
        let res = analyze_flavor(
            &[ing("apricot", "200"), ing("cream", "100")],
            &[apricot_profile()],
        );
        assert!(res
            .pairing_suggestions
            .iter()
            .all(|p| p.ingredient_name != "cream"));
    }

    #[test]
    fn weighted_average_respects_quantity() {
        let sour = LaboratoryIngredientProfile {
            slug: "lemon".into(),
            name: "Лимон".into(),
            sweetness: Some(1.0),
            acidity: Some(9.0),
            ..Default::default()
        };
        // 100g sweet apricot + 100g sour lemon → ~middle
        let r1 = analyze_flavor(
            &[ing("apricot", "100"), ing("lemon", "100")],
            &[apricot_profile(), sour.clone()],
        );
        assert!((r1.flavor_result.sweetness.unwrap() - 4.0).abs() < 0.01);

        // 200g apricot + 10g lemon → mostly apricot
        let r2 = analyze_flavor(
            &[ing("apricot", "200"), ing("lemon", "10")],
            &[apricot_profile(), sour],
        );
        assert!(r2.flavor_result.sweetness.unwrap() > 6.5);
    }

    #[test]
    fn no_profile_data_warns() {
        let res = analyze_flavor(&[ing("mystery", "100")], &[]);
        assert_eq!(res.flavor_result.dominant_profile, "unknown");
        assert!(res
            .warnings
            .iter()
            .any(|w| w.kind == "flavor_no_profile_data"));
    }

    #[test]
    fn empty_inputs_are_safe() {
        let res = analyze_flavor(&[], &[]);
        assert!(res.flavor_result.sweetness.is_none());
        assert!(res.warnings.is_empty());
        assert!(res.pairing_suggestions.is_empty());
    }
}
