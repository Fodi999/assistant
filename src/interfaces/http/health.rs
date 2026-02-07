use axum::{http::StatusCode, Json};
use serde_json::{json, Value};

/// GET /health
/// Simple health check endpoint for load balancers and monitoring
pub async fn health_check() -> (StatusCode, Json<Value>) {
    (
        StatusCode::OK,
        Json(json!({
            "status": "healthy",
            "service": "restaurant-backend"
        })),
    )
}
