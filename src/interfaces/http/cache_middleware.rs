//! Cache-Control middleware for public endpoints.
//!
//! Adds HTTP caching headers so CDN (Vercel / Cloudflare) and browsers
//! cache responses aggressively — reducing load on backend + Neon DB.
//!
//! Usage:
//!   .layer(axum::middleware::from_fn(cache_1h))   // 1 hour
//!   .layer(axum::middleware::from_fn(cache_1d))   // 1 day
//!   .layer(axum::middleware::from_fn(cache_5m))   // 5 minutes

use axum::{
    extract::Request,
    http::header,
    middleware::Next,
    response::Response,
};

/// 5 minutes — autocomplete, search results
pub async fn cache_5m(req: Request, next: Next) -> Response {
    let mut resp = next.run(req).await;
    resp.headers_mut().insert(
        header::CACHE_CONTROL,
        "public, max-age=300, s-maxage=300, stale-while-revalidate=60"
            .parse()
            .unwrap(),
    );
    resp
}

/// 1 hour — ingredient detail, states, nutrition pages
pub async fn cache_1h(req: Request, next: Next) -> Response {
    let mut resp = next.run(req).await;
    resp.headers_mut().insert(
        header::CACHE_CONTROL,
        "public, max-age=3600, s-maxage=3600, stale-while-revalidate=300"
            .parse()
            .unwrap(),
    );
    resp
}

/// 1 day — ingredient list, sitemap data, categories, static tools
pub async fn cache_1d(req: Request, next: Next) -> Response {
    let mut resp = next.run(req).await;
    resp.headers_mut().insert(
        header::CACHE_CONTROL,
        "public, max-age=86400, s-maxage=86400, stale-while-revalidate=3600"
            .parse()
            .unwrap(),
    );
    resp
}

/// Immutable — unit conversions, fish season tables (pure computation)
pub async fn cache_immutable(req: Request, next: Next) -> Response {
    let mut resp = next.run(req).await;
    resp.headers_mut().insert(
        header::CACHE_CONTROL,
        "public, max-age=604800, s-maxage=604800, immutable"
            .parse()
            .unwrap(),
    );
    resp
}
