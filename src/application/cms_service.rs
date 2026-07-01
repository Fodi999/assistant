use crate::shared::{AppError, AppResult};
use base64::Engine;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

// ── helpers ───────────────────────────────────────────────────────────────────

/// Convert any text to a URL-safe slug: "How to Choose Fish" → "how-to-choose-fish"
pub fn slugify(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn extract_json_object(raw: &str) -> AppResult<serde_json::Value> {
    let trimmed = raw.trim();
    if let Ok(value) = serde_json::from_str(trimmed) {
        return Ok(value);
    }
    let start = trimmed
        .find('{')
        .ok_or_else(|| AppError::internal("AI response does not contain JSON"))?;
    let end = trimmed
        .rfind('}')
        .ok_or_else(|| AppError::internal("AI response contains incomplete JSON"))?;
    serde_json::from_str(&trimmed[start..=end])
        .map_err(|e| AppError::internal(format!("Failed to parse AI article JSON: {}", e)))
}

// ── Shared structs ────────────────────────────────────────────────────────────

/// Multilingual text block used across all CMS entities
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct MultiLang {
    pub en: String,
    pub pl: String,
    pub ru: String,
    pub uk: String,
}

// ═══════════════════════════════════════════════════════════════════════════════
// ABOUT PAGE
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AboutPageRow {
    pub id: Uuid,
    pub title_en: String,
    pub title_pl: String,
    pub title_ru: String,
    pub title_uk: String,
    pub content_en: String,
    pub content_pl: String,
    pub content_ru: String,
    pub content_uk: String,
    pub image_url: Option<String>,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAboutRequest {
    pub title_en: Option<String>,
    pub title_pl: Option<String>,
    pub title_ru: Option<String>,
    pub title_uk: Option<String>,
    pub content_en: Option<String>,
    pub content_pl: Option<String>,
    pub content_ru: Option<String>,
    pub content_uk: Option<String>,
    pub image_url: Option<String>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// EXPERTISE
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ExpertiseRow {
    pub id: Uuid,
    pub icon: String,
    pub title_en: String,
    pub title_pl: String,
    pub title_ru: String,
    pub title_uk: String,
    pub order_index: i32,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateExpertiseRequest {
    pub icon: String,
    pub title_en: String,
    pub title_pl: Option<String>,
    pub title_ru: Option<String>,
    pub title_uk: Option<String>,
    pub order_index: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateExpertiseRequest {
    pub icon: Option<String>,
    pub title_en: Option<String>,
    pub title_pl: Option<String>,
    pub title_ru: Option<String>,
    pub title_uk: Option<String>,
    pub order_index: Option<i32>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// EXPERIENCE
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ExperienceRow {
    pub id: Uuid,
    pub restaurant: String,
    pub country: String,
    pub position: String,
    pub start_year: Option<i32>,
    pub end_year: Option<i32>,
    pub description_en: String,
    pub description_pl: String,
    pub description_ru: String,
    pub description_uk: String,
    pub order_index: i32,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateExperienceRequest {
    pub restaurant: String,
    pub country: Option<String>,
    pub position: Option<String>,
    pub start_year: Option<i32>,
    pub end_year: Option<i32>,
    pub description_en: Option<String>,
    pub description_pl: Option<String>,
    pub description_ru: Option<String>,
    pub description_uk: Option<String>,
    pub order_index: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateExperienceRequest {
    pub restaurant: Option<String>,
    pub country: Option<String>,
    pub position: Option<String>,
    pub start_year: Option<i32>,
    pub end_year: Option<i32>,
    pub description_en: Option<String>,
    pub description_pl: Option<String>,
    pub description_ru: Option<String>,
    pub description_uk: Option<String>,
    pub order_index: Option<i32>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// GALLERY
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct GalleryCategoryRow {
    pub id: Uuid,
    pub slug: String,
    pub title_en: String,
    pub title_pl: String,
    pub title_ru: String,
    pub title_uk: String,
    pub order_index: i32,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct GalleryRow {
    pub id: Uuid,
    pub image_url: String,
    pub category_id: Option<Uuid>,
    pub slug: String,
    pub status: String,
    pub title_en: String,
    pub title_pl: String,
    pub title_ru: String,
    pub title_uk: String,
    pub description_en: String,
    pub description_pl: String,
    pub description_ru: String,
    pub description_uk: String,
    pub alt_en: String,
    pub alt_pl: String,
    pub alt_ru: String,
    pub alt_uk: String,
    pub order_index: i32,
    pub instagram_url: Option<String>,
    pub pinterest_url: Option<String>,
    pub facebook_url: Option<String>,
    pub tiktok_url: Option<String>,
    pub website_url: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    /// Populated via LEFT JOIN with gallery_categories — not a real column
    #[sqlx(default)]
    pub category_slug: Option<String>,
}

/// Public-facing gallery response — no internal timestamps, clean SEO structure
#[derive(Debug, Serialize)]
pub struct GalleryPublicItem {
    pub id: Uuid,
    pub image_url: String,
    pub category_id: Option<Uuid>,
    pub category_slug: Option<String>,
    pub slug: String,
    pub status: String,
    pub order_index: i32,
    pub title_en: String,
    pub title_pl: String,
    pub title_ru: String,
    pub title_uk: String,
    pub description_en: String,
    pub description_pl: String,
    pub description_ru: String,
    pub description_uk: String,
    pub alt_en: String,
    pub alt_pl: String,
    pub alt_ru: String,
    pub alt_uk: String,
    pub instagram_url: Option<String>,
    pub pinterest_url: Option<String>,
    pub facebook_url: Option<String>,
    pub tiktok_url: Option<String>,
    pub website_url: Option<String>,
}

impl From<GalleryRow> for GalleryPublicItem {
    fn from(r: GalleryRow) -> Self {
        Self {
            id: r.id,
            image_url: r.image_url,
            category_id: r.category_id,
            category_slug: r.category_slug,
            slug: r.slug,
            status: r.status,
            order_index: r.order_index,
            title_en: r.title_en,
            title_pl: r.title_pl,
            title_ru: r.title_ru,
            title_uk: r.title_uk,
            description_en: r.description_en,
            description_pl: r.description_pl,
            description_ru: r.description_ru,
            description_uk: r.description_uk,
            alt_en: r.alt_en,
            alt_pl: r.alt_pl,
            alt_ru: r.alt_ru,
            alt_uk: r.alt_uk,
            instagram_url: r.instagram_url,
            pinterest_url: r.pinterest_url,
            facebook_url: r.facebook_url,
            tiktok_url: r.tiktok_url,
            website_url: r.website_url,
        }
    }
}

fn is_public_reference_url(url: &str) -> bool {
    let value = url.trim();
    if value.len() > 2048 || value.chars().any(char::is_whitespace) {
        return false;
    }
    value.starts_with("https://") || value.starts_with("http://")
}

#[derive(Debug, Deserialize)]
pub struct CreateGalleryRequest {
    pub image_url: String,
    pub category_id: Option<Uuid>,
    pub slug: Option<String>,
    pub status: Option<String>,
    pub title_en: Option<String>,
    pub title_pl: Option<String>,
    pub title_ru: Option<String>,
    pub title_uk: Option<String>,
    pub description_en: Option<String>,
    pub description_pl: Option<String>,
    pub description_ru: Option<String>,
    pub description_uk: Option<String>,
    pub alt_en: Option<String>,
    pub alt_pl: Option<String>,
    pub alt_ru: Option<String>,
    pub alt_uk: Option<String>,
    pub order_index: Option<i32>,
    pub instagram_url: Option<String>,
    pub pinterest_url: Option<String>,
    pub facebook_url: Option<String>,
    pub tiktok_url: Option<String>,
    pub website_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateGalleryRequest {
    pub image_url: Option<String>,
    pub category_id: Option<Uuid>,
    pub slug: Option<String>,
    pub status: Option<String>,
    pub title_en: Option<String>,
    pub title_pl: Option<String>,
    pub title_ru: Option<String>,
    pub title_uk: Option<String>,
    pub description_en: Option<String>,
    pub description_pl: Option<String>,
    pub description_ru: Option<String>,
    pub description_uk: Option<String>,
    pub alt_en: Option<String>,
    pub alt_pl: Option<String>,
    pub alt_ru: Option<String>,
    pub alt_uk: Option<String>,
    pub order_index: Option<i32>,
    pub instagram_url: Option<String>,
    pub pinterest_url: Option<String>,
    pub facebook_url: Option<String>,
    pub tiktok_url: Option<String>,
    pub website_url: Option<String>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// KNOWLEDGE ARTICLES
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ArticleRow {
    pub id: Uuid,
    pub slug: String,
    pub category: String,
    pub title_en: String,
    pub title_pl: String,
    pub title_ru: String,
    pub title_uk: String,
    pub content_en: String,
    pub content_pl: String,
    pub content_ru: String,
    pub content_uk: String,
    pub image_url: Option<String>,
    pub author_name: String,
    pub author_avatar_url: Option<String>,
    pub author_avatar_position: String,
    pub seo_title: String,
    pub seo_description: String,
    pub seo_title_en: String,
    pub seo_title_ru: String,
    pub seo_title_pl: String,
    pub seo_title_uk: String,
    pub seo_description_en: String,
    pub seo_description_ru: String,
    pub seo_description_pl: String,
    pub seo_description_uk: String,
    pub published: bool,
    pub published_at: Option<OffsetDateTime>,
    pub order_index: i32,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// Public-facing article response — clean ISO dates, no internal fields
#[derive(Debug, Serialize)]
pub struct ArticlePublicItem {
    pub id: Uuid,
    pub slug: String,
    pub category: String,
    pub title_en: String,
    pub title_pl: String,
    pub title_ru: String,
    pub title_uk: String,
    pub content_en: String,
    pub content_pl: String,
    pub content_ru: String,
    pub content_uk: String,
    pub image_url: Option<String>,
    pub author_name: String,
    pub author_avatar_url: Option<String>,
    pub author_avatar_position: String,
    pub seo_title: String,
    pub seo_description: String,
    pub seo_title_en: String,
    pub seo_title_ru: String,
    pub seo_title_pl: String,
    pub seo_title_uk: String,
    pub seo_description_en: String,
    pub seo_description_ru: String,
    pub seo_description_pl: String,
    pub seo_description_uk: String,
    pub published: bool,
    pub published_at: Option<String>,
    pub order_index: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ArticleRow> for ArticlePublicItem {
    fn from(r: ArticleRow) -> Self {
        Self {
            id: r.id,
            slug: r.slug,
            category: r.category,
            title_en: r.title_en,
            title_pl: r.title_pl,
            title_ru: r.title_ru,
            title_uk: r.title_uk,
            content_en: r.content_en,
            content_pl: r.content_pl,
            content_ru: r.content_ru,
            content_uk: r.content_uk,
            image_url: r.image_url,
            author_name: r.author_name,
            author_avatar_url: r.author_avatar_url,
            author_avatar_position: r.author_avatar_position,
            seo_title: r.seo_title,
            seo_description: r.seo_description,
            seo_title_en: r.seo_title_en,
            seo_title_ru: r.seo_title_ru,
            seo_title_pl: r.seo_title_pl,
            seo_title_uk: r.seo_title_uk,
            seo_description_en: r.seo_description_en,
            seo_description_ru: r.seo_description_ru,
            seo_description_pl: r.seo_description_pl,
            seo_description_uk: r.seo_description_uk,
            published: r.published,
            published_at: r.published_at.and_then(|value| {
                value
                    .format(&time::format_description::well_known::Rfc3339)
                    .ok()
            }),
            order_index: r.order_index,
            created_at: r
                .created_at
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default(),
            updated_at: r
                .updated_at
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateArticleRequest {
    pub slug: Option<String>, // auto-generated from title_en if empty
    pub category: Option<String>,
    pub title_en: String,
    pub title_pl: Option<String>,
    pub title_ru: Option<String>,
    pub title_uk: Option<String>,
    pub content_en: Option<String>,
    pub content_pl: Option<String>,
    pub content_ru: Option<String>,
    pub content_uk: Option<String>,
    pub image_url: Option<String>,
    pub author_name: Option<String>,
    pub author_avatar_url: Option<String>,
    pub author_avatar_position: Option<String>,
    pub seo_title: Option<String>,
    pub seo_description: Option<String>,
    pub seo_title_en: Option<String>,
    pub seo_title_ru: Option<String>,
    pub seo_title_pl: Option<String>,
    pub seo_title_uk: Option<String>,
    pub seo_description_en: Option<String>,
    pub seo_description_ru: Option<String>,
    pub seo_description_pl: Option<String>,
    pub seo_description_uk: Option<String>,
    #[serde(default)]
    pub published: bool,
    pub order_index: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct CreateAiArticleDraftRequest {
    pub topic: String,
    pub target_chars: Option<usize>,
    pub image_count: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AiArticleDraft {
    pub slug: String,
    pub category: String,
    pub title_en: String,
    pub title_pl: String,
    pub title_ru: String,
    pub title_uk: String,
    pub content_en: String,
    pub content_pl: String,
    pub content_ru: String,
    pub content_uk: String,
    pub seo_title: String,
    pub seo_description: String,
    pub seo_title_en: String,
    pub seo_title_ru: String,
    pub seo_title_pl: String,
    pub seo_title_uk: String,
    pub seo_description_en: String,
    pub seo_description_ru: String,
    pub seo_description_pl: String,
    pub seo_description_uk: String,
    pub image_prompts: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct GenerateAiArticleImagesRequest {
    pub title: String,
    pub prompt: Option<String>,
    #[serde(default)]
    pub index: usize,
    #[serde(default)]
    pub enhanced: bool,
    #[serde(default)]
    pub reference_urls: Vec<String>,
    pub model_preset: Option<String>,
    pub scene_preset: Option<String>,
    pub width_cm: Option<f32>,
    pub height_cm: Option<f32>,
    pub depth_cm: Option<f32>,
    pub weight_kg: Option<f32>,
    #[serde(default)]
    pub photo_scenarios: Vec<String>,
    pub scale_reference: Option<String>,
    pub custom_scale_reference: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AiArticleImageResponse {
    pub image_url: String,
}

// ═══════════════════════════════════════════════════════════════════════════════
// ONLINE SHOP PRODUCTS
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ShopProductRow {
    pub id: Uuid,
    pub slug: String,
    pub sku: Option<String>,
    pub category: String,
    pub name_en: String,
    pub name_ru: String,
    pub name_pl: String,
    pub name_uk: String,
    pub short_description_en: String,
    pub short_description_ru: String,
    pub short_description_pl: String,
    pub short_description_uk: String,
    pub description_en: String,
    pub description_ru: String,
    pub description_pl: String,
    pub description_uk: String,
    pub seo_title_en: String,
    pub seo_title_ru: String,
    pub seo_title_pl: String,
    pub seo_title_uk: String,
    pub seo_description_en: String,
    pub seo_description_ru: String,
    pub seo_description_pl: String,
    pub seo_description_uk: String,
    pub selling_points: Vec<String>,
    pub image_urls: Vec<String>,
    pub price_cents: Option<i64>,
    pub currency: String,
    pub stock_quantity: i32,
    pub status: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AiShopProductDraft {
    pub slug: String,
    pub category: String,
    pub name_en: String,
    pub name_ru: String,
    pub name_pl: String,
    pub name_uk: String,
    pub short_description_en: String,
    pub short_description_ru: String,
    pub short_description_pl: String,
    pub short_description_uk: String,
    pub description_en: String,
    pub description_ru: String,
    pub description_pl: String,
    pub description_uk: String,
    pub seo_title_en: String,
    pub seo_title_ru: String,
    pub seo_title_pl: String,
    pub seo_title_uk: String,
    pub seo_description_en: String,
    pub seo_description_ru: String,
    pub seo_description_pl: String,
    pub seo_description_uk: String,
    pub selling_points: Vec<String>,
    pub image_prompts: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateAiShopProductDraftRequest {
    pub product: String,
    pub image_count: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct CreateShopProductRequest {
    pub slug: Option<String>,
    pub sku: Option<String>,
    pub category: Option<String>,
    pub name_en: String,
    pub name_ru: Option<String>,
    pub name_pl: Option<String>,
    pub name_uk: Option<String>,
    pub short_description_en: Option<String>,
    pub short_description_ru: Option<String>,
    pub short_description_pl: Option<String>,
    pub short_description_uk: Option<String>,
    pub description_en: Option<String>,
    pub description_ru: Option<String>,
    pub description_pl: Option<String>,
    pub description_uk: Option<String>,
    pub seo_title_en: Option<String>,
    pub seo_title_ru: Option<String>,
    pub seo_title_pl: Option<String>,
    pub seo_title_uk: Option<String>,
    pub seo_description_en: Option<String>,
    pub seo_description_ru: Option<String>,
    pub seo_description_pl: Option<String>,
    pub seo_description_uk: Option<String>,
    #[serde(default)]
    pub selling_points: Vec<String>,
    #[serde(default)]
    pub image_urls: Vec<String>,
    pub price_cents: Option<i64>,
    pub currency: Option<String>,
    pub stock_quantity: Option<i32>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateShopProductStatusRequest {
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateArticleRequest {
    pub slug: Option<String>,
    pub category: Option<String>,
    pub title_en: Option<String>,
    pub title_pl: Option<String>,
    pub title_ru: Option<String>,
    pub title_uk: Option<String>,
    pub content_en: Option<String>,
    pub content_pl: Option<String>,
    pub content_ru: Option<String>,
    pub content_uk: Option<String>,
    pub image_url: Option<String>,
    pub author_name: Option<String>,
    pub author_avatar_url: Option<String>,
    pub author_avatar_position: Option<String>,
    pub seo_title: Option<String>,
    pub seo_description: Option<String>,
    pub seo_title_en: Option<String>,
    pub seo_title_ru: Option<String>,
    pub seo_title_pl: Option<String>,
    pub seo_title_uk: Option<String>,
    pub seo_description_en: Option<String>,
    pub seo_description_ru: Option<String>,
    pub seo_description_pl: Option<String>,
    pub seo_description_uk: Option<String>,
    pub published: Option<bool>,
    pub order_index: Option<i32>,
}

// ── Pagination & Search ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ArticleQuery {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub search: Option<String>,
    pub category: Option<String>,
    pub site_id: Option<Uuid>,
    pub site: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ArticleListResponse {
    pub data: Vec<ArticlePublicItem>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
}

// ── Sitemap ───────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ArticleSitemapRow {
    pub slug: String,
    pub updated_at: OffsetDateTime,
}

// ── Stats ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct PublicStats {
    pub articles_count: i64,
    pub ingredients_count: i64,
    pub tools_count: i64,
    pub experience_years: i64,
    pub countries: i64,
}

// ── Categories ────────────────────────────────────────────────────────────────

#[derive(Debug, sqlx::FromRow)]
pub struct ArticleCategoryRow {
    pub id: Uuid,
    pub slug: String,
    pub title_en: String,
    pub title_pl: String,
    pub title_ru: String,
    pub title_uk: String,
    pub order_index: i32,
    pub created_at: OffsetDateTime,
}

/// Public DTO — ISO 8601 date string instead of array
#[derive(Debug, Serialize)]
pub struct ArticleCategoryPublic {
    pub id: Uuid,
    pub slug: String,
    pub title_en: String,
    pub title_pl: String,
    pub title_ru: String,
    pub title_uk: String,
    pub order_index: i32,
    pub created_at: String,
}

impl From<ArticleCategoryRow> for ArticleCategoryPublic {
    fn from(r: ArticleCategoryRow) -> Self {
        Self {
            id: r.id,
            slug: r.slug,
            title_en: r.title_en,
            title_pl: r.title_pl,
            title_ru: r.title_ru,
            title_uk: r.title_uk,
            order_index: r.order_index,
            created_at: r
                .created_at
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SERVICE
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Clone)]
pub struct CmsService {
    pool: PgPool,
    r2_client: crate::infrastructure::R2Client,
    llm_adapter: Arc<crate::infrastructure::llm_adapter::LlmAdapter>,
}

impl CmsService {
    pub fn new(
        pool: PgPool,
        r2_client: crate::infrastructure::R2Client,
        llm_adapter: Arc<crate::infrastructure::llm_adapter::LlmAdapter>,
    ) -> Self {
        Self {
            pool,
            r2_client,
            llm_adapter,
        }
    }

    fn default_site_id() -> Uuid {
        Uuid::from_u128(0x00000000000000000000000000000103)
    }

    fn site_id_from_alias(value: &str) -> Option<Uuid> {
        match value.trim().to_ascii_lowercase().as_str() {
            "church" | "icons" => Some(Uuid::from_u128(0x00000000000000000000000000000101)),
            "construction" | "almabuild" => {
                Some(Uuid::from_u128(0x00000000000000000000000000000102))
            }
            "kitchen" | "culinary" | "blog" => Some(Self::default_site_id()),
            _ => None,
        }
    }

    fn site_id_from_query(site_id: Option<Uuid>, site: Option<&str>) -> Uuid {
        site_id
            .or_else(|| site.and_then(Self::site_id_from_alias))
            .unwrap_or_else(Self::default_site_id)
    }

    // ── ABOUT PAGE ────────────────────────────────────────────────────────────

    pub async fn get_about(&self) -> AppResult<AboutPageRow> {
        sqlx::query_as("SELECT * FROM about_page LIMIT 1")
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::not_found("About page not found"))
    }

    pub async fn update_about(&self, req: UpdateAboutRequest) -> AppResult<AboutPageRow> {
        let current = self.get_about().await?;

        sqlx::query_as(
            r#"UPDATE about_page SET
               title_en   = $1, title_pl   = $2, title_ru   = $3, title_uk   = $4,
               content_en = $5, content_pl = $6, content_ru = $7, content_uk = $8,
               image_url  = $9, updated_at = NOW()
               WHERE id = $10
               RETURNING *"#,
        )
        .bind(req.title_en.unwrap_or(current.title_en))
        .bind(req.title_pl.unwrap_or(current.title_pl))
        .bind(req.title_ru.unwrap_or(current.title_ru))
        .bind(req.title_uk.unwrap_or(current.title_uk))
        .bind(req.content_en.unwrap_or(current.content_en))
        .bind(req.content_pl.unwrap_or(current.content_pl))
        .bind(req.content_ru.unwrap_or(current.content_ru))
        .bind(req.content_uk.unwrap_or(current.content_uk))
        .bind(req.image_url.or(current.image_url))
        .bind(current.id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("update_about: {e}");
            AppError::internal("Failed to update about page")
        })
    }

    // ── EXPERTISE ─────────────────────────────────────────────────────────────

    pub async fn list_expertise(&self) -> AppResult<Vec<ExpertiseRow>> {
        sqlx::query_as("SELECT * FROM expertise ORDER BY order_index ASC, title_en ASC")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("list_expertise: {e}");
                AppError::internal("DB error")
            })
    }

    pub async fn create_expertise(&self, req: CreateExpertiseRequest) -> AppResult<ExpertiseRow> {
        sqlx::query_as(
            r#"INSERT INTO expertise (icon, title_en, title_pl, title_ru, title_uk, order_index)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING *"#,
        )
        .bind(&req.icon)
        .bind(&req.title_en)
        .bind(req.title_pl.unwrap_or_default())
        .bind(req.title_ru.unwrap_or_default())
        .bind(req.title_uk.unwrap_or_default())
        .bind(req.order_index.unwrap_or(0))
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("create_expertise: {e}");
            AppError::internal("Failed to create expertise")
        })
    }

    pub async fn update_expertise(
        &self,
        id: Uuid,
        req: UpdateExpertiseRequest,
    ) -> AppResult<ExpertiseRow> {
        let cur: ExpertiseRow = sqlx::query_as("SELECT * FROM expertise WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::not_found("Expertise not found"))?;

        sqlx::query_as(
            r#"UPDATE expertise SET icon=$1, title_en=$2, title_pl=$3, title_ru=$4, title_uk=$5, order_index=$6
               WHERE id=$7 RETURNING *"#,
        )
        .bind(req.icon.unwrap_or(cur.icon))
        .bind(req.title_en.unwrap_or(cur.title_en))
        .bind(req.title_pl.unwrap_or(cur.title_pl))
        .bind(req.title_ru.unwrap_or(cur.title_ru))
        .bind(req.title_uk.unwrap_or(cur.title_uk))
        .bind(req.order_index.unwrap_or(cur.order_index))
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| { tracing::error!("update_expertise: {e}"); AppError::internal("Failed to update") })
    }

    pub async fn delete_expertise(&self, id: Uuid) -> AppResult<()> {
        let r = sqlx::query("DELETE FROM expertise WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        if r.rows_affected() == 0 {
            return Err(AppError::not_found("Expertise not found"));
        }
        Ok(())
    }

    // ── EXPERIENCE ────────────────────────────────────────────────────────────

    pub async fn list_experience(&self) -> AppResult<Vec<ExperienceRow>> {
        sqlx::query_as("SELECT * FROM experience ORDER BY order_index ASC, start_year DESC")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("list_experience: {e}");
                AppError::internal("DB error")
            })
    }

    pub async fn create_experience(
        &self,
        req: CreateExperienceRequest,
    ) -> AppResult<ExperienceRow> {
        sqlx::query_as(
            r#"INSERT INTO experience
               (restaurant, country, position, start_year, end_year,
                description_en, description_pl, description_ru, description_uk, order_index)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
               RETURNING *"#,
        )
        .bind(&req.restaurant)
        .bind(req.country.unwrap_or_default())
        .bind(req.position.unwrap_or_default())
        .bind(req.start_year)
        .bind(req.end_year)
        .bind(req.description_en.unwrap_or_default())
        .bind(req.description_pl.unwrap_or_default())
        .bind(req.description_ru.unwrap_or_default())
        .bind(req.description_uk.unwrap_or_default())
        .bind(req.order_index.unwrap_or(0))
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("create_experience: {e}");
            AppError::internal("Failed to create")
        })
    }

    pub async fn update_experience(
        &self,
        id: Uuid,
        req: UpdateExperienceRequest,
    ) -> AppResult<ExperienceRow> {
        let cur: ExperienceRow = sqlx::query_as("SELECT * FROM experience WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::not_found("Experience not found"))?;

        sqlx::query_as(
            r#"UPDATE experience SET
               restaurant=$1, country=$2, position=$3, start_year=$4, end_year=$5,
               description_en=$6, description_pl=$7, description_ru=$8, description_uk=$9,
               order_index=$10
               WHERE id=$11 RETURNING *"#,
        )
        .bind(req.restaurant.unwrap_or(cur.restaurant))
        .bind(req.country.unwrap_or(cur.country))
        .bind(req.position.unwrap_or(cur.position))
        .bind(req.start_year.or(cur.start_year))
        .bind(req.end_year.or(cur.end_year))
        .bind(req.description_en.unwrap_or(cur.description_en))
        .bind(req.description_pl.unwrap_or(cur.description_pl))
        .bind(req.description_ru.unwrap_or(cur.description_ru))
        .bind(req.description_uk.unwrap_or(cur.description_uk))
        .bind(req.order_index.unwrap_or(cur.order_index))
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("update_experience: {e}");
            AppError::internal("Failed to update")
        })
    }

    pub async fn delete_experience(&self, id: Uuid) -> AppResult<()> {
        let r = sqlx::query("DELETE FROM experience WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        if r.rows_affected() == 0 {
            return Err(AppError::not_found("Experience not found"));
        }
        Ok(())
    }

    // ── GALLERY ───────────────────────────────────────────────────────────────

    pub async fn list_gallery_categories(&self) -> AppResult<Vec<GalleryCategoryRow>> {
        sqlx::query_as("SELECT * FROM gallery_categories ORDER BY order_index ASC")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("list_gallery_categories: {e}");
                AppError::internal("DB error")
            })
    }

    pub async fn list_gallery(
        &self,
        category_slug: Option<&str>,
        published_only: bool,
    ) -> AppResult<Vec<GalleryPublicItem>> {
        let rows: Vec<GalleryRow> = match category_slug.filter(|s| !s.is_empty()) {
            Some(slug) if published_only => sqlx::query_as(
                r#"SELECT g.*, gc.slug AS category_slug
                   FROM gallery g
                   LEFT JOIN gallery_categories gc ON gc.id = g.category_id
                   WHERE gc.slug = $1 AND g.status = 'published'
                   ORDER BY g.order_index ASC, g.created_at DESC"#,
            )
            .bind(slug)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("list_gallery: {e}");
                AppError::internal("DB error")
            })?,
            Some(slug) => sqlx::query_as(
                r#"SELECT g.*, gc.slug AS category_slug
                   FROM gallery g
                   LEFT JOIN gallery_categories gc ON gc.id = g.category_id
                   WHERE gc.slug = $1
                   ORDER BY g.order_index ASC, g.created_at DESC"#,
            )
            .bind(slug)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("list_gallery: {e}");
                AppError::internal("DB error")
            })?,
            None if published_only => sqlx::query_as(
                r#"SELECT g.*, gc.slug AS category_slug
                   FROM gallery g
                   LEFT JOIN gallery_categories gc ON gc.id = g.category_id
                   WHERE g.status = 'published'
                   ORDER BY g.order_index ASC, g.created_at DESC"#,
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("list_gallery: {e}");
                AppError::internal("DB error")
            })?,
            None => sqlx::query_as(
                r#"SELECT g.*, gc.slug AS category_slug
                   FROM gallery g
                   LEFT JOIN gallery_categories gc ON gc.id = g.category_id
                   ORDER BY g.order_index ASC, g.created_at DESC"#,
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("list_gallery: {e}");
                AppError::internal("DB error")
            })?,
        };
        Ok(rows.into_iter().map(GalleryPublicItem::from).collect())
    }

    pub async fn create_gallery(&self, req: CreateGalleryRequest) -> AppResult<GalleryRow> {
        // Auto-generate slug from title_en if not provided
        let slug = match req.slug.as_deref().filter(|s| !s.is_empty()) {
            Some(s) => s.to_string(),
            None => {
                let base = if req.title_en.as_deref().unwrap_or("").is_empty() {
                    "gallery".to_string()
                } else {
                    slugify(req.title_en.as_deref().unwrap_or("gallery"))
                };
                format!("{}-{}", base, &uuid::Uuid::new_v4().to_string()[..8])
            }
        };
        sqlx::query_as(
            r#"INSERT INTO gallery
               (image_url, category_id, slug, status,
                title_en, title_pl, title_ru, title_uk,
                description_en, description_pl, description_ru, description_uk,
                alt_en, alt_pl, alt_ru, alt_uk, order_index,
                instagram_url, pinterest_url, facebook_url, tiktok_url, website_url)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22)
               RETURNING *"#,
        )
        .bind(&req.image_url)
        .bind(req.category_id)
        .bind(&slug)
        .bind(req.status.unwrap_or_else(|| "published".to_string()))
        .bind(req.title_en.unwrap_or_default())
        .bind(req.title_pl.unwrap_or_default())
        .bind(req.title_ru.unwrap_or_default())
        .bind(req.title_uk.unwrap_or_default())
        .bind(req.description_en.unwrap_or_default())
        .bind(req.description_pl.unwrap_or_default())
        .bind(req.description_ru.unwrap_or_default())
        .bind(req.description_uk.unwrap_or_default())
        .bind(req.alt_en.unwrap_or_default())
        .bind(req.alt_pl.unwrap_or_default())
        .bind(req.alt_ru.unwrap_or_default())
        .bind(req.alt_uk.unwrap_or_default())
        .bind(req.order_index.unwrap_or(0))
        .bind(req.instagram_url)
        .bind(req.pinterest_url)
        .bind(req.facebook_url)
        .bind(req.tiktok_url)
        .bind(req.website_url)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("unique") || e.to_string().contains("duplicate") {
                AppError::conflict(&format!("Gallery item with slug '{slug}' already exists"))
            } else {
                tracing::error!("create_gallery: {e}"); AppError::internal("Failed to create")
            }
        })
    }

    pub async fn update_gallery(
        &self,
        id: Uuid,
        req: UpdateGalleryRequest,
    ) -> AppResult<GalleryRow> {
        let cur: GalleryRow = sqlx::query_as("SELECT * FROM gallery WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::not_found("Gallery item not found"))?;

        sqlx::query_as(
            r#"UPDATE gallery SET
               image_url=$1, category_id=$2, slug=$3, status=$4,
               title_en=$5, title_pl=$6, title_ru=$7, title_uk=$8,
               description_en=$9, description_pl=$10, description_ru=$11, description_uk=$12,
               alt_en=$13, alt_pl=$14, alt_ru=$15, alt_uk=$16,
               order_index=$17,
               instagram_url=$18, pinterest_url=$19, facebook_url=$20, tiktok_url=$21, website_url=$22
               WHERE id=$23 RETURNING *"#,
        )
        .bind(req.image_url.unwrap_or(cur.image_url))
        .bind(req.category_id.or(cur.category_id))
        .bind(req.slug.unwrap_or(cur.slug))
        .bind(req.status.unwrap_or(cur.status))
        .bind(req.title_en.unwrap_or(cur.title_en))
        .bind(req.title_pl.unwrap_or(cur.title_pl))
        .bind(req.title_ru.unwrap_or(cur.title_ru))
        .bind(req.title_uk.unwrap_or(cur.title_uk))
        .bind(req.description_en.unwrap_or(cur.description_en))
        .bind(req.description_pl.unwrap_or(cur.description_pl))
        .bind(req.description_ru.unwrap_or(cur.description_ru))
        .bind(req.description_uk.unwrap_or(cur.description_uk))
        .bind(req.alt_en.unwrap_or(cur.alt_en))
        .bind(req.alt_pl.unwrap_or(cur.alt_pl))
        .bind(req.alt_ru.unwrap_or(cur.alt_ru))
        .bind(req.alt_uk.unwrap_or(cur.alt_uk))
        .bind(req.order_index.unwrap_or(cur.order_index))
        .bind(req.instagram_url.or(cur.instagram_url))
        .bind(req.pinterest_url.or(cur.pinterest_url))
        .bind(req.facebook_url.or(cur.facebook_url))
        .bind(req.tiktok_url.or(cur.tiktok_url))
        .bind(req.website_url.or(cur.website_url))
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| { tracing::error!("update_gallery: {e}"); AppError::internal("Failed to update") })
    }

    pub async fn delete_gallery(&self, id: Uuid) -> AppResult<()> {
        let r = sqlx::query("DELETE FROM gallery WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        if r.rows_affected() == 0 {
            return Err(AppError::not_found("Gallery item not found"));
        }
        Ok(())
    }

    // ── KNOWLEDGE ARTICLES ────────────────────────────────────────────────────

    pub async fn create_ai_article_draft(
        &self,
        topic: &str,
        target_chars: Option<usize>,
        image_count: Option<usize>,
    ) -> AppResult<AiArticleDraft> {
        let topic = topic.trim();
        if topic.is_empty() {
            return Err(AppError::validation("Article topic cannot be empty"));
        }
        let target_chars = target_chars.unwrap_or(3500).clamp(1500, 12000);
        let image_count = image_count.unwrap_or(4).clamp(1, 12);
        let prompt = format!(
            r#"You are a senior chef, food technologist and professional culinary editor.
Create a concise practical expert article draft about: "{topic}".

Return ONLY valid JSON with this exact shape:
{{
  "slug": "english-url-slug",
  "category": "Ingredient|Technique|Recipe development|Restaurant management",
  "title_en": "...", "title_ru": "...", "title_pl": "...", "title_uk": "...",
  "content_en": "...", "content_ru": "...", "content_pl": "...", "content_uk": "...",
  "seo_title": "...", "seo_description": "...",
  "seo_title_en": "...", "seo_title_ru": "...", "seo_title_pl": "...", "seo_title_uk": "...",
  "seo_description_en": "...", "seo_description_ru": "...", "seo_description_pl": "...", "seo_description_uk": "...",
  "image_prompts": ["one prompt for every requested image"]
}}

Content rules:
- Each language is a complete natural article, not a summary and not machine-sounding
- Target approximately {target_chars} characters per language, Markdown format (within ±10%)
- Start with a useful introduction, then 3-5 sections with ## headings, practical checklist and conclusion
- Give factual, actionable culinary guidance; never invent scientific claims
- Do not include the article title as the first Markdown heading
- SEO titles are unique, localized and under 60 characters
- SEO descriptions are unique, localized and 120-160 characters
- image_prompts are concise English photography directions matching the article, no text or logos
- Return exactly {image_count} image prompts; for recipes they must follow the process chronologically
- Return ONLY JSON"#,
            topic = topic,
            target_chars = target_chars,
            image_count = image_count,
        );
        let raw = self
            .llm_adapter
            .groq_raw_request_with_model(&prompt, 12000, "gemini-3.5-flash")
            .await?;
        let json = extract_json_object(&raw)?;
        serde_json::from_value(json)
            .map_err(|e| AppError::internal(format!("Invalid AI article draft: {}", e)))
    }

    pub async fn create_ai_shop_product_draft(
        &self,
        product: &str,
        image_count: Option<usize>,
    ) -> AppResult<AiShopProductDraft> {
        let product = product.trim();
        if product.is_empty() {
            return Err(AppError::validation(
                "Shop product description cannot be empty",
            ));
        }
        let image_count = image_count.unwrap_or(4).clamp(1, 8);
        let prompt = format!(
            r#"You are a senior ecommerce copywriter and product catalog manager.
Create an online-store product card for this exact sellable item: "{product}".

Return ONLY valid JSON with this exact shape:
{{
  "slug": "english-url-slug",
  "category": "delivery-food|kitchen-tools|tableware|ingredients|beverages|other",
  "name_en": "...", "name_ru": "...", "name_pl": "...", "name_uk": "...",
  "short_description_en": "...", "short_description_ru": "...", "short_description_pl": "...", "short_description_uk": "...",
  "description_en": "...", "description_ru": "...", "description_pl": "...", "description_uk": "...",
  "seo_title_en": "...", "seo_title_ru": "...", "seo_title_pl": "...", "seo_title_uk": "...",
  "seo_description_en": "...", "seo_description_ru": "...", "seo_description_pl": "...", "seo_description_uk": "...",
  "selling_points": ["...", "...", "..."],
  "image_prompts": ["one prompt for every requested image"]
}}

STORE PRODUCT RULES:
- Describe only the exact sellable product; do not turn it into an article, recipe or ingredient encyclopedia
- Never invent price, SKU, stock, discounts, certifications, dimensions, materials or package contents not present in the input
- Names and descriptions must be natural localized ecommerce copy in all four languages
- Short descriptions: 90-180 characters; full descriptions: 500-900 characters
- SEO titles: maximum 60 characters; SEO descriptions: 120-160 characters
- Selling points must be short factual benefits in English
- Image prompts must show the exact same product as a commercial catalog series: hero, package/context, useful detail, alternate clean composition
- No people, hands, preparation process, added products, text, logos or watermarks
- Return exactly {image_count} image prompts
- Return ONLY JSON"#,
            product = product,
            image_count = image_count,
        );
        let raw = self
            .llm_adapter
            .groq_raw_request_with_model(&prompt, 10000, "gemini-3.5-flash")
            .await?;
        let json = extract_json_object(&raw)?;
        serde_json::from_value(json)
            .map_err(|e| AppError::internal(format!("Invalid AI shop product draft: {}", e)))
    }

    pub async fn generate_ai_article_image(
        &self,
        title: &str,
        prompt: Option<&str>,
        index: usize,
        enhanced: bool,
        reference_urls: &[String],
        model_preset: Option<&str>,
        scene_preset: Option<&str>,
        width_cm: Option<f32>,
        height_cm: Option<f32>,
        depth_cm: Option<f32>,
        weight_kg: Option<f32>,
        photo_scenarios: &[String],
        scale_reference: Option<&str>,
        custom_scale_reference: Option<&str>,
    ) -> AppResult<AiArticleImageResponse> {
        let title = title.trim();
        if title.is_empty() {
            return Err(AppError::validation("Article title cannot be empty"));
        }
        if reference_urls.len() > 4
            || reference_urls
                .iter()
                .any(|url| !is_public_reference_url(url))
        {
            return Err(AppError::validation(
                "Reference images must be public http(s) URLs",
            ));
        }
        let default_prompt = match index {
            0 => "editorial hero cover".to_string(),
            1 => "professional preparation process, first important step".to_string(),
            2 => "close-up ingredient or technique detail".to_string(),
            3 => "finished culinary result".to_string(),
            _ => format!("professional chronological preparation step {}", index),
        };
        let prompt = prompt
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(&default_prompt);
        let dimensions = [
            width_cm.map(|value| format!("width {:.1} cm", value)),
            height_cm.map(|value| format!("height {:.1} cm", value)),
            depth_cm.map(|value| format!("depth {:.1} cm", value)),
            weight_kg.map(|value| format!("weight {:.2} kg", value)),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(", ");
        let scale_reference = match scale_reference {
            Some("custom") => custom_scale_reference.unwrap_or("").trim(),
            Some(value) => value.trim(),
            None => "",
        };
        let scenarios = if photo_scenarios.is_empty() {
            "use the selected scene preset".to_string()
        } else {
            photo_scenarios.join(", ")
        };
        let scale_direction = format!(
            "Physical dimensions: {}. Requested photo scenarios: {}. Scale reference: {}.",
            if dimensions.is_empty() {
                "not specified"
            } else {
                &dimensions
            },
            scenarios,
            if scale_reference.is_empty() {
                "none"
            } else {
                scale_reference
            },
        );
        let base64 = self
            .llm_adapter
            .generate_blog_article_image(
                title,
                prompt,
                index,
                enhanced || model_preset == Some("pro"),
                reference_urls,
                scene_preset.unwrap_or("editorial"),
                &scale_direction,
            )
            .await?;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(base64)
            .map_err(|e| AppError::internal(format!("Failed to decode article image: {}", e)))?;
        let key = format!("assets/cms/articles/generated/{}.png", Uuid::new_v4());
        let image_url = self
            .r2_client
            .upload_image(&key, Bytes::from(bytes), "image/png")
            .await?;
        Ok(AiArticleImageResponse { image_url })
    }

    /// Admin: list all articles (including drafts)
    pub async fn list_articles_admin(&self, category: Option<&str>) -> AppResult<Vec<ArticleRow>> {
        self.list_articles_admin_for_site(Self::default_site_id(), category)
            .await
    }

    pub async fn list_articles_admin_for_site(
        &self,
        site_id: Uuid,
        category: Option<&str>,
    ) -> AppResult<Vec<ArticleRow>> {
        match category.filter(|s| !s.is_empty()) {
            Some(cat) => sqlx::query_as(
                "SELECT * FROM knowledge_articles WHERE (site_id = $1 OR is_global = true) AND category = $2 ORDER BY order_index ASC, created_at DESC",
            )
            .bind(site_id)
            .bind(cat)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| { tracing::error!("list_articles_admin: {e}"); AppError::internal("DB error") }),

            None => sqlx::query_as(
                "SELECT * FROM knowledge_articles WHERE site_id = $1 OR is_global = true ORDER BY order_index ASC, created_at DESC",
            )
            .bind(site_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| { tracing::error!("list_articles_admin: {e}"); AppError::internal("DB error") }),
        }
    }

    /// Public: list only published articles
    pub async fn list_articles_public(&self) -> AppResult<Vec<ArticleRow>> {
        self.list_articles_public_for_site(Self::default_site_id())
            .await
    }

    pub async fn list_articles_public_for_site(&self, site_id: Uuid) -> AppResult<Vec<ArticleRow>> {
        sqlx::query_as(
            "SELECT * FROM knowledge_articles
             WHERE published = true AND (site_id = $1 OR is_global = true)
             ORDER BY order_index ASC, created_at DESC",
        )
        .bind(site_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("list_articles_public: {e}");
            AppError::internal("DB error")
        })
    }

    /// Public: get single article by slug (published only)
    pub async fn get_article_by_slug(&self, slug: &str) -> AppResult<ArticleRow> {
        self.get_article_by_slug_for_site(slug, Self::default_site_id())
            .await
    }

    pub async fn get_article_by_slug_for_site(
        &self,
        slug: &str,
        site_id: Uuid,
    ) -> AppResult<ArticleRow> {
        sqlx::query_as(
            "SELECT * FROM knowledge_articles
             WHERE slug = $1 AND published = true AND (site_id = $2 OR is_global = true)",
        )
        .bind(slug)
        .bind(site_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::not_found("Article not found"))
    }

    /// Admin: get article by id (any status)
    pub async fn get_article_by_id(&self, id: Uuid) -> AppResult<ArticleRow> {
        self.get_article_by_id_for_site(id, Self::default_site_id())
            .await
    }

    pub async fn get_article_by_id_for_site(
        &self,
        id: Uuid,
        site_id: Uuid,
    ) -> AppResult<ArticleRow> {
        sqlx::query_as(
            "SELECT * FROM knowledge_articles WHERE id = $1 AND (site_id = $2 OR is_global = true)",
        )
        .bind(id)
        .bind(site_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::not_found("Article not found"))
    }

    pub async fn create_article(&self, req: CreateArticleRequest) -> AppResult<ArticleRow> {
        self.create_article_for_site(req, Self::default_site_id())
            .await
    }

    pub async fn create_article_for_site(
        &self,
        req: CreateArticleRequest,
        site_id: Uuid,
    ) -> AppResult<ArticleRow> {
        // Auto-generate slug from title_en if not provided
        let slug = match req.slug.as_deref() {
            Some(s) if !s.trim().is_empty() => s.trim().to_string(),
            _ => slugify(&req.title_en),
        };

        sqlx::query_as(
            r#"INSERT INTO knowledge_articles
               (site_id, slug, category, title_en, title_pl, title_ru, title_uk,
                content_en, content_pl, content_ru, content_uk,
                image_url, author_name, author_avatar_url, author_avatar_position, seo_title, seo_description,
                seo_title_en, seo_title_ru, seo_title_pl, seo_title_uk,
                seo_description_en, seo_description_ru, seo_description_pl, seo_description_uk,
                published, published_at, order_index)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24,$25,$26, CASE WHEN $26 THEN NOW() ELSE NULL END, $27)
               RETURNING *"#,
        )
        .bind(site_id)
        .bind(&slug)
        .bind(req.category.unwrap_or_default())
        .bind(&req.title_en)
        .bind(req.title_pl.clone().unwrap_or_default())
        .bind(req.title_ru.clone().unwrap_or_default())
        .bind(req.title_uk.clone().unwrap_or_default())
        .bind(req.content_en.unwrap_or_default())
        .bind(req.content_pl.unwrap_or_default())
        .bind(req.content_ru.unwrap_or_default())
        .bind(req.content_uk.unwrap_or_default())
        .bind(&req.image_url)
        .bind(req.author_name.unwrap_or_else(|| "Szef Kuchni".to_string()))
        .bind(req.author_avatar_url)
        .bind(
            req.author_avatar_position
                .unwrap_or_else(|| "center".to_string()),
        )
        .bind(
            req.seo_title
                .clone()
                .or_else(|| req.seo_title_en.clone())
                .unwrap_or_else(|| req.title_en.clone()),
        )
        .bind(
            req.seo_description
                .clone()
                .or_else(|| req.seo_description_en.clone())
                .unwrap_or_default(),
        )
        .bind(req.seo_title_en.unwrap_or_else(|| req.title_en.clone()))
        .bind(req.seo_title_ru.unwrap_or_else(|| {
            if let Some(title) = &req.title_ru {
                title.clone()
            } else {
                req.title_en.clone()
            }
        }))
        .bind(req.seo_title_pl.unwrap_or_else(|| {
            if let Some(title) = &req.title_pl {
                title.clone()
            } else {
                req.title_en.clone()
            }
        }))
        .bind(req.seo_title_uk.unwrap_or_else(|| {
            if let Some(title) = &req.title_uk {
                title.clone()
            } else {
                req.title_en.clone()
            }
        }))
        .bind(req.seo_description_en.unwrap_or_default())
        .bind(req.seo_description_ru.unwrap_or_default())
        .bind(req.seo_description_pl.unwrap_or_default())
        .bind(req.seo_description_uk.unwrap_or_default())
        .bind(req.published)
        .bind(req.order_index.unwrap_or(0))
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("create_article: {e}");
            if e.to_string().contains("unique") {
                AppError::conflict(&format!("Article with slug '{slug}' already exists"))
            } else {
                AppError::internal("Failed to create article")
            }
        })
    }

    pub async fn list_shop_products(&self) -> AppResult<Vec<ShopProductRow>> {
        self.list_shop_products_for_site(Self::default_site_id())
            .await
    }

    pub async fn list_shop_products_for_site(
        &self,
        site_id: Uuid,
    ) -> AppResult<Vec<ShopProductRow>> {
        sqlx::query_as(
            "SELECT * FROM shop_products WHERE site_id = $1 OR is_global = true ORDER BY updated_at DESC",
        )
            .bind(site_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("list_shop_products: {e}");
                AppError::internal("Failed to list shop products")
            })
    }

    pub async fn list_public_shop_products(&self) -> AppResult<Vec<ShopProductRow>> {
        self.list_public_shop_products_for_site(Self::default_site_id())
            .await
    }

    pub async fn list_public_shop_products_for_site(
        &self,
        site_id: Uuid,
    ) -> AppResult<Vec<ShopProductRow>> {
        sqlx::query_as(
            "SELECT * FROM shop_products
             WHERE status = 'active' AND (site_id = $1 OR is_global = true)
             ORDER BY updated_at DESC",
        )
        .bind(site_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("list_public_shop_products: {e}");
            AppError::internal("Failed to list shop products")
        })
    }

    pub async fn get_public_shop_product(&self, slug: &str) -> AppResult<ShopProductRow> {
        self.get_public_shop_product_for_site(slug, Self::default_site_id())
            .await
    }

    pub async fn get_public_shop_product_for_site(
        &self,
        slug: &str,
        site_id: Uuid,
    ) -> AppResult<ShopProductRow> {
        sqlx::query_as(
            "SELECT * FROM shop_products
             WHERE slug = $1 AND status = 'active' AND (site_id = $2 OR is_global = true)",
        )
        .bind(slug)
        .bind(site_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::not_found("Shop product not found"))
    }

    pub async fn update_shop_product_status(
        &self,
        id: Uuid,
        status: &str,
    ) -> AppResult<ShopProductRow> {
        self.update_shop_product_status_for_site(id, Self::default_site_id(), status)
            .await
    }

    pub async fn update_shop_product_status_for_site(
        &self,
        id: Uuid,
        site_id: Uuid,
        status: &str,
    ) -> AppResult<ShopProductRow> {
        if !matches!(status, "draft" | "active" | "archived") {
            return Err(AppError::validation("Invalid shop product status"));
        }
        sqlx::query_as(
            "UPDATE shop_products SET status = $1, updated_at = NOW() WHERE id = $2 AND site_id = $3 RETURNING *",
        )
        .bind(status)
        .bind(id)
        .bind(site_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::not_found("Shop product not found"))
    }

    pub async fn delete_shop_product(&self, id: Uuid) -> AppResult<()> {
        self.delete_shop_product_for_site(id, Self::default_site_id())
            .await
    }

    pub async fn delete_shop_product_for_site(&self, id: Uuid, site_id: Uuid) -> AppResult<()> {
        let result = sqlx::query("DELETE FROM shop_products WHERE id = $1 AND site_id = $2")
            .bind(id)
            .bind(site_id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::not_found("Shop product not found"));
        }
        Ok(())
    }

    pub async fn create_shop_product(
        &self,
        req: CreateShopProductRequest,
    ) -> AppResult<ShopProductRow> {
        self.create_shop_product_for_site(req, Self::default_site_id())
            .await
    }

    pub async fn create_shop_product_for_site(
        &self,
        req: CreateShopProductRequest,
        site_id: Uuid,
    ) -> AppResult<ShopProductRow> {
        let slug = req
            .slug
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| slugify(&req.name_en));
        let status = req.status.unwrap_or_else(|| "draft".to_string());
        if !matches!(status.as_str(), "draft" | "active" | "archived") {
            return Err(AppError::validation("Invalid shop product status"));
        }
        sqlx::query_as(
            r#"INSERT INTO shop_products (
                site_id, slug, sku, category, name_en, name_ru, name_pl, name_uk,
                short_description_en, short_description_ru, short_description_pl, short_description_uk,
                description_en, description_ru, description_pl, description_uk,
                seo_title_en, seo_title_ru, seo_title_pl, seo_title_uk,
                seo_description_en, seo_description_ru, seo_description_pl, seo_description_uk,
                selling_points, image_urls, price_cents, currency, stock_quantity, status
            ) VALUES (
                $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,
                $20,$21,$22,$23,$24,$25,$26,$27,$28,$29,$30
            ) RETURNING *"#,
        )
        .bind(site_id)
        .bind(slug)
        .bind(req.sku.filter(|value| !value.trim().is_empty()))
        .bind(req.category.unwrap_or_else(|| "other".to_string()))
        .bind(req.name_en)
        .bind(req.name_ru.unwrap_or_default())
        .bind(req.name_pl.unwrap_or_default())
        .bind(req.name_uk.unwrap_or_default())
        .bind(req.short_description_en.unwrap_or_default())
        .bind(req.short_description_ru.unwrap_or_default())
        .bind(req.short_description_pl.unwrap_or_default())
        .bind(req.short_description_uk.unwrap_or_default())
        .bind(req.description_en.unwrap_or_default())
        .bind(req.description_ru.unwrap_or_default())
        .bind(req.description_pl.unwrap_or_default())
        .bind(req.description_uk.unwrap_or_default())
        .bind(req.seo_title_en.unwrap_or_default())
        .bind(req.seo_title_ru.unwrap_or_default())
        .bind(req.seo_title_pl.unwrap_or_default())
        .bind(req.seo_title_uk.unwrap_or_default())
        .bind(req.seo_description_en.unwrap_or_default())
        .bind(req.seo_description_ru.unwrap_or_default())
        .bind(req.seo_description_pl.unwrap_or_default())
        .bind(req.seo_description_uk.unwrap_or_default())
        .bind(req.selling_points)
        .bind(req.image_urls)
        .bind(req.price_cents)
        .bind(req.currency.unwrap_or_else(|| "PLN".to_string()))
        .bind(req.stock_quantity.unwrap_or(0).max(0))
        .bind(status)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("create_shop_product: {e}");
            AppError::internal("Failed to create shop product")
        })
    }

    pub async fn update_shop_product_for_site(
        &self,
        id: Uuid,
        req: CreateShopProductRequest,
        site_id: Uuid,
    ) -> AppResult<ShopProductRow> {
        let slug = req
            .slug
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| slugify(&req.name_en));
        let status = req.status.unwrap_or_else(|| "draft".to_string());
        if !matches!(status.as_str(), "draft" | "active" | "archived") {
            return Err(AppError::validation("Invalid shop product status"));
        }
        sqlx::query_as(
            r#"UPDATE shop_products SET
                slug = $1,
                sku = $2,
                category = $3,
                name_en = $4,
                name_ru = $5,
                name_pl = $6,
                name_uk = $7,
                short_description_en = $8,
                short_description_ru = $9,
                short_description_pl = $10,
                short_description_uk = $11,
                description_en = $12,
                description_ru = $13,
                description_pl = $14,
                description_uk = $15,
                seo_title_en = $16,
                seo_title_ru = $17,
                seo_title_pl = $18,
                seo_title_uk = $19,
                seo_description_en = $20,
                seo_description_ru = $21,
                seo_description_pl = $22,
                seo_description_uk = $23,
                selling_points = $24,
                image_urls = $25,
                price_cents = $26,
                currency = $27,
                stock_quantity = $28,
                status = $29,
                updated_at = NOW()
            WHERE id = $30 AND site_id = $31
            RETURNING *"#,
        )
        .bind(slug)
        .bind(req.sku.filter(|value| !value.trim().is_empty()))
        .bind(req.category.unwrap_or_else(|| "other".to_string()))
        .bind(req.name_en)
        .bind(req.name_ru.unwrap_or_default())
        .bind(req.name_pl.unwrap_or_default())
        .bind(req.name_uk.unwrap_or_default())
        .bind(req.short_description_en.unwrap_or_default())
        .bind(req.short_description_ru.unwrap_or_default())
        .bind(req.short_description_pl.unwrap_or_default())
        .bind(req.short_description_uk.unwrap_or_default())
        .bind(req.description_en.unwrap_or_default())
        .bind(req.description_ru.unwrap_or_default())
        .bind(req.description_pl.unwrap_or_default())
        .bind(req.description_uk.unwrap_or_default())
        .bind(req.seo_title_en.unwrap_or_default())
        .bind(req.seo_title_ru.unwrap_or_default())
        .bind(req.seo_title_pl.unwrap_or_default())
        .bind(req.seo_title_uk.unwrap_or_default())
        .bind(req.seo_description_en.unwrap_or_default())
        .bind(req.seo_description_ru.unwrap_or_default())
        .bind(req.seo_description_pl.unwrap_or_default())
        .bind(req.seo_description_uk.unwrap_or_default())
        .bind(req.selling_points)
        .bind(req.image_urls)
        .bind(req.price_cents)
        .bind(req.currency.unwrap_or_else(|| "PLN".to_string()))
        .bind(req.stock_quantity.unwrap_or(0).max(0))
        .bind(status)
        .bind(id)
        .bind(site_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("update_shop_product: {e}");
            AppError::internal("Failed to update shop product")
        })?
        .ok_or_else(|| AppError::not_found("Shop product not found"))
    }

    pub async fn update_article(
        &self,
        id: Uuid,
        req: UpdateArticleRequest,
    ) -> AppResult<ArticleRow> {
        self.update_article_for_site(id, Self::default_site_id(), req)
            .await
    }

    pub async fn update_article_for_site(
        &self,
        id: Uuid,
        site_id: Uuid,
        req: UpdateArticleRequest,
    ) -> AppResult<ArticleRow> {
        let cur = self.get_article_by_id_for_site(id, site_id).await?;

        sqlx::query_as(
            r#"UPDATE knowledge_articles SET
               slug=$1, category=$2, title_en=$3, title_pl=$4, title_ru=$5, title_uk=$6,
               content_en=$7, content_pl=$8, content_ru=$9, content_uk=$10,
               image_url=$11, author_name=$12, author_avatar_url=$13, author_avatar_position=$14,
               seo_title=$15, seo_description=$16,
               seo_title_en=$17, seo_title_ru=$18, seo_title_pl=$19, seo_title_uk=$20,
               seo_description_en=$21, seo_description_ru=$22, seo_description_pl=$23, seo_description_uk=$24,
               published=$25,
               published_at=CASE
                 WHEN $25 = true AND published_at IS NULL THEN NOW()
                 WHEN $25 = false THEN NULL
                 ELSE published_at
               END,
               order_index=$26, updated_at=NOW()
               WHERE id=$27 AND site_id=$28 RETURNING *"#,
        )
        .bind(req.slug.unwrap_or(cur.slug))
        .bind(req.category.unwrap_or(cur.category))
        .bind(req.title_en.unwrap_or(cur.title_en))
        .bind(req.title_pl.unwrap_or(cur.title_pl))
        .bind(req.title_ru.unwrap_or(cur.title_ru))
        .bind(req.title_uk.unwrap_or(cur.title_uk))
        .bind(req.content_en.unwrap_or(cur.content_en))
        .bind(req.content_pl.unwrap_or(cur.content_pl))
        .bind(req.content_ru.unwrap_or(cur.content_ru))
        .bind(req.content_uk.unwrap_or(cur.content_uk))
        .bind(req.image_url.or(cur.image_url))
        .bind(req.author_name.unwrap_or(cur.author_name))
        .bind(req.author_avatar_url.or(cur.author_avatar_url))
        .bind(
            req.author_avatar_position
                .unwrap_or(cur.author_avatar_position),
        )
        .bind(req.seo_title.unwrap_or(cur.seo_title))
        .bind(req.seo_description.unwrap_or(cur.seo_description))
        .bind(req.seo_title_en.unwrap_or(cur.seo_title_en))
        .bind(req.seo_title_ru.unwrap_or(cur.seo_title_ru))
        .bind(req.seo_title_pl.unwrap_or(cur.seo_title_pl))
        .bind(req.seo_title_uk.unwrap_or(cur.seo_title_uk))
        .bind(req.seo_description_en.unwrap_or(cur.seo_description_en))
        .bind(req.seo_description_ru.unwrap_or(cur.seo_description_ru))
        .bind(req.seo_description_pl.unwrap_or(cur.seo_description_pl))
        .bind(req.seo_description_uk.unwrap_or(cur.seo_description_uk))
        .bind(req.published.unwrap_or(cur.published))
        .bind(req.order_index.unwrap_or(cur.order_index))
        .bind(id)
        .bind(site_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("update_article: {e}");
            AppError::internal("Failed to update")
        })
    }

    pub async fn delete_article(&self, id: Uuid) -> AppResult<()> {
        self.delete_article_for_site(id, Self::default_site_id())
            .await
    }

    pub async fn delete_article_for_site(&self, id: Uuid, site_id: Uuid) -> AppResult<()> {
        let r = sqlx::query("DELETE FROM knowledge_articles WHERE id = $1 AND site_id = $2")
            .bind(id)
            .bind(site_id)
            .execute(&self.pool)
            .await?;
        if r.rows_affected() == 0 {
            return Err(AppError::not_found("Article not found"));
        }
        Ok(())
    }

    // ── ARTICLES: pagination + search ─────────────────────────────────────────

    pub async fn list_articles_paged(&self, q: &ArticleQuery) -> AppResult<ArticleListResponse> {
        let page = q.page.unwrap_or(1).max(1);
        let limit = q.limit.unwrap_or(20).clamp(1, 100);
        let offset = (page - 1) * limit;
        let site_id = Self::site_id_from_query(q.site_id, q.site.as_deref());

        let category_filter = q.category.as_deref().filter(|s| !s.is_empty());

        let (total, rows) = if let Some(search) = q.search.as_deref().filter(|s| !s.is_empty()) {
            let pattern = format!("%{}%", search.to_lowercase());

            let (total, rows) = if let Some(cat) = category_filter {
                let total: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM knowledge_articles
                     WHERE published = true AND (site_id = $2 OR is_global = true) AND category = $3
                       AND (LOWER(title_en) LIKE $1 OR LOWER(title_ru) LIKE $1
                         OR LOWER(title_pl) LIKE $1 OR LOWER(title_uk) LIKE $1
                         OR LOWER(content_en) LIKE $1)",
                )
                .bind(&pattern)
                .bind(site_id)
                .bind(cat)
                .fetch_one(&self.pool)
                .await
                .unwrap_or(0);
                let rows: Vec<ArticleRow> = sqlx::query_as(
                    "SELECT * FROM knowledge_articles
                     WHERE published = true AND (site_id = $2 OR is_global = true) AND category = $3
                       AND (LOWER(title_en) LIKE $1 OR LOWER(title_ru) LIKE $1
                         OR LOWER(title_pl) LIKE $1 OR LOWER(title_uk) LIKE $1
                         OR LOWER(content_en) LIKE $1)
                     ORDER BY order_index ASC, created_at DESC LIMIT $4 OFFSET $5",
                )
                .bind(&pattern)
                .bind(site_id)
                .bind(cat)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| {
                    tracing::error!("list_articles_paged search/category: {e}");
                    AppError::internal("DB error")
                })?;
                (total, rows)
            } else {
                let total: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM knowledge_articles
                     WHERE published = true AND (site_id = $2 OR is_global = true)
                       AND (LOWER(title_en) LIKE $1 OR LOWER(title_ru) LIKE $1
                         OR LOWER(title_pl) LIKE $1 OR LOWER(title_uk) LIKE $1
                         OR LOWER(content_en) LIKE $1)",
                )
                .bind(&pattern)
                .bind(site_id)
                .fetch_one(&self.pool)
                .await
                .unwrap_or(0);
                let rows: Vec<ArticleRow> = sqlx::query_as(
                    "SELECT * FROM knowledge_articles
                     WHERE published = true AND (site_id = $2 OR is_global = true)
                       AND (LOWER(title_en) LIKE $1 OR LOWER(title_ru) LIKE $1
                         OR LOWER(title_pl) LIKE $1 OR LOWER(title_uk) LIKE $1
                         OR LOWER(content_en) LIKE $1)
                     ORDER BY order_index ASC, created_at DESC LIMIT $3 OFFSET $4",
                )
                .bind(&pattern)
                .bind(site_id)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| {
                    tracing::error!("list_articles_paged search: {e}");
                    AppError::internal("DB error")
                })?;
                (total, rows)
            };
            (total, rows)
        } else if let Some(cat) = category_filter {
            let total: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM knowledge_articles
                 WHERE published = true AND (site_id = $1 OR is_global = true) AND category = $2",
            )
            .bind(site_id)
            .bind(cat)
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);

            let rows: Vec<ArticleRow> = sqlx::query_as(
                "SELECT * FROM knowledge_articles
                 WHERE published = true AND (site_id = $1 OR is_global = true) AND category = $2
                 ORDER BY order_index ASC, created_at DESC LIMIT $3 OFFSET $4",
            )
            .bind(site_id)
            .bind(cat)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("list_articles_paged category: {e}");
                AppError::internal("DB error")
            })?;

            (total, rows)
        } else {
            let total: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM knowledge_articles
                 WHERE published = true AND (site_id = $1 OR is_global = true)",
            )
            .bind(site_id)
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);

            let rows: Vec<ArticleRow> = sqlx::query_as(
                "SELECT * FROM knowledge_articles
                 WHERE published = true AND (site_id = $1 OR is_global = true)
                 ORDER BY order_index ASC, created_at DESC LIMIT $2 OFFSET $3",
            )
            .bind(site_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("list_articles_paged: {e}");
                AppError::internal("DB error")
            })?;

            (total, rows)
        };

        let data = rows.into_iter().map(ArticlePublicItem::from).collect();
        Ok(ArticleListResponse {
            data,
            total,
            page,
            limit,
        })
    }

    // ── SITEMAP ───────────────────────────────────────────────────────────────

    pub async fn articles_sitemap(&self) -> AppResult<Vec<ArticleSitemapRow>> {
        sqlx::query_as(
            "SELECT slug, updated_at FROM knowledge_articles
             WHERE published = true ORDER BY updated_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("articles_sitemap: {e}");
            AppError::internal("DB error")
        })
    }

    // ── STATS ─────────────────────────────────────────────────────────────────

    pub async fn public_stats(&self) -> AppResult<PublicStats> {
        let articles_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM knowledge_articles WHERE published = true")
                .fetch_one(&self.pool)
                .await
                .unwrap_or(0);

        let ingredients_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM catalog_ingredients WHERE deleted_at IS NULL")
                .fetch_one(&self.pool)
                .await
                .unwrap_or(0);

        // Count distinct public tool routes (static number, update as tools grow)
        let tools_count: i64 = 18;

        let experience_years: i64 = sqlx::query_scalar(
            "SELECT COALESCE(EXTRACT(YEAR FROM NOW())::bigint - MIN(start_year)::bigint, 0)
             FROM experience WHERE start_year IS NOT NULL",
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(20);

        let countries: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT country) FROM experience WHERE country <> ''",
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        Ok(PublicStats {
            articles_count,
            ingredients_count,
            tools_count,
            experience_years,
            countries,
        })
    }

    // ── ARTICLE CATEGORIES ────────────────────────────────────────────────────

    pub async fn list_categories(&self) -> AppResult<Vec<ArticleCategoryPublic>> {
        let rows: Vec<ArticleCategoryRow> =
            sqlx::query_as("SELECT * FROM article_categories ORDER BY order_index ASC")
                .fetch_all(&self.pool)
                .await
                .map_err(|e| {
                    tracing::error!("list_categories: {e}");
                    AppError::internal("DB error")
                })?;
        Ok(rows.into_iter().map(ArticleCategoryPublic::from).collect())
    }

    // ── IMAGE UPLOAD: presigned URL ───────────────────────────────────────────

    pub async fn get_image_upload_url(
        &self,
        folder: &str,
        content_type: &str,
    ) -> AppResult<crate::application::user::AvatarUploadResponse> {
        let ext = match content_type {
            ct if ct.contains("jpeg") || ct.contains("jpg") => "jpg",
            ct if ct.contains("png") => "png",
            ct if ct.contains("gif") => "gif",
            _ => "webp",
        };
        let id = Uuid::new_v4();
        let key = format!("cms/{}/{}.{}", folder, id, ext);

        let upload_url = self
            .r2_client
            .generate_presigned_upload_url(&key, content_type)
            .await?;
        let public_url = self.r2_client.get_public_url(&key);

        Ok(crate::application::user::AvatarUploadResponse {
            upload_url,
            public_url,
        })
    }

    pub async fn upload_article_reference(
        &self,
        file_data: Bytes,
        content_type: &str,
    ) -> AppResult<String> {
        let extension = match content_type {
            "image/jpeg" | "image/jpg" => "jpg",
            "image/png" => "png",
            "image/webp" => "webp",
            _ => {
                return Err(AppError::validation(
                    "Allowed reference types: jpg, png, webp",
                ));
            }
        };
        if file_data.len() > 10 * 1024 * 1024 {
            return Err(AppError::validation(
                "Reference image must be smaller than 10 MB",
            ));
        }
        let key = format!("cms/article-references/{}.{}", Uuid::new_v4(), extension);
        self.r2_client
            .upload_image(&key, file_data, content_type)
            .await
    }

    // ── GALLERY (updated with alt fields) ─────────────────────────────────────

    pub async fn create_gallery_v2(&self, req: CreateGalleryRequest) -> AppResult<GalleryRow> {
        self.create_gallery(req).await
    }

    pub async fn update_gallery_v2(
        &self,
        id: Uuid,
        req: UpdateGalleryRequest,
    ) -> AppResult<GalleryRow> {
        self.update_gallery(id, req).await
    }
}
