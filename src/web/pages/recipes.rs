use crate::web::{language, pages::i18n};

pub fn list(lang: language::Lang) -> String {
    let text = i18n::pack(lang);
    let cards: String = text
        .recipes
        .iter()
        .map(|recipe| {
            format!(
                r#"<a href="/recipes/{}" class="recipe-card reveal">
  <div class="recipe-card-icon"><i class="bi {}"></i></div>
  <h3>{}</h3>
  <p><i class="bi bi-clock"></i> {} &bull; <i class="bi bi-bar-chart"></i> {}</p>
</a>"#,
                recipe.id, recipe.icon, recipe.name, recipe.time, recipe.difficulty
            )
        })
        .collect();
    format!(
        r#"<section class="page-header"><h1>{}</h1><p class="page-header-sub">{}</p></section><div class="recipes-grid">{}</div>"#,
        text.recipes_title, text.recipes_subtitle, cards
    )
}

pub fn detail(lang: language::Lang, id: &str) -> String {
    let text = i18n::pack(lang);
    let recipe = text.recipes.iter().find(|recipe| recipe.id == id);
    match recipe {
        None => format!(
            r#"<div class="not-found"><h1>404</h1><p>{}</p><a href="/recipes" class="btn"><i class="bi bi-arrow-left"></i> {}</a></div>"#,
            text.recipe_not_found, text.recipe_back
        ),
        Some(recipe) => {
            let ingredients: String = recipe
                .ingredients
                .iter()
                .map(|(name, quantity)| {
                    format!(
                        "<li><i class=\"bi bi-dot\"></i><strong>{}</strong>&ensp;{}</li>",
                        name, quantity
                    )
                })
                .collect();
            let steps: String = recipe
                .steps
                .iter()
                .enumerate()
                .map(|(index, step)| {
                    format!(
                        "<li><span class=\"step-num\">{}</span>{}</li>",
                        index + 1,
                        step
                    )
                })
                .collect();
            format!(
                r#"<a href="/recipes" class="back-link"><i class="bi bi-arrow-left"></i> {}</a>
<section class="page-header">
  <div style="font-size:2.5rem;color:var(--accent);margin-bottom:.5rem"><i class="bi {}"></i></div>
  <h1>{}</h1>
  <p><i class="bi bi-clock"></i> {} &bull; <i class="bi bi-bar-chart"></i> {}</p>
</section>
<section class="recipe-detail">
  <div class="ingredients"><h2><i class="bi bi-list-check"></i> {}</h2><ul>{}</ul></div>
  <div class="steps"><h2><i class="bi bi-card-list"></i> {}</h2><ol>{}</ol></div>
</section>"#,
                text.recipe_all,
                recipe.icon,
                recipe.name,
                recipe.time,
                recipe.difficulty,
                text.recipe_ingredients,
                ingredients,
                text.recipe_preparation,
                steps
            )
        }
    }
}
