//! End-to-end tests of the auth and business API through the real router.
//!
//! The service connects as the limited `beauty_app` role (like production), so
//! row-level security is enforced. Test data that must bypass it (making a user
//! an employee, disabling an account) goes through the superuser test pool.

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    Router,
};
use beauty_backend::infrastructure::{
    AppEnv, Config, CorsConfig, DatabaseConfig, JwtConfig, ServerConfig,
};
use beauty_backend::interfaces::http::{create_router, AppState};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tower::ServiceExt;

const PASSWORD: &str = "correct horse battery";

fn test_config() -> Config {
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
async fn app(superuser_pool: &PgPool) -> Router {
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
    create_router(AppState::new(test_config(), limited))
}

async fn call(
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

fn register_body(email: &str) -> Value {
    json!({
        "email": email,
        "password": PASSWORD,
        "display_name": "Anna",
        "accepted_terms": true,
        "device": { "platform": "ios", "app_version": "1.0.0" }
    })
}

/// Registers a user and returns (access token, refresh token, user id).
async fn sign_up(app: &Router, email: &str) -> (String, String, String) {
    let (status, body) = call(
        app,
        Method::POST,
        "/v1/auth/register",
        None,
        Some(register_body(email)),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    (
        body["tokens"]["access_token"].as_str().unwrap().to_string(),
        body["tokens"]["refresh_token"]
            .as_str()
            .unwrap()
            .to_string(),
        body["user"]["id"].as_str().unwrap().to_string(),
    )
}

#[sqlx::test(migrations = "./migrations")]
async fn register_login_and_me(pool: PgPool) {
    let app = app(&pool).await;
    let (access, _, user_id) = sign_up(&app, "Anna@Example.pl").await;

    // Email is normalised; a second registration is a conflict.
    let (status, _) = call(
        &app,
        Method::POST,
        "/v1/auth/register",
        None,
        Some(register_body("anna@example.pl")),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // Wrong password and unknown email look identical.
    for email in ["anna@example.pl", "nobody@example.pl"] {
        let (status, body) = call(
            &app,
            Method::POST,
            "/v1/auth/login",
            None,
            Some(json!({ "email": email, "password": "wrong password!" })),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["details"], "Invalid email or password");
    }

    let (status, body) = call(
        &app,
        Method::POST,
        "/v1/auth/login",
        None,
        Some(json!({ "email": "ANNA@example.pl", "password": PASSWORD })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["user"]["id"], user_id);

    let (status, body) = call(&app, Method::GET, "/v1/me", Some(&access), None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["user"]["email"], "anna@example.pl");
    assert_eq!(body["memberships"], json!([]));

    // No token, garbage token.
    let (status, _) = call(&app, Method::GET, "/v1/me", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = call(&app, Method::GET, "/v1/me", Some("garbage"), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "./migrations")]
async fn registration_validates_input_and_records_consent(pool: PgPool) {
    let app = app(&pool).await;

    let mut short = register_body("a@example.pl");
    short["password"] = json!("short");
    let mut no_terms = register_body("b@example.pl");
    no_terms["accepted_terms"] = json!(false);
    let bad_email = register_body("not-an-email");

    for body in [short, no_terms, bad_email] {
        let (status, _) = call(&app, Method::POST, "/v1/auth/register", None, Some(body)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    sign_up(&app, "c@example.pl").await;
    let consents: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM consent c JOIN users u ON u.id = c.user_id
         WHERE u.email = 'c@example.pl' AND c.granted",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(consents, 2, "terms and privacy are recorded");

    let devices: i64 = sqlx::query_scalar("SELECT count(*) FROM device")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(devices, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn refresh_rotates_and_reuse_revokes_the_login(pool: PgPool) {
    let app = app(&pool).await;
    let (_, first, _) = sign_up(&app, "anna@example.pl").await;

    let (status, body) = call(
        &app,
        Method::POST,
        "/v1/auth/refresh",
        None,
        Some(json!({ "refresh_token": first })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let second = body["refresh_token"].as_str().unwrap().to_string();
    assert_ne!(second, first);

    // Replaying the used-up token is rejected...
    let (status, _) = call(
        &app,
        Method::POST,
        "/v1/auth/refresh",
        None,
        Some(json!({ "refresh_token": first })),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // ...and it also killed the legitimate newer token (theft response).
    let (status, _) = call(
        &app,
        Method::POST,
        "/v1/auth/refresh",
        None,
        Some(json!({ "refresh_token": second })),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Only the hash is stored, never the token itself.
    let stored: i64 =
        sqlx::query_scalar("SELECT count(*) FROM refresh_token WHERE token_hash = $1")
            .bind(&second)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(stored, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn logout_ends_the_login_and_never_leaks(pool: PgPool) {
    let app = app(&pool).await;
    let (_, refresh, _) = sign_up(&app, "anna@example.pl").await;

    for token in [refresh.as_str(), "unknown-token"] {
        let (status, _) = call(
            &app,
            Method::POST,
            "/v1/auth/logout",
            None,
            Some(json!({ "refresh_token": token })),
        )
        .await;
        assert_eq!(status, StatusCode::NO_CONTENT);
    }

    let (status, _) = call(
        &app,
        Method::POST,
        "/v1/auth/refresh",
        None,
        Some(json!({ "refresh_token": refresh })),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "./migrations")]
async fn repeated_wrong_passwords_lock_sign_in(pool: PgPool) {
    let app = app(&pool).await;
    sign_up(&app, "anna@example.pl").await;

    let wrong = json!({ "email": "anna@example.pl", "password": "wrong password!" });
    for _ in 0..5 {
        let (status, _) = call(
            &app,
            Method::POST,
            "/v1/auth/login",
            None,
            Some(wrong.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    let right = json!({ "email": "anna@example.pl", "password": PASSWORD });
    let (status, body) = call(&app, Method::POST, "/v1/auth/login", None, Some(right)).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS, "{body}");
}

#[sqlx::test(migrations = "./migrations")]
async fn disabled_accounts_cannot_sign_in_or_refresh(pool: PgPool) {
    let app = app(&pool).await;
    let (_, refresh, _) = sign_up(&app, "anna@example.pl").await;

    sqlx::query("UPDATE users SET status = 'disabled' WHERE email = 'anna@example.pl'")
        .execute(&pool)
        .await
        .unwrap();

    let (status, _) = call(
        &app,
        Method::POST,
        "/v1/auth/login",
        None,
        Some(json!({ "email": "anna@example.pl", "password": PASSWORD })),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _) = call(
        &app,
        Method::POST,
        "/v1/auth/refresh",
        None,
        Some(json!({ "refresh_token": refresh })),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "./migrations")]
async fn business_creation_membership_and_roles(pool: PgPool) {
    let app = app(&pool).await;
    let (anna, _, _) = sign_up(&app, "anna@example.pl").await;
    let (olga, _, olga_id) = sign_up(&app, "olga@example.pl").await;

    // Anna creates a business and becomes its owner.
    let (status, body) = call(
        &app,
        Method::POST,
        "/v1/businesses",
        Some(&anna),
        Some(json!({ "name": "Anna Lashes", "slug": "anna-lashes", "country": "pl" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["role"], "owner");
    assert_eq!(body["country"], "PL");
    assert_eq!(body["timezone"], "Europe/Warsaw");
    let business_id = body["id"].as_str().unwrap().to_string();

    // A solo master is bookable immediately: a staff card was created.
    let staff: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM staff_member WHERE business_id = $1::text::uuid AND is_bookable",
    )
    .bind(&business_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(staff, 1);

    let audits: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_log WHERE business_id = $1::text::uuid AND action = 'business.create'",
    )
    .bind(&business_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audits, 1);

    // It shows up in "my businesses".
    let (_, me) = call(&app, Method::GET, "/v1/me", Some(&anna), None).await;
    assert_eq!(me["memberships"][0]["business_slug"], "anna-lashes");
    assert_eq!(me["memberships"][0]["role"], "owner");

    // Slug rules.
    let (status, _) = call(
        &app,
        Method::POST,
        "/v1/businesses",
        Some(&olga),
        Some(json!({ "name": "Copycat", "slug": "anna-lashes" })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    for bad in [
        json!({ "name": "X", "slug": "Bad Slug" }),
        json!({ "name": "  " }),
        json!({ "name": "X", "timezone": "Mars/Base" }),
    ] {
        let (status, _) = call(&app, Method::POST, "/v1/businesses", Some(&olga), Some(bad)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
    let (status, body) = call(
        &app,
        Method::POST,
        "/v1/businesses",
        Some(&olga),
        Some(json!({ "name": "Olga Nails" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert!(body["slug"].as_str().unwrap().starts_with("b-"));

    let uri = format!("/v1/businesses/{business_id}");

    // A stranger cannot even tell the business exists.
    let (status, _) = call(&app, Method::GET, &uri, Some(&olga), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = call(
        &app,
        Method::PATCH,
        &uri,
        Some(&olga),
        Some(json!({ "name": "Mine now" })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = call(&app, Method::GET, &uri, None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Owner may edit.
    let (status, body) = call(
        &app,
        Method::PATCH,
        &uri,
        Some(&anna),
        Some(json!({ "description": "Lash extensions" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["description"], "Lash extensions");
    assert_eq!(body["name"], "Anna Lashes");

    // Olga joins as an employee (inserted directly; invites come later).
    sqlx::query("INSERT INTO membership (business_id, user_id, role) VALUES ($1::text::uuid, $2::text::uuid, 'employee')")
        .bind(&business_id)
        .bind(&olga_id)
        .execute(&pool)
        .await
        .unwrap();

    let (status, body) = call(&app, Method::GET, &uri, Some(&olga), None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["role"], "employee");

    // ...but an employee may not change settings.
    let (status, _) = call(
        &app,
        Method::PATCH,
        &uri,
        Some(&olga),
        Some(json!({ "name": "Mine now" })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (_, body) = call(&app, Method::GET, &uri, Some(&anna), None).await;
    assert_eq!(body["name"], "Anna Lashes");

    // Suspended members lose access immediately.
    sqlx::query("UPDATE membership SET status = 'suspended' WHERE user_id = $1::text::uuid AND business_id = $2::text::uuid")
        .bind(&olga_id)
        .bind(&business_id)
        .execute(&pool)
        .await
        .unwrap();
    let (status, _) = call(&app, Method::GET, &uri, Some(&olga), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
