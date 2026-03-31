// ─── Recipe — Domain Aggregate with Invariants ─────────────────────────────
//
// A Recipe CANNOT be constructed in an invalid state.
// `Recipe::new()` enforces all invariants at construction time.
// This replaces the post-generation validation pattern with a
// "make invalid states unrepresentable" approach.

use super::dish_classifier::{CookingTechnique, DishProfile, DishType};
use serde::{Deserialize, Serialize};

// ── RecipeStep value object ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeStep {
    pub step: usize,
    #[serde(rename = "type")]
    pub step_type: String, // "preparation" | "forming" | "cooking" | "finishing"
    pub text: String,
    pub time_minutes: u32,
}

// ── RecipeQuality — rich quality signal (replaces bool valid/invalid) ───────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeQuality {
    /// 0–100 composite quality score
    pub score: u8,
    /// Specific issues found (may be non-blocking)
    pub issues: Vec<QualityIssue>,
    /// AI confidence: 0.0 = garbage, 1.0 = perfect
    pub confidence: f32,
    /// Human-readable verdict
    pub verdict: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityIssue {
    pub severity: IssueSeverity,
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueSeverity {
    /// Blocks publication — must be fixed
    Critical,
    /// Degrades quality — should be fixed
    Warning,
    /// Minor imperfection — nice to fix
    Info,
}

// ── Recipe Domain Aggregate ─────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Recipe {
    pub steps: Vec<RecipeStep>,
    pub quality: RecipeQuality,
}

/// Errors returned when Recipe invariants are violated.
#[derive(Debug)]
pub struct RecipeInvariantError {
    pub violations: Vec<String>,
    pub raw_steps: serde_json::Value,
}

impl std::fmt::Display for RecipeInvariantError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Recipe invariant violations: {}", self.violations.join("; "))
    }
}

impl Recipe {
    /// Construct a Recipe from raw AI output + DishProfile.
    ///
    /// **Invariants enforced at construction:**
    /// 1. Steps must be a non-empty array
    /// 2. Step count ≥ profile.min_steps
    /// 3. Allowed technique must be present
    /// 4. No forbidden techniques detected
    /// 5. Forming step present when required
    /// 6. Liquid base present when required
    /// 7. Oven mentioned when required
    ///
    /// Returns `Err(RecipeInvariantError)` if ANY critical invariant fails.
    /// Warnings/info issues are captured in `RecipeQuality` but don't block.
    pub fn new(
        steps_json: &serde_json::Value,
        profile: &DishProfile,
        ingredients: &[String],
    ) -> Result<Self, RecipeInvariantError> {
        let mut critical_violations = Vec::new();
        let mut issues = Vec::new();

        // ── Parse steps ─────────────────────────────────────────────────
        let raw_steps = match steps_json.as_array() {
            Some(arr) if !arr.is_empty() => arr,
            _ => {
                return Err(RecipeInvariantError {
                    violations: vec!["steps is not a non-empty array".into()],
                    raw_steps: steps_json.clone(),
                });
            }
        };

        let steps: Vec<RecipeStep> = raw_steps
            .iter()
            .enumerate()
            .map(|(i, s)| RecipeStep {
                step: s.get("step").and_then(|v| v.as_u64()).unwrap_or((i + 1) as u64) as usize,
                step_type: s
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("cooking")
                    .to_string(),
                text: s
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                time_minutes: s
                    .get("time_minutes")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(5) as u32,
            })
            .collect();

        // Collect all text for analysis
        let all_text: String = steps.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join(" ");
        let all_lower = all_text.to_lowercase();

        // ── Invariant 1: minimum step count ─────────────────────────────
        if steps.len() < profile.min_steps {
            critical_violations.push(format!(
                "NOT_ENOUGH_STEPS: got {}, need ≥{} for {}",
                steps.len(), profile.min_steps, profile.type_label
            ));
            issues.push(QualityIssue {
                severity: IssueSeverity::Critical,
                code: "MIN_STEPS",
                message: format!(
                    "Only {} steps, {} requires at least {}",
                    steps.len(), profile.type_label, profile.min_steps
                ),
            });
        }

        // ── Invariant 2: allowed technique present ──────────────────────
        let has_allowed = profile.allowed_techniques.iter().any(|t| {
            technique_in_text(t, &all_lower)
        });
        if !has_allowed && profile.dish_type != DishType::Generic {
            critical_violations.push(format!(
                "NO_ALLOWED_TECHNIQUE for {}: expected {:?}",
                profile.type_label, profile.allowed_techniques
            ));
            issues.push(QualityIssue {
                severity: IssueSeverity::Critical,
                code: "NO_TECHNIQUE",
                message: format!(
                    "No allowed technique found for {}",
                    profile.type_label
                ),
            });
        }

        // ── Invariant 3: no forbidden techniques ────────────────────────
        for forbidden in &profile.forbidden_techniques {
            if technique_in_text(forbidden, &all_lower) {
                critical_violations.push(format!(
                    "FORBIDDEN_TECHNIQUE: '{}' in {}",
                    forbidden, profile.type_label
                ));
                issues.push(QualityIssue {
                    severity: IssueSeverity::Critical,
                    code: "FORBIDDEN_TECHNIQUE",
                    message: format!(
                        "Forbidden technique '{}' detected for '{}'",
                        forbidden, profile.type_label
                    ),
                });
            }
        }

        // ── Invariant 4: forming step when required ─────────────────────
        if profile.requires_forming {
            let has_forming = all_lower.contains("сформируйте")
                || all_lower.contains("формиру")
                || all_lower.contains("form ")
                || all_lower.contains("shape ")
                || all_lower.contains("скатайте")
                || all_lower.contains("вылеп")
                || all_lower.contains("раскатайте")
                || all_lower.contains("нарежьте на палочки")
                || all_lower.contains("палочки длиной")
                || all_lower.contains("котлет")
                || all_lower.contains("шарик")
                || all_lower.contains("лепёшк")
                || steps.iter().any(|s| s.step_type == "forming");
            if !has_forming {
                critical_violations.push(format!(
                    "MISSING_FORMING: '{}' requires forming step",
                    profile.type_label
                ));
                issues.push(QualityIssue {
                    severity: IssueSeverity::Critical,
                    code: "NO_FORMING",
                    message: format!(
                        "'{}' requires a forming step but none found",
                        profile.type_label
                    ),
                });
            }
        }

        // ── Invariant 5: liquid base when required ──────────────────────
        if profile.requires_liquid {
            let has_liquid = all_lower.contains("бульон")
                || all_lower.contains("вод")
                || all_lower.contains("broth")
                || all_lower.contains("water")
                || all_lower.contains("stock")
                || all_lower.contains("молок")
                || all_lower.contains("milk")
                || all_lower.contains("жидкост")
                || all_lower.contains("liquid");
            if !has_liquid {
                critical_violations.push(format!(
                    "MISSING_LIQUID: '{}' requires liquid base",
                    profile.type_label
                ));
                issues.push(QualityIssue {
                    severity: IssueSeverity::Critical,
                    code: "NO_LIQUID",
                    message: format!(
                        "'{}' requires liquid base (broth/water/milk)",
                        profile.type_label
                    ),
                });
            }
        }

        // ── Invariant 6: oven when required ─────────────────────────────
        if profile.requires_oven {
            let has_oven = all_lower.contains("духовк")
                || all_lower.contains("oven")
                || all_lower.contains("°c")
                || all_lower.contains("°f")
                || all_lower.contains("градус")
                || all_lower.contains("degree")
                || all_lower.contains("запек")
                || all_lower.contains("bake");
            if !has_oven {
                critical_violations.push(format!(
                    "MISSING_OVEN: '{}' requires oven with temperature",
                    profile.type_label
                ));
                issues.push(QualityIssue {
                    severity: IssueSeverity::Critical,
                    code: "NO_OVEN",
                    message: format!(
                        "'{}' requires oven step with temperature",
                        profile.type_label
                    ),
                });
            }
        }

        // ── Soft checks (warnings, not blockers) ────────────────────────

        // Steps mention grams
        let has_grams = all_text.contains('г')
            || all_text.contains("g)")
            || all_text.contains("g ")
            || all_text.contains("гр");
        if !has_grams {
            issues.push(QualityIssue {
                severity: IssueSeverity::Warning,
                code: "NO_GRAMS",
                message: "Steps don't mention ingredient grams".into(),
            });
        }

        // Last step describes texture
        if let Some(last) = steps.last() {
            let last_lower = last.text.to_lowercase();
            let has_texture = last_lower.contains("хрустящ")
                || last_lower.contains("мягк")
                || last_lower.contains("золотист")
                || last_lower.contains("горяч")
                || last_lower.contains("нежн")
                || last_lower.contains("сочн")
                || last_lower.contains("crispy")
                || last_lower.contains("golden")
                || last_lower.contains("tender")
                || last_lower.contains("hot")
                || last_lower.contains("creamy")
                || last_lower.contains("подавайте")
                || last_lower.contains("serve");
            if !has_texture {
                issues.push(QualityIssue {
                    severity: IssueSeverity::Warning,
                    code: "NO_TEXTURE_DESC",
                    message: "Last step doesn't describe texture/result".into(),
                });
            }
        }

        // Ingredients mentioned
        let mut missing: Vec<String> = Vec::new();
        for ing in ingredients {
            let ing_lower = ing.to_lowercase();
            let found = all_lower.contains(&ing_lower)
                || all_lower.contains(&ing_lower.replace('-', " "))
                || all_lower.contains(&ing_lower.replace('-', ""));
            if !found {
                missing.push(ing.clone());
            }
        }
        if !missing.is_empty() && missing.len() > ingredients.len() / 2 {
            issues.push(QualityIssue {
                severity: IssueSeverity::Warning,
                code: "MISSING_INGREDIENTS",
                message: format!("Ingredients not mentioned: {}", missing.join(", ")),
            });
        }

        // ── Critical violations → reject ────────────────────────────────
        if !critical_violations.is_empty() {
            return Err(RecipeInvariantError {
                violations: critical_violations,
                raw_steps: steps_json.clone(),
            });
        }

        // ── Compute quality score ───────────────────────────────────────
        let quality = compute_quality(&steps, &issues, profile, ingredients);

        Ok(Recipe { steps, quality })
    }

    /// Serialize steps back to JSON (for DB storage)
    pub fn steps_json(&self) -> serde_json::Value {
        serde_json::to_value(&self.steps).unwrap_or(serde_json::json!([]))
    }
}

// ── Quality Score Computation ───────────────────────────────────────────────

fn compute_quality(
    steps: &[RecipeStep],
    issues: &[QualityIssue],
    profile: &DishProfile,
    ingredients: &[String],
) -> RecipeQuality {
    let mut score: f32 = 100.0;
    let mut confidence: f32 = 1.0;

    // ── Penalties per issue severity ────────────────────────────────────
    for issue in issues {
        match issue.severity {
            IssueSeverity::Critical => {
                score -= 30.0; // shouldn't happen (we rejected above), but just in case
                confidence -= 0.3;
            }
            IssueSeverity::Warning => {
                score -= 10.0;
                confidence -= 0.1;
            }
            IssueSeverity::Info => {
                score -= 3.0;
                confidence -= 0.02;
            }
        }
    }

    // ── Bonus: extra steps (shows detail) ───────────────────────────────
    let extra_steps = steps.len().saturating_sub(profile.min_steps);
    score += (extra_steps as f32 * 3.0).min(12.0);

    // ── Bonus: all ingredients mentioned ────────────────────────────────
    let all_text_lower: String = steps.iter().map(|s| s.text.to_lowercase()).collect::<Vec<_>>().join(" ");
    let mentioned_count = ingredients.iter().filter(|ing| {
        let lower = ing.to_lowercase();
        all_text_lower.contains(&lower)
            || all_text_lower.contains(&lower.replace('-', " "))
    }).count();
    let ingredient_coverage = if ingredients.is_empty() {
        1.0
    } else {
        mentioned_count as f32 / ingredients.len() as f32
    };
    score += ingredient_coverage * 10.0;
    confidence *= 0.5 + ingredient_coverage * 0.5;

    // ── Bonus: step types diversity (preparation, forming, cooking, finishing) ──
    let unique_types: std::collections::HashSet<&str> = steps.iter().map(|s| s.step_type.as_str()).collect();
    score += (unique_types.len() as f32 * 2.0).min(8.0);

    // ── Bonus: time data present ────────────────────────────────────────
    let steps_with_time = steps.iter().filter(|s| s.time_minutes > 0).count();
    if steps_with_time == steps.len() {
        score += 5.0;
    }

    // ── Clamp ───────────────────────────────────────────────────────────
    let final_score = score.clamp(0.0, 100.0).round() as u8;
    let final_confidence = confidence.clamp(0.0, 1.0);

    let verdict = match final_score {
        90..=100 => "excellent",
        75..=89 => "good",
        50..=74 => "acceptable",
        25..=49 => "poor",
        _ => "reject",
    };

    RecipeQuality {
        score: final_score,
        issues: issues.to_vec(),
        confidence: final_confidence,
        verdict,
    }
}

// ── Technique detection (shared with old validator) ─────────────────────────

fn technique_in_text(technique: &CookingTechnique, text: &str) -> bool {
    match technique {
        CookingTechnique::Fry => {
            text.contains("обжар") || text.contains("жар") || text.contains("сковород")
                || text.contains("fry") || text.contains("sear") || text.contains("sauté")
                || text.contains("pan ")
        }
        CookingTechnique::DeepFry => {
            text.contains("фритюр") || text.contains("deep fry") || text.contains("deep-fry")
                || text.contains("во фритюре") || (text.contains("масле") && text.contains("погру"))
        }
        CookingTechnique::Bake => {
            text.contains("духовк") || text.contains("запек") || text.contains("bake")
                || text.contains("oven") || text.contains("°c") || text.contains("°f")
                || text.contains("градус")
        }
        CookingTechnique::Boil => {
            text.contains("свар") || text.contains("кипя") || text.contains("варит")
                || text.contains("boil") || text.contains("кастрюл") || text.contains("кипящ")
        }
        CookingTechnique::Steam => {
            text.contains("пар") || text.contains("steam") || text.contains("на пару")
                || text.contains("пароварк")
        }
        CookingTechnique::Grill => {
            text.contains("гриль") || text.contains("grill") || text.contains("на углях")
                || text.contains("решётк") || text.contains("barbecue")
        }
        CookingTechnique::StirFry => {
            text.contains("вок") || text.contains("wok") || text.contains("стир-фрай")
                || text.contains("stir-fry") || text.contains("stir fry")
                || text.contains("быстро обжар") || text.contains("помешивая")
        }
        CookingTechnique::Braise => {
            text.contains("тушит") || text.contains("тушен") || text.contains("braise")
                || text.contains("на медленном") || text.contains("simmer")
        }
        CookingTechnique::RawAssembly => {
            // Raw assembly = dish assembled without heat (salads, bowls).
            // Detected when assembly verbs present (cut, mix, arrange, toss)
            // and NO heat technique is found anywhere in the text.
            let assembly_verbs = text.contains("нарежьте") || text.contains("выложите")
                || text.contains("смешайте") || text.contains("собер")
                || text.contains("перемешайте") || text.contains("полейте")
                || text.contains("assemble") || text.contains("arrange")
                || text.contains("toss") || text.contains("mix")
                || text.contains("нарежь") || text.contains("порвите");
            let heat_present = text.contains("обжар") || text.contains("свар")
                || text.contains("запек") || text.contains("духовк") || text.contains("сковород")
                || text.contains("жар") || text.contains("тушит") || text.contains("варит")
                || text.contains("кипя") || text.contains("гриль") || text.contains("вок")
                || text.contains("фритюр") || text.contains("пароварк")
                || text.contains("fry") || text.contains("bake") || text.contains("boil")
                || text.contains("grill") || text.contains("steam") || text.contains("braise");
            assembly_verbs && !heat_present
        }
        CookingTechnique::Blend => {
            text.contains("блендер") || text.contains("blend") || text.contains("взбейте")
                || text.contains("пюрир") || text.contains("измельч")
        }
        CookingTechnique::Simmer => {
            text.contains("томит") || text.contains("на медленном") || text.contains("simmer")
                || text.contains("тушит") || text.contains("помешива")
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::dish_classifier::classify_dish;

    #[test]
    fn test_valid_stick_recipe() {
        let profile = classify_dish("Куриные палочки");
        let steps = serde_json::json!([
            {"step": 1, "type": "preparation", "text": "Нарежьте курицу (200г) на палочки длиной 8 см", "time_minutes": 5},
            {"step": 2, "type": "forming", "text": "Сформируйте палочки, обваляйте в муке (50г) и яйце (100г)", "time_minutes": 5},
            {"step": 3, "type": "cooking", "text": "Обжарьте на сковороде в масле 4 мин с каждой стороны до золотистой корочки", "time_minutes": 8},
            {"step": 4, "type": "finishing", "text": "Подавайте горячими — снаружи хрустящая корочка, внутри нежная курица", "time_minutes": 1}
        ]);
        let recipe = Recipe::new(&steps, &profile, &["chicken".into()]);
        assert!(recipe.is_ok());
        let r = recipe.unwrap();
        assert!(r.quality.score >= 70);
        // Verdict may be "excellent" or "good" depending on bonuses
        assert!(r.quality.verdict == "good" || r.quality.verdict == "excellent");
    }

    #[test]
    fn test_invalid_stick_recipe_boiled() {
        let profile = classify_dish("Куриные палочки");
        let steps = serde_json::json!([
            {"step": 1, "type": "cooking", "text": "Сварите курицу в кастрюле 20 мин", "time_minutes": 20}
        ]);
        let recipe = Recipe::new(&steps, &profile, &["chicken".into()]);
        assert!(recipe.is_err());
        let err = recipe.unwrap_err();
        assert!(err.violations.iter().any(|v| v.contains("NOT_ENOUGH_STEPS")));
        assert!(err.violations.iter().any(|v| v.contains("FORBIDDEN_TECHNIQUE")));
    }

    #[test]
    fn test_invalid_empty_steps() {
        let profile = classify_dish("Салат");
        let steps = serde_json::json!([]);
        let recipe = Recipe::new(&steps, &profile, &["cucumber".into()]);
        assert!(recipe.is_err());
    }

    #[test]
    fn test_soup_without_liquid() {
        let profile = classify_dish("Куриный суп");
        let steps = serde_json::json!([
            {"step": 1, "type": "preparation", "text": "Нарежьте курицу (200г) кубиками", "time_minutes": 5},
            {"step": 2, "type": "cooking", "text": "Обжарьте на сковороде 10 мин", "time_minutes": 10},
            {"step": 3, "type": "cooking", "text": "Добавьте овощи (150г) и тушите 5 мин", "time_minutes": 5},
            {"step": 4, "type": "finishing", "text": "Подавайте горячим", "time_minutes": 1}
        ]);
        let recipe = Recipe::new(&steps, &profile, &["chicken".into()]);
        assert!(recipe.is_err());
        let err = recipe.unwrap_err();
        assert!(err.violations.iter().any(|v| v.contains("MISSING_LIQUID")));
    }

    #[test]
    fn test_quality_score_excellent() {
        let profile = classify_dish("Салат Цезарь");
        let steps = serde_json::json!([
            {"step": 1, "type": "preparation", "text": "Нарежьте листья салата (100г), помидоры (80г) и cucumber (60г)", "time_minutes": 5},
            {"step": 2, "type": "preparation", "text": "Смешайте соус: масло оливковое (20г), лимонный сок, соль", "time_minutes": 3},
            {"step": 3, "type": "finishing", "text": "Выложите все компоненты в тарелку, полейте соусом. Подавайте холодным — свежий, хрустящий салат", "time_minutes": 2}
        ]);
        let recipe = Recipe::new(&steps, &profile, &["салат".into(), "помидор".into()]);
        assert!(recipe.is_ok());
        let r = recipe.unwrap();
        assert!(r.quality.score >= 50, "score was {}", r.quality.score);
        assert!(r.quality.confidence > 0.5);
    }

    #[test]
    fn test_casserole_without_oven() {
        let profile = classify_dish("Овощная запеканка");
        let steps = serde_json::json!([
            {"step": 1, "type": "preparation", "text": "Нарежьте овощи (300г)", "time_minutes": 5},
            {"step": 2, "type": "cooking", "text": "Обжарьте на сковороде 10 мин до мягкости", "time_minutes": 10},
            {"step": 3, "type": "cooking", "text": "Залейте соусом (100г)", "time_minutes": 2},
            {"step": 4, "type": "finishing", "text": "Подавайте горячей", "time_minutes": 1}
        ]);
        let recipe = Recipe::new(&steps, &profile, &["vegetables".into()]);
        assert!(recipe.is_err());
        let err = recipe.unwrap_err();
        assert!(err.violations.iter().any(|v| v.contains("MISSING_OVEN")));
    }
}
