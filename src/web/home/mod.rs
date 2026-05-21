// ── home/ — entry point for the CAD editor web page ─────────────────────────
pub mod layout;
mod scripts;

use axum::http::{header, HeaderValue};
use axum::response::{IntoResponse, Response};

pub async fn home_page() -> impl IntoResponse {
    let css = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        layout::styles::styles(),
        layout::matter_lab_styles::matter_lab_styles(),
        layout::matter_lab_styles::matter_tools_styles(),
        layout::matter_lab_styles::matter_toolbar_styles(),
        layout::matter_lab_styles::matter_panel_styles(),
        layout::matter_lab_styles::matter_action_bar_styles(),
        layout::matter_lab_styles::matter_status_styles(),
        layout::cad_side_panel_styles::cad_side_panel_styles(),
    );
    let html = layout::template::template(&css, &scripts::all_scripts());
    Response::builder()
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/html; charset=utf-8"),
        )
        .header(header::CACHE_CONTROL, HeaderValue::from_static("no-store"))
        .body(axum::body::Body::from(html))
        .unwrap()
}
