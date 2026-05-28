//! Axum handlers — рендерят страницы в HTML и отдают браузеру.

use axum::{extract::Query, response::Html};
use serde::Deserialize;

use crate::web::{shell, pages};

pub async fn home() -> Html<String> {
    Html(shell("Главная", &pages::home::render()))
}

#[derive(Deserialize)]
pub struct CatQuery { pub cat: Option<String> }

pub async fn menu(Query(q): Query<CatQuery>) -> Html<String> {
    let cat = q.cat.as_deref();
    let title = cat.unwrap_or("Меню");
    Html(shell(title, &pages::menu::render(cat)))
}

pub async fn recipes_list() -> Html<String> {
    Html(shell("Рецепты", &pages::recipes::list()))
}

pub async fn recipe_detail(
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Html<String> {
    Html(shell("Рецепт", &pages::recipes::detail(&id)))
}

pub async fn about() -> Html<String> {
    Html(shell("О шефе", &pages::about::render()))
}

pub async fn not_found() -> Html<String> {
    Html(shell("404", r#"<div class="page-header"><h1>404</h1><p>Страница не найдена</p><a href="/" class="btn btn-ghost">На главную</a></div>"#))
}
