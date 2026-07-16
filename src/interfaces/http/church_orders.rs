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
    ChurchContentQuery, ChurchTranslationRef,
};
use super::site_context::CHURCH_SITE_ID;

/// Unique-violation-aware variant of [`db_error`] for inserts/updates whose
/// slug must be unique per site: surfaces a 409 instead of a generic 500 so
/// the admin form can show "this slug is already taken" rather than a crash.
fn conflict_or_db_error(error: sqlx::Error) -> StatusCode {
    if let sqlx::Error::Database(db_err) = &error {
        if db_err.code().as_deref() == Some("23505") {
            return StatusCode::CONFLICT;
        }
    }
    db_error(error)
}

/// Resolves the update-time sentinel convention used for nullable FK-ish
/// fields sent as `Option<String>`: omitted = don't touch, "" = clear,
/// non-empty = parse as the new UUID.
fn resolve_uuid_sentinel(payload_value: Option<String>, current: Option<Uuid>) -> Option<Uuid> {
    match payload_value {
        None => current,
        Some(value) if value.trim().is_empty() => None,
        Some(value) => Uuid::parse_str(value.trim()).ok().or(current),
    }
}

const STOCK_STATUSES: [&str; 3] = ["available", "made_to_order", "unavailable"];

fn validate_stock_status(value: &str) -> Result<(), StatusCode> {
    if STOCK_STATUSES.contains(&value) {
        Ok(())
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}

async fn linked_icon_group_exists(
    pool: &PgPool,
    site_id: Uuid,
    group_id: Uuid,
) -> Result<bool, StatusCode> {
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM church_icons WHERE translation_group_id = $1 AND (site_id = $2 OR is_global = true))",
    )
    .bind(group_id)
    .bind(site_id)
    .fetch_one(pool)
    .await
    .map_err(db_error)
}

async fn fetch_linked_icon_translations(
    pool: &PgPool,
    group_id: Uuid,
) -> Result<LinkedIconRef, StatusCode> {
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        r#"SELECT language, slug, title FROM church_icons
           WHERE translation_group_id = $1 AND (site_id = $2 OR is_global = true) AND status = 'published'
           ORDER BY language"#,
    )
    .bind(group_id)
    .bind(CHURCH_SITE_ID)
    .fetch_all(pool)
    .await
    .map_err(db_error)?;

    let translations = rows
        .into_iter()
        .map(|(language, slug, title)| ChurchTranslationRef {
            language,
            slug,
            title,
        })
        .collect();

    Ok(LinkedIconRef {
        translation_group_id: group_id,
        translations,
    })
}

// ── Icon product categories ─────────────────────────────────────────────────

const CATEGORY_COLUMNS: &str = "id, site_id, slug, name_uk, name_ru, name_en, description_uk, description_ru, description_en, image_url, is_active, sort_order, created_at::text AS created_at, updated_at::text AS updated_at";

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ChurchIconProductCategoryDto {
    pub id: Uuid,
    pub site_id: Uuid,
    pub slug: String,
    pub name_uk: String,
    pub name_ru: String,
    pub name_en: String,
    pub description_uk: String,
    pub description_ru: String,
    pub description_en: String,
    pub image_url: String,
    pub is_active: bool,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChurchIconProductCategoryPayload {
    pub slug: Option<String>,
    pub name_uk: Option<String>,
    pub name_ru: Option<String>,
    pub name_en: Option<String>,
    pub description_uk: Option<String>,
    pub description_ru: Option<String>,
    pub description_en: Option<String>,
    pub image_url: Option<String>,
    pub is_active: Option<bool>,
    pub sort_order: Option<i32>,
}

pub async fn list_icon_product_categories(
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let rows: Vec<ChurchIconProductCategoryDto> = sqlx::query_as(&format!(
        "SELECT {CATEGORY_COLUMNS} FROM icon_product_categories WHERE site_id = $1 ORDER BY sort_order ASC, name_uk ASC"
    ))
    .bind(query.site_id())
    .fetch_all(&pool)
    .await
    .map_err(db_error)?;

    Ok(Json(rows))
}

pub async fn get_icon_product_category(
    Path(id): Path<Uuid>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let row: ChurchIconProductCategoryDto = sqlx::query_as(&format!(
        "SELECT {CATEGORY_COLUMNS} FROM icon_product_categories WHERE id = $1 AND site_id = $2"
    ))
    .bind(id)
    .bind(query.site_id())
    .fetch_optional(&pool)
    .await
    .map_err(db_error)?
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(row))
}

pub async fn create_icon_product_category(
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
    Json(payload): Json<ChurchIconProductCategoryPayload>,
) -> Result<impl IntoResponse, StatusCode> {
    let name_uk = required(payload.name_uk, "nameUk")?;
    let slug = optional_non_empty(payload.slug).unwrap_or_else(|| slugify(&name_uk));

    let row: ChurchIconProductCategoryDto = sqlx::query_as(&format!(
        r#"INSERT INTO icon_product_categories
           (site_id, slug, name_uk, name_ru, name_en, description_uk, description_ru, description_en, image_url, is_active, sort_order)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
           RETURNING {CATEGORY_COLUMNS}"#
    ))
    .bind(query.site_id())
    .bind(slug)
    .bind(name_uk)
    .bind(payload.name_ru.unwrap_or_default())
    .bind(payload.name_en.unwrap_or_default())
    .bind(payload.description_uk.unwrap_or_default())
    .bind(payload.description_ru.unwrap_or_default())
    .bind(payload.description_en.unwrap_or_default())
    .bind(payload.image_url.unwrap_or_default())
    .bind(payload.is_active.unwrap_or(true))
    .bind(payload.sort_order.unwrap_or(0))
    .fetch_one(&pool)
    .await
    .map_err(db_error)?;

    Ok((StatusCode::CREATED, Json(row)))
}

pub async fn update_icon_product_category(
    Path(id): Path<Uuid>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
    Json(payload): Json<ChurchIconProductCategoryPayload>,
) -> Result<impl IntoResponse, StatusCode> {
    let site_id = query.site_id();
    let current: ChurchIconProductCategoryDto = sqlx::query_as(&format!(
        "SELECT {CATEGORY_COLUMNS} FROM icon_product_categories WHERE id = $1 AND site_id = $2"
    ))
    .bind(id)
    .bind(site_id)
    .fetch_optional(&pool)
    .await
    .map_err(db_error)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let row: ChurchIconProductCategoryDto = sqlx::query_as(&format!(
        r#"UPDATE icon_product_categories SET
              slug = $1, name_uk = $2, name_ru = $3, name_en = $4,
              description_uk = $5, description_ru = $6, description_en = $7,
              image_url = $8, is_active = $9, sort_order = $10
           WHERE id = $11 AND site_id = $12
           RETURNING {CATEGORY_COLUMNS}"#
    ))
    .bind(optional_non_empty(payload.slug).unwrap_or(current.slug))
    .bind(optional_non_empty(payload.name_uk).unwrap_or(current.name_uk))
    .bind(payload.name_ru.unwrap_or(current.name_ru))
    .bind(payload.name_en.unwrap_or(current.name_en))
    .bind(payload.description_uk.unwrap_or(current.description_uk))
    .bind(payload.description_ru.unwrap_or(current.description_ru))
    .bind(payload.description_en.unwrap_or(current.description_en))
    .bind(payload.image_url.unwrap_or(current.image_url))
    .bind(payload.is_active.unwrap_or(current.is_active))
    .bind(payload.sort_order.unwrap_or(current.sort_order))
    .bind(id)
    .bind(site_id)
    .fetch_one(&pool)
    .await
    .map_err(db_error)?;

    Ok(Json(row))
}

pub async fn delete_icon_product_category(
    Path(id): Path<Uuid>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    delete_owned(&pool, "icon_product_categories", id, query.site_id()).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn public_icon_product_categories_list(
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let rows: Vec<ChurchIconProductCategoryDto> = sqlx::query_as(&format!(
        "SELECT {CATEGORY_COLUMNS} FROM icon_product_categories WHERE site_id = $1 AND is_active = true ORDER BY sort_order ASC"
    ))
    .bind(CHURCH_SITE_ID)
    .fetch_all(&pool)
    .await
    .map_err(db_error)?;

    Ok(Json(rows))
}

// ── Product catalog ──────────────────────────────────────────────────────────
// Physically still the `icon_order_options` table (avoids breaking the
// `icon_order_items.option_id` FK and existing orders); the Rust/JSON/route
// surface uses "product" naming per the catalog rework.

const PRODUCT_COLUMNS: &str = "id, site_id, slug, name_uk, name_ru, name_en, description, category_id, \
    linked_icon_translation_group_id, full_description_uk, full_description_ru, full_description_en, \
    gallery_urls, photo_url, price_cents, currency, production_time, consecration_available, stock_status, \
    featured, seo_title_uk, seo_title_ru, seo_title_en, seo_description_uk, seo_description_ru, seo_description_en, \
    is_active, sort_order, created_at::text AS created_at, updated_at::text AS updated_at";

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ChurchProductDto {
    pub id: Uuid,
    pub site_id: Uuid,
    pub slug: String,
    pub name_uk: String,
    pub name_ru: String,
    pub name_en: String,
    pub description: String,
    pub category_id: Option<Uuid>,
    pub linked_icon_translation_group_id: Option<Uuid>,
    pub full_description_uk: String,
    pub full_description_ru: String,
    pub full_description_en: String,
    pub gallery_urls: Vec<String>,
    pub photo_url: String,
    pub price_cents: i64,
    pub currency: String,
    pub production_time: String,
    pub consecration_available: bool,
    pub stock_status: String,
    pub featured: bool,
    pub seo_title_uk: String,
    pub seo_title_ru: String,
    pub seo_title_en: String,
    pub seo_description_uk: String,
    pub seo_description_ru: String,
    pub seo_description_en: String,
    pub is_active: bool,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChurchProductPayload {
    pub slug: Option<String>,
    pub name_uk: Option<String>,
    pub name_ru: Option<String>,
    pub name_en: Option<String>,
    pub description: Option<String>,
    /// Sentinel convention (see [`resolve_uuid_sentinel`]): omitted = don't
    /// touch, "" = clear to uncategorized/unlinked, non-empty = new id.
    pub category_id: Option<String>,
    pub linked_icon_translation_group_id: Option<String>,
    pub full_description_uk: Option<String>,
    pub full_description_ru: Option<String>,
    pub full_description_en: Option<String>,
    pub gallery_urls: Option<Vec<String>>,
    pub photo_url: Option<String>,
    pub price_cents: Option<i64>,
    pub currency: Option<String>,
    pub production_time: Option<String>,
    pub consecration_available: Option<bool>,
    pub stock_status: Option<String>,
    pub featured: Option<bool>,
    pub seo_title_uk: Option<String>,
    pub seo_title_ru: Option<String>,
    pub seo_title_en: Option<String>,
    pub seo_description_uk: Option<String>,
    pub seo_description_ru: Option<String>,
    pub seo_description_en: Option<String>,
    pub is_active: Option<bool>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkedIconRef {
    pub translation_group_id: Uuid,
    pub translations: Vec<ChurchTranslationRef>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicProductPage {
    pub product: ChurchProductDto,
    pub linked_icon: Option<LinkedIconRef>,
    pub related: Vec<ChurchProductDto>,
}

#[derive(Debug, Deserialize)]
pub struct PublicProductQuery {
    #[allow(dead_code)]
    pub locale: Option<String>,
    pub category: Option<String>,
    pub search: Option<String>,
    pub featured: Option<bool>,
    pub linked_icon_group_id: Option<Uuid>,
}

pub async fn list_products(
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let rows: Vec<ChurchProductDto> = sqlx::query_as(&format!(
        "SELECT {PRODUCT_COLUMNS} FROM icon_order_options WHERE site_id = $1 ORDER BY sort_order ASC, name_uk ASC"
    ))
    .bind(query.site_id())
    .fetch_all(&pool)
    .await
    .map_err(db_error)?;

    Ok(Json(rows))
}

pub async fn get_product(
    Path(id): Path<Uuid>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let row: ChurchProductDto = sqlx::query_as(&format!(
        "SELECT {PRODUCT_COLUMNS} FROM icon_order_options WHERE id = $1 AND site_id = $2"
    ))
    .bind(id)
    .bind(query.site_id())
    .fetch_optional(&pool)
    .await
    .map_err(db_error)?
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(row))
}

pub async fn create_product(
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
    Json(payload): Json<ChurchProductPayload>,
) -> Result<impl IntoResponse, StatusCode> {
    let site_id = query.site_id();
    let name_uk = required(payload.name_uk, "nameUk")?;
    let slug = optional_non_empty(payload.slug).unwrap_or_else(|| slugify(&name_uk));

    if payload.price_cents.unwrap_or(0) < 0 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let stock_status = payload.stock_status.unwrap_or_else(|| "available".into());
    validate_stock_status(&stock_status)?;

    let linked_icon_group = resolve_uuid_sentinel(payload.linked_icon_translation_group_id, None);
    if let Some(group_id) = linked_icon_group {
        if !linked_icon_group_exists(&pool, site_id, group_id).await? {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    let row: ChurchProductDto = sqlx::query_as(&format!(
        r#"INSERT INTO icon_order_options
           (site_id, slug, name_uk, name_ru, name_en, description, category_id,
            linked_icon_translation_group_id, full_description_uk, full_description_ru, full_description_en,
            gallery_urls, photo_url, price_cents, currency, production_time, consecration_available,
            stock_status, featured, seo_title_uk, seo_title_ru, seo_title_en,
            seo_description_uk, seo_description_ru, seo_description_en, is_active, sort_order)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17,
                   $18, $19, $20, $21, $22, $23, $24, $25, $26, $27)
           RETURNING {PRODUCT_COLUMNS}"#
    ))
    .bind(site_id)
    .bind(slug)
    .bind(name_uk)
    .bind(payload.name_ru.unwrap_or_default())
    .bind(payload.name_en.unwrap_or_default())
    .bind(payload.description.unwrap_or_default())
    .bind(resolve_uuid_sentinel(payload.category_id, None))
    .bind(linked_icon_group)
    .bind(payload.full_description_uk.unwrap_or_default())
    .bind(payload.full_description_ru.unwrap_or_default())
    .bind(payload.full_description_en.unwrap_or_default())
    .bind(payload.gallery_urls.unwrap_or_default())
    .bind(payload.photo_url.unwrap_or_default())
    .bind(payload.price_cents.unwrap_or(0))
    .bind(payload.currency.unwrap_or_else(|| "UAH".into()))
    .bind(payload.production_time.unwrap_or_default())
    .bind(payload.consecration_available.unwrap_or(false))
    .bind(stock_status)
    .bind(payload.featured.unwrap_or(false))
    .bind(payload.seo_title_uk.unwrap_or_default())
    .bind(payload.seo_title_ru.unwrap_or_default())
    .bind(payload.seo_title_en.unwrap_or_default())
    .bind(payload.seo_description_uk.unwrap_or_default())
    .bind(payload.seo_description_ru.unwrap_or_default())
    .bind(payload.seo_description_en.unwrap_or_default())
    .bind(payload.is_active.unwrap_or(true))
    .bind(payload.sort_order.unwrap_or(0))
    .fetch_one(&pool)
    .await
    .map_err(conflict_or_db_error)?;

    Ok((StatusCode::CREATED, Json(row)))
}

pub async fn update_product(
    Path(id): Path<Uuid>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
    Json(payload): Json<ChurchProductPayload>,
) -> Result<impl IntoResponse, StatusCode> {
    let site_id = query.site_id();
    let current: ChurchProductDto = sqlx::query_as(&format!(
        "SELECT {PRODUCT_COLUMNS} FROM icon_order_options WHERE id = $1 AND site_id = $2"
    ))
    .bind(id)
    .bind(site_id)
    .fetch_optional(&pool)
    .await
    .map_err(db_error)?
    .ok_or(StatusCode::NOT_FOUND)?;

    if let Some(price) = payload.price_cents {
        if price < 0 {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    let stock_status = payload.stock_status.clone().unwrap_or(current.stock_status.clone());
    validate_stock_status(&stock_status)?;

    let linked_icon_group =
        resolve_uuid_sentinel(payload.linked_icon_translation_group_id, current.linked_icon_translation_group_id);
    if let Some(group_id) = linked_icon_group {
        if linked_icon_group != current.linked_icon_translation_group_id
            && !linked_icon_group_exists(&pool, site_id, group_id).await?
        {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    let row: ChurchProductDto = sqlx::query_as(&format!(
        r#"UPDATE icon_order_options SET
              slug = $1, name_uk = $2, name_ru = $3, name_en = $4, description = $5, category_id = $6,
              linked_icon_translation_group_id = $7, full_description_uk = $8, full_description_ru = $9,
              full_description_en = $10, gallery_urls = $11, photo_url = $12, price_cents = $13,
              currency = $14, production_time = $15, consecration_available = $16, stock_status = $17,
              featured = $18, seo_title_uk = $19, seo_title_ru = $20, seo_title_en = $21,
              seo_description_uk = $22, seo_description_ru = $23, seo_description_en = $24,
              is_active = $25, sort_order = $26
           WHERE id = $27 AND site_id = $28
           RETURNING {PRODUCT_COLUMNS}"#
    ))
    .bind(optional_non_empty(payload.slug).unwrap_or(current.slug))
    .bind(optional_non_empty(payload.name_uk).unwrap_or(current.name_uk))
    .bind(payload.name_ru.unwrap_or(current.name_ru))
    .bind(payload.name_en.unwrap_or(current.name_en))
    .bind(payload.description.unwrap_or(current.description))
    .bind(resolve_uuid_sentinel(payload.category_id, current.category_id))
    .bind(linked_icon_group)
    .bind(payload.full_description_uk.unwrap_or(current.full_description_uk))
    .bind(payload.full_description_ru.unwrap_or(current.full_description_ru))
    .bind(payload.full_description_en.unwrap_or(current.full_description_en))
    .bind(payload.gallery_urls.unwrap_or(current.gallery_urls))
    .bind(payload.photo_url.unwrap_or(current.photo_url))
    .bind(payload.price_cents.unwrap_or(current.price_cents))
    .bind(payload.currency.unwrap_or(current.currency))
    .bind(payload.production_time.unwrap_or(current.production_time))
    .bind(payload.consecration_available.unwrap_or(current.consecration_available))
    .bind(stock_status)
    .bind(payload.featured.unwrap_or(current.featured))
    .bind(payload.seo_title_uk.unwrap_or(current.seo_title_uk))
    .bind(payload.seo_title_ru.unwrap_or(current.seo_title_ru))
    .bind(payload.seo_title_en.unwrap_or(current.seo_title_en))
    .bind(payload.seo_description_uk.unwrap_or(current.seo_description_uk))
    .bind(payload.seo_description_ru.unwrap_or(current.seo_description_ru))
    .bind(payload.seo_description_en.unwrap_or(current.seo_description_en))
    .bind(payload.is_active.unwrap_or(current.is_active))
    .bind(payload.sort_order.unwrap_or(current.sort_order))
    .bind(id)
    .bind(site_id)
    .fetch_one(&pool)
    .await
    .map_err(conflict_or_db_error)?;

    Ok(Json(row))
}

pub async fn delete_product(
    Path(id): Path<Uuid>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    delete_owned(&pool, "icon_order_options", id, query.site_id()).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn public_products_list(
    Query(query): Query<PublicProductQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let search_pattern = query
        .search
        .as_ref()
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty())
        .map(|value| format!("%{value}%"));
    let category = query.category.filter(|value| !value.trim().is_empty());

    let rows: Vec<ChurchProductDto> = sqlx::query_as(&format!(
        r#"SELECT {PRODUCT_COLUMNS} FROM icon_order_options o
           WHERE o.site_id = $1 AND o.is_active = true
             AND ($2::text IS NULL OR o.category_id IN (
                 SELECT id FROM icon_product_categories WHERE site_id = $1 AND slug = $2
             ))
             AND ($3::text IS NULL OR lower(o.name_uk) LIKE $3 OR lower(o.name_ru) LIKE $3
                  OR lower(o.name_en) LIKE $3 OR lower(o.slug) LIKE $3)
             AND ($4::bool IS NULL OR o.featured = $4)
             AND ($5::uuid IS NULL OR o.linked_icon_translation_group_id = $5)
           ORDER BY o.sort_order ASC, o.name_uk ASC"#
    ))
    .bind(CHURCH_SITE_ID)
    .bind(category)
    .bind(search_pattern)
    .bind(query.featured)
    .bind(query.linked_icon_group_id)
    .fetch_all(&pool)
    .await
    .map_err(db_error)?;

    Ok(Json(rows))
}

pub async fn public_product_by_slug(
    Path(slug): Path<String>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let product: ChurchProductDto = sqlx::query_as(&format!(
        "SELECT {PRODUCT_COLUMNS} FROM icon_order_options WHERE slug = $1 AND site_id = $2 AND is_active = true"
    ))
    .bind(&slug)
    .bind(CHURCH_SITE_ID)
    .fetch_optional(&pool)
    .await
    .map_err(db_error)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let linked_icon = match product.linked_icon_translation_group_id {
        Some(group_id) => Some(fetch_linked_icon_translations(&pool, group_id).await?),
        None => None,
    };

    let related: Vec<ChurchProductDto> = sqlx::query_as(&format!(
        r#"SELECT {PRODUCT_COLUMNS} FROM icon_order_options
           WHERE site_id = $1 AND is_active = true AND id <> $2
             AND category_id IS NOT DISTINCT FROM $3
           ORDER BY sort_order ASC LIMIT 4"#
    ))
    .bind(CHURCH_SITE_ID)
    .bind(product.id)
    .bind(product.category_id)
    .fetch_all(&pool)
    .await
    .map_err(db_error)?;

    Ok(Json(PublicProductPage {
        product,
        linked_icon,
        related,
    }))
}

// ── Icon orders ─────────────────────────────────────────────────────────────

const ORDER_COLUMNS: &str = "id, site_id, is_global, order_number, icon_id, icon_title_snapshot, icon_slug_snapshot, \
    primary_product_id, primary_product_name_snapshot, primary_product_slug_snapshot, \
    primary_product_price_cents_snapshot, primary_product_photo_snapshot, \
    customer_name, contact_method, contact_value, preferred_contact_channel, country, city, consecration_requested, comment, consent_given, status, admin_note, total_price_cents, currency, is_read, created_at::text AS created_at, updated_at::text AS updated_at";
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
    pub primary_product_id: Option<Uuid>,
    pub primary_product_name_snapshot: String,
    pub primary_product_slug_snapshot: String,
    pub primary_product_price_cents_snapshot: i64,
    pub primary_product_photo_snapshot: String,
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

fn product_label(product: &ChurchProductDto) -> String {
    if !product.name_uk.trim().is_empty() {
        product.name_uk.clone()
    } else if !product.name_ru.trim().is_empty() {
        product.name_ru.clone()
    } else {
        product.name_en.clone()
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
    let options: Vec<ChurchProductDto> = if option_ids.is_empty() {
        Vec::new()
    } else {
        sqlx::query_as(&format!(
            "SELECT {PRODUCT_COLUMNS} FROM icon_order_options WHERE id = ANY($1) AND is_active = true"
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
        resolved_items.push((option.id, product_label(option), option.price_cents, quantity));
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

// ── Public: submit a product order (any catalog product, icon-linked or not) ─

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProductOrderItemInput {
    pub product_id: Uuid,
    pub quantity: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProductOrderRequest {
    pub product_slug: String,
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
    pub items: Vec<CreateProductOrderItemInput>,
    /// Honeypot: real visitors never see or fill this field.
    pub website: Option<String>,
}

pub async fn public_create_product_order(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<CreateProductOrderRequest>,
) -> Result<impl IntoResponse, StatusCode> {
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

    let product: ChurchProductDto = sqlx::query_as(&format!(
        "SELECT {PRODUCT_COLUMNS} FROM icon_order_options WHERE slug = $1 AND site_id = $2 AND is_active = true"
    ))
    .bind(payload.product_slug.trim())
    .bind(CHURCH_SITE_ID)
    .fetch_optional(&pool)
    .await
    .map_err(db_error)?
    .ok_or(StatusCode::BAD_REQUEST)?;

    let item_ids: Vec<Uuid> = payload.items.iter().map(|item| item.product_id).collect();
    let item_products: Vec<ChurchProductDto> = if item_ids.is_empty() {
        Vec::new()
    } else {
        sqlx::query_as(&format!(
            "SELECT {PRODUCT_COLUMNS} FROM icon_order_options WHERE id = ANY($1) AND is_active = true"
        ))
        .bind(&item_ids)
        .fetch_all(&pool)
        .await
        .map_err(db_error)?
    };

    let mut items_total: i64 = 0;
    let mut resolved_items: Vec<(Uuid, String, i64, i32)> = Vec::new();
    for item in &payload.items {
        let Some(found) = item_products.iter().find(|candidate| candidate.id == item.product_id) else {
            continue;
        };
        let quantity = item.quantity.unwrap_or(1).max(1);
        items_total += found.price_cents * quantity as i64;
        resolved_items.push((found.id, product_label(found), found.price_cents, quantity));
    }

    let total_price_cents = product.price_cents + items_total;
    let currency = if product.currency.trim().is_empty() {
        "UAH".to_string()
    } else {
        product.currency.clone()
    };
    let client_ip = extract_ip_from_headers(&headers);

    // Best-effort: if the product links back to an icon, snapshot that icon
    // too so the admin order list keeps showing "which icon" for continuity
    // with icon-only orders created through the older endpoint.
    let (icon_id, icon_title_snapshot, icon_slug_snapshot) = match product.linked_icon_translation_group_id {
        Some(group_id) => {
            let icon: Option<(Uuid, String, String)> = sqlx::query_as(
                r#"SELECT id, title, slug FROM church_icons
                   WHERE translation_group_id = $1 AND (site_id = $2 OR is_global = true) AND status = 'published'
                   ORDER BY CASE WHEN language = 'uk' THEN 0 ELSE 1 END LIMIT 1"#,
            )
            .bind(group_id)
            .bind(CHURCH_SITE_ID)
            .fetch_optional(&pool)
            .await
            .map_err(db_error)?;
            match icon {
                Some((id, title, slug)) => (Some(id), title, slug),
                None => (None, String::new(), String::new()),
            }
        }
        None => (None, String::new(), String::new()),
    };

    let mut tx = pool.begin().await.map_err(db_error)?;

    let (order_id, order_number): (Uuid, String) = sqlx::query_as(
        r#"INSERT INTO icon_orders
           (site_id, order_number, icon_id, icon_title_snapshot, icon_slug_snapshot,
            primary_product_id, primary_product_name_snapshot, primary_product_slug_snapshot,
            primary_product_price_cents_snapshot, primary_product_photo_snapshot,
            customer_name, contact_method, contact_value, preferred_contact_channel,
            country, city, consecration_requested, comment, consent_given,
            total_price_cents, currency, client_ip)
           VALUES ($1, 'IK-' || LPAD(nextval('icon_order_number_seq')::text, 6, '0'), $2, $3, $4,
                   $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21)
           RETURNING id, order_number"#,
    )
    .bind(CHURCH_SITE_ID)
    .bind(icon_id)
    .bind(&icon_title_snapshot)
    .bind(&icon_slug_snapshot)
    .bind(product.id)
    .bind(product_label(&product))
    .bind(&product.slug)
    .bind(product.price_cents)
    .bind(&product.photo_url)
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
