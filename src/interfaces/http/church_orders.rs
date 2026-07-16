use std::collections::HashMap;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::church_content::{
    db_error, delete_owned, get_public_icon_row, optional_non_empty, required, slugify,
    ChurchContentQuery,
};
use super::site_context::CHURCH_SITE_ID;

const OPTION_COLUMNS: &str = "id, site_id, slug, name_uk, name_ru, name_en, photo_url, price_cents, currency, is_active, sort_order, created_at::text AS created_at, updated_at::text AS updated_at";

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ChurchIconOrderOptionDto {
    pub id: Uuid,
    pub site_id: Uuid,
    pub slug: String,
    pub name_uk: String,
    pub name_ru: String,
    pub name_en: String,
    pub photo_url: String,
    pub price_cents: i64,
    pub currency: String,
    pub is_active: bool,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChurchIconOrderOptionPayload {
    pub slug: Option<String>,
    pub name_uk: Option<String>,
    pub name_ru: Option<String>,
    pub name_en: Option<String>,
    pub photo_url: Option<String>,
    pub price_cents: Option<i64>,
    pub currency: Option<String>,
    pub is_active: Option<bool>,
    pub sort_order: Option<i32>,
}

pub async fn list_icon_order_options(
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let rows: Vec<ChurchIconOrderOptionDto> = sqlx::query_as(&format!(
        "SELECT {OPTION_COLUMNS} FROM icon_order_options WHERE site_id = $1 ORDER BY sort_order ASC, name_uk ASC"
    ))
    .bind(query.site_id())
    .fetch_all(&pool)
    .await
    .map_err(db_error)?;

    Ok(Json(rows))
}

pub async fn get_icon_order_option(
    Path(id): Path<Uuid>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let row: ChurchIconOrderOptionDto = sqlx::query_as(&format!(
        "SELECT {OPTION_COLUMNS} FROM icon_order_options WHERE id = $1 AND site_id = $2"
    ))
    .bind(id)
    .bind(query.site_id())
    .fetch_optional(&pool)
    .await
    .map_err(db_error)?
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(row))
}

pub async fn create_icon_order_option(
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
    Json(payload): Json<ChurchIconOrderOptionPayload>,
) -> Result<impl IntoResponse, StatusCode> {
    let name_uk = required(payload.name_uk, "nameUk")?;
    let slug = optional_non_empty(payload.slug).unwrap_or_else(|| slugify(&name_uk));

    let row: ChurchIconOrderOptionDto = sqlx::query_as(&format!(
        r#"INSERT INTO icon_order_options
           (site_id, slug, name_uk, name_ru, name_en, photo_url, price_cents, currency, is_active, sort_order)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
           RETURNING {OPTION_COLUMNS}"#
    ))
    .bind(query.site_id())
    .bind(slug)
    .bind(name_uk)
    .bind(payload.name_ru.unwrap_or_default())
    .bind(payload.name_en.unwrap_or_default())
    .bind(payload.photo_url.unwrap_or_default())
    .bind(payload.price_cents.unwrap_or(0))
    .bind(payload.currency.unwrap_or_else(|| "UAH".into()))
    .bind(payload.is_active.unwrap_or(true))
    .bind(payload.sort_order.unwrap_or(0))
    .fetch_one(&pool)
    .await
    .map_err(db_error)?;

    Ok((StatusCode::CREATED, Json(row)))
}

pub async fn update_icon_order_option(
    Path(id): Path<Uuid>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
    Json(payload): Json<ChurchIconOrderOptionPayload>,
) -> Result<impl IntoResponse, StatusCode> {
    let site_id = query.site_id();
    let current: ChurchIconOrderOptionDto = sqlx::query_as(&format!(
        "SELECT {OPTION_COLUMNS} FROM icon_order_options WHERE id = $1 AND site_id = $2"
    ))
    .bind(id)
    .bind(site_id)
    .fetch_optional(&pool)
    .await
    .map_err(db_error)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let row: ChurchIconOrderOptionDto = sqlx::query_as(&format!(
        r#"UPDATE icon_order_options SET
              slug = $1, name_uk = $2, name_ru = $3, name_en = $4, photo_url = $5,
              price_cents = $6, currency = $7, is_active = $8, sort_order = $9
           WHERE id = $10 AND site_id = $11
           RETURNING {OPTION_COLUMNS}"#
    ))
    .bind(optional_non_empty(payload.slug).unwrap_or(current.slug))
    .bind(optional_non_empty(payload.name_uk).unwrap_or(current.name_uk))
    .bind(payload.name_ru.unwrap_or(current.name_ru))
    .bind(payload.name_en.unwrap_or(current.name_en))
    .bind(payload.photo_url.unwrap_or(current.photo_url))
    .bind(payload.price_cents.unwrap_or(current.price_cents))
    .bind(payload.currency.unwrap_or(current.currency))
    .bind(payload.is_active.unwrap_or(current.is_active))
    .bind(payload.sort_order.unwrap_or(current.sort_order))
    .bind(id)
    .bind(site_id)
    .fetch_one(&pool)
    .await
    .map_err(db_error)?;

    Ok(Json(row))
}

pub async fn delete_icon_order_option(
    Path(id): Path<Uuid>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    delete_owned(&pool, "icon_order_options", id, query.site_id()).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn public_icon_order_options_list(
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let rows: Vec<ChurchIconOrderOptionDto> = sqlx::query_as(&format!(
        "SELECT {OPTION_COLUMNS} FROM icon_order_options WHERE site_id = $1 AND is_active = true ORDER BY sort_order ASC"
    ))
    .bind(CHURCH_SITE_ID)
    .fetch_all(&pool)
    .await
    .map_err(db_error)?;

    Ok(Json(rows))
}

// ── Icon orders ─────────────────────────────────────────────────────────────

const ORDER_COLUMNS: &str = "id, site_id, is_global, order_number, icon_id, icon_title_snapshot, icon_slug_snapshot, customer_name, contact_method, contact_value, preferred_contact_channel, country, city, consecration_requested, comment, consent_given, status, admin_note, total_price_cents, currency, is_read, created_at::text AS created_at, updated_at::text AS updated_at";
const ITEM_COLUMNS: &str = "id, order_id, option_id, option_name_snapshot, price_cents_snapshot, quantity";

const ORDER_STATUSES: [&str; 8] = [
    "new",
    "contacted",
    "confirmed",
    "in_production",
    "ready",
    "shipped",
    "completed",
    "cancelled",
];

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ChurchIconOrderRowDto {
    pub id: Uuid,
    pub site_id: Uuid,
    pub is_global: bool,
    pub order_number: String,
    pub icon_id: Option<Uuid>,
    pub icon_title_snapshot: String,
    pub icon_slug_snapshot: String,
    pub customer_name: String,
    pub contact_method: String,
    pub contact_value: String,
    pub preferred_contact_channel: String,
    pub country: String,
    pub city: String,
    pub consecration_requested: bool,
    pub comment: String,
    pub consent_given: bool,
    pub status: String,
    pub admin_note: String,
    pub total_price_cents: i64,
    pub currency: String,
    pub is_read: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct IconOrderItemDto {
    pub id: Uuid,
    pub order_id: Uuid,
    pub option_id: Option<Uuid>,
    pub option_name_snapshot: String,
    pub price_cents_snapshot: i64,
    pub quantity: i32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChurchIconOrderDto {
    #[serde(flatten)]
    pub order: ChurchIconOrderRowDto,
    pub items: Vec<IconOrderItemDto>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateIconOrderRequest {
    pub status: Option<String>,
    pub admin_note: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnreadCountResponse {
    pub count: i64,
}

async fn get_icon_order_row(
    pool: &PgPool,
    id: Uuid,
    site_id: Uuid,
) -> Result<ChurchIconOrderRowDto, StatusCode> {
    sqlx::query_as(&format!(
        "SELECT {ORDER_COLUMNS} FROM icon_orders WHERE id = $1 AND (site_id = $2 OR is_global = true)"
    ))
    .bind(id)
    .bind(site_id)
    .fetch_optional(pool)
    .await
    .map_err(db_error)?
    .ok_or(StatusCode::NOT_FOUND)
}

async fn fetch_items_for_order(
    pool: &PgPool,
    order_id: Uuid,
) -> Result<Vec<IconOrderItemDto>, StatusCode> {
    sqlx::query_as(&format!(
        "SELECT {ITEM_COLUMNS} FROM icon_order_items WHERE order_id = $1"
    ))
    .bind(order_id)
    .fetch_all(pool)
    .await
    .map_err(db_error)
}

pub async fn list_icon_orders(
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let site_id = query.site_id();
    let orders: Vec<ChurchIconOrderRowDto> = sqlx::query_as(&format!(
        "SELECT {ORDER_COLUMNS} FROM icon_orders WHERE (site_id = $1 OR is_global = true) ORDER BY created_at DESC"
    ))
    .bind(site_id)
    .fetch_all(&pool)
    .await
    .map_err(db_error)?;

    let order_ids: Vec<Uuid> = orders.iter().map(|order| order.id).collect();
    let items: Vec<IconOrderItemDto> = if order_ids.is_empty() {
        Vec::new()
    } else {
        sqlx::query_as(&format!(
            "SELECT {ITEM_COLUMNS} FROM icon_order_items WHERE order_id = ANY($1)"
        ))
        .bind(&order_ids)
        .fetch_all(&pool)
        .await
        .map_err(db_error)?
    };

    let mut by_order: HashMap<Uuid, Vec<IconOrderItemDto>> = HashMap::new();
    for item in items {
        by_order.entry(item.order_id).or_default().push(item);
    }

    let result: Vec<ChurchIconOrderDto> = orders
        .into_iter()
        .map(|order| {
            let items = by_order.remove(&order.id).unwrap_or_default();
            ChurchIconOrderDto { order, items }
        })
        .collect();

    Ok(Json(result))
}

pub async fn get_icon_order(
    Path(id): Path<Uuid>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let order = get_icon_order_row(&pool, id, query.site_id()).await?;
    let items = fetch_items_for_order(&pool, order.id).await?;
    Ok(Json(ChurchIconOrderDto { order, items }))
}

pub async fn update_icon_order(
    Path(id): Path<Uuid>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
    Json(payload): Json<UpdateIconOrderRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let site_id = query.site_id();
    if let Some(status) = &payload.status {
        if !ORDER_STATUSES.contains(&status.as_str()) {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    let current = get_icon_order_row(&pool, id, site_id).await?;

    let row: ChurchIconOrderRowDto = sqlx::query_as(&format!(
        r#"UPDATE icon_orders SET status = $1, admin_note = $2
           WHERE id = $3 AND site_id = $4
           RETURNING {ORDER_COLUMNS}"#
    ))
    .bind(payload.status.unwrap_or(current.status))
    .bind(payload.admin_note.unwrap_or(current.admin_note))
    .bind(id)
    .bind(site_id)
    .fetch_one(&pool)
    .await
    .map_err(db_error)?;

    let items = fetch_items_for_order(&pool, row.id).await?;
    Ok(Json(ChurchIconOrderDto { order: row, items }))
}

pub async fn mark_icon_order_read(
    Path(id): Path<Uuid>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let site_id = query.site_id();
    let result = sqlx::query("UPDATE icon_orders SET is_read = true WHERE id = $1 AND site_id = $2")
        .bind(id)
        .bind(site_id)
        .execute(&pool)
        .await
        .map_err(db_error)?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn count_unread_icon_orders(
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let site_id = query.site_id();
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM icon_orders WHERE (site_id = $1 OR is_global = true) AND is_read = false",
    )
    .bind(site_id)
    .fetch_one(&pool)
    .await
    .map_err(db_error)?;

    Ok(Json(UnreadCountResponse { count }))
}

// ── Public: submit an icon order ────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateIconOrderItemInput {
    pub option_id: Uuid,
    pub quantity: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateIconOrderRequest {
    pub icon_id: Uuid,
    pub customer_name: String,
    pub contact_method: String,
    pub contact_value: String,
    pub preferred_contact_channel: Option<String>,
    pub country: Option<String>,
    pub city: Option<String>,
    pub consecration_requested: Option<bool>,
    pub comment: Option<String>,
    pub consent_given: bool,
    #[serde(default)]
    pub items: Vec<CreateIconOrderItemInput>,
    /// Honeypot: real visitors never see or fill this field.
    pub website: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateIconOrderResponse {
    pub order_number: String,
}

fn option_label(option: &ChurchIconOrderOptionDto) -> String {
    if !option.name_uk.trim().is_empty() {
        option.name_uk.clone()
    } else if !option.name_ru.trim().is_empty() {
        option.name_ru.clone()
    } else {
        option.name_en.clone()
    }
}

fn extract_ip_from_headers(headers: &HeaderMap) -> Option<String> {
    if let Some(forwarded) = headers.get("x-forwarded-for") {
        if let Ok(value) = forwarded.to_str() {
            if let Some(first) = value.split(',').next() {
                let trimmed = first.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
    }
    if let Some(real_ip) = headers.get("x-real-ip") {
        if let Ok(value) = real_ip.to_str() {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

pub async fn public_create_icon_order(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<CreateIconOrderRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    // Honeypot: bots that fill this hidden field get a fake success instead
    // of a signal that they were detected.
    if payload
        .website
        .as_deref()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
    {
        return Ok((
            StatusCode::CREATED,
            Json(CreateIconOrderResponse {
                order_number: String::new(),
            }),
        ));
    }

    let customer_name = payload.customer_name.trim().to_string();
    let contact_value = payload.contact_value.trim().to_string();
    if customer_name.is_empty() || contact_value.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    if payload.contact_method != "phone" && payload.contact_method != "email" {
        return Err(StatusCode::BAD_REQUEST);
    }
    if payload.contact_method == "email" && !(contact_value.contains('@') && contact_value.contains('.'))
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    if !payload.consent_given {
        return Err(StatusCode::BAD_REQUEST);
    }

    let icon = get_public_icon_row(&pool, payload.icon_id, false)
        .await?
        .ok_or(StatusCode::BAD_REQUEST)?;
    if !icon.order_enabled {
        return Err(StatusCode::BAD_REQUEST);
    }

    let option_ids: Vec<Uuid> = payload.items.iter().map(|item| item.option_id).collect();
    let options: Vec<ChurchIconOrderOptionDto> = if option_ids.is_empty() {
        Vec::new()
    } else {
        sqlx::query_as(&format!(
            "SELECT {OPTION_COLUMNS} FROM icon_order_options WHERE id = ANY($1) AND is_active = true"
        ))
        .bind(&option_ids)
        .fetch_all(&pool)
        .await
        .map_err(db_error)?
    };

    let icon_price = icon.price_cents.unwrap_or(0);
    let mut options_total: i64 = 0;
    let mut resolved_items: Vec<(Uuid, String, i64, i32)> = Vec::new();
    for item in &payload.items {
        let Some(option) = options.iter().find(|option| option.id == item.option_id) else {
            continue;
        };
        let quantity = item.quantity.unwrap_or(1).max(1);
        options_total += option.price_cents * quantity as i64;
        resolved_items.push((option.id, option_label(option), option.price_cents, quantity));
    }

    let total_price_cents = icon_price + options_total;
    let currency = if icon.currency.trim().is_empty() {
        "UAH".to_string()
    } else {
        icon.currency.clone()
    };
    let client_ip = extract_ip_from_headers(&headers);

    let mut tx = pool.begin().await.map_err(db_error)?;

    let (order_id, order_number): (Uuid, String) = sqlx::query_as(
        r#"INSERT INTO icon_orders
           (site_id, order_number, icon_id, icon_title_snapshot, icon_slug_snapshot,
            customer_name, contact_method, contact_value, preferred_contact_channel,
            country, city, consecration_requested, comment, consent_given,
            total_price_cents, currency, client_ip)
           VALUES ($1, 'IK-' || LPAD(nextval('icon_order_number_seq')::text, 6, '0'), $2, $3, $4,
                   $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
           RETURNING id, order_number"#,
    )
    .bind(CHURCH_SITE_ID)
    .bind(icon.id)
    .bind(&icon.title)
    .bind(&icon.slug)
    .bind(&customer_name)
    .bind(&payload.contact_method)
    .bind(&contact_value)
    .bind(payload.preferred_contact_channel.unwrap_or_default())
    .bind(payload.country.unwrap_or_default())
    .bind(payload.city.unwrap_or_default())
    .bind(payload.consecration_requested.unwrap_or(false))
    .bind(payload.comment.unwrap_or_default())
    .bind(payload.consent_given)
    .bind(total_price_cents)
    .bind(&currency)
    .bind(client_ip)
    .fetch_one(&mut *tx)
    .await
    .map_err(db_error)?;

    for (option_id, name_snapshot, price_snapshot, quantity) in resolved_items {
        sqlx::query(
            r#"INSERT INTO icon_order_items
               (order_id, option_id, option_name_snapshot, price_cents_snapshot, quantity)
               VALUES ($1, $2, $3, $4, $5)"#,
        )
        .bind(order_id)
        .bind(option_id)
        .bind(name_snapshot)
        .bind(price_snapshot)
        .bind(quantity)
        .execute(&mut *tx)
        .await
        .map_err(db_error)?;
    }

    tx.commit().await.map_err(db_error)?;

    Ok((StatusCode::CREATED, Json(CreateIconOrderResponse { order_number })))
}
