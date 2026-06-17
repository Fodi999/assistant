use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Row};
use std::sync::Arc;

use crate::{
    infrastructure::{llm_adapter::LlmAdapter, R2Client},
    shared::AppError,
};

const SITE_KEY: &str = "almabuild";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterialCategory {
    pub index: String,
    pub slug: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_ru: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_kk: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_en: Option<String>,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_ru: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_kk: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_en: Option<String>,
    pub bullets: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bullets_ru: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bullets_kk: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bullets_en: Vec<String>,
    pub photo: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail_image_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category_slug: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub availability: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supplier: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purchase_price: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purchase_currency: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sale_price: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sale_currency: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_percent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub languages: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seo_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seo_description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Product {
    pub category_slug: String,
    pub category: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category_ru: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category_kk: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category_en: Option<String>,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_ru: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_kk: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_en: Option<String>,
    pub spec: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec_ru: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec_kk: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec_en: Option<String>,
    pub photo: String,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Kit {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_ru: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_kk: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_en: Option<String>,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_ru: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_kk: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_en: Option<String>,
    pub items: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items_ru: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items_kk: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items_en: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub slug: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_ru: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_kk: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_en: Option<String>,
    pub meta: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta_ru: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta_kk: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta_en: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seo_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seo_title_ru: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seo_title_kk: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seo_title_en: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seo_description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seo_description_ru: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seo_description_kk: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seo_description_en: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_title_ru: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_title_kk: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_title_en: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_text_ru: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_text_kk: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_text_en: Option<String>,
    pub photo: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub image_urls: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlmabuildContent {
    pub material_categories: Vec<MaterialCategory>,
    pub products: Vec<Product>,
    pub kits: Vec<Kit>,
    pub projects: Vec<Project>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Lead {
    pub id: String,
    pub created_at: String,
    pub name: String,
    pub phone: String,
    #[serde(rename = "type")]
    pub object_type: String,
    pub area: String,
    pub comment: String,
    pub items: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateLeadRequest {
    pub name: Option<String>,
    pub phone: String,
    #[serde(rename = "type")]
    pub object_type: Option<String>,
    pub area: Option<String>,
    pub comment: Option<String>,
    pub items: Option<Vec<String>>,
}

fn clean_string(value: Option<String>) -> String {
    value.unwrap_or_default().trim().to_string()
}

fn clean_items(value: Option<Vec<String>>) -> Vec<String> {
    value
        .unwrap_or_default()
        .into_iter()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

fn non_empty(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn default_content() -> AlmabuildContent {
    AlmabuildContent {
        material_categories: vec![
            MaterialCategory { index: "[0:1]".into(), slug: "gipsokarton-profili".into(), title: "Гипсокартон и профили".into(), title_ru: None, title_kk: None, title_en: None, text: "Листы ГКЛ, направляющие и стоечные профили, подвесы, крепёж и комплектующие для перегородок и потолков.".into(), text_ru: None, text_kk: None, text_en: None, bullets: vec!["Листы ГКЛ".into(), "Профили и направляющие".into(), "Подвесы и крепёж".into(), "Комплектующие".into()], bullets_ru: vec![], bullets_kk: vec![], bullets_en: vec![], photo: "material-drywall".into(), image_url: None, detail_image_url: None, price: None, category_slug: None, unit: None, availability: None, city: Some("Алматы".into()), supplier: None, purchase_price: None, purchase_currency: Some("KZT".into()), sale_price: None, sale_currency: Some("KZT".into()), margin_percent: None, status: Some("published".into()), languages: vec!["RU".into()], seo_title: None, seo_description: None },
            MaterialCategory { index: "[0:2]".into(), slug: "sukhie-smesi".into(), title: "Сухие смеси".into(), title_ru: None, title_kk: None, title_en: None, text: "Штукатурка, шпаклёвка, наливные полы, плиточный клей, грунтовки и расходные материалы.".into(), text_ru: None, text_kk: None, text_en: None, bullets: vec!["Штукатурки и шпаклёвки".into(), "Плиточный клей".into(), "Наливные полы".into(), "Грунтовки и добавки".into()], bullets_ru: vec![], bullets_kk: vec![], bullets_en: vec![], photo: "material-mixes".into(), image_url: None, detail_image_url: None, price: None, category_slug: None, unit: None, availability: None, city: Some("Алматы".into()), supplier: None, purchase_price: None, purchase_currency: Some("KZT".into()), sale_price: None, sale_currency: Some("KZT".into()), margin_percent: None, status: Some("published".into()), languages: vec!["RU".into()], seo_title: None, seo_description: None },
            MaterialCategory { index: "[0:3]".into(), slug: "poly-plitka".into(), title: "Полы и плитка".into(), title_ru: None, title_kk: None, title_en: None, text: "Керамогранит, плитка, кварцвинил, ламинат, плинтусы, затирка и материалы для укладки.".into(), text_ru: None, text_kk: None, text_en: None, bullets: vec!["Керамогранит и плитка".into(), "Кварцвинил и ламинат".into(), "Плинтусы и пороги".into(), "Затирки и клеи".into()], bullets_ru: vec![], bullets_kk: vec![], bullets_en: vec![], photo: "material-flooring".into(), image_url: None, detail_image_url: None, price: None, category_slug: None, unit: None, availability: None, city: Some("Алматы".into()), supplier: None, purchase_price: None, purchase_currency: Some("KZT".into()), sale_price: None, sale_currency: Some("KZT".into()), margin_percent: None, status: Some("published".into()), languages: vec!["RU".into()], seo_title: None, seo_description: None },
            MaterialCategory { index: "[0:4]".into(), slug: "elektrika-osveshchenie".into(), title: "Электрика и освещение".into(), title_ru: None, title_kk: None, title_en: None, text: "Кабель, автоматы, розетки, трековое освещение, светильники и LED-решения для магазинов.".into(), text_ru: None, text_kk: None, text_en: None, bullets: vec!["Кабель и провода".into(), "Автоматы и щиты".into(), "Розетки и выключатели".into(), "Светильники и LED-решения".into()], bullets_ru: vec![], bullets_kk: vec![], bullets_en: vec![], photo: "material-electric".into(), image_url: None, detail_image_url: None, price: None, category_slug: None, unit: None, availability: None, city: Some("Алматы".into()), supplier: None, purchase_price: None, purchase_currency: Some("KZT".into()), sale_price: None, sale_currency: Some("KZT".into()), margin_percent: None, status: Some("published".into()), languages: vec!["RU".into()], seo_title: None, seo_description: None },
            MaterialCategory { index: "[0:5]".into(), slug: "potolochnye-sistemy".into(), title: "Потолочные системы".into(), title_ru: None, title_kk: None, title_en: None, text: "Армстронг, грильято, гипсокартонные потолки, подвесные системы и комплектующие.".into(), text_ru: None, text_kk: None, text_en: None, bullets: vec!["Армстронг и грильято".into(), "Гипсокартонные потолки".into(), "Подвесные системы".into(), "Комплектующие".into()], bullets_ru: vec![], bullets_kk: vec![], bullets_en: vec![], photo: "material-ceiling".into(), image_url: None, detail_image_url: None, price: None, category_slug: None, unit: None, availability: None, city: Some("Алматы".into()), supplier: None, purchase_price: None, purchase_currency: Some("KZT".into()), sale_price: None, sale_currency: Some("KZT".into()), margin_percent: None, status: Some("published".into()), languages: vec!["RU".into()], seo_title: None, seo_description: None },
            MaterialCategory { index: "[0:6]".into(), slug: "osb-fanera-uteplitel".into(), title: "OSB, фанера и утеплитель".into(), title_ru: None, title_kk: None, title_en: None, text: "OSB, фанера, минеральная вата, гидроизоляция, мембраны и теплоизоляционные материалы.".into(), text_ru: None, text_kk: None, text_en: None, bullets: vec!["OSB и фанера".into(), "Минеральная вата".into(), "Гидроизоляция".into(), "Мембраны и плёнки".into()], bullets_ru: vec![], bullets_kk: vec![], bullets_en: vec![], photo: "material-osb".into(), image_url: None, detail_image_url: None, price: None, category_slug: None, unit: None, availability: None, city: Some("Алматы".into()), supplier: None, purchase_price: None, purchase_currency: Some("KZT".into()), sale_price: None, sale_currency: Some("KZT".into()), margin_percent: None, status: Some("published".into()), languages: vec!["RU".into()], seo_title: None, seo_description: None },
        ],
        products: vec![
            Product { category_slug: "gipsokarton-profili".into(), category: "ГКЛ".into(), category_ru: None, category_kk: None, category_en: None, title: "ГКЛ 12.5 мм стандартный".into(), title_ru: None, title_kk: None, title_en: None, spec: "2500x1200 мм · стены и потолки".into(), spec_ru: None, spec_kk: None, spec_en: None, photo: "photo-plans".into() },
            Product { category_slug: "gipsokarton-profili".into(), category: "ГКЛ".into(), category_ru: None, category_kk: None, category_en: None, title: "ГКЛ влагостойкий 12.5 мм".into(), title_ru: None, title_kk: None, title_en: None, spec: "Для влажных зон и аптек".into(), spec_ru: None, spec_kk: None, spec_en: None, photo: "photo-plans".into() },
            Product { category_slug: "gipsokarton-profili".into(), category: "Профили".into(), category_ru: None, category_kk: None, category_en: None, title: "Профиль стоечный CW".into(), title_ru: None, title_kk: None, title_en: None, spec: "50/75/100 мм · перегородки".into(), spec_ru: None, spec_kk: None, spec_en: None, photo: "photo-building".into() },
            Product { category_slug: "sukhie-smesi".into(), category: "Сухие смеси".into(), category_ru: None, category_kk: None, category_en: None, title: "Плиточный клей усиленный".into(), title_ru: None, title_kk: None, title_en: None, spec: "Для керамогранита и плитки".into(), spec_ru: None, spec_kk: None, spec_en: None, photo: "photo-retail".into() },
        ],
        kits: vec![
            Kit { title: "Комплект для перегородок".into(), title_ru: None, title_kk: None, title_en: None, text: "Каркас, листы, крепёж и расходники.".into(), text_ru: None, text_kk: None, text_en: None, items: vec!["ГКЛ".into(), "CW/UW профили".into(), "Подвесы и саморезы".into()], items_ru: vec![], items_kk: vec![], items_en: vec![] },
            Kit { title: "Комплект для потолка".into(), title_ru: None, title_kk: None, title_en: None, text: "Система под монтаж потолков.".into(), text_ru: None, text_kk: None, text_en: None, items: vec!["Профили".into(), "Подвесы".into(), "Плиты / ГКЛ".into()], items_ru: vec![], items_kk: vec![], items_en: vec![] },
        ],
        projects: vec![
            Project { slug: "".into(), title: "BUTIK KZ".into(), title_ru: None, title_kk: None, title_en: None, meta: "Магазин одежды · 320 м² · 28 дней".into(), meta_ru: None, meta_kk: None, meta_en: None, photo: "photo-retail".into(), seo_title: None, seo_title_ru: None, seo_title_kk: None, seo_title_en: None, seo_description: None, seo_description_ru: None, seo_description_kk: None, seo_description_en: None, page_title: None, page_title_ru: None, page_title_kk: None, page_title_en: None, page_text: None, page_text_ru: None, page_text_kk: None, page_text_en: None, image_urls: Vec::new() },
            Project { slug: "".into(), title: "Green Mart".into(), title_ru: None, title_kk: None, title_en: None, meta: "Супермаркет · 1250 м² · 45 дней".into(), meta_ru: None, meta_kk: None, meta_en: None, photo: "photo-office".into(), seo_title: None, seo_title_ru: None, seo_title_kk: None, seo_title_en: None, seo_description: None, seo_description_ru: None, seo_description_kk: None, seo_description_en: None, page_title: None, page_title_ru: None, page_title_kk: None, page_title_en: None, page_text: None, page_text_ru: None, page_text_kk: None, page_text_en: None, image_urls: Vec::new() },
            Project { slug: "".into(), title: "Europharma".into(), title_ru: None, title_kk: None, title_en: None, meta: "Аптека · 110 м² · 18 дней".into(), meta_ru: None, meta_kk: None, meta_en: None, photo: "photo-building".into(), seo_title: None, seo_title_ru: None, seo_title_kk: None, seo_title_en: None, seo_description: None, seo_description_ru: None, seo_description_kk: None, seo_description_en: None, page_title: None, page_title_ru: None, page_title_kk: None, page_title_en: None, page_text: None, page_text_ru: None, page_text_kk: None, page_text_en: None, image_urls: Vec::new() },
        ],
    }
}

async fn load_content(pool: &PgPool) -> Result<AlmabuildContent, StatusCode> {
    let row: Option<Value> = sqlx::query_scalar("SELECT content FROM site_content WHERE site = $1")
        .bind(SITE_KEY)
        .fetch_optional(pool)
        .await
        .map_err(|error| {
            tracing::error!(%error, "failed to load almabuild content");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    match row {
        Some(value) => serde_json::from_value(value).map_err(|error| {
            tracing::error!(%error, "invalid almabuild content json");
            StatusCode::INTERNAL_SERVER_ERROR
        }),
        None => Ok(default_content()),
    }
}

pub async fn public_content(State(pool): State<PgPool>) -> Result<impl IntoResponse, StatusCode> {
    Ok(Json(load_content(&pool).await?))
}

pub async fn admin_get_content(
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    Ok(Json(load_content(&pool).await?))
}

pub async fn admin_put_content(
    State(pool): State<PgPool>,
    Json(content): Json<AlmabuildContent>,
) -> Result<impl IntoResponse, StatusCode> {
    let value = serde_json::to_value(&content).map_err(|error| {
        tracing::error!(%error, "failed to serialize almabuild content");
        StatusCode::BAD_REQUEST
    })?;

    sqlx::query(
        r#"
        INSERT INTO site_content (site, content, updated_at)
        VALUES ($1, $2, NOW())
        ON CONFLICT (site)
        DO UPDATE SET content = EXCLUDED.content, updated_at = NOW()
        "#,
    )
    .bind(SITE_KEY)
    .bind(value)
    .execute(&pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "failed to save almabuild content");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(content))
}

fn lead_from_row(row: sqlx::postgres::PgRow) -> Result<Lead, StatusCode> {
    Ok(Lead {
        id: row
            .try_get::<uuid::Uuid, _>("id")
            .map_err(|error| {
                tracing::error!(%error, "failed to decode lead id");
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .to_string(),
        created_at: row.try_get("created_at").map_err(|error| {
            tracing::error!(%error, "failed to decode lead created_at");
            StatusCode::INTERNAL_SERVER_ERROR
        })?,
        name: row.try_get("name").unwrap_or_default(),
        phone: row.try_get("phone").unwrap_or_default(),
        object_type: row.try_get("object_type").unwrap_or_default(),
        area: row.try_get("area").unwrap_or_default(),
        comment: row.try_get("comment").unwrap_or_default(),
        items: row.try_get("items").unwrap_or_default(),
    })
}

pub async fn public_create_lead(
    State(pool): State<PgPool>,
    Json(payload): Json<CreateLeadRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let phone = payload.phone.trim().to_string();
    if phone.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let row = sqlx::query(
        r#"
        INSERT INTO site_leads (site, name, phone, object_type, area, comment, items)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at,
                  name, phone, object_type, area, comment, items
        "#,
    )
    .bind(SITE_KEY)
    .bind(clean_string(payload.name))
    .bind(phone)
    .bind(clean_string(payload.object_type))
    .bind(clean_string(payload.area))
    .bind(clean_string(payload.comment))
    .bind(clean_items(payload.items))
    .fetch_one(&pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "failed to create almabuild lead");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok((StatusCode::CREATED, Json(lead_from_row(row)?)))
}

pub async fn admin_get_leads(State(pool): State<PgPool>) -> Result<impl IntoResponse, StatusCode> {
    let rows = sqlx::query(
        r#"
        SELECT id, to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at,
               name, phone, object_type, area, comment, items
        FROM site_leads
        WHERE site = $1
        ORDER BY created_at DESC
        LIMIT 500
        "#,
    )
    .bind(SITE_KEY)
    .fetch_all(&pool)
    .await
    .map_err(|error| {
        tracing::error!(%error, "failed to load almabuild leads");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let leads = rows
        .into_iter()
        .map(lead_from_row)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(leads))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiEditRequest {
    pub kind: String,
    pub instruction: String,
    pub value: Value,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterialsFromPhotoResponse {
    pub materials: Vec<MaterialCategory>,
}

fn strip_json_fence(raw: &str) -> &str {
    let trimmed = raw.trim();
    let fence_json = "`".repeat(3) + "json";
    let fence = "`".repeat(3);
    if let Some(rest) = trimmed.strip_prefix(&fence_json) {
        return rest.trim().trim_end_matches(&fence).trim();
    }
    if let Some(rest) = trimmed.strip_prefix(&fence) {
        return rest.trim().trim_end_matches(&fence).trim();
    }
    trimmed
}

fn normalize_gemini_materials_value(mut value: Value) -> Value {
    if let Some(materials) = value.get_mut("materials") {
        if let Some(object) = materials.as_object() {
            *materials = Value::Array(object.values().cloned().collect());
        }
        if let Some(array) = materials.as_array_mut() {
            for material in array {
                if let Some(object) = material.as_object_mut() {
                    if let Some(languages) = object.get_mut("languages") {
                        if let Some(language) = languages.as_str() {
                            *languages = Value::Array(
                                language
                                    .split(',')
                                    .map(|item| Value::String(item.trim().to_string()))
                                    .filter(|item| {
                                        item.as_str().is_some_and(|text| !text.is_empty())
                                    })
                                    .collect(),
                            );
                        } else if let Some(map) = languages.as_object() {
                            *languages = Value::Array(
                                map.iter()
                                    .filter(|(_, enabled)| enabled.as_bool().unwrap_or(true))
                                    .map(|(key, _)| Value::String(key.to_string()))
                                    .collect(),
                            );
                        }
                    }
                    if !object.contains_key("languages") {
                        object.insert(
                            "languages".to_string(),
                            Value::Array(vec![Value::String("RU".to_string())]),
                        );
                    }
                }
            }
        }
    } else if value.is_array() {
        value = serde_json::json!({ "materials": value });
    }
    value
}

fn almabuild_schema(kind: &str) -> &'static str {
    match kind {
        "material" => {
            r#"{"index":"[0:1]","slug":"slug","title":"Название","text":"Описание","bullets":["Пункт"],"photo":"material-class"}"#
        }
        "product" => {
            r#"{"categorySlug":"slug","category":"Категория","title":"Название","spec":"Характеристики","photo":"photo-class"}"#
        }
        "kit" => r#"{"title":"Название","text":"Описание","items":["Позиция"]}"#,
        "project" => r#"{"slug":"latin-slug","title":"Название","meta":"Тип · площадь · срок","seoTitle":"SEO title","seoDescription":"SEO description","pageTitle":"Заголовок SEO-страницы","pageText":"Текст SEO-страницы","photo":"photo-class","imageUrls":[]}"#,
        _ => r#"{}"#,
    }
}

fn validate_ai_value(kind: &str, value: Value) -> Result<Value, AppError> {
    match kind {
        "material" => {
            let parsed: MaterialCategory = serde_json::from_value(value).map_err(|e| {
                AppError::validation(format!("Gemini вернул неверную карточку материала: {e}"))
            })?;
            serde_json::to_value(parsed)
                .map_err(|e| AppError::internal(format!("serialize material: {e}")))
        }
        "product" => {
            let parsed: Product = serde_json::from_value(value).map_err(|e| {
                AppError::validation(format!("Gemini вернул неверную карточку товара: {e}"))
            })?;
            serde_json::to_value(parsed)
                .map_err(|e| AppError::internal(format!("serialize product: {e}")))
        }
        "kit" => {
            let parsed: Kit = serde_json::from_value(value).map_err(|e| {
                AppError::validation(format!("Gemini вернул неверный комплект: {e}"))
            })?;
            serde_json::to_value(parsed)
                .map_err(|e| AppError::internal(format!("serialize kit: {e}")))
        }
        "project" => {
            let parsed: Project = serde_json::from_value(value)
                .map_err(|e| AppError::validation(format!("Gemini вернул неверный проект: {e}")))?;
            serde_json::to_value(parsed)
                .map_err(|e| AppError::internal(format!("serialize project: {e}")))
        }
        _ => Err(AppError::validation(
            "Неизвестный тип карточки для AI редактирования",
        )),
    }
}

pub async fn admin_ai_edit(
    Extension(llm): Extension<Arc<LlmAdapter>>,
    Json(req): Json<AiEditRequest>,
) -> Result<Json<Value>, AppError> {
    let schema = almabuild_schema(&req.kind);
    if schema == "{}" {
        return Err(AppError::validation(
            "Неизвестный тип карточки для AI редактирования",
        ));
    }

    let prompt = format!(
        r#"Ты редактируешь контент строительного сайта KAZAXBUD / ALMABUILD.
Верни ТОЛЬКО JSON без markdown и пояснений.
Тип карточки: {kind}
Схема результата: {schema}

Задача администратора:
{instruction}

Текущая карточка JSON:
{value}

Правила:
- Пиши профессионально на русском, если исходный текст русский.
- Не меняй смысл, если администратор не попросил.
- Сохрани все поля схемы.
- slug/categorySlug/photo оставляй стабильными, если администратор явно не просит изменить.
- bullets/items должны быть массивом коротких пунктов.
"#,
        kind = req.kind,
        schema = schema,
        instruction = req.instruction,
        value = req.value
    );

    let raw = llm
        .groq_raw_request_with_model(&prompt, 4000, "gemini-3.1-pro-preview")
        .await?;
    let value: Value = serde_json::from_str(strip_json_fence(&raw))
        .map_err(|e| AppError::internal(format!("Gemini вернул не JSON: {e}")))?;
    Ok(Json(validate_ai_value(&req.kind, value)?))
}

pub async fn admin_ai_materials_from_photo(
    Extension(llm): Extension<Arc<LlmAdapter>>,
    Extension(r2): Extension<R2Client>,
    mut multipart: Multipart,
) -> Result<Json<MaterialsFromPhotoResponse>, AppError> {
    let mut image: Option<bytes::Bytes> = None;
    let mut mime_type = "image/jpeg".to_string();
    let mut detail_image: Option<bytes::Bytes> = None;
    let mut detail_mime_type = "image/jpeg".to_string();
    let mut count = 1usize;
    let mut instruction = String::new();
    let mut existing_count = 0usize;
    let mut existing = String::new();
    let mut price = String::new();
    let mut category_slug = String::new();
    let mut category_title = String::new();
    let mut unit = String::new();
    let mut availability = String::new();
    let mut city = "Алматы".to_string();
    let mut supplier = String::new();
    let mut purchase_price = String::new();
    let mut purchase_currency = "KZT".to_string();
    let mut sale_price = String::new();
    let mut sale_currency = "KZT".to_string();
    let mut margin_percent = String::new();
    let mut status = "draft".to_string();
    let mut languages = vec!["RU".to_string()];
    let mut seo_title = String::new();
    let mut seo_description = String::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::validation(format!("Invalid multipart data: {e}")))?
    {
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "image" | "file" => {
                mime_type = field
                    .content_type()
                    .filter(|value| value.starts_with("image/"))
                    .unwrap_or("image/jpeg")
                    .to_string();
                image = Some(field.bytes().await.map_err(|e| {
                    AppError::validation(format!("Failed to read uploaded image: {e}"))
                })?);
            }
            "detailImage" | "detailFile" => {
                detail_mime_type = field
                    .content_type()
                    .filter(|value| value.starts_with("image/"))
                    .unwrap_or("image/jpeg")
                    .to_string();
                detail_image = Some(field.bytes().await.map_err(|e| {
                    AppError::validation(format!("Failed to read detail image: {e}"))
                })?);
            }
            "count" => {
                let value = field.text().await.unwrap_or_default();
                count = value.parse::<usize>().unwrap_or(1).clamp(1, 12);
            }
            "instruction" => {
                instruction = field.text().await.unwrap_or_default();
            }
            "existingCount" => {
                let value = field.text().await.unwrap_or_default();
                existing_count = value.parse::<usize>().unwrap_or(0);
            }
            "existing" => {
                existing = field.text().await.unwrap_or_default();
            }
            "price" => {
                price = field.text().await.unwrap_or_default();
            }
            "categorySlug" => {
                category_slug = field.text().await.unwrap_or_default();
            }
            "categoryTitle" => {
                category_title = field.text().await.unwrap_or_default();
            }
            "unit" => unit = field.text().await.unwrap_or_default(),
            "availability" => availability = field.text().await.unwrap_or_default(),
            "city" => city = field.text().await.unwrap_or_else(|_| "Алматы".to_string()),
            "supplier" => supplier = field.text().await.unwrap_or_default(),
            "purchasePrice" => purchase_price = field.text().await.unwrap_or_default(),
            "purchaseCurrency" => {
                purchase_currency = field.text().await.unwrap_or_else(|_| "KZT".to_string())
            }
            "salePrice" => sale_price = field.text().await.unwrap_or_default(),
            "saleCurrency" => {
                sale_currency = field.text().await.unwrap_or_else(|_| "KZT".to_string())
            }
            "marginPercent" => margin_percent = field.text().await.unwrap_or_default(),
            "status" => status = field.text().await.unwrap_or_else(|_| "draft".to_string()),
            "languages" => {
                let value = field.text().await.unwrap_or_default();
                languages = value
                    .split(',')
                    .map(|item| item.trim().to_string())
                    .filter(|item| !item.is_empty())
                    .collect();
            }
            "seoTitle" => seo_title = field.text().await.unwrap_or_default(),
            "seoDescription" => seo_description = field.text().await.unwrap_or_default(),
            _ => {}
        }
    }

    let image = image
        .ok_or_else(|| AppError::validation("Загрузите фото материала в поле image или file"))?;

    let extension = match mime_type.as_str() {
        "image/png" => "png",
        "image/webp" => "webp",
        "image/gif" => "gif",
        "image/jpeg" | "image/jpg" => "jpg",
        _ => "jpg",
    };
    let image_key = format!("almabuild/materials/{}.{}", uuid::Uuid::new_v4(), extension);
    let image_url = r2
        .upload_image(&image_key, image.clone(), &mime_type)
        .await?;
    let detail_image_url = if let Some(detail_image) = detail_image.clone() {
        let extension = match detail_mime_type.as_str() {
            "image/png" => "png",
            "image/webp" => "webp",
            "image/gif" => "gif",
            "image/jpeg" | "image/jpg" => "jpg",
            _ => "jpg",
        };
        let key = format!(
            "almabuild/materials/detail-{}.{}",
            uuid::Uuid::new_v4(),
            extension
        );
        Some(
            r2.upload_image(&key, detail_image, &detail_mime_type)
                .await?,
        )
    } else {
        None
    };

    let prompt = format!(
        r#"Ты Vision-модель для CRM строительного сайта KAZAXBUD / ALMABUILD в Алматы.
Проанализируй загруженное фото: упаковки, листы, профили, смеси, плитку, инструмент, складскую полку или группу стройматериалов.
Создай ровно {count} карточек категорий/материалов для секции "Материалы" сайта.

Верни ТОЛЬКО JSON без markdown в формате:
{{"materials":[{{"index":"[0:7]","slug":"latin-slug","title":"Название","text":"Короткое B2B-описание","bullets":["Пункт"],"photo":"material-mixes","imageUrl":"{image_url}","detailImageUrl":"{detail_image_url}","price":"{price}","categorySlug":"{category_slug}","unit":"{unit}","availability":"{availability}","city":"{city}","supplier":"{supplier}","purchasePrice":"{purchase_price}","purchaseCurrency":"{purchase_currency}","salePrice":"{sale_price}","saleCurrency":"{sale_currency}","marginPercent":"{margin_percent}","status":"{status}","languages":["RU"],"seoTitle":"SEO title","seoDescription":"SEO description"}}]}}

Правила:
- Язык: русский.
- title: понятное название категории/материала, не длиннее 44 символов.
- text: 1 предложение для закупщика, наличие/объём/доставка/монтаж, без фантазий о бренде если на фото его не видно.
- bullets: 3-5 коротких пунктов, каждый до 32 символов.
- slug: латиница lower-case через дефис, уникальный.
- index начинай с [0:{first_index}] и продолжай по порядку.
- photo выбери один класс: material-drywall, material-mixes, material-flooring, material-electric, material-ceiling, material-osb.
- imageUrl можешь оставить пустым: backend заменит его отдельным сгенерированным фото для каждой карточки.
- detailImageUrl всегда ставь ровно: {detail_image_url}
- price всегда ставь ровно: {price}
- categorySlug всегда ставь ровно: {category_slug}; категория: {category_title}
- unit: {unit}; availability: {availability}; city: {city}; supplier: {supplier}
- purchasePrice: {purchase_price}; purchaseCurrency: {purchase_currency}; salePrice: {sale_price}; saleCurrency: {sale_currency}; marginPercent: {margin_percent}
- status: {status}; languages: {languages}; seoTitle/seoDescription заполни для SEO, если администратор не задал их.
- Если фото содержит конкретные товары, группируй их в полезные категории сайта, а не в случайные одиночные позиции.
- Первое фото — основное. Второе фото, если есть, используй как детальный референс фактуры/упаковки/строительного материала.
- Не повторяй уже существующие карточки, если можно сделать новые или уточнённые.

Текущее количество карточек: {existing_count}
Текущие карточки JSON:
{existing}

Задача администратора:
{instruction}
"#,
        count = count,
        image_url = image_url,
        detail_image_url = detail_image_url.as_deref().unwrap_or(""),
        price = price.trim(),
        category_slug = category_slug.trim(),
        category_title = category_title.trim(),
        unit = unit.trim(),
        availability = availability.trim(),
        city = city.trim(),
        supplier = supplier.trim(),
        purchase_price = purchase_price.trim(),
        purchase_currency = purchase_currency.trim(),
        sale_price = sale_price.trim(),
        sale_currency = sale_currency.trim(),
        margin_percent = margin_percent.trim(),
        status = status.trim(),
        languages = languages.join(", "),
        first_index = existing_count + 1,
        existing_count = existing_count,
        existing = if existing.trim().is_empty() {
            "[]"
        } else {
            existing.trim()
        },
        instruction = if instruction.trim().is_empty() {
            "Создай материалы по фото для каталога стройматериалов."
        } else {
            instruction.trim()
        }
    );

    let raw = if let Some(detail_image) = detail_image.as_ref() {
        llm.analyze_images_json(
            &prompt,
            &[
                (image.as_ref(), mime_type.as_str()),
                (detail_image.as_ref(), detail_mime_type.as_str()),
            ],
        )
        .await?
    } else {
        llm.analyze_image_json(&prompt, &image, &mime_type).await?
    };
    let value: Value = serde_json::from_str(strip_json_fence(&raw))
        .map_err(|e| AppError::internal(format!("Gemini Vision вернул не JSON: {e}")))?;
    let response: MaterialsFromPhotoResponse =
        serde_json::from_value(normalize_gemini_materials_value(value)).map_err(|e| {
            AppError::validation(format!("Gemini Vision вернул неверные материалы: {e}"))
        })?;
    if response.materials.is_empty() {
        return Err(AppError::validation(
            "Gemini Vision не вернул ни одной карточки материала",
        ));
    }
    let scenes = [
        "building-materials retail store aisle with gypsum boards and profiles selected for shop renovation",
        "commercial interior construction site where workers install gypsum board partitions and ceiling framing",
        "close realistic process scene of drywall sheet cutting, screw fastening, joint preparation and clean tools",
        "finished shop fit-out interior under construction with gypsum ceiling, wall lining, profiles and materials staged",
        "warehouse loading area with gypsum sheets prepared for delivery to a commercial renovation project",
        "detail scene of moisture-resistant drywall boards being mounted in a clean modern retail space",
    ];
    let mut materials = Vec::new();
    for (index, mut material) in response.materials.into_iter().take(count).enumerate() {
        let scene = scenes[index % scenes.len()];
        let generated_url = match llm
            .generate_material_scene_image(&material.title, &material.text, scene)
            .await
        {
            Ok(base64_png) => {
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(base64_png)
                    .map_err(|e| {
                        AppError::internal(format!("Gemini image base64 decode failed: {e}"))
                    })?;
                let key = format!(
                    "almabuild/materials/generated-{}-{}.png",
                    material.slug,
                    uuid::Uuid::new_v4()
                );
                r2.upload_image(&key, bytes::Bytes::from(bytes), "image/png")
                    .await
                    .unwrap_or_else(|error| {
                        tracing::error!(%error, "failed to upload generated material image to R2");
                        image_url.clone()
                    })
            }
            Err(error) => {
                tracing::error!(%error, title = %material.title, "failed to generate unique material scene image");
                image_url.clone()
            }
        };
        material.image_url = Some(generated_url);
        material.detail_image_url = detail_image_url.clone();
        if !price.trim().is_empty() {
            material.price = Some(price.trim().to_string());
        }
        if !category_slug.trim().is_empty() {
            material.category_slug = Some(category_slug.trim().to_string());
        }
        material.unit = non_empty(&unit);
        material.availability = non_empty(&availability);
        material.city = non_empty(&city);
        material.supplier = non_empty(&supplier);
        material.purchase_price = non_empty(&purchase_price);
        material.purchase_currency = non_empty(&purchase_currency);
        material.sale_price = non_empty(&sale_price);
        material.sale_currency = non_empty(&sale_currency);
        material.margin_percent = non_empty(&margin_percent);
        material.status = non_empty(&status);
        material.languages = languages.clone();
        if !seo_title.trim().is_empty() {
            material.seo_title = Some(seo_title.trim().to_string());
        }
        if !seo_description.trim().is_empty() {
            material.seo_description = Some(seo_description.trim().to_string());
        }
        materials.push(material);
    }

    Ok(Json(MaterialsFromPhotoResponse { materials }))
}
