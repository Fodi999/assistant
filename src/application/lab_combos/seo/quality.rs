// ─── SEO Quality — page quality scoring ─────────────────────────────────────

use crate::application::lab_combos::recipe::RecipeQuality;

/// Score a lab combo page (0-5) — legacy format for DB compatibility.
pub fn quality_score(page: &crate::application::lab_combos::types::LabComboPage) -> i16 {
    let mut score: i16 = 0;

    if page.title.chars().count() <= 60 && !page.title.is_empty() {
        score += 1;
    }

    let desc_len = page.description.chars().count();
    if desc_len >= 80 && desc_len <= 155 {
        score += 1;
    }

    if page.intro.chars().count() >= 100 {
        score += 1;
    }

    if page.why_it_works.chars().count() >= 50 {
        score += 1;
    }

    let cook_steps = page.how_to_cook.as_array().map(|a| a.len()).unwrap_or(0);
    if cook_steps >= 3 {
        score += 1;
    }

    score.min(5)
}

/// Convert RecipeQuality (0-100) to page quality_score (0-5) for DB.
pub fn page_quality_score(rq: &RecipeQuality) -> i16 {
    match rq.score {
        90..=100 => 5,
        75..=89 => 4,
        60..=74 => 3,
        40..=59 => 2,
        20..=39 => 1,
        _ => 0,
    }
}
