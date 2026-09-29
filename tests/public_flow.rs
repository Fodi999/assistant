//! End-to-end tests of the customer side: moderation, the public catalog,
//! guest sessions, customer bookings and isolation between customers and
//! businesses.
//!
//! The clock is 2027-03-20 (Saturday) 09:00 UTC. Every studio has a master who
//! works Monday 2027-03-22, 09:00-13:00 Warsaw time (08:00Z-12:00Z). "Classic"
//! takes 60 minutes plus a 10 minute buffer on a 30 minute grid, so the day
//! offers starts at 08:00Z, 08:30Z ... 11:00Z.

mod common;

use axum::http::{Method, StatusCode};
use common::{app_at, call, create_business, my_staff_id, sign_up};
use serde_json::{json, Value};
use sqlx::PgPool;
use time::macros::datetime;
use time::OffsetDateTime;

const NOW: OffsetDateTime = datetime!(2027-03-20 09:00 UTC);
const MON: &str = "2027-03-22";

async fn send(
    app: &axum::Router,
    method: Method,
    uri: &str,
    token: Option<&str>,
    key: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    use axum::body::Body;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let mut request = axum::http::Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
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

struct Studio {
    owner: String,
    owner_id: String,
    biz: String,
    base: String,
    staff: String,
    service: String,
    variant: String,
}

/// A master with one service, a Monday schedule and a published profile. Not
/// yet approved.
async fn studio(app: &axum::Router, email: &str, name: &str) -> Studio {
    let (owner, owner_id) = sign_up(app, email).await;
    let biz = create_business(app, &owner, name).await;
    let staff = my_staff_id(app, &owner, &biz).await;
    let base = format!("/v1/businesses/{biz}");
    let (status, service) = call(
        app,
        Method::POST,
        &format!("{base}/services"),
        Some(&owner),
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
            json!({ "intervals": [{ "weekday": 0, "start": "09:00", "end": "13:00" }] }),
        ),
        (
            format!("{base}/profile"),
            json!({ "city": "Warszawa", "headline": "Lash artist", "is_published": true }),
        ),
    ] {
        let (status, response) = call(app, Method::PUT, &uri, Some(&owner), Some(body)).await;
        assert_eq!(status, StatusCode::OK, "{response}");
    }
    Studio {
        owner,
        owner_id,
        biz,
        base,
        staff,
        service: service_id,
        variant,
    }
}

/// An account with the platform admin right (granted the way an operator does).
async fn admin(app: &axum::Router, pool: &PgPool) -> String {
    let (token, id) = sign_up(app, "admin@example.pl").await;
    sqlx::query("INSERT INTO platform_admin (user_id) VALUES ($1::text::uuid)")
        .bind(&id)
        .execute(pool)
        .await
        .unwrap();
    token
}

async fn approve(app: &axum::Router, admin: &str, biz: &str) {
    let (status, body) = call(
        app,
        Method::POST,
        &format!("/v1/admin/businesses/{biz}/approve"),
        Some(admin),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["moderation_status"], "approved");
}

async fn guest(app: &axum::Router) -> String {
    let (status, body) = call(
        app,
        Method::POST,
        "/v1/public/guest",
        None,
        Some(json!({ "accepted_terms": true, "display_name": "Guest" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    body["tokens"]["access_token"].as_str().unwrap().to_string()
}

impl Studio {
    fn public(&self) -> String {
        format!("/v1/public/businesses/{}", self.biz)
    }

    fn hold_body(&self, start: &str) -> Value {
        json!({
            "service_id": self.service, "variant_id": self.variant,
            "staff_id": self.staff, "start_at": start
        })
    }

    fn book_body(&self, start: &str, name: &str) -> Value {
        json!({
            "service_id": self.service, "variant_id": self.variant,
            "staff_id": self.staff, "start_at": start,
            "client_name": name, "client_phone": "+48 600 100 200"
        })
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn only_approved_and_published_businesses_are_public(pool: PgPool) {
    let app = app_at(&pool, NOW).await;
    let admin = admin(&app, &pool).await;
    let s = studio(&app, "anna@example.pl", "Anna Lashes").await;
    let catalog = "/v1/public/businesses";

    // Pending: invisible to everybody.
    let (status, list) = call(&app, Method::GET, catalog, None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list, json!([]));
    let (status, _) = call(&app, Method::GET, &s.public(), None, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = call(
        &app,
        Method::GET,
        &format!(
            "{}/availability?service_id={}&variant_id={}&from={MON}",
            s.public(),
            s.service,
            s.variant
        ),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let visitor = guest(&app).await;
    let (status, _) = send(
        &app,
        Method::POST,
        &format!("{}/holds", s.public()),
        Some(&visitor),
        Some("hold-key-0001"),
        Some(s.hold_body("2027-03-22T08:00:00Z")),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "a pending business cannot be booked"
    );

    // The owner sees the state, and cannot approve themselves.
    let (status, profile) = call(
        &app,
        Method::PUT,
        &format!("{}/profile", s.base),
        Some(&s.owner),
        Some(json!({ "moderation_status": "approved", "headline": "Lash artist" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{profile}");
    assert_eq!(profile["moderation_status"], "pending");
    assert_eq!(profile["publicly_visible"], false);
    let (status, _) = call(
        &app,
        Method::POST,
        &format!("/v1/admin/businesses/{}/approve", s.biz),
        Some(&s.owner),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = call(
        &app,
        Method::GET,
        "/v1/admin/businesses",
        Some(&s.owner),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = call(&app, Method::GET, "/v1/admin/businesses", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // The moderation queue shows the pending master and their owner.
    let (status, queue) = call(
        &app,
        Method::GET,
        "/v1/admin/businesses?status=pending",
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{queue}");
    let entry = queue
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["id"] == s.biz.as_str())
        .expect("pending business in the queue");
    assert_eq!(entry["owner_email"], "anna@example.pl");
    let (status, _) = call(
        &app,
        Method::GET,
        "/v1/admin/businesses?status=nonsense",
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Rejecting needs a reason.
    let (status, _) = call(
        &app,
        Method::POST,
        &format!("/v1/admin/businesses/{}/reject", s.biz),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    approve(&app, &admin, &s.biz).await;
    // Approving again changes nothing.
    approve(&app, &admin, &s.biz).await;

    // Now it is in the catalog, findable by city and text.
    let (_, list) = call(&app, Method::GET, catalog, None, None).await;
    assert_eq!(list.as_array().unwrap().len(), 1, "{list}");
    assert_eq!(list[0]["id"], s.biz.as_str());
    assert_eq!(list[0]["city"], "Warszawa");
    assert!(list[0].get("moderation_status").is_none());
    for (query, expected) in [
        ("?city=warszawa", 1),
        ("?city=Krakow", 0),
        ("?q=anna", 1),
        ("?q=zzz", 0),
        ("?city=Warszawa&q=lash", 1),
    ] {
        let (status, list) =
            call(&app, Method::GET, &format!("{catalog}{query}"), None, None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(list.as_array().unwrap().len(), expected, "{query}");
    }

    // The profile (by id and by slug) shows prices and masters, nothing private.
    let (_, mine) = call(&app, Method::GET, &s.base, Some(&s.owner), None).await;
    let slug = mine["slug"].as_str().unwrap().to_string();
    for key in [s.biz.clone(), slug] {
        let (status, profile) =
            call(&app, Method::GET, &format!("{catalog}/{key}"), None, None).await;
        assert_eq!(status, StatusCode::OK, "{profile}");
        assert_eq!(profile["name"], "Anna Lashes");
        assert_eq!(profile["headline"], "Lash artist");
        assert_eq!(profile["timezone"], "Europe/Warsaw");
        assert_eq!(profile["staff"].as_array().unwrap().len(), 1);
        let variant = &profile["services"][0]["variants"][0];
        assert_eq!(variant["price_minor"], 25000);
        assert_eq!(variant["duration_min"], 60);
        assert_eq!(variant["currency"], "PLN");
        assert_eq!(profile["services"][0]["staff_ids"][0], s.staff.as_str());
        assert_eq!(profile["portfolio"], json!([]));
        let text = profile.to_string();
        for private in [
            "buffer_after_min",
            "intake_questions",
            "min_notice",
            "anna@example.pl",
        ] {
            assert!(!text.contains(private), "public profile leaks {private}");
        }
    }

    // Suspending hides it again (needs a note; only an approved business).
    let suspend = format!("/v1/admin/businesses/{}/suspend", s.biz);
    let (status, _) = call(&app, Method::POST, &suspend, Some(&admin), None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, body) = call(
        &app,
        Method::POST,
        &suspend,
        Some(&admin),
        Some(json!({ "note": "Policy check" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["moderation_status"], "suspended");
    let (_, list) = call(&app, Method::GET, catalog, None, None).await;
    assert_eq!(list, json!([]));
    let (status, _) = call(&app, Method::GET, &s.public(), None, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (_, profile) = call(
        &app,
        Method::GET,
        &format!("{}/profile", s.base),
        Some(&s.owner),
        None,
    )
    .await;
    assert_eq!(profile["moderation_status"], "suspended");
    assert_eq!(profile["moderation_note"], "Policy check");

    // Reinstated, then hidden by the owner's own switch.
    approve(&app, &admin, &s.biz).await;
    let (_, list) = call(&app, Method::GET, catalog, None, None).await;
    assert_eq!(list.as_array().unwrap().len(), 1);
    let (status, _) = call(
        &app,
        Method::PUT,
        &format!("{}/profile", s.base),
        Some(&s.owner),
        Some(json!({ "is_published": false })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, list) = call(&app, Method::GET, catalog, None, None).await;
    assert_eq!(list, json!([]));

    // Even with a raw connection as the application role, the owner of a business
    // cannot set the moderation status: the database refuses (42501).
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SET LOCAL ROLE beauty_app")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query(
        "SELECT set_config('app.user_id', $1, true), set_config('app.business_id', $2, true)",
    )
    .bind(&s.owner_id)
    .bind(&s.biz)
    .execute(&mut *tx)
    .await
    .unwrap();
    let refused =
        sqlx::query("UPDATE business SET moderation_status = 'approved' WHERE id = $1::text::uuid")
            .bind(&s.biz)
            .execute(&mut *tx)
            .await
            .unwrap_err();
    let code = refused.as_database_error().and_then(|error| error.code());
    assert_eq!(code.as_deref(), Some("42501"), "{refused}");
}

#[sqlx::test(migrations = "./migrations")]
async fn a_guest_books_and_manages_only_their_own_appointments(pool: PgPool) {
    let app = app_at(&pool, NOW).await;
    let admin = admin(&app, &pool).await;
    let s = studio(&app, "anna@example.pl", "Anna Lashes").await;
    approve(&app, &admin, &s.biz).await;

    // Anyone can look at the free slots.
    let (status, slots) = call(
        &app,
        Method::GET,
        &format!(
            "{}/availability?service_id={}&variant_id={}&from={MON}",
            s.public(),
            s.service,
            s.variant
        ),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{slots}");
    assert_eq!(slots["slots"].as_array().unwrap().len(), 7, "{slots}");
    assert_eq!(slots["slots"][0]["start_at"], "2027-03-22T08:00:00Z");
    // A customer cannot ask for the staff-only channel.
    let (status, manual) = call(
        &app,
        Method::GET,
        &format!(
            "{}/availability?service_id={}&variant_id={}&from={MON}&channel=manual",
            s.public(),
            s.service,
            s.variant
        ),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(manual["slots"], slots["slots"]);

    let ewa = guest(&app).await;
    let zoe = guest(&app).await;
    let appointments = format!("{}/appointments", s.public());

    // Ewa holds a slot and confirms it.
    let (status, hold) = send(
        &app,
        Method::POST,
        &format!("{}/holds", s.public()),
        Some(&ewa),
        Some("ewa-hold-0001"),
        Some(s.hold_body("2027-03-22T08:00:00Z")),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{hold}");
    assert_eq!(hold["status"], "held");
    assert_eq!(hold["source"], "app");
    let hold_id = hold["id"].as_str().unwrap().to_string();
    let (status, confirmed) = send(
        &app,
        Method::POST,
        &appointments,
        Some(&ewa),
        None,
        Some(json!({
            "hold_id": hold_id, "client_name": "Ewa Nowak",
            "client_phone": "+48 600-100-200", "client_email": "Ewa@Example.PL"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{confirmed}");
    assert_eq!(confirmed["status"], "confirmed");
    assert_eq!(confirmed["client_phone"], "+48600100200");

    // Ewa books a second time directly; a retry with the same key is a replay.
    let (status, second) = send(
        &app,
        Method::POST,
        &appointments,
        Some(&ewa),
        Some("ewa-book-0002"),
        Some(s.book_body("2027-03-22T09:30:00Z", "Ewa Nowak")),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{second}");
    let second_id = second["id"].as_str().unwrap().to_string();
    let (status, replay) = send(
        &app,
        Method::POST,
        &appointments,
        Some(&ewa),
        Some("ewa-book-0002"),
        Some(s.book_body("2027-03-22T09:30:00Z", "Ewa Nowak")),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{replay}");
    assert_eq!(replay["id"], second_id.as_str());
    // A direct booking needs a key; customers cannot book as staff.
    let (status, _) = send(
        &app,
        Method::POST,
        &appointments,
        Some(&ewa),
        None,
        Some(s.book_body("2027-03-22T11:00:00Z", "Ewa Nowak")),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let mut manual = s.book_body("2027-03-22T11:00:00Z", "Ewa Nowak");
    manual["source"] = json!("manual");
    let (status, _) = send(
        &app,
        Method::POST,
        &appointments,
        Some(&ewa),
        Some("ewa-manual-01"),
        Some(manual),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // The master sees both in the calendar, and one client with two visits.
    let (status, calendar) = call(
        &app,
        Method::GET,
        &format!("{}/appointments?from={MON}", s.base),
        Some(&s.owner),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{calendar}");
    assert_eq!(calendar.as_array().unwrap().len(), 2);
    assert_eq!(calendar[0]["client_name"], "Ewa Nowak");
    let (status, clients) = call(
        &app,
        Method::GET,
        &format!("{}/clients", s.base),
        Some(&s.owner),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{clients}");
    assert_eq!(clients.as_array().unwrap().len(), 1);
    assert_eq!(clients[0]["full_name"], "Ewa Nowak");
    assert_eq!(clients[0]["email"], "ewa@example.pl");
    assert_eq!(clients[0]["source"], "guest");
    assert_eq!(clients[0]["has_account"], false);
    assert_eq!(clients[0]["confirmed_appointments"], 2);

    // Ewa sees exactly her own; Zoe sees nothing and can touch nothing.
    let (_, mine) = send(&app, Method::GET, &appointments, Some(&ewa), None, None).await;
    assert_eq!(mine.as_array().unwrap().len(), 2, "{mine}");
    let (status, none) = send(&app, Method::GET, &appointments, Some(&zoe), None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(none, json!([]));
    let (status, _) = send(
        &app,
        Method::GET,
        &format!("{appointments}/{second_id}"),
        Some(&zoe),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = send(
        &app,
        Method::POST,
        &format!("{appointments}/{second_id}/cancel"),
        Some(&zoe),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = send(
        &app,
        Method::POST,
        &format!("{appointments}/{second_id}/reschedule"),
        Some(&zoe),
        None,
        Some(json!({ "start_at": "2027-03-22T11:00:00Z" })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    // Nor can she confirm or release a hold that is Ewa's.
    let (status, hold3) = send(
        &app,
        Method::POST,
        &format!("{}/holds", s.public()),
        Some(&ewa),
        Some("ewa-hold-0003"),
        Some(s.hold_body("2027-03-22T11:00:00Z")),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{hold3}");
    let hold3_id = hold3["id"].as_str().unwrap().to_string();
    let (status, _) = send(
        &app,
        Method::POST,
        &appointments,
        Some(&zoe),
        None,
        Some(json!({ "hold_id": hold3_id, "client_name": "Zoe" })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = send(
        &app,
        Method::DELETE,
        &format!("{}/holds/{hold3_id}", s.public()),
        Some(&zoe),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = send(
        &app,
        Method::DELETE,
        &format!("{}/holds/{hold3_id}", s.public()),
        Some(&ewa),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // A customer is not a member: the staff API is closed to her.
    for uri in [
        format!("{}/appointments?from={MON}", s.base),
        format!("{}/clients", s.base),
        format!("{}/profile", s.base),
    ] {
        let (status, _) = call(&app, Method::GET, &uri, Some(&ewa), None).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
    }

    // Ewa moves her second visit, then cancels it; the slot is free again.
    let (status, moved) = send(
        &app,
        Method::POST,
        &format!("{appointments}/{second_id}/reschedule"),
        Some(&ewa),
        None,
        Some(json!({ "start_at": "2027-03-22T11:00:00Z", "reason": "work" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{moved}");
    assert_eq!(moved["start_at"], "2027-03-22T11:00:00Z");
    assert_eq!(moved["id"], second_id.as_str());
    let (status, taken) = send(
        &app,
        Method::POST,
        &format!("{appointments}/{second_id}/reschedule"),
        Some(&ewa),
        None,
        Some(json!({ "start_at": "2027-03-22T08:00:00Z" })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{taken}");
    assert_eq!(taken["code"], "SLOT_UNAVAILABLE");
    let (status, cancelled) = send(
        &app,
        Method::POST,
        &format!("{appointments}/{second_id}/cancel"),
        Some(&ewa),
        None,
        Some(json!({ "reason": "ill" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{cancelled}");
    assert_eq!(cancelled["status"], "cancelled");
    assert_eq!(cancelled["late_cancellation"], false);
    let (_, slots) = call(
        &app,
        Method::GET,
        &format!(
            "{}/availability?service_id={}&variant_id={}&from={MON}",
            s.public(),
            s.service,
            s.variant
        ),
        None,
        None,
    )
    .await;
    let starts: Vec<&str> = slots["slots"]
        .as_array()
        .unwrap()
        .iter()
        .map(|slot| slot["start_at"].as_str().unwrap())
        .collect();
    assert!(starts.contains(&"2027-03-22T11:00:00Z"), "{starts:?}");

    // The master sees the history of that visit, with who did what.
    let (status, history) = call(
        &app,
        Method::GET,
        &format!("{}/appointments/{second_id}/history", s.base),
        Some(&s.owner),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{history}");
    let kinds: Vec<&str> = history
        .as_array()
        .unwrap()
        .iter()
        .map(|event| event["type"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["booked", "rescheduled", "cancelled"]);
}

#[sqlx::test(migrations = "./migrations")]
async fn clients_and_appointments_stay_inside_their_business(pool: PgPool) {
    let app = app_at(&pool, NOW).await;
    let admin = admin(&app, &pool).await;
    let a = studio(&app, "anna@example.pl", "Anna Lashes").await;
    let b = studio(&app, "bea@example.pl", "Bea Brows").await;
    let pending = studio(&app, "cleo@example.pl", "Cleo Nails").await;
    approve(&app, &admin, &a.biz).await;
    approve(&app, &admin, &b.biz).await;

    let ewa = guest(&app).await;
    let (status, booking) = send(
        &app,
        Method::POST,
        &format!("{}/appointments", a.public()),
        Some(&ewa),
        Some("ewa-a-0001"),
        Some(a.book_body("2027-03-22T08:00:00Z", "Ewa Nowak")),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{booking}");
    let booking_id = booking["id"].as_str().unwrap().to_string();

    // Business A knows the client, business B does not.
    let (_, clients_a) = call(
        &app,
        Method::GET,
        &format!("{}/clients", a.base),
        Some(&a.owner),
        None,
    )
    .await;
    assert_eq!(clients_a.as_array().unwrap().len(), 1);
    let client_id = clients_a[0]["id"].as_str().unwrap().to_string();
    let (_, clients_b) = call(
        &app,
        Method::GET,
        &format!("{}/clients", b.base),
        Some(&b.owner),
        None,
    )
    .await;
    assert_eq!(clients_b, json!([]));
    let (status, _) = call(
        &app,
        Method::GET,
        &format!("{}/clients/{client_id}", b.base),
        Some(&b.owner),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = call(
        &app,
        Method::GET,
        &format!("{}/clients/{client_id}", a.base),
        Some(&a.owner),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    // B's owner cannot read A's calendar, clients or the appointment.
    for uri in [
        format!("{}/clients", a.base),
        format!("{}/appointments?from={MON}", a.base),
        format!("{}/appointments/{booking_id}", a.base),
    ] {
        let (status, _) = call(&app, Method::GET, &uri, Some(&b.owner), None).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
    }
    let (status, _) = call(
        &app,
        Method::GET,
        &format!("{}/clients", a.base),
        Some(&b.owner),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // A's appointment id used in B's public URL is nothing.
    let (status, _) = send(
        &app,
        Method::GET,
        &format!("{}/appointments/{booking_id}", b.public()),
        Some(&ewa),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = send(
        &app,
        Method::POST,
        &format!("{}/appointments/{booking_id}/cancel", b.public()),
        Some(&ewa),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    // A's service in B's URL cannot be booked.
    let (status, _) = send(
        &app,
        Method::POST,
        &format!("{}/appointments", b.public()),
        Some(&ewa),
        Some("ewa-b-evil1"),
        Some(a.book_body("2027-03-22T08:00:00Z", "Ewa Nowak")),
    )
    .await;
    assert!(
        status == StatusCode::NOT_FOUND || status == StatusCode::BAD_REQUEST,
        "{status}"
    );

    // Booking in B makes a separate client row there.
    let (status, _) = send(
        &app,
        Method::POST,
        &format!("{}/appointments", b.public()),
        Some(&ewa),
        Some("ewa-b-0001"),
        Some(b.book_body("2027-03-22T08:00:00Z", "Ewa N.")),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let (_, clients_b) = call(
        &app,
        Method::GET,
        &format!("{}/clients", b.base),
        Some(&b.owner),
        None,
    )
    .await;
    assert_eq!(clients_b.as_array().unwrap().len(), 1);
    assert_ne!(clients_b[0]["id"], client_id.as_str());
    assert_eq!(clients_b[0]["full_name"], "Ewa N.");
    // ... and A's record still has A's details.
    let (_, client_a) = call(
        &app,
        Method::GET,
        &format!("{}/clients/{client_id}", a.base),
        Some(&a.owner),
        None,
    )
    .await;
    assert_eq!(client_a["full_name"], "Ewa Nowak");

    // A studio that was never approved cannot be booked.
    let (status, _) = send(
        &app,
        Method::POST,
        &format!("{}/appointments", pending.public()),
        Some(&ewa),
        Some("ewa-c-0001"),
        Some(pending.book_body("2027-03-22T08:00:00Z", "Ewa Nowak")),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "./migrations")]
async fn two_customers_racing_for_one_slot_get_one_winner(pool: PgPool) {
    let app = app_at(&pool, NOW).await;
    let admin = admin(&app, &pool).await;
    let s = studio(&app, "anna@example.pl", "Anna Lashes").await;
    approve(&app, &admin, &s.biz).await;

    let ewa = guest(&app).await;
    let zoe = guest(&app).await;
    let uri = format!("{}/appointments", s.public());
    for (n, slot) in ["2027-03-22T08:00:00Z", "2027-03-22T09:30:00Z"]
        .into_iter()
        .enumerate()
    {
        let ewa_key = format!("ewa-race-{n:04}");
        let zoe_key = format!("zoe-race-{n:04}");
        let (first, second) = tokio::join!(
            send(
                &app,
                Method::POST,
                &uri,
                Some(&ewa),
                Some(&ewa_key),
                Some(s.book_body(slot, "Ewa Nowak"))
            ),
            send(
                &app,
                Method::POST,
                &uri,
                Some(&zoe),
                Some(&zoe_key),
                Some(s.book_body(slot, "Zoe Kowal"))
            ),
        );
        let mut codes = [first.0, second.0];
        codes.sort();
        assert_eq!(
            codes,
            [StatusCode::CREATED, StatusCode::CONFLICT],
            "slot {slot}: {} / {}",
            first.1,
            second.1
        );
        let loser = if first.0 == StatusCode::CONFLICT {
            &first.1
        } else {
            &second.1
        };
        assert_eq!(loser["code"], "SLOT_UNAVAILABLE");
    }

    // The staff bookings and the customers' share one calendar guard.
    let (status, calendar) = call(
        &app,
        Method::GET,
        &format!("{}/appointments?from={MON}", s.base),
        Some(&s.owner),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(calendar.as_array().unwrap().len(), 2);
}
