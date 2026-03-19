use sqlx::PgPool;

use crate::shared::{AppError, Language};

/// A resolved catalog ingredient row with match quality info.
#[derive(Debug, Clone)]
pub struct MatchedRow {
    pub slug: String,
    pub name: String,
    /// 1=exact_slug, 2-6=exact_name, 7-9=ilike, 10=fuzzy
    pub priority: i32,
    /// Trigram similarity score 0.0–1.0
    pub similarity: f32,
}

impl MatchedRow {
    /// Compute a confidence score 0.0–1.0 from priority + similarity.
    pub fn confidence(&self) -> f32 {
        match self.priority {
            1     => 1.0,                          // exact slug
            2..=6 => 0.95,                         // exact name match
            7..=9 => (0.70 + self.similarity * 0.20).min(0.90), // ILIKE
            _     => (self.similarity * 0.9).min(0.65).max(0.10), // fuzzy
        }
    }

    /// Human-readable match type string.
    pub fn match_type(&self) -> &'static str {
        match self.priority {
            1     => "exact",
            2..=6 => "name",
            7..=9 => "ilike",
            _     => "fuzzy",
        }
    }
}

/// Batch-match ALL tokens in a single SQL round-trip.
///
/// Uses `UNNEST` + lateral join with a priority-based match:
///   1. exact slug
///   2–6. exact localized name (case-insensitive, any of 4 languages)
///   7–9. ILIKE partial match
///   10. trigram similarity ≥ 0.25 (uses gin_trgm_ops indexes)
///
/// Returns a map: token → MatchedRow (only for matched tokens).
pub async fn batch_match(
    pool: &PgPool,
    tokens: &[String],
    lang: Language,
) -> Result<std::collections::HashMap<String, MatchedRow>, AppError> {
    if tokens.is_empty() {
        return Ok(std::collections::HashMap::new());
    }

    let name_col = match lang {
        Language::En => "name_en",
        Language::Ru => "name_ru",
        Language::Pl => "name_pl",
        Language::Uk => "name_uk",
    };

    let sql = format!(
        r#"
        SELECT t.token, m.slug, m.name, m.priority, m.sim
        FROM UNNEST($1::text[]) AS t(token)
        CROSS JOIN LATERAL (
            SELECT ci.slug, ci.{name_col} AS name,
                   CASE
                     WHEN ci.slug = t.token THEN 1
                     WHEN LOWER(ci.{name_col}) = t.token THEN 2
                     WHEN LOWER(ci.name_en) = t.token THEN 3
                     WHEN LOWER(ci.name_ru) = t.token THEN 4
                     WHEN LOWER(ci.name_pl) = t.token THEN 5
                     WHEN LOWER(ci.name_uk) = t.token THEN 6
                     WHEN ci.slug ILIKE '%' || t.token || '%' THEN 7
                     WHEN ci.{name_col} ILIKE '%' || t.token || '%' THEN 8
                     WHEN ci.name_en ILIKE '%' || t.token || '%' THEN 9
                     ELSE 10
                   END AS priority,
                   GREATEST(
                     similarity(ci.slug, t.token),
                     similarity(ci.{name_col}, t.token),
                     similarity(ci.name_en, t.token)
                   ) AS sim
            FROM catalog_ingredients ci
            WHERE COALESCE(ci.is_active, true) = true
              AND (
                ci.slug = t.token
                OR LOWER(ci.{name_col}) = t.token
                OR LOWER(ci.name_en) = t.token
                OR LOWER(ci.name_ru) = t.token
                OR LOWER(ci.name_pl) = t.token
                OR LOWER(ci.name_uk) = t.token
                OR ci.slug ILIKE '%' || t.token || '%'
                OR ci.{name_col} ILIKE '%' || t.token || '%'
                OR ci.name_en ILIKE '%' || t.token || '%'
                OR similarity(ci.slug, t.token) >= 0.25
                OR similarity(ci.{name_col}, t.token) >= 0.25
                OR similarity(ci.name_en, t.token) >= 0.25
              )
            ORDER BY priority ASC, sim DESC, length(ci.slug) ASC
            LIMIT 1
        ) m
        "#,
        name_col = name_col,
    );

    #[derive(sqlx::FromRow)]
    struct BatchRow {
        token: String,
        slug: String,
        name: String,
        priority: i32,
        sim: f32,
    }

    let rows: Vec<BatchRow> = sqlx::query_as(&sql)
        .bind(tokens)
        .fetch_all(pool)
        .await
        .map_err(|e| {
            tracing::error!("smart_parse batch_match error: {}", e);
            AppError::internal(&format!("smart_parse batch match failed: {}", e))
        })?;

    let mut result = std::collections::HashMap::with_capacity(rows.len());
    for r in rows {
        result.insert(
            r.token,
            MatchedRow {
                slug: r.slug,
                name: r.name,
                priority: r.priority,
                similarity: r.sim,
            },
        );
    }

    Ok(result)
}
