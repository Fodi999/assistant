use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
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
    pub text: String,
    pub bullets: Vec<String>,
    pub photo: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Product {
    pub category_slug: String,
    pub category: String,
    pub title: String,
    pub spec: String,
    pub photo: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Kit {
    pub title: String,
    pub text: String,
    pub items: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub title: String,
    pub meta: String,
    pub photo: String,
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

fn default_content() -> AlmabuildContent {
    AlmabuildContent {
        material_categories: vec![
            MaterialCategory { index: "[0:1]".into(), slug: "gipsokarton-profili".into(), title: "Гипсокартон и профили".into(), text: "Листы ГКЛ, направляющие и стоечные профили, подвесы, крепёж и комплектующие для перегородок и потолков.".into(), bullets: vec!["Листы ГКЛ".into(), "Профили и направляющие".into(), "Подвесы и крепёж".into(), "Комплектующие".into()], photo: "material-drywall".into(), image_url: None },
            MaterialCategory { index: "[0:2]".into(), slug: "sukhie-smesi".into(), title: "Сухие смеси".into(), text: "Штукатурка, шпаклёвка, наливные полы, плиточный клей, грунтовки и расходные материалы.".into(), bullets: vec!["Штукатурки и шпаклёвки".into(), "Плиточный клей".into(), "Наливные полы".into(), "Грунтовки и добавки".into()], photo: "material-mixes".into(), image_url: None },
            MaterialCategory { index: "[0:3]".into(), slug: "poly-plitka".into(), title: "Полы и плитка".into(), text: "Керамогранит, плитка, кварцвинил, ламинат, плинтусы, затирка и материалы для укладки.".into(), bullets: vec!["Керамогранит и плитка".into(), "Кварцвинил и ламинат".into(), "Плинтусы и пороги".into(), "Затирки и клеи".into()], photo: "material-flooring".into(), image_url: None },
            MaterialCategory { index: "[0:4]".into(), slug: "elektrika-osveshchenie".into(), title: "Электрика и освещение".into(), text: "Кабель, автоматы, розетки, трековое освещение, светильники и LED-решения для магазинов.".into(), bullets: vec!["Кабель и провода".into(), "Автоматы и щиты".into(), "Розетки и выключатели".into(), "Светильники и LED-решения".into()], photo: "material-electric".into(), image_url: None },
            MaterialCategory { index: "[0:5]".into(), slug: "potolochnye-sistemy".into(), title: "Потолочные системы".into(), text: "Армстронг, грильято, гипсокартонные потолки, подвесные системы и комплектующие.".into(), bullets: vec!["Армстронг и грильято".into(), "Гипсокартонные потолки".into(), "Подвесные системы".into(), "Комплектующие".into()], photo: "material-ceiling".into(), image_url: None },
            MaterialCategory { index: "[0:6]".into(), slug: "osb-fanera-uteplitel".into(), title: "OSB, фанера и утеплитель".into(), text: "OSB, фанера, минеральная вата, гидроизоляция, мембраны и теплоизоляционные материалы.".into(), bullets: vec!["OSB и фанера".into(), "Минеральная вата".into(), "Гидроизоляция".into(), "Мембраны и плёнки".into()], photo: "material-osb".into(), image_url: None },
        ],
        products: vec![
            Product { category_slug: "gipsokarton-profili".into(), category: "ГКЛ".into(), title: "ГКЛ 12.5 мм стандартный".into(), spec: "2500x1200 мм · стены и потолки".into(), photo: "photo-plans".into() },
            Product { category_slug: "gipsokarton-profili".into(), category: "ГКЛ".into(), title: "ГКЛ влагостойкий 12.5 мм".into(), spec: "Для влажных зон и аптек".into(), photo: "photo-plans".into() },
            Product { category_slug: "gipsokarton-profili".into(), category: "Профили".into(), title: "Профиль стоечный CW".into(), spec: "50/75/100 мм · перегородки".into(), photo: "photo-building".into() },
            Product { category_slug: "sukhie-smesi".into(), category: "Сухие смеси".into(), title: "Плиточный клей усиленный".into(), spec: "Для керамогранита и плитки".into(), photo: "photo-retail".into() },
        ],
        kits: vec![
            Kit { title: "Комплект для перегородок".into(), text: "Каркас, листы, крепёж и расходники.".into(), items: vec!["ГКЛ".into(), "CW/UW профили".into(), "Подвесы и саморезы".into()] },
            Kit { title: "Комплект для потолка".into(), text: "Система под монтаж потолков.".into(), items: vec!["Профили".into(), "Подвесы".into(), "Плиты / ГКЛ".into()] },
        ],
        projects: vec![
            Project { title: "BUTIK KZ".into(), meta: "Магазин одежды · 320 м² · 28 дней".into(), photo: "photo-retail".into() },
            Project { title: "Green Mart".into(), meta: "Супермаркет · 1250 м² · 45 дней".into(), photo: "photo-office".into() },
            Project { title: "Europharma".into(), meta: "Аптека · 110 м² · 18 дней".into(), photo: "photo-building".into() },
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

fn almabuild_schema(kind: &str) -> &'static str {
    match kind {
        "material" => {
            r#"{"index":"[0:1]","slug":"slug","title":"Название","text":"Описание","bullets":["Пункт"],"photo":"material-class"}"#
        }
        "product" => {
            r#"{"categorySlug":"slug","category":"Категория","title":"Название","spec":"Характеристики","photo":"photo-class"}"#
        }
        "kit" => r#"{"title":"Название","text":"Описание","items":["Позиция"]}"#,
        "project" => r#"{"title":"Название","meta":"Тип · площадь · срок","photo":"photo-class"}"#,
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
    let mut count = 1usize;
    let mut instruction = String::new();
    let mut existing_count = 0usize;
    let mut existing = String::new();

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

    let prompt = format!(
        r#"Ты Vision-модель для CRM строительного сайта KAZAXBUD / ALMABUILD в Алматы.
Проанализируй загруженное фото: упаковки, листы, профили, смеси, плитку, инструмент, складскую полку или группу стройматериалов.
Создай ровно {count} карточек категорий/материалов для секции "Материалы" сайта.

Верни ТОЛЬКО JSON без markdown в формате:
{{"materials":[{{"index":"[0:7]","slug":"latin-slug","title":"Название","text":"Короткое B2B-описание","bullets":["Пункт"],"photo":"material-mixes","imageUrl":"{image_url}"}}]}}

Правила:
- Язык: русский.
- title: понятное название категории/материала, не длиннее 44 символов.
- text: 1 предложение для закупщика, наличие/объём/доставка/монтаж, без фантазий о бренде если на фото его не видно.
- bullets: 3-5 коротких пунктов, каждый до 32 символов.
- slug: латиница lower-case через дефис, уникальный.
- index начинай с [0:{first_index}] и продолжай по порядку.
- photo выбери один класс: material-drywall, material-mixes, material-flooring, material-electric, material-ceiling, material-osb.
- imageUrl всегда ставь ровно: {image_url}
- Если фото содержит конкретные товары, группируй их в полезные категории сайта, а не в случайные одиночные позиции.
- Не повторяй уже существующие карточки, если можно сделать новые или уточнённые.

Текущее количество карточек: {existing_count}
Текущие карточки JSON:
{existing}

Задача администратора:
{instruction}
"#,
        count = count,
        image_url = image_url,
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

    let raw = llm.analyze_image_json(&prompt, &image, &mime_type).await?;
    let value: Value = serde_json::from_str(strip_json_fence(&raw))
        .map_err(|e| AppError::internal(format!("Gemini Vision вернул не JSON: {e}")))?;
    let response: MaterialsFromPhotoResponse = serde_json::from_value(value).map_err(|e| {
        AppError::validation(format!("Gemini Vision вернул неверные материалы: {e}"))
    })?;
    if response.materials.is_empty() {
        return Err(AppError::validation(
            "Gemini Vision не вернул ни одной карточки материала",
        ));
    }
    Ok(Json(MaterialsFromPhotoResponse {
        materials: response
            .materials
            .into_iter()
            .take(count)
            .map(|mut material| {
                material.image_url = Some(image_url.clone());
                material
            })
            .collect(),
    }))
}
