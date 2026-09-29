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

// ---------------------------------------------------------------------------
// Team list, role changes, suspending, the last owner
// ---------------------------------------------------------------------------

struct Team {
    app: axum::Router,
    biz: String,
    owner: String,
    manager: String,
    employee: String,
    reception: String,
}

async fn team(pool: &PgPool) -> Team {
    let app = app(pool).await;
    let (owner, _) = sign_up(&app, "anna@example.pl").await;
    let (manager, _) = sign_up(&app, "mia@example.pl").await;
    let (employee, _) = sign_up(&app, "olga@example.pl").await;
    let (reception, _) = sign_up(&app, "ewa@example.pl").await;
    let biz = create_business(&app, &owner, "Anna Lashes").await;
    for (email, role) in [
        ("mia@example.pl", "manager"),
        ("olga@example.pl", "employee"),
        ("ewa@example.pl", "reception"),
    ] {
        let (status, body) = call(
            &app,
            Method::POST,
            &format!("/v1/businesses/{biz}/members"),
            Some(&owner),
            Some(json!({ "email": email, "role": role })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
    }
    Team {
        app,
        biz,
        owner,
        manager,
        employee,
        reception,
    }
}

impl Team {
    fn base(&self) -> String {
        format!("/v1/businesses/{}", self.biz)
    }

    async fn members(&self, token: &str) -> (StatusCode, serde_json::Value) {
        call(
            &self.app,
            Method::GET,
            &format!("{}/members", self.base()),
            Some(token),
            None,
        )
        .await
    }

    /// The membership id of the person with `role` (first one).
    async fn membership(&self, role: &str) -> String {
        let (_, list) = self.members(&self.owner).await;
        list.as_array()
            .unwrap()
            .iter()
            .find(|member| member["role"] == role)
            .unwrap()["membership_id"]
            .as_str()
            .unwrap()
            .to_string()
    }

    async fn patch(
        &self,
        token: &str,
        membership: &str,
        body: serde_json::Value,
    ) -> (StatusCode, serde_json::Value) {
        call(
            &self.app,
            Method::PATCH,
            &format!("{}/members/{membership}", self.base()),
            Some(token),
            Some(body),
        )
        .await
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn the_team_list_is_for_owner_and_manager(pool: PgPool) {
    let team = team(&pool).await;
    let (status, list) = team.members(&team.owner).await;
    assert_eq!(status, StatusCode::OK, "{list}");
    let list = list.as_array().unwrap();
    assert_eq!(list.len(), 4);
    assert_eq!(list[0]["role"], "owner", "the owner comes first");
    assert_eq!(list[0]["is_me"], true);
    assert!(list.iter().all(|member| member["status"] == "active"));
    assert!(list.iter().all(|member| member["staff_id"].is_string()));

    let (status, _) = team.members(&team.manager).await;
    assert_eq!(status, StatusCode::OK);
    for token in [&team.employee, &team.reception] {
        let (status, _) = team.members(token).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn roles_change_within_what_each_role_may_manage(pool: PgPool) {
    let team = team(&pool).await;
    let employee = team.membership("employee").await;
    let manager = team.membership("manager").await;
    let reception = team.membership("reception").await;

    // The owner promotes and demotes.
    let (status, body) = team
        .patch(&team.owner, &employee, json!({ "role": "reception" }))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["role"], "reception");
    let (status, body) = team
        .patch(&team.owner, &employee, json!({ "role": "employee" }))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    // A manager handles employees and reception only, and cannot make managers.
    let (status, _) = team
        .patch(&team.manager, &employee, json!({ "role": "reception" }))
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = team
        .patch(&team.manager, &reception, json!({ "role": "manager" }))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let owner = team.membership("owner").await;
    let (status, _) = team
        .patch(&team.manager, &owner, json!({ "role": "employee" }))
        .await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "the only owner is protected first"
    );

    // Employees and reception cannot manage anyone.
    for token in [&team.employee, &team.reception] {
        let (status, _) = team
            .patch(token, &manager, json!({ "role": "employee" }))
            .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn nobody_changes_themselves_and_the_last_owner_stays(pool: PgPool) {
    let team = team(&pool).await;
    let owner = team.membership("owner").await;
    let manager = team.membership("manager").await;

    // The owner cannot demote or suspend themselves, whatever the change.
    for body in [
        json!({ "role": "manager" }),
        json!({ "status": "suspended" }),
        json!({ "role": "employee", "status": "suspended" }),
    ] {
        let (status, response) = team.patch(&team.owner, &owner, body.clone()).await;
        assert_eq!(status, StatusCode::CONFLICT, "{body} {response}");
    }
    // Nor can a manager change themselves.
    let (status, _) = team
        .patch(&team.manager, &manager, json!({ "role": "employee" }))
        .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // The owner is still the owner and still active.
    let (_, list) = team.members(&team.owner).await;
    let first = &list.as_array().unwrap()[0];
    assert_eq!(first["role"], "owner");
    assert_eq!(first["status"], "active");
    let (status, _) = call(
        &team.app,
        Method::GET,
        &team.base(),
        Some(&team.owner),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test(migrations = "./migrations")]
async fn suspending_removes_access_and_bookability_and_can_be_undone(pool: PgPool) {
    let team = team(&pool).await;
    let employee = team.membership("employee").await;

    let (status, body) = team
        .patch(&team.owner, &employee, json!({ "status": "suspended" }))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["status"], "suspended");
    assert_eq!(body["is_bookable"], false);

    // Olga is out: the business is gone for her, and her card is not bookable.
    let (status, _) = call(
        &team.app,
        Method::GET,
        &team.base(),
        Some(&team.employee),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (_, list) = team.members(&team.owner).await;
    let olga = list
        .as_array()
        .unwrap()
        .iter()
        .find(|member| member["membership_id"] == employee.as_str())
        .unwrap();
    assert_eq!(
        olga["status"], "suspended",
        "still listed, so the owner can restore her"
    );

    // Restored: access and booking are back.
    let (status, body) = team
        .patch(&team.owner, &employee, json!({ "status": "active" }))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["is_bookable"], true);
    let (status, _) = call(
        &team.app,
        Method::GET,
        &team.base(),
        Some(&team.employee),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test(migrations = "./migrations")]
async fn member_updates_are_validated_and_tenant_safe(pool: PgPool) {
    let team = team(&pool).await;
    let employee = team.membership("employee").await;
    for body in [
        json!({}),
        json!({ "role": "owner" }),
        json!({ "role": "king" }),
        json!({ "status": "removed" }),
        json!({ "status": "invited" }),
    ] {
        let (status, response) = team.patch(&team.owner, &employee, body.clone()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body} {response}");
    }
    let unknown = uuid::Uuid::new_v4().to_string();
    let (status, _) = team
        .patch(&team.owner, &unknown, json!({ "role": "employee" }))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Someone else's business: nothing is revealed or changed.
    let (stranger, _) = sign_up(&team.app, "zed@example.pl").await;
    let other = create_business(&team.app, &stranger, "Other").await;
    let (status, _) = call(
        &team.app,
        Method::PATCH,
        &format!("/v1/businesses/{other}/members/{employee}"),
        Some(&stranger),
        Some(json!({ "role": "reception" })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = call(
        &team.app,
        Method::PATCH,
        &format!("{}/members/{employee}", team.base()),
        Some(&stranger),
        Some(json!({ "role": "reception" })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(team.membership("employee").await, employee);
}

#[sqlx::test(migrations = "./migrations")]
async fn who_takes_bookings_is_set_by_owner_or_manager_only(pool: PgPool) {
    let team = team(&pool).await;
    let (_, list) = team.members(&team.owner).await;
    let staff = list
        .as_array()
        .unwrap()
        .iter()
        .find(|member| member["role"] == "employee")
        .unwrap()["staff_id"]
        .as_str()
        .unwrap()
        .to_string();
    let uri = format!("{}/staff/{staff}", team.base());

    // The master edits their own name and bio but not their own bookability.
    let (status, _) = call(
        &team.app,
        Method::PATCH,
        &uri,
        Some(&team.employee),
        Some(json!({ "name": "Olga N." })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = call(
        &team.app,
        Method::PATCH,
        &uri,
        Some(&team.employee),
        Some(json!({ "is_bookable": false })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, card) = call(
        &team.app,
        Method::PATCH,
        &uri,
        Some(&team.manager),
        Some(json!({ "is_bookable": false })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{card}");
    assert_eq!(card["is_bookable"], false);
    assert_eq!(card["display_name"], "Olga N.");
    let (status, card) = call(
        &team.app,
        Method::PATCH,
        &uri,
        Some(&team.owner),
        Some(json!({ "is_bookable": true })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{card}");
    assert_eq!(card["is_bookable"], true);
}
