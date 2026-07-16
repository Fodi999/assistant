use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use super::icons_site::IconsSiteContent;
use super::site_context::{resolve_site_id, SiteQuery, CHURCH_SITE_ID};

const OLD_ICONS_SITE_KEY: &str = "svet-ikony";

#[derive(Debug, Deserialize)]
pub struct ChurchContentQuery {
    pub site_id: Option<Uuid>,
    pub site: Option<String>,
    pub year: Option<i32>,
    pub month: Option<u32>,
    pub calendar_day_id: Option<Uuid>,
    pub icon_id: Option<Uuid>,
    pub language: Option<String>,
    pub preview_token: Option<String>,
}

impl ChurchContentQuery {
    fn site_query(&self) -> SiteQuery {
        SiteQuery {
            site_id: self.site_id,
            site: self.site.clone(),
        }
    }

    pub(crate) fn site_id(&self) -> Uuid {
        resolve_site_id(&self.site_query(), CHURCH_SITE_ID)
    }
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ChurchCalendarDayDto {
    pub id: Uuid,
    pub site_id: Uuid,
    pub date_old_style: Option<String>,
    pub date_new_style: Option<String>,
    pub calendar_type: String,
    pub title: String,
    pub day_type: String,
    pub description: String,
    pub rank: i32,
    pub status: String,
    pub is_global: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChurchCalendarDayPayload {
    pub date_old_style: Option<String>,
    pub date_new_style: Option<String>,
    pub calendar_type: Option<String>,
    pub title: Option<String>,
    pub day_type: Option<String>,
    pub description: Option<String>,
    pub rank: Option<i32>,
    pub status: Option<String>,
    pub is_global: Option<bool>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ChurchIconDto {
    pub id: Uuid,
    pub site_id: Uuid,
    pub calendar_day_id: Option<Uuid>,
    pub title: String,
    pub slug: String,
    pub image_url: String,
    pub saint_name: String,
    pub feast_name: String,
    pub description: String,
    pub language: String,
    pub translation_group_id: Uuid,
    pub status: String,
    pub is_global: bool,
    pub order_enabled: bool,
    pub order_block_text: String,
    pub production_time: String,
    pub price_cents: Option<i64>,
    pub currency: String,
    pub consecration_available: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChurchIconPayload {
    pub calendar_day_id: Option<Uuid>,
    pub title: Option<String>,
    pub slug: Option<String>,
    pub image_url: Option<String>,
    pub saint_name: Option<String>,
    pub feast_name: Option<String>,
    pub description: Option<String>,
    pub language: Option<String>,
    pub status: Option<String>,
    pub is_global: Option<bool>,
    pub order_enabled: Option<bool>,
    pub order_block_text: Option<String>,
    pub production_time: Option<String>,
    pub price_cents: Option<i64>,
    pub currency: Option<String>,
    pub consecration_available: Option<bool>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ChurchPrayerDto {
    pub id: Uuid,
    pub site_id: Uuid,
    pub icon_id: Option<Uuid>,
    pub calendar_day_id: Option<Uuid>,
    pub slug: String,
    pub title: String,
    pub text: String,
    pub audio_url: String,
    pub qr_code_url: String,
    pub image_url: String,
    pub source: String,
    pub source_url: String,
    pub note: String,
    pub language: String,
    pub prayer_type: String,
    pub translation_group_id: Uuid,
    pub status: String,
    pub is_global: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChurchPrayerPayload {
    pub icon_id: Option<Uuid>,
    pub calendar_day_id: Option<Uuid>,
    pub slug: Option<String>,
    pub title: Option<String>,
    pub text: Option<String>,
    pub audio_url: Option<String>,
    pub qr_code_url: Option<String>,
    pub image_url: Option<String>,
    pub source: Option<String>,
    pub source_url: Option<String>,
    pub note: Option<String>,
    pub language: Option<String>,
    pub prayer_type: Option<String>,
    pub status: Option<String>,
    pub is_global: Option<bool>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ChurchSaintDto {
    pub id: Uuid,
    pub site_id: Uuid,
    pub icon_id: Option<Uuid>,
    pub calendar_day_id: Option<Uuid>,
    pub slug: String,
    pub name: String,
    pub short_description: String,
    pub biography: String,
    pub feast_day: String,
    pub image_url: String,
    pub language: String,
    pub translation_group_id: Uuid,
    pub status: String,
    pub is_global: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChurchSaintPayload {
    pub icon_id: Option<Uuid>,
    pub calendar_day_id: Option<Uuid>,
    pub slug: Option<String>,
    pub name: Option<String>,
    pub short_description: Option<String>,
    pub biography: Option<String>,
    pub feast_day: Option<String>,
    pub image_url: Option<String>,
    pub language: Option<String>,
    pub status: Option<String>,
    pub is_global: Option<bool>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ChurchAlphabetLetterDto {
    pub id: Uuid,
    pub site_id: Uuid,
    pub slug: String,
    pub letter: String,
    pub sort_order: i32,
    pub name: String,
    pub short_description: String,
    pub full_text: String,
    pub numeric_value: Option<i32>,
    pub modern_equivalent: String,
    pub color: String,
    pub card_image_url: String,
    pub main_image_url: String,
    pub seo_title: String,
    pub seo_description: String,
    pub language: String,
    pub translation_group_id: Uuid,
    pub status: String,
    pub is_global: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChurchAlphabetLetterPayload {
    pub slug: Option<String>,
    pub letter: Option<String>,
    pub sort_order: Option<i32>,
    pub name: Option<String>,
    pub short_description: Option<String>,
    pub full_text: Option<String>,
    pub numeric_value: Option<i32>,
    pub modern_equivalent: Option<String>,
    pub color: Option<String>,
    pub card_image_url: Option<String>,
    pub main_image_url: Option<String>,
    pub seo_title: Option<String>,
    pub seo_description: Option<String>,
    pub language: Option<String>,
    pub status: Option<String>,
    pub is_global: Option<bool>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ChurchArticleDto {
    pub id: Uuid,
    pub site_id: Uuid,
    pub icon_id: Option<Uuid>,
    pub calendar_day_id: Option<Uuid>,
    pub title: String,
    pub slug: String,
    pub content: String,
    pub language: String,
    pub seo_title: String,
    pub seo_description: String,
    pub status: String,
    pub is_global: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChurchArticlePayload {
    pub icon_id: Option<Uuid>,
    pub calendar_day_id: Option<Uuid>,
    pub title: Option<String>,
    pub slug: Option<String>,
    pub content: Option<String>,
    pub language: Option<String>,
    pub seo_title: Option<String>,
    pub seo_description: Option<String>,
    pub status: Option<String>,
    pub is_global: Option<bool>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ChurchGospelDto {
    pub id: Uuid,
    pub site_id: Uuid,
    pub icon_id: Option<Uuid>,
    pub calendar_day_id: Option<Uuid>,
    pub slug: String,
    pub title: String,
    pub reference: String,
    pub text: String,
    pub explanation: String,
    pub language: String,
    pub status: String,
    pub is_global: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChurchGospelPayload {
    pub icon_id: Option<Uuid>,
    pub calendar_day_id: Option<Uuid>,
    pub slug: Option<String>,
    pub title: Option<String>,
    pub reference: Option<String>,
    pub text: Option<String>,
    pub explanation: Option<String>,
    pub language: Option<String>,
    pub status: Option<String>,
    pub is_global: Option<bool>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ChurchInfoDto {
    pub id: Uuid,
    pub site_id: Uuid,
    pub address: String,
    pub maps_url: String,
    pub phone_or_site: String,
    pub priest_phone: String,
    pub image_url: String,
    pub gallery_images: Vec<String>,
    pub translations: Value,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChurchInfoPayload {
    pub address: Option<String>,
    pub maps_url: Option<String>,
    pub phone_or_site: Option<String>,
    pub priest_phone: Option<String>,
    pub image_url: Option<String>,
    pub gallery_images: Option<Vec<String>>,
    pub translations: Option<Value>,
    pub status: Option<String>,
}

fn empty_church_info(site_id: Uuid) -> ChurchInfoDto {
    ChurchInfoDto {
        id: Uuid::nil(),
        site_id,
        address: String::new(),
        maps_url: String::new(),
        phone_or_site: String::new(),
        priest_phone: String::new(),
        image_url: String::new(),
        gallery_images: Vec::new(),
        translations: Value::Object(Default::default()),
        status: "draft".into(),
        created_at: String::new(),
        updated_at: String::new(),
    }
}

pub async fn get_church_info(
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let site_id = query.site_id();
    let row: Option<ChurchInfoDto> = sqlx::query_as(
        r#"SELECT id, site_id, address, maps_url, phone_or_site, priest_phone, image_url,
                  gallery_images, translations, status, created_at::text AS created_at, updated_at::text AS updated_at
           FROM church_info WHERE site_id = $1"#,
    )
    .bind(site_id)
    .fetch_optional(&pool)
    .await
    .map_err(db_error)?;

    Ok(Json(row.unwrap_or_else(|| empty_church_info(site_id))))
}

pub async fn put_church_info(
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
    Json(payload): Json<ChurchInfoPayload>,
) -> Result<impl IntoResponse, StatusCode> {
    let site_id = query.site_id();
    let row: ChurchInfoDto = sqlx::query_as(
        r#"INSERT INTO church_info (site_id, address, maps_url, phone_or_site, priest_phone, image_url, gallery_images, translations, status)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
           ON CONFLICT (site_id) DO UPDATE SET
              address = EXCLUDED.address,
              maps_url = EXCLUDED.maps_url,
              phone_or_site = EXCLUDED.phone_or_site,
              priest_phone = EXCLUDED.priest_phone,
              image_url = EXCLUDED.image_url,
              gallery_images = EXCLUDED.gallery_images,
              translations = EXCLUDED.translations,
              status = EXCLUDED.status
           RETURNING id, site_id, address, maps_url, phone_or_site, priest_phone, image_url,
                     gallery_images, translations, status, created_at::text AS created_at, updated_at::text AS updated_at"#,
    )
    .bind(site_id)
    .bind(payload.address.unwrap_or_default())
    .bind(payload.maps_url.unwrap_or_default())
    .bind(payload.phone_or_site.unwrap_or_default())
    .bind(payload.priest_phone.unwrap_or_default())
    .bind(payload.image_url.unwrap_or_default())
    .bind(payload.gallery_images.unwrap_or_default())
    .bind(payload.translations.unwrap_or_else(|| Value::Object(Default::default())))
    .bind(payload.status.unwrap_or_else(|| "draft".into()))
    .fetch_one(&pool)
    .await
    .map_err(db_error)?;

    Ok(Json(row))
}

pub async fn public_church_info(
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let site_id = resolve_site_id(&query.site_query(), CHURCH_SITE_ID);
    let row: Option<ChurchInfoDto> = sqlx::query_as(
        r#"SELECT id, site_id, address, maps_url, phone_or_site, priest_phone, image_url,
                  gallery_images, translations, status, created_at::text AS created_at, updated_at::text AS updated_at
           FROM church_info WHERE site_id = $1 AND status = 'published'"#,
    )
    .bind(site_id)
    .fetch_optional(&pool)
    .await
    .map_err(db_error)?;

    Ok(Json(row))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChurchImportPreview {
    pub calendar_days: usize,
    pub icons: usize,
    pub prayers: usize,
    pub articles: usize,
    pub gospel: usize,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicChurchContentPage {
    pub calendar_day: ChurchCalendarDayDto,
    pub icons: Vec<ChurchIconDto>,
    pub prayers: Vec<ChurchPrayerDto>,
    pub articles: Vec<ChurchArticleDto>,
    pub gospel: Vec<ChurchGospelDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChurchTranslationRef {
    pub language: String,
    pub slug: String,
    pub title: String,
}

/// `icon` is None when the slug resolves to a translation group that has no
/// published record in the requested language; `translations` then tells the
/// client which languages are available instead of silently falling back.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicChurchIconPage {
    pub icon: Option<ChurchIconDto>,
    pub calendar_day: Option<ChurchCalendarDayDto>,
    pub prayers: Vec<ChurchPrayerDto>,
    pub articles: Vec<ChurchArticleDto>,
    pub gospel: Vec<ChurchGospelDto>,
    pub translations: Vec<ChurchTranslationRef>,
}

/// Same contract as [`PublicChurchIconPage`]: `prayer` is None when the
/// requested language has no published record in the translation group.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicChurchPrayerPage {
    pub prayer: Option<ChurchPrayerDto>,
    pub icon: Option<ChurchIconDto>,
    pub calendar_day: Option<ChurchCalendarDayDto>,
    pub translations: Vec<ChurchTranslationRef>,
}

/// Same contract as [`PublicChurchIconPage`]: `saint` is None when the
/// requested language has no published record in the translation group.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicChurchSaintPage {
    pub saint: Option<ChurchSaintDto>,
    pub icon: Option<ChurchIconDto>,
    pub calendar_day: Option<ChurchCalendarDayDto>,
    pub prayers: Vec<ChurchPrayerDto>,
    pub translations: Vec<ChurchTranslationRef>,
}

/// Same contract as [`PublicChurchSaintPage`]: `letter` is None when the
/// requested language has no published record in the translation group.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicChurchAlphabetPage {
    pub letter: Option<ChurchAlphabetLetterDto>,
    pub translations: Vec<ChurchTranslationRef>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicChurchArticlePage {
    pub article: ChurchArticleDto,
    pub icon: Option<ChurchIconDto>,
    pub calendar_day: Option<ChurchCalendarDayDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicChurchGospelPage {
    pub gospel: ChurchGospelDto,
    pub icon: Option<ChurchIconDto>,
    pub calendar_day: Option<ChurchCalendarDayDto>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PublicChurchSitemapItem {
    pub kind: String,
    pub slug: String,
    pub date: Option<String>,
    pub updated_at: String,
}

pub async fn list_calendar_days(
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let site_id = query.site_id();
    let rows: Vec<ChurchCalendarDayDto> = sqlx::query_as(
        r#"SELECT id, site_id, date_old_style::text AS date_old_style,
                  date_new_style::text AS date_new_style, calendar_type, title, day_type,
                  description, rank, status, is_global,
                  created_at::text AS created_at, updated_at::text AS updated_at
           FROM church_calendar_days
           WHERE site_id = $1 OR is_global = true
           ORDER BY COALESCE(date_new_style, date_old_style), rank DESC, title ASC"#,
    )
    .bind(site_id)
    .fetch_all(&pool)
    .await
    .map_err(db_error)?;

    Ok(Json(filter_calendar_rows(rows, query.year, query.month)))
}

pub async fn get_calendar_day(
    Path(id): Path<Uuid>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let site_id = query.site_id();
    let row: ChurchCalendarDayDto = sqlx::query_as(
        r#"SELECT id, site_id, date_old_style::text AS date_old_style,
                  date_new_style::text AS date_new_style, calendar_type, title, day_type,
                  description, rank, status, is_global,
                  created_at::text AS created_at, updated_at::text AS updated_at
           FROM church_calendar_days
           WHERE id = $1 AND (site_id = $2 OR is_global = true)"#,
    )
    .bind(id)
    .bind(site_id)
    .fetch_optional(&pool)
    .await
    .map_err(db_error)?
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(row))
}

pub async fn create_calendar_day(
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
    Json(payload): Json<ChurchCalendarDayPayload>,
) -> Result<impl IntoResponse, StatusCode> {
    let site_id = query.site_id();
    let title = required(payload.title, "title")?;
    if payload.date_old_style.is_none() && payload.date_new_style.is_none() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let row: ChurchCalendarDayDto = sqlx::query_as(
        r#"INSERT INTO church_calendar_days
           (site_id, date_old_style, date_new_style, calendar_type, title, day_type,
            description, rank, status, is_global)
           VALUES ($1, $2::date, $3::date, $4, $5, $6, $7, $8, $9, $10)
           RETURNING id, site_id, date_old_style::text AS date_old_style,
                     date_new_style::text AS date_new_style, calendar_type, title, day_type,
                     description, rank, status, is_global,
                     created_at::text AS created_at, updated_at::text AS updated_at"#,
    )
    .bind(site_id)
    .bind(payload.date_old_style)
    .bind(payload.date_new_style)
    .bind(payload.calendar_type.unwrap_or_else(|| "both".into()))
    .bind(title)
    .bind(payload.day_type.unwrap_or_else(|| "saint".into()))
    .bind(payload.description.unwrap_or_default())
    .bind(payload.rank.unwrap_or_default())
    .bind(payload.status.unwrap_or_else(|| "draft".into()))
    .bind(payload.is_global.unwrap_or(false))
    .fetch_one(&pool)
    .await
    .map_err(db_error)?;

    Ok((StatusCode::CREATED, Json(row)))
}

pub async fn update_calendar_day(
    Path(id): Path<Uuid>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
    Json(payload): Json<ChurchCalendarDayPayload>,
) -> Result<impl IntoResponse, StatusCode> {
    let site_id = query.site_id();
    let current: ChurchCalendarDayDto = sqlx::query_as(
        r#"SELECT id, site_id, date_old_style::text AS date_old_style,
                  date_new_style::text AS date_new_style, calendar_type, title, day_type,
                  description, rank, status, is_global,
                  created_at::text AS created_at, updated_at::text AS updated_at
           FROM church_calendar_days WHERE id = $1 AND site_id = $2"#,
    )
    .bind(id)
    .bind(site_id)
    .fetch_optional(&pool)
    .await
    .map_err(db_error)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let row: ChurchCalendarDayDto = sqlx::query_as(
        r#"UPDATE church_calendar_days SET
              date_old_style = $1::date, date_new_style = $2::date, calendar_type = $3, title = $4,
              day_type = $5, description = $6, rank = $7, status = $8, is_global = $9
           WHERE id = $10 AND site_id = $11
           RETURNING id, site_id, date_old_style::text AS date_old_style,
                     date_new_style::text AS date_new_style, calendar_type, title, day_type,
                     description, rank, status, is_global,
                     created_at::text AS created_at, updated_at::text AS updated_at"#,
    )
    .bind(payload.date_old_style.or(current.date_old_style))
    .bind(payload.date_new_style.or(current.date_new_style))
    .bind(payload.calendar_type.unwrap_or(current.calendar_type))
    .bind(optional_non_empty(payload.title).unwrap_or(current.title))
    .bind(payload.day_type.unwrap_or(current.day_type))
    .bind(payload.description.unwrap_or(current.description))
    .bind(payload.rank.unwrap_or(current.rank))
    .bind(payload.status.unwrap_or(current.status))
    .bind(payload.is_global.unwrap_or(current.is_global))
    .bind(id)
    .bind(site_id)
    .fetch_one(&pool)
    .await
    .map_err(db_error)?;

    Ok(Json(row))
}

pub async fn delete_calendar_day(
    Path(id): Path<Uuid>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    delete_owned(&pool, "church_calendar_days", id, query.site_id()).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_icons(
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let rows: Vec<ChurchIconDto> = sqlx::query_as(
        r#"SELECT id, site_id, calendar_day_id, title, slug, image_url, saint_name, feast_name,
                  description, language, translation_group_id, status, is_global,
                  order_enabled, order_block_text, production_time, price_cents, currency, consecration_available,
                  created_at::text AS created_at, updated_at::text AS updated_at
           FROM church_icons
           WHERE (site_id = $1 OR is_global = true)
             AND ($2::uuid IS NULL OR calendar_day_id = $2)
             AND ($3::text IS NULL OR language = $3)
           ORDER BY updated_at DESC"#,
    )
    .bind(query.site_id())
    .bind(query.calendar_day_id)
    .bind(query.language)
    .fetch_all(&pool)
    .await
    .map_err(db_error)?;

    Ok(Json(rows))
}

pub async fn get_icon(
    Path(id): Path<Uuid>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    Ok(Json(get_icon_row(&pool, id, query.site_id(), true).await?))
}

pub async fn create_icon(
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
    Json(payload): Json<ChurchIconPayload>,
) -> Result<impl IntoResponse, StatusCode> {
    let title = required(payload.title, "title")?;
    let slug = required(payload.slug, "slug")?;

    let row: ChurchIconDto = sqlx::query_as(
        r#"INSERT INTO church_icons
           (site_id, calendar_day_id, title, slug, image_url, saint_name, feast_name,
            description, language, status, is_global, translation_group_id,
            order_enabled, order_block_text, production_time, price_cents, currency, consecration_available)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11,
                   COALESCE(
                       (SELECT translation_group_id FROM church_icons WHERE site_id = $1 AND slug = $4 LIMIT 1),
                       gen_random_uuid()
                   ),
                   $12, $13, $14, $15, $16, $17)
           RETURNING id, site_id, calendar_day_id, title, slug, image_url, saint_name,
                     feast_name, description, language, translation_group_id, status, is_global,
                     order_enabled, order_block_text, production_time, price_cents, currency, consecration_available,
                     created_at::text AS created_at, updated_at::text AS updated_at"#,
    )
    .bind(query.site_id())
    .bind(payload.calendar_day_id)
    .bind(title)
    .bind(slug)
    .bind(payload.image_url.unwrap_or_default())
    .bind(payload.saint_name.unwrap_or_default())
    .bind(payload.feast_name.unwrap_or_default())
    .bind(payload.description.unwrap_or_default())
    .bind(payload.language.unwrap_or_else(|| "uk".into()))
    .bind(payload.status.unwrap_or_else(|| "draft".into()))
    .bind(payload.is_global.unwrap_or(false))
    .bind(payload.order_enabled.unwrap_or(false))
    .bind(payload.order_block_text.unwrap_or_default())
    .bind(payload.production_time.unwrap_or_default())
    .bind(payload.price_cents)
    .bind(payload.currency.unwrap_or_else(|| "UAH".into()))
    .bind(payload.consecration_available.unwrap_or(false))
    .fetch_one(&pool)
    .await
    .map_err(db_error)?;

    Ok((StatusCode::CREATED, Json(row)))
}

pub async fn update_icon(
    Path(id): Path<Uuid>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
    Json(payload): Json<ChurchIconPayload>,
) -> Result<impl IntoResponse, StatusCode> {
    let site_id = query.site_id();
    let current = get_icon_row(&pool, id, site_id, false).await?;
    let row: ChurchIconDto = sqlx::query_as(
        r#"UPDATE church_icons SET
              calendar_day_id = $1, title = $2, slug = $3, image_url = $4,
              saint_name = $5, feast_name = $6, description = $7, language = $8,
              status = $9, is_global = $10,
              order_enabled = $13, order_block_text = $14, production_time = $15,
              price_cents = $16, currency = $17, consecration_available = $18,
              translation_group_id = COALESCE(
                  (SELECT other.translation_group_id FROM church_icons other
                   WHERE other.site_id = $12 AND other.slug = $3 AND other.id <> $11 LIMIT 1),
                  church_icons.translation_group_id
              )
           WHERE id = $11 AND site_id = $12
           RETURNING id, site_id, calendar_day_id, title, slug, image_url, saint_name,
                     feast_name, description, language, translation_group_id, status, is_global,
                     order_enabled, order_block_text, production_time, price_cents, currency, consecration_available,
                     created_at::text AS created_at, updated_at::text AS updated_at"#,
    )
    .bind(payload.calendar_day_id.or(current.calendar_day_id))
    .bind(optional_non_empty(payload.title).unwrap_or(current.title))
    .bind(optional_non_empty(payload.slug).unwrap_or(current.slug))
    .bind(payload.image_url.unwrap_or(current.image_url))
    .bind(payload.saint_name.unwrap_or(current.saint_name))
    .bind(payload.feast_name.unwrap_or(current.feast_name))
    .bind(payload.description.unwrap_or(current.description))
    .bind(payload.language.unwrap_or(current.language))
    .bind(payload.status.unwrap_or(current.status))
    .bind(payload.is_global.unwrap_or(current.is_global))
    .bind(id)
    .bind(site_id)
    .bind(payload.order_enabled.unwrap_or(current.order_enabled))
    .bind(payload.order_block_text.unwrap_or(current.order_block_text))
    .bind(payload.production_time.unwrap_or(current.production_time))
    .bind(payload.price_cents.or(current.price_cents))
    .bind(payload.currency.unwrap_or(current.currency))
    .bind(payload.consecration_available.unwrap_or(current.consecration_available))
    .fetch_one(&pool)
    .await
    .map_err(db_error)?;

    Ok(Json(row))
}

pub async fn delete_icon(
    Path(id): Path<Uuid>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    delete_owned(&pool, "church_icons", id, query.site_id()).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_prayers(
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let rows: Vec<ChurchPrayerDto> = sqlx::query_as(
        r#"SELECT id, site_id, icon_id, calendar_day_id, slug, title, text, audio_url, qr_code_url, image_url, source, source_url, note, language, prayer_type, translation_group_id,
                  status, is_global, created_at::text AS created_at, updated_at::text AS updated_at
           FROM church_prayers
           WHERE (site_id = $1 OR is_global = true)
             AND ($2::uuid IS NULL OR calendar_day_id = $2)
             AND ($3::uuid IS NULL OR icon_id = $3)
             AND ($4::text IS NULL OR language = $4)
           ORDER BY prayer_type ASC, updated_at DESC"#,
    )
    .bind(query.site_id())
    .bind(query.calendar_day_id)
    .bind(query.icon_id)
    .bind(query.language)
    .fetch_all(&pool)
    .await
    .map_err(db_error)?;

    Ok(Json(rows))
}

pub async fn get_prayer(
    Path(id): Path<Uuid>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let row: ChurchPrayerDto = sqlx::query_as(
        r#"SELECT id, site_id, icon_id, calendar_day_id, slug, title, text, audio_url, qr_code_url, image_url, source, source_url, note, language, prayer_type, translation_group_id,
                  status, is_global, created_at::text AS created_at, updated_at::text AS updated_at
           FROM church_prayers
           WHERE id = $1 AND (site_id = $2 OR is_global = true)"#,
    )
    .bind(id)
    .bind(query.site_id())
    .fetch_optional(&pool)
    .await
    .map_err(db_error)?
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(row))
}

pub async fn create_prayer(
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
    Json(payload): Json<ChurchPrayerPayload>,
) -> Result<impl IntoResponse, StatusCode> {
    let title = required(payload.title, "title")?;
    let slug = optional_non_empty(payload.slug).unwrap_or_else(|| slugify(&title));

    let row: ChurchPrayerDto = sqlx::query_as(
        r#"INSERT INTO church_prayers
           (site_id, icon_id, calendar_day_id, slug, title, text, audio_url, qr_code_url, image_url, source, source_url, note, language, prayer_type, status, is_global, translation_group_id)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16,
                   COALESCE(
                       (SELECT translation_group_id FROM church_prayers WHERE site_id = $1 AND slug = $4 LIMIT 1),
                       gen_random_uuid()
                   ))
           RETURNING id, site_id, icon_id, calendar_day_id, slug, title, text, audio_url, qr_code_url, image_url, source, source_url, note, language, prayer_type, translation_group_id,
                     status, is_global, created_at::text AS created_at, updated_at::text AS updated_at"#,
    )
    .bind(query.site_id())
    .bind(payload.icon_id)
    .bind(payload.calendar_day_id)
    .bind(slug)
    .bind(title)
    .bind(payload.text.unwrap_or_default())
    .bind(payload.audio_url.unwrap_or_default())
    .bind(payload.qr_code_url.unwrap_or_default())
    .bind(payload.image_url.unwrap_or_default())
    .bind(payload.source.unwrap_or_default())
    .bind(payload.source_url.unwrap_or_default())
    .bind(payload.note.unwrap_or_default())
    .bind(payload.language.unwrap_or_else(|| "uk".into()))
    .bind(payload.prayer_type.unwrap_or_else(|| "prayer".into()))
    .bind(payload.status.unwrap_or_else(|| "draft".into()))
    .bind(payload.is_global.unwrap_or(false))
    .fetch_one(&pool)
    .await
    .map_err(db_error)?;

    Ok((StatusCode::CREATED, Json(row)))
}

pub async fn update_prayer(
    Path(id): Path<Uuid>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
    Json(payload): Json<ChurchPrayerPayload>,
) -> Result<impl IntoResponse, StatusCode> {
    let site_id = query.site_id();
    let current: ChurchPrayerDto = sqlx::query_as(
        r#"SELECT id, site_id, icon_id, calendar_day_id, slug, title, text, audio_url, qr_code_url, image_url, source, source_url, note, language, prayer_type, translation_group_id,
                  status, is_global, created_at::text AS created_at, updated_at::text AS updated_at
           FROM church_prayers WHERE id = $1 AND site_id = $2"#,
    )
    .bind(id)
    .bind(site_id)
    .fetch_optional(&pool)
    .await
    .map_err(db_error)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let row: ChurchPrayerDto = sqlx::query_as(
        r#"UPDATE church_prayers SET
              icon_id = $1, calendar_day_id = $2, slug = $3, title = $4, text = $5,
              audio_url = $6, qr_code_url = $7, image_url = $8, source = $9, source_url = $10, note = $11,
              language = $12, prayer_type = $13, status = $14, is_global = $15,
              translation_group_id = COALESCE(
                  (SELECT other.translation_group_id FROM church_prayers other
                   WHERE other.site_id = $17 AND other.slug = $3 AND other.id <> $16 LIMIT 1),
                  church_prayers.translation_group_id
              )
           WHERE id = $16 AND site_id = $17
           RETURNING id, site_id, icon_id, calendar_day_id, slug, title, text, audio_url, qr_code_url, image_url, source, source_url, note, language, prayer_type, translation_group_id,
                     status, is_global, created_at::text AS created_at, updated_at::text AS updated_at"#,
    )
    .bind(payload.icon_id.or(current.icon_id))
    .bind(payload.calendar_day_id.or(current.calendar_day_id))
    .bind(optional_non_empty(payload.slug).unwrap_or(current.slug))
    .bind(optional_non_empty(payload.title).unwrap_or(current.title))
    .bind(payload.text.unwrap_or(current.text))
    .bind(payload.audio_url.unwrap_or(current.audio_url))
    .bind(payload.qr_code_url.unwrap_or(current.qr_code_url))
    .bind(payload.image_url.unwrap_or(current.image_url))
    .bind(payload.source.unwrap_or(current.source))
    .bind(payload.source_url.unwrap_or(current.source_url))
    .bind(payload.note.unwrap_or(current.note))
    .bind(payload.language.unwrap_or(current.language))
    .bind(payload.prayer_type.unwrap_or(current.prayer_type))
    .bind(payload.status.unwrap_or(current.status))
    .bind(payload.is_global.unwrap_or(current.is_global))
    .bind(id)
    .bind(site_id)
    .fetch_one(&pool)
    .await
    .map_err(db_error)?;

    Ok(Json(row))
}

pub async fn delete_prayer(
    Path(id): Path<Uuid>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    delete_owned(&pool, "church_prayers", id, query.site_id()).await?;
    Ok(StatusCode::NO_CONTENT)
}

const SAINT_COLUMNS: &str = "id, site_id, icon_id, calendar_day_id, slug, name, short_description, biography, feast_day, image_url, language, translation_group_id, status, is_global, created_at::text AS created_at, updated_at::text AS updated_at";

pub async fn list_saints(
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let sql = format!(
        r#"SELECT {SAINT_COLUMNS}
           FROM church_saints
           WHERE (site_id = $1 OR is_global = true)
             AND ($2::uuid IS NULL OR calendar_day_id = $2)
             AND ($3::uuid IS NULL OR icon_id = $3)
             AND ($4::text IS NULL OR language = $4)
           ORDER BY name ASC, updated_at DESC"#
    );
    let rows: Vec<ChurchSaintDto> = sqlx::query_as(&sql)
        .bind(query.site_id())
        .bind(query.calendar_day_id)
        .bind(query.icon_id)
        .bind(query.language)
        .fetch_all(&pool)
        .await
        .map_err(db_error)?;

    Ok(Json(rows))
}

pub async fn get_saint(
    Path(id): Path<Uuid>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let sql = format!(
        r#"SELECT {SAINT_COLUMNS} FROM church_saints WHERE id = $1 AND (site_id = $2 OR is_global = true)"#
    );
    let row: ChurchSaintDto = sqlx::query_as(&sql)
        .bind(id)
        .bind(query.site_id())
        .fetch_optional(&pool)
        .await
        .map_err(db_error)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(row))
}

pub async fn create_saint(
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
    Json(payload): Json<ChurchSaintPayload>,
) -> Result<impl IntoResponse, StatusCode> {
    let name = required(payload.name, "name")?;
    let slug = optional_non_empty(payload.slug).unwrap_or_else(|| slugify(&name));

    let sql = format!(
        r#"INSERT INTO church_saints
           (site_id, icon_id, calendar_day_id, slug, name, short_description, biography, feast_day, image_url, language, status, is_global, translation_group_id)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12,
                   COALESCE(
                       (SELECT translation_group_id FROM church_saints WHERE site_id = $1 AND slug = $4 LIMIT 1),
                       gen_random_uuid()
                   ))
           RETURNING {SAINT_COLUMNS}"#
    );
    let row: ChurchSaintDto = sqlx::query_as(&sql)
        .bind(query.site_id())
        .bind(payload.icon_id)
        .bind(payload.calendar_day_id)
        .bind(slug)
        .bind(name)
        .bind(payload.short_description.unwrap_or_default())
        .bind(payload.biography.unwrap_or_default())
        .bind(payload.feast_day.unwrap_or_default())
        .bind(payload.image_url.unwrap_or_default())
        .bind(payload.language.unwrap_or_else(|| "uk".into()))
        .bind(payload.status.unwrap_or_else(|| "draft".into()))
        .bind(payload.is_global.unwrap_or(false))
        .fetch_one(&pool)
        .await
        .map_err(db_error)?;

    Ok((StatusCode::CREATED, Json(row)))
}

pub async fn update_saint(
    Path(id): Path<Uuid>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
    Json(payload): Json<ChurchSaintPayload>,
) -> Result<impl IntoResponse, StatusCode> {
    let site_id = query.site_id();
    let current_sql = format!(
        r#"SELECT {SAINT_COLUMNS} FROM church_saints WHERE id = $1 AND site_id = $2"#
    );
    let current: ChurchSaintDto = sqlx::query_as(&current_sql)
        .bind(id)
        .bind(site_id)
        .fetch_optional(&pool)
        .await
        .map_err(db_error)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let sql = format!(
        r#"UPDATE church_saints SET
              icon_id = $1, calendar_day_id = $2, slug = $3, name = $4, short_description = $5,
              biography = $6, feast_day = $7, image_url = $8, language = $9, status = $10, is_global = $11,
              translation_group_id = COALESCE(
                  (SELECT other.translation_group_id FROM church_saints other
                   WHERE other.site_id = $13 AND other.slug = $3 AND other.id <> $12 LIMIT 1),
                  church_saints.translation_group_id
              ),
              updated_at = NOW()
           WHERE id = $12 AND site_id = $13
           RETURNING {SAINT_COLUMNS}"#
    );
    let row: ChurchSaintDto = sqlx::query_as(&sql)
        .bind(payload.icon_id.or(current.icon_id))
        .bind(payload.calendar_day_id.or(current.calendar_day_id))
        .bind(optional_non_empty(payload.slug).unwrap_or(current.slug))
        .bind(optional_non_empty(payload.name).unwrap_or(current.name))
        .bind(payload.short_description.unwrap_or(current.short_description))
        .bind(payload.biography.unwrap_or(current.biography))
        .bind(payload.feast_day.unwrap_or(current.feast_day))
        .bind(payload.image_url.unwrap_or(current.image_url))
        .bind(payload.language.unwrap_or(current.language))
        .bind(payload.status.unwrap_or(current.status))
        .bind(payload.is_global.unwrap_or(current.is_global))
        .bind(id)
        .bind(site_id)
        .fetch_one(&pool)
        .await
        .map_err(db_error)?;

    Ok(Json(row))
}

pub async fn delete_saint(
    Path(id): Path<Uuid>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    delete_owned(&pool, "church_saints", id, query.site_id()).await?;
    Ok(StatusCode::NO_CONTENT)
}

const ALPHABET_COLUMNS: &str = "id, site_id, slug, letter, sort_order, name, short_description, full_text, numeric_value, modern_equivalent, color, card_image_url, main_image_url, seo_title, seo_description, language, translation_group_id, status, is_global, created_at::text AS created_at, updated_at::text AS updated_at";

pub async fn list_alphabet_letters(
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let sql = format!(
        r#"SELECT {ALPHABET_COLUMNS}
           FROM church_alphabet_letters
           WHERE (site_id = $1 OR is_global = true)
             AND ($2::text IS NULL OR language = $2)
           ORDER BY sort_order ASC, language ASC"#
    );
    let rows: Vec<ChurchAlphabetLetterDto> = sqlx::query_as(&sql)
        .bind(query.site_id())
        .bind(query.language)
        .fetch_all(&pool)
        .await
        .map_err(db_error)?;

    Ok(Json(rows))
}

pub async fn get_alphabet_letter(
    Path(id): Path<Uuid>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let sql = format!(
        r#"SELECT {ALPHABET_COLUMNS} FROM church_alphabet_letters WHERE id = $1 AND (site_id = $2 OR is_global = true)"#
    );
    let row: ChurchAlphabetLetterDto = sqlx::query_as(&sql)
        .bind(id)
        .bind(query.site_id())
        .fetch_optional(&pool)
        .await
        .map_err(db_error)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(row))
}

pub async fn create_alphabet_letter(
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
    Json(payload): Json<ChurchAlphabetLetterPayload>,
) -> Result<impl IntoResponse, StatusCode> {
    let name = required(payload.name, "name")?;
    let letter = required(payload.letter, "letter")?;
    let slug = optional_non_empty(payload.slug).unwrap_or_else(|| slugify(&name));

    let sql = format!(
        r#"INSERT INTO church_alphabet_letters
           (site_id, slug, letter, sort_order, name, short_description, full_text, numeric_value,
            modern_equivalent, color, card_image_url, main_image_url, seo_title, seo_description,
            language, status, is_global, translation_group_id)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17,
                   COALESCE(
                       (SELECT translation_group_id FROM church_alphabet_letters WHERE site_id = $1 AND slug = $2 LIMIT 1),
                       gen_random_uuid()
                   ))
           RETURNING {ALPHABET_COLUMNS}"#
    );
    let row: ChurchAlphabetLetterDto = sqlx::query_as(&sql)
        .bind(query.site_id())
        .bind(slug)
        .bind(letter)
        .bind(payload.sort_order.unwrap_or(0))
        .bind(name)
        .bind(payload.short_description.unwrap_or_default())
        .bind(payload.full_text.unwrap_or_default())
        .bind(payload.numeric_value)
        .bind(payload.modern_equivalent.unwrap_or_default())
        .bind(payload.color.unwrap_or_default())
        .bind(payload.card_image_url.unwrap_or_default())
        .bind(payload.main_image_url.unwrap_or_default())
        .bind(payload.seo_title.unwrap_or_default())
        .bind(payload.seo_description.unwrap_or_default())
        .bind(payload.language.unwrap_or_else(|| "uk".into()))
        .bind(payload.status.unwrap_or_else(|| "draft".into()))
        .bind(payload.is_global.unwrap_or(false))
        .fetch_one(&pool)
        .await
        .map_err(db_error)?;

    Ok((StatusCode::CREATED, Json(row)))
}

pub async fn update_alphabet_letter(
    Path(id): Path<Uuid>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
    Json(payload): Json<ChurchAlphabetLetterPayload>,
) -> Result<impl IntoResponse, StatusCode> {
    let site_id = query.site_id();
    let current_sql = format!(
        r#"SELECT {ALPHABET_COLUMNS} FROM church_alphabet_letters WHERE id = $1 AND site_id = $2"#
    );
    let current: ChurchAlphabetLetterDto = sqlx::query_as(&current_sql)
        .bind(id)
        .bind(site_id)
        .fetch_optional(&pool)
        .await
        .map_err(db_error)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let sql = format!(
        r#"UPDATE church_alphabet_letters SET
              slug = $1, letter = $2, sort_order = $3, name = $4, short_description = $5,
              full_text = $6, numeric_value = $7, modern_equivalent = $8, color = $9,
              card_image_url = $10, main_image_url = $11, seo_title = $12, seo_description = $13,
              language = $14, status = $15, is_global = $16,
              translation_group_id = COALESCE(
                  (SELECT other.translation_group_id FROM church_alphabet_letters other
                   WHERE other.site_id = $18 AND other.slug = $1 AND other.id <> $17 LIMIT 1),
                  church_alphabet_letters.translation_group_id
              ),
              updated_at = NOW()
           WHERE id = $17 AND site_id = $18
           RETURNING {ALPHABET_COLUMNS}"#
    );
    let row: ChurchAlphabetLetterDto = sqlx::query_as(&sql)
        .bind(optional_non_empty(payload.slug).unwrap_or(current.slug))
        .bind(optional_non_empty(payload.letter).unwrap_or(current.letter))
        .bind(payload.sort_order.unwrap_or(current.sort_order))
        .bind(optional_non_empty(payload.name).unwrap_or(current.name))
        .bind(payload.short_description.unwrap_or(current.short_description))
        .bind(payload.full_text.unwrap_or(current.full_text))
        .bind(payload.numeric_value.or(current.numeric_value))
        .bind(payload.modern_equivalent.unwrap_or(current.modern_equivalent))
        .bind(payload.color.unwrap_or(current.color))
        .bind(payload.card_image_url.unwrap_or(current.card_image_url))
        .bind(payload.main_image_url.unwrap_or(current.main_image_url))
        .bind(payload.seo_title.unwrap_or(current.seo_title))
        .bind(payload.seo_description.unwrap_or(current.seo_description))
        .bind(payload.language.unwrap_or(current.language))
        .bind(payload.status.unwrap_or(current.status))
        .bind(payload.is_global.unwrap_or(current.is_global))
        .bind(id)
        .bind(site_id)
        .fetch_one(&pool)
        .await
        .map_err(db_error)?;

    Ok(Json(row))
}

pub async fn delete_alphabet_letter(
    Path(id): Path<Uuid>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    delete_owned(&pool, "church_alphabet_letters", id, query.site_id()).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Bulk reorder: body is an ordered list of `translation_group_id`s so all
/// language rows of a letter move together (order is shared across languages).
pub async fn reorder_alphabet_letters(
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
    Json(ordered_group_ids): Json<Vec<Uuid>>,
) -> Result<impl IntoResponse, StatusCode> {
    let site_id = query.site_id();
    for (index, group_id) in ordered_group_ids.iter().enumerate() {
        sqlx::query(
            "UPDATE church_alphabet_letters SET sort_order = $1
             WHERE translation_group_id = $2 AND (site_id = $3 OR is_global = true)",
        )
        .bind(index as i32 + 1)
        .bind(group_id)
        .bind(site_id)
        .execute(&pool)
        .await
        .map_err(db_error)?;
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn public_alphabet_list(
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let language = query.language.clone().unwrap_or_else(|| "uk".into());
    let sql = format!(
        r#"SELECT {ALPHABET_COLUMNS}
           FROM church_alphabet_letters
           WHERE language = $3
             AND (site_id = $1 OR is_global = true)
             AND ($2::bool OR status = 'published')
           ORDER BY sort_order ASC"#
    );
    let letters: Vec<ChurchAlphabetLetterDto> = sqlx::query_as(&sql)
        .bind(CHURCH_SITE_ID)
        .bind(preview_allowed(&query))
        .bind(language)
        .fetch_all(&pool)
        .await
        .map_err(db_error)?;

    Ok(Json(letters))
}

pub async fn public_alphabet_by_slug(
    Path(slug): Path<String>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let language = query.language.clone().unwrap_or_else(|| "uk".into());
    let include_drafts = preview_allowed(&query);

    // The slug may belong to any language version; resolve the whole
    // translation group so language switching never falls back silently.
    let sql = format!(
        r#"SELECT {ALPHABET_COLUMNS}
           FROM church_alphabet_letters
           WHERE translation_group_id = (
                     SELECT translation_group_id FROM church_alphabet_letters
                     WHERE slug = $1
                       AND (site_id = $2 OR is_global = true)
                       AND ($3::bool OR status = 'published')
                     ORDER BY CASE WHEN language = $4 THEN 0 ELSE 1 END,
                              CASE WHEN site_id = $2 THEN 0 ELSE 1 END
                     LIMIT 1
                 )
             AND (site_id = $2 OR is_global = true)
             AND ($3::bool OR status = 'published')
           ORDER BY CASE WHEN site_id = $2 THEN 0 ELSE 1 END"#
    );
    let group: Vec<ChurchAlphabetLetterDto> = sqlx::query_as(&sql)
        .bind(&slug)
        .bind(CHURCH_SITE_ID)
        .bind(include_drafts)
        .bind(&language)
        .fetch_all(&pool)
        .await
        .map_err(db_error)?;

    if group.is_empty() {
        return Err(StatusCode::NOT_FOUND);
    }

    let mut translations: Vec<ChurchTranslationRef> = Vec::new();
    for item in &group {
        if !translations.iter().any(|t| t.language == item.language) {
            translations.push(ChurchTranslationRef {
                language: item.language.clone(),
                slug: item.slug.clone(),
                title: item.name.clone(),
            });
        }
    }

    let position = group.iter().position(|item| item.language == language);
    let letter = position.map(|index| group.into_iter().nth(index).expect("position within group"));

    Ok(Json(PublicChurchAlphabetPage { letter, translations }))
}

pub async fn list_articles(
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let rows: Vec<ChurchArticleDto> = sqlx::query_as(
        r#"SELECT id, site_id, icon_id, calendar_day_id, title, slug, content, language,
                  seo_title, seo_description, status, is_global, created_at::text AS created_at, updated_at::text AS updated_at
           FROM church_articles
           WHERE (site_id = $1 OR is_global = true)
             AND ($2::uuid IS NULL OR calendar_day_id = $2)
             AND ($3::uuid IS NULL OR icon_id = $3)
             AND ($4::text IS NULL OR language = $4)
           ORDER BY updated_at DESC"#,
    )
    .bind(query.site_id())
    .bind(query.calendar_day_id)
    .bind(query.icon_id)
    .bind(query.language)
    .fetch_all(&pool)
    .await
    .map_err(db_error)?;

    Ok(Json(rows))
}

pub async fn get_article(
    Path(id): Path<Uuid>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let row: ChurchArticleDto = sqlx::query_as(
        r#"SELECT id, site_id, icon_id, calendar_day_id, title, slug, content, language,
                  seo_title, seo_description, status, is_global, created_at::text AS created_at, updated_at::text AS updated_at
           FROM church_articles
           WHERE id = $1 AND (site_id = $2 OR is_global = true)"#,
    )
    .bind(id)
    .bind(query.site_id())
    .fetch_optional(&pool)
    .await
    .map_err(db_error)?
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(row))
}

pub async fn create_article(
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
    Json(payload): Json<ChurchArticlePayload>,
) -> Result<impl IntoResponse, StatusCode> {
    let row: ChurchArticleDto = sqlx::query_as(
        r#"INSERT INTO church_articles
           (site_id, icon_id, calendar_day_id, title, slug, content, language,
            seo_title, seo_description, status, is_global)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
           RETURNING id, site_id, icon_id, calendar_day_id, title, slug, content, language,
                     seo_title, seo_description, status, is_global, created_at::text AS created_at, updated_at::text AS updated_at"#,
    )
    .bind(query.site_id())
    .bind(payload.icon_id)
    .bind(payload.calendar_day_id)
    .bind(required(payload.title, "title")?)
    .bind(required(payload.slug, "slug")?)
    .bind(payload.content.unwrap_or_default())
    .bind(payload.language.unwrap_or_else(|| "uk".into()))
    .bind(payload.seo_title.unwrap_or_default())
    .bind(payload.seo_description.unwrap_or_default())
    .bind(payload.status.unwrap_or_else(|| "draft".into()))
    .bind(payload.is_global.unwrap_or(false))
    .fetch_one(&pool)
    .await
    .map_err(db_error)?;

    Ok((StatusCode::CREATED, Json(row)))
}

pub async fn update_article(
    Path(id): Path<Uuid>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
    Json(payload): Json<ChurchArticlePayload>,
) -> Result<impl IntoResponse, StatusCode> {
    let site_id = query.site_id();
    let current: ChurchArticleDto = sqlx::query_as(
        r#"SELECT id, site_id, icon_id, calendar_day_id, title, slug, content, language,
                  seo_title, seo_description, status, is_global, created_at::text AS created_at, updated_at::text AS updated_at
           FROM church_articles WHERE id = $1 AND site_id = $2"#,
    )
    .bind(id)
    .bind(site_id)
    .fetch_optional(&pool)
    .await
    .map_err(db_error)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let row: ChurchArticleDto = sqlx::query_as(
        r#"UPDATE church_articles SET
              icon_id = $1, calendar_day_id = $2, title = $3, slug = $4, content = $5,
              language = $6, seo_title = $7, seo_description = $8, status = $9, is_global = $10
           WHERE id = $11 AND site_id = $12
           RETURNING id, site_id, icon_id, calendar_day_id, title, slug, content, language,
                     seo_title, seo_description, status, is_global, created_at::text AS created_at, updated_at::text AS updated_at"#,
    )
    .bind(payload.icon_id.or(current.icon_id))
    .bind(payload.calendar_day_id.or(current.calendar_day_id))
    .bind(optional_non_empty(payload.title).unwrap_or(current.title))
    .bind(optional_non_empty(payload.slug).unwrap_or(current.slug))
    .bind(payload.content.unwrap_or(current.content))
    .bind(payload.language.unwrap_or(current.language))
    .bind(payload.seo_title.unwrap_or(current.seo_title))
    .bind(payload.seo_description.unwrap_or(current.seo_description))
    .bind(payload.status.unwrap_or(current.status))
    .bind(payload.is_global.unwrap_or(current.is_global))
    .bind(id)
    .bind(site_id)
    .fetch_one(&pool)
    .await
    .map_err(db_error)?;

    Ok(Json(row))
}

pub async fn delete_article(
    Path(id): Path<Uuid>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    delete_owned(&pool, "church_articles", id, query.site_id()).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_gospel(
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let rows: Vec<ChurchGospelDto> = sqlx::query_as(
        r#"SELECT id, site_id, icon_id, calendar_day_id, slug, title, reference, text, explanation,
                  language, status, is_global, created_at::text AS created_at, updated_at::text AS updated_at
           FROM church_gospel_readings
           WHERE (site_id = $1 OR is_global = true)
             AND ($2::uuid IS NULL OR calendar_day_id = $2)
             AND ($3::uuid IS NULL OR icon_id = $3)
             AND ($4::text IS NULL OR language = $4)
           ORDER BY updated_at DESC"#,
    )
    .bind(query.site_id())
    .bind(query.calendar_day_id)
    .bind(query.icon_id)
    .bind(query.language)
    .fetch_all(&pool)
    .await
    .map_err(db_error)?;

    Ok(Json(rows))
}

pub async fn get_gospel(
    Path(id): Path<Uuid>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let row: ChurchGospelDto = sqlx::query_as(
        r#"SELECT id, site_id, icon_id, calendar_day_id, slug, title, reference, text, explanation,
                  language, status, is_global, created_at::text AS created_at, updated_at::text AS updated_at
           FROM church_gospel_readings
           WHERE id = $1 AND (site_id = $2 OR is_global = true)"#,
    )
    .bind(id)
    .bind(query.site_id())
    .fetch_optional(&pool)
    .await
    .map_err(db_error)?
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(row))
}

pub async fn create_gospel(
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
    Json(payload): Json<ChurchGospelPayload>,
) -> Result<impl IntoResponse, StatusCode> {
    let title = required(payload.title, "title")?;
    let slug = optional_non_empty(payload.slug).unwrap_or_else(|| slugify(&title));

    let row: ChurchGospelDto = sqlx::query_as(
        r#"INSERT INTO church_gospel_readings
           (site_id, icon_id, calendar_day_id, slug, title, reference, text, explanation, language, status, is_global)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
           RETURNING id, site_id, icon_id, calendar_day_id, slug, title, reference, text, explanation,
                     language, status, is_global, created_at::text AS created_at, updated_at::text AS updated_at"#,
    )
    .bind(query.site_id())
    .bind(payload.icon_id)
    .bind(payload.calendar_day_id)
    .bind(slug)
    .bind(title)
    .bind(payload.reference.unwrap_or_default())
    .bind(payload.text.unwrap_or_default())
    .bind(payload.explanation.unwrap_or_default())
    .bind(payload.language.unwrap_or_else(|| "uk".into()))
    .bind(payload.status.unwrap_or_else(|| "draft".into()))
    .bind(payload.is_global.unwrap_or(false))
    .fetch_one(&pool)
    .await
    .map_err(db_error)?;

    Ok((StatusCode::CREATED, Json(row)))
}

pub async fn update_gospel(
    Path(id): Path<Uuid>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
    Json(payload): Json<ChurchGospelPayload>,
) -> Result<impl IntoResponse, StatusCode> {
    let site_id = query.site_id();
    let current: ChurchGospelDto = sqlx::query_as(
        r#"SELECT id, site_id, icon_id, calendar_day_id, slug, title, reference, text, explanation,
                  language, status, is_global, created_at::text AS created_at, updated_at::text AS updated_at
           FROM church_gospel_readings WHERE id = $1 AND site_id = $2"#,
    )
    .bind(id)
    .bind(site_id)
    .fetch_optional(&pool)
    .await
    .map_err(db_error)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let row: ChurchGospelDto = sqlx::query_as(
        r#"UPDATE church_gospel_readings SET
              icon_id = $1, calendar_day_id = $2, slug = $3, title = $4, reference = $5,
              text = $6, explanation = $7, language = $8, status = $9, is_global = $10
           WHERE id = $11 AND site_id = $12
           RETURNING id, site_id, icon_id, calendar_day_id, slug, title, reference, text, explanation,
                     language, status, is_global, created_at::text AS created_at, updated_at::text AS updated_at"#,
    )
    .bind(payload.icon_id.or(current.icon_id))
    .bind(payload.calendar_day_id.or(current.calendar_day_id))
    .bind(optional_non_empty(payload.slug).unwrap_or(current.slug))
    .bind(optional_non_empty(payload.title).unwrap_or(current.title))
    .bind(payload.reference.unwrap_or(current.reference))
    .bind(payload.text.unwrap_or(current.text))
    .bind(payload.explanation.unwrap_or(current.explanation))
    .bind(payload.language.unwrap_or(current.language))
    .bind(payload.status.unwrap_or(current.status))
    .bind(payload.is_global.unwrap_or(current.is_global))
    .bind(id)
    .bind(site_id)
    .fetch_one(&pool)
    .await
    .map_err(db_error)?;

    Ok(Json(row))
}

pub async fn delete_gospel(
    Path(id): Path<Uuid>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    delete_owned(&pool, "church_gospel_readings", id, query.site_id()).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn preview_import(
    Query(_query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let content = load_old_icons_content(&pool).await?;
    Ok(Json(build_import_preview(&content)))
}

pub async fn apply_import(
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let site_id = query.site_id();
    let content = load_old_icons_content(&pool).await?;
    let preview = build_import_preview(&content);

    for icon in content.icons.iter() {
        let slug = icon.slug.trim();
        let Some(date_text) = icon
            .calendar_date
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        if slug.is_empty() || NaiveDate::parse_from_str(date_text, "%Y-%m-%d").is_err() {
            continue;
        }

        let day_id = upsert_calendar_day_from_icon(&pool, site_id, icon, date_text).await?;
        let icon_id = upsert_icon_from_old_content(&pool, site_id, day_id, icon).await?;

        if !icon.prayer_text.trim().is_empty() {
            upsert_prayer_from_icon(&pool, site_id, day_id, icon_id, icon).await?;
        }

        if !icon.gospel_text.trim().is_empty() {
            upsert_gospel_from_icon(&pool, site_id, day_id, icon_id, icon).await?;
        }

        if has_article_content(icon) {
            upsert_article_from_icon(&pool, site_id, day_id, icon_id, icon).await?;
        }
    }

    for page in content.pages.iter() {
        if !page.slug.trim().is_empty() && !page.content.trim().is_empty() {
            upsert_article_from_legacy_page(&pool, site_id, page).await?;
        }
    }

    Ok(Json(preview))
}

pub async fn public_calendar_today(
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let today = chrono::Utc::now().date_naive().to_string();
    public_calendar_by_date(&pool, &today, query.language.as_deref(), preview_allowed(&query)).await
}

pub async fn public_calendar_day(
    Path(date): Path<String>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    public_calendar_by_date(&pool, &date, query.language.as_deref(), preview_allowed(&query)).await
}

pub async fn public_calendar_month(
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let today = chrono::Utc::now().date_naive();
    let year = query.year.unwrap_or_else(|| today.year());
    let month = query.month.unwrap_or_else(|| today.month());
    let include_drafts = preview_allowed(&query);
    let language = query.language.clone();
    let rows: Vec<ChurchCalendarDayDto> = sqlx::query_as(
        r#"SELECT id, site_id, date_old_style::text AS date_old_style,
                  date_new_style::text AS date_new_style, calendar_type, title, day_type,
                  description, rank, status, is_global,
                  created_at::text AS created_at, updated_at::text AS updated_at
           FROM church_calendar_days
           WHERE (site_id = $1 OR is_global = true)
             AND EXTRACT(YEAR FROM COALESCE(date_new_style, date_old_style)) = $2::int
             AND EXTRACT(MONTH FROM COALESCE(date_new_style, date_old_style)) = $3::int
             AND ($4::bool OR status = 'published')
           ORDER BY COALESCE(date_new_style, date_old_style), rank DESC, title ASC"#,
    )
    .bind(CHURCH_SITE_ID)
    .bind(year)
    .bind(month as i32)
    .bind(include_drafts)
    .fetch_all(&pool)
    .await
    .map_err(db_error)?;

    let mut pages = Vec::with_capacity(rows.len());
    for calendar_day in rows {
        let icons = list_public_icons(&pool, calendar_day.id, language.as_deref(), include_drafts).await?;
        let prayers =
            list_public_prayers(&pool, Some(calendar_day.id), None, language.as_deref(), include_drafts).await?;
        let articles =
            list_public_articles(&pool, Some(calendar_day.id), None, language.as_deref(), include_drafts).await?;
        let gospel =
            list_public_gospel(&pool, Some(calendar_day.id), None, language.as_deref(), include_drafts).await?;
        pages.push(PublicChurchContentPage {
            calendar_day,
            icons,
            prayers,
            articles,
            gospel,
        });
    }

    Ok(Json(pages))
}

pub async fn public_icon_by_slug(
    Path(slug): Path<String>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let include_drafts = preview_allowed(&query);
    let language = query.language.clone().unwrap_or_else(|| "uk".into());

    // The slug may belong to any language version; resolve the whole
    // translation group so language switching never falls back silently.
    let group: Vec<ChurchIconDto> = sqlx::query_as(
        r#"SELECT id, site_id, calendar_day_id, title, slug, image_url, saint_name, feast_name,
                  description, language, translation_group_id, status, is_global,
                  order_enabled, order_block_text, production_time, price_cents, currency, consecration_available,
                  created_at::text AS created_at, updated_at::text AS updated_at
           FROM church_icons
           WHERE translation_group_id = (
                     SELECT translation_group_id FROM church_icons
                     WHERE slug = $1
                       AND (site_id = $2 OR is_global = true)
                       AND ($3::bool OR status = 'published')
                     ORDER BY CASE WHEN language = $4 THEN 0 ELSE 1 END,
                              CASE WHEN site_id = $2 THEN 0 ELSE 1 END
                     LIMIT 1
                 )
             AND (site_id = $2 OR is_global = true)
             AND ($3::bool OR status = 'published')
           ORDER BY CASE WHEN site_id = $2 THEN 0 ELSE 1 END"#,
    )
    .bind(&slug)
    .bind(CHURCH_SITE_ID)
    .bind(include_drafts)
    .bind(&language)
    .fetch_all(&pool)
    .await
    .map_err(db_error)?;

    if group.is_empty() {
        return Err(StatusCode::NOT_FOUND);
    }

    let mut translations: Vec<ChurchTranslationRef> = Vec::new();
    for item in &group {
        if !translations.iter().any(|t| t.language == item.language) {
            translations.push(ChurchTranslationRef {
                language: item.language.clone(),
                slug: item.slug.clone(),
                title: item.title.clone(),
            });
        }
    }

    let position = group.iter().position(|item| item.language == language);
    let icon = position.map(|index| group.into_iter().nth(index).expect("position within group"));

    let (calendar_day, prayers, articles, gospel) = match &icon {
        Some(found) => {
            let calendar_day = match found.calendar_day_id {
                Some(day_id) => get_public_calendar_row(&pool, day_id, include_drafts).await?,
                None => None,
            };
            let prayers =
                list_public_prayers(&pool, found.calendar_day_id, Some(found.id), Some(&found.language), include_drafts).await?;
            let articles =
                list_public_articles(&pool, found.calendar_day_id, Some(found.id), Some(&found.language), include_drafts).await?;
            let gospel =
                list_public_gospel(&pool, found.calendar_day_id, Some(found.id), Some(&found.language), include_drafts).await?;
            (calendar_day, prayers, articles, gospel)
        }
        None => (None, Vec::new(), Vec::new(), Vec::new()),
    };

    Ok(Json(PublicChurchIconPage {
        icon,
        calendar_day,
        prayers,
        articles,
        gospel,
        translations,
    }))
}

pub async fn public_icon_list(
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let language = query.language.clone().unwrap_or_else(|| "uk".into());
    let icons: Vec<ChurchIconDto> = sqlx::query_as(
        r#"SELECT id, site_id, calendar_day_id, title, slug, image_url, saint_name, feast_name,
                  description, language, translation_group_id, status, is_global,
                  order_enabled, order_block_text, production_time, price_cents, currency, consecration_available,
                  created_at::text AS created_at, updated_at::text AS updated_at
           FROM church_icons
           WHERE language = $3
             AND (site_id = $1 OR is_global = true)
             AND ($2::bool OR status = 'published')
           ORDER BY title ASC"#,
    )
    .bind(CHURCH_SITE_ID)
    .bind(preview_allowed(&query))
    .bind(language)
    .fetch_all(&pool)
    .await
    .map_err(db_error)?;

    Ok(Json(icons))
}

pub async fn public_prayer_by_slug(
    Path(slug): Path<String>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let language = query.language.clone().unwrap_or_else(|| "uk".into());
    let include_drafts = preview_allowed(&query);

    // The slug may belong to any language version; resolve the whole
    // translation group so language switching never falls back silently.
    let group: Vec<ChurchPrayerDto> = sqlx::query_as(
        r#"SELECT id, site_id, icon_id, calendar_day_id, slug, title, text, audio_url, qr_code_url, image_url, source, source_url, note, language, prayer_type, translation_group_id,
                  status, is_global, created_at::text AS created_at, updated_at::text AS updated_at
           FROM church_prayers
           WHERE translation_group_id = (
                     SELECT translation_group_id FROM church_prayers
                     WHERE slug = $1
                       AND (site_id = $2 OR is_global = true)
                       AND ($3::bool OR status = 'published')
                     ORDER BY CASE WHEN language = $4 THEN 0 ELSE 1 END,
                              CASE WHEN site_id = $2 THEN 0 ELSE 1 END
                     LIMIT 1
                 )
             AND (site_id = $2 OR is_global = true)
             AND ($3::bool OR status = 'published')
           ORDER BY CASE WHEN site_id = $2 THEN 0 ELSE 1 END"#,
    )
    .bind(&slug)
    .bind(CHURCH_SITE_ID)
    .bind(include_drafts)
    .bind(&language)
    .fetch_all(&pool)
    .await
    .map_err(db_error)?;

    if group.is_empty() {
        return Err(StatusCode::NOT_FOUND);
    }

    let mut translations: Vec<ChurchTranslationRef> = Vec::new();
    for item in &group {
        if !translations.iter().any(|t| t.language == item.language) {
            translations.push(ChurchTranslationRef {
                language: item.language.clone(),
                slug: item.slug.clone(),
                title: item.title.clone(),
            });
        }
    }

    let position = group.iter().position(|item| item.language == language);
    let prayer = position.map(|index| group.into_iter().nth(index).expect("position within group"));

    let (icon, calendar_day) = match &prayer {
        Some(found) => {
            let icon = match found.icon_id {
                Some(icon_id) => get_public_icon_row(&pool, icon_id, include_drafts).await?,
                None => None,
            };
            let calendar_day = match found.calendar_day_id {
                Some(day_id) => get_public_calendar_row(&pool, day_id, include_drafts).await?,
                None => None,
            };
            (icon, calendar_day)
        }
        None => (None, None),
    };

    Ok(Json(PublicChurchPrayerPage {
        prayer,
        icon,
        calendar_day,
        translations,
    }))
}

pub async fn public_prayer_list(
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let language = query.language.clone().unwrap_or_else(|| "uk".into());
    let prayers: Vec<ChurchPrayerDto> = sqlx::query_as(
        r#"SELECT id, site_id, icon_id, calendar_day_id, slug, title, text, audio_url, qr_code_url, image_url, source, source_url, note, language, prayer_type, translation_group_id,
                  status, is_global, created_at::text AS created_at, updated_at::text AS updated_at
           FROM church_prayers
           WHERE language = $3
             AND (site_id = $1 OR is_global = true)
             AND ($2::bool OR status = 'published')
           ORDER BY created_at DESC"#,
    )
    .bind(CHURCH_SITE_ID)
    .bind(preview_allowed(&query))
    .bind(language)
    .fetch_all(&pool)
    .await
    .map_err(db_error)?;

    Ok(Json(prayers))
}

pub async fn public_saint_list(
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let language = query.language.clone().unwrap_or_else(|| "uk".into());
    let sql = format!(
        r#"SELECT {SAINT_COLUMNS}
           FROM church_saints
           WHERE language = $3
             AND (site_id = $1 OR is_global = true)
             AND ($2::bool OR status = 'published')
           ORDER BY name ASC"#
    );
    let saints: Vec<ChurchSaintDto> = sqlx::query_as(&sql)
        .bind(CHURCH_SITE_ID)
        .bind(preview_allowed(&query))
        .bind(language)
        .fetch_all(&pool)
        .await
        .map_err(db_error)?;

    Ok(Json(saints))
}

pub async fn public_saint_by_slug(
    Path(slug): Path<String>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let language = query.language.clone().unwrap_or_else(|| "uk".into());
    let include_drafts = preview_allowed(&query);

    // The slug may belong to any language version; resolve the whole
    // translation group so language switching never falls back silently.
    let sql = format!(
        r#"SELECT {SAINT_COLUMNS}
           FROM church_saints
           WHERE translation_group_id = (
                     SELECT translation_group_id FROM church_saints
                     WHERE slug = $1
                       AND (site_id = $2 OR is_global = true)
                       AND ($3::bool OR status = 'published')
                     ORDER BY CASE WHEN language = $4 THEN 0 ELSE 1 END,
                              CASE WHEN site_id = $2 THEN 0 ELSE 1 END
                     LIMIT 1
                 )
             AND (site_id = $2 OR is_global = true)
             AND ($3::bool OR status = 'published')
           ORDER BY CASE WHEN site_id = $2 THEN 0 ELSE 1 END"#
    );
    let group: Vec<ChurchSaintDto> = sqlx::query_as(&sql)
        .bind(&slug)
        .bind(CHURCH_SITE_ID)
        .bind(include_drafts)
        .bind(&language)
        .fetch_all(&pool)
        .await
        .map_err(db_error)?;

    if group.is_empty() {
        return Err(StatusCode::NOT_FOUND);
    }

    let mut translations: Vec<ChurchTranslationRef> = Vec::new();
    for item in &group {
        if !translations.iter().any(|t| t.language == item.language) {
            translations.push(ChurchTranslationRef {
                language: item.language.clone(),
                slug: item.slug.clone(),
                title: item.name.clone(),
            });
        }
    }

    let position = group.iter().position(|item| item.language == language);
    let saint = position.map(|index| group.into_iter().nth(index).expect("position within group"));

    let (icon, calendar_day, prayers) = match &saint {
        Some(found) => {
            let icon = match found.icon_id {
                Some(icon_id) => get_public_icon_row(&pool, icon_id, include_drafts).await?,
                None => None,
            };
            let calendar_day = match found.calendar_day_id {
                Some(day_id) => get_public_calendar_row(&pool, day_id, include_drafts).await?,
                None => None,
            };
            let prayers = if found.calendar_day_id.is_some() || found.icon_id.is_some() {
                list_public_prayers(&pool, found.calendar_day_id, found.icon_id, Some(&found.language), include_drafts).await?
            } else {
                Vec::new()
            };
            (icon, calendar_day, prayers)
        }
        None => (None, None, Vec::new()),
    };

    Ok(Json(PublicChurchSaintPage {
        saint,
        icon,
        calendar_day,
        prayers,
        translations,
    }))
}

pub async fn public_article_by_slug(
    Path(slug): Path<String>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let language = query.language.clone().unwrap_or_else(|| "uk".into());
    let article: ChurchArticleDto = sqlx::query_as(
        r#"SELECT id, site_id, icon_id, calendar_day_id, title, slug, content, language,
                  seo_title, seo_description, status, is_global, created_at::text AS created_at, updated_at::text AS updated_at
           FROM church_articles
           WHERE slug = $1
             AND language = $4
             AND (site_id = $2 OR is_global = true)
             AND ($3::bool OR status = 'published')
           ORDER BY CASE WHEN site_id = $2 THEN 0 ELSE 1 END
           LIMIT 1"#,
    )
    .bind(slug)
    .bind(CHURCH_SITE_ID)
    .bind(preview_allowed(&query))
    .bind(language)
    .fetch_optional(&pool)
    .await
    .map_err(db_error)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let icon = match article.icon_id {
        Some(icon_id) => get_public_icon_row(&pool, icon_id, preview_allowed(&query)).await?,
        None => None,
    };
    let calendar_day = match article.calendar_day_id {
        Some(day_id) => get_public_calendar_row(&pool, day_id, preview_allowed(&query)).await?,
        None => None,
    };

    Ok(Json(PublicChurchArticlePage {
        article,
        icon,
        calendar_day,
    }))
}

pub async fn public_gospel_list(
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let language = query.language.clone().unwrap_or_else(|| "uk".into());
    let readings: Vec<ChurchGospelDto> = sqlx::query_as(
        r#"SELECT id, site_id, icon_id, calendar_day_id, slug, title, reference, text, explanation,
                  language, status, is_global, created_at::text AS created_at, updated_at::text AS updated_at
           FROM church_gospel_readings
           WHERE language = $3
             AND (site_id = $1 OR is_global = true)
             AND ($2::bool OR status = 'published')
           ORDER BY reference ASC, title ASC"#,
    )
    .bind(CHURCH_SITE_ID)
    .bind(preview_allowed(&query))
    .bind(language)
    .fetch_all(&pool)
    .await
    .map_err(db_error)?;

    Ok(Json(readings))
}

pub async fn public_gospel_by_slug(
    Path(slug): Path<String>,
    Query(query): Query<ChurchContentQuery>,
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let language = query.language.clone().unwrap_or_else(|| "uk".into());
    let gospel: ChurchGospelDto = sqlx::query_as(
        r#"SELECT id, site_id, icon_id, calendar_day_id, slug, title, reference, text, explanation,
                  language, status, is_global, created_at::text AS created_at, updated_at::text AS updated_at
           FROM church_gospel_readings
           WHERE slug = $1
             AND language = $4
             AND (site_id = $2 OR is_global = true)
             AND ($3::bool OR status = 'published')
           ORDER BY CASE WHEN site_id = $2 THEN 0 ELSE 1 END
           LIMIT 1"#,
    )
    .bind(slug)
    .bind(CHURCH_SITE_ID)
    .bind(preview_allowed(&query))
    .bind(language)
    .fetch_optional(&pool)
    .await
    .map_err(db_error)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let icon = match gospel.icon_id {
        Some(icon_id) => get_public_icon_row(&pool, icon_id, preview_allowed(&query)).await?,
        None => None,
    };
    let calendar_day = match gospel.calendar_day_id {
        Some(day_id) => get_public_calendar_row(&pool, day_id, preview_allowed(&query)).await?,
        None => None,
    };

    Ok(Json(PublicChurchGospelPage {
        gospel,
        icon,
        calendar_day,
    }))
}

pub async fn public_sitemap(State(pool): State<PgPool>) -> Result<impl IntoResponse, StatusCode> {
    let rows: Vec<PublicChurchSitemapItem> = sqlx::query_as(
        r#"SELECT 'calendar'::text AS kind,
                  COALESCE(date_new_style::text, date_old_style::text, id::text) AS slug,
                  COALESCE(date_new_style::text, date_old_style::text) AS date,
                  updated_at::text AS updated_at
           FROM church_calendar_days
           WHERE status = 'published' AND (site_id = $1 OR is_global = true)
           UNION ALL
           SELECT 'icon'::text AS kind, slug, NULL::text AS date, updated_at::text AS updated_at
           FROM church_icons
           WHERE status = 'published' AND (site_id = $1 OR is_global = true)
           UNION ALL
           SELECT 'prayer'::text AS kind, slug, NULL::text AS date, updated_at::text AS updated_at
           FROM church_prayers
           WHERE status = 'published' AND (site_id = $1 OR is_global = true)
           UNION ALL
           SELECT 'article'::text AS kind, slug, NULL::text AS date, updated_at::text AS updated_at
           FROM church_articles
           WHERE status = 'published' AND (site_id = $1 OR is_global = true)
           UNION ALL
           SELECT 'gospel'::text AS kind, slug, NULL::text AS date, updated_at::text AS updated_at
           FROM church_gospel_readings
           WHERE status = 'published' AND (site_id = $1 OR is_global = true)
           UNION ALL
           SELECT 'saint'::text AS kind, slug, NULL::text AS date, updated_at::text AS updated_at
           FROM church_saints
           WHERE status = 'published' AND (site_id = $1 OR is_global = true)
           ORDER BY kind ASC, updated_at DESC"#,
    )
    .bind(CHURCH_SITE_ID)
    .fetch_all(&pool)
    .await
    .map_err(db_error)?;

    Ok(Json(rows))
}

fn filter_calendar_rows(
    rows: Vec<ChurchCalendarDayDto>,
    year: Option<i32>,
    month: Option<u32>,
) -> Vec<ChurchCalendarDayDto> {
    rows.into_iter()
        .filter(|row| {
            let Some(date_text) = row
                .date_new_style
                .as_deref()
                .or(row.date_old_style.as_deref())
            else {
                return true;
            };
            let Ok(date) = NaiveDate::parse_from_str(date_text, "%Y-%m-%d") else {
                return true;
            };
            year.map_or(true, |year| date.year() == year)
                && month.map_or(true, |month| date.month() == month)
        })
        .collect()
}

async fn load_old_icons_content(pool: &PgPool) -> Result<IconsSiteContent, StatusCode> {
    let row: Value = sqlx::query_scalar("SELECT content FROM site_content WHERE site = $1")
        .bind(OLD_ICONS_SITE_KEY)
        .fetch_optional(pool)
        .await
        .map_err(db_error)?
        .ok_or(StatusCode::NOT_FOUND)?;

    serde_json::from_value(row).map_err(|error| {
        tracing::error!(%error, "failed to parse old icons JSON for church import");
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

fn build_import_preview(content: &IconsSiteContent) -> ChurchImportPreview {
    let mut calendar_days = 0;
    let mut icons = 0;
    let mut prayers = 0;
    let mut articles = 0;
    let mut gospel = 0;
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    for icon in content.icons.iter() {
        let label = if icon.title.trim().is_empty() {
            icon.id.as_str()
        } else {
            icon.title.as_str()
        };
        let slug = icon.slug.trim();
        let date_text = icon.calendar_date.as_deref().unwrap_or_default().trim();

        if slug.is_empty() {
            errors.push(format!("{label}: запись без slug"));
            continue;
        }
        icons += 1;

        if date_text.is_empty() {
            errors.push(format!("{label}: запись без даты календаря"));
        } else if NaiveDate::parse_from_str(date_text, "%Y-%m-%d").is_err() {
            errors.push(format!("{label}: неверный формат даты {date_text}"));
        } else {
            calendar_days += 1;
        }

        if !icon.prayer_text.trim().is_empty() {
            prayers += 1;
        } else {
            warnings.push(format!("{label}: нет текста молитвы"));
        }

        if !icon.gospel_text.trim().is_empty() {
            gospel += 1;
        }

        if has_article_content(icon) {
            articles += 1;
        }
    }

    for page in content.pages.iter() {
        let label = if page.title.trim().is_empty() {
            page.id.as_str()
        } else {
            page.title.as_str()
        };
        if page.slug.trim().is_empty() {
            errors.push(format!("{label}: SEO-страница без slug"));
            continue;
        }
        if page.content.trim().is_empty() {
            warnings.push(format!("{label}: SEO-страница без текста"));
            continue;
        }
        articles += 1;
    }

    ChurchImportPreview {
        calendar_days,
        icons,
        prayers,
        articles,
        gospel,
        errors,
        warnings,
    }
}

async fn upsert_calendar_day_from_icon(
    pool: &PgPool,
    site_id: Uuid,
    icon: &super::icons_site::IconPage,
    date_text: &str,
) -> Result<Uuid, StatusCode> {
    if let Some(id) = sqlx::query_scalar::<_, Uuid>(
        r#"SELECT id
           FROM church_calendar_days
           WHERE site_id = $1
             AND date_new_style = $2::date
             AND title = $3
           LIMIT 1"#,
    )
    .bind(site_id)
    .bind(date_text)
    .bind(icon.title.trim())
    .fetch_optional(pool)
    .await
    .map_err(db_error)?
    {
        return Ok(id);
    }

    let day_type = if normalize_for_slug(&icon.category).contains("post") {
        "fasting"
    } else if !icon.saint_name.trim().is_empty() {
        "saint"
    } else {
        "feast"
    };

    sqlx::query_scalar(
        r#"INSERT INTO church_calendar_days
           (site_id, date_new_style, calendar_type, title, day_type, description, rank, status, is_global)
           VALUES ($1, $2::date, 'both', $3, $4, $5, $6, $7, false)
           RETURNING id"#,
    )
    .bind(site_id)
    .bind(date_text)
    .bind(icon.title.trim())
    .bind(day_type)
    .bind(first_non_empty(&[&icon.short_description, &icon.full_description]))
    .bind(if icon.status == "published" { 80 } else { 10 })
    .bind(status_or_draft(&icon.status))
    .fetch_one(pool)
    .await
    .map_err(db_error)
}

async fn upsert_icon_from_old_content(
    pool: &PgPool,
    site_id: Uuid,
    calendar_day_id: Uuid,
    icon: &super::icons_site::IconPage,
) -> Result<Uuid, StatusCode> {
    sqlx::query_scalar(
        r#"INSERT INTO church_icons
           (site_id, calendar_day_id, title, slug, image_url, saint_name, feast_name,
            description, language, status, is_global)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'uk', $9, false)
           ON CONFLICT (site_id, slug, language)
           DO UPDATE SET
              calendar_day_id = EXCLUDED.calendar_day_id,
              title = EXCLUDED.title,
              image_url = EXCLUDED.image_url,
              saint_name = EXCLUDED.saint_name,
              feast_name = EXCLUDED.feast_name,
              description = EXCLUDED.description,
              status = EXCLUDED.status,
              updated_at = NOW()
           RETURNING id"#,
    )
    .bind(site_id)
    .bind(calendar_day_id)
    .bind(icon.title.trim())
    .bind(icon.slug.trim())
    .bind(icon.image_url.trim())
    .bind(icon.saint_name.trim())
    .bind(icon.category.trim())
    .bind(first_non_empty(&[
        &icon.full_description,
        &icon.short_description,
    ]))
    .bind(status_or_draft(&icon.status))
    .fetch_one(pool)
    .await
    .map_err(db_error)
}

async fn upsert_prayer_from_icon(
    pool: &PgPool,
    site_id: Uuid,
    calendar_day_id: Uuid,
    icon_id: Uuid,
    icon: &super::icons_site::IconPage,
) -> Result<Uuid, StatusCode> {
    let slug = format!("{}-prayer", icon.slug.trim());
    let title = format!("Молитва: {}", icon.title.trim());
    sqlx::query_scalar(
        r#"INSERT INTO church_prayers
           (site_id, icon_id, calendar_day_id, slug, title, text, audio_url, qr_code_url, language, prayer_type, status, is_global)
           VALUES ($1, $2, $3, $4, $5, $6, '', '', 'uk', 'prayer', $7, false)
           ON CONFLICT (site_id, slug, language)
           DO UPDATE SET
              icon_id = EXCLUDED.icon_id,
              calendar_day_id = EXCLUDED.calendar_day_id,
              title = EXCLUDED.title,
              text = EXCLUDED.text,
              audio_url = COALESCE(NULLIF(church_prayers.audio_url, ''), EXCLUDED.audio_url),
              qr_code_url = COALESCE(NULLIF(church_prayers.qr_code_url, ''), EXCLUDED.qr_code_url),
              prayer_type = EXCLUDED.prayer_type,
              status = EXCLUDED.status,
              updated_at = NOW()
           RETURNING id"#,
    )
    .bind(site_id)
    .bind(icon_id)
    .bind(calendar_day_id)
    .bind(slug)
    .bind(title)
    .bind(icon.prayer_text.trim())
    .bind(status_or_draft(&icon.status))
    .fetch_one(pool)
    .await
    .map_err(db_error)
}

async fn upsert_gospel_from_icon(
    pool: &PgPool,
    site_id: Uuid,
    calendar_day_id: Uuid,
    icon_id: Uuid,
    icon: &super::icons_site::IconPage,
) -> Result<Uuid, StatusCode> {
    let slug = format!("{}-gospel", icon.slug.trim());
    let title = format!("Євангеліє: {}", icon.title.trim());
    sqlx::query_scalar(
        r#"INSERT INTO church_gospel_readings
           (site_id, icon_id, calendar_day_id, slug, title, reference, text, explanation, language, status, is_global)
           VALUES ($1, $2, $3, $4, $5, '', $6, '', 'uk', $7, false)
           ON CONFLICT (site_id, slug, language)
           DO UPDATE SET
              icon_id = EXCLUDED.icon_id,
              calendar_day_id = EXCLUDED.calendar_day_id,
              title = EXCLUDED.title,
              text = EXCLUDED.text,
              status = EXCLUDED.status,
              updated_at = NOW()
           RETURNING id"#,
    )
    .bind(site_id)
    .bind(icon_id)
    .bind(calendar_day_id)
    .bind(slug)
    .bind(title)
    .bind(icon.gospel_text.trim())
    .bind(status_or_draft(&icon.status))
    .fetch_one(pool)
    .await
    .map_err(db_error)
}

async fn upsert_article_from_icon(
    pool: &PgPool,
    site_id: Uuid,
    calendar_day_id: Uuid,
    icon_id: Uuid,
    icon: &super::icons_site::IconPage,
) -> Result<Uuid, StatusCode> {
    let content = [
        icon.full_description.trim(),
        icon.life_text.trim(),
        icon.history_text.trim(),
    ]
    .into_iter()
    .filter(|value| !value.is_empty())
    .collect::<Vec<_>>()
    .join("\n\n");

    sqlx::query_scalar(
        r#"INSERT INTO church_articles
           (site_id, icon_id, calendar_day_id, title, slug, content, language,
            seo_title, seo_description, status, is_global)
           VALUES ($1, $2, $3, $4, $5, $6, 'uk', $7, $8, $9, false)
           ON CONFLICT (site_id, slug, language)
           DO UPDATE SET
              icon_id = EXCLUDED.icon_id,
              calendar_day_id = EXCLUDED.calendar_day_id,
              title = EXCLUDED.title,
              content = EXCLUDED.content,
              seo_title = EXCLUDED.seo_title,
              seo_description = EXCLUDED.seo_description,
              status = EXCLUDED.status,
              updated_at = NOW()
           RETURNING id"#,
    )
    .bind(site_id)
    .bind(icon_id)
    .bind(calendar_day_id)
    .bind(icon.title.trim())
    .bind(icon.slug.trim())
    .bind(content)
    .bind(icon.seo_title.as_deref().unwrap_or_default())
    .bind(icon.seo_description.as_deref().unwrap_or_default())
    .bind(status_or_draft(&icon.status))
    .fetch_one(pool)
    .await
    .map_err(db_error)
}

async fn upsert_article_from_legacy_page(
    pool: &PgPool,
    site_id: Uuid,
    page: &super::icons_site::SeoPage,
) -> Result<Uuid, StatusCode> {
    sqlx::query_scalar(
        r#"INSERT INTO church_articles
           (site_id, icon_id, calendar_day_id, title, slug, content, language,
            seo_title, seo_description, status, is_global)
           VALUES ($1, NULL, NULL, $2, $3, $4, $5, $6, $7, $8, false)
           ON CONFLICT (site_id, slug, language)
           DO UPDATE SET
              title = EXCLUDED.title,
              content = EXCLUDED.content,
              seo_title = EXCLUDED.seo_title,
              seo_description = EXCLUDED.seo_description,
              status = EXCLUDED.status,
              updated_at = NOW()
           RETURNING id"#,
    )
    .bind(site_id)
    .bind(first_non_empty(&[&page.title, &page.h1]))
    .bind(page.slug.trim())
    .bind(page.content.trim())
    .bind(first_non_empty(&[&page.language, "uk"]))
    .bind(page.seo_title.as_deref().unwrap_or_default())
    .bind(page.seo_description.as_deref().unwrap_or_default())
    .bind(status_or_draft(&page.status))
    .fetch_one(pool)
    .await
    .map_err(db_error)
}

async fn public_calendar_by_date(
    pool: &PgPool,
    date: &str,
    language: Option<&str>,
    include_drafts: bool,
) -> Result<impl IntoResponse, StatusCode> {
    let calendar_day: ChurchCalendarDayDto = sqlx::query_as(
        r#"SELECT id, site_id, date_old_style::text AS date_old_style,
                  date_new_style::text AS date_new_style, calendar_type, title, day_type,
                  description, rank, status, is_global,
                  created_at::text AS created_at, updated_at::text AS updated_at
           FROM church_calendar_days
           WHERE (site_id = $1 OR is_global = true)
             AND (date_new_style = $2::date OR date_old_style = $2::date)
             AND ($3::bool OR status = 'published')
           ORDER BY rank DESC, title ASC
           LIMIT 1"#,
    )
    .bind(CHURCH_SITE_ID)
    .bind(date)
    .bind(include_drafts)
    .fetch_optional(pool)
    .await
    .map_err(db_error)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let icons = list_public_icons(pool, calendar_day.id, language, include_drafts).await?;
    let prayers = list_public_prayers(pool, Some(calendar_day.id), None, language, include_drafts).await?;
    let articles = list_public_articles(pool, Some(calendar_day.id), None, language, include_drafts).await?;
    let gospel = list_public_gospel(pool, Some(calendar_day.id), None, language, include_drafts).await?;

    Ok(Json(PublicChurchContentPage {
        calendar_day,
        icons,
        prayers,
        articles,
        gospel,
    }))
}

async fn get_public_calendar_row(
    pool: &PgPool,
    id: Uuid,
    include_drafts: bool,
) -> Result<Option<ChurchCalendarDayDto>, StatusCode> {
    sqlx::query_as(
        r#"SELECT id, site_id, date_old_style::text AS date_old_style,
                  date_new_style::text AS date_new_style, calendar_type, title, day_type,
                  description, rank, status, is_global,
                  created_at::text AS created_at, updated_at::text AS updated_at
           FROM church_calendar_days
           WHERE id = $1
             AND (site_id = $2 OR is_global = true)
             AND ($3::bool OR status = 'published')"#,
    )
    .bind(id)
    .bind(CHURCH_SITE_ID)
    .bind(include_drafts)
    .fetch_optional(pool)
    .await
    .map_err(db_error)
}

pub(crate) async fn get_public_icon_row(
    pool: &PgPool,
    id: Uuid,
    include_drafts: bool,
) -> Result<Option<ChurchIconDto>, StatusCode> {
    sqlx::query_as(
        r#"SELECT id, site_id, calendar_day_id, title, slug, image_url, saint_name, feast_name,
                  description, language, translation_group_id, status, is_global,
                  order_enabled, order_block_text, production_time, price_cents, currency, consecration_available,
                  created_at::text AS created_at, updated_at::text AS updated_at
           FROM church_icons
           WHERE id = $1
             AND (site_id = $2 OR is_global = true)
             AND ($3::bool OR status = 'published')"#,
    )
    .bind(id)
    .bind(CHURCH_SITE_ID)
    .bind(include_drafts)
    .fetch_optional(pool)
    .await
    .map_err(db_error)
}

async fn list_public_icons(
    pool: &PgPool,
    calendar_day_id: Uuid,
    language: Option<&str>,
    include_drafts: bool,
) -> Result<Vec<ChurchIconDto>, StatusCode> {
    sqlx::query_as(
        r#"SELECT id, site_id, calendar_day_id, title, slug, image_url, saint_name, feast_name,
                  description, language, translation_group_id, status, is_global,
                  order_enabled, order_block_text, production_time, price_cents, currency, consecration_available,
                  created_at::text AS created_at, updated_at::text AS updated_at
           FROM church_icons
           WHERE calendar_day_id = $1
             AND (site_id = $2 OR is_global = true)
             AND ($3::bool OR status = 'published')
             AND ($4::text IS NULL OR language = $4)
           ORDER BY title ASC"#,
    )
    .bind(calendar_day_id)
    .bind(CHURCH_SITE_ID)
    .bind(include_drafts)
    .bind(language)
    .fetch_all(pool)
    .await
    .map_err(db_error)
}

async fn list_public_prayers(
    pool: &PgPool,
    calendar_day_id: Option<Uuid>,
    icon_id: Option<Uuid>,
    language: Option<&str>,
    include_drafts: bool,
) -> Result<Vec<ChurchPrayerDto>, StatusCode> {
    sqlx::query_as(
        r#"SELECT id, site_id, icon_id, calendar_day_id, slug, title, text, audio_url, qr_code_url, image_url, source, source_url, note, language, prayer_type, translation_group_id,
                  status, is_global, created_at::text AS created_at, updated_at::text AS updated_at
           FROM church_prayers
           WHERE ($1::uuid IS NULL OR calendar_day_id = $1)
             AND ($2::uuid IS NULL OR icon_id = $2)
             AND (site_id = $3 OR is_global = true)
             AND ($4::bool OR status = 'published')
             AND ($5::text IS NULL OR language = $5)
           ORDER BY prayer_type ASC, title ASC"#,
    )
    .bind(calendar_day_id)
    .bind(icon_id)
    .bind(CHURCH_SITE_ID)
    .bind(include_drafts)
    .bind(language)
    .fetch_all(pool)
    .await
    .map_err(db_error)
}

async fn list_public_articles(
    pool: &PgPool,
    calendar_day_id: Option<Uuid>,
    icon_id: Option<Uuid>,
    language: Option<&str>,
    include_drafts: bool,
) -> Result<Vec<ChurchArticleDto>, StatusCode> {
    sqlx::query_as(
        r#"SELECT id, site_id, icon_id, calendar_day_id, title, slug, content, language,
                  seo_title, seo_description, status, is_global, created_at::text AS created_at, updated_at::text AS updated_at
           FROM church_articles
           WHERE ($1::uuid IS NULL OR calendar_day_id = $1)
             AND ($2::uuid IS NULL OR icon_id = $2)
             AND (site_id = $3 OR is_global = true)
             AND ($4::bool OR status = 'published')
             AND ($5::text IS NULL OR language = $5)
           ORDER BY title ASC"#,
    )
    .bind(calendar_day_id)
    .bind(icon_id)
    .bind(CHURCH_SITE_ID)
    .bind(include_drafts)
    .bind(language)
    .fetch_all(pool)
    .await
    .map_err(db_error)
}

async fn list_public_gospel(
    pool: &PgPool,
    calendar_day_id: Option<Uuid>,
    icon_id: Option<Uuid>,
    language: Option<&str>,
    include_drafts: bool,
) -> Result<Vec<ChurchGospelDto>, StatusCode> {
    sqlx::query_as(
        r#"SELECT id, site_id, icon_id, calendar_day_id, slug, title, reference, text, explanation,
                  language, status, is_global, created_at::text AS created_at, updated_at::text AS updated_at
           FROM church_gospel_readings
           WHERE ($1::uuid IS NULL OR calendar_day_id = $1)
             AND ($2::uuid IS NULL OR icon_id = $2)
             AND (site_id = $3 OR is_global = true)
             AND ($4::bool OR status = 'published')
             AND ($5::text IS NULL OR language = $5)
           ORDER BY title ASC"#,
    )
    .bind(calendar_day_id)
    .bind(icon_id)
    .bind(CHURCH_SITE_ID)
    .bind(include_drafts)
    .bind(language)
    .fetch_all(pool)
    .await
    .map_err(db_error)
}

pub(crate) fn preview_allowed(query: &ChurchContentQuery) -> bool {
    let Some(token) = std::env::var("CHURCH_PREVIEW_TOKEN")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    else {
        return false;
    };

    query.preview_token.as_deref() == Some(token.as_str())
}

fn has_article_content(icon: &super::icons_site::IconPage) -> bool {
    [&icon.full_description, &icon.life_text, &icon.history_text]
        .iter()
        .any(|value| !value.trim().is_empty())
}

fn first_non_empty(values: &[&str]) -> String {
    values
        .iter()
        .map(|value| value.trim())
        .find(|value| !value.is_empty())
        .unwrap_or_default()
        .to_string()
}

fn status_or_draft(status: &str) -> &str {
    if status.trim() == "published" {
        "published"
    } else {
        "draft"
    }
}

pub(crate) fn slugify(value: &str) -> String {
    let normalized = normalize_for_slug(value);
    if normalized.is_empty() {
        "prayer".into()
    } else {
        normalized
    }
}

fn normalize_for_slug(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .map(|ch| if ch.is_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

async fn get_icon_row(
    pool: &PgPool,
    id: Uuid,
    site_id: Uuid,
    include_global: bool,
) -> Result<ChurchIconDto, StatusCode> {
    let predicate = if include_global {
        "id = $1 AND (site_id = $2 OR is_global = true)"
    } else {
        "id = $1 AND site_id = $2"
    };
    let sql = format!(
        r#"SELECT id, site_id, calendar_day_id, title, slug, image_url, saint_name, feast_name,
                  description, language, translation_group_id, status, is_global,
                  order_enabled, order_block_text, production_time, price_cents, currency, consecration_available,
                  created_at::text AS created_at, updated_at::text AS updated_at
           FROM church_icons WHERE {predicate}"#
    );
    sqlx::query_as::<_, ChurchIconDto>(&sql)
        .bind(id)
        .bind(site_id)
        .fetch_optional(pool)
        .await
        .map_err(db_error)?
        .ok_or(StatusCode::NOT_FOUND)
}

pub(crate) async fn delete_owned(
    pool: &PgPool,
    table: &'static str,
    id: Uuid,
    site_id: Uuid,
) -> Result<(), StatusCode> {
    let sql = format!("DELETE FROM {table} WHERE id = $1 AND site_id = $2");
    let result = sqlx::query(&sql)
        .bind(id)
        .bind(site_id)
        .execute(pool)
        .await
        .map_err(db_error)?;

    if result.rows_affected() == 0 {
        Err(StatusCode::NOT_FOUND)
    } else {
        Ok(())
    }
}

pub(crate) fn required(value: Option<String>, _field: &'static str) -> Result<String, StatusCode> {
    optional_non_empty(value).ok_or(StatusCode::BAD_REQUEST)
}

pub(crate) fn optional_non_empty(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(crate) fn db_error(error: sqlx::Error) -> StatusCode {
    tracing::error!(%error, "church content database error");
    StatusCode::INTERNAL_SERVER_ERROR
}
