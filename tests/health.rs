use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use beauty_backend::infrastructure::{
    AppEnv, Config, CorsConfig, DatabaseConfig, JwtConfig, ServerConfig,
};
use beauty_backend::interfaces::http::{create_router, AppState};
use http_body_util::BodyExt;
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

fn test_state() -> AppState {
    let config = Config {
        env: AppEnv::Development,
        database: DatabaseConfig {
            url: "postgres://user:pass@localhost:5432/none".to_string(),
            max_connections: 1,
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
            allowed_origins: vec!["http://localhost:3000".to_string()],
        },
    };
    // Lazy pool: never connects unless a handler runs a query.
    let pool = PgPoolOptions::new()
        .connect_lazy(&config.database.url)
        .expect("lazy pool");
    AppState::new(config, pool)
}

#[tokio::test]
async fn health_is_ok_without_a_database() {
    let app = create_router(test_state());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "healthy");
    assert_eq!(json["service"], "beauty-backend");
}

#[tokio::test]
async fn unknown_route_is_404() {
    let app = create_router(test_state());

    let response = app
        .oneshot(Request::builder().uri("/nope").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
