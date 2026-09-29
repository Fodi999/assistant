//! End-to-end tests of slot holds and double-booking protection.
//!
//! The clock is 2027-03-20 (a Saturday) 09:00 UTC. Anna works Monday
//! 2027-03-22 09:00-13:00 Warsaw time (CET, UTC+1): 08:00Z-12:00Z.
//! "Classic" takes 60 minutes plus a 10 minute buffer on a 30 minute grid.

mod common;

use axum::http::{Method, StatusCode};
use common::{add_member, app_at, call, create_business, my_staff_id, sign_up};
use serde_json::{json, Value};
use sqlx::PgPool;
use time::macros::datetime;
use time::{Duration, OffsetDateTime};

const NOW: OffsetDateTime = datetime!(2027-03-20 09:00 UTC);

struct Shop {
    app: axum::Router,
    token: String,
    biz: String,
    base: String,
    staff: String,
    service: String,
    variant: String,
}

async fn shop(pool: &PgPool, service_extra: Value) -> Shop {
    let app = app_at(pool, NOW).await;
    let (token, _) = sign_up(&app, "anna@example.pl").await;
    let biz = create_business(&app, &token, "Anna Lashes").await;
    let staff = my_staff_id(&app, &token, &biz).await;
    let base = format!("/v1/businesses/{biz}");

    let mut body = json!({
        "name": { "pl": "Classic" },
        "booking_step_minutes": 30,
        "buffer_after_min": 10,
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
    let (status, body) = call(
        &app,
        Method::PUT,
        &format!("{base}/staff/{staff}/schedule/weekly"),
        Some(&token),
        Some(json!({ "intervals": [{ "weekday": 0, "start": "09:00", "end": "13:00" }] })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    Shop {
        app,
        token,
        biz,
        base,
        staff,
        service: service_id,
        variant,
    }
}

impl Shop {
    fn hold_body(&self, staff: &str, start: &str) -> Value {
        json!({
            "service_id": self.service, "variant_id": self.variant,
            "staff_id": staff, "start_at": start
        })
    }

    async fn hold_as(
        &self,
        app: &axum::Router,
        token: &str,
        key: &str,
        staff: &str,
        start: &str,
    ) -> (StatusCode, Value) {
        call_with_key(
            app,
            Method::POST,
            &format!("{}/holds", self.base),
            token,
            Some(key),
            Some(self.hold_body(staff, start)),
        )
        .await
    }

    async fn hold(&self, key: &str, start: &str) -> (StatusCode, Value) {
        self.hold_as(&self.app, &self.token, key, &self.staff, start)
            .await
    }

    async fn slots(&self, app: &axum::Router, date: &str) -> Vec<String> {
        let uri = format!(
            "{}/availability?service_id={}&variant_id={}&from={date}",
            self.base, self.service, self.variant
        );
        let (status, body) = call(app, Method::GET, &uri, Some(&self.token), None).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body["slots"]
            .as_array()
            .unwrap()
            .iter()
            .map(|slot| slot["start_at"].as_str().unwrap().to_string())
            .collect()
    }
}

/// Like `common::call`, with an Idempotency-Key header (the shared helper has
/// no way to add headers).
async fn call_with_key(
    app: &axum::Router,
    method: Method,
    uri: &str,
    token: &str,
    key: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    use axum::body::Body;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let mut request = axum::http::Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {token}"));
    if let Some(key) = key {
        request = request.header("idempotency-key", key);
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
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[sqlx::test(migrations = "./migrations")]
async fn a_hold_blocks_the_slot_and_can_be_released(pool: PgPool) {
    let shop = shop(&pool, json!({})).await;
    let before = shop.slots(&shop.app, "2027-03-22").await;
    assert_eq!(before[0], "2027-03-22T08:00:00Z");

    let (status, hold) = shop.hold("hold-key-0001", "2027-03-22T08:00:00Z").await;
    assert_eq!(status, StatusCode::CREATED, "{hold}");
    assert_eq!(hold["status"], "held");
    assert_eq!(hold["start_at"], "2027-03-22T08:00:00Z");
    assert_eq!(hold["end_at"], "2027-03-22T09:00:00Z");
    assert_eq!(hold["hold_expires_at"], "2027-03-20T09:10:00Z");
    assert_eq!(hold["staff_id"], shop.staff.as_str());
    assert_eq!(hold["source"], "manual");
    let id = hold["id"].as_str().unwrap().to_string();

    // The service (08:00-09:00Z) plus its 10 minute buffer are taken:
    // 08:00, 08:30 and 09:00 are gone, 09:30 is the first free start.
    let after = shop.slots(&shop.app, "2027-03-22").await;
    assert_eq!(after[0], "2027-03-22T09:30:00Z");
    assert_eq!(after.len(), before.len() - 3);

    // Retrying the same request returns the same hold, without a second one.
    let (status, again) = shop.hold("hold-key-0001", "2027-03-22T08:00:00Z").await;
    assert_eq!(status, StatusCode::OK, "{again}");
    assert_eq!(again["id"], id.as_str());
    // The same key for a different request is refused.
    let (status, body) = shop.hold("hold-key-0001", "2027-03-22T09:30:00Z").await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    let held: i64 = sqlx::query_scalar("SELECT count(*) FROM appointment")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(held, 1);

    // Read it back; snapshot of the booked item was stored.
    let (status, got) = call(
        &shop.app,
        Method::GET,
        &format!("{}/holds/{id}", shop.base),
        Some(&shop.token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{got}");
    assert_eq!(got["status"], "held");
    let price: i64 = sqlx::query_scalar("SELECT price_minor FROM appointment_item")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(price, 25000);

    // Releasing frees the slot; releasing twice is harmless.
    for _ in 0..2 {
        let (status, _) = call(
            &shop.app,
            Method::DELETE,
            &format!("{}/holds/{id}", shop.base),
            Some(&shop.token),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NO_CONTENT);
    }
    assert_eq!(shop.slots(&shop.app, "2027-03-22").await, before);
    let (_, got) = call(
        &shop.app,
        Method::GET,
        &format!("{}/holds/{id}", shop.base),
        Some(&shop.token),
        None,
    )
    .await;
    assert_eq!(got["status"], "expired");
    let events: Vec<String> =
        sqlx::query_scalar("SELECT type FROM appointment_event ORDER BY created_at, type")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(events, ["hold_created", "hold_released"]);
}

#[sqlx::test(migrations = "./migrations")]
async fn an_expired_hold_frees_its_slot(pool: PgPool) {
    let shop = shop(&pool, json!({})).await;
    let (status, hold) = shop.hold("hold-key-0001", "2027-03-22T08:00:00Z").await;
    assert_eq!(status, StatusCode::CREATED, "{hold}");
    let id = hold["id"].as_str().unwrap().to_string();

    // Nine minutes later the hold still blocks (10 minute TTL) ...
    let almost = app_at(&pool, NOW + Duration::minutes(9)).await;
    assert_eq!(
        shop.slots(&almost, "2027-03-22").await[0],
        "2027-03-22T09:30:00Z"
    );
    let (status, _) = shop
        .hold_as(
            &almost,
            &shop.token,
            "hold-key-0002",
            &shop.staff,
            "2027-03-22T08:00:00Z",
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // ... eleven minutes later it is over: the slot is offered and bookable.
    let later = app_at(&pool, NOW + Duration::minutes(11)).await;
    assert_eq!(
        shop.slots(&later, "2027-03-22").await[0],
        "2027-03-22T08:00:00Z"
    );
    let (_, old) = call(
        &later,
        Method::GET,
        &format!("{}/holds/{id}", shop.base),
        Some(&shop.token),
        None,
    )
    .await;
    assert_eq!(old["status"], "expired");
    let (status, fresh) = shop
        .hold_as(
            &later,
            &shop.token,
            "hold-key-0003",
            &shop.staff,
            "2027-03-22T08:00:00Z",
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{fresh}");
    assert_ne!(fresh["id"], id.as_str());

    // The old row was marked expired and the fact recorded.
    let status: String =
        sqlx::query_scalar("SELECT status FROM appointment WHERE id = $1::text::uuid")
            .bind(&id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "expired");
    let expired_events: i64 =
        sqlx::query_scalar("SELECT count(*) FROM appointment_event WHERE type = 'hold_expired'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(expired_events, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn only_offered_times_can_be_held(pool: PgPool) {
    let shop = shop(&pool, json!({})).await;
    let (status, first) = shop.hold("hold-key-0001", "2027-03-22T08:00:00Z").await;
    assert_eq!(status, StatusCode::CREATED, "{first}");

    // Overlapping, inside the buffer, off-grid, outside hours, in the past.
    for (n, start) in [
        "2027-03-22T08:30:00Z", // overlaps the held service
        "2027-03-22T09:00:00Z", // inside the buffer after it
        "2027-03-22T09:35:00Z", // off the 30 minute grid
        "2027-03-22T05:00:00Z", // 06:00 local, before work
        "2027-03-22T12:00:00Z", // 13:00 local, after work
        "2027-03-21T08:00:00Z", // a day without hours
        "2027-03-19T08:00:00Z", // in the past
    ]
    .into_iter()
    .enumerate()
    {
        let (status, body) = shop.hold(&format!("reject-key-{n:04}"), start).await;
        assert_eq!(status, StatusCode::CONFLICT, "{start} {body}");
        assert_eq!(body["code"], "SLOT_UNAVAILABLE", "{start} {body}");
    }
    // Right after the buffer is fine.
    let (status, body) = shop.hold("hold-key-0002", "2027-03-22T09:30:00Z").await;
    assert_eq!(status, StatusCode::CREATED, "{body}");

    // Even around the API the database refuses an overlap.
    let overlap = sqlx::query(
        "INSERT INTO appointment (business_id, staff_id, status, start_at, end_at, blocked_end,
                                 client_name, confirmed_at)
         VALUES ($1::text::uuid, $2::text::uuid, 'confirmed',
                 '2027-03-22T08:15:00Z', '2027-03-22T09:00:00Z', '2027-03-22T09:00:00Z',
                 'Direct SQL', now())",
    )
    .bind(&shop.biz)
    .bind(&shop.staff)
    .execute(&pool)
    .await;
    let code = overlap
        .unwrap_err()
        .as_database_error()
        .and_then(|error| error.code().map(|code| code.to_string()));
    assert_eq!(code.as_deref(), Some("23P01"));
}

#[sqlx::test(migrations = "./migrations")]
async fn hold_requests_are_validated(pool: PgPool) {
    let shop = shop(&pool, json!({ "is_online_bookable": false })).await;
    let uri = format!("{}/holds", shop.base);
    let good = shop.hold_body(&shop.staff, "2027-03-22T08:00:00Z");

    // Idempotency-Key: missing, too short.
    for key in [None, Some("short")] {
        let (status, body) = call_with_key(
            &shop.app,
            Method::POST,
            &uri,
            &shop.token,
            key,
            Some(good.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{key:?} {body}");
    }
    // Bad start, bad source, an online source for a service closed to online.
    for (n, body) in [
        json!({ "start_at": "tomorrow" }),
        json!({ "source": "phone" }),
        json!({ "source": "app" }),
        json!({ "source": "web" }),
    ]
    .into_iter()
    .enumerate()
    {
        let mut request = good.clone();
        for (key, value) in body.as_object().unwrap() {
            request[key] = value.clone();
        }
        let (status, response) = call_with_key(
            &shop.app,
            Method::POST,
            &uri,
            &shop.token,
            Some(&format!("valid-key-{n:04}")),
            Some(request),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body} {response}");
    }
    // Unknown service, variant, staff.
    let unknown = uuid::Uuid::new_v4().to_string();
    for field in ["service_id", "variant_id", "staff_id"] {
        let mut request = good.clone();
        request[field] = json!(unknown);
        let (status, body) = call_with_key(
            &shop.app,
            Method::POST,
            &uri,
            &shop.token,
            Some("unknown-key-0001"),
            Some(request),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{field} {body}");
    }
    // Nothing was created.
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM appointment")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    // The manual channel works for that service.
    let (status, body) = shop.hold("manual-key-0001", "2027-03-22T08:00:00Z").await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
}

#[sqlx::test(migrations = "./migrations")]
async fn one_user_cannot_hoard_slots(pool: PgPool) {
    let shop = shop(&pool, json!({})).await;
    // Work every day so there are plenty of separate slots.
    let intervals: Vec<Value> = (0..7)
        .map(|weekday| json!({ "weekday": weekday, "start": "09:00", "end": "13:00" }))
        .collect();
    let (status, _) = call(
        &shop.app,
        Method::PUT,
        &format!("{}/staff/{}/schedule/weekly", shop.base, shop.staff),
        Some(&shop.token),
        Some(json!({ "intervals": intervals })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let mut created = 0;
    for day in 22..=27 {
        for time in ["08:00", "09:30"] {
            let (status, body) = shop
                .hold(
                    &format!("hoard-key-{day}-{time}"),
                    &format!("2027-03-{day}T{time}:00Z"),
                )
                .await;
            if created < 10 {
                assert_eq!(status, StatusCode::CREATED, "{day} {time} {body}");
                created += 1;
            } else {
                assert_eq!(status, StatusCode::TOO_MANY_REQUESTS, "{day} {time} {body}");
            }
        }
    }
    assert_eq!(created, 10);
}

#[sqlx::test(migrations = "./migrations")]
async fn roles_and_tenants_are_respected(pool: PgPool) {
    let shop = shop(&pool, json!({})).await;
    let (olga, olga_id) = sign_up(&shop.app, "olga@example.pl").await;
    let olga_staff = add_member(&pool, &shop.biz, &olga_id, "employee").await;
    let (rita, rita_id) = sign_up(&shop.app, "rita@example.pl").await;
    add_member(&pool, &shop.biz, &rita_id, "reception").await;

    // Olga also performs the service and works on Mondays.
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
        Some(json!({ "intervals": [{ "weekday": 0, "start": "09:00", "end": "13:00" }] })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // An employee books only their own calendar.
    let (status, body) = shop
        .hold_as(
            &shop.app,
            &olga,
            "olga-key-0001",
            &olga_staff,
            "2027-03-22T08:00:00Z",
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let olga_hold = body["id"].as_str().unwrap().to_string();
    let (status, _) = shop
        .hold_as(
            &shop.app,
            &olga,
            "olga-key-0002",
            &shop.staff,
            "2027-03-22T08:00:00Z",
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Reception books any master; the same slot of two masters is independent.
    let (status, body) = shop
        .hold_as(
            &shop.app,
            &rita,
            "rita-key-0001",
            &shop.staff,
            "2027-03-22T08:00:00Z",
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let anna_hold = body["id"].as_str().unwrap().to_string();

    // An employee cannot read or release someone else's calendar hold.
    for method in [Method::GET, Method::DELETE] {
        let (status, _) = call(
            &shop.app,
            method,
            &format!("{}/holds/{anna_hold}", shop.base),
            Some(&olga),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
    }
    let (status, _) = call(
        &shop.app,
        Method::GET,
        &format!("{}/holds/{olga_hold}", shop.base),
        Some(&olga),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // A stranger's business sees none of it, and our ids do not cross over.
    let (zed, _) = sign_up(&shop.app, "zed@example.pl").await;
    let other = create_business(&shop.app, &zed, "Zed Studio").await;
    let (status, _) = shop
        .hold_as(
            &shop.app,
            &zed,
            "zed-key-00001",
            &shop.staff,
            "2027-03-22T08:00:00Z",
        )
        .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "not a member of Anna's business"
    );
    let (status, _) = call_with_key(
        &shop.app,
        Method::POST,
        &format!("/v1/businesses/{other}/holds"),
        &zed,
        Some("zed-key-00002"),
        Some(shop.hold_body(&shop.staff, "2027-03-22T08:00:00Z")),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "our staff and service in their business"
    );
    let (status, _) = call(
        &shop.app,
        Method::GET,
        &format!("/v1/businesses/{other}/holds/{anna_hold}"),
        Some(&zed),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = call_with_key(
        &shop.app,
        Method::POST,
        &format!("{}/holds", shop.base),
        "not-a-token",
        Some("anon-key-00001"),
        Some(shop.hold_body(&shop.staff, "2027-03-22T08:00:00Z")),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "./migrations")]
async fn two_simultaneous_requests_for_one_slot_book_it_once(pool: PgPool) {
    let shop = shop(&pool, json!({})).await;
    let (rita, rita_id) = sign_up(&shop.app, "rita@example.pl").await;
    add_member(&pool, &shop.biz, &rita_id, "reception").await;

    // Owner and reception click on the same free slot at the same moment.
    let slots = [
        "2027-03-22T08:00:00Z",
        "2027-03-22T09:30:00Z",
        "2027-03-22T11:00:00Z",
    ];
    for (n, slot) in slots.into_iter().enumerate() {
        let (a, b) = tokio::join!(
            shop.hold_as(
                &shop.app,
                &shop.token,
                &format!("owner-key-{n:04}"),
                &shop.staff,
                slot
            ),
            shop.hold_as(
                &shop.app,
                &rita,
                &format!("rita-key-{n:04}"),
                &shop.staff,
                slot
            ),
        );
        let mut statuses = [a.0, b.0];
        statuses.sort();
        assert_eq!(
            statuses,
            [StatusCode::CREATED, StatusCode::CONFLICT],
            "slot {slot}: {} / {}",
            a.1,
            b.1
        );
        let loser = if a.0 == StatusCode::CONFLICT {
            &a.1
        } else {
            &b.1
        };
        assert_eq!(loser["code"], "SLOT_UNAVAILABLE", "{loser}");
    }
    let held: i64 = sqlx::query_scalar("SELECT count(*) FROM appointment WHERE status = 'held'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(held, 3, "exactly one booking per slot");
}

#[sqlx::test(migrations = "./migrations")]
async fn a_retry_racing_its_original_request_is_not_a_conflict(pool: PgPool) {
    let shop = shop(&pool, json!({})).await;
    // The very same request (same key) sent twice at once, e.g. a double tap.
    let (a, b) = tokio::join!(
        shop.hold("double-tap-0001", "2027-03-22T08:00:00Z"),
        shop.hold("double-tap-0001", "2027-03-22T08:00:00Z"),
    );
    let mut statuses = [a.0, b.0];
    statuses.sort();
    assert_eq!(
        statuses,
        [StatusCode::OK, StatusCode::CREATED],
        "{} / {}",
        a.1,
        b.1
    );
    assert_eq!(a.1["id"], b.1["id"]);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM appointment")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}
