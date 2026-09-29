//! Master onboarding: phone at registration, business type/address, staff card edits.

mod common;

use axum::http::{Method, StatusCode};
use common::{app, call, create_business, my_staff_id, sign_up, PASSWORD};
use serde_json::{json, Value};
use sqlx::PgPool;

fn body_with_phone(email: &str, phone: Option<&str>) -> Value {
    let mut body = json!({
        "email": email,
        "password": PASSWORD,
        "display_name": "Anna Master",
        "accepted_terms": true
    });
    if let Some(phone) = phone {
        body["phone"] = json!(phone);
    }
    body
}

#[sqlx::test(migrations = "./migrations")]
async fn registration_accepts_an_optional_phone(pool: PgPool) {
    let app = app(&pool).await;

    // With a phone: stored in E.164 and returned by register and /v1/me.
    let (status, body) = call(
        &app,
        Method::POST,
        "/v1/auth/register",
        None,
        Some(body_with_phone("anna@example.pl", Some("+48 600-100-200"))),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["user"]["phone"], "+48600100200");
    let token = body["tokens"]["access_token"].as_str().unwrap().to_string();
    let (_, me) = call(&app, Method::GET, "/v1/me", Some(&token), None).await;
    assert_eq!(me["user"]["phone"], "+48600100200");

    // Without a phone: still works, phone is null.
    let (status, body) = call(
        &app,
        Method::POST,
        "/v1/auth/register",
        None,
        Some(body_with_phone("olga@example.pl", None)),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert!(body["user"]["phone"].is_null());

    // Same phone again is a conflict.
    let (status, _) = call(
        &app,
        Method::POST,
        "/v1/auth/register",
        None,
        Some(body_with_phone("ewa@example.pl", Some("+48600100200"))),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // Malformed phones are rejected.
    for bad in ["600100200", "+0123456", "+48abc", "+1234"] {
        let (status, _) = call(
            &app,
            Method::POST,
            "/v1/auth/register",
            None,
            Some(body_with_phone("mia@example.pl", Some(bad))),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad}");
    }

    // Nothing is verified yet.
    let verified: i64 =
        sqlx::query_scalar("SELECT count(*) FROM users WHERE phone_verified_at IS NOT NULL")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(verified, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn profile_takes_business_type_and_address(pool: PgPool) {
    let app = app(&pool).await;
    let (owner, _) = sign_up(&app, "anna@example.pl").await;
    let biz = create_business(&app, &owner, "Anna Lashes").await;
    let uri = format!("/v1/businesses/{biz}/profile");

    let (status, body) = call(&app, Method::GET, &uri, Some(&owner), None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(body["business_type"].is_null());
    assert!(body["address_line"].is_null());
    assert_eq!(body["moderation_status"], "pending");

    let (status, body) = call(
        &app,
        Method::PUT,
        &uri,
        Some(&owner),
        Some(json!({
            "city": "Warszawa",
            "business_type": "lashes",
            "address_line": "  ul. Marszałkowska 10/5  ",
            "is_published": true
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["business_type"], "lashes");
    assert_eq!(body["address_line"], "ul. Marszałkowska 10/5");
    assert_eq!(body["is_published"], true);
    assert_eq!(body["publicly_visible"], false, "still pending");

    // Unknown types and over-long addresses are refused.
    for bad in [
        json!({ "business_type": "barber" }),
        json!({ "address_line": "x".repeat(201) }),
    ] {
        let (status, _) = call(&app, Method::PUT, &uri, Some(&owner), Some(bad)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    // Absent fields stay; empty strings clear.
    let (_, body) = call(
        &app,
        Method::PUT,
        &uri,
        Some(&owner),
        Some(json!({ "about": "Hi" })),
    )
    .await;
    assert_eq!(body["business_type"], "lashes");
    let (_, body) = call(
        &app,
        Method::PUT,
        &uri,
        Some(&owner),
        Some(json!({ "business_type": "", "address_line": "" })),
    )
    .await;
    assert!(body["business_type"].is_null());
    assert!(body["address_line"].is_null());
}

#[sqlx::test(migrations = "./migrations")]
async fn staff_cards_can_be_renamed(pool: PgPool) {
    let app = app(&pool).await;
    let (owner, _) = sign_up(&app, "anna@example.pl").await;
    let (_mia, _) = sign_up(&app, "mia@example.pl").await;
    let (stranger, _) = sign_up(&app, "olga@example.pl").await;
    let biz = create_business(&app, &owner, "Anna Lashes").await;
    let other_biz = create_business(&app, &stranger, "Olga Nails").await;
    let mine = my_staff_id(&app, &owner, &biz).await;
    let uri = format!("/v1/businesses/{biz}/staff/{mine}");

    let (status, body) = call(
        &app,
        Method::PATCH,
        &uri,
        Some(&owner),
        Some(json!({ "name": "  Anna K.  ", "bio": "Lash artist, 5 years" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["display_name"], "Anna K.");
    assert_eq!(body["bio"], "Lash artist, 5 years");
    assert_eq!(body["is_mine"], true);

    // Only the given fields change; an empty bio clears it.
    let (_, body) = call(
        &app,
        Method::PATCH,
        &uri,
        Some(&owner),
        Some(json!({ "bio": "" })),
    )
    .await;
    assert_eq!(body["display_name"], "Anna K.");
    assert!(body["bio"].is_null());

    // Validation.
    for bad in [json!({ "name": "  " }), json!({ "name": "x".repeat(121) })] {
        let (status, _) = call(&app, Method::PATCH, &uri, Some(&owner), Some(bad)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    // An employee edits only their own card.
    let (status, _) = call(
        &app,
        Method::POST,
        &format!("/v1/businesses/{biz}/members"),
        Some(&owner),
        Some(json!({ "email": "mia@example.pl", "role": "employee" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let (status, login) = call(
        &app,
        Method::POST,
        "/v1/auth/login",
        None,
        Some(json!({ "email": "mia@example.pl", "password": PASSWORD })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let mia = login["tokens"]["access_token"]
        .as_str()
        .unwrap()
        .to_string();
    let mia_card = my_staff_id(&app, &mia, &biz).await;
    let (status, _) = call(
        &app,
        Method::PATCH,
        &uri,
        Some(&mia),
        Some(json!({ "name": "Hacked" })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, body) = call(
        &app,
        Method::PATCH,
        &format!("/v1/businesses/{biz}/staff/{mia_card}"),
        Some(&mia),
        Some(json!({ "name": "Mia" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    // The owner can edit an employee's card.
    let (status, _) = call(
        &app,
        Method::PATCH,
        &format!("/v1/businesses/{biz}/staff/{mia_card}"),
        Some(&owner),
        Some(json!({ "bio": "Brows" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Another business cannot touch it, and cards do not leak across tenants.
    let (status, _) = call(
        &app,
        Method::PATCH,
        &uri,
        Some(&stranger),
        Some(json!({ "name": "X" })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = call(
        &app,
        Method::PATCH,
        &format!("/v1/businesses/{other_biz}/staff/{mine}"),
        Some(&stranger),
        Some(json!({ "name": "X" })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "./migrations")]
async fn admin_sees_the_owner_phone_and_business_details(pool: PgPool) {
    let app = app(&pool).await;
    let (status, reg) = call(
        &app,
        Method::POST,
        "/v1/auth/register",
        None,
        Some(body_with_phone("anna@example.pl", Some("+48600100200"))),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{reg}");
    let owner = reg["tokens"]["access_token"].as_str().unwrap().to_string();
    let biz = create_business(&app, &owner, "Anna Lashes").await;
    let (status, _) = call(
        &app,
        Method::PUT,
        &format!("/v1/businesses/{biz}/profile"),
        Some(&owner),
        Some(json!({ "city": "Warszawa", "business_type": "lashes",
                     "address_line": "ul. Prosta 1", "is_published": true })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (admin, admin_id) = sign_up(&app, "admin@example.pl").await;
    sqlx::query("INSERT INTO platform_admin (user_id) VALUES ($1::text::uuid)")
        .bind(&admin_id)
        .execute(&pool)
        .await
        .unwrap();
    let (status, detail) = call(
        &app,
        Method::GET,
        &format!("/v1/admin/businesses/{biz}"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{detail}");
    assert_eq!(detail["owner"]["phone"], "+48600100200");
    assert_eq!(detail["owner"]["email"], "anna@example.pl");
    assert_eq!(detail["owner"]["name"], "Anna Master");
    assert_eq!(detail["city"], "Warszawa");
    assert_eq!(detail["business_type"], "lashes");
    assert_eq!(detail["address_line"], "ul. Prosta 1");
    assert_eq!(detail["moderation_status"], "pending");
}
