use crate::shared::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
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
    pub id:         Uuid,
    pub title_en:   String,
    pub title_pl:   String,
    pub title_ru:   String,
    pub title_uk:   String,
    pub content_en: String,
    pub content_pl: String,
    pub content_ru: String,
    pub content_uk: String,
    pub image_url:  Option<String>,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAboutRequest {
    pub title_en:   Option<String>,
    pub title_pl:   Option<String>,
    pub title_ru:   Option<String>,
    pub title_uk:   Option<String>,
    pub content_en: Option<String>,
    pub content_pl: Option<String>,
    pub content_ru: Option<String>,
    pub content_uk: Option<String>,
    pub image_url:  Option<String>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// EXPERTISE
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ExpertiseRow {
    pub id:          Uuid,
    pub icon:        String,
    pub title_en:    String,
    pub title_pl:    String,
    pub title_ru:    String,
    pub title_uk:    String,
    pub order_index: i32,
    pub created_at:  OffsetDateTime,
    pub updated_at:  OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateExpertiseRequest {
    pub icon:        String,
    pub title_en:    String,
    pub title_pl:    Option<String>,
    pub title_ru:    Option<String>,
    pub title_uk:    Option<String>,
    pub order_index: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateExpertiseRequest {
    pub icon:        Option<String>,
    pub title_en:    Option<String>,
    pub title_pl:    Option<String>,
    pub title_ru:    Option<String>,
    pub title_uk:    Option<String>,
    pub order_index: Option<i32>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// EXPERIENCE
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ExperienceRow {
    pub id:             Uuid,
    pub restaurant:     String,
    pub country:        String,
    pub position:       String,
    pub start_year:     Option<i32>,
    pub end_year:       Option<i32>,
    pub description_en: String,
    pub description_pl: String,
    pub description_ru: String,
    pub description_uk: String,
    pub order_index:    i32,
    pub created_at:     OffsetDateTime,
    pub updated_at:     OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateExperienceRequest {
    pub restaurant:     String,
    pub country:        Option<String>,
    pub position:       Option<String>,
    pub start_year:     Option<i32>,
    pub end_year:       Option<i32>,
    pub description_en: Option<String>,
    pub description_pl: Option<String>,
    pub description_ru: Option<String>,
    pub description_uk: Option<String>,
    pub order_index:    Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateExperienceRequest {
    pub restaurant:     Option<String>,
    pub country:        Option<String>,
    pub position:       Option<String>,
    pub start_year:     Option<i32>,
    pub end_year:       Option<i32>,
    pub description_en: Option<String>,
    pub description_pl: Option<String>,
    pub description_ru: Option<String>,
    pub description_uk: Option<String>,
    pub order_index:    Option<i32>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// GALLERY
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct GalleryRow {
    pub id:             Uuid,
    pub image_url:      String,
    pub category:       String,
    pub title_en:       String,
    pub title_pl:       String,
    pub title_ru:       String,
    pub title_uk:       String,
    pub description_en: String,
    pub description_pl: String,
    pub description_ru: String,
    pub description_uk: String,
    pub alt_en:         String,
    pub alt_pl:         String,
    pub alt_ru:         String,
    pub alt_uk:         String,
    pub order_index:    i32,
    pub instagram_url:  Option<String>,
    pub pinterest_url:  Option<String>,
    pub facebook_url:   Option<String>,
    pub tiktok_url:     Option<String>,
    pub website_url:    Option<String>,
    pub created_at:     OffsetDateTime,
    pub updated_at:     OffsetDateTime,
}

/// Public-facing gallery response — no internal timestamps, clean SEO structure
#[derive(Debug, Serialize)]
pub struct GalleryPublicItem {
    pub id:             Uuid,
    pub image_url:      String,
    pub category:       String,
    pub order_index:    i32,
    pub title_en:       String,
    pub title_pl:       String,
    pub title_ru:       String,
    pub title_uk:       String,
    pub description_en: String,
    pub description_pl: String,
    pub description_ru: String,
    pub description_uk: String,
    pub alt_en:         String,
    pub alt_pl:         String,
    pub alt_ru:         String,
    pub alt_uk:         String,
    pub instagram_url:  Option<String>,
    pub pinterest_url:  Option<String>,
    pub facebook_url:   Option<String>,
    pub tiktok_url:     Option<String>,
    pub website_url:    Option<String>,
}

impl From<GalleryRow> for GalleryPublicItem {
    fn from(r: GalleryRow) -> Self {
        Self {
            id:             r.id,
            image_url:      r.image_url,
            category:       r.category,
            order_index:    r.order_index,
            title_en:       r.title_en,
            title_pl:       r.title_pl,
            title_ru:       r.title_ru,
            title_uk:       r.title_uk,
            description_en: r.description_en,
            description_pl: r.description_pl,
            description_ru: r.description_ru,
            description_uk: r.description_uk,
            alt_en:         r.alt_en,
            alt_pl:         r.alt_pl,
            alt_ru:         r.alt_ru,
            alt_uk:         r.alt_uk,
            instagram_url:  r.instagram_url,
            pinterest_url:  r.pinterest_url,
            facebook_url:   r.facebook_url,
            tiktok_url:     r.tiktok_url,
            website_url:    r.website_url,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateGalleryRequest {
    pub image_url:      String,
    pub category:       Option<String>,
    pub title_en:       Option<String>,
    pub title_pl:       Option<String>,
    pub title_ru:       Option<String>,
    pub title_uk:       Option<String>,
    pub description_en: Option<String>,
    pub description_pl: Option<String>,
    pub description_ru: Option<String>,
    pub description_uk: Option<String>,
    pub alt_en:         Option<String>,
    pub alt_pl:         Option<String>,
    pub alt_ru:         Option<String>,
    pub alt_uk:         Option<String>,
    pub order_index:    Option<i32>,
    pub instagram_url:  Option<String>,
    pub pinterest_url:  Option<String>,
    pub facebook_url:   Option<String>,
    pub tiktok_url:     Option<String>,
    pub website_url:    Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateGalleryRequest {
    pub image_url:      Option<String>,
    pub category:       Option<String>,
    pub title_en:       Option<String>,
    pub title_pl:       Option<String>,
    pub title_ru:       Option<String>,
    pub title_uk:       Option<String>,
    pub description_en: Option<String>,
    pub description_pl: Option<String>,
    pub description_ru: Option<String>,
    pub description_uk: Option<String>,
    pub alt_en:         Option<String>,
    pub alt_pl:         Option<String>,
    pub alt_ru:         Option<String>,
    pub alt_uk:         Option<String>,
    pub order_index:    Option<i32>,
    pub instagram_url:  Option<String>,
    pub pinterest_url:  Option<String>,
    pub facebook_url:   Option<String>,
    pub tiktok_url:     Option<String>,
    pub website_url:    Option<String>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// KNOWLEDGE ARTICLES
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ArticleRow {
    pub id:              Uuid,
    pub slug:            String,
    pub category:        String,
    pub title_en:        String,
    pub title_pl:        String,
    pub title_ru:        String,
    pub title_uk:        String,
    pub content_en:      String,
    pub content_pl:      String,
    pub content_ru:      String,
    pub content_uk:      String,
    pub image_url:       Option<String>,
    pub seo_title:       String,
    pub seo_description: String,
    pub published:       bool,
    pub order_index:     i32,
    pub created_at:      OffsetDateTime,
    pub updated_at:      OffsetDateTime,
}

/// Public-facing article response — clean ISO dates, no internal fields
#[derive(Debug, Serialize)]
pub struct ArticlePublicItem {
    pub id:              Uuid,
    pub slug:            String,
    pub category:        String,
    pub title_en:        String,
    pub title_pl:        String,
    pub title_ru:        String,
    pub title_uk:        String,
    pub content_en:      String,
    pub content_pl:      String,
    pub content_ru:      String,
    pub content_uk:      String,
    pub image_url:       Option<String>,
    pub seo_title:       String,
    pub seo_description: String,
    pub published:       bool,
    pub order_index:     i32,
    pub created_at:      String,
    pub updated_at:      String,
}

impl From<ArticleRow> for ArticlePublicItem {
    fn from(r: ArticleRow) -> Self {
        Self {
            id:              r.id,
            slug:            r.slug,
            category:        r.category,
            title_en:        r.title_en,
            title_pl:        r.title_pl,
            title_ru:        r.title_ru,
            title_uk:        r.title_uk,
            content_en:      r.content_en,
            content_pl:      r.content_pl,
            content_ru:      r.content_ru,
            content_uk:      r.content_uk,
            image_url:       r.image_url,
            seo_title:       r.seo_title,
            seo_description: r.seo_description,
            published:       r.published,
            order_index:     r.order_index,
            created_at:      r.created_at.format(&time::format_description::well_known::Rfc3339)
                               .unwrap_or_default(),
            updated_at:      r.updated_at.format(&time::format_description::well_known::Rfc3339)
                               .unwrap_or_default(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateArticleRequest {
    pub slug:            Option<String>,   // auto-generated from title_en if empty
    pub category:        Option<String>,
    pub title_en:        String,
    pub title_pl:        Option<String>,
    pub title_ru:        Option<String>,
    pub title_uk:        Option<String>,
    pub content_en:      Option<String>,
    pub content_pl:      Option<String>,
    pub content_ru:      Option<String>,
    pub content_uk:      Option<String>,
    pub image_url:       Option<String>,
    pub seo_title:       Option<String>,
    pub seo_description: Option<String>,
    #[serde(default)]
    pub published:       bool,
    pub order_index:     Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateArticleRequest {
    pub slug:            Option<String>,
    pub category:        Option<String>,
    pub title_en:        Option<String>,
    pub title_pl:        Option<String>,
    pub title_ru:        Option<String>,
    pub title_uk:        Option<String>,
    pub content_en:      Option<String>,
    pub content_pl:      Option<String>,
    pub content_ru:      Option<String>,
    pub content_uk:      Option<String>,
    pub image_url:       Option<String>,
    pub seo_title:       Option<String>,
    pub seo_description: Option<String>,
    pub published:       Option<bool>,
    pub order_index:     Option<i32>,
}

// ── Pagination & Search ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ArticleQuery {
    pub page:     Option<i64>,
    pub limit:    Option<i64>,
    pub search:   Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ArticleListResponse {
    pub data:  Vec<ArticlePublicItem>,
    pub total: i64,
    pub page:  i64,
    pub limit: i64,
}

// ── Sitemap ───────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ArticleSitemapRow {
    pub slug:       String,
    pub updated_at: OffsetDateTime,
}

// ── Stats ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct PublicStats {
    pub articles_count:    i64,
    pub ingredients_count: i64,
    pub tools_count:       i64,
    pub experience_years:  i64,
    pub countries:         i64,
}

// ── Categories ────────────────────────────────────────────────────────────────

#[derive(Debug, sqlx::FromRow)]
pub struct ArticleCategoryRow {
    pub id:          Uuid,
    pub slug:        String,
    pub title_en:    String,
    pub title_pl:    String,
    pub title_ru:    String,
    pub title_uk:    String,
    pub order_index: i32,
    pub created_at:  OffsetDateTime,
}

/// Public DTO — ISO 8601 date string instead of array
#[derive(Debug, Serialize)]
pub struct ArticleCategoryPublic {
    pub id:          Uuid,
    pub slug:        String,
    pub title_en:    String,
    pub title_pl:    String,
    pub title_ru:    String,
    pub title_uk:    String,
    pub order_index: i32,
    pub created_at:  String,
}

impl From<ArticleCategoryRow> for ArticleCategoryPublic {
    fn from(r: ArticleCategoryRow) -> Self {
        Self {
            id:          r.id,
            slug:        r.slug,
            title_en:    r.title_en,
            title_pl:    r.title_pl,
            title_ru:    r.title_ru,
            title_uk:    r.title_uk,
            order_index: r.order_index,
            created_at:  r.created_at.format(&time::format_description::well_known::Rfc3339)
                          .unwrap_or_default(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SERVICE
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Clone)]
pub struct CmsService {
    pool:      PgPool,
    r2_client: crate::infrastructure::R2Client,
}

impl CmsService {
    pub fn new(pool: PgPool, r2_client: crate::infrastructure::R2Client) -> Self {
        Self { pool, r2_client }
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
        .map_err(|e| { tracing::error!("update_about: {e}"); AppError::internal("Failed to update about page") })
    }

    // ── EXPERTISE ─────────────────────────────────────────────────────────────

    pub async fn list_expertise(&self) -> AppResult<Vec<ExpertiseRow>> {
        sqlx::query_as("SELECT * FROM expertise ORDER BY order_index ASC, title_en ASC")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| { tracing::error!("list_expertise: {e}"); AppError::internal("DB error") })
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
        .map_err(|e| { tracing::error!("create_expertise: {e}"); AppError::internal("Failed to create expertise") })
    }

    pub async fn update_expertise(&self, id: Uuid, req: UpdateExpertiseRequest) -> AppResult<ExpertiseRow> {
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
            .bind(id).execute(&self.pool).await?;
        if r.rows_affected() == 0 { return Err(AppError::not_found("Expertise not found")); }
        Ok(())
    }

    // ── EXPERIENCE ────────────────────────────────────────────────────────────

    pub async fn list_experience(&self) -> AppResult<Vec<ExperienceRow>> {
        sqlx::query_as("SELECT * FROM experience ORDER BY order_index ASC, start_year DESC")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| { tracing::error!("list_experience: {e}"); AppError::internal("DB error") })
    }

    pub async fn create_experience(&self, req: CreateExperienceRequest) -> AppResult<ExperienceRow> {
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
        .map_err(|e| { tracing::error!("create_experience: {e}"); AppError::internal("Failed to create") })
    }

    pub async fn update_experience(&self, id: Uuid, req: UpdateExperienceRequest) -> AppResult<ExperienceRow> {
        let cur: ExperienceRow = sqlx::query_as("SELECT * FROM experience WHERE id = $1")
            .bind(id).fetch_optional(&self.pool).await?
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
        .map_err(|e| { tracing::error!("update_experience: {e}"); AppError::internal("Failed to update") })
    }

    pub async fn delete_experience(&self, id: Uuid) -> AppResult<()> {
        let r = sqlx::query("DELETE FROM experience WHERE id = $1")
            .bind(id).execute(&self.pool).await?;
        if r.rows_affected() == 0 { return Err(AppError::not_found("Experience not found")); }
        Ok(())
    }

    // ── GALLERY ───────────────────────────────────────────────────────────────

    pub async fn list_gallery(&self, category: Option<&str>) -> AppResult<Vec<GalleryPublicItem>> {
        let rows: Vec<GalleryRow> = match category.filter(|s| !s.is_empty()) {
            Some(cat) => sqlx::query_as(
                "SELECT * FROM gallery WHERE category = $1 ORDER BY order_index ASC, created_at DESC",
            )
            .bind(cat)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| { tracing::error!("list_gallery: {e}"); AppError::internal("DB error") })?,
            None => sqlx::query_as("SELECT * FROM gallery ORDER BY order_index ASC, created_at DESC")
                .fetch_all(&self.pool)
                .await
                .map_err(|e| { tracing::error!("list_gallery: {e}"); AppError::internal("DB error") })?,
        };
        Ok(rows.into_iter().map(GalleryPublicItem::from).collect())
    }

    pub async fn create_gallery(&self, req: CreateGalleryRequest) -> AppResult<GalleryRow> {
        sqlx::query_as(
            r#"INSERT INTO gallery
               (image_url, category, title_en, title_pl, title_ru, title_uk,
                description_en, description_pl, description_ru, description_uk,
                alt_en, alt_pl, alt_ru, alt_uk, order_index,
                instagram_url, pinterest_url, facebook_url, tiktok_url, website_url)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20)
               RETURNING *"#,
        )
        .bind(&req.image_url)
        .bind(req.category.unwrap_or_default())
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
        .map_err(|e| { tracing::error!("create_gallery: {e}"); AppError::internal("Failed to create") })
    }

    pub async fn update_gallery(&self, id: Uuid, req: UpdateGalleryRequest) -> AppResult<GalleryRow> {
        let cur: GalleryRow = sqlx::query_as("SELECT * FROM gallery WHERE id = $1")
            .bind(id).fetch_optional(&self.pool).await?
            .ok_or_else(|| AppError::not_found("Gallery item not found"))?;

        sqlx::query_as(
            r#"UPDATE gallery SET
               image_url=$1, category=$2, title_en=$3, title_pl=$4, title_ru=$5, title_uk=$6,
               description_en=$7, description_pl=$8, description_ru=$9, description_uk=$10,
               alt_en=$11, alt_pl=$12, alt_ru=$13, alt_uk=$14,
               order_index=$15,
               instagram_url=$16, pinterest_url=$17, facebook_url=$18, tiktok_url=$19, website_url=$20
               WHERE id=$21 RETURNING *"#,
        )
        .bind(req.image_url.unwrap_or(cur.image_url))
        .bind(req.category.unwrap_or(cur.category))
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
            .bind(id).execute(&self.pool).await?;
        if r.rows_affected() == 0 { return Err(AppError::not_found("Gallery item not found")); }
        Ok(())
    }

    // ── KNOWLEDGE ARTICLES ────────────────────────────────────────────────────

    /// Admin: list all articles (including drafts)
    pub async fn list_articles_admin(&self, category: Option<&str>) -> AppResult<Vec<ArticleRow>> {
        match category.filter(|s| !s.is_empty()) {
            Some(cat) => sqlx::query_as(
                "SELECT * FROM knowledge_articles WHERE category = $1 ORDER BY order_index ASC, created_at DESC",
            )
            .bind(cat)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| { tracing::error!("list_articles_admin: {e}"); AppError::internal("DB error") }),

            None => sqlx::query_as(
                "SELECT * FROM knowledge_articles ORDER BY order_index ASC, created_at DESC",
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| { tracing::error!("list_articles_admin: {e}"); AppError::internal("DB error") }),
        }
    }

    /// Public: list only published articles
    pub async fn list_articles_public(&self) -> AppResult<Vec<ArticleRow>> {
        sqlx::query_as(
            "SELECT * FROM knowledge_articles WHERE published = true ORDER BY order_index ASC, created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| { tracing::error!("list_articles_public: {e}"); AppError::internal("DB error") })
    }

    /// Public: get single article by slug (published only)
    pub async fn get_article_by_slug(&self, slug: &str) -> AppResult<ArticleRow> {
        sqlx::query_as(
            "SELECT * FROM knowledge_articles WHERE slug = $1 AND published = true",
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::not_found("Article not found"))
    }

    /// Admin: get article by id (any status)
    pub async fn get_article_by_id(&self, id: Uuid) -> AppResult<ArticleRow> {
        sqlx::query_as("SELECT * FROM knowledge_articles WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::not_found("Article not found"))
    }

    pub async fn create_article(&self, req: CreateArticleRequest) -> AppResult<ArticleRow> {
        // Auto-generate slug from title_en if not provided
        let slug = match req.slug.as_deref() {
            Some(s) if !s.trim().is_empty() => s.trim().to_string(),
            _ => slugify(&req.title_en),
        };

        sqlx::query_as(
            r#"INSERT INTO knowledge_articles
               (slug, category, title_en, title_pl, title_ru, title_uk,
                content_en, content_pl, content_ru, content_uk,
                image_url, seo_title, seo_description, published, order_index)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)
               RETURNING *"#,
        )
        .bind(&slug)
        .bind(req.category.unwrap_or_default())
        .bind(&req.title_en)
        .bind(req.title_pl.unwrap_or_default())
        .bind(req.title_ru.unwrap_or_default())
        .bind(req.title_uk.unwrap_or_default())
        .bind(req.content_en.unwrap_or_default())
        .bind(req.content_pl.unwrap_or_default())
        .bind(req.content_ru.unwrap_or_default())
        .bind(req.content_uk.unwrap_or_default())
        .bind(&req.image_url)
        .bind(req.seo_title.unwrap_or_else(|| req.title_en.clone()))
        .bind(req.seo_description.unwrap_or_default())
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

    pub async fn update_article(&self, id: Uuid, req: UpdateArticleRequest) -> AppResult<ArticleRow> {
        let cur = self.get_article_by_id(id).await?;

        sqlx::query_as(
            r#"UPDATE knowledge_articles SET
               slug=$1, category=$2, title_en=$3, title_pl=$4, title_ru=$5, title_uk=$6,
               content_en=$7, content_pl=$8, content_ru=$9, content_uk=$10,
               image_url=$11, seo_title=$12, seo_description=$13,
               published=$14, order_index=$15, updated_at=NOW()
               WHERE id=$16 RETURNING *"#,
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
        .bind(req.seo_title.unwrap_or(cur.seo_title))
        .bind(req.seo_description.unwrap_or(cur.seo_description))
        .bind(req.published.unwrap_or(cur.published))
        .bind(req.order_index.unwrap_or(cur.order_index))
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| { tracing::error!("update_article: {e}"); AppError::internal("Failed to update") })
    }

    pub async fn delete_article(&self, id: Uuid) -> AppResult<()> {
        let r = sqlx::query("DELETE FROM knowledge_articles WHERE id = $1")
            .bind(id).execute(&self.pool).await?;
        if r.rows_affected() == 0 { return Err(AppError::not_found("Article not found")); }
        Ok(())
    }

    // ── ARTICLES: pagination + search ─────────────────────────────────────────

    pub async fn list_articles_paged(&self, q: &ArticleQuery) -> AppResult<ArticleListResponse> {
        let page  = q.page.unwrap_or(1).max(1);
        let limit = q.limit.unwrap_or(20).clamp(1, 100);
        let offset = (page - 1) * limit;

        let category_filter = q.category.as_deref().filter(|s| !s.is_empty());

        let (total, rows) = if let Some(search) = q.search.as_deref().filter(|s| !s.is_empty()) {
            let pattern = format!("%{}%", search.to_lowercase());

            let (count_sql, list_sql) = if let Some(cat) = category_filter {
                (
                    format!("SELECT COUNT(*) FROM knowledge_articles WHERE published = true AND category = '{cat}'
                     AND (LOWER(title_en) LIKE $1 OR LOWER(title_ru) LIKE $1
                          OR LOWER(title_pl) LIKE $1 OR LOWER(title_uk) LIKE $1
                          OR LOWER(content_en) LIKE $1)"),
                    format!("SELECT * FROM knowledge_articles WHERE published = true AND category = '{cat}'
                     AND (LOWER(title_en) LIKE $1 OR LOWER(title_ru) LIKE $1
                          OR LOWER(title_pl) LIKE $1 OR LOWER(title_uk) LIKE $1
                          OR LOWER(content_en) LIKE $1)
                     ORDER BY order_index ASC, created_at DESC LIMIT $2 OFFSET $3"),
                )
            } else {
                (
                    "SELECT COUNT(*) FROM knowledge_articles WHERE published = true
                     AND (LOWER(title_en) LIKE $1 OR LOWER(title_ru) LIKE $1
                          OR LOWER(title_pl) LIKE $1 OR LOWER(title_uk) LIKE $1
                          OR LOWER(content_en) LIKE $1)".to_string(),
                    "SELECT * FROM knowledge_articles WHERE published = true
                     AND (LOWER(title_en) LIKE $1 OR LOWER(title_ru) LIKE $1
                          OR LOWER(title_pl) LIKE $1 OR LOWER(title_uk) LIKE $1
                          OR LOWER(content_en) LIKE $1)
                     ORDER BY order_index ASC, created_at DESC LIMIT $2 OFFSET $3".to_string(),
                )
            };

            let total: i64 = sqlx::query_scalar(&count_sql)
                .bind(&pattern).fetch_one(&self.pool).await.unwrap_or(0);
            let rows: Vec<ArticleRow> = sqlx::query_as(&list_sql)
                .bind(&pattern).bind(limit).bind(offset)
                .fetch_all(&self.pool).await
                .map_err(|e| { tracing::error!("list_articles_paged search: {e}"); AppError::internal("DB error") })?;
            (total, rows)

        } else if let Some(cat) = category_filter {
            let total: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM knowledge_articles WHERE published = true AND category = $1",
            ).bind(cat).fetch_one(&self.pool).await.unwrap_or(0);

            let rows: Vec<ArticleRow> = sqlx::query_as(
                "SELECT * FROM knowledge_articles WHERE published = true AND category = $1
                 ORDER BY order_index ASC, created_at DESC LIMIT $2 OFFSET $3",
            ).bind(cat).bind(limit).bind(offset)
            .fetch_all(&self.pool).await
            .map_err(|e| { tracing::error!("list_articles_paged category: {e}"); AppError::internal("DB error") })?;

            (total, rows)
        } else {
            let total: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM knowledge_articles WHERE published = true",
            ).fetch_one(&self.pool).await.unwrap_or(0);

            let rows: Vec<ArticleRow> = sqlx::query_as(
                "SELECT * FROM knowledge_articles WHERE published = true
                 ORDER BY order_index ASC, created_at DESC LIMIT $1 OFFSET $2",
            ).bind(limit).bind(offset)
            .fetch_all(&self.pool).await
            .map_err(|e| { tracing::error!("list_articles_paged: {e}"); AppError::internal("DB error") })?;

            (total, rows)
        };

        let data = rows.into_iter().map(ArticlePublicItem::from).collect();
        Ok(ArticleListResponse { data, total, page, limit })
    }

    // ── SITEMAP ───────────────────────────────────────────────────────────────

    pub async fn articles_sitemap(&self) -> AppResult<Vec<ArticleSitemapRow>> {
        sqlx::query_as(
            "SELECT slug, updated_at FROM knowledge_articles
             WHERE published = true ORDER BY updated_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| { tracing::error!("articles_sitemap: {e}"); AppError::internal("DB error") })
    }

    // ── STATS ─────────────────────────────────────────────────────────────────

    pub async fn public_stats(&self) -> AppResult<PublicStats> {
        let articles_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM knowledge_articles WHERE published = true",
        )
        .fetch_one(&self.pool)
        .await.unwrap_or(0);

        let ingredients_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM catalog_ingredients WHERE deleted_at IS NULL",
        )
        .fetch_one(&self.pool)
        .await.unwrap_or(0);

        // Count distinct public tool routes (static number, update as tools grow)
        let tools_count: i64 = 18;

        let experience_years: i64 = sqlx::query_scalar(
            "SELECT COALESCE(EXTRACT(YEAR FROM NOW())::bigint - MIN(start_year)::bigint, 0)
             FROM experience WHERE start_year IS NOT NULL",
        )
        .fetch_one(&self.pool)
        .await.unwrap_or(20);

        let countries: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT country) FROM experience WHERE country <> ''",
        )
        .fetch_one(&self.pool)
        .await.unwrap_or(0);

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
        let rows: Vec<ArticleCategoryRow> = sqlx::query_as(
            "SELECT * FROM article_categories ORDER BY order_index ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| { tracing::error!("list_categories: {e}"); AppError::internal("DB error") })?;
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
            ct if ct.contains("png")  => "png",
            ct if ct.contains("gif")  => "gif",
            _ => "webp",
        };
        let id  = Uuid::new_v4();
        let key = format!("cms/{}/{}.{}", folder, id, ext);

        let upload_url = self.r2_client.generate_presigned_upload_url(&key, content_type).await?;
        let public_url = self.r2_client.get_public_url(&key);

        Ok(crate::application::user::AvatarUploadResponse { upload_url, public_url })
    }

    // ── GALLERY (updated with alt fields) ─────────────────────────────────────

    pub async fn create_gallery_v2(&self, req: CreateGalleryRequest) -> AppResult<GalleryRow> {
        sqlx::query_as(
            r#"INSERT INTO gallery
               (image_url, category, title_en, title_pl, title_ru, title_uk,
                description_en, description_pl, description_ru, description_uk,
                alt_en, alt_pl, alt_ru, alt_uk, order_index)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)
               RETURNING *"#,
        )
        .bind(&req.image_url)
        .bind(req.category.unwrap_or_default())
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
        .fetch_one(&self.pool)
        .await
        .map_err(|e| { tracing::error!("create_gallery_v2: {e}"); AppError::internal("Failed to create") })
    }

    pub async fn update_gallery_v2(&self, id: Uuid, req: UpdateGalleryRequest) -> AppResult<GalleryRow> {
        let cur: GalleryRow = sqlx::query_as("SELECT * FROM gallery WHERE id = $1")
            .bind(id).fetch_optional(&self.pool).await?
            .ok_or_else(|| AppError::not_found("Gallery item not found"))?;

        sqlx::query_as(
            r#"UPDATE gallery SET
               image_url=$1, category=$2, title_en=$3, title_pl=$4, title_ru=$5, title_uk=$6,
               description_en=$7, description_pl=$8, description_ru=$9, description_uk=$10,
               alt_en=$11, alt_pl=$12, alt_ru=$13, alt_uk=$14, order_index=$15
               WHERE id=$16 RETURNING *"#,
        )
        .bind(req.image_url.unwrap_or(cur.image_url))
        .bind(req.category.unwrap_or(cur.category))
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
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| { tracing::error!("update_gallery_v2: {e}"); AppError::internal("Failed to update") })
    }
}
