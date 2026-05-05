mod scripts;
mod styles;
mod template;

use axum::response::{Html, IntoResponse};

pub async fn home_page() -> impl IntoResponse {
    Html(template::template(
        styles::styles(),
        &scripts::all_scripts(),
    ))
}
