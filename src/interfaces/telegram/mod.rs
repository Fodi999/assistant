//! Telegram Bot integration for "Світло Ікони" — Stage 1 (MVP).
//!
//! Wiring notes:
//! - Mounted at `/telegram` (top-level, not under `/api`) directly inside
//!   `interfaces::http::routes::create_router`, exactly like every other
//!   feature area in this monolith (CMS, billing, copilot, ...). There is no
//!   separate backend/process/deployment.
//! - Shares the same `PgPool` (Neon/Postgres) the rest of the app uses —
//!   `TelegramState` is this module's slice of shared app state, built once
//!   in `router()` and passed to Axum via `.with_state(...)`.
//! - When `TELEGRAM_BOT_TOKEN` is absent, `router()` still mounts
//!   `/telegram/status` (reports `configured: false`) and
//!   `/telegram/webhook` (always answers 503) — the rest of the backend is
//!   completely unaffected either way.
//! - The bot token is never logged. `TelegramClient`'s `Debug` impl redacts
//!   it, and no code path in this module passes a Telegram API URL (which
//!   embeds the token) to `tracing`/`log`.

pub mod client;
pub mod commands;
pub mod keyboards;
pub mod webhook;

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use sqlx::PgPool;

pub use client::TelegramClient;

use crate::infrastructure::config::TelegramConfig;

#[derive(Clone)]
pub struct TelegramState {
    pub pool: PgPool,
    pub client: Option<Arc<TelegramClient>>,
    pub webhook_secret: Option<String>,
    pub channel: Option<String>,
    pub configured: bool,
}

/// Builds the `/telegram/*` router. `config` is `None` whenever
/// `TELEGRAM_BOT_TOKEN` isn't set — the router still mounts both routes so
/// `GET /telegram/status` can truthfully report `configured: false` instead
/// of 404ing, while `POST /telegram/webhook` answers 503 without touching
/// Telegram or the database.
pub fn router(pool: PgPool, config: Option<TelegramConfig>) -> Router {
    let state = match config {
        Some(cfg) => TelegramState {
            pool,
            client: Some(Arc::new(TelegramClient::new(cfg.bot_token))),
            webhook_secret: cfg.webhook_secret,
            channel: Some(cfg.channel),
            configured: true,
        },
        None => TelegramState {
            pool,
            client: None,
            webhook_secret: None,
            channel: None,
            configured: false,
        },
    };

    Router::new()
        .route("/webhook", post(webhook::telegram_webhook_handler))
        .route("/status", get(webhook::get_status))
        .with_state(state)
}
