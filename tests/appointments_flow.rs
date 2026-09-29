//! End-to-end tests of confirmed appointments, cancellation and rescheduling.
//!
//! The clock is 2027-03-20 (a Saturday) 09:00 UTC. Anna works Monday
//! 2027-03-22 and Tuesday 2027-03-23, 09:00-13:00 Warsaw time (CET, UTC+1),
//! i.e. 08:00Z-12:00Z. "Classic" takes 60 minutes plus a 10 minute buffer on a
//! 30 minute grid.

mod common;

use axum::http::{Method, StatusCode};
use common::{add_member, app_at, call, create_business, my_staff_id, sign_up};
use serde_json::{json, Value};
use sqlx::PgPool;
use time::macros::datetime;
use time::OffsetDateTime;

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

async fn shop(pool: &PgPool) -> Shop {
    let app = app_at(pool, NOW).await;
    let (token, _) = sign_up(&app, "anna@example.pl").await;
    let biz = create_business(&app, &token, "Anna Lashes").await;
    let staff = my_staff_id(&app, &token, &biz).await;
    let base = format!("/v1/businesses/{biz}");
    let (status, service) = call(
        &app,
        Method::POST,
        &format!("{base}/services"),
        Some(&token),
        Some(json!({
            "name": { "pl": "Classic" },
            "booking_step_minutes": 30,
            "buffer_after_min": 10,
            "min_notice_min": 0,
            "variants": [{ "duration_min": 60, "price_minor": 25000 }]
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{service}");
    let service_id = service["id"].as_str().unwrap().to_string();
    let variant = service["variants"][0]["id"].as_str().unwrap().to_string();
    for (uri, body) in [
        (
            format!("{base}/services/{service_id}/staff"),
            json!({ "staff_ids": [staff] }),
        ),
        (
            format!("{base}/staff/{staff}/schedule/weekly"),
            json!({ "intervals": [
                { "weekday": 0, "start": "09:00", "end": "13:00" },
                { "weekday": 1, "start": "09:00", "end": "13:00" }
            ]}),
        ),
    ] {
        let (status, response) = call(&app, Method::PUT, &uri, Some(&token), Some(body)).await;
        assert_eq!(status, StatusCode::OK, "{response}");
    }
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

/// A request with an optional Idempotency-Key and any JSON body.
async fn send(
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

impl Shop {
    fn booking(&self, staff: &str, start: &str, client: &str) -> Value {
        json!({
            "service_id": self.service, "variant_id": self.variant, "staff_id": staff,
            "start_at": start, "client_name": client
        })
    }

    async fn book_as(
        &self,
        app: &axum::Router,
        token: &str,
        key: &str,
        staff: &str,
        start: &str,
    ) -> (StatusCode, Value) {
        send(
            app,
            Method::POST,
            &format!("{}/appointments", self.base),
            token,
            Some(key),
            Some(self.booking(staff, start, "Ewa Nowak")),
        )
        .await
    }

    async fn book(&self, key: &str, start: &str) -> (StatusCode, Value) {
        self.book_as(&self.app, &self.token, key, &self.staff, start)
            .await
    }

    async fn hold(&self, key: &str, start: &str) -> (StatusCode, Value) {
        send(
            &self.app,
            Method::POST,
            &format!("{}/holds", self.base),
            &self.token,
            Some(key),
            Some(json!({
                "service_id": self.service, "variant_id": self.variant,
                "staff_id": self.staff, "start_at": start
            })),
        )
        .await
    }

    async fn post(&self, app: &axum::Router, path: &str, body: Value) -> (StatusCode, Value) {
        send(
            app,
            Method::POST,
            &format!("{}{path}", self.base),
            &self.token,
            None,
            Some(body),
        )
        .await
    }

    async fn get(&self, app: &axum::Router, path: &str) -> (StatusCode, Value) {
        send(
            app,
            Method::GET,
            &format!("{}{path}", self.base),
            &self.token,
            None,
            None,
        )
        .await
    }

    async fn slots(&self, date: &str) -> Vec<String> {
        let (status, body) = self
            .get(
                &self.app,
                &format!(
                    "/availability?service_id={}&variant_id={}&from={date}",
                    self.service, self.variant
                ),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body["slots"]
            .as_array()
            .unwrap()
            .iter()
            .map(|slot| slot["start_at"].as_str().unwrap().to_string())
            .collect()
    }

    async fn events(&self, id: &str) -> Vec<String> {
        let (status, body) = self
            .get(&self.app, &format!("/appointments/{id}/history"))
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body.as_array()
            .unwrap()
            .iter()
            .map(|event| event["type"].as_str().unwrap().to_string())
            .collect()
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn direct_booking_stores_client_price_and_history(pool: PgPool) {
    let shop = shop(&pool).await;
    let (status, booking) = send(
        &shop.app,
        Method::POST,
        &format!("{}/appointments", shop.base),
        &shop.token,
        Some("book-key-0001"),
        Some(json!({
            "service_id": shop.service, "variant_id": shop.variant, "staff_id": shop.staff,
            "start_at": "2027-03-22T08:00:00Z",
            "client_name": " Ewa Nowak ", "client_phone": "+48 600-100-200",
            "note": "Prefers a natural look"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{booking}");
    assert_eq!(booking["status"], "confirmed");
    assert_eq!(booking["client_name"], "Ewa Nowak");
    assert_eq!(booking["client_phone"], "+48600100200");
    assert_eq!(booking["note"], "Prefers a natural look");
    assert_eq!(booking["start_at"], "2027-03-22T08:00:00Z");
    assert_eq!(booking["end_at"], "2027-03-22T09:00:00Z");
    assert_eq!(booking["price_minor"], 25000);
    assert_eq!(booking["currency"], "PLN");
    assert_eq!(booking["duration_min"], 60);
    assert_eq!(booking["source"], "manual");
    assert_eq!(booking["confirmed_at"], "2027-03-20T09:00:00Z");
    assert!(booking["hold_expires_at"].is_null());
    assert_eq!(booking["version"], 1);
    let id = booking["id"].as_str().unwrap().to_string();

    // The booking blocks the calendar like a hold: service plus buffer.
    assert_eq!(shop.slots("2027-03-22").await[0], "2027-03-22T09:30:00Z");

    // Retrying with the same key returns it; another request is refused.
    let (status, again) = shop.book("book-key-0001", "2027-03-22T08:00:00Z").await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "different client details: {again}"
    );
    let (status, replay) = send(
        &shop.app,
        Method::POST,
        &format!("{}/appointments", shop.base),
        &shop.token,
        Some("book-key-0001"),
        Some(json!({
            "service_id": shop.service, "variant_id": shop.variant, "staff_id": shop.staff,
            "start_at": "2027-03-22T08:00:00Z",
            "client_name": " Ewa Nowak ", "client_phone": "+48 600-100-200",
            "note": "Prefers a natural look"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{replay}");
    assert_eq!(replay["id"], id.as_str());

    // Read back, by id and as a hold-shaped id, with its history.
    let (status, got) = shop.get(&shop.app, &format!("/appointments/{id}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(got["client_name"], "Ewa Nowak");
    assert_eq!(shop.events(&id).await, ["booked"]);

    // Who booked it, and the price snapshot survives a later price change.
    let by: Option<String> = sqlx::query_scalar(
        "SELECT booked_by_user_id::text FROM appointment WHERE id = $1::text::uuid",
    )
    .bind(&id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(by.is_some());
    let (status, _) = call(
        &shop.app,
        Method::PATCH,
        &format!("{}/variants/{}", shop.base, shop.variant),
        Some(&shop.token),
        Some(json!({ "price_minor": 99900 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, got) = shop.get(&shop.app, &format!("/appointments/{id}")).await;
    assert_eq!(got["price_minor"], 25000);
}

#[sqlx::test(migrations = "./migrations")]
async fn booking_requests_are_validated(pool: PgPool) {
    let shop = shop(&pool).await;
    let uri = format!("{}/appointments", shop.base);
    let good = shop.booking(&shop.staff, "2027-03-22T08:00:00Z", "Ewa");

    // Missing key for a direct booking; bad client data.
    let (status, _) = send(
        &shop.app,
        Method::POST,
        &uri,
        &shop.token,
        None,
        Some(good.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let cases = [
        json!({ "client_name": "  " }),
        json!({ "client_name": "x".repeat(121) }),
        json!({ "client_phone": "600100200" }),
        json!({ "note": "x".repeat(501) }),
        json!({ "start_at": "soon" }),
        json!({ "source": "phone" }),
        json!({ "hold_id": uuid::Uuid::new_v4() }), // hold_id + slot fields
    ];
    for (n, patch) in cases.into_iter().enumerate() {
        let mut body = good.clone();
        for (key, value) in patch.as_object().unwrap() {
            body[key] = value.clone();
        }
        let (status, response) = send(
            &shop.app,
            Method::POST,
            &uri,
            &shop.token,
            Some(&format!("bad-key-{n:04}")),
            Some(body),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{patch} {response}");
    }
    // Neither a hold nor a full slot description.
    let (status, _) = send(
        &shop.app,
        Method::POST,
        &uri,
        &shop.token,
        Some("bad-key-9999"),
        Some(json!({ "client_name": "Ewa", "service_id": shop.service })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    // Not offered: 409; unknown ids: 404.
    let (status, body) = shop.book("late-key-0001", "2027-03-22T12:00:00Z").await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "SLOT_UNAVAILABLE");
    let unknown = uuid::Uuid::new_v4().to_string();
    let (status, _) = shop
        .book_as(
            &shop.app,
            &shop.token,
            "ghost-key-0001",
            &unknown,
            "2027-03-22T08:00:00Z",
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM appointment")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0, "nothing was stored");

    // The database itself refuses a confirmed row without a client.
    let bad = sqlx::query(
        "INSERT INTO appointment (business_id, staff_id, status, start_at, end_at, blocked_end)
         VALUES ($1::text::uuid, $2::text::uuid, 'confirmed',
                 '2027-03-22T08:00:00Z', '2027-03-22T09:00:00Z', '2027-03-22T09:00:00Z')",
    )
    .bind(&shop.biz)
    .bind(&shop.staff)
    .execute(&pool)
    .await;
    let code = bad
        .unwrap_err()
        .as_database_error()
        .and_then(|error| error.code().map(|code| code.to_string()));
    assert_eq!(code.as_deref(), Some("23514"));
}

#[sqlx::test(migrations = "./migrations")]
async fn a_hold_is_confirmed_once_and_only_while_it_lives(pool: PgPool) {
    let shop = shop(&pool).await;
    let (status, hold) = shop.hold("hold-key-0001", "2027-03-22T09:30:00Z").await;
    assert_eq!(status, StatusCode::CREATED, "{hold}");
    let hold_id = hold["id"].as_str().unwrap().to_string();

    let confirm =
        json!({ "hold_id": hold_id, "client_name": "Ewa Nowak", "client_phone": "+48600100200" });
    let (status, booked) = shop.post(&shop.app, "/appointments", confirm.clone()).await;
    assert_eq!(status, StatusCode::CREATED, "{booked}");
    assert_eq!(booked["id"], hold_id.as_str());
    assert_eq!(booked["status"], "confirmed");
    assert!(booked["hold_expires_at"].is_null());
    assert_eq!(booked["start_at"], "2027-03-22T09:30:00Z");

    // Confirming again with the same details is a replay; other details conflict.
    let (status, replay) = shop.post(&shop.app, "/appointments", confirm).await;
    assert_eq!(status, StatusCode::OK, "{replay}");
    let (status, _) = shop
        .post(
            &shop.app,
            "/appointments",
            json!({ "hold_id": hold_id, "client_name": "Someone Else" }),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(shop.events(&hold_id).await, ["hold_created", "confirmed"]);

    // A confirmed appointment is not a hold: it cannot be released as one.
    let (status, _) = send(
        &shop.app,
        Method::DELETE,
        &format!("{}/holds/{hold_id}", shop.base),
        &shop.token,
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // Unknown hold.
    let (status, _) = shop
        .post(
            &shop.app,
            "/appointments",
            json!({ "hold_id": uuid::Uuid::new_v4(), "client_name": "Ewa" }),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // A hold that ran out cannot be confirmed; neither can a released one.
    let (_, old) = shop.hold("hold-key-0002", "2027-03-22T11:00:00Z").await;
    let old_id = old["id"].as_str().unwrap().to_string();
    let later = app_at(&pool, NOW + time::Duration::minutes(11)).await;
    let (status, body) = shop
        .post(
            &later,
            "/appointments",
            json!({ "hold_id": old_id, "client_name": "Ewa" }),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["code"], "SLOT_UNAVAILABLE");
    let (_, released) = shop.hold("hold-key-0003", "2027-03-23T08:00:00Z").await;
    let released_id = released["id"].as_str().unwrap().to_string();
    let (status, _) = send(
        &shop.app,
        Method::DELETE,
        &format!("{}/holds/{released_id}", shop.base),
        &shop.token,
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = shop
        .post(
            &shop.app,
            "/appointments",
            json!({ "hold_id": released_id, "client_name": "Ewa" }),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[sqlx::test(migrations = "./migrations")]
async fn the_calendar_lists_appointments_by_local_date(pool: PgPool) {
    let shop = shop(&pool).await;
    for (key, start) in [
        ("list-key-0001", "2027-03-22T09:30:00Z"),
        ("list-key-0002", "2027-03-22T08:00:00Z"),
        ("list-key-0003", "2027-03-23T08:00:00Z"),
    ] {
        let (status, body) = shop.book(key, start).await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
    }
    let (_, hold) = shop.hold("list-hold-0001", "2027-03-22T11:00:00Z").await;
    assert_eq!(hold["status"], "held");

    let (status, monday) = shop.get(&shop.app, "/appointments?from=2027-03-22").await;
    assert_eq!(status, StatusCode::OK, "{monday}");
    let starts: Vec<&str> = monday
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["start_at"].as_str().unwrap())
        .collect();
    assert_eq!(
        starts,
        ["2027-03-22T08:00:00Z", "2027-03-22T09:30:00Z"],
        "oldest first, holds excluded"
    );

    let (_, both) = shop
        .get(
            &shop.app,
            &format!(
                "/appointments?from=2027-03-22&to=2027-03-23&staff_id={}",
                shop.staff
            ),
        )
        .await;
    assert_eq!(both.as_array().unwrap().len(), 3);
    let (_, holds) = shop
        .get(&shop.app, "/appointments?from=2027-03-22&status=held")
        .await;
    assert_eq!(holds.as_array().unwrap().len(), 1);
    let (_, none) = shop.get(&shop.app, "/appointments?from=2027-03-24").await;
    assert_eq!(none, json!([]));
    let (_, cancelled) = shop
        .get(&shop.app, "/appointments?from=2027-03-22&status=cancelled")
        .await;
    assert_eq!(cancelled, json!([]));

    for uri in [
        "/appointments?from=22-03-2027",
        "/appointments?from=2027-03-23&to=2027-03-22",
        "/appointments?from=2027-03-01&to=2027-04-15",
        "/appointments?from=2027-03-22&status=done",
    ] {
        let (status, _) = shop.get(&shop.app, uri).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{uri}");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn cancelling_frees_the_slot_and_marks_late_ones(pool: PgPool) {
    let shop = shop(&pool).await;
    let (_, early) = shop.book("cancel-key-0001", "2027-03-22T08:00:00Z").await;
    let early_id = early["id"].as_str().unwrap().to_string();
    let path = format!("/appointments/{early_id}/cancel");

    // 47 hours ahead: free cancellation.
    let (status, cancelled) = shop
        .post(&shop.app, &path, json!({ "reason": "Client is ill" }))
        .await;
    assert_eq!(status, StatusCode::OK, "{cancelled}");
    assert_eq!(cancelled["status"], "cancelled");
    assert_eq!(cancelled["late_cancellation"], false);
    assert_eq!(cancelled["cancel_reason"], "Client is ill");
    assert_eq!(cancelled["cancelled_at"], "2027-03-20T09:00:00Z");
    assert_eq!(
        shop.slots("2027-03-22").await[0],
        "2027-03-22T08:00:00Z",
        "slot is free again"
    );

    // Cancelling twice is harmless, and the body is optional.
    let (status, again) = send(
        &shop.app,
        Method::POST,
        &format!("{}{path}", shop.base),
        &shop.token,
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(again["status"], "cancelled");
    assert_eq!(shop.events(&early_id).await, ["booked", "cancelled"]);
    let (_, history) = shop
        .get(&shop.app, &format!("/appointments/{early_id}/history"))
        .await;
    assert_eq!(history[1]["data"]["reason"], "Client is ill");

    // The freed slot can be booked again; a cancelled row cannot be moved.
    let (status, _) = shop.book("cancel-key-0002", "2027-03-22T08:00:00Z").await;
    assert_eq!(status, StatusCode::CREATED);
    let (status, _) = shop
        .post(
            &shop.app,
            &format!("/appointments/{early_id}/reschedule"),
            json!({ "start_at": "2027-03-23T08:00:00Z" }),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // 12 hours before the start: cancelled, but flagged as late.
    let (_, close) = shop.book("cancel-key-0003", "2027-03-22T09:30:00Z").await;
    let close_id = close["id"].as_str().unwrap().to_string();
    let evening = app_at(&pool, datetime!(2027-03-21 21:00 UTC)).await;
    let (status, late) = shop
        .post(
            &evening,
            &format!("/appointments/{close_id}/cancel"),
            json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{late}");
    assert_eq!(late["late_cancellation"], true);

    // After the start it is too late to cancel; a hold is released, not cancelled.
    let (_, started) = shop.book("cancel-key-0004", "2027-03-22T11:00:00Z").await;
    let started_id = started["id"].as_str().unwrap().to_string();
    let after = app_at(&pool, datetime!(2027-03-22 11:30 UTC)).await;
    let (status, _) = shop
        .post(
            &after,
            &format!("/appointments/{started_id}/cancel"),
            json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (_, hold) = shop.hold("cancel-hold-0001", "2027-03-23T08:00:00Z").await;
    let (status, _) = shop
        .post(
            &shop.app,
            &format!("/appointments/{}/cancel", hold["id"].as_str().unwrap()),
            json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (status, _) = shop
        .post(
            &shop.app,
            &format!("/appointments/{}/cancel", uuid::Uuid::new_v4()),
            json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "./migrations")]
async fn rescheduling_moves_the_same_appointment(pool: PgPool) {
    let shop = shop(&pool).await;
    let (_, booked) = shop.book("move-key-00001", "2027-03-22T08:00:00Z").await;
    let id = booked["id"].as_str().unwrap().to_string();
    let path = format!("/appointments/{id}/reschedule");

    // To Tuesday: same id, new time, the old slot is free, the new one is taken.
    let (status, moved) = shop
        .post(
            &shop.app,
            &path,
            json!({ "start_at": "2027-03-23T08:00:00Z", "reason": "Client asked" }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{moved}");
    assert_eq!(moved["id"], id.as_str());
    assert_eq!(moved["start_at"], "2027-03-23T08:00:00Z");
    assert_eq!(moved["end_at"], "2027-03-23T09:00:00Z");
    assert_eq!(moved["status"], "confirmed");
    assert_eq!(moved["version"], 2);
    assert_eq!(moved["client_name"], "Ewa Nowak");
    assert_eq!(shop.slots("2027-03-22").await[0], "2027-03-22T08:00:00Z");
    assert_eq!(shop.slots("2027-03-23").await[0], "2027-03-23T09:30:00Z");
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM appointment")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(rows, 1, "the same row was moved, no copy");

    // Moving by half an hour overlaps the appointment's own old time: allowed.
    let (status, nudged) = shop
        .post(
            &shop.app,
            &path,
            json!({ "start_at": "2027-03-23T08:30:00Z" }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{nudged}");

    // History keeps every move with where it came from.
    assert_eq!(
        shop.events(&id).await,
        ["booked", "rescheduled", "rescheduled"]
    );
    let (_, history) = shop
        .get(&shop.app, &format!("/appointments/{id}/history"))
        .await;
    assert_eq!(
        history[1]["data"]["from"]["start_at"],
        "2027-03-22T08:00:00Z"
    );
    assert_eq!(history[1]["data"]["to"]["start_at"], "2027-03-23T08:00:00Z");
    assert_eq!(history[1]["data"]["reason"], "Client asked");

    // Refused: no change, off the grid, taken, outside hours, past.
    let (_, other) = shop.book("move-key-00002", "2027-03-22T08:00:00Z").await;
    assert_eq!(other["status"], "confirmed");
    for (start, want) in [
        ("2027-03-23T08:30:00Z", StatusCode::BAD_REQUEST), // nothing to change
        ("2027-03-23T09:05:00Z", StatusCode::CONFLICT),    // off the grid
        ("2027-03-22T08:30:00Z", StatusCode::CONFLICT),    // overlaps the other booking
        ("2027-03-23T12:30:00Z", StatusCode::CONFLICT),    // after work
        ("2027-03-19T08:00:00Z", StatusCode::CONFLICT),    // in the past
    ] {
        let (status, body) = shop
            .post(&shop.app, &path, json!({ "start_at": start }))
            .await;
        assert_eq!(status, want, "{start} {body}");
    }
    let (_, still) = shop.get(&shop.app, &format!("/appointments/{id}")).await;
    assert_eq!(
        still["start_at"], "2027-03-23T08:30:00Z",
        "refused moves change nothing"
    );

    // Unknown master; a master who does not perform the service.
    let (status, _) = shop
        .post(
            &shop.app,
            &path,
            json!({ "start_at": "2027-03-23T10:00:00Z", "staff_id": uuid::Uuid::new_v4() }),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "./migrations")]
async fn an_appointment_can_move_to_another_master(pool: PgPool) {
    let shop = shop(&pool).await;
    let (_, olga_id) = sign_up(&shop.app, "olga@example.pl").await;
    let olga_staff = add_member(&pool, &shop.biz, &olga_id, "employee").await;
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

    let (_, booked) = shop.book("staff-key-0001", "2027-03-22T08:00:00Z").await;
    let id = booked["id"].as_str().unwrap().to_string();
    // Olga works 10:00-12:00 local (09:00Z-11:00Z): 08:00Z is outside her hours.
    let path = format!("/appointments/{id}/reschedule");
    let (status, _) = shop
        .post(
            &shop.app,
            &path,
            json!({ "start_at": "2027-03-22T08:00:00Z", "staff_id": olga_staff }),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (status, moved) = shop
        .post(
            &shop.app,
            &path,
            json!({ "start_at": "2027-03-22T09:00:00Z", "staff_id": olga_staff }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{moved}");
    assert_eq!(moved["staff_id"], olga_staff.as_str());
    let (_, history) = shop
        .get(&shop.app, &format!("/appointments/{id}/history"))
        .await;
    assert_eq!(history[1]["data"]["from"]["staff_id"], shop.staff.as_str());
    assert_eq!(history[1]["data"]["to"]["staff_id"], olga_staff.as_str());
    // Anna's calendar is free again.
    assert_eq!(
        shop.slots("2027-03-22")
            .await
            .iter()
            .filter(|s| s.as_str() == "2027-03-22T08:00:00Z")
            .count(),
        1
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn roles_and_tenants_are_respected(pool: PgPool) {
    let shop = shop(&pool).await;
    let (olga, olga_id) = sign_up(&shop.app, "olga@example.pl").await;
    let olga_staff = add_member(&pool, &shop.biz, &olga_id, "employee").await;
    let (rita, rita_id) = sign_up(&shop.app, "rita@example.pl").await;
    add_member(&pool, &shop.biz, &rita_id, "reception").await;
    call(
        &shop.app,
        Method::PUT,
        &format!("{}/services/{}/staff", shop.base, shop.service),
        Some(&shop.token),
        Some(json!({ "staff_ids": [shop.staff, olga_staff] })),
    )
    .await;
    call(
        &shop.app,
        Method::PUT,
        &format!("{}/staff/{olga_staff}/schedule/weekly", shop.base),
        Some(&shop.token),
        Some(json!({ "intervals": [{ "weekday": 0, "start": "09:00", "end": "13:00" }] })),
    )
    .await;

    // Reception books, moves and cancels for any master.
    let (status, anna_booking) = shop
        .book_as(
            &shop.app,
            &rita,
            "rita-key-0001",
            &shop.staff,
            "2027-03-22T08:00:00Z",
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{anna_booking}");
    let anna_id = anna_booking["id"].as_str().unwrap().to_string();

    // An employee books only their own calendar.
    let (status, own) = shop
        .book_as(
            &shop.app,
            &olga,
            "olga-key-0001",
            &olga_staff,
            "2027-03-22T08:00:00Z",
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{own}");
    let own_id = own["id"].as_str().unwrap().to_string();
    let (status, _) = shop
        .book_as(
            &shop.app,
            &olga,
            "olga-key-0002",
            &shop.staff,
            "2027-03-22T09:30:00Z",
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // ... and sees, moves and cancels only their own appointments.
    for (method, path, body) in [
        (Method::GET, format!("/appointments/{anna_id}"), None),
        (
            Method::GET,
            format!("/appointments/{anna_id}/history"),
            None,
        ),
        (
            Method::POST,
            format!("/appointments/{anna_id}/cancel"),
            Some(json!({})),
        ),
        (
            Method::POST,
            format!("/appointments/{anna_id}/reschedule"),
            Some(json!({ "start_at": "2027-03-23T08:00:00Z" })),
        ),
    ] {
        let (status, _) = send(
            &shop.app,
            method.clone(),
            &format!("{}{path}", shop.base),
            &olga,
            None,
            body,
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{method} {path}");
    }
    // Olga may not move her own appointment onto Anna's calendar either.
    let (status, _) = send(
        &shop.app,
        Method::POST,
        &format!("{}/appointments/{own_id}/reschedule", shop.base),
        &olga,
        None,
        Some(json!({ "start_at": "2027-03-22T09:30:00Z", "staff_id": shop.staff })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // The employee's list is limited to her own calendar.
    let (status, mine) = send(
        &shop.app,
        Method::GET,
        &format!("{}/appointments?from=2027-03-22", shop.base),
        &olga,
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{mine}");
    assert_eq!(mine.as_array().unwrap().len(), 1);
    assert_eq!(mine[0]["id"], own_id.as_str());
    let (status, _) = send(
        &shop.app,
        Method::GET,
        &format!(
            "{}/appointments?from=2027-03-22&staff_id={}",
            shop.base, shop.staff
        ),
        &olga,
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (_, everyone) = send(
        &shop.app,
        Method::GET,
        &format!("{}/appointments?from=2027-03-22", shop.base),
        &rita,
        None,
        None,
    )
    .await;
    assert_eq!(everyone.as_array().unwrap().len(), 2);

    // Reception moves and cancels Anna's appointment.
    let (status, _) = send(
        &shop.app,
        Method::POST,
        &format!("{}/appointments/{anna_id}/reschedule", shop.base),
        &rita,
        None,
        Some(json!({ "start_at": "2027-03-22T09:30:00Z" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = send(
        &shop.app,
        Method::POST,
        &format!("{}/appointments/{anna_id}/cancel", shop.base),
        &rita,
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // A stranger's business: nothing of ours is reachable, with any of our ids.
    let (zed, _) = sign_up(&shop.app, "zed@example.pl").await;
    let other = create_business(&shop.app, &zed, "Zed Studio").await;
    for (method, path, body) in [
        (
            Method::GET,
            format!("/v1/businesses/{}/appointments?from=2027-03-22", shop.biz),
            None,
        ),
        (
            Method::GET,
            format!("/v1/businesses/{other}/appointments/{anna_id}"),
            None,
        ),
        (
            Method::GET,
            format!("/v1/businesses/{other}/appointments/{anna_id}/history"),
            None,
        ),
        (
            Method::POST,
            format!("/v1/businesses/{other}/appointments/{anna_id}/cancel"),
            Some(json!({})),
        ),
        (
            Method::POST,
            format!("/v1/businesses/{other}/appointments/{anna_id}/reschedule"),
            Some(json!({ "start_at": "2027-03-23T08:00:00Z" })),
        ),
    ] {
        let (status, _) = send(&shop.app, method.clone(), &path, &zed, None, body).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{method} {path}");
    }
    let (_, ours) = shop
        .get(&shop.app, &format!("/appointments/{anna_id}"))
        .await;
    assert_eq!(
        ours["status"], "cancelled",
        "only our own cancel touched it"
    );
    let (status, _) = send(
        &shop.app,
        Method::GET,
        &format!("{}/appointments?from=2027-03-22", shop.base),
        "bad-token",
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "./migrations")]
async fn concurrent_bookings_and_moves_never_double_book(pool: PgPool) {
    let shop = shop(&pool).await;
    let (rita, rita_id) = sign_up(&shop.app, "rita@example.pl").await;
    add_member(&pool, &shop.biz, &rita_id, "reception").await;

    // Two direct bookings for one slot at the same moment.
    let (a, b) = tokio::join!(
        shop.book_as(
            &shop.app,
            &shop.token,
            "race-key-a001",
            &shop.staff,
            "2027-03-22T08:00:00Z"
        ),
        shop.book_as(
            &shop.app,
            &rita,
            "race-key-b001",
            &shop.staff,
            "2027-03-22T08:00:00Z"
        ),
    );
    let mut statuses = [a.0, b.0];
    statuses.sort();
    assert_eq!(
        statuses,
        [StatusCode::CREATED, StatusCode::CONFLICT],
        "{} / {}",
        a.1,
        b.1
    );

    // A hold and a direct booking for one slot.
    let (hold, booking) = tokio::join!(
        shop.hold("race-hold-0001", "2027-03-22T11:00:00Z"),
        shop.book_as(
            &shop.app,
            &rita,
            "race-key-c001",
            &shop.staff,
            "2027-03-22T11:00:00Z"
        ),
    );
    let mut statuses = [hold.0, booking.0];
    statuses.sort();
    assert_eq!(
        statuses,
        [StatusCode::CREATED, StatusCode::CONFLICT],
        "{} / {}",
        hold.1,
        booking.1
    );

    // Two appointments both moved onto the same free slot.
    let (_, x) = shop.book("move-key-x0001", "2027-03-23T08:00:00Z").await;
    let (_, y) = shop.book("move-key-y0001", "2027-03-23T09:30:00Z").await;
    let target = json!({ "start_at": "2027-03-23T11:00:00Z" });
    let path_x = format!("/appointments/{}/reschedule", x["id"].as_str().unwrap());
    let path_y = format!("/appointments/{}/reschedule", y["id"].as_str().unwrap());
    let (mx, my) = tokio::join!(
        shop.post(&shop.app, &path_x, target.clone()),
        shop.post(&shop.app, &path_y, target.clone()),
    );
    let mut statuses = [mx.0, my.0];
    statuses.sort();
    assert_eq!(
        statuses,
        [StatusCode::OK, StatusCode::CONFLICT],
        "{} / {}",
        mx.1,
        my.1
    );

    // Whatever happened, no two blocking rows overlap on the calendar.
    let overlapping: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM appointment p JOIN appointment q
           ON p.staff_id = q.staff_id AND p.id < q.id
          AND tstzrange(p.start_at, p.blocked_end) && tstzrange(q.start_at, q.blocked_end)
        WHERE p.status IN ('held', 'confirmed') AND q.status IN ('held', 'confirmed')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(overlapping, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn a_started_visit_is_marked_done_or_no_show(pool: PgPool) {
    let shop = shop(&pool).await;
    let (_, first) = shop.book("close-key-0001", "2027-03-22T08:00:00Z").await;
    let first_id = first["id"].as_str().unwrap().to_string();
    let (_, second) = shop.book("close-key-0002", "2027-03-22T10:00:00Z").await;
    let second_id = second["id"].as_str().unwrap().to_string();

    // Before the start (the clock is still Saturday 09:00): refused, still confirmed.
    for action in ["complete", "no-show"] {
        let (status, body) = shop
            .post(
                &shop.app,
                &format!("/appointments/{first_id}/{action}"),
                json!({}),
            )
            .await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
    }
    let (_, body) = shop
        .get(&shop.app, &format!("/appointments/{first_id}"))
        .await;
    assert_eq!(body["status"], "confirmed");

    // Monday 11:00Z: the first visit (08:00Z) has started, the second (10:00Z) too.
    let later = app_at(&pool, datetime!(2027-03-22 11:00 UTC)).await;
    let (status, done) = shop
        .post(
            &later,
            &format!("/appointments/{first_id}/complete"),
            json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{done}");
    assert_eq!(done["status"], "completed");
    // Marking again is harmless.
    let (status, again) = shop
        .post(
            &later,
            &format!("/appointments/{first_id}/complete"),
            json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(again["status"], "completed");
    // A finished visit cannot become a no-show, be cancelled or moved.
    for path in [
        format!("/appointments/{first_id}/no-show"),
        format!("/appointments/{first_id}/cancel"),
    ] {
        let (status, _) = shop.post(&later, &path, json!({})).await;
        assert_eq!(status, StatusCode::CONFLICT, "{path}");
    }

    let (status, missed) = shop
        .post(
            &later,
            &format!("/appointments/{second_id}/no-show"),
            json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{missed}");
    assert_eq!(missed["status"], "no_show");
    let (status, _) = shop
        .post(
            &later,
            &format!("/appointments/{second_id}/complete"),
            json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(shop.events(&first_id).await, ["booked", "completed"]);
    assert_eq!(shop.events(&second_id).await, ["booked", "no_show"]);

    // The list takes several statuses at once; the default still hides closed visits.
    let (_, only) = shop.get(&later, "/appointments?from=2027-03-22").await;
    assert_eq!(only.as_array().unwrap().len(), 0);
    let (status, all) = shop
        .get(
            &later,
            "/appointments?from=2027-03-22&status=confirmed,completed,no_show",
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{all}");
    let statuses: Vec<&str> = all
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["status"].as_str().unwrap())
        .collect();
    assert_eq!(statuses, ["completed", "no_show"]);
    let (status, _) = shop
        .get(
            &later,
            "/appointments?from=2027-03-22&status=confirmed,bogus",
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Unknown appointment.
    let (status, _) = shop
        .post(
            &later,
            "/appointments/00000000-0000-4000-8000-000000000001/complete",
            json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "./migrations")]
async fn closing_a_visit_follows_calendar_roles(pool: PgPool) {
    let shop = shop(&pool).await;
    let (olga, olga_id) = sign_up(&shop.app, "olga@example.pl").await;
    add_member(&pool, &shop.biz, &olga_id, "employee").await;
    let (stranger, _) = sign_up(&shop.app, "stranger@example.pl").await;
    let (_, booking) = shop.book("close-key-0003", "2027-03-22T08:00:00Z").await;
    let id = booking["id"].as_str().unwrap().to_string();
    let later = app_at(&pool, datetime!(2027-03-22 11:00 UTC)).await;
    let path = format!("{}/appointments/{id}/complete", shop.base);

    // An employee cannot close someone else's visit; another business sees a 404.
    let (status, _) = send(&later, Method::POST, &path, &olga, None, None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = send(&later, Method::POST, &path, &stranger, None, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // The owner can.
    let (status, body) = send(&later, Method::POST, &path, &shop.token, None, None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

// ---------------------------------------------------------------------------
// Deleting catalog items and the booking snapshot
// ---------------------------------------------------------------------------

impl Shop {
    async fn delete(&self, app: &axum::Router, path: &str) -> (StatusCode, Value) {
        send(
            app,
            Method::DELETE,
            &format!("{}{path}", self.base),
            &self.token,
            None,
            None,
        )
        .await
    }

    /// Adds a second offered variant so the first one is no longer the last.
    async fn add_variant(&self) -> String {
        let (status, body) = self
            .post(
                &self.app,
                &format!("/services/{}/variants", self.service),
                json!({ "name": { "pl": "Volume" }, "duration_min": 90, "price_minor": 35000 }),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        body["id"].as_str().unwrap().to_string()
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn a_service_or_variant_with_an_upcoming_visit_cannot_be_deleted(pool: PgPool) {
    let shop = shop(&pool).await;
    let (status, booked) = shop.book("del-key-0001", "2027-03-22T08:00:00Z").await;
    assert_eq!(status, StatusCode::CREATED, "{booked}");
    let second = shop.add_variant().await;

    let (status, body) = shop
        .delete(&shop.app, &format!("/services/{}", shop.service))
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    let (status, body) = shop
        .delete(&shop.app, &format!("/variants/{}", shop.variant))
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");

    // Nothing was touched: the service is still listed and active.
    let (_, service) = shop
        .get(&shop.app, &format!("/services/{}", shop.service))
        .await;
    assert_eq!(service["is_active"], true);
    assert_eq!(service["variants"].as_array().unwrap().len(), 2);

    // A variant nobody booked can go, while another one stays offered.
    let (status, _) = shop.delete(&shop.app, &format!("/variants/{second}")).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Hiding is always possible and keeps the visit.
    let (status, hidden) = send(
        &shop.app,
        Method::PATCH,
        &format!("{}/services/{}", shop.base, shop.service),
        &shop.token,
        None,
        Some(json!({ "is_active": false })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{hidden}");
    let id = booked["id"].as_str().unwrap();
    let (_, visit) = shop.get(&shop.app, &format!("/appointments/{id}")).await;
    assert_eq!(visit["status"], "confirmed");
}

#[sqlx::test(migrations = "./migrations")]
async fn a_running_hold_blocks_deleting_but_an_expired_one_does_not(pool: PgPool) {
    let shop = shop(&pool).await;
    let (status, held) = shop.hold("del-hold-0001", "2027-03-22T08:00:00Z").await;
    assert_eq!(status, StatusCode::CREATED, "{held}");

    let (status, _) = shop
        .delete(&shop.app, &format!("/services/{}", shop.service))
        .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // A day later the hold is long over.
    let later = app_at(&pool, datetime!(2027-03-21 09:00 UTC)).await;
    let (status, body) = shop
        .delete(&later, &format!("/services/{}", shop.service))
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT, "{body}");
}

#[sqlx::test(migrations = "./migrations")]
async fn finished_and_cancelled_visits_do_not_block_and_keep_their_snapshot(pool: PgPool) {
    let shop = shop(&pool).await;
    let (_, cancelled) = shop.book("snap-key-0001", "2027-03-22T08:00:00Z").await;
    let cancelled_id = cancelled["id"].as_str().unwrap().to_string();
    let (_, done) = shop.book("snap-key-0002", "2027-03-22T10:00:00Z").await;
    let done_id = done["id"].as_str().unwrap().to_string();
    assert_eq!(done["service_name"]["pl"], "Classic");
    assert!(done["variant_name"].is_null());

    let (status, body) = shop
        .post(
            &shop.app,
            &format!("/appointments/{cancelled_id}/cancel"),
            json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    // The catalog moves on: new names and price. The visit keeps what was booked.
    let (status, renamed) = send(
        &shop.app,
        Method::PATCH,
        &format!("{}/services/{}", shop.base, shop.service),
        &shop.token,
        None,
        Some(json!({ "name": { "pl": "Renamed" } })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{renamed}");
    let (status, repriced) = send(
        &shop.app,
        Method::PATCH,
        &format!("{}/variants/{}", shop.base, shop.variant),
        &shop.token,
        None,
        Some(json!({ "name": { "pl": "Pro" }, "price_minor": 99000, "duration_min": 120 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{repriced}");
    let (_, visit) = shop
        .get(&shop.app, &format!("/appointments/{done_id}"))
        .await;
    assert_eq!(visit["service_name"]["pl"], "Classic");
    assert!(visit["variant_name"].is_null());
    assert_eq!(visit["price_minor"], 25000);
    assert_eq!(visit["duration_min"], 60);

    // Monday 13:00Z: the second visit has ended and is marked done.
    let later = app_at(&pool, datetime!(2027-03-22 13:00 UTC)).await;
    let (status, body) = shop
        .post(
            &later,
            &format!("/appointments/{done_id}/complete"),
            json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    // With another variant offered, the used one and then the service can go.
    let extra = shop.add_variant().await;
    let (status, body) = shop
        .delete(&later, &format!("/variants/{}", shop.variant))
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT, "{body}");
    let (status, body) = shop
        .delete(&later, &format!("/services/{}", shop.service))
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT, "{body}");
    let _ = extra;

    // Gone from the catalog, yet both visits still say what was booked and for how much.
    let (_, services) = shop.get(&later, "/services").await;
    assert!(services.as_array().unwrap().is_empty());
    for id in [&cancelled_id, &done_id] {
        let (status, visit) = shop.get(&later, &format!("/appointments/{id}")).await;
        assert_eq!(status, StatusCode::OK, "{visit}");
        assert_eq!(visit["service_name"]["pl"], "Classic", "{visit}");
        assert!(visit["variant_name"].is_null());
        assert_eq!(visit["price_minor"], 25000);
        assert_eq!(visit["duration_min"], 60);
        assert_eq!(visit["currency"], "PLN");
    }
    let (_, list) = shop
        .get(
            &later,
            "/appointments?from=2027-03-22&status=confirmed,completed,cancelled",
        )
        .await;
    assert_eq!(list.as_array().unwrap().len(), 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn the_last_offered_variant_of_an_offered_service_cannot_be_deleted(pool: PgPool) {
    let shop = shop(&pool).await;
    let second = shop.add_variant().await;

    let (status, _) = shop
        .delete(&shop.app, &format!("/variants/{}", shop.variant))
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Only the second one is left: refused while the service is offered.
    let (status, body) = shop.delete(&shop.app, &format!("/variants/{second}")).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");

    // A hidden service may lose all its variants.
    let (status, _) = send(
        &shop.app,
        Method::PATCH,
        &format!("{}/services/{}", shop.base, shop.service),
        &shop.token,
        None,
        Some(json!({ "is_active": false })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = shop.delete(&shop.app, &format!("/variants/{second}")).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "{body}");
}
