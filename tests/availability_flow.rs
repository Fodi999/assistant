//! End-to-end tests of availability with a fixed clock.
//!
//! The clock is 2027-03-20 (a Saturday) 09:00 UTC. The Warsaw clocks change on
//! Sunday 2027-03-28 (CET, UTC+1 -> CEST, UTC+2), which the tests cross.

mod common;

use axum::http::{Method, StatusCode};
use common::{add_member, app_at, call, create_business, my_staff_id, sign_up};
use serde_json::{json, Value};
use sqlx::PgPool;
use time::macros::datetime;

const NOW: time::OffsetDateTime = datetime!(2027-03-20 09:00 UTC);

struct Shop {
    app: axum::Router,
    token: String,
    base: String,
    staff: String,
    service: String,
    variant: String,
}

/// Anna Lashes: Monday 09-13 and 14-17 with a 12:00-12:30 break, Sunday 09-12.
/// Service "Classic": 60 min, 30-minute grid, no buffer, no minimum notice.
async fn shop_at(pool: &PgPool, now: time::OffsetDateTime, service_extra: Value) -> Shop {
    let app = app_at(pool, now).await;
    let (token, _) = sign_up(&app, "anna@example.pl").await;
    let biz = create_business(&app, &token, "Anna Lashes").await;
    let staff = my_staff_id(&app, &token, &biz).await;
    let base = format!("/v1/businesses/{biz}");

    let mut body = json!({
        "name": { "pl": "Classic" },
        "booking_step_minutes": 30,
        "min_notice_min": 0,
        "variants": [{ "duration_min": 60, "price_minor": 25000 }]
    });
    for (key, value) in service_extra.as_object().unwrap() {
        body[key] = value.clone();
    }
    let (status, service) = call(
        &app,
        Method::POST,
        &format!("{base}/services"),
        Some(&token),
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{service}");
    let service_id = service["id"].as_str().unwrap().to_string();
    let variant = service["variants"][0]["id"].as_str().unwrap().to_string();
    let (status, _) = call(
        &app,
        Method::PUT,
        &format!("{base}/services/{service_id}/staff"),
        Some(&token),
        Some(json!({ "staff_ids": [staff] })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let staff_base = format!("{base}/staff/{staff}/schedule");
    let (status, body) = call(
        &app,
        Method::PUT,
        &format!("{staff_base}/weekly"),
        Some(&token),
        Some(json!({ "intervals": [
            { "weekday": 0, "start": "09:00", "end": "13:00" },
            { "weekday": 0, "start": "14:00", "end": "17:00" },
            { "weekday": 6, "start": "09:00", "end": "12:00" }
        ]})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) = call(
        &app,
        Method::PUT,
        &format!("{staff_base}/breaks"),
        Some(&token),
        Some(json!({ "breaks": [{ "weekday": 0, "start": "12:00", "end": "12:30" }] })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    Shop {
        app,
        token,
        base,
        staff,
        service: service_id,
        variant,
    }
}

impl Shop {
    fn query(&self, from: &str, to: &str, extra: &str) -> String {
        format!(
            "{}/availability?service_id={}&variant_id={}&from={from}&to={to}{extra}",
            self.base, self.service, self.variant
        )
    }

    async fn slots(&self, from: &str, to: &str) -> Vec<String> {
        let (status, body) = call(
            &self.app,
            Method::GET,
            &self.query(from, to, ""),
            Some(&self.token),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        starts(&body)
    }
}

fn starts(body: &Value) -> Vec<String> {
    body["slots"]
        .as_array()
        .unwrap()
        .iter()
        .map(|slot| slot["start_at"].as_str().unwrap().to_string())
        .collect()
}

#[sqlx::test(migrations = "./migrations")]
async fn slots_follow_hours_breaks_and_the_slot_grid(pool: PgPool) {
    let shop = shop_at(&pool, NOW, json!({})).await;

    // Monday 2027-03-22 (CET, UTC+1): 09-13 with the 12:00 break, then 14-17.
    let (status, body) = call(
        &shop.app,
        Method::GET,
        &shop.query("2027-03-22", "2027-03-22", ""),
        Some(&shop.token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["timezone"], "Europe/Warsaw");
    assert_eq!(body["duration_min"], 60);
    assert_eq!(
        starts(&body),
        [
            "2027-03-22T08:00:00Z",
            "2027-03-22T08:30:00Z",
            "2027-03-22T09:00:00Z",
            "2027-03-22T09:30:00Z",
            "2027-03-22T10:00:00Z", // 11:00 local: the last start before the break
            "2027-03-22T13:00:00Z", // 14:00 local
            "2027-03-22T13:30:00Z",
            "2027-03-22T14:00:00Z",
            "2027-03-22T14:30:00Z",
            "2027-03-22T15:00:00Z", // 16:00 local: last start of the shift
        ]
    );
    let first = &body["slots"][0];
    assert_eq!(first["date"], "2027-03-22");
    assert_eq!(first["end_at"], "2027-03-22T09:00:00Z");
    assert_eq!(first["staff_id"], shop.staff.as_str());

    // A day without working hours has no slots.
    assert!(shop.slots("2027-03-23", "2027-03-23").await.is_empty());
}

#[sqlx::test(migrations = "./migrations")]
async fn daylight_saving_shifts_utc_but_not_local_hours(pool: PgPool) {
    let shop = shop_at(&pool, NOW, json!({})).await;
    // Sunday 03-21 is CET (09:00 local = 08:00Z); Sunday 03-28 is CEST (07:00Z).
    let winter = shop.slots("2027-03-21", "2027-03-21").await;
    let summer = shop.slots("2027-03-28", "2027-03-28").await;
    assert_eq!(winter.len(), 5);
    assert_eq!(summer.len(), 5);
    assert_eq!(winter[0], "2027-03-21T08:00:00Z");
    assert_eq!(summer[0], "2027-03-28T07:00:00Z");
    assert_eq!(summer[4], "2027-03-28T09:00:00Z"); // 11:00 local

    // One request across the change returns both, with the local date attached.
    let (_, body) = call(
        &shop.app,
        Method::GET,
        &shop.query("2027-03-21", "2027-03-28", ""),
        Some(&shop.token),
        None,
    )
    .await;
    let slots = body["slots"].as_array().unwrap();
    assert_eq!(slots.len(), 5 + 10 + 5);
    assert_eq!(slots[0]["date"], "2027-03-21");
    assert_eq!(slots[slots.len() - 1]["date"], "2027-03-28");
}

#[sqlx::test(migrations = "./migrations")]
async fn exceptions_and_time_off_change_the_day(pool: PgPool) {
    let shop = shop_at(&pool, NOW, json!({})).await;
    let staff_base = format!("{}/staff/{}", shop.base, shop.staff);

    // Time off 14:00-14:30 local (13:00-13:30Z) removes the 14:00 start only.
    let (status, _) = call(
        &shop.app,
        Method::POST,
        &format!("{staff_base}/time-off"),
        Some(&shop.token),
        Some(json!({
            "start_at": "2027-03-22T13:00:00Z", "end_at": "2027-03-22T13:30:00Z", "kind": "blocked"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let slots = shop.slots("2027-03-22", "2027-03-22").await;
    assert_eq!(slots.len(), 9);
    assert!(!slots.contains(&"2027-03-22T13:00:00Z".to_string()));
    assert!(slots.contains(&"2027-03-22T13:30:00Z".to_string()));

    // A day off empties the day.
    let (status, _) = call(
        &shop.app,
        Method::PUT,
        &format!("{staff_base}/schedule/exceptions/2027-03-22"),
        Some(&shop.token),
        Some(json!({ "kind": "day_off" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(shop.slots("2027-03-22", "2027-03-22").await.is_empty());

    // Custom hours replace the pattern; the weekly break does not apply to them.
    let (status, _) = call(
        &shop.app,
        Method::PUT,
        &format!("{staff_base}/schedule/exceptions/2027-03-22"),
        Some(&shop.token),
        Some(json!({ "kind": "custom_hours", "start": "11:00", "end": "13:00" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        shop.slots("2027-03-22", "2027-03-22").await,
        [
            "2027-03-22T10:00:00Z", // 11:00 local
            "2027-03-22T10:30:00Z",
            "2027-03-22T11:00:00Z" // 12:00 local, across the (ignored) break
        ]
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn buffer_after_blocks_the_start_before_a_break(pool: PgPool) {
    let shop = shop_at(&pool, NOW, json!({ "buffer_after_min": 30 })).await;
    // 60 min + 30 min buffer must clear the 12:00 break: 11:00 is no longer offered.
    let slots = shop.slots("2027-03-22", "2027-03-22").await;
    assert_eq!(
        &slots[..4],
        [
            "2027-03-22T08:00:00Z",
            "2027-03-22T08:30:00Z",
            "2027-03-22T09:00:00Z",
            "2027-03-22T09:30:00Z"
        ]
    );
    assert!(!slots.contains(&"2027-03-22T10:00:00Z".to_string()));
    let (_, body) = call(
        &shop.app,
        Method::GET,
        &shop.query("2027-03-22", "2027-03-22", ""),
        Some(&shop.token),
        None,
    )
    .await;
    assert_eq!(body["buffer_after_min"], 30);
}

#[sqlx::test(migrations = "./migrations")]
async fn minimum_notice_and_booking_horizon_apply(pool: PgPool) {
    // Monday 10:30 local; 60 minutes notice -> nothing before 11:30 local.
    let now = datetime!(2027-03-22 09:30 UTC);
    let shop = shop_at(&pool, now, json!({ "min_notice_min": 60 })).await;
    assert_eq!(
        shop.slots("2027-03-22", "2027-03-22").await,
        [
            "2027-03-22T13:00:00Z",
            "2027-03-22T13:30:00Z",
            "2027-03-22T14:00:00Z",
            "2027-03-22T14:30:00Z",
            "2027-03-22T15:00:00Z"
        ]
    );

    // Horizon of two days from Saturday 09:00Z ends Monday 09:00Z (10:00 local).
    let horizon = shop_at_horizon(&pool).await;
    assert_eq!(
        horizon.slots("2027-03-22", "2027-03-22").await,
        [
            "2027-03-22T08:00:00Z",
            "2027-03-22T08:30:00Z",
            "2027-03-22T09:00:00Z"
        ]
    );
}

async fn shop_at_horizon(pool: &PgPool) -> Shop {
    // A separate database is not needed: a second business in the same pool.
    let app = app_at(pool, NOW).await;
    let (token, _) = sign_up(&app, "horizon@example.pl").await;
    let biz = create_business(&app, &token, "Horizon").await;
    let staff = my_staff_id(&app, &token, &biz).await;
    let base = format!("/v1/businesses/{biz}");
    let (_, service) = call(
        &app,
        Method::POST,
        &format!("{base}/services"),
        Some(&token),
        Some(json!({
            "name": { "pl": "Short horizon" },
            "booking_step_minutes": 30,
            "min_notice_min": 0,
            "max_advance_days": 2,
            "variants": [{ "duration_min": 60, "price_minor": 100 }]
        })),
    )
    .await;
    let service_id = service["id"].as_str().unwrap().to_string();
    let variant = service["variants"][0]["id"].as_str().unwrap().to_string();
    call(
        &app,
        Method::PUT,
        &format!("{base}/services/{service_id}/staff"),
        Some(&token),
        Some(json!({ "staff_ids": [staff] })),
    )
    .await;
    call(
        &app,
        Method::PUT,
        &format!("{base}/staff/{staff}/schedule/weekly"),
        Some(&token),
        Some(json!({ "intervals": [{ "weekday": 0, "start": "09:00", "end": "13:00" }] })),
    )
    .await;
    Shop {
        app,
        token,
        base,
        staff,
        service: service_id,
        variant,
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn staff_filter_and_request_validation(pool: PgPool) {
    let shop = shop_at(&pool, NOW, json!({})).await;

    // A second master who also performs the service, working Monday 10-12.
    let (_, olga_id) = sign_up(&shop.app, "olga@example.pl").await;
    let olga_staff = add_member(
        &pool,
        shop.base.rsplit('/').next().unwrap(),
        &olga_id,
        "employee",
    )
    .await;
    let (status, _) = call(
        &shop.app,
        Method::PUT,
        &format!("{}/services/{}/staff", shop.base, shop.service),
        Some(&shop.token),
        Some(json!({ "staff_ids": [shop.staff, olga_staff] })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = call(
        &shop.app,
        Method::PUT,
        &format!("{}/staff/{olga_staff}/schedule/weekly", shop.base),
        Some(&shop.token),
        Some(json!({ "intervals": [{ "weekday": 0, "start": "10:00", "end": "12:00" }] })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Both masters without a filter; each master alone with one.
    let (_, all) = call(
        &shop.app,
        Method::GET,
        &shop.query("2027-03-22", "2027-03-22", ""),
        Some(&shop.token),
        None,
    )
    .await;
    // Anna 10 slots + Olga 10:00-12:00 with 60 min = 10:00, 10:30, 11:00.
    assert_eq!(all["slots"].as_array().unwrap().len(), 13);
    let (_, only_olga) = call(
        &shop.app,
        Method::GET,
        &shop.query(
            "2027-03-22",
            "2027-03-22",
            &format!("&staff_id={olga_staff}"),
        ),
        Some(&shop.token),
        None,
    )
    .await;
    assert_eq!(only_olga["slots"].as_array().unwrap().len(), 3);
    assert!(only_olga["slots"]
        .as_array()
        .unwrap()
        .iter()
        .all(|slot| slot["staff_id"] == olga_staff.as_str()));

    // Validation.
    let bad = [
        // Dates.
        shop.query("22-03-2027", "2027-03-22", ""),
        shop.query("2027-03-22", "2027-03-21", ""),
        shop.query("2027-03-01", "2027-03-15", ""), // 15 days
        shop.query("2027-03-22", "2027-03-22", "&channel=phone"),
    ];
    for uri in bad {
        let (status, body) = call(&shop.app, Method::GET, &uri, Some(&shop.token), None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{uri} {body}");
    }
    // Missing required parameter.
    let (status, _) = call(
        &shop.app,
        Method::GET,
        &format!("{}/availability?from=2027-03-22", shop.base),
        Some(&shop.token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    // Unknown service or variant; a staff member who does not do the service.
    let unknown = uuid::Uuid::new_v4();
    for uri in [
        format!(
            "{}/availability?service_id={unknown}&variant_id={}&from=2027-03-22",
            shop.base, shop.variant
        ),
        format!(
            "{}/availability?service_id={}&variant_id={unknown}&from=2027-03-22",
            shop.base, shop.service
        ),
        shop.query("2027-03-22", "2027-03-22", &format!("&staff_id={unknown}")),
    ] {
        let (status, body) = call(&shop.app, Method::GET, &uri, Some(&shop.token), None).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri} {body}");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn online_channel_and_deactivated_services(pool: PgPool) {
    let shop = shop_at(&pool, NOW, json!({ "is_online_bookable": false })).await;
    let (status, _) = call(
        &shop.app,
        Method::GET,
        &shop.query("2027-03-22", "2027-03-22", ""),
        Some(&shop.token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "not bookable online");
    let (status, body) = call(
        &shop.app,
        Method::GET,
        &shop.query("2027-03-22", "2027-03-22", "&channel=manual"),
        Some(&shop.token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(starts(&body).len(), 10);

    // A deactivated variant is not offered.
    let (status, _) = call(
        &shop.app,
        Method::PATCH,
        &format!("{}/variants/{}", shop.base, shop.variant),
        Some(&shop.token),
        Some(json!({ "is_active": false })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = call(
        &shop.app,
        Method::GET,
        &shop.query("2027-03-22", "2027-03-22", "&channel=manual"),
        Some(&shop.token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "./migrations")]
async fn availability_is_scoped_to_the_business(pool: PgPool) {
    let shop = shop_at(&pool, NOW, json!({})).await;

    // Reception may look at availability (it books for clients).
    let (rita, rita_id) = sign_up(&shop.app, "rita@example.pl").await;
    add_member(
        &pool,
        shop.base.rsplit('/').next().unwrap(),
        &rita_id,
        "reception",
    )
    .await;
    let (status, body) = call(
        &shop.app,
        Method::GET,
        &shop.query("2027-03-22", "2027-03-22", ""),
        Some(&rita),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(starts(&body).len(), 10);

    // Another business cannot use ours, and ours ids do not work in its URL.
    let (zed, _) = sign_up(&shop.app, "zed@example.pl").await;
    let other = create_business(&shop.app, &zed, "Zed Studio").await;
    let (status, _) = call(
        &shop.app,
        Method::GET,
        &shop.query("2027-03-22", "2027-03-22", ""),
        Some(&zed),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = call(
        &shop.app,
        Method::GET,
        &format!(
            "/v1/businesses/{other}/availability?service_id={}&variant_id={}&from=2027-03-22",
            shop.service, shop.variant
        ),
        Some(&zed),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = call(
        &shop.app,
        Method::GET,
        &shop.query("2027-03-22", "2027-03-22", ""),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
