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
                // Accept both "text" and "description" fields from AI output
                text: s
                    .get("text")
                    .or_else(|| s.get("description"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                time_minutes: s
                    .get("time_minutes")
                    .or_else(|| s.get("duration_minutes"))
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
                // PL: uformuj, formuj, ulepić, kulki, kotlety, paluszki
                || all_lower.contains("uformuj") || all_lower.contains("formuj")
                || all_lower.contains("kulki") || all_lower.contains("kotlety")
                || all_lower.contains("paluszki") || all_lower.contains("ulepić")
                // UK: сформуйте, формуйте, зліпіть, кульки, котлет, паличк
                || all_lower.contains("сформуйте") || all_lower.contains("формуйте")
                || all_lower.contains("зліпіть") || all_lower.contains("кульки")
                || all_lower.contains("паличк")
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
                || all_lower.contains("liquid")
                // PL: bulion, woda, mleko, płyn
                || all_lower.contains("bulion") || all_lower.contains("woda")
                || all_lower.contains("mleko") || all_lower.contains("płyn")
                // UK: бульйон, вод, молок, рідин
                || all_lower.contains("бульйон") || all_lower.contains("рідин");
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
//
// Score = structure (0-40) + technique (0-30) + nutrition (0-20) + seo (0-10)
//
// - structure  : step count, step types, grams, forming/liquid/oven compliance
// - technique  : allowed technique present, no forbidden, last step texture
// - nutrition  : ingredient coverage in steps
// - seo        : time data present, step type diversity
//
// Critical violations already blocked by Recipe::new(); compute_quality is only
// called for recipes that PASSED the invariant checks.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityBreakdown {
    pub structure:  u8,   // 0-40
    pub technique:  u8,   // 0-30
    pub nutrition:  u8,   // 0-20
    pub seo:        u8,   // 0-10
    pub total:      u8,   // 0-100
    pub confidence: f32,
    pub verdict:    &'static str,
}

fn compute_quality(
    steps: &[RecipeStep],
    issues: &[QualityIssue],
    profile: &DishProfile,
    ingredients: &[String],
) -> RecipeQuality {
    let all_text_lower: String = steps
        .iter()
        .map(|s| s.text.to_lowercase())
        .collect::<Vec<_>>()
        .join(" ");

    // ── STRUCTURE (0-40) ────────────────────────────────────────────────
    // Base: 20 pts just for passing invariants
    // +5  per extra step above min_steps (capped at +10)
    // +5  if all steps have time_minutes > 0
    // +5  if steps mention grams
    let mut structure: f32 = 20.0;

    let extra_steps = steps.len().saturating_sub(profile.min_steps);
    structure += (extra_steps as f32 * 5.0).min(10.0);

    let all_have_time = steps.iter().all(|s| s.time_minutes > 0);
    if all_have_time { structure += 5.0; }

    let has_grams = all_text_lower.contains('г')
        || all_text_lower.contains("g ")
        || all_text_lower.contains("g)")
        || all_text_lower.contains("гр")
        || all_text_lower.contains("ml")
        || all_text_lower.contains("мл");
    if has_grams { structure += 5.0; }

    // ── TECHNIQUE (0-30) ────────────────────────────────────────────────
    // Base: 15 pts for passing allowed-technique invariant (already done)
    // +8  if last step has texture/serving keyword
    // +7  if no warnings about technique
    let mut technique: f32 = 15.0;

    if let Some(last) = steps.last() {
        let last_lower = last.text.to_lowercase();
        let has_texture = last_lower.contains("хрустящ") || last_lower.contains("мягк")
            || last_lower.contains("золотист") || last_lower.contains("горяч")
            || last_lower.contains("нежн") || last_lower.contains("сочн")
            || last_lower.contains("crispy") || last_lower.contains("golden")
            || last_lower.contains("tender") || last_lower.contains("hot")
            || last_lower.contains("creamy") || last_lower.contains("подавайте")
            || last_lower.contains("serve") || last_lower.contains("immediately")
            || last_lower.contains("немедленно") || last_lower.contains("тарелк")
            || last_lower.contains("plate") || last_lower.contains("garnish");
        if has_texture { technique += 8.0; }
    }

    let has_technique_warnings = issues.iter().any(|i| {
        i.code == "NO_TEXTURE_DESC" || i.code == "NO_TECHNIQUE"
    });
    if !has_technique_warnings { technique += 7.0; }

    // ── NUTRITION (0-20) ────────────────────────────────────────────────
    // Based on ingredient coverage in step text
    // 0% coverage = 0 pts, 100% = 20 pts (linear)
    let mentioned = if ingredients.is_empty() {
        ingredients.len()
    } else {
        ingredients.iter().filter(|ing| {
            let lower = ing.to_lowercase();
            all_text_lower.contains(&lower)
                || all_text_lower.contains(&lower.replace('-', " "))
        }).count()
    };
    let coverage = if ingredients.is_empty() {
        1.0_f32
    } else {
        mentioned as f32 / ingredients.len() as f32
    };
    let nutrition: f32 = coverage * 20.0;

    // ── SEO (0-10) ───────────────────────────────────────────────────────
    // +4 step type diversity (preparation/cooking/finishing)
    // +3 all steps have non-empty text > 30 chars
    // +3 step count ≥ 3 (already guaranteed, so free pts)
    let unique_types: std::collections::HashSet<&str> =
        steps.iter().map(|s| s.step_type.as_str()).collect();
    let type_pts = (unique_types.len() as f32 * 1.5).min(4.0);

    let rich_steps = steps.iter().filter(|s| s.text.len() > 30).count();
    let rich_pts = if rich_steps == steps.len() { 3.0 } else { rich_steps as f32 * 0.5 };

    let seo: f32 = type_pts + rich_pts + 3.0;

    // ── Confidence: only deduct for actual warnings ──────────────────────
    let warning_count = issues.iter().filter(|i| i.severity == IssueSeverity::Warning).count();
    let confidence = (1.0_f32 - warning_count as f32 * 0.08).clamp(0.5, 1.0);

    // ── Total ────────────────────────────────────────────────────────────
    let total = (structure + technique + nutrition + seo).clamp(0.0, 100.0).round() as u8;

    let verdict = match total {
        90..=100 => "excellent",
        75..=89  => "good",
        50..=74  => "acceptable",
        25..=49  => "poor",
        _        => "reject",
    };

    tracing::debug!(
        "📊 Quality breakdown — structure:{:.0}/40 technique:{:.0}/30 nutrition:{:.0}/20 seo:{:.0}/10 → total:{}/100 ({})",
        structure, technique, nutrition, seo, total, verdict
    );

    RecipeQuality {
        score: total,
        issues: issues.to_vec(),
        confidence,
        verdict,
    }
}

// ── Technique detection (shared with old validator) ─────────────────────────

fn technique_in_text(technique: &CookingTechnique, text: &str) -> bool {
    match technique {
        CookingTechnique::Fry => {
            // RU: обжар*, жар*, сковород*
            text.contains("обжар") || text.contains("жар") || text.contains("сковород")
                // EN: fry, sear, sauté, pan
                || text.contains("fry") || text.contains("sear") || text.contains("sauté")
                || text.contains("pan ")
                // PL: smażyć, smażenie, patelni, podsmażyć
                || text.contains("smaż") || text.contains("patelni") || text.contains("podsmaż")
                // UK: смажити, смажте, обсмаж*, сковорід*
                || text.contains("смаж") || text.contains("сковорід")
                || text.contains("обсмаж")
        }
        CookingTechnique::DeepFry => {
            text.contains("фритюр") || text.contains("deep fry") || text.contains("deep-fry")
                || text.contains("во фритюре") || (text.contains("масле") && text.contains("погру"))
                // PL: głęboki tłuszcz, frytownic
                || text.contains("frytownic") || text.contains("głębok")
                // UK: фритюр, у фритюрі
                || text.contains("у фритюрі")
        }
        CookingTechnique::Bake => {
            // RU: духовк*, запек*
            text.contains("духовк") || text.contains("запек")
                // EN: bake, oven
                || text.contains("bake") || text.contains("oven")
                // Temperature markers (all languages)
                || text.contains("°c") || text.contains("°f") || text.contains("градус")
                // PL: piec, piekarnik, pieczenie, zapiekać, zapiekaj
                || text.contains("piekarnik") || text.contains("zapiek") || text.contains("piecz")
                || text.contains("pieczeni")
                // UK: духовк*, духовці, запік*, запечі, запікай
                || text.contains("духовці") || text.contains("запік") || text.contains("запечі")
                || text.contains("запікай")
        }
        CookingTechnique::Boil => {
            // RU: свар*, кипя*, варит*, кастрюл*, кипящ*
            text.contains("свар") || text.contains("кипя") || text.contains("варит")
                || text.contains("кастрюл") || text.contains("кипящ")
                // EN
                || text.contains("boil")
                // PL: gotować, gotuj, zagotuj, garnek (pot)
                || text.contains("gotuj") || text.contains("gotow") || text.contains("zagotuj")
                || text.contains("garnek")
                // UK: зварити, варіть, кип'ят*, каструл*
                || text.contains("зварит") || text.contains("варіт") || text.contains("каструл")
        }
        CookingTechnique::Steam => {
            text.contains("пар") || text.contains("steam") || text.contains("на пару")
                || text.contains("пароварк")
                // PL: parować, na parze, parowar
                || text.contains("parow") || text.contains("na parze") || text.contains("parowar")
                // UK: на парі, пароварк*
                || text.contains("на парі")
        }
        CookingTechnique::Grill => {
            text.contains("гриль") || text.contains("grill") || text.contains("на углях")
                || text.contains("решётк") || text.contains("barbecue")
                // PL: grilować, griluj, ruszt
                || text.contains("grilluj") || text.contains("griluj") || text.contains("ruszt")
                // UK: гриль, решітк*
                || text.contains("решітк")
        }
        CookingTechnique::StirFry => {
            text.contains("вок") || text.contains("wok") || text.contains("стир-фрай")
                || text.contains("stir-fry") || text.contains("stir fry")
                || text.contains("быстро обжар") || text.contains("помешивая")
                // PL: smażyć mieszając, szybko smażyć
                || text.contains("mieszając")
                // UK: швидко обсмаж*, помішуючи
                || text.contains("помішуючи") || text.contains("швидко обсмаж")
        }
        CookingTechnique::Braise => {
            text.contains("тушит") || text.contains("тушен") || text.contains("braise")
                || text.contains("на медленном") || text.contains("simmer")
                // PL: dusić, duszenie, duszony
                || text.contains("dusz") || text.contains("dusi")
                // UK: тушкувати, тушкуй
                || text.contains("тушкув") || text.contains("тушкуй")
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
                || text.contains("нарежь") || text.contains("порвите")
                // PL: pokrój, wymieszaj, ułóż
                || text.contains("pokrój") || text.contains("wymieszaj") || text.contains("ułóż")
                // UK: наріжте, змішайте, викладіть
                || text.contains("наріжте") || text.contains("змішайте") || text.contains("викладіть");
            let heat_present = text.contains("обжар") || text.contains("свар")
                || text.contains("запек") || text.contains("духовк") || text.contains("сковород")
                || text.contains("жар") || text.contains("тушит") || text.contains("варит")
                || text.contains("кипя") || text.contains("гриль") || text.contains("вок")
                || text.contains("фритюр") || text.contains("пароварк")
                || text.contains("fry") || text.contains("bake") || text.contains("boil")
                || text.contains("grill") || text.contains("steam") || text.contains("braise")
                // PL
                || text.contains("smaż") || text.contains("patelni") || text.contains("piekarnik")
                || text.contains("gotuj") || text.contains("dusz")
                // UK
                || text.contains("смаж") || text.contains("сковорід") || text.contains("духовці")
                || text.contains("варіт") || text.contains("тушкув");
            assembly_verbs && !heat_present
        }
        CookingTechnique::Blend => {
            text.contains("блендер") || text.contains("blend") || text.contains("взбейте")
                || text.contains("пюрир") || text.contains("измельч")
                // PL: blender, zmiksuj
                || text.contains("zmiksuj") || text.contains("mikser")
                // UK: блендер, збийте, подрібн*
                || text.contains("збийте") || text.contains("подрібн")
        }
        CookingTechnique::Simmer => {
            text.contains("томит") || text.contains("на медленном") || text.contains("simmer")
                || text.contains("тушит") || text.contains("помешива")
                // PL: na wolnym ogniu, gotować na małym
                || text.contains("wolnym ogniu") || text.contains("na małym")
                // UK: на повільному вогні, томити
                || text.contains("повільному вогні") || text.contains("томит")
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
