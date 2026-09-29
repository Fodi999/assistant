//! End-to-end tests of master schedules through the real router.

mod common;

use axum::http::{Method, StatusCode};
use common::{add_member, app, call, create_business, my_staff_id, sign_up};
use serde_json::json;
use sqlx::PgPool;

#[sqlx::test(migrations = "./migrations")]
async fn schedule_lifecycle(pool: PgPool) {
    let app = app(&pool).await;
    let (token, _) = sign_up(&app, "anna@example.pl").await;
    let biz = create_business(&app, &token, "Anna Lashes").await;
    let staff = my_staff_id(&app, &token, &biz).await;
    let base = format!("/v1/businesses/{biz}/staff/{staff}");

    // A fresh schedule is empty and states the business time zone.
    let (status, schedule) = call(
        &app,
        Method::GET,
        &format!("{base}/schedule"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{schedule}");
    assert_eq!(schedule["timezone"], "Europe/Warsaw");
    assert_eq!(schedule["weekly"], json!([]));
    assert_eq!(schedule["breaks"], json!([]));
    assert_eq!(schedule["exceptions"], json!([]));

    // Weekly pattern with a split shift on Monday.
    let (status, weekly) = call(
        &app,
        Method::PUT,
        &format!("{base}/schedule/weekly"),
        Some(&token),
        Some(json!({ "intervals": [
            { "weekday": 0, "start": "09:00", "end": "13:00" },
            { "weekday": 0, "start": "14:00", "end": "18:00" },
            { "weekday": 1, "start": "10:00", "end": "19:00" }
        ]})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{weekly}");
    let weekly = weekly.as_array().unwrap();
    assert_eq!(weekly.len(), 3);
    assert_eq!(weekly[0]["weekday"], 0);
    assert_eq!(weekly[0]["start"], "09:00");
    assert_eq!(weekly[0]["end"], "13:00");
    assert_eq!(weekly[1]["start"], "14:00");
    assert_eq!(weekly[2]["weekday"], 1);

    // Overlapping intervals are rejected and the old set stays intact.
    let (status, body) = call(
        &app,
        Method::PUT,
        &format!("{base}/schedule/weekly"),
        Some(&token),
        Some(json!({ "intervals": [
            { "weekday": 2, "start": "09:00", "end": "13:00" },
            { "weekday": 2, "start": "12:00", "end": "16:00" }
        ]})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    let (_, schedule) = call(
        &app,
        Method::GET,
        &format!("{base}/schedule"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(schedule["weekly"].as_array().unwrap().len(), 3);

    // Replacing the set works, including validity windows that do not overlap.
    let (status, weekly) = call(
        &app,
        Method::PUT,
        &format!("{base}/schedule/weekly"),
        Some(&token),
        Some(json!({ "intervals": [
            { "weekday": 4, "start": "09:00", "end": "15:00", "valid_to": "2030-05-31" },
            { "weekday": 4, "start": "11:00", "end": "19:00", "valid_from": "2030-06-01" }
        ]})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{weekly}");
    assert_eq!(weekly.as_array().unwrap().len(), 2);
    assert_eq!(weekly[0]["valid_to"], "2030-05-31");
    assert_eq!(weekly[1]["valid_from"], "2030-06-01");

    // An empty set clears the pattern.
    let (status, weekly) = call(
        &app,
        Method::PUT,
        &format!("{base}/schedule/weekly"),
        Some(&token),
        Some(json!({ "intervals": [] })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(weekly, json!([]));

    // Breaks.
    let (status, breaks) = call(
        &app,
        Method::PUT,
        &format!("{base}/schedule/breaks"),
        Some(&token),
        Some(json!({ "breaks": [
            { "weekday": 0, "start": "13:00", "end": "14:00" },
            { "weekday": 1, "start": "14:00", "end": "14:30" }
        ]})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{breaks}");
    assert_eq!(breaks.as_array().unwrap().len(), 2);
    let (status, body) = call(
        &app,
        Method::PUT,
        &format!("{base}/schedule/breaks"),
        Some(&token),
        Some(json!({ "breaks": [
            { "weekday": 0, "start": "13:00", "end": "14:00" },
            { "weekday": 0, "start": "13:30", "end": "14:30" }
        ]})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");

    // Exceptions: upsert, list, delete.
    let (status, exception) = call(
        &app,
        Method::PUT,
        &format!("{base}/schedule/exceptions/2030-12-24"),
        Some(&token),
        Some(json!({ "kind": "day_off" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{exception}");
    assert_eq!(exception["date"], "2030-12-24");
    assert_eq!(exception["kind"], "day_off");
    assert!(exception["start"].is_null());
    let exception_id = exception["id"].clone();

    let (status, exception) = call(
        &app,
        Method::PUT,
        &format!("{base}/schedule/exceptions/2030-12-24"),
        Some(&token),
        Some(json!({ "kind": "custom_hours", "start": "10:00", "end": "14:00" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{exception}");
    assert_eq!(exception["id"], exception_id, "upsert keeps the row");
    assert_eq!(exception["kind"], "custom_hours");
    assert_eq!(exception["start"], "10:00");

    let (_, schedule) = call(
        &app,
        Method::GET,
        &format!("{base}/schedule"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(schedule["exceptions"].as_array().unwrap().len(), 1);

    let (status, _) = call(
        &app,
        Method::DELETE,
        &format!("{base}/schedule/exceptions/2030-12-24"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(
        &app,
        Method::DELETE,
        &format!("{base}/schedule/exceptions/2030-12-24"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Time off: an offset input is normalised to UTC.
    let (status, off) = call(
        &app,
        Method::POST,
        &format!("{base}/time-off"),
        Some(&token),
        Some(json!({
            "start_at": "2030-07-01T09:00:00+02:00",
            "end_at": "2030-07-15T00:00:00Z",
            "kind": "vacation",
            "note": "Urlop"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{off}");
    assert_eq!(off["start_at"], "2030-07-01T07:00:00Z");
    assert_eq!(off["end_at"], "2030-07-15T00:00:00Z");
    assert_eq!(off["kind"], "vacation");
    assert_eq!(off["staff_id"], staff.as_str());
    let off_id = off["id"].as_str().unwrap().to_string();

    let (status, _) = call(
        &app,
        Method::POST,
        &format!("{base}/time-off"),
        Some(&token),
        Some(json!({
            "start_at": "2030-09-01T00:00:00Z",
            "end_at": "2030-09-02T00:00:00Z",
            "kind": "sick"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    // Listing without a filter returns both, ordered; the window filters by overlap.
    let (_, all) = call(
        &app,
        Method::GET,
        &format!("{base}/time-off"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(all.as_array().unwrap().len(), 2);
    assert_eq!(all[0]["kind"], "vacation");
    let (_, july) = call(
        &app,
        Method::GET,
        &format!("{base}/time-off?from=2030-07-10T00:00:00Z&to=2030-08-01T00:00:00Z"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(july.as_array().unwrap().len(), 1);
    assert_eq!(july[0]["kind"], "vacation");
    let (_, none) = call(
        &app,
        Method::GET,
        &format!("{base}/time-off?from=2030-07-15T00:00:00Z&to=2030-09-01T00:00:00Z"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(none, json!([]), "touching bounds do not overlap");

    let (status, _) = call(
        &app,
        Method::DELETE,
        &format!("/v1/businesses/{biz}/time-off/{off_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(
        &app,
        Method::DELETE,
        &format!("/v1/businesses/{biz}/time-off/{off_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "./migrations")]
async fn schedule_input_is_validated(pool: PgPool) {
    let app = app(&pool).await;
    let (token, _) = sign_up(&app, "anna@example.pl").await;
    let biz = create_business(&app, &token, "Anna Lashes").await;
    let staff = my_staff_id(&app, &token, &biz).await;
    let base = format!("/v1/businesses/{biz}/staff/{staff}");

    let weekly = |interval: serde_json::Value| json!({ "intervals": [interval] });
    let bad_weekly = [
        json!({ "weekday": 7, "start": "09:00", "end": "10:00" }),
        json!({ "weekday": -1, "start": "09:00", "end": "10:00" }),
        json!({ "weekday": 0, "start": "18:00", "end": "09:00" }),
        json!({ "weekday": 0, "start": "09:00", "end": "09:00" }),
        json!({ "weekday": 0, "start": "9am", "end": "10:00" }),
        json!({ "weekday": 0, "start": "09:00", "end": "24:00" }),
        json!({ "weekday": 0, "start": "09:00", "end": "10:00", "valid_from": "2030-02-30" }),
        json!({ "weekday": 0, "start": "09:00", "end": "10:00",
                "valid_from": "2030-06-01", "valid_to": "2030-05-01" }),
    ];
    for interval in bad_weekly {
        let (status, body) = call(
            &app,
            Method::PUT,
            &format!("{base}/schedule/weekly"),
            Some(&token),
            Some(weekly(interval.clone())),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{interval} -> {body}");
    }

    let bad_exceptions = [
        json!({ "kind": "holiday" }),
        json!({ "kind": "day_off", "start": "10:00", "end": "12:00" }),
        json!({ "kind": "custom_hours" }),
        json!({ "kind": "custom_hours", "start": "12:00", "end": "10:00" }),
    ];
    for exception in bad_exceptions {
        let (status, body) = call(
            &app,
            Method::PUT,
            &format!("{base}/schedule/exceptions/2030-12-24"),
            Some(&token),
            Some(exception.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{exception} -> {body}");
    }
    let (status, _) = call(
        &app,
        Method::PUT,
        &format!("{base}/schedule/exceptions/not-a-date"),
        Some(&token),
        Some(json!({ "kind": "day_off" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let bad_time_off = [
        // end before start
        json!({ "start_at": "2030-07-02T00:00:00Z", "end_at": "2030-07-01T00:00:00Z", "kind": "sick" }),
        // unknown kind
        json!({ "start_at": "2030-07-01T00:00:00Z", "end_at": "2030-07-02T00:00:00Z", "kind": "party" }),
        // not RFC 3339
        json!({ "start_at": "2030-07-01", "end_at": "2030-07-02", "kind": "sick" }),
        // longer than 366 days
        json!({ "start_at": "2030-01-01T00:00:00Z", "end_at": "2031-06-01T00:00:00Z", "kind": "vacation" }),
        // recurrence is reserved
        json!({ "start_at": "2030-07-01T00:00:00Z", "end_at": "2030-07-02T00:00:00Z",
                "kind": "blocked", "rrule": "FREQ=WEEKLY" }),
        // note too long
        json!({ "start_at": "2030-07-01T00:00:00Z", "end_at": "2030-07-02T00:00:00Z",
                "kind": "blocked", "note": "x".repeat(501) }),
    ];
    for time_off in bad_time_off {
        let (status, body) = call(
            &app,
            Method::POST,
            &format!("{base}/time-off"),
            Some(&token),
            Some(time_off.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{time_off} -> {body}");
    }

    let (status, _) = call(
        &app,
        Method::GET,
        &format!("{base}/time-off?from=yesterday"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Nothing was stored by the rejected requests.
    let (_, schedule) = call(
        &app,
        Method::GET,
        &format!("{base}/schedule"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(schedule["weekly"], json!([]));
    let (_, off) = call(
        &app,
        Method::GET,
        &format!("{base}/time-off"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(off, json!([]));
}

#[sqlx::test(migrations = "./migrations")]
async fn schedule_permissions_and_tenant_isolation(pool: PgPool) {
    let app = app(&pool).await;
    let (owner, _) = sign_up(&app, "anna@example.pl").await;
    let biz = create_business(&app, &owner, "Anna Lashes").await;
    let owner_staff = my_staff_id(&app, &owner, &biz).await;

    let (olga, olga_id) = sign_up(&app, "olga@example.pl").await;
    let olga_staff = add_member(&pool, &biz, &olga_id, "employee").await;
    let (ewa, ewa_id) = sign_up(&app, "ewa@example.pl").await;
    let ewa_staff = add_member(&pool, &biz, &ewa_id, "employee").await;
    let (mia, mia_id) = sign_up(&app, "mia@example.pl").await;
    let mia_staff = add_member(&pool, &biz, &mia_id, "manager").await;
    let (rita, rita_id) = sign_up(&app, "rita@example.pl").await;
    let rita_staff = add_member(&pool, &biz, &rita_id, "reception").await;

    let staff_base = |staff: &str| format!("/v1/businesses/{biz}/staff/{staff}");
    let hours = json!({ "intervals": [{ "weekday": 0, "start": "09:00", "end": "17:00" }] });
    let future_off = json!({
        "start_at": "2030-07-01T00:00:00Z", "end_at": "2030-07-02T00:00:00Z", "kind": "sick"
    });
    let past_off = json!({
        "start_at": "2020-07-01T00:00:00Z", "end_at": "2020-07-02T00:00:00Z", "kind": "sick"
    });

    // An employee edits their own schedule and adds future time off.
    let own = staff_base(&olga_staff);
    let (status, body) = call(
        &app,
        Method::PUT,
        &format!("{own}/schedule/weekly"),
        Some(&olga),
        Some(hours.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, own_off) = call(
        &app,
        Method::POST,
        &format!("{own}/time-off"),
        Some(&olga),
        Some(future_off.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{own_off}");
    let own_off_id = own_off["id"].as_str().unwrap().to_string();

    // ... but not the past, for either exceptions or time off.
    let (status, _) = call(
        &app,
        Method::POST,
        &format!("{own}/time-off"),
        Some(&olga),
        Some(past_off.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = call(
        &app,
        Method::PUT,
        &format!("{own}/schedule/exceptions/2020-01-06"),
        Some(&olga),
        Some(json!({ "kind": "day_off" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // An employee cannot touch a colleague's schedule (read or write).
    let other = staff_base(&ewa_staff);
    for (method, path, body) in [
        (Method::GET, format!("{other}/schedule"), None),
        (
            Method::PUT,
            format!("{other}/schedule/weekly"),
            Some(hours.clone()),
        ),
        (
            Method::PUT,
            format!("{other}/schedule/breaks"),
            Some(json!({ "breaks": [] })),
        ),
        (Method::GET, format!("{other}/time-off"), None),
        (
            Method::POST,
            format!("{other}/time-off"),
            Some(future_off.clone()),
        ),
    ] {
        let (status, response) = call(&app, method.clone(), &path, Some(&olga), body).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{method} {path}: {response}");
    }

    // Nor delete a colleague's time off; the owner can.
    let (status, ewa_off) = call(
        &app,
        Method::POST,
        &format!("{other}/time-off"),
        Some(&ewa),
        Some(future_off.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{ewa_off}");
    let ewa_off_id = ewa_off["id"].as_str().unwrap().to_string();
    let (status, _) = call(
        &app,
        Method::DELETE,
        &format!("/v1/businesses/{biz}/time-off/{ewa_off_id}"),
        Some(&olga),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = call(
        &app,
        Method::DELETE,
        &format!("/v1/businesses/{biz}/time-off/{ewa_off_id}"),
        Some(&owner),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Reception has no schedule access at all, not even to its own card.
    for staff in [&rita_staff, &olga_staff] {
        let (status, _) = call(
            &app,
            Method::GET,
            &format!("{}/schedule", staff_base(staff)),
            Some(&rita),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    // Owner and manager manage anyone's schedule, including the past.
    for token in [&owner, &mia] {
        let (status, body) = call(
            &app,
            Method::PUT,
            &format!("{own}/schedule/weekly"),
            Some(token),
            Some(hours.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let (status, body) = call(
            &app,
            Method::PUT,
            &format!("{own}/schedule/exceptions/2020-01-06"),
            Some(token),
            Some(json!({ "kind": "day_off" })),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
    }
    let (status, _) = call(
        &app,
        Method::DELETE,
        &format!("/v1/businesses/{biz}/time-off/{own_off_id}"),
        Some(&mia),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let _ = mia_staff;

    // Another business: outsiders get 404, and its staff ids do not work here.
    let (stranger, _) = sign_up(&app, "zed@example.pl").await;
    let other_biz = create_business(&app, &stranger, "Zed Studio").await;
    let stranger_staff = my_staff_id(&app, &stranger, &other_biz).await;
    let (status, _) = call(
        &app,
        Method::GET,
        &format!("{}/schedule", staff_base(&owner_staff)),
        Some(&stranger),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = call(
        &app,
        Method::GET,
        &format!("{}/schedule", staff_base(&stranger_staff)),
        Some(&owner),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "staff of another business");
    let (status, _) = call(
        &app,
        Method::PUT,
        &format!("/v1/businesses/{other_biz}/staff/{owner_staff}/schedule/weekly"),
        Some(&stranger),
        Some(hours.clone()),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "our staff id in their business"
    );
    let (status, _) = call(
        &app,
        Method::GET,
        &format!("{}/schedule", staff_base(&owner_staff)),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Unknown staff id inside the business.
    let (status, _) = call(
        &app,
        Method::GET,
        &format!("{}/schedule", staff_base(&uuid::Uuid::new_v4().to_string())),
        Some(&owner),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
