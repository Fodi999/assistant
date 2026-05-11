pub mod matter_lab;
pub mod matter_lab_styles;
pub mod matter_outliner;
pub mod matter_panels;
mod scripts;
mod styles;
mod template;

use axum::http::{header, HeaderValue};
use axum::response::{Html, IntoResponse, Response};

pub async fn home_page() -> impl IntoResponse {
    let css = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}",
        styles::styles(),
        matter_lab_styles::matter_lab_styles(),
        matter_lab_styles::matter_tools_styles(),
        matter_lab_styles::matter_panel_styles(),
        matter_lab_styles::matter_action_bar_styles(),
        matter_lab_styles::matter_status_styles(),
        matter_outliner::outliner_styles(),
    );
    let html = template::template(&css, &scripts::all_scripts());
    Response::builder()
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/html; charset=utf-8"),
        )
        .header(header::CACHE_CONTROL, HeaderValue::from_static("no-store"))
        .body(axum::body::Body::from(html))
        .unwrap()
}
