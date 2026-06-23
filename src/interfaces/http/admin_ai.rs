use axum::{extract::Multipart, Extension, Json};
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    domain::AdminClaims,
    infrastructure::{llm_adapter::LlmAdapter, R2Client},
    shared::AppError,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiGenerationRequest {
    pub site: String,
    pub language: String,
    #[serde(rename = "type")]
    pub generation_type: String,
    pub source_text: Option<String>,
    pub product_id: Option<String>,
    pub tone: Option<String>,
    pub keywords: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiGenerationResult {
    pub id: String,
    pub request: AiGenerationRequest,
    pub title: Option<String>,
    pub description: Option<String>,
    pub slug: Option<String>,
    pub photo_prompt: Option<String>,
    pub quality_score: Option<u8>,
    pub suggestions: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiVisionResult {
    pub id: String,
    pub site: String,
    pub image_url: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub slug: Option<String>,
    pub price_hint: Option<String>,
    pub materials: Vec<Value>,
    pub affiliate_products: Vec<Value>,
    pub article_ideas: Vec<Value>,
    pub seo_title: Option<String>,
    pub seo_description: Option<String>,
    pub suggestions: Vec<String>,
    pub raw: Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiImageRequest {
    pub site: String,
    pub title: String,
    pub description: Option<String>,
    pub scene: Option<String>,
    pub image_type: Option<String>,
    #[serde(default)]
    pub reference_urls: Vec<String>,
    #[serde(default)]
    pub variant: Option<usize>,
    #[serde(default)]
    pub enhanced: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiImageResult {
    pub id: String,
    pub image_url: String,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_model: Option<String>,
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
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

fn slugify(value: &str) -> String {
    let slug = deunicode::deunicode(value)
        .to_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        format!("ai-{}", Uuid::new_v4())
    } else {
        slug.chars().take(72).collect()
    }
}

fn context_text(req: &AiGenerationRequest) -> String {
    req.source_text
        .clone()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| req.keywords.as_ref().map(|items| items.join(", ")))
        .unwrap_or_else(|| "новый коммерческий контент".to_string())
}

fn generation_prompt(req: &AiGenerationRequest, route_kind: &str) -> String {
    let context = context_text(req);
    let business_model = match req.site.as_str() {
        "construction" => {
            "строительный сайт: affiliate, калькулятор ремонта, заявки, поставщики, комплекты материалов/работ"
        }
        "icons" => {
            "православный сайт Свет Иконы: церковный календарь, иконы, молитвы, Евангелие дня, жития святых, QR-страницы и SEO-материалы для храмов"
        }
        _ => "кулинарный сайт: affiliate, статьи, обзоры, рецепты, обзоры кухонных товаров",
    };
    let language = req.language.as_str();
    let generation_type = req.generation_type.as_str();
    let tone = req.tone.as_deref().unwrap_or("seo");

    if req.site == "icons" {
        return format!(
            r#"Ты православный редактор, факт-чекер и SEO ассистент админ-панели "Свет Иконы".
Сайт: {business_model}
Язык результата: {language}
Тип генерации: {generation_type}
Endpoint: {route_kind}
Тон: {tone}

Контекст администратора:
{context}

Верни ТОЛЬКО JSON без markdown:
{{
  "title": "точное церковное или редакционное название страницы",
  "description": "полноценный православный материал 3000-4500 знаков с разделами: Краткое описание, Полное описание, История образа, В чем помогает, Как молиться, Молитва перед иконой, Евангельская связка, SEO title, SEO description",
  "slug": "latin-url-slug",
  "photoPrompt": "English prompt for faithful Orthodox icon restoration/generation from reference: preserve subject, composition, colors, halo/riza, sacred icon style; no photorealistic people, no church interior scene, no typography, no watermark",
  "qualityScore": 0,
  "suggestions": ["какие факты сверить перед публикацией", "что добавить: молитва, источник, дата памяти, QR"]
}}

Правила:
- Пиши как православный справочник и музейный каталог: спокойно, точно, благоговейно.
- Не делай короткую карточку. Нужен подробный текст страницы: история, смысл, о чем молятся, как молиться, молитва.
- Не выдумывай явления, даты, чудеса, авторство и канонические детали. Если данных нет, пиши осторожно и предложи проверить источник.
- Молитва должна быть перед конкретным образом/святым, без обещаний гарантированного результата и без суеверий.
- Для икон объясняй, что молитва обращена к Господу, Богородице или святому, а не к материалу изображения.
- Slug только латиницей lower-case через дефис."#,
        );
    }

    format!(
        r#"Ты контент-редактор и performance SEO ассистент админ-панели.
Сайт и модель заработка: {business_model}
Язык результата: {language}
Тип генерации: {generation_type}
Endpoint: {route_kind}
Тон: {tone}

Контекст администратора:
{context}

Верни ТОЛЬКО JSON без markdown:
{{
  "title": "короткий коммерческий заголовок или SEO title",
  "description": "готовый текст: описание, статья, обзор, улучшение карточки или SEO description",
  "slug": "latin-url-slug",
  "photoPrompt": "английский промпт для Gemini Image, если нужен",
  "qualityScore": 0,
  "suggestions": ["что проверить перед публикацией", "что добавить для заработка"]
}}

Правила:
- Не выдумывай точные цены, комиссии, бренды и наличие, если их нет в контексте.
- Для affiliate всегда упомяни проверку ссылки, продавца, комиссии/cookie days и schema.org Product/Review.
- Для кулинарного сайта делай формат статьи/обзора полезным: плюсы, минусы, кому подходит, критерии выбора.
- Для строительного сайта добавляй связку: материал + работа + поставщик + заявка + расчет в калькуляторе.
- Slug только латиницей lower-case через дефис.
- photoPrompt должен быть реалистичным, без текста, логотипов и водяных знаков."#,
    )
}

async fn generate_ai(
    llm: Arc<LlmAdapter>,
    req: AiGenerationRequest,
    route_kind: &str,
) -> Result<Json<AiGenerationResult>, AppError> {
    let prompt = generation_prompt(&req, route_kind);
    let max_tokens = if req.site == "icons" { 9000 } else { 4500 };
    let raw = llm
        .groq_raw_request_with_model(&prompt, max_tokens, "gemini-3.1-pro-preview")
        .await?;
    let parsed: Value = serde_json::from_str(strip_json_fence(&raw))
        .map_err(|e| AppError::internal(format!("Gemini returned invalid JSON: {e}")))?;
    let title = parsed
        .get("title")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let description = parsed
        .get("description")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let slug = parsed
        .get("slug")
        .and_then(Value::as_str)
        .map(slugify)
        .or_else(|| title.as_deref().map(slugify));
    let suggestions = parsed
        .get("suggestions")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|items| !items.is_empty())
        .unwrap_or_else(|| {
            vec![
                "Проверить факты, цену, валюту и продавца".to_string(),
                "Добавить CTA: заявка или affiliate переход".to_string(),
            ]
        });

    Ok(Json(AiGenerationResult {
        id: Uuid::new_v4().to_string(),
        request: req,
        title,
        description,
        slug,
        photo_prompt: parsed
            .get("photoPrompt")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        quality_score: parsed
            .get("qualityScore")
            .and_then(Value::as_u64)
            .map(|value| value.min(100) as u8),
        suggestions,
        created_at: now(),
    }))
}

pub async fn affiliate_product(
    _claims: AdminClaims,
    Extension(llm): Extension<Arc<LlmAdapter>>,
    Json(req): Json<AiGenerationRequest>,
) -> Result<Json<AiGenerationResult>, AppError> {
    generate_ai(llm, req, "affiliate-product").await
}

pub async fn seo(
    _claims: AdminClaims,
    Extension(llm): Extension<Arc<LlmAdapter>>,
    Json(req): Json<AiGenerationRequest>,
) -> Result<Json<AiGenerationResult>, AppError> {
    generate_ai(llm, req, "seo").await
}

pub async fn photo_prompt(
    _claims: AdminClaims,
    Extension(llm): Extension<Arc<LlmAdapter>>,
    Json(req): Json<AiGenerationRequest>,
) -> Result<Json<AiGenerationResult>, AppError> {
    generate_ai(llm, req, "photo-prompt").await
}

pub async fn improve_product_card(
    _claims: AdminClaims,
    Extension(llm): Extension<Arc<LlmAdapter>>,
    Json(req): Json<AiGenerationRequest>,
) -> Result<Json<AiGenerationResult>, AppError> {
    generate_ai(llm, req, "quality-check").await
}

pub async fn vision_from_photo(
    _claims: AdminClaims,
    Extension(llm): Extension<Arc<LlmAdapter>>,
    Extension(r2): Extension<R2Client>,
    mut multipart: Multipart,
) -> Result<Json<AiVisionResult>, AppError> {
    let mut image: Option<bytes::Bytes> = None;
    let mut mime_type = "image/jpeg".to_string();
    let mut site = "culinary".to_string();
    let mut language = "ru".to_string();
    let mut instruction = String::new();

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
            "site" => {
                site = field
                    .text()
                    .await
                    .unwrap_or_else(|_| "culinary".to_string())
            }
            "language" => language = field.text().await.unwrap_or_else(|_| "ru".to_string()),
            "instruction" => instruction = field.text().await.unwrap_or_default(),
            _ => {}
        }
    }

    let image = image.ok_or_else(|| AppError::validation("Upload image field is required"))?;
    let extension = match mime_type.as_str() {
        "image/png" => "png",
        "image/webp" => "webp",
        "image/jpeg" | "image/jpg" => "jpg",
        _ => "jpg",
    };
    let image_key = format!("admin-ai/uploads/{}.{}", Uuid::new_v4(), extension);
    let image_url = r2
        .upload_image(&image_key, image.clone(), &mime_type)
        .await
        .ok();
    let business_model = if site == "construction" {
        "строительный сайт: affiliate + калькулятор + заявки + поставщики + комплекты материалов/работ"
    } else if site == "icons" {
        "православный сайт Свет Иконы: каталог икон, молитвы, святые, календарь, QR-страницы и SEO"
    } else {
        "кулинарный сайт: affiliate + статьи + обзоры"
    };
    let prompt = if site == "icons" {
        format!(
            r#"Ты Gemini Vision для православной админ-панели "Свет Иконы".
Модель сайта: {business_model}
Язык результата: {language}
Инструкция администратора: {instruction}

Проанализируй загруженное фото как православную икону, церковный образ или фрагмент святого изображения.
Определи, если можно без выдумки:
- кто изображен или какой праздник/сюжет;
- тип иконографии, композицию, жесты, нимбы, ризы, цвета, фон, надписи, состояние изображения;
- категорию: Богородичные, Господские, Святые, Праздничные, Ангелы, Другое;
- основу для SEO-страницы, истории, духовного смысла и молитвы.

Верни ТОЛЬКО JSON без markdown:
{{
  "title": "точное название образа или осторожное рабочее название",
  "description": "подробный материал 2500-4500 знаков с разделами: Краткое описание, История образа, В чем помогает, Как молиться, Молитва перед иконой",
  "category": "категория",
  "slug": "latin-url-slug",
  "priceHint": "",
  "materials": [],
  "affiliateProducts": [],
  "articleIdeas": [{{"title":"идея православной страницы", "type":"icon|prayer|calendar"}}],
  "seoTitle": "SEO title",
  "seoDescription": "SEO description",
  "suggestions": ["что проверить: название, дата памяти, источник молитвы, качество изображения"]
}}

Правила:
- Не превращай икону в интерьерную фотографию с candles/church. Главный объект — сам образ и его иконография.
- Не выдумывай имя святого, если не уверен; лучше укажи осторожность в suggestions.
- Молитва должна быть благоговейной и конкретной к изображенному Господу, Богородице или святому.
- Slug только латиницей."#,
            business_model = business_model,
            language = language,
            instruction = if instruction.trim().is_empty() {
                "Создай заготовку православной страницы по фото."
            } else {
                instruction.trim()
            }
        )
    } else {
        format!(
            r#"Ты Gemini Vision для админ-панели коммерческих сайтов.
Модель сайта: {business_model}
Язык результата: {language}
Инструкция администратора: {instruction}

Проанализируй фото товара, блюда, кухни, упаковки, строительного материала, инструмента, склада или объекта.
Верни ТОЛЬКО JSON без markdown:
{{
  "title": "название того, что видно",
  "description": "коммерческое описание без выдуманных фактов",
  "category": "категория",
  "slug": "latin-url-slug",
  "priceHint": "что нужно спросить/проверить по цене",
  "materials": [{{"title":"для стройки", "unit":"m2|piece|bag|kg", "work":"какая работа связана"}}],
  "affiliateProducts": [{{"title":"товар для affiliate", "reviewAngle":"угол обзора"}}],
  "articleIdeas": [{{"title":"идея статьи или обзора", "type":"article|review|comparison|roundup"}}],
  "seoTitle": "SEO title",
  "seoDescription": "SEO description",
  "suggestions": ["следующее действие в админке"]
}}

Правила:
- Если это строительный материал, дай связку материал/работа/поставщик/заявка.
- Если это кухонный товар или еда, дай affiliate/обзор/статья.
- Не придумывай бренд, цену, сертификаты и точные характеристики, если они не видны.
- Slug только латиницей."#,
            business_model = business_model,
            language = language,
            instruction = if instruction.trim().is_empty() {
                "Создай заготовки для админки по фото."
            } else {
                instruction.trim()
            }
        )
    };
    let raw = llm.analyze_image_json(&prompt, &image, &mime_type).await?;
    let parsed: Value = serde_json::from_str(strip_json_fence(&raw))
        .map_err(|e| AppError::internal(format!("Gemini Vision returned invalid JSON: {e}")))?;

    Ok(Json(AiVisionResult {
        id: Uuid::new_v4().to_string(),
        site,
        image_url,
        title: parsed
            .get("title")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        description: parsed
            .get("description")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        category: parsed
            .get("category")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        slug: parsed.get("slug").and_then(Value::as_str).map(slugify),
        price_hint: parsed
            .get("priceHint")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        materials: parsed
            .get("materials")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
        affiliate_products: parsed
            .get("affiliateProducts")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
        article_ideas: parsed
            .get("articleIdeas")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
        seo_title: parsed
            .get("seoTitle")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        seo_description: parsed
            .get("seoDescription")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        suggestions: parsed
            .get("suggestions")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
        raw: parsed,
    }))
}

pub async fn generate_image(
    _claims: AdminClaims,
    Extension(llm): Extension<Arc<LlmAdapter>>,
    Extension(r2): Extension<R2Client>,
    Json(req): Json<AiImageRequest>,
) -> Result<Json<AiImageResult>, AppError> {
    let title = req.title.trim();
    if title.is_empty() {
        return Err(AppError::validation("title is required"));
    }
    let scene = req.scene.as_deref().unwrap_or("commercial editorial image");
    let image_type = req.image_type.as_deref().unwrap_or("auto");
    let request_context = format!(
        "{} {} {}",
        req.site,
        req.description.as_deref().unwrap_or(""),
        scene
    )
    .to_lowercase();
    let is_calendar_day_image = image_type == "calendar"
        || (req.site == "icons"
            && (request_context.contains("calendar day")
                || request_context.contains("church calendar")
                || request_context.contains("julian date")
                || request_context.contains("julian calendar")
                || request_context.contains("церковный календар")
                || request_context.contains("юлианск")));
    let (base64, image_model) = if is_calendar_day_image {
        (
            llm.generate_calendar_day_image(title, req.description.as_deref().unwrap_or(""), scene)
                .await?,
            None,
        )
    } else if req.site == "icons" {
        let selected_photo_instruction = if req.reference_urls.len() == 1 {
            format!(
                r#"
Selected photo regeneration instruction:
{scene}
"#
            )
        } else {
            String::new()
        };
        let icon_scene = format!(
            r#"Generate ONE IMAGE ONLY. Do not write JSON, markdown, captions or article text.

Product mockup task.
Reference URLs: {refs}
{selected_photo_instruction}

Reference contract:
- Use Reference 1 as the product mockup template: wooden frame, warm light, QR module, button, phone, camera angle and background.
- Place Reference 2 inside the framed artwork area.

Instructions:
- Keep Reference 2 visually recognizable and preserve its composition, colors and details.
- Adjust only perspective, crop and lighting so it fits naturally inside the frame.
- Keep all product elements from Reference 1 unchanged.
- Avoid adding readable new text, logos, watermarks, UI captions or marketing text.

Output: realistic premium product photo."#,
            refs = if req.reference_urls.is_empty() {
                "none".to_string()
            } else {
                req.reference_urls.join(", ")
            },
            selected_photo_instruction = selected_photo_instruction
        );
        let (base64, model) = llm
            .generate_icon_product_mockup_image(
                title,
                &icon_scene,
                &req.reference_urls,
                "faithful iconographic scale",
            )
            .await?;
        (base64, Some(model))
    } else if req.site == "construction" || image_type == "construction" {
        (
            llm.generate_construction_project_image(
                title,
                req.description.as_deref().unwrap_or(""),
                scene,
                req.variant.unwrap_or(0),
                req.enhanced.unwrap_or(false),
                &req.reference_urls,
            )
            .await?,
            None,
        )
    } else if image_type == "article" || image_type == "review" {
        (
            llm.generate_blog_article_image(
                title,
                scene,
                1,
                false,
                &[],
                "product-editorial",
                "normal realistic scale",
            )
            .await?,
            None,
        )
    } else {
        (
            llm.generate_catalog_product_image(
                &format!("admin-ai-{}", slugify(title)),
                title,
                req.description.as_deref(),
            )
            .await?,
            None,
        )
    };
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(base64)
        .map_err(|e| AppError::internal(format!("Gemini image base64 decode failed: {e}")))?;
    let key = format!(
        "admin-ai/generated/{}-{}.png",
        slugify(title),
        Uuid::new_v4()
    );
    let image_url = r2
        .upload_image(&key, bytes::Bytes::from(bytes), "image/png")
        .await?;

    Ok(Json(AiImageResult {
        id: Uuid::new_v4().to_string(),
        image_url,
        prompt: scene.to_string(),
        image_model,
    }))
}
