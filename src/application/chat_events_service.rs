//! Chat Events Service — telemetry collection for ChefOS Chat (Step 4).
//!
//! Appends rows to `chat_events` with a bounded, strongly-typed API.
//! Designed for fire-and-forget: insertion failures are logged but never
//! bubble up to the user-facing chat flow.

use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::shared::{AppError, AppResult, UserId};

/// Typed event payload. Mirrors the iOS client side 1:1.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatEvent {
    /// `query_sent` | `card_shown` | `card_dismissed` | `action_clicked` | `suggestion_clicked`
    pub event_type: String,

    /// Stable session identifier from the client (uuid or nanoid) — optional.
    #[serde(default)]
    pub session_id: Option<String>,

    /// `product` | `recipe` | `nutrition` | `conversion`
    #[serde(default)]
    pub card_type: Option<String>,
    #[serde(default)]
    pub card_slug: Option<String>,

    /// `add_to_plan` | `add_to_shopping` | `start_cooking` | …
    #[serde(default)]
    pub action_type: Option<String>,

    #[serde(default)]
    pub intent: Option<String>,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub lang: Option<String>,

    /// Free-form metadata (card position, dismiss reason, etc.).
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Clone)]
pub struct ChatEventsService {
    pool: PgPool,
}

impl ChatEventsService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Append a single event. `user_id` may be `None` for anonymous sessions.
    /// Long `query` values are truncated to 500 chars to protect the table.
    pub async fn record(&self, user_id: Option<UserId>, event: ChatEvent) -> AppResult<()> {
        let query_truncated = event.query.as_ref().map(|q| {
            if q.chars().count() > 500 {
                q.chars().take(500).collect::<String>()
            } else {
                q.clone()
            }
        });

        sqlx::query(
            r#"INSERT INTO chat_events (
                user_id, session_id, event_type,
                card_type, card_slug, action_type,
                intent, query, lang, metadata
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)"#,
        )
        .bind(user_id.map(|u| *u.as_uuid()))
        .bind(event.session_id.as_deref())
        .bind(&event.event_type)
        .bind(event.card_type.as_deref())
        .bind(event.card_slug.as_deref())
        .bind(event.action_type.as_deref())
        .bind(event.intent.as_deref())
        .bind(query_truncated.as_deref())
        .bind(event.lang.as_deref())
        .bind(&event.metadata)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("chat_events insert: {e}")))?;

        Ok(())
    }

    /// Slugs of products the user dismissed ≥ `threshold` times in the last
    /// `days` days — used for nightly preference learning.
    ///
    /// Not called by the hot path; safe to keep SQL simple.
    #[allow(dead_code)]
    pub async fn recent_dismisses(
        &self,
        user_id: UserId,
        days: i32,
        threshold: i64,
    ) -> AppResult<Vec<String>> {
        let rows: Vec<(String,)> = sqlx::query_as(
            r#"SELECT card_slug
               FROM chat_events
               WHERE user_id = $1
                 AND event_type = 'card_dismissed'
                 AND card_slug IS NOT NULL
                 AND created_at > now() - ($2::INT || ' days')::interval
               GROUP BY card_slug
               HAVING count(*) >= $3
               ORDER BY count(*) DESC
               LIMIT 50"#,
        )
        .bind(user_id.as_uuid())
        .bind(days)
        .bind(threshold)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::internal(format!("recent_dismisses: {e}")))?;

        Ok(rows.into_iter().map(|(s,)| s).collect())
    }
}
