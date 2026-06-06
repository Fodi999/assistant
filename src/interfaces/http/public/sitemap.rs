use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use sqlx::PgPool;
use std::fmt::Write as _;

#[derive(sqlx::FromRow)]
struct IngredientSitemapRow {
    slug: String,
    updated_at: sqlx::types::time::OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct IngredientStateSitemapRow {
    slug: String,
    state: String,
    updated_at: sqlx::types::time::OffsetDateTime,
}

#[derive(Clone)]
struct SitemapEntry {
    path: String,
    lastmod: Option<String>,
}

fn escape_xml(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn base_url(headers: &HeaderMap) -> String {
    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .unwrap_or("https");
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .unwrap_or("dima-fomin.pl");

    format!("{scheme}://{host}")
}

fn to_url(base: &str, path: &str) -> String {
    format!("{}{}", base.trim_end_matches('/'), path)
}

fn xml_response(base: &str, entries: &[SitemapEntry]) -> Response {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">"#,
    );

    for entry in entries {
        let loc = escape_xml(&to_url(base, &entry.path));
        let _ = writeln!(xml, "  <url>");
        let _ = writeln!(xml, "    <loc>{loc}</loc>");
        if let Some(lastmod) = &entry.lastmod {
            let _ = writeln!(xml, "    <lastmod>{}</lastmod>", escape_xml(lastmod));
        }
        let _ = writeln!(xml, "  </url>");
    }

    xml.push_str("</urlset>");

    (
        [
            (header::CONTENT_TYPE, "application/xml; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        xml,
    )
        .into_response()
}

fn robots_response(base: &str) -> Response {
    let body = format!(
        "User-agent: *\nAllow: /\nDisallow: /api/\nDisallow: /public/\nDisallow: /static/\nDisallow: /wasm/\nSitemap: {}/sitemap.xml\n",
        base.trim_end_matches('/')
    );

    (
        [
            (header::CONTENT_TYPE, "text/plain; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        body,
    )
        .into_response()
}

async fn load_entries(pool: &PgPool) -> Result<Vec<SitemapEntry>, StatusCode> {
    let mut entries = vec![
        SitemapEntry {
            path: "/".to_string(),
            lastmod: None,
        },
        SitemapEntry {
            path: "/about".to_string(),
            lastmod: None,
        },
        SitemapEntry {
            path: "/menu".to_string(),
            lastmod: None,
        },
        SitemapEntry {
            path: "/delivery".to_string(),
            lastmod: None,
        },
        SitemapEntry {
            path: "/booking".to_string(),
            lastmod: None,
        },
        SitemapEntry {
            path: "/recipes".to_string(),
            lastmod: None,
        },
        SitemapEntry {
            path: "/ingredient-catalog".to_string(),
            lastmod: None,
        },
        SitemapEntry {
            path: "/recipes/borsch".to_string(),
            lastmod: None,
        },
        SitemapEntry {
            path: "/recipes/tiramisu".to_string(),
            lastmod: None,
        },
        SitemapEntry {
            path: "/recipes/risotto".to_string(),
            lastmod: None,
        },
        SitemapEntry {
            path: "/cookie".to_string(),
            lastmod: None,
        },
        SitemapEntry {
            path: "/privacy".to_string(),
            lastmod: None,
        },
        SitemapEntry {
            path: "/terms".to_string(),
            lastmod: None,
        },
    ];

    let ingredient_rows: Vec<IngredientSitemapRow> = sqlx::query_as(
        r#"
        SELECT slug, updated_at
        FROM catalog_ingredients
        WHERE is_active = true
          AND COALESCE(is_published, false) = true
          AND slug IS NOT NULL
          AND slug != ''
        ORDER BY updated_at DESC
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    entries.extend(ingredient_rows.into_iter().map(|row| SitemapEntry {
        path: format!("/ingredient-catalog/{}", row.slug),
        lastmod: Some(row.updated_at.to_string()),
    }));

    let state_rows: Vec<IngredientStateSitemapRow> = sqlx::query_as(
        r#"
        SELECT ci.slug, states.state::text AS state, ci.updated_at
        FROM ingredient_states states
        JOIN catalog_ingredients ci ON ci.id = states.ingredient_id
        WHERE ci.is_active = true
          AND COALESCE(ci.is_published, false) = true
          AND ci.slug IS NOT NULL
          AND ci.slug != ''
        ORDER BY ci.slug, states.state
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    entries.extend(state_rows.into_iter().map(|row| SitemapEntry {
        path: format!("/ingredient-catalog/{}/{}", row.slug, row.state),
        lastmod: Some(row.updated_at.to_string()),
    }));

    Ok(entries)
}

pub async fn sitemap_xml(
    headers: HeaderMap,
    State(pool): State<PgPool>,
) -> Result<Response, StatusCode> {
    let base = base_url(&headers);
    let entries = load_entries(&pool).await?;
    Ok(xml_response(&base, &entries))
}

pub async fn robots_txt(headers: HeaderMap) -> Response {
    let base = base_url(&headers);
    robots_response(&base)
}
