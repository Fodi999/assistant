//! Resolver — transforms strategies into MealVariants using IngredientCache.
//!
//! 0 SQL at runtime. All data from in-memory cache (loaded at startup).

use crate::infrastructure::ingredient_cache::IngredientCache;
use super::goal::Goal;
use super::strategy::strategies_for;
use super::types::{MealIngredient, MealVariant};

/// Resolve strategies for a goal into fully calculated MealVariants.
pub async fn build_variants(
    goal: Goal,
    lang: &str,
    cache: &IngredientCache,
) -> Vec<MealVariant> {
    let strategies = strategies_for(goal);
    let mut variants = Vec::with_capacity(strategies.len());

    for strat in strategies {
        let mut ings = Vec::with_capacity(strat.picks.len());
        let mut total_kcal: u32 = 0;
        let mut total_prot: f32 = 0.0;
        let mut total_fat: f32 = 0.0;
        let mut total_carbs: f32 = 0.0;

        for pick in &strat.picks {
            let data = cache.get(pick.slug).await;

            let (kcal, prot, fat, carbs, image_url, name) = if let Some(ref d) = data {
                (
                    d.kcal_for(pick.grams),
                    d.protein_for(pick.grams),
                    d.fat_for(pick.grams),
                    d.carbs_for(pick.grams),
                    d.image_url.clone(),
                    d.name(lang).to_string(),
                )
            } else {
                // Slug not in catalog — use label, zero nutrition
                (0, 0.0, 0.0, 0.0, None, pick.labels.pick(lang))
            };

            total_kcal += kcal;
            total_prot += prot;
            total_fat += fat;
            total_carbs += carbs;

            ings.push(MealIngredient {
                name,
                amount: pick.amounts.pick(lang),
                calories: kcal,
                image_url,
            });
        }

        variants.push(MealVariant {
            level: strat.level.into(),
            emoji: strat.emoji.into(),
            title: strat.titles.pick(lang),
            short_description: strat.descs.pick(lang),
            calories: total_kcal,
            protein_g: total_prot.round() as u32,
            fat_g: total_fat.round() as u32,
            carbs_g: total_carbs.round() as u32,
            ingredients: ings,
        });
    }

    variants
}
