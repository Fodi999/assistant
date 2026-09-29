//! End-to-end tests of the CRM: client cards, phone matching, visit numbers,
//! what an employee may see, and the backfill of old manual visits.
//!
//! The clock is 2027-03-20 (a Saturday) 09:00 UTC. Anna works Monday
//! 2027-03-22 and Tuesday 2027-03-23, 09:00-13:00 Warsaw time (CET, UTC+1),
//! i.e. 08:00Z-12:00Z. "Classic" costs 250.00 and takes 60 minutes plus a
//! 10 minute buffer.

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

async fn shop_with(app: axum::Router, email: &str) -> Shop {
    let (token, _) = sign_up(&app, email).await;
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

async fn shop(pool: &PgPool) -> Shop {
    shop_with(app_at(pool, NOW).await, "anna@example.pl").await
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
    async fn req(
        &self,
        method: Method,
        path: &str,
        token: &str,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        send(
            &self.app,
            method,
            &format!("{}{path}", self.base),
            token,
            None,
            body,
        )
        .await
    }

    async fn get(&self, path: &str, token: &str) -> (StatusCode, Value) {
        self.req(Method::GET, path, token, None).await
    }

    /// Books a visit typed in by hand.
    async fn book(
        &self,
        token: &str,
        key: &str,
        staff: &str,
        start: &str,
        name: &str,
        phone: Option<&str>,
    ) -> (StatusCode, Value) {
        let mut body = json!({
            "service_id": self.service, "variant_id": self.variant, "staff_id": staff,
            "start_at": start, "client_name": name
        });
        if let Some(phone) = phone {
            body["client_phone"] = json!(phone);
        }
        send(
            &self.app,
            Method::POST,
            &format!("{}/appointments", self.base),
            token,
            Some(key),
            Some(body),
        )
        .await
    }

    async fn clients(&self, token: &str) -> Vec<Value> {
        let (status, body) = self.get("/clients", token).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body.as_array().unwrap().clone()
    }

    async fn client(&self, token: &str, id: &str) -> (StatusCode, Value) {
        self.get(&format!("/clients/{id}"), token).await
    }

    /// An employee with a calendar of their own (Monday 09:00-13:00 Warsaw time).
    async fn employee(&self, pool: &PgPool, email: &str) -> (String, String) {
        let (token, id) = sign_up(&self.app, email).await;
        let staff = add_member(pool, &self.biz, &id, "employee").await;
        let (status, body) = call(
            &self.app,
            Method::PUT,
            &format!("{}/services/{}/staff", self.base, self.service),
            Some(&self.token),
            Some(json!({ "staff_ids": [self.staff, staff] })),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let (status, body) = call(
            &self.app,
            Method::PUT,
            &format!("{}/staff/{staff}/schedule/weekly", self.base),
            Some(&self.token),
            Some(json!({ "intervals": [{ "weekday": 0, "start": "09:00", "end": "13:00" }] })),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        (token, staff)
    }
}

// ---------------------------------------------------------------------------
// Matching a manual visit to a card
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
async fn a_manual_visit_with_a_phone_finds_or_makes_the_card(pool: PgPool) {
    let s = shop(&pool).await;

    // The same number written two ways is one person; the card keeps its first name.
    let (status, first) = s
        .book(
            &s.token,
            "crm-key-0001",
            &s.staff,
            "2027-03-22T08:00:00Z",
            "Ewa Nowak",
            Some("+48 600-100-200"),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{first}");
    let (status, second) = s
        .book(
            &s.token,
            "crm-key-0002",
            &s.staff,
            "2027-03-22T10:00:00Z",
            "Ewa N.",
            Some("+48600100200"),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{second}");
    assert!(first["client_id"].is_string(), "{first}");
    assert_eq!(first["client_id"], second["client_id"]);
    // The visit keeps the name that was typed for it.
    assert_eq!(second["client_name"], "Ewa N.");

    let cards = s.clients(&s.token).await;
    assert_eq!(cards.len(), 1, "{cards:?}");
    assert_eq!(cards[0]["full_name"], "Ewa Nowak");
    assert_eq!(cards[0]["phone"], "+48600100200");
    assert_eq!(cards[0]["source"], "staff");
    assert_eq!(cards[0]["confirmed_appointments"], 2);

    // Without a phone no card is made and the visit stays without one.
    let (status, bare) = s
        .book(
            &s.token,
            "crm-key-0003",
            &s.staff,
            "2027-03-23T08:00:00Z",
            "Ola",
            None,
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{bare}");
    assert!(bare["client_id"].is_null(), "{bare}");
    assert_eq!(s.clients(&s.token).await.len(), 1);

    // Another number is another person.
    let (status, other) = s
        .book(
            &s.token,
            "crm-key-0004",
            &s.staff,
            "2027-03-23T10:00:00Z",
            "Kasia",
            Some("+48 601 000 000"),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{other}");
    assert_ne!(other["client_id"], first["client_id"]);
    assert_eq!(s.clients(&s.token).await.len(), 2);

    // A retry with the same key neither books nor makes another card.
    let (status, again) = s
        .book(
            &s.token,
            "crm-key-0004",
            &s.staff,
            "2027-03-23T10:00:00Z",
            "Kasia",
            Some("+48 601 000 000"),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{again}");
    assert_eq!(s.clients(&s.token).await.len(), 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn a_failed_booking_leaves_no_card_behind(pool: PgPool) {
    let s = shop(&pool).await;
    // 03:00Z is outside the working hours: refused, and nothing is written.
    let (status, body) = s
        .book(
            &s.token,
            "crm-key-0001",
            &s.staff,
            "2027-03-22T03:00:00Z",
            "Ewa",
            Some("+48600100200"),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert!(s.clients(&s.token).await.is_empty());
}

#[sqlx::test(migrations = "./migrations")]
async fn the_phone_is_unique_inside_one_business_only(pool: PgPool) {
    let a = shop(&pool).await;
    let b = shop_with(app_at(&pool, NOW).await, "beata@example.pl").await;
    let (_, in_a) = a
        .book(
            &a.token,
            "crm-key-0001",
            &a.staff,
            "2027-03-22T08:00:00Z",
            "Ewa",
            Some("+48600100200"),
        )
        .await;
    let (status, in_b) = b
        .book(
            &b.token,
            "crm-key-0001",
            &b.staff,
            "2027-03-22T08:00:00Z",
            "Ewa",
            Some("+48600100200"),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{in_b}");
    assert_ne!(in_a["client_id"], in_b["client_id"]);
    assert_eq!(a.clients(&a.token).await.len(), 1);
    assert_eq!(b.clients(&b.token).await.len(), 1);

    // A card of another business is simply not there.
    let id = in_a["client_id"].as_str().unwrap();
    let (status, _) = b.client(&b.token, id).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = b
        .req(
            Method::PATCH,
            &format!("/clients/{id}"),
            &b.token,
            Some(json!({ "note": "x" })),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = b
        .get(&format!("/clients/{id}/appointments"), &b.token)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "./migrations")]
async fn a_manual_visit_never_joins_an_app_customers_card(pool: PgPool) {
    let s = shop(&pool).await;
    // A customer's card with the same, unverified, number.
    let (_, customer_id) = sign_up(&s.app, "customer@example.pl").await;
    let customer_card: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO client (business_id, user_id, source, full_name, phone_e164)
         VALUES ($1::text::uuid, $2::text::uuid, 'account', 'Customer', '+48600100200')
         RETURNING id",
    )
    .bind(&s.biz)
    .bind(&customer_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    let (status, visit) = s
        .book(
            &s.token,
            "crm-key-0001",
            &s.staff,
            "2027-03-22T08:00:00Z",
            "Someone Else",
            Some("+48600100200"),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{visit}");
    assert_ne!(visit["client_id"], json!(customer_card.to_string()));

    let cards = s.clients(&s.token).await;
    assert_eq!(cards.len(), 2, "{cards:?}");
    let customer = cards
        .iter()
        .find(|card| card["id"] == json!(customer_card.to_string()))
        .unwrap();
    assert_eq!(customer["confirmed_appointments"], 0);
}

// ---------------------------------------------------------------------------
// Creating and editing cards
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
async fn a_card_can_be_made_by_hand_with_or_without_a_phone(pool: PgPool) {
    let s = shop(&pool).await;
    let (status, bare) = s
        .req(
            Method::POST,
            "/clients",
            &s.token,
            Some(json!({ "full_name": "  Ola  " })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{bare}");
    assert_eq!(bare["full_name"], "Ola");
    assert!(bare["phone"].is_null());
    assert_eq!(bare["source"], "staff");
    assert_eq!(bare["completed_count"], 0);
    assert_eq!(bare["total_spent_minor"], 0);
    assert_eq!(bare["total_spent_currency"], "PLN");

    // Two cards without a phone are fine: only a phone identifies a person.
    let (status, _) = s
        .req(
            Method::POST,
            "/clients",
            &s.token,
            Some(json!({ "full_name": "Ola" })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, ewa) = s
        .req(
            Method::POST,
            "/clients",
            &s.token,
            Some(json!({
                "full_name": "Ewa", "phone": "+48 600 100 200",
                "email": " Ewa@Example.PL ", "note": "Prefers mornings"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{ewa}");
    assert_eq!(ewa["phone"], "+48600100200");
    assert_eq!(ewa["email"], "ewa@example.pl");
    assert_eq!(ewa["note"], "Prefers mornings");

    // The same number again: 409 with a machine code and the existing card.
    let (status, duplicate) = s
        .req(
            Method::POST,
            "/clients",
            &s.token,
            Some(json!({ "full_name": "Ewa again", "phone": "+48600100200" })),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{duplicate}");
    assert_eq!(duplicate["code"], "CLIENT_PHONE_EXISTS");
    assert_eq!(duplicate["details"], ewa["id"]);

    for body in [
        json!({ "full_name": "   " }),
        json!({ "full_name": "Ewa", "phone": "600100200" }),
        json!({ "full_name": "Ewa", "email": "not-an-email" }),
        json!({ "full_name": "Ewa", "note": "x".repeat(1001) }),
    ] {
        let (status, _) = s.req(Method::POST, "/clients", &s.token, Some(body)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    // A booking with that number now finds the card made by hand.
    let (_, visit) = s
        .book(
            &s.token,
            "crm-key-0001",
            &s.staff,
            "2027-03-22T08:00:00Z",
            "Ewa N.",
            Some("+48600100200"),
        )
        .await;
    assert_eq!(visit["client_id"], ewa["id"]);
    assert_eq!(s.clients(&s.token).await.len(), 3);
}

#[sqlx::test(migrations = "./migrations")]
async fn cards_are_edited_within_their_rules(pool: PgPool) {
    let s = shop(&pool).await;
    let (_, ewa) = s
        .req(
            Method::POST,
            "/clients",
            &s.token,
            Some(json!({ "full_name": "Ewa", "phone": "+48600100200" })),
        )
        .await;
    let (_, kasia) = s
        .req(
            Method::POST,
            "/clients",
            &s.token,
            Some(json!({ "full_name": "Kasia", "phone": "+48601000000" })),
        )
        .await;
    let id = ewa["id"].as_str().unwrap();
    let path = format!("/clients/{id}");

    let (status, edited) = s
        .req(
            Method::PATCH,
            &path,
            &s.token,
            Some(json!({ "full_name": "Ewa Nowak", "email": "EWA@example.pl", "note": "Allergic to nothing we know of" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{edited}");
    assert_eq!(edited["full_name"], "Ewa Nowak");
    assert_eq!(edited["email"], "ewa@example.pl");
    assert_eq!(edited["phone"], "+48600100200"); // untouched

    // An empty text clears the field; the name cannot be emptied.
    let (status, cleared) = s
        .req(
            Method::PATCH,
            &path,
            &s.token,
            Some(json!({ "email": "", "note": "  " })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{cleared}");
    assert!(cleared["email"].is_null());
    assert!(cleared.get("note").map(Value::is_null).unwrap_or(true));
    let (status, _) = s
        .req(
            Method::PATCH,
            &path,
            &s.token,
            Some(json!({ "full_name": " " })),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = s.req(Method::PATCH, &path, &s.token, Some(json!({}))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Taking another card's number is refused, pointing at that card.
    let (status, clash) = s
        .req(
            Method::PATCH,
            &path,
            &s.token,
            Some(json!({ "phone": "+48 601 000 000" })),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{clash}");
    assert_eq!(clash["code"], "CLIENT_PHONE_EXISTS");
    assert_eq!(clash["details"], kasia["id"]);
    // Keeping one's own number is not a clash; the phone can also be removed.
    let (status, _) = s
        .req(
            Method::PATCH,
            &path,
            &s.token,
            Some(json!({ "phone": "+48600100200" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, no_phone) = s
        .req(Method::PATCH, &path, &s.token, Some(json!({ "phone": "" })))
        .await;
    assert_eq!(status, StatusCode::OK, "{no_phone}");
    assert!(no_phone["phone"].is_null());

    // A customer who uses the app keeps their own contact details: note only.
    let (_, user_id) = sign_up(&s.app, "customer@example.pl").await;
    let account: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO client (business_id, user_id, source, full_name, phone_e164)
         VALUES ($1::text::uuid, $2::text::uuid, 'account', 'Customer', '+48777000111')
         RETURNING id",
    )
    .bind(&s.biz)
    .bind(&user_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let account_path = format!("/clients/{account}");
    for body in [
        json!({ "full_name": "Renamed" }),
        json!({ "phone": "+48777000222" }),
        json!({ "email": "x@example.pl" }),
    ] {
        let (status, _) = s
            .req(Method::PATCH, &account_path, &s.token, Some(body))
            .await;
        assert_eq!(status, StatusCode::CONFLICT);
    }
    let (status, noted) = s
        .req(
            Method::PATCH,
            &account_path,
            &s.token,
            Some(json!({ "note": "VIP" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{noted}");
    assert_eq!(noted["note"], "VIP");
    assert_eq!(noted["full_name"], "Customer");
}

#[sqlx::test(migrations = "./migrations")]
async fn only_owner_manager_and_reception_edit_cards(pool: PgPool) {
    let s = shop(&pool).await;
    let (_, card) = s
        .req(
            Method::POST,
            "/clients",
            &s.token,
            Some(json!({ "full_name": "Ewa", "phone": "+48600100200" })),
        )
        .await;
    let id = card["id"].as_str().unwrap();
    let (rita, rita_id) = sign_up(&s.app, "rita@example.pl").await;
    add_member(&pool, &s.biz, &rita_id, "reception").await;
    let (max, max_id) = sign_up(&s.app, "max@example.pl").await;
    add_member(&pool, &s.biz, &max_id, "manager").await;

    for token in [&rita, &max] {
        let (status, made) = s
            .req(
                Method::POST,
                "/clients",
                token,
                Some(json!({ "full_name": "By staff" })),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "{made}");
        let (status, _) = s
            .req(
                Method::PATCH,
                &format!("/clients/{id}"),
                token,
                Some(json!({ "note": "seen" })),
            )
            .await;
        assert_eq!(status, StatusCode::OK);
    }

    let (olga, _) = s.employee(&pool, "olga@example.pl").await;
    let (status, _) = s
        .req(
            Method::POST,
            "/clients",
            &olga,
            Some(json!({ "full_name": "Nope" })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = s
        .req(
            Method::PATCH,
            &format!("/clients/{id}"),
            &olga,
            Some(json!({ "note": "nope" })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Someone who is not a member gets nothing.
    let (stranger, _) = sign_up(&s.app, "stranger@example.pl").await;
    let (status, _) = s.get("/clients", &stranger).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// Numbers and history
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
async fn the_numbers_count_completed_visits_at_their_booked_price(pool: PgPool) {
    let s = shop(&pool).await;
    let phone = Some("+48600100200");
    let mut ids = Vec::new();
    for (key, start) in [
        ("crm-key-0001", "2027-03-22T08:00:00Z"),
        ("crm-key-0002", "2027-03-22T10:00:00Z"),
        ("crm-key-0003", "2027-03-23T08:00:00Z"),
        ("crm-key-0004", "2027-03-23T10:00:00Z"),
    ] {
        let (status, body) = s.book(&s.token, key, &s.staff, start, "Ewa", phone).await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        ids.push(body["id"].as_str().unwrap().to_string());
    }
    let client_id = s.clients(&s.token).await[0]["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Monday 11:00Z: the first two visits have started. One is done, one is a no-show;
    // the third is cancelled; the fourth is still ahead.
    let later = app_at(&pool, datetime!(2027-03-22 11:00 UTC)).await;
    let (status, body) = send(
        &later,
        Method::POST,
        &format!("{}/appointments/{}/complete", s.base, ids[0]),
        &s.token,
        None,
        Some(json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) = send(
        &later,
        Method::POST,
        &format!("{}/appointments/{}/no-show", s.base, ids[1]),
        &s.token,
        None,
        Some(json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) = send(
        &later,
        Method::POST,
        &format!("{}/appointments/{}/cancel", s.base, ids[2]),
        &s.token,
        None,
        Some(json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, card) = s.client(&s.token, &client_id).await;
    assert_eq!(status, StatusCode::OK, "{card}");
    assert_eq!(card["completed_count"], 1);
    assert_eq!(card["no_show_count"], 1);
    assert_eq!(card["confirmed_appointments"], 1);
    assert_eq!(card["total_spent_minor"], 25000);
    assert_eq!(card["total_spent_currency"], "PLN");
    assert!(card["last_visit_at"]
        .as_str()
        .unwrap()
        .starts_with("2027-03-22T08:00"));

    // The booked price is a snapshot: a price change later does not rewrite the total.
    let (status, _) = s
        .req(
            Method::PATCH,
            &format!("/variants/{}", s.variant),
            &s.token,
            Some(json!({ "price_minor": 99900 })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (_, card) = s.client(&s.token, &client_id).await;
    assert_eq!(card["total_spent_minor"], 25000);

    // A client with no visits at all reads as zeros, not as missing.
    let (_, fresh) = s
        .req(
            Method::POST,
            "/clients",
            &s.token,
            Some(json!({ "full_name": "New" })),
        )
        .await;
    assert!(fresh["last_visit_at"].is_null());
    assert_eq!(fresh["total_spent_minor"], 0);

    // Sorting by the latest visit puts the client with visits first.
    let (status, recent) = s.get("/clients?sort=recent", &s.token).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(recent[0]["id"], json!(client_id));
    let (status, _) = s.get("/clients?sort=bogus", &s.token).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "./migrations")]
async fn history_is_newest_first_and_pages(pool: PgPool) {
    let s = shop(&pool).await;
    let phone = Some("+48600100200");
    let mut ids = Vec::new();
    for (key, start) in [
        ("crm-key-0001", "2027-03-22T08:00:00Z"),
        ("crm-key-0002", "2027-03-22T10:00:00Z"),
        ("crm-key-0003", "2027-03-23T08:00:00Z"),
    ] {
        let (_, body) = s.book(&s.token, key, &s.staff, start, "Ewa", phone).await;
        ids.push(body["id"].as_str().unwrap().to_string());
    }
    let (status, body) = s
        .req(
            Method::POST,
            &format!("/appointments/{}/cancel", ids[1]),
            &s.token,
            Some(json!({})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    // A hold is not a visit and never shows up here.
    let (status, hold) = send(
        &s.app,
        Method::POST,
        &format!("{}/holds", s.base),
        &s.token,
        Some("crm-hold-0001"),
        Some(json!({
            "service_id": s.service, "variant_id": s.variant,
            "staff_id": s.staff, "start_at": "2027-03-23T10:00:00Z"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{hold}");

    let client_id = s.clients(&s.token).await[0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let path = format!("/clients/{client_id}/appointments");
    let (status, all) = s.get(&path, &s.token).await;
    assert_eq!(status, StatusCode::OK, "{all}");
    let list = all.as_array().unwrap();
    assert_eq!(list.len(), 3, "{all}");
    let starts: Vec<&str> = list
        .iter()
        .map(|v| v["start_at"].as_str().unwrap())
        .collect();
    assert!(starts.windows(2).all(|w| w[0] > w[1]), "{starts:?}");
    let statuses: Vec<&str> = list.iter().map(|v| v["status"].as_str().unwrap()).collect();
    assert_eq!(statuses, ["confirmed", "cancelled", "confirmed"]);
    // What was booked, as it was booked.
    assert_eq!(list[0]["service_name"]["pl"], "Classic");
    assert_eq!(list[0]["price_minor"], 25000);
    assert_eq!(list[0]["client_id"], json!(client_id));

    let (_, page) = s.get(&format!("{path}?limit=1&offset=1"), &s.token).await;
    assert_eq!(page.as_array().unwrap().len(), 1);
    assert_eq!(page[0]["id"], list[1]["id"]);

    let (status, _) = s
        .get(
            "/clients/00000000-0000-4000-8000-000000000001/appointments",
            &s.token,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// Booking for a chosen card
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
async fn a_visit_can_be_booked_for_a_chosen_card(pool: PgPool) {
    let s = shop(&pool).await;
    let (_, ewa) = s
        .req(
            Method::POST,
            "/clients",
            &s.token,
            Some(json!({ "full_name": "Ewa Nowak", "phone": "+48600100200" })),
        )
        .await;
    let (_, no_phone) = s
        .req(
            Method::POST,
            "/clients",
            &s.token,
            Some(json!({ "full_name": "Ola" })),
        )
        .await;

    // Only the card is sent: its name and phone become the visit's.
    let (status, visit) = send(
        &s.app,
        Method::POST,
        &format!("{}/appointments", s.base),
        &s.token,
        Some("crm-key-0001"),
        Some(json!({
            "service_id": s.service, "variant_id": s.variant, "staff_id": s.staff,
            "start_at": "2027-03-22T08:00:00Z", "client_id": ewa["id"]
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{visit}");
    assert_eq!(visit["client_id"], ewa["id"]);
    assert_eq!(visit["client_name"], "Ewa Nowak");
    assert_eq!(visit["client_phone"], "+48600100200");

    // A card without a phone gives a visit without one, still linked.
    let (status, visit) = send(
        &s.app,
        Method::POST,
        &format!("{}/appointments", s.base),
        &s.token,
        Some("crm-key-0002"),
        Some(json!({
            "service_id": s.service, "variant_id": s.variant, "staff_id": s.staff,
            "start_at": "2027-03-22T10:00:00Z", "client_id": no_phone["id"]
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{visit}");
    assert_eq!(visit["client_id"], no_phone["id"]);
    assert!(visit["client_phone"].is_null());

    // An unknown card is a 404 and books nothing.
    let (status, _) = send(
        &s.app,
        Method::POST,
        &format!("{}/appointments", s.base),
        &s.token,
        Some("crm-key-0003"),
        Some(json!({
            "service_id": s.service, "variant_id": s.variant, "staff_id": s.staff,
            "start_at": "2027-03-23T08:00:00Z",
            "client_id": "00000000-0000-4000-8000-000000000001"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Without a card the name is still required.
    let (status, _) = send(
        &s.app,
        Method::POST,
        &format!("{}/appointments", s.base),
        &s.token,
        Some("crm-key-0004"),
        Some(json!({
            "service_id": s.service, "variant_id": s.variant, "staff_id": s.staff,
            "start_at": "2027-03-23T08:00:00Z"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

// ---------------------------------------------------------------------------
// The employee's narrow view
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
async fn an_employee_sees_only_clients_of_their_own_visits(pool: PgPool) {
    let s = shop(&pool).await;
    let (olga, olga_staff) = s.employee(&pool, "olga@example.pl").await;

    // Anna's client, whom Olga never served.
    let (_, ann_visit) = s
        .book(
            &s.token,
            "crm-key-0001",
            &s.staff,
            "2027-03-22T08:00:00Z",
            "Anna's client",
            Some("+48600000001"),
        )
        .await;
    let annas = ann_visit["client_id"].as_str().unwrap().to_string();

    // Olga books her own client by hand: the card appears for her by itself.
    let (status, own) = s
        .book(
            &olga,
            "crm-key-0002",
            &olga_staff,
            "2027-03-22T08:00:00Z",
            "Olga's client",
            Some("+48600000002"),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{own}");
    let olgas = own["client_id"].as_str().unwrap().to_string();
    // Anna serves Olga's client too, on another day.
    let (status, shared) = s
        .book(
            &s.token,
            "crm-key-0003",
            &s.staff,
            "2027-03-22T10:00:00Z",
            "Olga's client",
            Some("+48600000002"),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{shared}");
    assert_eq!(shared["client_id"], json!(olgas));

    // The owner sees both and can keep a note.
    assert_eq!(s.clients(&s.token).await.len(), 2);
    let (status, _) = s
        .req(
            Method::PATCH,
            &format!("/clients/{olgas}"),
            &s.token,
            Some(json!({ "note": "Likes tea" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (_, owner_view) = s.client(&s.token, &olgas).await;
    assert_eq!(owner_view["note"], "Likes tea");
    assert_eq!(owner_view["confirmed_appointments"], 2);

    // Olga sees only hers, never the note, and counts only her own visits.
    let list = s.clients(&olga).await;
    assert_eq!(list.len(), 1, "{list:?}");
    assert_eq!(list[0]["id"], json!(olgas));
    assert!(list[0].get("note").is_none(), "{}", list[0]);
    assert_eq!(list[0]["confirmed_appointments"], 1);
    let (status, mine) = s.client(&olga, &olgas).await;
    assert_eq!(status, StatusCode::OK, "{mine}");
    assert!(mine.get("note").is_none());
    assert_eq!(mine["confirmed_appointments"], 1);

    // Anna's client does not exist for her: card, history, search, booking for them.
    let (status, _) = s.client(&olga, &annas).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = s
        .get(&format!("/clients/{annas}/appointments"), &olga)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, found) = s.get("/clients?q=Anna", &olga).await;
    assert_eq!(status, StatusCode::OK);
    assert!(found.as_array().unwrap().is_empty());
    let (status, _) = send(
        &s.app,
        Method::POST,
        &format!("{}/appointments", s.base),
        &olga,
        Some("crm-key-0004"),
        Some(json!({
            "service_id": s.service, "variant_id": s.variant, "staff_id": olga_staff,
            "start_at": "2027-03-22T10:00:00Z", "client_id": annas
        })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Her history holds her visit only, not Anna's visit with the same client.
    let (status, history) = s
        .get(&format!("/clients/{olgas}/appointments"), &olga)
        .await;
    assert_eq!(status, StatusCode::OK, "{history}");
    assert_eq!(history.as_array().unwrap().len(), 1);
    assert_eq!(history[0]["staff_id"], json!(olga_staff));
    let (_, full) = s
        .get(&format!("/clients/{olgas}/appointments"), &s.token)
        .await;
    assert_eq!(full.as_array().unwrap().len(), 2);

    // She may book again for her own client by card.
    let (status, again) = send(
        &s.app,
        Method::POST,
        &format!("{}/appointments", s.base),
        &olga,
        Some("crm-key-0005"),
        Some(json!({
            "service_id": s.service, "variant_id": s.variant, "staff_id": olga_staff,
            "start_at": "2027-03-22T10:00:00Z", "client_id": olgas
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{again}");
    assert_eq!(again["client_name"], "Olga's client");
}

#[sqlx::test(migrations = "./migrations")]
async fn an_employee_counts_only_their_own_finished_visits(pool: PgPool) {
    let s = shop(&pool).await;
    let (olga, olga_staff) = s.employee(&pool, "olga@example.pl").await;
    let phone = Some("+48600000002");
    let (_, hers) = s
        .book(
            &olga,
            "crm-key-0001",
            &olga_staff,
            "2027-03-22T08:00:00Z",
            "Ewa",
            phone,
        )
        .await;
    let (_, annas) = s
        .book(
            &s.token,
            "crm-key-0002",
            &s.staff,
            "2027-03-22T08:00:00Z",
            "Ewa",
            phone,
        )
        .await;
    let client_id = hers["client_id"].as_str().unwrap().to_string();

    let later = app_at(&pool, datetime!(2027-03-22 11:00 UTC)).await;
    for (token, visit) in [(&olga, &hers), (&s.token, &annas)] {
        let id = visit["id"].as_str().unwrap();
        let (status, body) = send(
            &later,
            Method::POST,
            &format!("{}/appointments/{id}/complete", s.base),
            token,
            None,
            Some(json!({})),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
    }
    let (_, owner_view) = s.client(&s.token, &client_id).await;
    assert_eq!(owner_view["completed_count"], 2);
    assert_eq!(owner_view["total_spent_minor"], 50000);
    let (_, employee_view) = s.client(&olga, &client_id).await;
    assert_eq!(employee_view["completed_count"], 1);
    assert_eq!(employee_view["total_spent_minor"], 25000);
}

// ---------------------------------------------------------------------------
// Audit
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
async fn client_changes_are_logged_without_personal_data(pool: PgPool) {
    let s = shop(&pool).await;
    let (_, card) = s
        .req(
            Method::POST,
            "/clients",
            &s.token,
            Some(json!({ "full_name": "Ewa", "phone": "+48600100200" })),
        )
        .await;
    let id = card["id"].as_str().unwrap();
    s.req(
        Method::PATCH,
        &format!("/clients/{id}"),
        &s.token,
        Some(json!({ "note": "secret-note", "email": "ewa@example.pl" })),
    )
    .await;
    s.book(
        &s.token,
        "crm-key-0001",
        &s.staff,
        "2027-03-22T08:00:00Z",
        "New Person",
        Some("+48601000000"),
    )
    .await;

    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT action, meta::text FROM audit_log WHERE entity = 'client' ORDER BY created_at, id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    let actions: Vec<&str> = rows.iter().map(|(action, _)| action.as_str()).collect();
    assert_eq!(
        actions,
        ["client.create", "client.update", "client.create"],
        "{rows:?}"
    );
    assert!(rows[0].1.contains("manual"));
    assert!(rows[1].1.contains("note") && rows[1].1.contains("email"));
    assert!(rows[2].1.contains("booking"));
    for (_, meta) in &rows {
        for secret in [
            "secret-note",
            "ewa@example.pl",
            "+48600100200",
            "+48601000000",
            "New Person",
        ] {
            assert!(!meta.contains(secret), "{meta}");
        }
    }
}

// ---------------------------------------------------------------------------
// The migration's backfill
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
async fn the_backfill_links_old_manual_visits_by_phone(pool: PgPool) {
    let s = shop(&pool).await;
    let other = shop_with(app_at(&pool, NOW).await, "beata@example.pl").await;

    // Old-style data: manual visits with no card. Made through the API, then stripped.
    for (key, start, name, phone) in [
        (
            "old-key-0001",
            "2027-03-22T08:00:00Z",
            "Ewa",
            Some("+48600100200"),
        ),
        (
            "old-key-0002",
            "2027-03-22T10:00:00Z",
            "Ewa Nowak",
            Some("+48600100200"),
        ),
        (
            "old-key-0003",
            "2027-03-23T08:00:00Z",
            "Kasia",
            Some("+48601000000"),
        ),
        ("old-key-0004", "2027-03-23T10:00:00Z", "No Phone", None),
    ] {
        let (status, body) = s.book(&s.token, key, &s.staff, start, name, phone).await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
    }
    let (status, body) = other
        .book(
            &other.token,
            "old-key-0001",
            &other.staff,
            "2027-03-22T08:00:00Z",
            "Ewa",
            Some("+48600100200"),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    sqlx::query("UPDATE appointment SET client_id = NULL")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM client")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DROP INDEX client_staff_phone_key")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("ALTER TABLE client DROP COLUMN note")
        .execute(&pool)
        .await
        .unwrap();
    let versions_before: Vec<(uuid::Uuid, i32)> =
        sqlx::query_as("SELECT id, version FROM appointment ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();

    // Run the migration again over that state.
    sqlx::raw_sql(include_str!("../migrations/20260930100000_crm.sql"))
        .execute(&pool)
        .await
        .unwrap();

    // Two people in Anna's business, one in the other; the no-phone visit has no card.
    let cards: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT b.name || '/' || c.source, c.full_name, c.phone_e164
         FROM client c JOIN business b ON b.id = c.business_id
         ORDER BY c.business_id, c.phone_e164",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(cards.len(), 3, "{cards:?}");
    assert!(cards.iter().all(|(who, _, _)| who.ends_with("/staff")));
    // The name of the latest visit wins.
    assert!(cards
        .iter()
        .any(|(_, name, phone)| name == "Ewa Nowak" && phone == "+48600100200"));

    let (linked, unlinked): (i64, i64) = sqlx::query_as(
        "SELECT count(*) FILTER (WHERE client_id IS NOT NULL),
                count(*) FILTER (WHERE client_id IS NULL)
         FROM appointment",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!((linked, unlinked), (4, 1));
    let no_phone_card: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM appointment WHERE client_phone IS NULL AND client_id IS NOT NULL",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(no_phone_card, 0);

    // Each visit points at a card of its own business, with its own phone.
    let mismatched: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM appointment a JOIN client c ON c.id = a.client_id
         WHERE c.business_id <> a.business_id OR c.phone_e164 <> a.client_phone",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(mismatched, 0);

    // Linking history is not an edit of the visits.
    let versions_after: Vec<(uuid::Uuid, i32)> =
        sqlx::query_as("SELECT id, version FROM appointment ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(versions_before, versions_after);

    // The phone is unique per business again, and the API sees the cards.
    let duplicate = sqlx::query(
        "INSERT INTO client (business_id, source, full_name, phone_e164)
         VALUES ($1::text::uuid, 'staff', 'Copy', '+48600100200')",
    )
    .bind(&s.biz)
    .execute(&pool)
    .await;
    assert!(duplicate.is_err());
    assert_eq!(s.clients(&s.token).await.len(), 2);

    // Running it a second time changes nothing (safe to repeat on a half-done database).
    sqlx::query("DROP INDEX client_staff_phone_key")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("ALTER TABLE client DROP COLUMN note")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::raw_sql(include_str!("../migrations/20260930100000_crm.sql"))
        .execute(&pool)
        .await
        .unwrap();
    let total: i64 = sqlx::query_scalar("SELECT count(*) FROM client")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(total, 3);
}
