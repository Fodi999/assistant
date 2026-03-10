use crate::shared::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

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
    pub title_en:       String,
    pub title_pl:       String,
    pub title_ru:       String,
    pub title_uk:       String,
    pub description_en: String,
    pub description_pl: String,
    pub description_ru: String,
    pub description_uk: String,
    pub order_index:    i32,
    pub created_at:     OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateGalleryRequest {
    pub image_url:      String,
    pub title_en:       Option<String>,
    pub title_pl:       Option<String>,
    pub title_ru:       Option<String>,
    pub title_uk:       Option<String>,
    pub description_en: Option<String>,
    pub description_pl: Option<String>,
    pub description_ru: Option<String>,
    pub description_uk: Option<String>,
    pub order_index:    Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateGalleryRequest {
    pub image_url:      Option<String>,
    pub title_en:       Option<String>,
    pub title_pl:       Option<String>,
    pub title_ru:       Option<String>,
    pub title_uk:       Option<String>,
    pub description_en: Option<String>,
    pub description_pl: Option<String>,
    pub description_ru: Option<String>,
    pub description_uk: Option<String>,
    pub order_index:    Option<i32>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// KNOWLEDGE ARTICLES
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ArticleRow {
    pub id:              Uuid,
    pub slug:            String,
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

#[derive(Debug, Deserialize)]
pub struct CreateArticleRequest {
    pub slug:            String,
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

// ═══════════════════════════════════════════════════════════════════════════════
// SERVICE
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Clone)]
pub struct CmsService {
    pool: PgPool,
}

impl CmsService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
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

    pub async fn list_gallery(&self) -> AppResult<Vec<GalleryRow>> {
        sqlx::query_as("SELECT * FROM gallery ORDER BY order_index ASC, created_at DESC")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| { tracing::error!("list_gallery: {e}"); AppError::internal("DB error") })
    }

    pub async fn create_gallery(&self, req: CreateGalleryRequest) -> AppResult<GalleryRow> {
        sqlx::query_as(
            r#"INSERT INTO gallery
               (image_url, title_en, title_pl, title_ru, title_uk,
                description_en, description_pl, description_ru, description_uk, order_index)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
               RETURNING *"#,
        )
        .bind(&req.image_url)
        .bind(req.title_en.unwrap_or_default())
        .bind(req.title_pl.unwrap_or_default())
        .bind(req.title_ru.unwrap_or_default())
        .bind(req.title_uk.unwrap_or_default())
        .bind(req.description_en.unwrap_or_default())
        .bind(req.description_pl.unwrap_or_default())
        .bind(req.description_ru.unwrap_or_default())
        .bind(req.description_uk.unwrap_or_default())
        .bind(req.order_index.unwrap_or(0))
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
               image_url=$1, title_en=$2, title_pl=$3, title_ru=$4, title_uk=$5,
               description_en=$6, description_pl=$7, description_ru=$8, description_uk=$9,
               order_index=$10
               WHERE id=$11 RETURNING *"#,
        )
        .bind(req.image_url.unwrap_or(cur.image_url))
        .bind(req.title_en.unwrap_or(cur.title_en))
        .bind(req.title_pl.unwrap_or(cur.title_pl))
        .bind(req.title_ru.unwrap_or(cur.title_ru))
        .bind(req.title_uk.unwrap_or(cur.title_uk))
        .bind(req.description_en.unwrap_or(cur.description_en))
        .bind(req.description_pl.unwrap_or(cur.description_pl))
        .bind(req.description_ru.unwrap_or(cur.description_ru))
        .bind(req.description_uk.unwrap_or(cur.description_uk))
        .bind(req.order_index.unwrap_or(cur.order_index))
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
    pub async fn list_articles_admin(&self) -> AppResult<Vec<ArticleRow>> {
        sqlx::query_as("SELECT * FROM knowledge_articles ORDER BY order_index ASC, created_at DESC")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| { tracing::error!("list_articles_admin: {e}"); AppError::internal("DB error") })
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
        sqlx::query_as(
            r#"INSERT INTO knowledge_articles
               (slug, title_en, title_pl, title_ru, title_uk,
                content_en, content_pl, content_ru, content_uk,
                image_url, seo_title, seo_description, published, order_index)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)
               RETURNING *"#,
        )
        .bind(&req.slug)
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
                AppError::conflict(&format!("Article with slug '{}' already exists", req.slug))
            } else {
                AppError::internal("Failed to create article")
            }
        })
    }

    pub async fn update_article(&self, id: Uuid, req: UpdateArticleRequest) -> AppResult<ArticleRow> {
        let cur = self.get_article_by_id(id).await?;

        sqlx::query_as(
            r#"UPDATE knowledge_articles SET
               slug=$1, title_en=$2, title_pl=$3, title_ru=$4, title_uk=$5,
               content_en=$6, content_pl=$7, content_ru=$8, content_uk=$9,
               image_url=$10, seo_title=$11, seo_description=$12,
               published=$13, order_index=$14, updated_at=NOW()
               WHERE id=$15 RETURNING *"#,
        )
        .bind(req.slug.unwrap_or(cur.slug))
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
}
