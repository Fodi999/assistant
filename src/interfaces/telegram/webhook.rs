//! `POST /telegram/webhook` and `GET /telegram/status`.
//!
//! The webhook is the only untrusted entry point in this module: every
//! request is checked against `TELEGRAM_WEBHOOK_SECRET` (via the
//! `X-Telegram-Bot-Api-Secret-Token` header) before anything else happens.
//! Business data (calendar/saints/prayers/gospel) is read straight from the
//! existing `church_content` query functions on the shared `PgPool` — this
//! module never makes an HTTP call back into its own server.

use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use subtle::ConstantTimeEq;

use super::client::TelegramClient;
use super::commands::{self, Command};
use super::{keyboards, TelegramState};
use crate::interfaces::http::church_content;

// ── Telegram Update payload (only the fields this bot uses) ───────────────

#[derive(Debug, Deserialize)]
pub struct TelegramUpdate {
    #[allow(dead_code)]
    #[serde(default)]
    pub update_id: i64,
    #[serde(default)]
    pub message: Option<TelegramMessage>,
    #[serde(default)]
    pub callback_query: Option<TelegramCallbackQuery>,
}

#[derive(Debug, Deserialize)]
pub struct TelegramMessage {
    pub chat: TelegramChat,
    #[serde(default)]
    pub text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TelegramChat {
    pub id: i64,
}

#[derive(Debug, Deserialize)]
pub struct TelegramCallbackQuery {
    pub id: String,
    #[serde(default)]
    pub message: Option<TelegramMessage>,
    #[serde(default)]
    pub data: Option<String>,
}

// ── Webhook secret validation ──────────────────────────────────────────────

/// Constant-time comparison against `TELEGRAM_WEBHOOK_SECRET`, mirroring the
/// pattern already used for Stripe webhook signatures
/// (`infrastructure::stripe_service::verify_webhook_signature`). Fails closed:
/// if the secret isn't configured at all, every request is rejected — never
/// treated as "skip the check".
pub(crate) fn validate_secret(headers: &HeaderMap, expected: &Option<String>) -> bool {
    let Some(expected) = expected.as_ref().filter(|s| !s.is_empty()) else {
        return false;
    };
    let Some(received) = headers
        .get("X-Telegram-Bot-Api-Secret-Token")
        .and_then(|v| v.to_str().ok())
    else {
        return false;
    };

    received.len() == expected.len() && received.as_bytes().ct_eq(expected.as_bytes()).into()
}

// ── GET /telegram/status ────────────────────────────────────────────────

pub async fn get_status(State(state): State<TelegramState>) -> impl IntoResponse {
    Json(json!({
        "configured": state.configured,
        "channel": state.channel,
    }))
}

// ── POST /telegram/webhook ──────────────────────────────────────────────

pub async fn telegram_webhook_handler(
    State(state): State<TelegramState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let Some(client) = state.client.clone() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "Telegram integration is not configured" })),
        )
            .into_response();
    };

    if !validate_secret(&headers, &state.webhook_secret) {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    let update: TelegramUpdate = match serde_json::from_slice(&body) {
        Ok(update) => update,
        Err(err) => {
            tracing::warn!("Telegram webhook: failed to parse update body: {}", err);
            return StatusCode::BAD_REQUEST.into_response();
        }
    };

    handle_update(&state.pool, &client, update).await;

    // Always 200 once the update is accepted, so Telegram doesn't retry —
    // any downstream failure (e.g. a transient DB hiccup) is only logged.
    StatusCode::OK.into_response()
}

async fn handle_update(pool: &PgPool, client: &TelegramClient, update: TelegramUpdate) {
    if let Some(callback) = update.callback_query {
        handle_callback_query(pool, client, callback).await;
        return;
    }

    if let Some(message) = update.message {
        let Some(text) = message.text else {
            return;
        };
        if let Some(command) = Command::parse_slash(&text) {
            dispatch_command(pool, client, message.chat.id, command).await;
        }
        // Free-text messages are intentionally ignored in this MVP.
    }
}

async fn handle_callback_query(
    pool: &PgPool,
    client: &TelegramClient,
    callback: TelegramCallbackQuery,
) {
    // Acknowledge immediately so Telegram stops showing the loading spinner
    // on the button, regardless of whether we recognize `data`.
    if let Err(err) = client.answer_callback_query(&callback.id, None).await {
        tracing::warn!("Telegram: answerCallbackQuery failed: {}", err);
    }

    let Some(chat_id) = callback.message.as_ref().map(|m| m.chat.id) else {
        return;
    };
    let Some(data) = callback.data.as_deref() else {
        return;
    };

    match Command::from_callback_data(data) {
        Some(command) => dispatch_command(pool, client, chat_id, command).await,
        None if data == "settings" => {
            if let Err(err) = client
                .send_message(chat_id, commands::SETTINGS_STUB_TEXT, None)
                .await
            {
                tracing::warn!("Telegram: sendMessage failed: {}", err);
            }
        }
        None => {}
    }
}

async fn dispatch_command(pool: &PgPool, client: &TelegramClient, chat_id: i64, command: Command) {
    let result = match command {
        Command::Start => {
            client
                .send_message(
                    chat_id,
                    commands::START_TEXT,
                    Some(keyboards::main_menu_keyboard()),
                )
                .await
        }
        Command::Help => {
            client
                .send_message(chat_id, commands::HELP_TEXT, None)
                .await
        }
        Command::Today => {
            let text = fetch_today_text(pool).await;
            client
                .send_message(chat_id, &text, Some(keyboards::main_menu_keyboard()))
                .await
        }
        Command::Prayer => {
            let text = fetch_prayer_text(pool).await;
            client.send_message(chat_id, &text, None).await
        }
        Command::Saint => {
            let text = fetch_saint_text(pool).await;
            client.send_message(chat_id, &text, None).await
        }
        Command::Gospel => {
            let text = fetch_gospel_text(pool).await;
            client.send_message(chat_id, &text, None).await
        }
    };

    if let Err(err) = result {
        tracing::warn!("Telegram: sendMessage failed for chat {}: {}", chat_id, err);
    }
}

// ── Church-domain lookups — reuse existing query functions, no HTTP calls ──

const LANGUAGE: &str = "uk";

async fn fetch_today_text(pool: &PgPool) -> String {
    let today = chrono::Utc::now().date_naive().to_string();
    match church_content::public_calendar_by_date(pool, &today, Some(LANGUAGE), false).await {
        Ok(Json(page)) => {
            let saints = church_content::list_public_saints(
                pool,
                Some(page.calendar_day.id),
                None,
                Some(LANGUAGE),
                false,
            )
            .await
            .unwrap_or_default();
            commands::format_today(&page, &saints)
        }
        Err(StatusCode::NOT_FOUND) => commands::NO_CONTENT_TODAY.to_string(),
        Err(err) => {
            tracing::warn!("Telegram /today: calendar lookup failed: {:?}", err);
            commands::GENERIC_ERROR_TEXT.to_string()
        }
    }
}

/// Today's calendar day id, when a published one exists — `None` on any
/// error or when there simply isn't one yet, so callers can fall back to an
/// unscoped lookup uniformly.
async fn today_calendar_day_id(pool: &PgPool) -> Option<uuid::Uuid> {
    let today = chrono::Utc::now().date_naive().to_string();
    church_content::calendar_day_id_for_date(pool, &today)
        .await
        .ok()
        .flatten()
}

async fn fetch_prayer_text(pool: &PgPool) -> String {
    let day_id = today_calendar_day_id(pool).await;

    let mut prayers = match day_id {
        Some(id) => {
            church_content::list_public_prayers(pool, Some(id), None, Some(LANGUAGE), false)
                .await
                .unwrap_or_default()
        }
        None => Vec::new(),
    };
    if prayers.is_empty() {
        prayers = church_content::list_public_prayers(pool, None, None, Some(LANGUAGE), false)
            .await
            .unwrap_or_default();
    }

    match prayers.into_iter().next() {
        Some(prayer) => commands::format_prayer(&prayer),
        None => commands::NO_PRAYER_TEXT.to_string(),
    }
}

async fn fetch_saint_text(pool: &PgPool) -> String {
    let day_id = today_calendar_day_id(pool).await;

    let mut saints = match day_id {
        Some(id) => church_content::list_public_saints(pool, Some(id), None, Some(LANGUAGE), false)
            .await
            .unwrap_or_default(),
        None => Vec::new(),
    };
    if saints.is_empty() {
        saints = church_content::list_public_saints(pool, None, None, Some(LANGUAGE), false)
            .await
            .unwrap_or_default();
    }

    match saints.into_iter().next() {
        Some(saint) => commands::format_saint(&saint),
        None => commands::NO_SAINT_TEXT.to_string(),
    }
}

async fn fetch_gospel_text(pool: &PgPool) -> String {
    let day_id = today_calendar_day_id(pool).await;

    let mut readings = match day_id {
        Some(id) => church_content::list_public_gospel(pool, Some(id), None, Some(LANGUAGE), false)
            .await
            .unwrap_or_default(),
        None => Vec::new(),
    };
    if readings.is_empty() {
        readings = church_content::list_public_gospel(pool, None, None, Some(LANGUAGE), false)
            .await
            .unwrap_or_default();
    }

    match readings.into_iter().next() {
        Some(gospel) => commands::format_gospel(&gospel),
        None => commands::NO_GOSPEL_TEXT.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn headers_with_secret(value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-Telegram-Bot-Api-Secret-Token",
            HeaderValue::from_str(value).unwrap(),
        );
        headers
    }

    #[test]
    fn accepts_matching_secret() {
        let expected = Some("expected-secret".to_string());
        assert!(validate_secret(
            &headers_with_secret("expected-secret"),
            &expected
        ));
    }

    #[test]
    fn rejects_wrong_secret() {
        let expected = Some("expected-secret".to_string());
        assert!(!validate_secret(
            &headers_with_secret("wrong-secret"),
            &expected
        ));
    }

    #[test]
    fn rejects_missing_header() {
        let expected = Some("expected-secret".to_string());
        assert!(!validate_secret(&HeaderMap::new(), &expected));
    }

    #[test]
    fn fails_closed_when_no_secret_is_configured() {
        // Even a request that happens to send an (empty) header must be
        // rejected when TELEGRAM_WEBHOOK_SECRET was never set.
        assert!(!validate_secret(&headers_with_secret("anything"), &None));
        assert!(!validate_secret(&HeaderMap::new(), &None));
    }

    #[test]
    fn rejects_blank_configured_secret() {
        // Defense in depth: an accidentally-blank env var must not disable
        // the check.
        assert!(!validate_secret(
            &headers_with_secret(""),
            &Some(String::new())
        ));
    }

    #[test]
    fn parses_message_update() {
        let json = r#"{
            "update_id": 1,
            "message": { "message_id": 10, "chat": { "id": 555 }, "text": "/start" }
        }"#;
        let update: TelegramUpdate = serde_json::from_str(json).unwrap();
        let message = update.message.expect("message present");
        assert_eq!(message.chat.id, 555);
        assert_eq!(message.text.as_deref(), Some("/start"));
        assert!(update.callback_query.is_none());
    }

    #[test]
    fn parses_callback_query_update() {
        let json = r#"{
            "update_id": 2,
            "callback_query": {
                "id": "cbq-123",
                "data": "today",
                "message": { "message_id": 11, "chat": { "id": 777 } }
            }
        }"#;
        let update: TelegramUpdate = serde_json::from_str(json).unwrap();
        let callback = update.callback_query.expect("callback_query present");
        assert_eq!(callback.id, "cbq-123");
        assert_eq!(callback.data.as_deref(), Some("today"));
        assert_eq!(callback.message.unwrap().chat.id, 777);
        assert_eq!(
            Command::from_callback_data(callback.data.as_deref().unwrap()),
            Some(Command::Today)
        );
    }

    #[test]
    fn ignores_unknown_fields_in_update() {
        // Telegram updates carry many fields we don't model; `serde(default)`
        // on our optional fields must not choke on extras like `edited_message`.
        let json = r#"{ "update_id": 3, "edited_message": { "some": "thing" } }"#;
        let update: TelegramUpdate = serde_json::from_str(json).unwrap();
        assert!(update.message.is_none());
        assert!(update.callback_query.is_none());
    }
}
