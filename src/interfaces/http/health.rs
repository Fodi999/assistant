use crate::interfaces::http::state::AppState;
use axum::{extract::State, http::StatusCode, Json};
use serde_json::{json, Value};

/// GET /health — liveness. Never touches the database, so a database outage
/// does not make the orchestrator kill a healthy process.
pub async fn health_check() -> (StatusCode, Json<Value>) {
    (
        StatusCode::OK,
        Json(json!({
            "status": "healthy",
            "service": env!("CARGO_PKG_NAME"),
            "version": env!("CARGO_PKG_VERSION"),
        })),
    )
}

/// GET /ready — readiness. Fails with 503 while the database is unreachable.
pub async fn ready_check(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    match sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.pool)
        .await
    {
        Ok(_) => (StatusCode::OK, Json(json!({ "status": "ready" }))),
        Err(e) => {
            tracing::error!("Readiness check failed: {}", e);
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "status": "database_unavailable" })),
            )
        }
    }
}
