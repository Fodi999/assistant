//! End-to-end tests of the service catalog through the real router.

mod common;

use axum::http::{Method, StatusCode};
use common::{add_member, app, call, create_business, my_staff_id, sign_up};
use serde_json::json;
use sqlx::PgPool;

#[sqlx::test(migrations = "./migrations")]
async fn catalog_lifecycle(pool: PgPool) {
    let app = app(&pool).await;
    let (token, _) = sign_up(&app, "anna@example.pl").await;
    let biz = create_business(&app, &token, "Anna Lashes").await;
    let staff = my_staff_id(&app, &token, &biz).await;
    let base = format!("/v1/businesses/{biz}");

    // Category.
    let (status, category) = call(
        &app,
        Method::POST,
        &format!("{base}/categories"),
        Some(&token),
        Some(json!({ "name": { "pl": "Rzęsy", "en": "Lashes" }, "sort_order": 1 })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{category}");
    assert_eq!(category["name"]["en"], "Lashes");
    assert_eq!(category["version"], 1);
    let category_id = category["id"].as_str().unwrap().to_string();

    // Service created together with two variants.
    let (status, service) = call(
        &app,
        Method::POST,
        &format!("{base}/services"),
        Some(&token),
        Some(json!({
            "category_id": category_id,
            "name": { "pl": "Klasyczne 1:1", "en": "Classic 1:1" },
            "buffer_after_min": 10,
            "variants": [
                { "name": { "pl": "Nowy zestaw" }, "duration_min": 120, "price_minor": 25000 },
                { "duration_min": 150, "price_minor": 32000, "price_type": "from" }
            ]
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{service}");
    let service_id = service["id"].as_str().unwrap().to_string();
    assert_eq!(service["category_id"], category_id.as_str());
    assert_eq!(service["booking_step_minutes"], 15);
    assert_eq!(service["buffer_after_min"], 10);
    assert_eq!(service["staff_ids"], json!([]));
    let variants = service["variants"].as_array().unwrap();
    assert_eq!(variants.len(), 2);
    // Money is an integer in minor units, with the business currency.
    assert_eq!(variants[0]["price_minor"], 25000);
    assert_eq!(variants[0]["currency"], "PLN");
    assert_eq!(variants[1]["price_type"], "from");

    // Listed.
    let (status, list) = call(
        &app,
        Method::GET,
        &format!("{base}/services"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 1);
    assert_eq!(list[0]["variants"].as_array().unwrap().len(), 2);

    // Staff who perform it.
    let (status, service) = call(
        &app,
        Method::PUT,
        &format!("{base}/services/{service_id}/staff"),
        Some(&token),
        Some(json!({ "staff_ids": [staff, staff] })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{service}");
    assert_eq!(service["staff_ids"], json!([staff]));

    // Update: set fields, then clear the description and the category.
    let (status, service) = call(
        &app,
        Method::PATCH,
        &format!("{base}/services/{service_id}"),
        Some(&token),
        Some(json!({
            "is_active": false,
            "booking_step_minutes": 30,
            "description": { "pl": "Naturalny efekt" }
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{service}");
    assert_eq!(service["is_active"], false);
    assert_eq!(service["booking_step_minutes"], 30);
    assert_eq!(service["description"]["pl"], "Naturalny efekt");
    assert_eq!(service["version"], 2);

    let (status, service) = call(
        &app,
        Method::PATCH,
        &format!("{base}/services/{service_id}"),
        Some(&token),
        Some(json!({ "description": null, "category_id": null })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{service}");
    assert_eq!(service["description"], json!(null));
    assert_eq!(service["category_id"], json!(null));
    assert_eq!(
        service["name"]["pl"], "Klasyczne 1:1",
        "untouched fields stay"
    );

    // Variants: add, change, remove.
    let (status, variant) = call(
        &app,
        Method::POST,
        &format!("{base}/services/{service_id}/variants"),
        Some(&token),
        Some(json!({ "name": { "pl": "Korekta" }, "duration_min": 60, "price_minor": 15000 })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{variant}");
    let variant_id = variant["id"].as_str().unwrap().to_string();

    let (status, variant) = call(
        &app,
        Method::PATCH,
        &format!("{base}/variants/{variant_id}"),
        Some(&token),
        Some(json!({ "price_minor": 17000, "duration_min": 75 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{variant}");
    assert_eq!(variant["price_minor"], 17000);
    assert_eq!(variant["duration_min"], 75);
    assert_eq!(variant["version"], 2);

    let (status, _) = call(
        &app,
        Method::DELETE,
        &format!("{base}/variants/{variant_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, service) = call(
        &app,
        Method::GET,
        &format!("{base}/services/{service_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(service["variants"].as_array().unwrap().len(), 2);

    // Deleting a category keeps its services, uncategorised.
    let (status, service) = call(
        &app,
        Method::PATCH,
        &format!("{base}/services/{service_id}"),
        Some(&token),
        Some(json!({ "category_id": category_id })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{service}");
    let (status, _) = call(
        &app,
        Method::DELETE,
        &format!("{base}/categories/{category_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, categories) = call(
        &app,
        Method::GET,
        &format!("{base}/categories"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(categories, json!([]));
    let (_, service) = call(
        &app,
        Method::GET,
        &format!("{base}/services/{service_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(service["category_id"], json!(null));

    // Deleting the service hides it and its variants; the row is kept.
    let (status, _) = call(
        &app,
        Method::DELETE,
        &format!("{base}/services/{service_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(
        &app,
        Method::GET,
        &format!("{base}/services/{service_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (_, list) = call(
        &app,
        Method::GET,
        &format!("{base}/services"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(list, json!([]));
    let kept: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM service WHERE id = $1::text::uuid AND deleted_at IS NOT NULL",
    )
    .bind(&service_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(kept, 1);
    let staff_links: i64 = sqlx::query_scalar("SELECT count(*) FROM staff_service")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(staff_links, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn catalog_input_is_validated(pool: PgPool) {
    let app = app(&pool).await;
    let (token, _) = sign_up(&app, "anna@example.pl").await;
    let biz = create_business(&app, &token, "Anna Lashes").await;
    let base = format!("/v1/businesses/{biz}");
    let services = format!("{base}/services");

    let bad_services = [
        json!({ "name": {} }),
        json!({ "name": "Classic" }),
        json!({ "name": { "de": "Klassisch" } }),
        json!({ "name": { "pl": "  " } }),
        json!({ "name": { "pl": "Ok" }, "booking_step_minutes": 7 }),
        json!({ "name": { "pl": "Ok" }, "buffer_after_min": 500 }),
        json!({ "name": { "pl": "Ok" }, "max_advance_days": 0 }),
        json!({ "name": { "pl": "Ok" }, "category_id": "00000000-0000-0000-0000-000000000000" }),
        json!({ "name": { "pl": "Ok" }, "intake_questions": { "not": "an array" } }),
        json!({ "name": { "pl": "Ok" }, "variants": [{ "duration_min": 3, "price_minor": 100 }] }),
        json!({ "name": { "pl": "Ok" }, "variants": [{ "duration_min": 60, "price_minor": -1 }] }),
        json!({ "name": { "pl": "Ok" }, "variants": [{ "duration_min": 60, "price_minor": 100, "currency": "EUR" }] }),
        json!({ "name": { "pl": "Ok" }, "variants": [{ "duration_min": 60, "price_minor": 100, "price_type": "free" }] }),
    ];
    for body in bad_services {
        let (status, response) = call(
            &app,
            Method::POST,
            &services,
            Some(&token),
            Some(body.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body} -> {response}");
    }
    // A failed create leaves nothing behind (single transaction).
    let (_, list) = call(&app, Method::GET, &services, Some(&token), None).await;
    assert_eq!(list, json!([]));

    // A free service is fine; the currency may be repeated when it matches.
    let (status, service) = call(
        &app,
        Method::POST,
        &services,
        Some(&token),
        Some(json!({
            "name": { "pl": "Konsultacja" },
            "variants": [{ "duration_min": 15, "price_minor": 0, "currency": "pln" }]
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{service}");
    assert_eq!(service["variants"][0]["price_minor"], 0);
    let service_id = service["id"].as_str().unwrap();

    // Staff must belong to the business; unknown ids are refused.
    let (status, _) = call(
        &app,
        Method::PUT,
        &format!("{services}/{service_id}/staff"),
        Some(&token),
        Some(json!({ "staff_ids": ["00000000-0000-0000-0000-000000000000"] })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Unknown ids in the URL.
    let unknown = "00000000-0000-0000-0000-000000000000";
    for (method, uri, body) in [
        (Method::GET, format!("{services}/{unknown}"), None),
        (
            Method::PATCH,
            format!("{services}/{unknown}"),
            Some(json!({ "is_active": true })),
        ),
        (Method::DELETE, format!("{services}/{unknown}"), None),
        (
            Method::PATCH,
            format!("{base}/variants/{unknown}"),
            Some(json!({ "price_minor": 1 })),
        ),
        (
            Method::PATCH,
            format!("{base}/categories/{unknown}"),
            Some(json!({ "sort_order": 1 })),
        ),
    ] {
        let (status, _) = call(&app, method.clone(), &uri, Some(&token), body).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{method} {uri}");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn catalog_permissions_and_tenant_isolation(pool: PgPool) {
    let app = app(&pool).await;
    let (anna, _) = sign_up(&app, "anna@example.pl").await;
    let (olga, olga_id) = sign_up(&app, "olga@example.pl").await;
    let (bob, _) = sign_up(&app, "bob@example.pl").await;

    let biz_a = create_business(&app, &anna, "Anna Lashes").await;
    let biz_b = create_business(&app, &bob, "Bob Brows").await;
    let staff_a = my_staff_id(&app, &anna, &biz_a).await;
    let _employee_card = add_member(&pool, &biz_a, &olga_id, "employee").await;
    let a = format!("/v1/businesses/{biz_a}");
    let b = format!("/v1/businesses/{biz_b}");

    let (status, category) = call(
        &app,
        Method::POST,
        &format!("{a}/categories"),
        Some(&anna),
        Some(json!({ "name": { "pl": "Rzęsy" } })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let category_a = category["id"].as_str().unwrap().to_string();
    let (status, service) = call(
        &app,
        Method::POST,
        &format!("{a}/services"),
        Some(&anna),
        Some(json!({ "name": { "pl": "Classic" } })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let service_a = service["id"].as_str().unwrap().to_string();

    // An employee reads the catalog and the team, but cannot change anything.
    for uri in [
        format!("{a}/services"),
        format!("{a}/categories"),
        format!("{a}/services/{service_a}"),
        format!("{a}/staff"),
    ] {
        let (status, body) = call(&app, Method::GET, &uri, Some(&olga), None).await;
        assert_eq!(status, StatusCode::OK, "{uri} {body}");
    }
    for (method, uri, body) in [
        (
            Method::POST,
            format!("{a}/categories"),
            json!({ "name": { "pl": "X" } }),
        ),
        (
            Method::POST,
            format!("{a}/services"),
            json!({ "name": { "pl": "X" } }),
        ),
        (
            Method::PATCH,
            format!("{a}/services/{service_a}"),
            json!({ "is_active": false }),
        ),
        (
            Method::DELETE,
            format!("{a}/services/{service_a}"),
            json!({}),
        ),
        (
            Method::POST,
            format!("{a}/services/{service_a}/variants"),
            json!({ "duration_min": 60, "price_minor": 1 }),
        ),
        (
            Method::PUT,
            format!("{a}/services/{service_a}/staff"),
            json!({ "staff_ids": [staff_a] }),
        ),
    ] {
        let (status, _) = call(&app, method.clone(), &uri, Some(&olga), Some(body)).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{method} {uri}");
    }

    // No token: 401.
    let (status, _) = call(&app, Method::GET, &format!("{a}/services"), None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Another business's owner sees nothing of business A.
    for (method, uri, body) in [
        (Method::GET, format!("{a}/services"), None),
        (Method::GET, format!("{a}/services/{service_a}"), None),
        (Method::GET, format!("{a}/staff"), None),
        (
            Method::POST,
            format!("{a}/services"),
            Some(json!({ "name": { "pl": "Evil" } })),
        ),
    ] {
        let (status, _) = call(&app, method.clone(), &uri, Some(&bob), body).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{method} {uri}");
    }
    // Using A's service id inside B's URL finds nothing.
    let (status, _) = call(
        &app,
        Method::GET,
        &format!("{b}/services/{service_a}"),
        Some(&bob),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (_, list) = call(
        &app,
        Method::GET,
        &format!("{b}/services"),
        Some(&bob),
        None,
    )
    .await;
    assert_eq!(list, json!([]));

    // ...and cannot reference A's category or staff from B.
    let (status, _) = call(
        &app,
        Method::POST,
        &format!("{b}/services"),
        Some(&bob),
        Some(json!({ "name": { "pl": "Mine" }, "category_id": category_a })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, mine) = call(
        &app,
        Method::POST,
        &format!("{b}/services"),
        Some(&bob),
        Some(json!({ "name": { "pl": "Mine" } })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let mine_id = mine["id"].as_str().unwrap();
    let (status, _) = call(
        &app,
        Method::PUT,
        &format!("{b}/services/{mine_id}/staff"),
        Some(&bob),
        Some(json!({ "staff_ids": [staff_a] })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // The database refuses the same references even if the API were bypassed.
    let cross_business_link = sqlx::query(
        "INSERT INTO staff_service (staff_id, service_id, business_id)
         VALUES ($1::text::uuid, $2::text::uuid, $3::text::uuid)",
    )
    .bind(&staff_a)
    .bind(mine_id)
    .bind(&biz_b)
    .execute(&pool)
    .await;
    assert!(
        cross_business_link.is_err(),
        "composite foreign key must reject it"
    );
}
