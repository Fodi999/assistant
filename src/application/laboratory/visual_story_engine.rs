//! Visual story engine — turns a project plan + process analysis into a
//! sequence of "scene frames" describing the visual narrative of the product
//! across cooking. Pure, deterministic, no I/O.
//!
//! Each frame answers the questions:
//!  * what does the product look like at this point?
//!  * which animation tokens are active?
//!  * what would I prompt an image model with to render it?
//!
//! In Step 9 we only build the *structure*. A follow-up step will use this
//! plan to call Gemini Image / Imagen and persist `image_url` per frame.
//!
//! Pipeline position:
//!   process_engine → flavor_engine → shelf_life_engine → **visual_story_engine**

use rust_decimal::prelude::ToPrimitive;
use serde::Serialize;
use uuid::Uuid;

use super::process_engine::{LaboratoryEffect, LaboratoryProcessAnalysis, LaboratoryStepEffects};
use super::types::{LabProcessStepDto, LabProjectIngredientDto};

// ─────────────────────────────────────────────────────────────────────────────
// Output DTOs
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Default)]
pub struct LaboratoryVisualStory {
    /// Inferred high-level product type ("sauce", "soup", "smoothie", …).
    pub product_type: Option<String>,
    /// Human-readable narrative summary (e.g. "Apricot → heat → juice → sauce").
    pub headline: String,
    /// Ordered scene frames.
    pub scenes: Vec<LaboratorySceneFrame>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LaboratorySceneFrame {
    /// Step this frame belongs to (None for the opening "raw" and closing
    /// "ready" frames that don't map to a concrete step).
    pub step_id: Option<Uuid>,
    /// Stable identifier for the scene type, used by the frontend to pick
    /// transitions, particles, presets.
    pub scene_key: String, // "raw" | "heated" | "juicing" | "browning" | "blended" | "set" | "ready" | "frozen" | "fermenting" | …
    /// Order in the story (0-based). Always monotonically increasing.
    pub order_index: i32,
    /// Short title for the frame (e.g. "Свежий абрикос").
    pub title: String,
    /// One-sentence description of what's happening visually.
    pub description: String,
    /// Visual tokens active at this frame (mirrors process_engine tokens).
    /// Used by the frontend ProductProcessScene to layer particles.
    pub visual_tokens: Vec<String>,
    /// Optional camera/composition hint for downstream image generation
    /// ("close-up of a saucepan, top-down").
    pub composition: String,
    /// Free-form prompt suitable for Gemini Image / Imagen. Stable & in
    /// English (image models are best in English).
    pub prompt_hint: String,
    /// Optional URL of a previously generated image (filled by a later step).
    pub image_url: Option<String>,
    /// How many adjacent technologically-identical steps were collapsed into
    /// this single frame (Laboratory v2 — keeps the visual narrative clean
    /// when a user repeats `heat 85°C 15min` two times in a row).
    /// `None` for the default case (== 1, single source step).
    /// `Some(n)` where `n >= 2` means the frame represents `n` repeated steps.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repeated_count: Option<i32>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Public entry
// ─────────────────────────────────────────────────────────────────────────────

/// Build a visual story from already-computed process analysis. Pure.
pub fn build_visual_story(
    ingredients: &[LabProjectIngredientDto],
    steps: &[LabProcessStepDto],
    process: &LaboratoryProcessAnalysis,
    target_product_type: Option<&str>,
) -> LaboratoryVisualStory {
    let product_type = target_product_type
        .map(|s| s.to_string())
        .or_else(|| infer_product_type(steps));
    let primary = primary_ingredient(ingredients);

    let mut scenes: Vec<LaboratorySceneFrame> = Vec::with_capacity(steps.len() + 2);
    let mut order = 0i32;

    // ── (1) Opening "raw" frame ────────────────────────────────────────────
    scenes.push(LaboratorySceneFrame {
        step_id: None,
        scene_key: "raw".into(),
        order_index: order,
        title: title_raw(&primary),
        description: description_raw(&primary, ingredients),
        visual_tokens: vec!["raw_ingredients".into()],
        composition: "top-down studio shot of fresh ingredients on a wooden board".into(),
        prompt_hint: prompt_raw(&primary, ingredients),
        image_url: None,
        repeated_count: None,
    });
    order += 1;

    // ── (2) One frame per step, in plan order ──────────────────────────────
    let mut sorted_steps: Vec<&LabProcessStepDto> = steps.iter().collect();
    sorted_steps.sort_by_key(|s| s.order_index);

    for step in sorted_steps.iter() {
        let step_effects = process.step_effects.iter().find(|e| e.step_id == step.id);
        let frame = scene_for_step(step, step_effects, &primary, order);
        scenes.push(frame);
        order += 1;
    }

    // ── (3) Closing "ready" frame ──────────────────────────────────────────
    scenes.push(LaboratorySceneFrame {
        step_id: None,
        scene_key: "ready".into(),
        order_index: order,
        title: title_ready(product_type.as_deref(), &primary),
        description: description_ready(product_type.as_deref(), &primary),
        visual_tokens: vec!["plated".into()],
        composition: "hero plating shot, soft natural light, shallow depth of field".into(),
        prompt_hint: prompt_ready(product_type.as_deref(), &primary),
        image_url: None,
        repeated_count: None,
    });

    // ── (4) Collapse adjacent technologically-identical step frames ───────
    //
    // After a user iterates on the recipe, the same step may appear back-to-
    // back (e.g. legacy data created before the duplicate-step guard, or two
    // steps that happen to produce the same scene_key + tokens because they
    // share temperature + technique). We merge them into a single frame and
    // record how many were folded together via `repeated_count`. The opening
    // "raw" and closing "ready" frames are never collapsed.
    let scenes = collapse_adjacent_identical(scenes);

    let headline = headline_for(&primary, product_type.as_deref(), &scenes);

    LaboratoryVisualStory {
        product_type,
        headline,
        scenes,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Per-step scene
// ─────────────────────────────────────────────────────────────────────────────

fn scene_for_step(
    step: &LabProcessStepDto,
    effects: Option<&LaboratoryStepEffects>,
    primary: &str,
    order: i32,
) -> LaboratorySceneFrame {
    let temp_c = step.temperature_c.as_ref().and_then(|d| d.to_f64());
    let duration = step.duration_min;
    let technique = step.technique.to_lowercase();

    // Pick the visually dominant effect for this step (highest intensity).
    let dominant: Option<&LaboratoryEffect> =
        effects.map(|e| e.effects.as_slice()).and_then(|fx| {
            fx.iter().max_by(|a, b| {
                a.intensity
                    .partial_cmp(&b.intensity)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        });

    let scene_key = scene_key_for(&technique, dominant);
    let mut tokens: Vec<String> = effects
        .map(|e| {
            let mut t: Vec<String> = e.effects.iter().map(|x| x.visual_token.clone()).collect();
            t.sort();
            t.dedup();
            t
        })
        .unwrap_or_default();
    if tokens.is_empty() {
        tokens.push(scene_key.clone());
    }

    let title = title_for_step(&technique, temp_c, dominant, primary);
    let description = description_for_step(&technique, temp_c, duration, dominant, primary);
    let composition = composition_for_scene(&scene_key);
    let prompt_hint = prompt_for_step(&scene_key, &technique, temp_c, primary);

    LaboratorySceneFrame {
        step_id: Some(step.id),
        scene_key,
        order_index: order,
        title,
        description,
        visual_tokens: tokens,
        composition,
        prompt_hint,
        image_url: None,
        repeated_count: None,
    }
}

/// Walks the scene list once and merges back-to-back frames whose
/// `(scene_key, technique-equivalent visual_tokens)` are identical. Opening
/// `raw` and closing `ready` frames are intentionally untouched.
fn collapse_adjacent_identical(scenes: Vec<LaboratorySceneFrame>) -> Vec<LaboratorySceneFrame> {
    let mut out: Vec<LaboratorySceneFrame> = Vec::with_capacity(scenes.len());
    for frame in scenes.into_iter() {
        let can_merge = frame.scene_key != "raw" && frame.scene_key != "ready";
        if can_merge {
            if let Some(prev) = out.last_mut() {
                let prev_can_merge = prev.scene_key != "raw" && prev.scene_key != "ready";
                if prev_can_merge
                    && prev.scene_key == frame.scene_key
                    && prev.visual_tokens == frame.visual_tokens
                {
                    // Fold this frame into the previous one.
                    prev.repeated_count = Some(prev.repeated_count.unwrap_or(1) + 1);
                    continue;
                }
            }
        }
        out.push(frame);
    }
    // Re-number order_index after collapses so the frontend timeline stays
    // contiguous (0..n).
    for (i, f) in out.iter_mut().enumerate() {
        f.order_index = i as i32;
    }
    out
}

fn scene_key_for(technique: &str, dominant: Option<&LaboratoryEffect>) -> String {
    // Prefer the visual outcome over the technique name where possible.
    if let Some(eff) = dominant {
        match eff.visual_token.as_str() {
            "browning" | "maillard" => return "browning".into(),
            "juice_release" => return "juicing".into(),
            "soften" => return "softening".into(),
            "smooth_mix" => return "blended".into(),
            "viscosity_up" => return "thickening".into(),
            "ice_crystals" => return "frozen".into(),
            "bubbles" => return "fermenting".into(),
            "safety_shield" => return "pasteurized".into(),
            "split" => return "split".into(),
            _ => {}
        }
    }
    match technique {
        "blend" | "puree" | "emulsify" | "whip" => "blended".into(),
        "freeze" | "chill" | "cool" => "chilled".into(),
        "strain" | "sieve" => "strained".into(),
        "ferment" => "fermenting".into(),
        "dry" | "dehydrate" => "drying".into(),
        "pasteurize" => "pasteurized".into(),
        "heat" | "boil" | "simmer" | "bake" | "roast" | "fry" | "sear" => "heated".into(),
        _ => "transforming".into(),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Copywriting helpers (Russian — UI is RU-first; future: localize)
// ─────────────────────────────────────────────────────────────────────────────

fn title_raw(primary: &str) -> String {
    if primary.is_empty() {
        "Свежие ингредиенты".into()
    } else {
        format!("Свежий {primary}")
    }
}

fn description_raw(primary: &str, ingredients: &[LabProjectIngredientDto]) -> String {
    let n = ingredients.len();
    if n == 0 {
        return "Подготовьте ингредиенты для дальнейшей обработки.".into();
    }
    if primary.is_empty() {
        return format!("На столе {n} ингредиент(ов), готовы к обработке.");
    }
    if n == 1 {
        format!("На столе свежий {primary}, готов к обработке.")
    } else {
        format!("На столе {primary} и ещё {} ингредиент(ов).", n - 1)
    }
}

fn title_for_step(
    technique: &str,
    temp_c: Option<f64>,
    dominant: Option<&LaboratoryEffect>,
    primary: &str,
) -> String {
    if let Some(eff) = dominant {
        return eff.label.clone();
    }
    let p = if primary.is_empty() {
        "продукт".to_string()
    } else {
        primary.to_string()
    };
    match (technique, temp_c) {
        ("heat" | "boil" | "simmer", Some(t)) => format!("Нагрев {p} до {t:.0}°C"),
        ("blend" | "puree", _) => format!("Пюрирование: {p}"),
        ("freeze", _) => format!("Заморозка: {p}"),
        ("ferment", _) => format!("Ферментация: {p}"),
        ("strain" | "sieve", _) => "Процеживание".into(),
        ("pasteurize", _) => "Пастеризация".into(),
        ("dry" | "dehydrate", _) => format!("Сушка: {p}"),
        _ => format!("Этап: {technique}"),
    }
}

fn description_for_step(
    technique: &str,
    temp_c: Option<f64>,
    duration: Option<i32>,
    dominant: Option<&LaboratoryEffect>,
    primary: &str,
) -> String {
    // Prefer the dominant effect's natural-language message — it's already nice.
    if let Some(eff) = dominant {
        if !eff.message.is_empty() {
            return eff.message.clone();
        }
    }
    let p = if primary.is_empty() {
        "продукт".to_string()
    } else {
        primary.to_string()
    };
    let temp = temp_c.map(|t| format!(" при {t:.0}°C")).unwrap_or_default();
    let dur = duration.map(|m| format!(" {m} мин")).unwrap_or_default();
    match technique {
        "heat" | "boil" | "simmer" => format!("Нагреваем {p}{temp}{dur} — текстура смягчается."),
        "blend" | "puree" => format!("Пюрируем {p} до однородной массы{dur}."),
        "freeze" => format!("Замораживаем {p}{temp} — образуются кристаллы льда."),
        "ferment" => format!("Ферментация {p}{dur} — появляются пузырьки и кислотность."),
        "strain" | "sieve" => "Процеживаем массу — убираем крупные частицы.".into(),
        "pasteurize" => format!("Пастеризация{temp}{dur} — повышается срок хранения."),
        "dry" | "dehydrate" => format!("Сушим {p}{temp} — испаряется влага, концентрируется вкус."),
        _ => format!("Технологический этап «{technique}»."),
    }
}

fn title_ready(product_type: Option<&str>, primary: &str) -> String {
    match product_type {
        Some("sauce") => format!("Готовый соус из {primary}"),
        Some("soup") => format!(
            "Готовый суп{}",
            if primary.is_empty() {
                String::new()
            } else {
                format!(" из {primary}")
            }
        ),
        Some("smoothie") => format!(
            "Смузи{}",
            if primary.is_empty() {
                String::new()
            } else {
                format!(" из {primary}")
            }
        ),
        Some("jam") => format!("Готовое варенье из {primary}"),
        Some("dessert") => "Готовый десерт".into(),
        Some("marinade") => "Готовый маринад".into(),
        Some("dressing") => "Готовая заправка".into(),
        Some("drink") => "Готовый напиток".into(),
        _ => "Готовый продукт".into(),
    }
}

fn description_ready(product_type: Option<&str>, primary: &str) -> String {
    match product_type {
        Some("sauce") => format!("Бархатистый соус из {primary} — насыщенный цвет и аромат."),
        Some("soup") => "Горячий суп подаётся в глубокой тарелке.".into(),
        Some("smoothie") => "Холодный смузи в высоком стакане.".into(),
        Some("jam") => format!("Густое варенье из {primary} в стеклянной банке."),
        Some("dessert") => "Десерт на тарелке, готовый к подаче.".into(),
        Some("marinade") => "Маринад готов к использованию.".into(),
        Some("dressing") => "Заправка эмульгирована и блестит.".into(),
        _ => "Продукт готов к подаче.".into(),
    }
}

fn headline_for(primary: &str, pt: Option<&str>, scenes: &[LaboratorySceneFrame]) -> String {
    let mid: Vec<&str> = scenes
        .iter()
        .filter(|s| s.scene_key != "raw" && s.scene_key != "ready")
        .map(|s| s.scene_key.as_str())
        .collect();
    let mid_str = if mid.is_empty() {
        "обработка".to_string()
    } else {
        mid.join(" → ")
    };
    let target = pt.unwrap_or("продукт");
    if primary.is_empty() {
        format!("сырьё → {mid_str} → {target}")
    } else {
        format!("{primary} → {mid_str} → {target}")
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Composition + image prompts (English — for Gemini / Imagen)
// ─────────────────────────────────────────────────────────────────────────────

fn composition_for_scene(scene_key: &str) -> String {
    match scene_key {
        "raw" => "top-down studio shot, wooden board, soft window light".into(),
        "heated" | "juicing" | "softening" | "browning" | "thickening" => {
            "close-up of a stainless steel saucepan on stove, slight steam".into()
        }
        "blended" => "close-up of a glossy puree in a high-speed blender jar".into(),
        "frozen" | "chilled" => "macro shot of frosted surface, cool blue light".into(),
        "fermenting" => "side view of a glass jar with bubbles rising, warm light".into(),
        "drying" => "macro of dehydrated pieces on parchment, warm tone".into(),
        "pasteurized" => "labelled jar on a pastel background, clean studio".into(),
        "ready" => "hero plating shot, soft natural light, shallow depth of field".into(),
        _ => "studio close-up, neutral background".into(),
    }
}

fn prompt_raw(primary: &str, ingredients: &[LabProjectIngredientDto]) -> String {
    let names: Vec<String> = ingredients
        .iter()
        .take(4)
        .map(|i| i.ingredient_slug.replace('_', " "))
        .collect();
    let list = if names.is_empty() {
        "fresh ingredients".to_string()
    } else {
        names.join(", ")
    };
    let p = if primary.is_empty() {
        "fresh produce".to_string()
    } else {
        primary.replace('_', " ")
    };
    format!(
        "photo, fresh {p}, {list} on a rustic wooden board, top-down, soft natural light, food photography, 35mm, shallow depth of field"
    )
}

fn prompt_for_step(scene_key: &str, technique: &str, temp_c: Option<f64>, primary: &str) -> String {
    let p = if primary.is_empty() {
        "ingredient".to_string()
    } else {
        primary.replace('_', " ")
    };
    let temp = temp_c.map(|t| format!(" at {t:.0}°C")).unwrap_or_default();
    match scene_key {
        "heated" => format!("photo, {p} heated{temp} in a saucepan, gentle steam rising, glossy surface, food photography"),
        "juicing" => format!("photo, soft {p} releasing bright juices in a saucepan, droplets pooling, food photography"),
        "softening" => format!("photo, {p} softening and breaking down into a tender mass, warm light, food photography"),
        "browning" => format!("photo, {p} caramelising{temp}, deep amber color, glossy bubbles, food photography"),
        "thickening" => format!("photo, glossy {p} sauce thickening on a wooden spoon, drips slowly, food photography"),
        "blended" => format!("photo, smooth velvety {p} puree in a blender, shiny surface, food photography"),
        "frozen" | "chilled" => format!("photo, {p} preparation chilled, frosted surface, cool blue tone, macro"),
        "fermenting" => format!("photo, {p} ferment in a glass jar with rising bubbles, warm tone, side view"),
        "drying" => format!("photo, dehydrated {p} pieces on parchment paper, soft warm light, macro"),
        "pasteurized" => format!("photo, labelled glass jar of {p} preserve on a pastel background, studio shot"),
        "split" => format!("photo, {p} sauce showing slight separation between fat and water phases, macro detail"),
        _ => format!("photo, {p} during {technique}{temp}, food photography, soft light"),
    }
}

fn prompt_ready(product_type: Option<&str>, primary: &str) -> String {
    let p = if primary.is_empty() {
        "ingredient".to_string()
    } else {
        primary.replace('_', " ")
    };
    match product_type {
        Some("sauce") => format!("photo, glossy {p} sauce in a small ceramic bowl, ribbon of sauce on a spoon, hero shot, food photography, soft natural light"),
        Some("soup") => format!("photo, hot {p} soup in a deep bowl, steam, hero plating, food photography"),
        Some("smoothie") => format!("photo, cold {p} smoothie in a tall glass with straw, condensation, hero shot"),
        Some("jam") => format!("photo, thick {p} jam in a glass jar with a spoon, vintage label, soft light"),
        Some("dessert") => format!("photo, plated {p} dessert with garnish, hero shot, fine dining"),
        Some("marinade") => format!("photo, {p} marinade in a glass jug, herbs and spices around, top-down"),
        Some("dressing") => format!("photo, emulsified {p} dressing in a small jar, glossy surface, top-down"),
        Some("drink") => format!("photo, refreshing {p} drink in a glass with ice, condensation, hero shot"),
        _ => format!("photo, finished {p} product on a clean plate, hero plating, food photography"),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Inference helpers
// ─────────────────────────────────────────────────────────────────────────────

fn infer_product_type(steps: &[LabProcessStepDto]) -> Option<String> {
    let techniques: Vec<String> = steps.iter().map(|s| s.technique.to_lowercase()).collect();
    let has = |t: &str| techniques.iter().any(|x| x == t);
    if has("ferment") {
        Some("ferment".into())
    } else if has("freeze") {
        Some("frozen".into())
    } else if has("blend") || has("puree") {
        if has("heat") || has("boil") || has("simmer") {
            Some("sauce".into())
        } else {
            Some("smoothie".into())
        }
    } else if has("heat") || has("boil") || has("simmer") {
        Some("sauce".into())
    } else {
        None
    }
}

/// Pick the "primary" ingredient for narrative purposes — the one tagged
/// "base" if present, otherwise the heaviest, otherwise the first.
fn primary_ingredient(ingredients: &[LabProjectIngredientDto]) -> String {
    if let Some(base) = ingredients
        .iter()
        .find(|i| i.role.as_deref() == Some("base"))
    {
        return base.ingredient_slug.replace('_', " ");
    }
    let heaviest = ingredients.iter().max_by(|a, b| {
        a.quantity
            .to_f64()
            .unwrap_or(0.0)
            .partial_cmp(&b.quantity.to_f64().unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    if let Some(h) = heaviest {
        return h.ingredient_slug.replace('_', " ");
    }
    String::new()
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

    fn ing(slug: &str, qty: &str, role: Option<&str>) -> LabProjectIngredientDto {
        LabProjectIngredientDto {
            id: Uuid::new_v4(),
            ingredient_slug: slug.into(),
            quantity: Decimal::from_str(qty).unwrap(),
            unit: "g".into(),
            role: role.map(String::from),
            sort_order: 0,
            notes: None,
            created_at: OffsetDateTime::now_utc(),
            merged: None,
        }
    }

    fn step(order: i32, technique: &str, t: Option<f64>) -> LabProcessStepDto {
        LabProcessStepDto {
            id: Uuid::new_v4(),
            order_index: order,
            technique: technique.into(),
            temperature_c: t.and_then(Decimal::from_f64_retain),
            duration_min: Some(15),
            target_slugs: vec![],
            notes: None,
            created_at: OffsetDateTime::now_utc(),
        }
    }

    #[test]
    fn empty_inputs_produce_raw_and_ready() {
        let story = build_visual_story(&[], &[], &LaboratoryProcessAnalysis::default(), None);
        assert_eq!(story.scenes.len(), 2);
        assert_eq!(story.scenes[0].scene_key, "raw");
        assert_eq!(story.scenes.last().unwrap().scene_key, "ready");
    }

    #[test]
    fn sauce_pipeline_produces_ordered_frames() {
        let ings = vec![ing("apricot", "300", Some("base"))];
        let steps = vec![step(1, "heat", Some(85.0)), step(2, "blend", None)];
        let story = build_visual_story(
            &ings,
            &steps,
            &LaboratoryProcessAnalysis::default(),
            Some("sauce"),
        );
        assert_eq!(story.scenes.len(), 4); // raw + 2 + ready
        assert_eq!(story.scenes[0].scene_key, "raw");
        assert_eq!(story.scenes[1].scene_key, "heated");
        assert_eq!(story.scenes[2].scene_key, "blended");
        assert_eq!(story.scenes[3].scene_key, "ready");
        // Scene order is monotonic.
        for w in story.scenes.windows(2) {
            assert!(w[0].order_index < w[1].order_index);
        }
        assert_eq!(story.product_type.as_deref(), Some("sauce"));
        assert!(story.headline.contains("apricot"));
    }

    #[test]
    fn raw_prompt_mentions_primary_ingredient() {
        let ings = vec![ing("strawberry", "200", Some("base"))];
        let story = build_visual_story(
            &ings,
            &[],
            &LaboratoryProcessAnalysis::default(),
            Some("sauce"),
        );
        assert!(story.scenes[0]
            .prompt_hint
            .to_lowercase()
            .contains("strawberry"));
    }

    #[test]
    fn step_inherits_dominant_effect_label_when_present() {
        use crate::application::laboratory::process_engine::{
            LaboratoryEffect, LaboratoryStepEffects,
        };
        let ings = vec![ing("apricot", "300", Some("base"))];
        let s = step(1, "heat", Some(85.0));
        let analysis = LaboratoryProcessAnalysis {
            step_effects: vec![LaboratoryStepEffects {
                step_id: s.id,
                order_index: 1,
                technique: "heat".into(),
                temperature_c: Some(85.0),
                duration_min: Some(15),
                effects: vec![LaboratoryEffect {
                    ingredient_slug: Some("apricot".into()),
                    ingredient_name: Some("apricot".into()),
                    effect_type: "moisture_release".into(),
                    visual_token: "juice_release".into(),
                    label: "Абрикос — выделение сока".into(),
                    intensity: 0.8,
                    confidence: 0.9,
                    trigger_temperature_c: Some(60.0),
                    actual_temperature_c: Some(85.0),
                    message: "При нагреве абрикос выделяет сок.".into(),
                }],
            }],
            ..Default::default()
        };
        let story = build_visual_story(&ings, &[s], &analysis, Some("sauce"));
        assert_eq!(story.scenes[1].scene_key, "juicing");
        assert_eq!(story.scenes[1].title, "Абрикос — выделение сока");
        assert!(story.scenes[1]
            .visual_tokens
            .contains(&"juice_release".to_string()));
    }

    #[test]
    fn infer_product_type_blend_only_is_smoothie() {
        let steps = vec![step(1, "blend", None)];
        let pt = infer_product_type(&steps);
        assert_eq!(pt.as_deref(), Some("smoothie"));
    }

    #[test]
    fn infer_product_type_heat_then_blend_is_sauce() {
        let steps = vec![step(1, "heat", Some(85.0)), step(2, "blend", None)];
        let pt = infer_product_type(&steps);
        assert_eq!(pt.as_deref(), Some("sauce"));
    }

    #[test]
    fn adjacent_identical_step_frames_collapse_with_repeated_count() {
        // Two identical "heat 85°C" steps in a row → single visual frame
        // tagged with `repeated_count = 2`. Opening/closing untouched.
        let ings = vec![ing("apricot", "300", Some("base"))];
        let steps = vec![step(1, "heat", Some(85.0)), step(2, "heat", Some(85.0))];
        let story = build_visual_story(
            &ings,
            &steps,
            &LaboratoryProcessAnalysis::default(),
            Some("sauce"),
        );
        // raw + 1 collapsed step + ready
        assert_eq!(story.scenes.len(), 3, "scenes={:?}", story.scenes);
        assert_eq!(story.scenes[0].scene_key, "raw");
        assert_eq!(story.scenes[1].scene_key, "heated");
        assert_eq!(story.scenes[1].repeated_count, Some(2));
        assert_eq!(story.scenes[2].scene_key, "ready");
        // order_index re-numbered after collapse.
        assert_eq!(story.scenes[0].order_index, 0);
        assert_eq!(story.scenes[1].order_index, 1);
        assert_eq!(story.scenes[2].order_index, 2);
    }

    #[test]
    fn distinct_steps_do_not_collapse() {
        let ings = vec![ing("apricot", "300", Some("base"))];
        let steps = vec![step(1, "heat", Some(85.0)), step(2, "blend", None)];
        let story = build_visual_story(
            &ings,
            &steps,
            &LaboratoryProcessAnalysis::default(),
            Some("sauce"),
        );
        assert_eq!(story.scenes.len(), 4); // raw + heat + blend + ready
        assert!(story.scenes.iter().all(|s| s.repeated_count.is_none()));
    }
}
