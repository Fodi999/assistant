//! Integration tests for `/telegram/webhook` and `/telegram/status`.
//!
//! These exercise the real Axum router (middleware, extractors, JSON
//! (de)serialization) end-to-end via `tower::ServiceExt::oneshot`, without
//! needing a live Postgres or a live Telegram bot:
//! - The pool is built with `connect_lazy`, which parses the URL but opens
//!   no TCP connection until a query actually runs.
//! - The webhook bodies used here (`{"update_id": 1}`, no `message` /
//!   `callback_query`) are valid Telegram updates that make zero DB queries
//!   and zero outbound Telegram API calls, so they exercise the secret-check
//!   and dispatch plumbing without touching the network.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use restaurant_backend::infrastructure::config::TelegramConfig;
use restaurant_backend::interfaces::telegram::router;
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

const SECRET_HEADER: &str = "X-Telegram-Bot-Api-Secret-Token";

fn lazy_pool() -> sqlx::PgPool {
    PgPoolOptions::new()
        .connect_lazy("postgres://user:pass@localhost/db")
        .expect("connect_lazy must not touch the network")
}

fn telegram_config(secret: &str) -> TelegramConfig {
    TelegramConfig {
        bot_token: "123456:fake-token-for-tests".to_string(),
        webhook_secret: Some(secret.to_string()),
        channel: "@svit_ikony".to_string(),
    }
}

fn empty_update_request(headers: &[(&str, &str)]) -> Request<Body> {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/webhook")
        .header("content-type", "application/json");
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    builder.body(Body::from(r#"{"update_id": 1}"#)).unwrap()
}

async fn json_body(response: axum::response::Response) -> serde_json::Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn webhook_rejects_missing_secret_header() {
    let app = router(lazy_pool(), Some(telegram_config("expected-secret")));

    let response = app.oneshot(empty_update_request(&[])).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn webhook_rejects_wrong_secret_header() {
    let app = router(lazy_pool(), Some(telegram_config("expected-secret")));

    let response = app
        .oneshot(empty_update_request(&[(SECRET_HEADER, "wrong-secret")]))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn webhook_accepts_matching_secret() {
    let app = router(lazy_pool(), Some(telegram_config("expected-secret")));

    let response = app
        .oneshot(empty_update_request(&[(SECRET_HEADER, "expected-secret")]))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn webhook_returns_503_when_telegram_disabled() {
    let app = router(lazy_pool(), None);

    let response = app.oneshot(empty_update_request(&[])).await.unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn status_reports_configured_true_with_channel() {
    let app = router(lazy_pool(), Some(telegram_config("secret")));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let json = json_body(response).await;
    assert_eq!(json["configured"], true);
    assert_eq!(json["channel"], "@svit_ikony");
    // Never leak the bot token or webhook secret through this endpoint.
    let raw = json.to_string();
    assert!(!raw.contains("fake-token-for-tests"));
    assert!(!raw.contains("secret"));
}

#[tokio::test]
async fn status_reports_configured_false_when_token_missing() {
    let app = router(lazy_pool(), None);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let json = json_body(response).await;
    assert_eq!(json["configured"], false);
    assert!(json["channel"].is_null());
}
