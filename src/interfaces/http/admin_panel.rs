use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use rust_decimal::{prelude::ToPrimitive, Decimal};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{PgPool, Row};
use std::collections::BTreeMap;
use uuid::Uuid;

use super::site_context::{
    canonical_site_key, resolve_site_id, SiteQuery, CONSTRUCTION_SITE_ID, KITCHEN_SITE_ID,
};
use crate::{domain::AdminClaims, shared::AppError};

type LocalizedText = BTreeMap<String, String>;

fn default_text(value: &str) -> LocalizedText {
    ["ru", "pl", "en", "kk"]
        .into_iter()
        .map(|lang| (lang.to_string(), value.to_string()))
        .collect()
}

fn localized_from_value(value: Option<Value>, fallback: &str) -> LocalizedText {
    let mut result = default_text(fallback);
    if let Some(Value::Object(map)) = value {
        for lang in ["ru", "pl", "en", "kk"] {
            if let Some(text) = map.get(lang).and_then(Value::as_str) {
                result.insert(lang.to_string(), text.to_string());
            }
        }
    }
    result
}

fn dec(value: Option<Decimal>) -> Option<f64> {
    value.and_then(|item| item.to_f64())
}

fn dec_or_zero(value: Option<Decimal>) -> f64 {
    dec(value).unwrap_or(0.0)
}

fn slugify(value: &str) -> String {
    let slug = value
        .trim()
        .to_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        format!("item-{}", Uuid::new_v4())
    } else {
        slug
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AffiliateOfferDto {
    pub id: String,
    pub product_id: String,
    pub network: String,
    pub merchant: String,
    pub affiliate_url: String,
    pub price: Option<f64>,
    pub currency: String,
    pub commission_percent: Option<f64>,
    pub cookie_days: Option<i32>,
    pub is_active: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AffiliateProductDto {
    pub id: String,
    pub site: String,
    pub title: LocalizedText,
    pub slug: String,
    pub category: String,
    pub network: String,
    pub merchant: String,
    pub affiliate_url: String,
    pub image_url: Option<String>,
    pub detail_image_url: Option<String>,
    pub price: Option<f64>,
    pub currency: String,
    pub commission_percent: Option<f64>,
    pub cookie_days: Option<i32>,
    pub status: String,
    pub languages: Vec<String>,
    pub seo_title: Option<LocalizedText>,
    pub seo_description: Option<LocalizedText>,
    pub offers: Vec<AffiliateOfferDto>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AffiliateProductPayload {
    pub site: Option<String>,
    pub title: Option<LocalizedText>,
    pub slug: Option<String>,
    pub category: Option<String>,
    pub network: Option<String>,
    pub merchant: Option<String>,
    pub affiliate_url: Option<String>,
    pub image_url: Option<String>,
    pub detail_image_url: Option<String>,
    pub price: Option<f64>,
    pub currency: Option<String>,
    pub commission_percent: Option<f64>,
    pub cookie_days: Option<i32>,
    pub status: Option<String>,
    pub languages: Option<Vec<String>>,
    pub seo_title: Option<LocalizedText>,
    pub seo_description: Option<LocalizedText>,
}

fn affiliate_from_row(
    row: sqlx::postgres::PgRow,
    offers: Vec<AffiliateOfferDto>,
) -> AffiliateProductDto {
    let fallback_title = row.try_get::<String, _>("slug").unwrap_or_default();
    let seo_title = row
        .try_get::<Option<Value>, _>("seo_title")
        .ok()
        .flatten()
        .map(|value| localized_from_value(Some(value), ""));
    let seo_description = row
        .try_get::<Option<Value>, _>("seo_description")
        .ok()
        .flatten()
        .map(|value| localized_from_value(Some(value), ""));

    AffiliateProductDto {
        id: row.try_get::<Uuid, _>("id").unwrap().to_string(),
        site: row
            .try_get("site")
            .unwrap_or_else(|_| "kitchen".to_string()),
        title: localized_from_value(row.try_get("title").ok(), &fallback_title),
        slug: fallback_title,
        category: row.try_get("category").unwrap_or_default(),
        network: row
            .try_get("network")
            .unwrap_or_else(|_| "custom".to_string()),
        merchant: row.try_get("merchant").unwrap_or_default(),
        affiliate_url: row.try_get("affiliate_url").unwrap_or_default(),
        image_url: row.try_get("image_url").ok().flatten(),
        detail_image_url: row.try_get("detail_image_url").ok().flatten(),
        price: dec(row.try_get("price").ok().flatten()),
        currency: row
            .try_get("currency")
            .unwrap_or_else(|_| "PLN".to_string()),
        commission_percent: dec(row.try_get("commission_percent").ok().flatten()),
        cookie_days: row.try_get("cookie_days").ok().flatten(),
        status: row
            .try_get("status")
            .unwrap_or_else(|_| "draft".to_string()),
        languages: row
            .try_get("languages")
            .unwrap_or_else(|_| vec!["ru".to_string()]),
        seo_title,
        seo_description,
        offers,
        created_at: row.try_get("created_at").unwrap_or_default(),
        updated_at: row.try_get("updated_at").unwrap_or_default(),
    }
}

fn offer_from_row(row: sqlx::postgres::PgRow) -> AffiliateOfferDto {
    AffiliateOfferDto {
        id: row.try_get::<Uuid, _>("id").unwrap().to_string(),
        product_id: row.try_get::<Uuid, _>("product_id").unwrap().to_string(),
        network: row
            .try_get("network")
            .unwrap_or_else(|_| "custom".to_string()),
        merchant: row.try_get("merchant").unwrap_or_default(),
        affiliate_url: row.try_get("affiliate_url").unwrap_or_default(),
        price: dec(row.try_get("price").ok().flatten()),
        currency: row
            .try_get("currency")
            .unwrap_or_else(|_| "PLN".to_string()),
        commission_percent: dec(row.try_get("commission_percent").ok().flatten()),
        cookie_days: row.try_get("cookie_days").ok().flatten(),
        is_active: row.try_get("is_active").unwrap_or(true),
    }
}

async fn offers_for(pool: &PgPool, product_id: Uuid) -> Result<Vec<AffiliateOfferDto>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT id, product_id, network, merchant, affiliate_url, price, currency,
               commission_percent, cookie_days, is_active
        FROM admin_affiliate_offers
        WHERE product_id = $1
        ORDER BY is_active DESC, updated_at DESC
        "#,
    )
    .bind(product_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(offer_from_row).collect())
}

pub async fn list_affiliate_products(
    _claims: AdminClaims,
    Query(query): Query<SiteQuery>,
    State(pool): State<PgPool>,
) -> Result<Json<Vec<AffiliateProductDto>>, AppError> {
    let rows = if query.site.as_deref() == Some("all") && query.site_id.is_none() {
        sqlx::query(
            r#"
            SELECT id, site, title, slug, category, network, merchant, affiliate_url, image_url,
                   detail_image_url, price, currency, commission_percent, cookie_days, status,
                   languages, seo_title, seo_description,
                   to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at,
                   to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS updated_at
            FROM admin_affiliate_products
            ORDER BY updated_at DESC
            "#,
        )
        .fetch_all(&pool)
        .await?
    } else {
        let site_id = resolve_site_id(&query, KITCHEN_SITE_ID);
        sqlx::query(
            r#"
            SELECT id, site, title, slug, category, network, merchant, affiliate_url, image_url,
                   detail_image_url, price, currency, commission_percent, cookie_days, status,
                   languages, seo_title, seo_description,
                   to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at,
                   to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS updated_at
            FROM admin_affiliate_products
            WHERE site_id = $1 OR is_global = true
            ORDER BY updated_at DESC
            "#,
        )
        .bind(site_id)
        .fetch_all(&pool)
        .await?
    };

    let mut products = Vec::with_capacity(rows.len());
    for row in rows {
        let product_id = row.try_get::<Uuid, _>("id")?;
        let offers = offers_for(&pool, product_id).await?;
        products.push(affiliate_from_row(row, offers));
    }
    Ok(Json(products))
}

pub async fn get_affiliate_product(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    Query(query): Query<SiteQuery>,
    State(pool): State<PgPool>,
) -> Result<Json<AffiliateProductDto>, AppError> {
    let site_id = resolve_site_id(&query, KITCHEN_SITE_ID);
    let row = sqlx::query(
        r#"
        SELECT id, site, title, slug, category, network, merchant, affiliate_url, image_url,
               detail_image_url, price, currency, commission_percent, cookie_days, status,
               languages, seo_title, seo_description,
               to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at,
               to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS updated_at
        FROM admin_affiliate_products
        WHERE id = $1 AND (site_id = $2 OR is_global = true)
        "#,
    )
    .bind(id)
    .bind(site_id)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::not_found("Affiliate product not found"))?;
    let offers = offers_for(&pool, id).await?;
    Ok(Json(affiliate_from_row(row, offers)))
}

pub async fn create_affiliate_product(
    _claims: AdminClaims,
    Query(query): Query<SiteQuery>,
    State(pool): State<PgPool>,
    Json(payload): Json<AffiliateProductPayload>,
) -> Result<(StatusCode, Json<AffiliateProductDto>), AppError> {
    let site_id = resolve_site_id(&query, KITCHEN_SITE_ID);
    let site_key = canonical_site_key(site_id);
    let title = payload
        .title
        .unwrap_or_else(|| default_text("Новый партнерский товар"));
    let title_ru = title
        .get("ru")
        .cloned()
        .unwrap_or_else(|| "Новый партнерский товар".to_string());
    let slug = payload.slug.unwrap_or_else(|| slugify(&title_ru));
    let price = payload.price.and_then(Decimal::from_f64_retain);
    let commission = payload
        .commission_percent
        .and_then(Decimal::from_f64_retain);
    let row = sqlx::query(
        r#"
        INSERT INTO admin_affiliate_products (
            site_id, site, title, slug, category, network, merchant, affiliate_url, image_url,
            detail_image_url, price, currency, commission_percent, cookie_days, status,
            languages, seo_title, seo_description
        )
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18)
        RETURNING id, site, title, slug, category, network, merchant, affiliate_url, image_url,
                  detail_image_url, price, currency, commission_percent, cookie_days, status,
                  languages, seo_title, seo_description,
                  to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at,
                  to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS updated_at
        "#,
    )
    .bind(site_id)
    .bind(site_key)
    .bind(json!(title))
    .bind(slug)
    .bind(payload.category.unwrap_or_default())
    .bind(payload.network.unwrap_or_else(|| "custom".to_string()))
    .bind(payload.merchant.unwrap_or_default())
    .bind(payload.affiliate_url.unwrap_or_default())
    .bind(payload.image_url)
    .bind(payload.detail_image_url)
    .bind(price)
    .bind(payload.currency.unwrap_or_else(|| "PLN".to_string()))
    .bind(commission)
    .bind(payload.cookie_days)
    .bind(payload.status.unwrap_or_else(|| "draft".to_string()))
    .bind(payload.languages.unwrap_or_else(|| vec!["ru".to_string()]))
    .bind(payload.seo_title.map(|value| json!(value)))
    .bind(payload.seo_description.map(|value| json!(value)))
    .fetch_one(&pool)
    .await?;
    let id = row.try_get::<Uuid, _>("id")?;
    let offers = offers_for(&pool, id).await?;
    Ok((StatusCode::CREATED, Json(affiliate_from_row(row, offers))))
}

pub async fn update_affiliate_product(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    Query(query): Query<SiteQuery>,
    State(pool): State<PgPool>,
    Json(payload): Json<AffiliateProductPayload>,
) -> Result<Json<AffiliateProductDto>, AppError> {
    let site_id = resolve_site_id(&query, KITCHEN_SITE_ID);
    let site_key = canonical_site_key(site_id);
    let current = sqlx::query(
        r#"
        SELECT id, site, title, slug, category, network, merchant, affiliate_url, image_url,
               detail_image_url, price, currency, commission_percent, cookie_days, status,
               languages, seo_title, seo_description
        FROM admin_affiliate_products WHERE id = $1 AND site_id = $2
        "#,
    )
    .bind(id)
    .bind(site_id)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::not_found("Affiliate product not found"))?;

    let title = payload
        .title
        .map(|value| json!(value))
        .unwrap_or_else(|| current.try_get("title").unwrap_or_else(|_| json!({})));
    let price = payload
        .price
        .and_then(Decimal::from_f64_retain)
        .or_else(|| current.try_get("price").ok().flatten());
    let commission = payload
        .commission_percent
        .and_then(Decimal::from_f64_retain)
        .or_else(|| current.try_get("commission_percent").ok().flatten());

    let row = sqlx::query(
        r#"
        UPDATE admin_affiliate_products SET
            site = $2, title = $3, slug = $4, category = $5, network = $6, merchant = $7,
            affiliate_url = $8, image_url = $9, detail_image_url = $10, price = $11,
            currency = $12, commission_percent = $13, cookie_days = $14, status = $15,
            languages = $16, seo_title = $17, seo_description = $18, updated_at = NOW()
        WHERE id = $1 AND site_id = $19
        RETURNING id, site, title, slug, category, network, merchant, affiliate_url, image_url,
                  detail_image_url, price, currency, commission_percent, cookie_days, status,
                  languages, seo_title, seo_description,
                  to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at,
                  to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS updated_at
        "#,
    )
    .bind(id)
    .bind(site_key)
    .bind(title)
    .bind(payload.slug.unwrap_or_else(|| current.try_get("slug").unwrap_or_default()))
    .bind(payload.category.unwrap_or_else(|| current.try_get("category").unwrap_or_default()))
    .bind(payload.network.unwrap_or_else(|| current.try_get("network").unwrap_or_else(|_| "custom".to_string())))
    .bind(payload.merchant.unwrap_or_else(|| current.try_get("merchant").unwrap_or_default()))
    .bind(payload.affiliate_url.unwrap_or_else(|| current.try_get("affiliate_url").unwrap_or_default()))
    .bind(payload.image_url.or_else(|| current.try_get("image_url").ok().flatten()))
    .bind(payload.detail_image_url.or_else(|| current.try_get("detail_image_url").ok().flatten()))
    .bind(price)
    .bind(payload.currency.unwrap_or_else(|| current.try_get("currency").unwrap_or_else(|_| "PLN".to_string())))
    .bind(commission)
    .bind(payload.cookie_days.or_else(|| current.try_get("cookie_days").ok().flatten()))
    .bind(payload.status.unwrap_or_else(|| current.try_get("status").unwrap_or_else(|_| "draft".to_string())))
    .bind(payload.languages.unwrap_or_else(|| current.try_get("languages").unwrap_or_else(|_| vec!["ru".to_string()])))
    .bind(payload.seo_title.map(|value| json!(value)).or_else(|| current.try_get("seo_title").ok().flatten()))
    .bind(payload.seo_description.map(|value| json!(value)).or_else(|| current.try_get("seo_description").ok().flatten()))
    .bind(site_id)
    .fetch_one(&pool)
    .await?;
    let offers = offers_for(&pool, id).await?;
    Ok(Json(affiliate_from_row(row, offers)))
}

pub async fn delete_affiliate_product(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    Query(query): Query<SiteQuery>,
    State(pool): State<PgPool>,
) -> Result<StatusCode, AppError> {
    let site_id = resolve_site_id(&query, KITCHEN_SITE_ID);
    sqlx::query("DELETE FROM admin_affiliate_products WHERE id = $1 AND site_id = $2")
        .bind(id)
        .bind(site_id)
        .execute(&pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct ImportAffiliateUrlRequest {
    pub url: String,
    pub site: String,
}

pub async fn import_affiliate_url(
    _claims: AdminClaims,
    Json(req): Json<ImportAffiliateUrlRequest>,
) -> Json<Value> {
    let merchant = req
        .url
        .split('/')
        .nth(2)
        .unwrap_or("custom")
        .trim_start_matches("www.")
        .to_string();
    Json(json!({
        "site": req.site,
        "affiliateUrl": req.url,
        "merchant": merchant,
        "network": "custom"
    }))
}

pub async fn list_affiliate_offers(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    State(pool): State<PgPool>,
) -> Result<Json<Vec<AffiliateOfferDto>>, AppError> {
    Ok(Json(offers_for(&pool, id).await?))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentArticleDto {
    pub id: String,
    pub site: String,
    #[serde(rename = "type")]
    pub article_type: String,
    pub title: LocalizedText,
    pub slug: String,
    pub excerpt: LocalizedText,
    pub status: String,
    pub languages: Vec<String>,
    pub affiliate_product_ids: Vec<String>,
    pub seo_title: Option<LocalizedText>,
    pub seo_description: Option<LocalizedText>,
    pub published_at: Option<String>,
    pub updated_at: Option<String>,
}

fn article_from_row(row: sqlx::postgres::PgRow, article_type: &str) -> ContentArticleDto {
    let title_ru: Option<String> = row.try_get("title_ru").ok().flatten();
    let title_pl: Option<String> = row.try_get("title_pl").ok().flatten();
    let title_en: Option<String> = row.try_get("title_en").ok().flatten();
    let fallback = title_ru
        .clone()
        .or(title_pl.clone())
        .or(title_en.clone())
        .unwrap_or_else(|| row.try_get("slug").unwrap_or_default());
    let mut title = default_text(&fallback);
    if let Some(value) = title_ru {
        title.insert("ru".to_string(), value);
    }
    if let Some(value) = title_pl {
        title.insert("pl".to_string(), value);
    }
    if let Some(value) = title_en {
        title.insert("en".to_string(), value);
    }

    let mut excerpt = default_text("");
    if let Some(value) = row
        .try_get::<Option<String>, _>("seo_description_ru")
        .ok()
        .flatten()
    {
        excerpt.insert("ru".to_string(), value);
    }
    if let Some(value) = row
        .try_get::<Option<String>, _>("seo_description_pl")
        .ok()
        .flatten()
    {
        excerpt.insert("pl".to_string(), value);
    }
    if let Some(value) = row
        .try_get::<Option<String>, _>("seo_description_en")
        .ok()
        .flatten()
    {
        excerpt.insert("en".to_string(), value);
    }

    ContentArticleDto {
        id: row.try_get::<Uuid, _>("id").unwrap().to_string(),
        site: "kitchen".to_string(),
        article_type: article_type.to_string(),
        title,
        slug: row.try_get("slug").unwrap_or_default(),
        excerpt,
        status: if row.try_get::<bool, _>("published").unwrap_or(false) {
            "published".to_string()
        } else {
            "draft".to_string()
        },
        languages: vec!["ru".to_string(), "pl".to_string(), "en".to_string()],
        affiliate_product_ids: Vec::new(),
        seo_title: None,
        seo_description: None,
        published_at: row.try_get("published_at").ok().flatten(),
        updated_at: row.try_get("updated_at").ok().flatten(),
    }
}

async fn list_articles_by_kind(
    pool: &PgPool,
    kind: &str,
) -> Result<Vec<ContentArticleDto>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT id, slug, category, title_ru, title_pl, title_en, seo_description_ru,
               seo_description_pl, seo_description_en, published,
               to_char(published_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS published_at,
               to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS updated_at
        FROM articles
        WHERE ($1 = 'review' AND category ILIKE '%review%')
           OR ($1 <> 'review' AND (category IS NULL OR category NOT ILIKE '%review%'))
        ORDER BY updated_at DESC
        LIMIT 200
        "#,
    )
    .bind(kind)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| article_from_row(row, kind))
        .collect())
}

pub async fn list_culinary_products(
    claims: AdminClaims,
    State(pool): State<PgPool>,
) -> Result<Json<Vec<AffiliateProductDto>>, AppError> {
    list_affiliate_products(
        claims,
        Query(SiteQuery {
            site_id: Some(KITCHEN_SITE_ID),
            site: Some("kitchen".to_string()),
        }),
        State(pool),
    )
    .await
}

pub async fn list_culinary_recipes(
    _claims: AdminClaims,
    State(pool): State<PgPool>,
) -> Result<Json<Vec<ContentArticleDto>>, AppError> {
    Ok(Json(list_articles_by_kind(&pool, "article").await?))
}

pub async fn list_culinary_reviews(
    _claims: AdminClaims,
    State(pool): State<PgPool>,
) -> Result<Json<Vec<ContentArticleDto>>, AppError> {
    Ok(Json(list_articles_by_kind(&pool, "review").await?))
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstructionMaterialDto {
    pub id: String,
    pub title: LocalizedText,
    pub slug: String,
    pub category: String,
    pub city: String,
    pub supplier_ids: Vec<String>,
    pub unit: String,
    pub material_price: Option<f64>,
    pub work_price: Option<f64>,
    pub currency: String,
    pub margin_percent: Option<f64>,
    pub status: String,
}

fn material_from_row(row: sqlx::postgres::PgRow) -> ConstructionMaterialDto {
    let slug: String = row.try_get("slug").unwrap_or_default();
    let supplier_ids: Vec<Uuid> = row.try_get("supplier_ids").unwrap_or_default();
    ConstructionMaterialDto {
        id: row.try_get::<Uuid, _>("id").unwrap().to_string(),
        title: localized_from_value(row.try_get("title").ok(), &slug),
        slug,
        category: row.try_get("category").unwrap_or_default(),
        city: row.try_get("city").unwrap_or_else(|_| "Алматы".to_string()),
        supplier_ids: supplier_ids.into_iter().map(|id| id.to_string()).collect(),
        unit: row.try_get("unit").unwrap_or_else(|_| "m2".to_string()),
        material_price: dec(row.try_get("material_price").ok().flatten()),
        work_price: dec(row.try_get("work_price").ok().flatten()),
        currency: row
            .try_get("currency")
            .unwrap_or_else(|_| "KZT".to_string()),
        margin_percent: dec(row.try_get("margin_percent").ok().flatten()),
        status: row
            .try_get("status")
            .unwrap_or_else(|_| "draft".to_string()),
    }
}

pub async fn list_construction_materials(
    _claims: AdminClaims,
    State(pool): State<PgPool>,
) -> Result<Json<Vec<ConstructionMaterialDto>>, AppError> {
    let rows = sqlx::query(
        "SELECT id, title, slug, category, city, supplier_ids, unit, material_price, work_price, currency, margin_percent, status FROM admin_construction_materials ORDER BY updated_at DESC",
    )
    .fetch_all(&pool)
    .await?;
    Ok(Json(rows.into_iter().map(material_from_row).collect()))
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstructionBundleDto {
    pub id: String,
    pub title: LocalizedText,
    pub slug: String,
    pub city: String,
    pub materials: Vec<String>,
    pub works: Vec<String>,
    pub area_m2: Option<f64>,
    pub material_cost: Option<f64>,
    pub work_cost: Option<f64>,
    pub total_price: Option<f64>,
    pub currency: String,
    pub supplier_ids: Vec<String>,
    pub lead_form_enabled: bool,
    pub status: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstructionBundlePayload {
    pub title: Option<LocalizedText>,
    pub slug: Option<String>,
    pub city: Option<String>,
    pub materials: Option<Vec<String>>,
    pub works: Option<Vec<String>>,
    pub area_m2: Option<f64>,
    pub material_cost: Option<f64>,
    pub work_cost: Option<f64>,
    pub total_price: Option<f64>,
    pub currency: Option<String>,
    pub supplier_ids: Option<Vec<String>>,
    pub lead_form_enabled: Option<bool>,
    pub status: Option<String>,
}

fn bundle_from_row(row: sqlx::postgres::PgRow) -> ConstructionBundleDto {
    let slug: String = row.try_get("slug").unwrap_or_default();
    let supplier_ids: Vec<Uuid> = row.try_get("supplier_ids").unwrap_or_default();
    ConstructionBundleDto {
        id: row.try_get::<Uuid, _>("id").unwrap().to_string(),
        title: localized_from_value(row.try_get("title").ok(), &slug),
        slug,
        city: row.try_get("city").unwrap_or_else(|_| "Алматы".to_string()),
        materials: row.try_get("materials").unwrap_or_default(),
        works: row.try_get("works").unwrap_or_default(),
        area_m2: dec(row.try_get("area_m2").ok().flatten()),
        material_cost: dec(row.try_get("material_cost").ok().flatten()),
        work_cost: dec(row.try_get("work_cost").ok().flatten()),
        total_price: dec(row.try_get("total_price").ok().flatten()),
        currency: row
            .try_get("currency")
            .unwrap_or_else(|_| "KZT".to_string()),
        supplier_ids: supplier_ids.into_iter().map(|id| id.to_string()).collect(),
        lead_form_enabled: row.try_get("lead_form_enabled").unwrap_or(true),
        status: row
            .try_get("status")
            .unwrap_or_else(|_| "draft".to_string()),
    }
}

pub async fn list_construction_bundles(
    _claims: AdminClaims,
    State(pool): State<PgPool>,
) -> Result<Json<Vec<ConstructionBundleDto>>, AppError> {
    let rows = sqlx::query(
        "SELECT id, title, slug, city, materials, works, area_m2, material_cost, work_cost, total_price, currency, supplier_ids, lead_form_enabled, status FROM admin_construction_bundles ORDER BY updated_at DESC",
    )
    .fetch_all(&pool)
    .await?;
    Ok(Json(rows.into_iter().map(bundle_from_row).collect()))
}

pub async fn create_construction_bundle(
    _claims: AdminClaims,
    State(pool): State<PgPool>,
    Json(payload): Json<ConstructionBundlePayload>,
) -> Result<(StatusCode, Json<ConstructionBundleDto>), AppError> {
    let title = payload
        .title
        .unwrap_or_else(|| default_text("Комплект материалов"));
    let title_ru = title
        .get("ru")
        .cloned()
        .unwrap_or_else(|| "Комплект материалов".to_string());
    let total = payload.total_price.or_else(|| {
        Some((payload.material_cost.unwrap_or(0.0) + payload.work_cost.unwrap_or(0.0)) * 1.18)
    });
    let supplier_ids = payload
        .supplier_ids
        .unwrap_or_default()
        .into_iter()
        .filter_map(|id| Uuid::parse_str(&id).ok())
        .collect::<Vec<_>>();
    let row = sqlx::query(
        r#"
        INSERT INTO admin_construction_bundles (
            title, slug, city, materials, works, area_m2, material_cost, work_cost,
            total_price, currency, supplier_ids, lead_form_enabled, status
        )
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)
        RETURNING id, title, slug, city, materials, works, area_m2, material_cost, work_cost,
                  total_price, currency, supplier_ids, lead_form_enabled, status
        "#,
    )
    .bind(json!(title))
    .bind(payload.slug.unwrap_or_else(|| slugify(&title_ru)))
    .bind(payload.city.unwrap_or_else(|| "Алматы".to_string()))
    .bind(payload.materials.unwrap_or_default())
    .bind(payload.works.unwrap_or_default())
    .bind(payload.area_m2.and_then(Decimal::from_f64_retain))
    .bind(payload.material_cost.and_then(Decimal::from_f64_retain))
    .bind(payload.work_cost.and_then(Decimal::from_f64_retain))
    .bind(total.and_then(Decimal::from_f64_retain))
    .bind(payload.currency.unwrap_or_else(|| "KZT".to_string()))
    .bind(supplier_ids)
    .bind(payload.lead_form_enabled.unwrap_or(true))
    .bind(payload.status.unwrap_or_else(|| "draft".to_string()))
    .fetch_one(&pool)
    .await?;
    Ok((StatusCode::CREATED, Json(bundle_from_row(row))))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalculateRepairRequest {
    pub area_m2: f64,
    pub material_cost: f64,
    pub work_cost: f64,
    pub margin_percent: f64,
    pub currency: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalculateRepairResponse {
    pub id: String,
    pub title: LocalizedText,
    pub city: String,
    pub area_m2: f64,
    pub material_cost: f64,
    pub work_cost: f64,
    pub margin_percent: f64,
    pub total_price: f64,
    pub currency: String,
    pub updated_at: String,
}

pub async fn calculate_repair(
    _claims: AdminClaims,
    Json(payload): Json<CalculateRepairRequest>,
) -> Json<CalculateRepairResponse> {
    let total = ((payload.material_cost + payload.work_cost)
        * (1.0 + payload.margin_percent / 100.0))
        .round();
    Json(CalculateRepairResponse {
        id: "calc-live".to_string(),
        title: default_text("Расчет ремонта"),
        city: "Алматы".to_string(),
        area_m2: payload.area_m2,
        material_cost: payload.material_cost,
        work_cost: payload.work_cost,
        margin_percent: payload.margin_percent,
        total_price: total,
        currency: payload.currency,
        updated_at: chrono::Utc::now().to_rfc3339(),
    })
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SupplierDto {
    pub id: String,
    pub name: String,
    pub country: String,
    pub city: Option<String>,
    pub categories: Vec<String>,
    pub contact: String,
    pub website: Option<String>,
    pub commission_terms: Option<String>,
    #[serde(rename = "type")]
    pub supplier_type: String,
}

fn supplier_from_row(row: sqlx::postgres::PgRow) -> SupplierDto {
    SupplierDto {
        id: row.try_get::<Uuid, _>("id").unwrap().to_string(),
        name: row.try_get("name").unwrap_or_default(),
        country: row.try_get("country").unwrap_or_default(),
        city: row.try_get("city").ok().flatten(),
        categories: row.try_get("categories").unwrap_or_default(),
        contact: row.try_get("contact").unwrap_or_default(),
        website: row.try_get("website").ok().flatten(),
        commission_terms: row.try_get("commission_terms").ok().flatten(),
        supplier_type: row
            .try_get("supplier_type")
            .unwrap_or_else(|_| "local_supplier".to_string()),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SupplierPayload {
    pub name: Option<String>,
    pub country: Option<String>,
    pub city: Option<String>,
    pub categories: Option<Vec<String>>,
    pub contact: Option<String>,
    pub website: Option<String>,
    pub commission_terms: Option<String>,
    #[serde(rename = "type")]
    pub supplier_type: Option<String>,
}

pub async fn list_suppliers(
    _claims: AdminClaims,
    Query(query): Query<SiteQuery>,
    State(pool): State<PgPool>,
) -> Result<Json<Vec<SupplierDto>>, AppError> {
    let site_id = resolve_site_id(&query, CONSTRUCTION_SITE_ID);
    let rows = sqlx::query(
        "SELECT id, name, country, city, categories, contact, website, commission_terms, supplier_type FROM admin_suppliers WHERE site_id = $1 OR is_global = true ORDER BY updated_at DESC",
    )
    .bind(site_id)
    .fetch_all(&pool)
    .await?;
    Ok(Json(rows.into_iter().map(supplier_from_row).collect()))
}

pub async fn create_supplier(
    _claims: AdminClaims,
    Query(query): Query<SiteQuery>,
    State(pool): State<PgPool>,
    Json(payload): Json<SupplierPayload>,
) -> Result<(StatusCode, Json<SupplierDto>), AppError> {
    let site_id = resolve_site_id(&query, CONSTRUCTION_SITE_ID);
    let row = sqlx::query(
        r#"
        INSERT INTO admin_suppliers (site_id, name, country, city, categories, contact, website, commission_terms, supplier_type)
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
        RETURNING id, name, country, city, categories, contact, website, commission_terms, supplier_type
        "#,
    )
    .bind(site_id)
    .bind(payload.name.unwrap_or_else(|| "Новый поставщик".to_string()))
    .bind(payload.country.unwrap_or_default())
    .bind(payload.city)
    .bind(payload.categories.unwrap_or_default())
    .bind(payload.contact.unwrap_or_default())
    .bind(payload.website)
    .bind(payload.commission_terms)
    .bind(payload.supplier_type.unwrap_or_else(|| "local_supplier".to_string()))
    .fetch_one(&pool)
    .await?;
    Ok((StatusCode::CREATED, Json(supplier_from_row(row))))
}

pub async fn update_supplier(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    Query(query): Query<SiteQuery>,
    State(pool): State<PgPool>,
    Json(payload): Json<SupplierPayload>,
) -> Result<Json<SupplierDto>, AppError> {
    let site_id = resolve_site_id(&query, CONSTRUCTION_SITE_ID);
    let current = sqlx::query(
        "SELECT id, name, country, city, categories, contact, website, commission_terms, supplier_type FROM admin_suppliers WHERE id = $1 AND site_id = $2",
    )
    .bind(id)
    .bind(site_id)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::not_found("Supplier not found"))?;
    let row = sqlx::query(
        r#"
        UPDATE admin_suppliers SET name = $2, country = $3, city = $4, categories = $5,
            contact = $6, website = $7, commission_terms = $8, supplier_type = $9, updated_at = NOW()
        WHERE id = $1 AND site_id = $10
        RETURNING id, name, country, city, categories, contact, website, commission_terms, supplier_type
        "#,
    )
    .bind(id)
    .bind(payload.name.unwrap_or_else(|| current.try_get("name").unwrap_or_default()))
    .bind(payload.country.unwrap_or_else(|| current.try_get("country").unwrap_or_default()))
    .bind(payload.city.or_else(|| current.try_get("city").ok().flatten()))
    .bind(payload.categories.unwrap_or_else(|| current.try_get("categories").unwrap_or_default()))
    .bind(payload.contact.unwrap_or_else(|| current.try_get("contact").unwrap_or_default()))
    .bind(payload.website.or_else(|| current.try_get("website").ok().flatten()))
    .bind(payload.commission_terms.or_else(|| current.try_get("commission_terms").ok().flatten()))
    .bind(payload.supplier_type.unwrap_or_else(|| current.try_get("supplier_type").unwrap_or_else(|_| "local_supplier".to_string())))
    .bind(site_id)
    .fetch_one(&pool)
    .await?;
    Ok(Json(supplier_from_row(row)))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeadDto {
    pub id: String,
    pub client_name: String,
    pub contact: String,
    pub source_site: String,
    pub category: String,
    pub city: Option<String>,
    pub message: String,
    pub status: String,
    pub potential_value: Option<f64>,
    pub currency: String,
    pub created_at: String,
}

fn lead_from_row(row: sqlx::postgres::PgRow) -> LeadDto {
    let phone: String = row.try_get("phone").unwrap_or_default();
    LeadDto {
        id: row.try_get::<Uuid, _>("id").unwrap().to_string(),
        client_name: row.try_get("name").unwrap_or_default(),
        contact: row
            .try_get::<Option<String>, _>("contact")
            .ok()
            .flatten()
            .unwrap_or(phone),
        source_site: match row
            .try_get::<String, _>("site")
            .unwrap_or_default()
            .as_str()
        {
            "almabuild" | "construction" => "construction".to_string(),
            "icons" | "church" => "church".to_string(),
            _ => "kitchen".to_string(),
        },
        category: row.try_get::<String, _>("category").unwrap_or_default(),
        city: row.try_get("city").ok().flatten(),
        message: row.try_get("comment").unwrap_or_default(),
        status: row.try_get("status").unwrap_or_else(|_| "new".to_string()),
        potential_value: dec(row.try_get("potential_value").ok().flatten()),
        currency: row
            .try_get("currency")
            .unwrap_or_else(|_| "KZT".to_string()),
        created_at: row.try_get("created_at").unwrap_or_default(),
    }
}

pub async fn list_leads(
    _claims: AdminClaims,
    Query(query): Query<SiteQuery>,
    State(pool): State<PgPool>,
) -> Result<Json<Vec<LeadDto>>, AppError> {
    let site_id = resolve_site_id(&query, KITCHEN_SITE_ID);
    let rows = sqlx::query(
        r#"
        SELECT id, site, name, phone, object_type, area, comment, items, status,
               potential_value, currency, category, city, contact,
               to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at
        FROM site_leads
        WHERE site_id = $1 OR is_global = true
        ORDER BY created_at DESC
        LIMIT 500
        "#,
    )
    .bind(site_id)
    .fetch_all(&pool)
    .await?;
    Ok(Json(rows.into_iter().map(lead_from_row).collect()))
}

#[derive(Debug, Deserialize)]
pub struct UpdateLeadStatusRequest {
    pub status: String,
}

pub async fn update_lead_status(
    _claims: AdminClaims,
    Path(id): Path<Uuid>,
    Query(query): Query<SiteQuery>,
    State(pool): State<PgPool>,
    Json(payload): Json<UpdateLeadStatusRequest>,
) -> Result<Json<LeadDto>, AppError> {
    let site_id = resolve_site_id(&query, KITCHEN_SITE_ID);
    let row = sqlx::query(
        r#"
        UPDATE site_leads SET status = $2
        WHERE id = $1 AND site_id = $3
        RETURNING id, site, name, phone, object_type, area, comment, items, status,
                  potential_value, currency, category, city, contact,
                  to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at
        "#,
    )
    .bind(id)
    .bind(payload.status)
    .bind(site_id)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::not_found("Lead not found"))?;
    Ok(Json(lead_from_row(row)))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteDashboardMetricsDto {
    pub site: String,
    pub visitors: i64,
    pub affiliate_clicks: i64,
    pub leads: i64,
    pub revenue_estimate: f64,
    pub currency: String,
    pub published_pages: i64,
    pub ai_drafts: i64,
    pub seo_status: String,
    pub top_pages: Vec<Value>,
    pub top_products: Vec<Value>,
    pub recent_leads: Vec<LeadDto>,
    pub seo_tasks: Vec<Value>,
}

pub async fn dashboard_metrics(
    _claims: AdminClaims,
    Query(query): Query<SiteQuery>,
    State(pool): State<PgPool>,
) -> Result<Json<SiteDashboardMetricsDto>, AppError> {
    let site_id = resolve_site_id(&query, KITCHEN_SITE_ID);
    let site = canonical_site_key(site_id).to_string();
    let products: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM admin_affiliate_products WHERE site_id = $1 OR is_global = true",
    )
    .bind(site_id)
    .fetch_one(&pool)
    .await?;
    let published_products: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM admin_affiliate_products WHERE (site_id = $1 OR is_global = true) AND status IN ('active', 'published')")
        .bind(site_id)
        .fetch_one(&pool)
        .await?;
    let leads_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM site_leads WHERE site_id = $1 OR is_global = true",
    )
    .bind(site_id)
    .fetch_one(&pool)
    .await
    .unwrap_or(0);
    let revenue: Option<Decimal> = sqlx::query_scalar(
        "SELECT COALESCE(SUM(potential_value), 0) FROM site_leads WHERE site_id = $1 OR is_global = true",
    )
    .bind(site_id)
    .fetch_one(&pool)
    .await
    .ok();
    let recent_rows = sqlx::query(
        r#"
        SELECT id, site, name, phone, object_type, area, comment, items, status,
               potential_value, currency, category, city, contact,
               to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at
        FROM site_leads
        WHERE site_id = $1 OR is_global = true
        ORDER BY created_at DESC
        LIMIT 5
        "#,
    )
    .bind(site_id)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();
    let recent = recent_rows
        .into_iter()
        .map(lead_from_row)
        .collect::<Vec<_>>();
    let product_rows = sqlx::query("SELECT id, title, slug, price, currency FROM admin_affiliate_products WHERE site_id = $1 OR is_global = true ORDER BY updated_at DESC LIMIT 5")
        .bind(site_id)
        .fetch_all(&pool)
        .await?;
    let top_products = product_rows
        .into_iter()
        .map(|row| {
            let id = row.try_get::<Uuid, _>("id").unwrap().to_string();
            let slug: String = row.try_get("slug").unwrap_or_default();
            let title = localized_from_value(row.try_get("title").ok(), &slug)
                .get("ru")
                .cloned()
                .unwrap_or(slug);
            json!({
                "productId": id,
                "title": title,
                "clicks": 0,
                "revenue": dec_or_zero(row.try_get("price").ok().flatten())
            })
        })
        .collect();

    Ok(Json(SiteDashboardMetricsDto {
        site: site.clone(),
        visitors: 0,
        affiliate_clicks: 0,
        leads: leads_count,
        revenue_estimate: dec_or_zero(revenue),
        currency: if site == "construction" { "KZT" } else { "PLN" }.to_string(),
        published_pages: published_products,
        ai_drafts: products.saturating_sub(published_products),
        seo_status: "needs_work".to_string(),
        top_pages: Vec::new(),
        top_products,
        recent_leads: recent,
        seo_tasks: vec![json!({
            "title": "Подключить события affiliate_click и lead_submit",
            "priority": "medium",
            "status": "draft"
        })],
    }))
}
