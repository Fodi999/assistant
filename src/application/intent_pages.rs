//! Intent Pages Service — full pSEO pipeline with queue system
//!
//! Statuses: draft → queued → published → archived
//!
//! Admin flow:
//!   POST  /admin/intent-pages/generate           → single page (→ draft)
//!   POST  /admin/intent-pages/generate-batch     → batch (→ draft)
//!   GET   /admin/intent-pages                    → list (filter by status/locale)
//!   GET   /admin/intent-pages/stats              → counts per status
//!   PUT   /admin/intent-pages/:id                → edit content
//!   POST  /admin/intent-pages/:id/publish        → force publish now
//!   POST  /admin/intent-pages/:id/unpublish      → back to draft
//!   POST  /admin/intent-pages/:id/enqueue        → draft → queued
//!   POST  /admin/intent-pages/:id/archive        → any → archived
//!   POST  /admin/intent-pages/enqueue-bulk       → bulk enqueue by IDs
//!   DELETE /admin/intent-pages/:id               → hard delete
//!
//!   GET   /admin/intent-pages/settings           → get publish_limit_per_day
//!   PUT   /admin/intent-pages/settings           → set publish_limit_per_day
//!   POST  /admin/intent-pages/scheduler/run      → trigger scheduler manually
//!
//! Public flow:
//!   GET /public/intent-pages?locale=en            → published pages
//!   GET /public/intent-pages/:slug?locale=en      → single published page
//!
//! Scheduler (background task):
//!   Every hour checks: if queued pages exist AND daily limit not exhausted
//!   → publishes next N pages from queue (FIFO by queued_at)

use crate::application::admin_catalog::revalidate_blog;
use crate::application::public_seo_content::{
    PublicSeoContentService, SeoContentRequest,
};
use crate::infrastructure::R2Client;
use crate::shared::{AppError, AppResult};
use deunicode::deunicode;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

// ── Sub-intent templates (long-tail keywords) ────────────────────────────────

/// Each product generates multiple pages via these sub-intent templates.
/// `{a}` = entity_a slug, `{b}` = entity_b slug
///
/// These match REAL Google search queries (high-volume long-tail).
const QUESTION_SUB_INTENTS: &[&str] = &[
    "is-{a}-healthy",
    "{a}-calories",
    "{a}-protein",
    "is-{a}-good-for-weight-loss",
    "{a}-benefits",
    "how-to-cook-{a}",
    "{a}-side-effects",
    "is-{a}-good-for-skin",
];

const GOAL_SUB_INTENTS: &[&str] = &[
    "best-{a}-for-diet",
    "{a}-for-muscle-building",
    "{a}-for-pregnancy",
    "{a}-for-kids",
];

const COMPARISON_SUB_INTENTS: &[&str] = &[
    "{a}-vs-{b}",
    "{a}-vs-{b}-nutrition",
    "which-is-healthier-{a}-or-{b}",
];

/// Heuristic: does this English ingredient name look plural?
/// Covers common patterns: almonds, eggs, beans, oats, carrots, etc.
fn looks_plural(word: &str) -> bool {
    let w = word.to_lowercase();
    // Explicit singular exceptions that end in 's' but aren't plural
    const SINGULAR_EXCEPTIONS: &[&str] = &[
        "hummus", "couscous", "asparagus", "citrus", "hibiscus",
        "octopus", "lettuce", "rice", "quinoa", "tofu",
    ];
    if SINGULAR_EXCEPTIONS.iter().any(|&ex| w == ex) {
        return false;
    }
    // Common English plural endings
    w.ends_with('s')
        && !w.ends_with("ss")   // "grass" is not plural-looking
        && !w.ends_with("us")   // "asparagus"
}

/// Map a sub-intent slug template to a natural search query for the AI prompt.
/// Handles is/are grammar: "Is salmon healthy?" vs "Are almonds healthy?"
fn sub_intent_to_query(sub: &str, entity_a: &str, entity_b: Option<&str>) -> String {
    let q = sub
        .replace("{a}", entity_a)
        .replace("{b}", entity_b.unwrap_or(""))
        .replace('-', " ");

    // Fix is/are agreement based on whether entity_a looks plural
    let q = if looks_plural(entity_a) {
        // "is almonds healthy" → "are almonds healthy"
        // "which is healthier almonds or ..." → "which are healthier almonds or ..."
        q.replacen("is ", "are ", 1)
    } else {
        q
    };

    // Capitalize first letter
    let mut chars = q.chars();
    match chars.next() {
        Some(c) => format!("{}{}", c.to_uppercase(), chars.as_str()),
        None => q,
    }
}

fn sub_intent_to_slug(sub: &str, entity_a: &str, entity_b: Option<&str>) -> String {
    let slug = sub.replace("{a}", entity_a)
       .replace("{b}", entity_b.unwrap_or(""))
       .trim()
       .to_lowercase()
       .replace(' ', "-");

    // Clean up multiple dashes
    let slug = slug.split('-').filter(|s| !s.is_empty()).collect::<Vec<_>>().join("-");

    // Fix slug grammar: "is-almonds-healthy" → "are-almonds-healthy"
    if looks_plural(entity_a) {
        let slug = slug.replacen("is-", "are-", 1);
        slug.replacen("which-is-", "which-are-", 1)
    } else {
        slug
    }
}

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct IntentPage {
    pub id: Uuid,
    pub intent_type: String,
    pub entity_a: String,
    pub entity_b: Option<String>,
    pub locale: String,
    pub title: String,
    pub description: String,
    pub answer: String,
    pub faq: serde_json::Value,
    pub slug: String,
    pub status: String,
    pub priority: i32,
    pub content_blocks: serde_json::Value,
    pub published_at: Option<String>,
    pub queued_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct GenerateRequest {
    pub intent_type: String,
    pub entity_a: String,
    pub entity_b: Option<String>,
    pub locale: String,
    /// Specific sub-intent (e.g. "is-{a}-healthy"). If None, AI decides the title.
    pub sub_intent: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BatchGenerateRequest {
    /// Which intent types to generate. Default: ["question", "goal"]
    pub intent_types: Option<Vec<String>>,
    /// Which locales to generate. Default: all 4
    pub locales: Option<Vec<String>>,
    /// Max pages to generate in this batch (to control AI costs)
    pub limit: Option<i64>,
    /// Auto-publish generated pages (skip draft stage)
    pub auto_publish: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct BatchResult {
    pub generated: i32,
    pub published: i32,
    pub skipped: i32,
    pub errors: i32,
    pub details: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateIntentPageRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub answer: Option<String>,
    pub faq: Option<serde_json::Value>,
    pub slug: Option<String>,
    pub priority: Option<i32>,
    pub content_blocks: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub status: Option<String>,
    pub locale: Option<String>,
    pub entity_a: Option<String>,
    pub intent_type: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct PublicListQuery {
    pub locale: Option<String>,
    pub entity_a: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct PublicSlugQuery {
    pub locale: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EnqueueBulkRequest {
    pub ids: Vec<Uuid>,
    /// Priority for all enqueued pages (0=low, 1=normal, 2=high). Default: 1
    pub priority: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSettingsRequest {
    pub publish_limit_per_day: i64,
}

#[derive(Debug, Deserialize)]
pub struct BulkActionRequest {
    pub ids: Vec<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct DuplicateGroup {
    pub entity_a: String,
    pub locale: String,
    pub canonical_slug: String,
    pub pages: Vec<DuplicateEntry>,
}

#[derive(Debug, Serialize)]
pub struct DuplicateEntry {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub status: String,
    pub is_canonical: bool,
}

/// Related page for internal linking
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct RelatedPage {
    pub title: String,
    pub slug: String,
    pub intent_type: String,
    pub entity_a: String,
}

/// Presigned upload URL response for intent-page images
#[derive(Debug, Serialize)]
pub struct ImageUploadResponse {
    pub upload_url: String,
    pub public_url: String,
}

// ── Service ──────────────────────────────────────────────────────────────────

pub struct IntentPagesService {
    pool: PgPool,
    seo_service: Arc<PublicSeoContentService>,
    r2_client: R2Client,
}

impl IntentPagesService {
    pub fn new(pool: PgPool, seo_service: Arc<PublicSeoContentService>, r2_client: R2Client) -> Self {
        Self { pool, seo_service, r2_client }
    }

    // ── Generate single ──────────────────────────────────────────────────────

    pub async fn generate(&self, req: &GenerateRequest) -> AppResult<IntentPage> {
        // 1. Determine slug upfront (from sub_intent or will use AI title later)
        let predetermined_slug = req.sub_intent.as_ref().map(|si| {
            sub_intent_to_slug(si, &req.entity_a, req.entity_b.as_deref())
        });

        // 2. If we have a predetermined slug, check if page already exists
        if let Some(ref slug) = predetermined_slug {
            let existing = sqlx::query_as::<_, IntentPage>(
                r#"SELECT id, intent_type, entity_a, entity_b, locale,
                          title, description, answer, faq, slug, status, priority, content_blocks,
                          published_at::text, queued_at::text, created_at::text, updated_at::text
                   FROM intent_pages
                   WHERE slug = $1 AND locale = $2"#,
            )
            .bind(slug)
            .bind(&req.locale)
            .fetch_optional(&self.pool)
            .await?;

            if let Some(page) = existing {
                return Ok(page);
            }
        }

        // 3. Build search query for AI
        //    sub_intent gives us a specific user query to answer
        let search_query = req.sub_intent.as_ref().map(|si| {
            sub_intent_to_query(si, &req.entity_a, req.entity_b.as_deref())
        });

        // 4. Call AI to generate content
        let seo_req = SeoContentRequest {
            intent_type: req.intent_type.clone(),
            entity_a: req.entity_a.clone(),
            entity_b: req.entity_b.clone(),
            locale: req.locale.clone(),
        };

        // Pass the specific search query to the AI for better targeting
        let content = if let Some(ref query) = search_query {
            self.seo_service.generate_with_query(&seo_req, query).await?
        } else {
            self.seo_service.generate(&seo_req).await?
        };

        // 5. Use predetermined slug, or AI-generated slug, or fallback
        let slug = predetermined_slug.unwrap_or_else(|| {
            // Prefer AI-generated slug (semantic, in content language → auto-transliterated)
            if let Some(ref ai_slug) = content.slug {
                let cleaned = transliterate_to_slug(ai_slug.trim());
                if cleaned.len() >= 5 {
                    return cleaned;
                }
            }
            generate_slug(&content.title, &req.intent_type, &req.entity_a, req.entity_b.as_deref())
        });

        // 6. Insert into DB
        let faq_json = serde_json::to_value(&content.faq)
            .unwrap_or_else(|_| serde_json::json!([]));

        let content_blocks_json = serde_json::to_value(&content.content_blocks)
            .unwrap_or_else(|_| serde_json::json!([]));

        let page = sqlx::query_as::<_, IntentPage>(
            r#"INSERT INTO intent_pages
               (intent_type, entity_a, entity_b, locale, title, description, answer, faq, slug, status, content_blocks)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'draft', $10)
               ON CONFLICT (slug, locale) DO NOTHING
               RETURNING id, intent_type, entity_a, entity_b, locale,
                         title, description, answer, faq, slug, status, priority, content_blocks,
                         published_at::text, queued_at::text, created_at::text, updated_at::text"#,
        )
        .bind(&req.intent_type)
        .bind(&req.entity_a)
        .bind(&req.entity_b)
        .bind(&req.locale)
        .bind(&content.title)
        .bind(&content.description)
        .bind(&content.answer)
        .bind(&faq_json)
        .bind(&slug)
        .bind(&content_blocks_json)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::conflict("Intent page with this slug already exists"))?;

        tracing::info!(
            "✅ Intent page generated: {} / {} / {} → '{}' [{}]",
            req.intent_type, req.entity_a, req.locale, content.title, slug
        );

        Ok(page)
    }

    // ── Batch generate (sub-intent based) ───────────────────────────────────
    //
    // One product → 5-10 specific long-tail pages:
    //   "is-salmon-healthy", "salmon-calories", "salmon-protein", etc.
    //
    // Instead of one generic "salmon nutrition" page.

    pub async fn generate_batch(&self, req: &BatchGenerateRequest) -> AppResult<BatchResult> {
        let intent_types = req.intent_types.clone().unwrap_or_else(|| {
            vec!["question".into(), "goal".into()]
        });
        let locales = req.locales.clone().unwrap_or_else(|| {
            vec!["en".into(), "pl".into(), "ru".into(), "uk".into()]
        });
        let limit = req.limit.unwrap_or(50);
        let auto_publish = req.auto_publish.unwrap_or(false);

        // Get all published product slugs
        let slugs: Vec<String> = sqlx::query_scalar(
            "SELECT slug FROM catalog_ingredients WHERE is_published = true AND is_active = true ORDER BY name_en"
        )
        .fetch_all(&self.pool)
        .await?;

        if slugs.is_empty() {
            return Ok(BatchResult {
                generated: 0, published: 0, skipped: 0, errors: 0,
                details: vec!["⚠️ No published products found".into()],
            });
        }

        let mut result = BatchResult {
            generated: 0, published: 0, skipped: 0, errors: 0, details: vec![],
        };
        let mut count = 0i64;

        // ── Phase 1: question sub-intents (8 per product) ────────────────────
        if intent_types.iter().any(|t| t == "question") {
            for slug in &slugs {
                if count >= limit { break; }

                for sub in QUESTION_SUB_INTENTS {
                    if count >= limit { break; }
                    let target_slug = sub_intent_to_slug(sub, slug, None);

                    for locale in &locales {
                        if count >= limit { break; }

                        // Fast check: slug already in DB?
                        let exists: bool = sqlx::query_scalar(
                            "SELECT EXISTS(SELECT 1 FROM intent_pages WHERE slug = $1 AND locale = $2)"
                        )
                        .bind(&target_slug)
                        .bind(locale)
                        .fetch_one(&self.pool)
                        .await
                        .unwrap_or(true);

                        if exists { result.skipped += 1; continue; }

                        let gen_req = GenerateRequest {
                            intent_type: "question".into(),
                            entity_a: slug.clone(),
                            entity_b: None,
                            locale: locale.clone(),
                            sub_intent: Some(sub.to_string()),
                        };

                        match self.generate(&gen_req).await {
                            Ok(page) => {
                                result.generated += 1;
                                result.details.push(format!("✅ {} / {} → '{}'",
                                    target_slug, locale, page.title));
                                count += 1;
                                if auto_publish {
                                    if let Ok(_) = self.publish(page.id).await {
                                        result.published += 1;
                                    }
                                }
                            }
                            Err(e) => {
                                result.errors += 1;
                                result.details.push(format!("❌ {} / {} → {}", target_slug, locale, e));
                            }
                        }
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    }
                }
            }
        }

        // ── Phase 2: goal sub-intents (4 per product) ────────────────────────
        if intent_types.iter().any(|t| t == "goal") {
            for slug in &slugs {
                if count >= limit { break; }

                for sub in GOAL_SUB_INTENTS {
                    if count >= limit { break; }
                    let target_slug = sub_intent_to_slug(sub, slug, None);

                    for locale in &locales {
                        if count >= limit { break; }

                        let exists: bool = sqlx::query_scalar(
                            "SELECT EXISTS(SELECT 1 FROM intent_pages WHERE slug = $1 AND locale = $2)"
                        )
                        .bind(&target_slug)
                        .bind(locale)
                        .fetch_one(&self.pool)
                        .await
                        .unwrap_or(true);

                        if exists { result.skipped += 1; continue; }

                        let gen_req = GenerateRequest {
                            intent_type: "goal".into(),
                            entity_a: slug.clone(),
                            entity_b: None,
                            locale: locale.clone(),
                            sub_intent: Some(sub.to_string()),
                        };

                        match self.generate(&gen_req).await {
                            Ok(page) => {
                                result.generated += 1;
                                result.details.push(format!("✅ {} / {} → '{}'",
                                    target_slug, locale, page.title));
                                count += 1;
                                if auto_publish {
                                    if let Ok(_) = self.publish(page.id).await {
                                        result.published += 1;
                                    }
                                }
                            }
                            Err(e) => {
                                result.errors += 1;
                                result.details.push(format!("❌ {} / {} → {}", target_slug, locale, e));
                            }
                        }
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    }
                }
            }
        }

        // ── Phase 3: comparison sub-intents (3 per pair) ─────────────────────
        if intent_types.iter().any(|t| t == "comparison") && slugs.len() >= 2 {
            for i in 0..slugs.len() {
                if count >= limit { break; }
                let slug_a = &slugs[i];
                let slug_b = &slugs[(i + 1) % slugs.len()];
                if slug_a == slug_b { continue; }

                for sub in COMPARISON_SUB_INTENTS {
                    if count >= limit { break; }
                    let target_slug = sub_intent_to_slug(sub, slug_a, Some(slug_b));

                    for locale in &locales {
                        if count >= limit { break; }

                        let exists: bool = sqlx::query_scalar(
                            "SELECT EXISTS(SELECT 1 FROM intent_pages WHERE slug = $1 AND locale = $2)"
                        )
                        .bind(&target_slug)
                        .bind(locale)
                        .fetch_one(&self.pool)
                        .await
                        .unwrap_or(true);

                        if exists { result.skipped += 1; continue; }

                        let gen_req = GenerateRequest {
                            intent_type: "comparison".into(),
                            entity_a: slug_a.clone(),
                            entity_b: Some(slug_b.clone()),
                            locale: locale.clone(),
                            sub_intent: Some(sub.to_string()),
                        };

                        match self.generate(&gen_req).await {
                            Ok(page) => {
                                result.generated += 1;
                                result.details.push(format!("✅ {} / {} → '{}'",
                                    target_slug, locale, page.title));
                                count += 1;
                                if auto_publish {
                                    if let Ok(_) = self.publish(page.id).await {
                                        result.published += 1;
                                    }
                                }
                            }
                            Err(e) => {
                                result.errors += 1;
                                result.details.push(format!("❌ {} / {} → {}", target_slug, locale, e));
                            }
                        }
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    }
                }
            }
        }

        tracing::info!(
            "📦 Batch done: {} generated, {} published, {} skipped, {} errors",
            result.generated, result.published, result.skipped, result.errors
        );

        Ok(result)
    }

    // ── List (admin) ─────────────────────────────────────────────────────────

    pub async fn list(&self, q: &ListQuery) -> AppResult<Vec<IntentPage>> {
        let limit = q.limit.unwrap_or(50).min(200);
        let offset = q.offset.unwrap_or(0);

        let pages = sqlx::query_as::<_, IntentPage>(
            r#"SELECT id, intent_type, entity_a, entity_b, locale,
                      title, description, answer, faq, slug, status, priority, content_blocks,
                      published_at::text, queued_at::text, created_at::text, updated_at::text
               FROM intent_pages
               WHERE ($1::text IS NULL OR status = $1)
                 AND ($2::text IS NULL OR locale = $2)
                 AND ($3::text IS NULL OR entity_a = $3)
                 AND ($4::text IS NULL OR intent_type = $4)
               ORDER BY created_at DESC
               LIMIT $5 OFFSET $6"#,
        )
        .bind(&q.status)
        .bind(&q.locale)
        .bind(&q.entity_a)
        .bind(&q.intent_type)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(pages)
    }

    // ── Get by ID ────────────────────────────────────────────────────────────

    pub async fn get_by_id(&self, id: Uuid) -> AppResult<IntentPage> {
        sqlx::query_as::<_, IntentPage>(
            r#"SELECT id, intent_type, entity_a, entity_b, locale,
                      title, description, answer, faq, slug, status, priority, content_blocks,
                      published_at::text, queued_at::text, created_at::text, updated_at::text
               FROM intent_pages WHERE id = $1"#,
        )
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::not_found("Intent page not found"))
    }

    // ── Update ───────────────────────────────────────────────────────────────

    pub async fn update(&self, id: Uuid, req: &UpdateIntentPageRequest) -> AppResult<IntentPage> {
        let current = self.get_by_id(id).await?;

        let page = sqlx::query_as::<_, IntentPage>(
            r#"UPDATE intent_pages SET
                title = COALESCE($1, title),
                description = COALESCE($2, description),
                answer = COALESCE($3, answer),
                faq = COALESCE($4, faq),
                slug = COALESCE($5, slug),
                priority = COALESCE($6, priority),
                content_blocks = COALESCE($7, content_blocks)
               WHERE id = $8
               RETURNING id, intent_type, entity_a, entity_b, locale,
                         title, description, answer, faq, slug, status, priority, content_blocks,
                         published_at::text, queued_at::text, created_at::text, updated_at::text"#,
        )
        .bind(&req.title)
        .bind(&req.description)
        .bind(&req.answer)
        .bind(&req.faq)
        .bind(&req.slug)
        .bind(&req.priority)
        .bind(&req.content_blocks)
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        // If published, trigger revalidation
        if current.status == "published" {
            tokio::spawn(revalidate_blog(Some(current.entity_a.clone())));
        }

        Ok(page)
    }

    // ── Publish ──────────────────────────────────────────────────────────────

    pub async fn publish(&self, id: Uuid) -> AppResult<IntentPage> {
        let page = sqlx::query_as::<_, IntentPage>(
            r#"UPDATE intent_pages
               SET status = 'published', published_at = NOW()
               WHERE id = $1
               RETURNING id, intent_type, entity_a, entity_b, locale,
                         title, description, answer, faq, slug, status, priority, content_blocks,
                         published_at::text, queued_at::text, created_at::text, updated_at::text"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::not_found("Intent page not found"))?;

        // Trigger blog revalidation
        tokio::spawn(revalidate_blog(Some(page.entity_a.clone())));

        tracing::info!("📢 Intent page published: '{}' ({})", page.title, page.locale);
        Ok(page)
    }

    // ── Unpublish ────────────────────────────────────────────────────────────

    pub async fn unpublish(&self, id: Uuid) -> AppResult<IntentPage> {
        let page = sqlx::query_as::<_, IntentPage>(
            r#"UPDATE intent_pages
               SET status = 'draft', published_at = NULL
               WHERE id = $1
               RETURNING id, intent_type, entity_a, entity_b, locale,
                         title, description, answer, faq, slug, status, priority, content_blocks,
                         published_at::text, queued_at::text, created_at::text, updated_at::text"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::not_found("Intent page not found"))?;

        tokio::spawn(revalidate_blog(Some(page.entity_a.clone())));

        tracing::info!("Intent page unpublished: '{}' ({})", page.title, page.locale);
        Ok(page)
    }

    // ── Delete ───────────────────────────────────────────────────────────────

    pub async fn delete(&self, id: Uuid) -> AppResult<()> {
        let page = self.get_by_id(id).await?;

        sqlx::query("DELETE FROM intent_pages WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if page.status == "published" {
            tokio::spawn(revalidate_blog(Some(page.entity_a.clone())));
        }

        tracing::info!("🗑️ Intent page deleted: '{}' ({})", page.title, page.locale);
        Ok(())
    }

    // ── Regenerate (clear AI cache + delete + generate fresh) ────────────────

    pub async fn regenerate(&self, id: Uuid) -> AppResult<IntentPage> {
        let page = self.get_by_id(id).await?;

        // 1. Clear AI cache so we get fresh content from LLM
        let seo_req = SeoContentRequest {
            intent_type: page.intent_type.clone(),
            entity_a: page.entity_a.clone(),
            entity_b: page.entity_b.clone(),
            locale: page.locale.clone(),
        };

        // Invalidate both generic and all sub-intent caches for this entity
        self.seo_service.invalidate_cache(&seo_req).await;

        // Also try to figure out the sub_intent/search_query from the slug
        // and invalidate that specific cache too
        let search_query = sub_intent_to_query(
            &page.slug,
            &page.entity_a,
            page.entity_b.as_deref(),
        );
        self.seo_service.invalidate_cache_with_query(&seo_req, &search_query).await;

        // 2. Delete the page from DB
        sqlx::query("DELETE FROM intent_pages WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if page.status == "published" {
            tokio::spawn(revalidate_blog(Some(page.entity_a.clone())));
        }

        tracing::info!("🔄 Regenerating intent page: '{}' ({}) — cache cleared", page.title, page.locale);

        // 3. Generate fresh (will call LLM since cache is cleared)
        let gen_req = GenerateRequest {
            intent_type: page.intent_type,
            entity_a: page.entity_a,
            entity_b: page.entity_b,
            locale: page.locale,
            sub_intent: None,
        };
        self.generate(&gen_req).await
    }

    // ── Public: list published ───────────────────────────────────────────────

    pub async fn list_published(&self, q: &PublicListQuery) -> AppResult<Vec<IntentPage>> {
        let locale = q.locale.as_deref().unwrap_or("en");
        let limit = q.limit.unwrap_or(50).min(200);
        let offset = q.offset.unwrap_or(0);

        let pages = sqlx::query_as::<_, IntentPage>(
            r#"SELECT id, intent_type, entity_a, entity_b, locale,
                      title, description, answer, faq, slug, status, priority, content_blocks,
                      published_at::text, queued_at::text, created_at::text, updated_at::text
               FROM intent_pages
               WHERE status = 'published'
                 AND locale = $1
                 AND ($2::text IS NULL OR entity_a = $2)
               ORDER BY published_at DESC
               LIMIT $3 OFFSET $4"#,
        )
        .bind(locale)
        .bind(&q.entity_a)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(pages)
    }

    // ── Public: get by slug ──────────────────────────────────────────────────

    pub async fn get_by_slug(&self, slug: &str, locale: &str) -> AppResult<IntentPage> {
        sqlx::query_as::<_, IntentPage>(
            r#"SELECT id, intent_type, entity_a, entity_b, locale,
                      title, description, answer, faq, slug, status, priority, content_blocks,
                      published_at::text, queued_at::text, created_at::text, updated_at::text
               FROM intent_pages
               WHERE slug = $1 AND locale = $2 AND status = 'published'"#,
        )
        .bind(slug)
        .bind(locale)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::not_found("Intent page not found"))
    }

    // ── Related pages (internal linking) ─────────────────────────────────────

    /// Returns up to 8 related published pages for internal linking.
    /// Priority:
    ///   0 — same entity_a (different intent) → cluster links
    ///   1 — pages that reference THIS entity as entity_b (reverse links: A→B makes B→A)
    ///   2 — same intent_type (different entity) → topical links
    pub async fn get_related(&self, slug: &str, locale: &str) -> AppResult<Vec<RelatedPage>> {
        let pages = sqlx::query_as::<_, RelatedPage>(
            r#"WITH current AS (
                 SELECT entity_a, entity_b, intent_type
                 FROM intent_pages
                 WHERE slug = $1 AND locale = $2 AND status = 'published'
                 LIMIT 1
               )
               SELECT DISTINCT ON (ip.slug) ip.title, ip.slug, ip.intent_type, ip.entity_a
               FROM intent_pages ip, current c
               WHERE ip.locale = $2
                 AND ip.status = 'published'
                 AND ip.slug != $1
                 AND (
                   ip.entity_a = c.entity_a
                   OR (c.entity_a IS NOT NULL AND ip.entity_b = c.entity_a)
                   OR (c.entity_b IS NOT NULL AND ip.entity_a = c.entity_b)
                   OR ip.intent_type = c.intent_type
                 )
               ORDER BY
                 ip.slug,
                 CASE
                   WHEN ip.entity_a = c.entity_a THEN 0
                   WHEN c.entity_a IS NOT NULL AND ip.entity_b = c.entity_a THEN 1
                   WHEN c.entity_b IS NOT NULL AND ip.entity_a = c.entity_b THEN 1
                   ELSE 2
                 END,
                 ip.published_at DESC NULLS LAST
               LIMIT 8"#,
        )
        .bind(slug)
        .bind(locale)
        .fetch_all(&self.pool)
        .await?;

        Ok(pages)
    }

    /// Returns all published intent pages for a given ingredient (hub page).
    /// Used by the ingredient page to show "Articles about this ingredient".
    pub async fn list_for_ingredient(&self, entity_a: &str, locale: &str) -> AppResult<Vec<RelatedPage>> {
        let pages = sqlx::query_as::<_, RelatedPage>(
            r#"SELECT title, slug, intent_type, entity_a
               FROM intent_pages
               WHERE entity_a = $1
                 AND locale = $2
                 AND status = 'published'
               ORDER BY
                 CASE intent_type
                   WHEN 'question' THEN 0
                   WHEN 'goal' THEN 1
                   WHEN 'comparison' THEN 2
                   ELSE 3
                 END,
                 published_at DESC NULLS LAST
               LIMIT 20"#,
        )
        .bind(entity_a)
        .bind(locale)
        .fetch_all(&self.pool)
        .await?;

        Ok(pages)
    }

    // ── Image upload for content_blocks ──────────────────────────────────────

    /// Generate a presigned URL for uploading an intent-page image to R2.
    /// Key pattern: `assets/seo/{page_id}/{image_key}.webp`
    pub async fn get_image_upload_url(
        &self,
        page_id: Uuid,
        image_key: &str,
        content_type: &str,
    ) -> AppResult<ImageUploadResponse> {
        // Validate page exists
        let _ = self.get_by_id(page_id).await?;

        // Validate image_key
        const VALID_KEYS: &[&str] = &["hero", "benefits", "nutrition", "cooking"];
        if !VALID_KEYS.contains(&image_key) {
            return Err(AppError::validation(&format!(
                "Invalid image key '{}'. Must be one of: {}",
                image_key,
                VALID_KEYS.join(", ")
            )));
        }

        let ext = if content_type.contains("png") { "png" }
                  else if content_type.contains("jpeg") || content_type.contains("jpg") { "jpg" }
                  else { "webp" };
        let key = format!("assets/seo/{}/{}.{}", page_id, image_key, ext);

        let upload_url = self.r2_client
            .generate_presigned_upload_url(&key, content_type)
            .await?;
        let public_url = self.r2_client.get_public_url(&key);

        Ok(ImageUploadResponse { upload_url, public_url })
    }

    /// After frontend uploaded to R2, update the matching image block's `src` in content_blocks.
    pub async fn save_image_url(
        &self,
        page_id: Uuid,
        image_key: &str,
        image_url: String,
    ) -> AppResult<IntentPage> {
        let page = self.get_by_id(page_id).await?;

        // Patch content_blocks: find image block with this key and set src
        let mut blocks: Vec<serde_json::Value> = serde_json::from_value(page.content_blocks)
            .unwrap_or_default();

        for block in blocks.iter_mut() {
            if block.get("type").and_then(|v| v.as_str()) == Some("image")
                && block.get("key").and_then(|v| v.as_str()) == Some(image_key)
            {
                block.as_object_mut().map(|obj| {
                    obj.insert("src".into(), serde_json::Value::String(image_url.clone()));
                });
            }
        }

        let updated_blocks = serde_json::to_value(&blocks).unwrap_or_default();

        let updated = sqlx::query_as::<_, IntentPage>(
            r#"UPDATE intent_pages
               SET content_blocks = $1, updated_at = NOW()
               WHERE id = $2
               RETURNING id, intent_type, entity_a, entity_b, locale,
                         title, description, answer, faq, slug, status, priority, content_blocks,
                         published_at::text, queued_at::text, created_at::text, updated_at::text"#,
        )
        .bind(&updated_blocks)
        .bind(page_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(updated)
    }

    // ── Stats ────────────────────────────────────────────────────────────────

    pub async fn stats(&self) -> AppResult<serde_json::Value> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM intent_pages")
            .fetch_one(&self.pool)
            .await?;
        let published: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM intent_pages WHERE status = 'published'"
        )
        .fetch_one(&self.pool)
        .await?;
        let draft: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM intent_pages WHERE status = 'draft'"
        )
        .fetch_one(&self.pool)
        .await?;
        let queued: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM intent_pages WHERE status = 'queued'"
        )
        .fetch_one(&self.pool)
        .await?;
        let archived: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM intent_pages WHERE status = 'archived'"
        )
        .fetch_one(&self.pool)
        .await?;

        // How many published today?
        let published_today: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM intent_pages WHERE status = 'published' AND published_at >= CURRENT_DATE"
        )
        .fetch_one(&self.pool)
        .await?;

        // Queue breakdown by priority
        let queued_high: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM intent_pages WHERE status = 'queued' AND priority = 2"
        )
        .fetch_one(&self.pool)
        .await?;
        let queued_normal: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM intent_pages WHERE status = 'queued' AND priority = 1"
        )
        .fetch_one(&self.pool)
        .await?;
        let queued_low: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM intent_pages WHERE status = 'queued' AND priority = 0"
        )
        .fetch_one(&self.pool)
        .await?;

        // Get daily limit
        let limit = self.get_publish_limit().await?;

        // ── Site-wide SEO page breakdown ──────────────────────────────────────
        //
        // These match exactly what the blog sitemap.ts generates × 4 locales.
        // Reference: /Users/dmitrijfomin/Desktop/blog/app/sitemap.ts
        //
        // locales = [pl, en, ru, uk]  →  LOCALE_MULTIPLIER = 4

        const LOCALE_MULTIPLIER: i64 = 4;

        // Active ingredients in catalog (base for most per-ingredient pages)
        let count_active_ingredients: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM catalog_ingredients WHERE COALESCE(is_active, true) = true AND slug IS NOT NULL"
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        // /chef-tools/nutrition/:slug  →  ingredients × 4 locales
        let count_nutrition = count_active_ingredients * LOCALE_MULTIPLIER;

        // /chef-tools/ingredients/:slug  →  ingredient profile pages × 4 locales
        let count_ingredient_profiles = count_active_ingredients * LOCALE_MULTIPLIER;

        // /chef-tools/how-many/how-many-{unit}-in-a-{measure}-of-{slug}
        // HOW_MANY_COMBOS = 14 combos (matches sitemap.ts HOW_MANY_COMBOS array)
        const HOW_MANY_COMBOS: i64 = 14;
        let count_how_many = count_active_ingredients * HOW_MANY_COMBOS * LOCALE_MULTIPLIER;

        // /chef-tools/ingredients/:slug/:state  →  INDEXABLE_STATES = 3 (raw, boiled, fried)
        // (other states have noindex — not counted)
        const INDEXABLE_STATES: i64 = 3;
        let count_states = count_active_ingredients * INDEXABLE_STATES * LOCALE_MULTIPLIER;

        // /chef-tools/diet/:flag  →  7 diet flags × 4 locales
        const DIET_FLAGS: i64 = 7;
        let count_diet = DIET_FLAGS * LOCALE_MULTIPLIER;

        // /chef-tools/ranking/:metric  →  15 metrics × 4 locales
        const RANKING_METRICS: i64 = 15;
        let count_ranking = RANKING_METRICS * LOCALE_MULTIPLIER;

        // /chef-tools/fish-season/:month  →  12 months × 4 locales
        let count_fish_season = 12 * LOCALE_MULTIPLIER;

        // /seo/:slug  →  published intent pages (these are NOT locale-multiplied yet,
        // each intent page has its own locale field)
        let count_intent = published;

        // Static pages (~40) × 4 locales
        const STATIC_PAGES: i64 = 40;
        let count_static = STATIC_PAGES * LOCALE_MULTIPLIER;

        // Total tracked system pages
        let system_total = count_nutrition
            + count_ingredient_profiles
            + count_how_many
            + count_states
            + count_diet
            + count_ranking
            + count_fish_season
            + count_static
            + count_intent;

        // Google baseline from seo_settings
        let google_discovered: i64 = sqlx::query_scalar(
            "SELECT COALESCE(value::bigint, 0) FROM seo_settings WHERE key = 'google_discovered_pages'"
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(7192);

        // Coverage: how many of our system pages Google has discovered
        let coverage_pct = if system_total > 0 {
            (google_discovered as f64 / system_total as f64 * 100.0).min(100.0)
        } else {
            0.0
        };

        // New intent pages this week
        let intent_this_week: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM intent_pages WHERE status = 'published' AND published_at >= NOW() - INTERVAL '7 days'"
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        Ok(serde_json::json!({
            // Intent pages pipeline stats
            "total": total,
            "draft": draft,
            "queued": queued,
            "queued_high": queued_high,
            "queued_normal": queued_normal,
            "queued_low": queued_low,
            "published": published,
            "archived": archived,
            "published_today": published_today,
            "publish_limit_per_day": limit,
            "remaining_today": (limit - published_today).max(0),

            // Site-wide SEO breakdown (matches sitemap.ts exactly)
            "seo": {
                // Per-ingredient page families
                "active_ingredients": count_active_ingredients,
                "nutrition": count_nutrition,
                "ingredient_profiles": count_ingredient_profiles,
                "how_many": count_how_many,
                "states": count_states,
                // Taxonomy / static
                "diet": count_diet,
                "ranking": count_ranking,
                "fish_season": count_fish_season,
                "static_pages": count_static,
                // New growth layer
                "intent_pages": count_intent,
                "intent_today": published_today,
                "intent_this_week": intent_this_week,
                // KPI totals
                "system_total": system_total,
                "google_discovered": google_discovered,
                "coverage_pct": (coverage_pct * 10.0).round() / 10.0,
                "untracked": (google_discovered - system_total).max(0),
                "not_yet_indexed": (system_total - google_discovered).max(0),
            }
        }))
    }

    /// Update the google_discovered_pages baseline (from Google Search Console).
    pub async fn set_google_discovered(&self, count: i64) -> AppResult<serde_json::Value> {
        sqlx::query(
            "INSERT INTO seo_settings (key, value) VALUES ('google_discovered_pages', $1)
             ON CONFLICT (key) DO UPDATE SET value = $1, updated_at = NOW()"
        )
        .bind(count.to_string())
        .execute(&self.pool)
        .await?;

        Ok(serde_json::json!({ "google_discovered_pages": count }))
    }

    // ── Enqueue (draft → queued) ─────────────────────────────────────────────

    pub async fn enqueue(&self, id: Uuid) -> AppResult<IntentPage> {
        let page = self.get_by_id(id).await?;
        if page.status != "draft" {
            return Err(AppError::validation(format!(
                "Can only enqueue draft pages, current status: {}", page.status
            )));
        }

        let page = sqlx::query_as::<_, IntentPage>(
            r#"UPDATE intent_pages
               SET status = 'queued', queued_at = NOW()
               WHERE id = $1
               RETURNING id, intent_type, entity_a, entity_b, locale,
                         title, description, answer, faq, slug, status, priority, content_blocks,
                         published_at::text, queued_at::text, created_at::text, updated_at::text"#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        tracing::info!("📥 Intent page queued: '{}' ({})", page.title, page.locale);
        Ok(page)
    }

    // ── Enqueue bulk ─────────────────────────────────────────────────────────

    pub async fn enqueue_bulk(&self, ids: &[Uuid], priority: i32) -> AppResult<serde_json::Value> {
        let priority = priority.clamp(0, 2);
        let mut queued = 0i32;
        let mut skipped = 0i32;

        for id in ids {
            // Only enqueue drafts
            let result = sqlx::query(
                "UPDATE intent_pages SET status = 'queued', queued_at = NOW(), priority = $2 WHERE id = $1 AND status = 'draft'"
            )
            .bind(id)
            .bind(priority)
            .execute(&self.pool)
            .await?;

            if result.rows_affected() > 0 {
                queued += 1;
            } else {
                skipped += 1;
            }
        }

        tracing::info!("📥 Bulk enqueue: {} queued, {} skipped", queued, skipped);

        Ok(serde_json::json!({
            "queued": queued,
            "skipped": skipped,
        }))
    }

    // ── Archive ──────────────────────────────────────────────────────────────

    pub async fn archive(&self, id: Uuid) -> AppResult<IntentPage> {
        let page = sqlx::query_as::<_, IntentPage>(
            r#"UPDATE intent_pages
               SET status = 'archived'
               WHERE id = $1
               RETURNING id, intent_type, entity_a, entity_b, locale,
                         title, description, answer, faq, slug, status, priority, content_blocks,
                         published_at::text, queued_at::text, created_at::text, updated_at::text"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::not_found("Intent page not found"))?;

        // If was published, revalidate to remove from site
        if page.published_at.is_some() {
            tokio::spawn(revalidate_blog(Some(page.entity_a.clone())));
        }

        tracing::info!("📦 Intent page archived: '{}' ({})", page.title, page.locale);
        Ok(page)
    }

    // ── Settings ─────────────────────────────────────────────────────────────

    pub async fn get_publish_limit(&self) -> AppResult<i64> {
        let val: Option<String> = sqlx::query_scalar(
            "SELECT value FROM seo_settings WHERE key = 'publish_limit_per_day'"
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(val.and_then(|v| v.parse::<i64>().ok()).unwrap_or(24))
    }

    pub async fn get_settings(&self) -> AppResult<serde_json::Value> {
        let limit = self.get_publish_limit().await?;
        Ok(serde_json::json!({
            "publish_limit_per_day": limit,
        }))
    }

    pub async fn update_settings(&self, limit: i64) -> AppResult<serde_json::Value> {
        if limit < 1 || limit > 200 {
            return Err(AppError::validation("Limit must be between 1 and 200"));
        }

        sqlx::query(
            "INSERT INTO seo_settings (key, value) VALUES ('publish_limit_per_day', $1)
             ON CONFLICT (key) DO UPDATE SET value = $1, updated_at = NOW()"
        )
        .bind(limit.to_string())
        .execute(&self.pool)
        .await?;

        tracing::info!("⚙️ Publish limit updated to {} pages/day", limit);
        Ok(serde_json::json!({
            "publish_limit_per_day": limit,
        }))
    }

    // ── Scheduler: publish from queue ────────────────────────────────────────
    //
    // Called by cron or manually via admin endpoint.
    // Publishes up to (daily_limit - already_published_today) pages from queue.

    pub async fn run_scheduled_publish(&self) -> AppResult<serde_json::Value> {
        let limit = self.get_publish_limit().await?;

        // How many already published today?
        let published_today: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM intent_pages WHERE status = 'published' AND published_at >= CURRENT_DATE"
        )
        .fetch_one(&self.pool)
        .await?;

        let remaining = (limit - published_today).max(0);
        if remaining == 0 {
            tracing::info!("⏸️ Scheduler: daily limit reached ({}/{}), skipping", published_today, limit);
            return Ok(serde_json::json!({
                "published": 0,
                "reason": "daily_limit_reached",
                "published_today": published_today,
                "limit": limit,
            }));
        }

        // Get queued pages (priority first, then FIFO by queued_at)
        let queued_ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM intent_pages WHERE status = 'queued' ORDER BY priority DESC, queued_at ASC LIMIT $1"
        )
        .bind(remaining)
        .fetch_all(&self.pool)
        .await?;

        if queued_ids.is_empty() {
            tracing::info!("⏸️ Scheduler: no queued pages to publish");
            return Ok(serde_json::json!({
                "published": 0,
                "reason": "queue_empty",
                "published_today": published_today,
                "limit": limit,
            }));
        }

        let mut published_count = 0i32;
        let mut errors = 0i32;

        for id in &queued_ids {
            match self.publish(*id).await {
                Ok(page) => {
                    published_count += 1;
                    tracing::info!("🚀 Scheduler published: '{}' ({})", page.title, page.locale);
                }
                Err(e) => {
                    errors += 1;
                    tracing::error!("❌ Scheduler publish failed for {}: {}", id, e);
                }
            }
            // Small delay to avoid hammering revalidation
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }

        tracing::info!(
            "🕐 Scheduler done: {} published, {} errors (today total: {}/{})",
            published_count, errors, published_today + published_count as i64, limit
        );

        Ok(serde_json::json!({
            "published": published_count,
            "errors": errors,
            "published_today": published_today + published_count as i64,
            "limit": limit,
        }))
    }

    // ── Find duplicate slugs ─────────────────────────────────────────────────
    //
    // Finds pages that semantically cover the same sub-intent but have
    // different slug forms (e.g. "is almonds healthy" vs "are-almonds-healthy").

    pub async fn find_duplicates(&self) -> AppResult<Vec<DuplicateGroup>> {
        // Find pages where the normalised slug (lowercase, spaces→dashes,
        // is→are for plurals) matches another page in the same locale.
        let pages = sqlx::query_as::<_, IntentPage>(
            r#"SELECT id, intent_type, entity_a, entity_b, locale,
                      title, description, answer, faq, slug, status, priority, content_blocks,
                      published_at::text, queued_at::text, created_at::text, updated_at::text
               FROM intent_pages
               ORDER BY entity_a, locale, created_at"#,
        )
        .fetch_all(&self.pool)
        .await?;

        // Group by (normalised_slug, locale)
        let mut groups: std::collections::HashMap<(String, String), Vec<&IntentPage>> =
            std::collections::HashMap::new();

        for page in &pages {
            let normalised = normalise_slug(&page.slug);
            groups
                .entry((normalised, page.locale.clone()))
                .or_default()
                .push(page);
        }

        // Only keep groups with 2+ entries (actual duplicates)
        let mut duplicates: Vec<DuplicateGroup> = Vec::new();
        for ((canonical_slug, locale), group_pages) in &groups {
            if group_pages.len() < 2 {
                continue;
            }

            let entries: Vec<DuplicateEntry> = group_pages
                .iter()
                .map(|p| {
                    let is_canonical = p.slug == *canonical_slug;
                    DuplicateEntry {
                        id: p.id,
                        slug: p.slug.clone(),
                        title: p.title.clone(),
                        status: p.status.clone(),
                        is_canonical,
                    }
                })
                .collect();

            duplicates.push(DuplicateGroup {
                entity_a: group_pages[0].entity_a.clone(),
                locale: locale.clone(),
                canonical_slug: canonical_slug.clone(),
                pages: entries,
            });
        }

        duplicates.sort_by(|a, b| a.entity_a.cmp(&b.entity_a).then(a.locale.cmp(&b.locale)));

        tracing::info!("🔍 Found {} duplicate groups", duplicates.len());
        Ok(duplicates)
    }

    // ── Cleanup duplicate slugs ──────────────────────────────────────────────
    //
    // For each duplicate group, keep the canonical slug page (or the oldest)
    // and delete the rest. Returns count of deleted pages.

    pub async fn cleanup_duplicate_slugs(&self) -> AppResult<serde_json::Value> {
        let groups = self.find_duplicates().await?;
        let mut deleted = 0i32;
        let mut kept = 0i32;

        for group in &groups {
            // Prefer: canonical slug, then published, then oldest
            let keep_id = group
                .pages
                .iter()
                .find(|p| p.is_canonical)
                .or_else(|| group.pages.iter().find(|p| p.status == "published"))
                .unwrap_or(&group.pages[0])
                .id;

            for entry in &group.pages {
                if entry.id == keep_id {
                    // If the kept page has a non-canonical slug, fix it
                    if entry.slug != group.canonical_slug {
                        sqlx::query("UPDATE intent_pages SET slug = $1 WHERE id = $2")
                            .bind(&group.canonical_slug)
                            .bind(entry.id)
                            .execute(&self.pool)
                            .await?;
                        tracing::info!("✏️ Fixed slug: '{}' → '{}'", entry.slug, group.canonical_slug);
                    }
                    kept += 1;
                } else {
                    sqlx::query("DELETE FROM intent_pages WHERE id = $1")
                        .bind(entry.id)
                        .execute(&self.pool)
                        .await?;
                    tracing::info!("🗑️ Deleted duplicate: '{}' ({})", entry.slug, entry.id);
                    deleted += 1;
                }
            }
        }

        tracing::info!("🧹 Cleanup done: {} deleted, {} kept", deleted, kept);
        Ok(serde_json::json!({
            "groups": groups.len(),
            "deleted": deleted,
            "kept": kept,
        }))
    }

    // ── Bulk publish ─────────────────────────────────────────────────────────

    pub async fn bulk_publish(&self, ids: &[Uuid]) -> AppResult<serde_json::Value> {
        let mut published = 0i32;
        let mut skipped = 0i32;

        for id in ids {
            match self.publish(*id).await {
                Ok(_) => published += 1,
                Err(_) => skipped += 1,
            }
        }

        tracing::info!("📢 Bulk publish: {} published, {} skipped", published, skipped);
        Ok(serde_json::json!({
            "published": published,
            "skipped": skipped,
        }))
    }

    // ── Bulk archive ─────────────────────────────────────────────────────────

    pub async fn bulk_archive(&self, ids: &[Uuid]) -> AppResult<serde_json::Value> {
        let mut archived = 0i32;
        let mut skipped = 0i32;

        for id in ids {
            match self.archive(*id).await {
                Ok(_) => archived += 1,
                Err(_) => skipped += 1,
            }
        }

        tracing::info!("📦 Bulk archive: {} archived, {} skipped", archived, skipped);
        Ok(serde_json::json!({
            "archived": archived,
            "skipped": skipped,
        }))
    }

    // ── Bulk delete ──────────────────────────────────────────────────────────

    pub async fn bulk_delete(&self, ids: &[Uuid]) -> AppResult<serde_json::Value> {
        let mut deleted = 0i32;
        let mut skipped = 0i32;

        for id in ids {
            match self.delete(*id).await {
                Ok(_) => deleted += 1,
                Err(_) => skipped += 1,
            }
        }

        tracing::info!("🗑️ Bulk delete: {} deleted, {} skipped", deleted, skipped);
        Ok(serde_json::json!({
            "deleted": deleted,
            "skipped": skipped,
        }))
    }
}

// ── Slug generator ───────────────────────────────────────────────────────────

/// Normalise any slug variant to its canonical form.
/// "is almonds healthy" → "are-almonds-healthy"
/// "IS-ALMONDS-HEALTHY" → "are-almonds-healthy"
fn normalise_slug(slug: &str) -> String {
    let s = slug
        .trim()
        .to_lowercase()
        .replace(' ', "-");

    // Clean up multiple dashes
    let s = s.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-");

    // Fix grammar: is- → are- for plural entities
    // Extract entity from common patterns
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() >= 3 && parts[0] == "is" {
        let entity = parts[1];
        if looks_plural(entity) {
            return format!("are-{}", parts[1..].join("-"));
        }
    }
    if parts.len() >= 4 && parts[0] == "which" && parts[1] == "is" {
        let entity = parts[3]; // "which-is-healthier-almonds-or-cashews"
        if looks_plural(entity) {
            return format!("which-are-{}", parts[2..].join("-"));
        }
    }

    s
}

/// Transliterate any Unicode string to an ASCII URL slug.
/// "полезен ли артишок" → "polezen-li-artishok"
/// "czy karczoch jest zdrowy" → "czy-karczoch-jest-zdrowy"
/// "is artichoke healthy" → "is-artichoke-healthy"
fn transliterate_to_slug(text: &str) -> String {
    let ascii = deunicode(text);
    let slug: String = ascii
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() { c }
            else if c == ' ' || c == '-' || c == '_' { '-' }
            else { ' ' }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("");

    // Clean up multiple dashes, trim
    slug.split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn generate_slug(title: &str, intent: &str, entity_a: &str, entity_b: Option<&str>) -> String {
    let slug = transliterate_to_slug(title);

    // If slug is too short after transliteration, generate from intent+entity
    if slug.len() < 5 {
        match entity_b {
            Some(b) => format!("{}-{}-vs-{}", intent, entity_a, b),
            None => format!("{}-{}", intent, entity_a),
        }
    } else {
        slug
    }
}
