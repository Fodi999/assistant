//! Helpers shared by the end-to-end API tests.
//!
//! The router talks to the database as the limited `beauty_app` role, exactly
//! like production, so row-level security is enforced. Fixtures that must bypass
//! it (adding an employee, disabling a user) go through the superuser test pool.

#![allow(dead_code)]

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    Router,
};
use beauty_backend::infrastructure::{
    AppEnv, Config, CorsConfig, DatabaseConfig, JwtConfig, ServerConfig,
};
use beauty_backend::interfaces::http::{create_router, AppState};
use beauty_backend::shared::Clock;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tower::ServiceExt;

pub const PASSWORD: &str = "correct horse battery";

pub fn test_config() -> Config {
    Config {
        env: AppEnv::Development,
        database: DatabaseConfig {
            url: "unused".to_string(),
            max_connections: 4,
        },
        server: ServerConfig {
            port: 0,
            rate_limit_per_second: 50,
        },
        jwt: JwtConfig {
            secret: "test-secret-that-is-long-enough-1234567890".to_string(),
            issuer: "test".to_string(),
            audience: "test".to_string(),
            access_token_ttl_minutes: 15,
            refresh_token_ttl_days: 30,
        },
        cors: CorsConfig {
            allowed_origins: vec![],
        },
    }
}

/// A router whose database connections run as the limited application role.
pub async fn app(superuser_pool: &PgPool) -> Router {
    app_with_clock(superuser_pool, Clock::system()).await
}

/// Same, with the application clock fixed at `now`.
pub async fn app_at(superuser_pool: &PgPool, now: time::OffsetDateTime) -> Router {
    app_with_clock(superuser_pool, Clock::fixed(now)).await
}

async fn app_with_clock(superuser_pool: &PgPool, clock: Clock) -> Router {
    let limited = PgPoolOptions::new()
        .max_connections(4)
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                sqlx::query("SET ROLE beauty_app")
                    .execute(&mut *conn)
                    .await?;
                Ok(())
            })
        })
        .connect_with((*superuser_pool.connect_options()).clone())
        .await
        .expect("limited pool");
    create_router(AppState::with_clock(test_config(), limited, clock))
}

pub async fn call(
    app: &Router,
    method: Method,
    uri: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut request = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let body = match body {
        Some(value) => {
            request = request.header("content-type", "application/json");
            Body::from(value.to_string())
        }
        None => Body::empty(),
    };
    let response = app
        .clone()
        .oneshot(request.body(body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

/// Registers a user; returns (access token, user id).
pub async fn sign_up(app: &Router, email: &str) -> (String, String) {
    let (status, body) = call(
        app,
        Method::POST,
        "/v1/auth/register",
        None,
        Some(json!({
            "email": email,
            "password": PASSWORD,
            "display_name": "Test User",
            "accepted_terms": true
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    (
        body["tokens"]["access_token"].as_str().unwrap().to_string(),
        body["user"]["id"].as_str().unwrap().to_string(),
    )
}

/// Creates a business for the token's user; returns its id.
pub async fn create_business(app: &Router, token: &str, name: &str) -> String {
    let (status, body) = call(
        app,
        Method::POST,
        "/v1/businesses",
        Some(token),
        Some(json!({ "name": name })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    body["id"].as_str().unwrap().to_string()
}

/// The caller's own staff card in a business (created with the business).
pub async fn my_staff_id(app: &Router, token: &str, business_id: &str) -> String {
    let (status, body) = call(
        app,
        Method::GET,
        &format!("/v1/businesses/{business_id}/staff"),
        Some(token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body.as_array()
        .unwrap()
        .iter()
        .find(|staff| staff["is_mine"] == true)
        .expect("own staff card")["id"]
        .as_str()
        .unwrap()
        .to_string()
}

/// Makes a user a member of a business with a staff card (bypasses the API:
/// invitations come with the team stage). Returns the new staff id.
pub async fn add_member(pool: &PgPool, business_id: &str, user_id: &str, role: &str) -> String {
    let membership: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO membership (business_id, user_id, role)
         VALUES ($1::text::uuid, $2::text::uuid, $3)
         RETURNING id",
    )
    .bind(business_id)
    .bind(user_id)
    .bind(role)
    .fetch_one(pool)
    .await
    .unwrap();
    let staff: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO staff_member (business_id, membership_id, display_name)
         VALUES ($1::text::uuid, $2, $3)
         RETURNING id",
    )
    .bind(business_id)
    .bind(membership)
    .bind(format!("{role} card"))
    .fetch_one(pool)
    .await
    .unwrap();
    staff.to_string()
}
