use axum::{http::StatusCode, Json};
use serde_json::{json, Value};

/// GET /api/admin/version
pub async fn version() -> (StatusCode, Json<Value>) {
    let commit = first_env(&[
        "KOYEB_GIT_COMMIT_SHA",
        "KOYEB_GIT_SHA",
        "KOYEB_COMMIT_SHA",
        "GIT_COMMIT_SHA",
        "SOURCE_COMMIT",
    ])
    .unwrap_or_else(|| env!("BUILD_GIT_SHA").to_string());

    (
        StatusCode::OK,
        Json(json!({
            "ok": true,
            "status": "ok",
            "service": env!("CARGO_PKG_NAME"),
            "version": env!("CARGO_PKG_VERSION"),
            "commit": commit,
            "build_time_unix": env!("BUILD_TIME_UNIX"),
            "deployment_id": std::env::var("KOYEB_DEPLOYMENT_ID").ok(),
            "koyeb_app": std::env::var("KOYEB_APP_NAME").ok(),
            "koyeb_service": std::env::var("KOYEB_SERVICE_NAME").ok(),
        })),
    )
}

fn first_env(keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| std::env::var(key).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
