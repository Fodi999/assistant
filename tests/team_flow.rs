//! Adding members through the API.

mod common;

use axum::http::{Method, StatusCode};
use common::{app, call, create_business, sign_up};
use serde_json::json;
use sqlx::PgPool;

#[sqlx::test(migrations = "./migrations")]
async fn members_are_added_with_role_rules(pool: PgPool) {
    let app = app(&pool).await;
    let (owner, _) = sign_up(&app, "anna@example.pl").await;
    let (_mia, _) = sign_up(&app, "mia@example.pl").await;
    let (olga, _) = sign_up(&app, "olga@example.pl").await;
    let (_ewa, _) = sign_up(&app, "ewa@example.pl").await;
    let biz = create_business(&app, &owner, "Anna Lashes").await;
    let members = format!("/v1/businesses/{biz}/members");

    // Owner adds a manager; the new card shows up in the staff list.
    let (status, member) = call(
        &app,
        Method::POST,
        &members,
        Some(&owner),
        Some(json!({ "email": " Mia@Example.pl ", "role": "manager" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{member}");
    assert_eq!(member["role"], "manager");
    assert_eq!(member["display_name"], "Test User");
    let (_, staff) = call(
        &app,
        Method::GET,
        &format!("/v1/businesses/{biz}/staff"),
        Some(&owner),
        None,
    )
    .await;
    assert_eq!(staff.as_array().unwrap().len(), 2);

    // Adding the same person again conflicts.
    let (status, _) = call(
        &app,
        Method::POST,
        &members,
        Some(&owner),
        Some(json!({ "email": "mia@example.pl", "role": "employee" })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // The manager (Mia) signs in and can add employees but not managers.
    let (status, login) = call(
        &app,
        Method::POST,
        "/v1/auth/login",
        None,
        Some(json!({ "email": "mia@example.pl", "password": common::PASSWORD })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{login}");
    let mia = login["tokens"]["access_token"]
        .as_str()
        .unwrap()
        .to_string();
    let (status, _) = call(
        &app,
        Method::POST,
        &members,
        Some(&mia),
        Some(json!({ "email": "olga@example.pl", "role": "manager" })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = call(
        &app,
        Method::POST,
        &members,
        Some(&mia),
        Some(json!({ "email": "olga@example.pl", "role": "employee", "display_name": "Olga" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    // Olga is now an employee: she cannot add anybody.
    let (status, _) = call(
        &app,
        Method::POST,
        &members,
        Some(&olga),
        Some(json!({ "email": "ewa@example.pl", "role": "employee" })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Validation and unknown accounts.
    for body in [
        json!({ "email": "ewa@example.pl", "role": "owner" }),
        json!({ "email": "ewa@example.pl", "role": "king" }),
        json!({ "email": "not-an-email", "role": "employee" }),
    ] {
        let (status, response) = call(
            &app,
            Method::POST,
            &members,
            Some(&owner),
            Some(body.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body} {response}");
    }
    let (status, _) = call(
        &app,
        Method::POST,
        &members,
        Some(&owner),
        Some(json!({ "email": "nobody@example.pl", "role": "employee" })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Outsiders and anonymous callers get nothing.
    let (stranger, _) = sign_up(&app, "zed@example.pl").await;
    let (status, _) = call(
        &app,
        Method::POST,
        &members,
        Some(&stranger),
        Some(json!({ "email": "ewa@example.pl", "role": "employee" })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = call(
        &app,
        Method::POST,
        &members,
        None,
        Some(json!({ "email": "ewa@example.pl", "role": "employee" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
