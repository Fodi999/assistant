//! Laboratory **process engine** — pure, deterministic transformer that
//! converts a structured cooking plan into a stream of frontend-friendly
//! visual events.
//!
//! Inputs
//! ------
//! * `Vec<LabProcessStepDto>`        — the user's plan (technique, °C, time, targets)
//! * `Vec<LabProjectIngredientDto>`  — what they actually picked (slug, qty, role)
//! * `Vec<LaboratoryIngredientProfile>` — catalog facts (behaviors, smoke point, …)
//!
//! Output: `LaboratoryProcessAnalysis` — per-step effects + warnings + a few
//! global rollups, ready to be persisted into `lab_project_analysis` and
//! consumed by the frontend animation layer.
//!
//! Design notes
//! ------------
//! * Pure, no I/O. Easy to unit-test, easy to call from the future
//!   `/analyze` endpoint without setting up a DB.
//! * Defensive: missing temperature, missing profile, empty `target_slugs`
//!   are all handled gracefully — they downgrade the result, never panic.
//! * MVP scope: heat-driven effects from `culinary_behaviors` + flat
//!   processing fields + technique-level fallback. No reaction kinetics,
//!   no time-dependent simulation yet.

use rust_decimal::prelude::ToPrimitive;
use serde::Serialize;
use std::collections::HashMap;
use uuid::Uuid;

use super::catalog_profile_adapter::{LaboratoryCulinaryBehavior, LaboratoryIngredientProfile};
use super::types::{LabProcessStepDto, LabProjectIngredientDto};

// ─────────────────────────────────────────────────────────────────────────────
// Output DTOs
// ─────────────────────────────────────────────────────────────────────────────

/// Final analysis bundle returned by the engine.
#[derive(Debug, Clone, Serialize, Default)]
pub struct LaboratoryProcessAnalysis {
    pub step_effects: Vec<LaboratoryStepEffects>,
    pub global_effects: Vec<LaboratoryEffect>,
    pub warnings: Vec<LaboratoryWarning>,
}

/// All effects produced by a single step, grouped under that step's id.
#[derive(Debug, Clone, Serialize)]
pub struct LaboratoryStepEffects {
    pub step_id: Uuid,
    pub order_index: i32,
    pub technique: String,
    pub temperature_c: Option<f64>,
    pub duration_min: Option<i32>,
    pub effects: Vec<LaboratoryEffect>,
}

/// One frontend-ready effect.
#[derive(Debug, Clone, Serialize)]
pub struct LaboratoryEffect {
    /// `None` when the effect is global to the step (e.g. a technique-only
    /// effect with no targets).
    pub ingredient_slug: Option<String>,
    pub ingredient_name: Option<String>,
    /// Domain-level effect category (`maillard`, `protein_change`, …).
    pub effect_type: String,
    /// Frontend animation token (`browning`, `juice_release`, …).
    pub visual_token: String,
    /// Short human label for the UI.
    pub label: String,
    /// 0.0 .. 1.0 — relative strength.
    pub intensity: f64,
    /// 0.0 .. 1.0 — how confident we are in the prediction.
    pub confidence: f64,
    /// Threshold from catalog (if any).
    pub trigger_temperature_c: Option<f64>,
    /// Step temperature actually used (if any).
    pub actual_temperature_c: Option<f64>,
    /// Free-form, locale-agnostic explanation. i18n later.
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LaboratoryWarning {
    pub kind: String,
    pub severity: String, // "info" | "warning" | "critical"
    pub ingredient_slug: Option<String>,
    pub message: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Engine entry point
// ─────────────────────────────────────────────────────────────────────────────

pub fn analyze_process(
    ingredients: &[LabProjectIngredientDto],
    steps: &[LabProcessStepDto],
    profiles: &[LaboratoryIngredientProfile],
) -> LaboratoryProcessAnalysis {
    // Index profiles by slug for O(1) lookup.
    let profiles_by_slug: HashMap<&str, &LaboratoryIngredientProfile> =
        profiles.iter().map(|p| (p.slug.as_str(), p)).collect();

    // Index ingredient names: prefer catalog name, fall back to slug.
    let display_names: HashMap<&str, String> = ingredients
        .iter()
        .map(|ing| {
            let slug = ing.ingredient_slug.as_str();
            let name = profiles_by_slug
                .get(slug)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| slug.to_string());
            (slug, name)
        })
        .collect();

    let all_slugs: Vec<&str> = ingredients
        .iter()
        .map(|i| i.ingredient_slug.as_str())
        .collect();

    let mut analysis = LaboratoryProcessAnalysis::default();

    // Sort steps by order_index defensively (they should already be ordered).
    let mut sorted_steps: Vec<&LabProcessStepDto> = steps.iter().collect();
    sorted_steps.sort_by_key(|s| s.order_index);

    for step in sorted_steps {
        let step_temp = step.temperature_c.as_ref().and_then(|d| d.to_f64());

        // Resolve which slugs this step targets.
        let targets: Vec<&str> = if step.target_slugs.is_empty() {
            all_slugs.clone()
        } else {
            step.target_slugs
                .iter()
                .map(|s| s.as_str())
                .filter(|s| all_slugs.contains(s))
                .collect()
        };

        let mut step_effects: Vec<LaboratoryEffect> = Vec::new();

        // ── (1) per-ingredient effects ────────────────────────────────────
        for slug in &targets {
            let profile = match profiles_by_slug.get(slug) {
                Some(p) => *p,
                // No profile for this slug — only technique fallback applies.
                None => continue,
            };
            let display = display_names
                .get(slug)
                .cloned()
                .unwrap_or_else(|| (*slug).to_string());

            // (1a) culinary_behaviors triggered by temperature.
            for behavior in &profile.culinary_behaviors {
                if let Some(eff) =
                    behavior_to_effect(behavior, slug, &display, step_temp, &mut analysis.warnings)
                {
                    step_effects.push(eff);
                }
            }

            // (1b) flat processing effects (Maillard / denature / smoke /
            //      vitamin loss) driven by step_temp.
            processing_effects_for(
                profile,
                slug,
                &display,
                step_temp,
                &mut step_effects,
                &mut analysis.warnings,
            );
        }

        // ── (2) technique-level effects (independent of catalog data) ─────
        if let Some(tech_effect) = technique_effect(&step.technique, &targets, &display_names) {
            step_effects.push(tech_effect);
        }

        analysis.step_effects.push(LaboratoryStepEffects {
            step_id: step.id,
            order_index: step.order_index,
            technique: step.technique.clone(),
            temperature_c: step_temp,
            duration_min: step.duration_min,
            effects: step_effects,
        });
    }

    // ── (3) global rollups (e.g. unused ingredient warning) ───────────────
    derive_global(ingredients, steps, &mut analysis.warnings);

    analysis
}

// ─────────────────────────────────────────────────────────────────────────────
// (1a) Behavior → effect
// ─────────────────────────────────────────────────────────────────────────────

fn behavior_to_effect(
    behavior: &LaboratoryCulinaryBehavior,
    slug: &str,
    display: &str,
    step_temp: Option<f64>,
    _warnings: &mut Vec<LaboratoryWarning>,
) -> Option<LaboratoryEffect> {
    // We need *some* identifying handle on the behavior.
    let effect_type = behavior
        .effect
        .clone()
        .or_else(|| behavior.title.clone())
        .or_else(|| behavior.category.clone())?;

    // Activation rule: if the behavior declares a temperature threshold AND
    // the step has a temperature, the step temp must reach the threshold.
    // If either is missing, we do NOT activate (avoids spurious effects on
    // non-thermal steps like `mix`).
    let activated = match (step_temp, behavior.temperature_c) {
        (Some(t), Some(threshold)) => t >= threshold,
        (Some(_), None) => {
            // Behavior is unconditional — only fire on thermal-ish triggers.
            matches!(
                behavior.trigger.as_deref(),
                Some("heat") | Some("temperature") | None
            )
        }
        _ => false,
    };
    if !activated {
        return None;
    }

    let label = label_for_effect(&effect_type);
    let visual_token = visual_token_for_effect(&effect_type).to_string();
    let intensity = behavior.intensity.unwrap_or(0.6).clamp(0.0, 1.0);
    let confidence = behavior.confidence.unwrap_or(0.7).clamp(0.0, 1.0);

    let message = match (step_temp, behavior.temperature_c) {
        (Some(t), Some(threshold)) => natural_effect_message(&effect_type, display, threshold, t),
        (Some(t), None) => natural_effect_message_no_threshold(&effect_type, display, t),
        _ => format!("{display}: активируется эффект «{label}».")
    };

    Some(LaboratoryEffect {
        ingredient_slug: Some(slug.to_string()),
        ingredient_name: Some(display.to_string()),
        effect_type,
        visual_token,
        label,
        intensity,
        confidence,
        trigger_temperature_c: behavior.temperature_c,
        actual_temperature_c: step_temp,
        message,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// (1b) Flat processing-effect rules
// ─────────────────────────────────────────────────────────────────────────────

fn processing_effects_for(
    profile: &LaboratoryIngredientProfile,
    slug: &str,
    display: &str,
    step_temp: Option<f64>,
    out: &mut Vec<LaboratoryEffect>,
    warnings: &mut Vec<LaboratoryWarning>,
) {
    let Some(t) = step_temp else { return };

    // Maillard / browning.
    if let Some(threshold) = profile.maillard_temp {
        if t >= threshold {
            out.push(make_effect(
                slug,
                display,
                "maillard",
                0.8,
                0.85,
                Some(threshold),
                Some(t),
                format!("{display}: реакция Майяра при {t}°C (порог {threshold}°C)."),
            ));
        }
    }

    // Protein denaturation.
    if let Some(threshold) = profile.protein_denature_temp {
        if t >= threshold {
            out.push(make_effect(
                slug,
                display,
                "protein_change",
                0.75,
                0.85,
                Some(threshold),
                Some(t),
                format!("{display}: денатурация белка при {t}°C (порог {threshold}°C)."),
            ));
        }
    }

    // Smoke point — warning + effect.
    if let Some(smoke) = profile.smoke_point {
        if t >= smoke {
            out.push(make_effect(
                slug,
                display,
                "smoke_risk",
                0.9,
                0.9,
                Some(smoke),
                Some(t),
                format!("{display}: достигнута точка дымления ({smoke}°C)."),
            ));
            warnings.push(LaboratoryWarning {
                kind: "smoke_point_exceeded".to_string(),
                severity: "warning".to_string(),
                ingredient_slug: Some(slug.to_string()),
                message: format!(
                    "{display} перегревается: {t}°C ≥ точки дымления {smoke}°C."
                ),
            });
        }
    }

    // Vitamin loss above ~100°C with low retention.
    if t >= 100.0 {
        if let Some(retention) = profile.vitamin_retention_pct {
            if retention < 70.0 {
                out.push(make_effect(
                    slug,
                    display,
                    "vitamin_loss",
                    ((100.0 - retention) / 100.0).clamp(0.2, 1.0),
                    0.7,
                    Some(100.0),
                    Some(t),
                    format!(
                        "{display}: потеря витаминов (сохраняется {retention}%)."
                    ),
                ));
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Natural-language effect messages
// ─────────────────────────────────────────────────────────────────────────────

fn natural_effect_message(effect_type: &str, display: &str, threshold: f64, actual: f64) -> String {
    match effect_type {
        "softening" | "texture_breakdown" => format!(
            "При {actual}°C {display} быстро размягчается: структура становится мягкой и пюреобразной."
        ),
        "moisture_release" | "juice_release" => format!(
            "При нагреве {display} выделяет сок, поэтому соус станет более жидким."
        ),
        "caramelization" => format!(
            "При {actual}°C сахара в {display} начинают карамелизоваться — появляется золотистый цвет и сладковатый аромат."
        ),
        "maillard" | "browning" => format!(
            "При {actual}°C в {display} запускается реакция Майяра: поверхность темнеет и появляется насыщенный аромат."
        ),
        "protein_denaturation" | "protein_change" => format!(
            "При {actual}°C белки в {display} денатурируют — текстура уплотняется."
        ),
        "thickening" | "starch_gelatinization" => format!(
            "При {actual}°C крахмал в {display} клейстеризуется — соус становится гуще."
        ),
        "vitamin_loss" | "nutrition_loss" => format!(
            "Температура {actual}°C разрушает часть витаминов в {display}. Добавьте лимонный сок после охлаждения для компенсации."
        ),
        "smoke_risk" | "smoke" => format!(
            "{display} перегревается: {actual}°C ≥ точки дымления {threshold}°C. Снизьте температуру."
        ),
        _ => format!(
            "При {actual}°C в {display} активируется эффект «{}».",
            label_for_effect(effect_type)
        ),
    }
}

fn natural_effect_message_no_threshold(effect_type: &str, display: &str, actual: f64) -> String {
    match effect_type {
        "softening" | "texture_breakdown" => format!(
            "При {actual}°C {display} размягчается."
        ),
        "moisture_release" | "juice_release" => format!(
            "При нагреве {display} выделяет сок — консистенция разжижается."
        ),
        "caramelization" => format!(
            "При {actual}°C начинается карамелизация {display}."
        ),
        _ => format!(
            "При {actual}°C в {display} активируется эффект «{}».",
            label_for_effect(effect_type)
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn make_effect(
    slug: &str,
    display: &str,
    effect_type: &str,
    intensity: f64,
    confidence: f64,
    trigger: Option<f64>,
    actual: Option<f64>,
    message: String,
) -> LaboratoryEffect {
    LaboratoryEffect {
        ingredient_slug: Some(slug.to_string()),
        ingredient_name: Some(display.to_string()),
        effect_type: effect_type.to_string(),
        visual_token: visual_token_for_effect(effect_type).to_string(),
        label: label_for_effect(effect_type),
        intensity: intensity.clamp(0.0, 1.0),
        confidence: confidence.clamp(0.0, 1.0),
        trigger_temperature_c: trigger,
        actual_temperature_c: actual,
        message,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// (2) Technique fallback
// ─────────────────────────────────────────────────────────────────────────────

fn technique_effect(
    technique: &str,
    targets: &[&str],
    display_names: &HashMap<&str, String>,
) -> Option<LaboratoryEffect> {
    let key = technique.trim().to_lowercase();
    let (effect_type, label, intensity) = match key.as_str() {
        "blend" | "puree" => ("smooth_mix", "однородная текстура", 0.8),
        "strain" | "sieve" => ("smooth_texture", "удаление волокон", 0.7),
        "cool" | "chill" => ("stabilization", "стабилизация", 0.5),
        "freeze" => ("freezing", "образование кристаллов льда", 0.85),
        "dry" | "dehydrate" => ("drying", "потеря влаги", 0.8),
        "ferment" => ("fermentation", "ферментация", 0.7),
        "pasteurize" => ("pasteurization", "пастеризация", 0.8),
        "emulsify" | "whip" => ("emulsification", "эмульсия", 0.75),
        _ => return None,
    };

    let (slug, name) = match targets.first() {
        Some(s) if targets.len() == 1 => (
            Some((*s).to_string()),
            display_names.get(*s).cloned(),
        ),
        _ => (None, None),
    };

    let scope = if targets.is_empty() {
        "ко всем ингредиентам".to_string()
    } else if targets.len() == 1 {
        display_names
            .get(targets[0])
            .cloned()
            .unwrap_or_else(|| targets[0].to_string())
    } else {
        format!("к {} ингредиентам", targets.len())
    };

    Some(LaboratoryEffect {
        ingredient_slug: slug,
        ingredient_name: name,
        effect_type: effect_type.to_string(),
        visual_token: visual_token_for_effect(effect_type).to_string(),
        label: label.to_string(),
        intensity,
        confidence: 0.9,
        trigger_temperature_c: None,
        actual_temperature_c: None,
        message: format!("Техника «{technique}» применена {scope}.",),
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// (3) Global rollups
// ─────────────────────────────────────────────────────────────────────────────

fn derive_global(
    ingredients: &[LabProjectIngredientDto],
    steps: &[LabProcessStepDto],
    warnings: &mut Vec<LaboratoryWarning>,
) {
    if steps.is_empty() {
        warnings.push(LaboratoryWarning {
            kind: "no_process_steps".into(),
            severity: "info".into(),
            ingredient_slug: None,
            message: "В проекте нет ни одного технологического шага.".into(),
        });
        return;
    }

    // Slugs explicitly mentioned in any step's target_slugs.
    let mut mentioned: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut has_global_step = false;
    for s in steps {
        if s.target_slugs.is_empty() {
            has_global_step = true;
        } else {
            for t in &s.target_slugs {
                mentioned.insert(t.as_str());
            }
        }
    }

    if !has_global_step {
        for ing in ingredients {
            if !mentioned.contains(ing.ingredient_slug.as_str()) {
                warnings.push(LaboratoryWarning {
                    kind: "ingredient_not_processed".into(),
                    severity: "warning".into(),
                    ingredient_slug: Some(ing.ingredient_slug.clone()),
                    message: format!(
                        "Ингредиент '{}' не упомянут ни в одном шаге.",
                        ing.ingredient_slug
                    ),
                });
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Visual token & label maps (single source of truth for the frontend layer)
// ─────────────────────────────────────────────────────────────────────────────

pub fn visual_token_for_effect(effect_type: &str) -> &'static str {
    match effect_type {
        "softening" | "texture_breakdown" | "smooth_texture" => "soften",
        "moisture_release" | "juice_release" => "juice_release",
        "caramelization" | "maillard" | "browning" => "browning",
        "emulsification" | "smooth_mix" => "smooth_mix",
        "thickening" => "viscosity_up",
        "separation" => "split",
        "protein_change" | "protein_denaturation" => "protein_change",
        "vitamin_loss" | "nutrition_loss" => "nutrition_loss",
        "smoke_risk" | "smoke" => "smoke",
        "cooling" | "stabilization" => "stabilize",
        "freezing" | "ice_crystals" => "ice_crystals",
        "drying" | "moisture_loss" | "shrink" => "shrink",
        "fermentation" | "bubbles" => "bubbles",
        "pasteurization" | "safety_shield" => "safety_shield",
        _ => "generic_change",
    }
}

fn label_for_effect(effect_type: &str) -> String {
    match effect_type {
        "softening" => "размягчение",
        "texture_breakdown" => "разрушение волокон",
        "smooth_texture" => "гладкая текстура",
        "moisture_release" | "juice_release" => "выделение сока",
        "caramelization" => "карамелизация",
        "maillard" | "browning" => "реакция Майяра",
        "emulsification" => "эмульгирование",
        "smooth_mix" => "однородная масса",
        "thickening" => "загущение",
        "separation" => "расслоение",
        "protein_change" | "protein_denaturation" => "денатурация белка",
        "vitamin_loss" | "nutrition_loss" => "потеря витаминов",
        "smoke_risk" | "smoke" => "точка дымления",
        "cooling" | "stabilization" => "охлаждение",
        "freezing" | "ice_crystals" => "замораживание",
        "drying" | "moisture_loss" | "shrink" => "усушка",
        "fermentation" | "bubbles" => "ферментация",
        "pasteurization" | "safety_shield" => "пастеризация",
        _ => "изменение",
    }
    .to_string()
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests — pure engine, easy to cover.
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;
    use time::OffsetDateTime;

    fn ing(slug: &str) -> LabProjectIngredientDto {
        LabProjectIngredientDto {
            id: Uuid::new_v4(),
            ingredient_slug: slug.to_string(),
            quantity: Decimal::from_str("100").unwrap(),
            unit: "g".into(),
            role: None,
            sort_order: 0,
            notes: None,
            created_at: OffsetDateTime::now_utc(),
        }
    }

    fn step(order: i32, technique: &str, temp: Option<f64>, targets: Vec<&str>) -> LabProcessStepDto {
        LabProcessStepDto {
            id: Uuid::new_v4(),
            order_index: order,
            technique: technique.into(),
            temperature_c: temp.map(|t| Decimal::from_f64_retain(t).unwrap()),
            duration_min: None,
            target_slugs: targets.into_iter().map(String::from).collect(),
            notes: None,
            created_at: OffsetDateTime::now_utc(),
        }
    }

    fn profile(slug: &str, behaviors: Vec<LaboratoryCulinaryBehavior>) -> LaboratoryIngredientProfile {
        LaboratoryIngredientProfile {
            slug: slug.into(),
            name: slug.into(),
            culinary_behaviors: behaviors,
            ..Default::default()
        }
    }

    #[test]
    fn behavior_fires_when_temp_reaches_threshold() {
        let p = profile(
            "apricot",
            vec![LaboratoryCulinaryBehavior {
                title: Some("moisture_release".into()),
                category: Some("texture".into()),
                trigger: Some("heat".into()),
                effect: Some("moisture_release".into()),
                intensity: Some(0.8),
                temperature_c: Some(60.0),
                confidence: Some(0.9),
                targets: vec![],
            }],
        );
        let analysis = analyze_process(
            &[ing("apricot")],
            &[step(0, "heat", Some(75.0), vec![])],
            &[p],
        );
        assert_eq!(analysis.step_effects.len(), 1);
        let effects = &analysis.step_effects[0].effects;
        assert!(effects.iter().any(|e| e.effect_type == "moisture_release"));
        assert!(effects.iter().any(|e| e.visual_token == "juice_release"));
    }

    #[test]
    fn behavior_does_not_fire_below_threshold() {
        let p = profile(
            "apricot",
            vec![LaboratoryCulinaryBehavior {
                effect: Some("moisture_release".into()),
                temperature_c: Some(60.0),
                trigger: Some("heat".into()),
                ..Default::default()
            }],
        );
        let analysis = analyze_process(
            &[ing("apricot")],
            &[step(0, "heat", Some(40.0), vec![])],
            &[p],
        );
        let effects = &analysis.step_effects[0].effects;
        assert!(!effects.iter().any(|e| e.effect_type == "moisture_release"));
    }

    #[test]
    fn smoke_point_emits_warning() {
        let p = LaboratoryIngredientProfile {
            slug: "olive_oil".into(),
            name: "olive_oil".into(),
            smoke_point: Some(190.0),
            ..Default::default()
        };
        let analysis = analyze_process(
            &[ing("olive_oil")],
            &[step(0, "fry", Some(220.0), vec![])],
            &[p],
        );
        assert!(analysis
            .warnings
            .iter()
            .any(|w| w.kind == "smoke_point_exceeded"));
    }

    #[test]
    fn target_slugs_scope_step_to_subset() {
        let apricot = profile(
            "apricot",
            vec![LaboratoryCulinaryBehavior {
                effect: Some("moisture_release".into()),
                temperature_c: Some(60.0),
                trigger: Some("heat".into()),
                ..Default::default()
            }],
        );
        let cream = profile(
            "cream",
            vec![LaboratoryCulinaryBehavior {
                effect: Some("protein_change".into()),
                temperature_c: Some(60.0),
                trigger: Some("heat".into()),
                ..Default::default()
            }],
        );
        let analysis = analyze_process(
            &[ing("apricot"), ing("cream")],
            &[step(0, "heat", Some(80.0), vec!["apricot"])],
            &[apricot, cream],
        );
        let effects = &analysis.step_effects[0].effects;
        // Only apricot effects should be present.
        assert!(effects
            .iter()
            .all(|e| e.ingredient_slug.as_deref() != Some("cream")));
        assert!(effects
            .iter()
            .any(|e| e.ingredient_slug.as_deref() == Some("apricot")));
    }

    #[test]
    fn technique_fallback_emits_visual_token() {
        let analysis = analyze_process(
            &[ing("apricot")],
            &[step(0, "blend", None, vec![])],
            &[],
        );
        let tokens: Vec<_> = analysis.step_effects[0]
            .effects
            .iter()
            .map(|e| e.visual_token.as_str())
            .collect();
        assert!(tokens.contains(&"smooth_mix"));
    }

    #[test]
    fn unprocessed_ingredient_warns() {
        let analysis = analyze_process(
            &[ing("apricot"), ing("cream")],
            &[step(0, "heat", Some(80.0), vec!["apricot"])],
            &[],
        );
        assert!(analysis
            .warnings
            .iter()
            .any(|w| w.kind == "ingredient_not_processed"
                && w.ingredient_slug.as_deref() == Some("cream")));
    }
}
