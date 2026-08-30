//! Thin Telegram Bot API client built on `reqwest`.
//!
//! SECURITY: the bot token lives only inside this struct and is never
//! rendered — `Debug` is hand-implemented to redact it, and no log line in
//! this file (or any caller) is allowed to print the URLs built by `url()`,
//! since they embed the token (`https://api.telegram.org/bot<TOKEN>/...`).

use std::fmt;

use serde::{Deserialize, Serialize};

use super::keyboards::InlineKeyboardMarkup;

const TELEGRAM_API_BASE: &str = "https://api.telegram.org";
/// Telegram's hard message-length limit is 4096 UTF-16 code units; we cut a
/// little earlier to stay safely under it regardless of encoding overhead.
const MAX_MESSAGE_CHARS: usize = 3900;

#[derive(Clone)]
pub struct TelegramClient {
    http: reqwest::Client,
    bot_token: String,
}

impl fmt::Debug for TelegramClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TelegramClient")
            .field("bot_token", &"***redacted***")
            .finish()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TelegramApiError {
    #[error("Telegram API request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("Telegram API returned an error: {description}")]
    Api { description: String },
}

#[derive(Debug, Deserialize)]
struct TelegramApiResponse {
    #[allow(dead_code)]
    ok: bool,
    description: Option<String>,
}

fn truncate_for_telegram(text: &str) -> String {
    if text.chars().count() <= MAX_MESSAGE_CHARS {
        return text.to_string();
    }
    let mut truncated: String = text.chars().take(MAX_MESSAGE_CHARS).collect();
    truncated.push('…');
    truncated
}

impl TelegramClient {
    pub fn new(bot_token: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            bot_token,
        }
    }

    /// Builds a `bot<TOKEN>/<method>` URL. NEVER pass the result of this to
    /// `tracing`/`println!`/error messages — it contains the live token.
    fn url(&self, method: &str) -> String {
        format!("{TELEGRAM_API_BASE}/bot{}/{method}", self.bot_token)
    }

    async fn call<T: Serialize + ?Sized>(
        &self,
        method: &'static str,
        payload: &T,
    ) -> Result<(), TelegramApiError> {
        let response = self
            .http
            .post(self.url(method))
            .json(payload)
            .send()
            .await?;
        let status = response.status();
        if status.is_success() {
            return Ok(());
        }

        let description = response
            .json::<TelegramApiResponse>()
            .await
            .ok()
            .and_then(|body| body.description)
            .unwrap_or_else(|| format!("HTTP {status}"));

        tracing::warn!(
            method,
            status = %status,
            description = %description,
            "Telegram API call failed"
        );

        Err(TelegramApiError::Api { description })
    }

    pub async fn send_message(
        &self,
        chat_id: i64,
        text: &str,
        reply_markup: Option<InlineKeyboardMarkup>,
    ) -> Result<(), TelegramApiError> {
        #[derive(Serialize)]
        struct Payload<'a> {
            chat_id: i64,
            text: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            reply_markup: Option<InlineKeyboardMarkup>,
        }

        let text = truncate_for_telegram(text);
        self.call(
            "sendMessage",
            &Payload {
                chat_id,
                text: &text,
                reply_markup,
            },
        )
        .await
    }

    pub async fn send_photo(
        &self,
        chat_id: i64,
        photo_url: &str,
        caption: Option<&str>,
    ) -> Result<(), TelegramApiError> {
        #[derive(Serialize)]
        struct Payload<'a> {
            chat_id: i64,
            photo: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            caption: Option<&'a str>,
        }

        self.call(
            "sendPhoto",
            &Payload {
                chat_id,
                photo: photo_url,
                caption,
            },
        )
        .await
    }

    pub async fn answer_callback_query(
        &self,
        callback_query_id: &str,
        text: Option<&str>,
    ) -> Result<(), TelegramApiError> {
        #[derive(Serialize)]
        struct Payload<'a> {
            callback_query_id: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            text: Option<&'a str>,
        }

        self.call(
            "answerCallbackQuery",
            &Payload {
                callback_query_id,
                text,
            },
        )
        .await
    }

    pub async fn set_webhook(&self, url: &str, secret_token: &str) -> Result<(), TelegramApiError> {
        #[derive(Serialize)]
        struct Payload<'a> {
            url: &'a str,
            secret_token: &'a str,
        }

        self.call("setWebhook", &Payload { url, secret_token })
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_never_prints_the_token() {
        let client = TelegramClient::new("123456:super-secret-token".to_string());
        let debug_output = format!("{client:?}");
        assert!(!debug_output.contains("123456:super-secret-token"));
    }

    #[test]
    fn url_embeds_token_but_is_never_logged() {
        // Documents the contract: `url()` DOES contain the token by design
        // (Telegram's API shape requires it in the path) — callers must
        // never pass this string to tracing/log macros.
        let client = TelegramClient::new("123456:token".to_string());
        assert_eq!(
            client.url("sendMessage"),
            "https://api.telegram.org/bot123456:token/sendMessage"
        );
    }

    #[test]
    fn truncates_overly_long_messages() {
        let long = "a".repeat(5000);
        let truncated = truncate_for_telegram(&long);
        assert!(truncated.chars().count() <= MAX_MESSAGE_CHARS + 1);
        assert!(truncated.ends_with('…'));
    }

    #[test]
    fn short_messages_pass_through_unchanged() {
        assert_eq!(truncate_for_telegram("hello"), "hello");
    }
}
