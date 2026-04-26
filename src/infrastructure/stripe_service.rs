//! Stripe integration — Checkout Sessions + Webhook signature verification.
//!
//! We deliberately avoid the heavyweight `async-stripe` SDK and talk to
//! the REST API with `reqwest` directly. The integration surface is tiny:
//!
//!   • `create_checkout_session` — build a hosted Checkout URL for a
//!     pre-defined action bundle. The bundle is identified by a stable
//!     server-side key (e.g. `"actions_20"`) which maps to a Stripe
//!     `price_id` configured via env vars. We pass `client_reference_id`
//!     = `user_id` and `metadata.actions` so the webhook can credit the
//!     right account without trusting the client.
//!
//!   • `verify_webhook_signature` — manually compute HMAC-SHA256 over
//!     `timestamp.body` and constant-time-compare against any of the
//!     comma-separated `v1=` signatures in the `Stripe-Signature` header.
//!     This matches the algorithm Stripe documents in their official
//!     server-side libraries.
//!
//! Test mode vs live mode is an environment-only switch (sk_test_… vs
//! sk_live_…). The same code paths run identically in both.

use crate::shared::{AppError, AppResult};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};
use subtle::ConstantTimeEq;

type HmacSha256 = Hmac<Sha256>;

// ════════════════════════════════════════════════════════════════════════════
// Bundle catalog — server-truth.
// Maps a stable bundle key to (Stripe price_id, action_count, label).
// Keys are referenced by clients; price_ids are wired via env vars so the
// same code works for test and live modes.
// ════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct ActionBundle {
    pub key: &'static str,
    pub label: &'static str,
    pub actions: i32,
    pub price_id_env: &'static str,
    /// Display price in minor units (cents) — for sanity logs only; the
    /// real charge is whatever Stripe's price object says.
    pub display_amount_eur_cents: i32,
}

pub const BUNDLES: &[ActionBundle] = &[
    ActionBundle {
        key: "actions_20",
        label: "20 actions",
        actions: 20,
        price_id_env: "STRIPE_PRICE_ACTIONS_20",
        display_amount_eur_cents: 199,
    },
    ActionBundle {
        key: "actions_100",
        label: "100 actions",
        actions: 100,
        price_id_env: "STRIPE_PRICE_ACTIONS_100",
        display_amount_eur_cents: 799,
    },
    ActionBundle {
        key: "actions_500",
        label: "500 actions",
        actions: 500,
        price_id_env: "STRIPE_PRICE_ACTIONS_500",
        display_amount_eur_cents: 2999,
    },
];

pub fn find_bundle(key: &str) -> Option<&'static ActionBundle> {
    BUNDLES.iter().find(|b| b.key == key)
}

// ════════════════════════════════════════════════════════════════════════════
// StripeService
// ════════════════════════════════════════════════════════════════════════════

#[derive(Clone)]
pub struct StripeService {
    secret_key: String,
    webhook_secret: String,
    success_url: String,
    cancel_url: String,
    http: reqwest::Client,
}

impl StripeService {
    /// Build from env vars. Returns `None` if `STRIPE_SECRET_KEY` is not set
    /// — billing is then disabled (handlers return 503).
    pub fn from_env() -> Option<Self> {
        let secret_key = std::env::var("STRIPE_SECRET_KEY").ok()?;
        let webhook_secret = std::env::var("STRIPE_WEBHOOK_SECRET").unwrap_or_default();
        let success_url = std::env::var("STRIPE_SUCCESS_URL")
            .unwrap_or_else(|_| "https://dima-fomin.pl/app/billing/success".to_string());
        let cancel_url = std::env::var("STRIPE_CANCEL_URL")
            .unwrap_or_else(|_| "https://dima-fomin.pl/app/billing/cancel".to_string());

        if webhook_secret.is_empty() {
            tracing::warn!(
                "STRIPE_WEBHOOK_SECRET is empty — webhook signature verification will fail!"
            );
        }

        Some(Self {
            secret_key,
            webhook_secret,
            success_url,
            cancel_url,
            http: reqwest::Client::new(),
        })
    }

    /// Resolve a bundle key to a real Stripe price_id (from env). We do NOT
    /// trust the client to send a price_id directly — that would let an
    /// attacker buy 500 actions for the price of 20.
    fn resolve_price_id(bundle: &ActionBundle) -> AppResult<String> {
        std::env::var(bundle.price_id_env).map_err(|_| {
            AppError::internal(format!(
                "Stripe price for bundle '{}' is not configured ({})",
                bundle.key, bundle.price_id_env
            ))
        })
    }

    /// Create a hosted Checkout Session and return its URL. The session
    /// carries `client_reference_id = user_id` and `metadata.actions =
    /// bundle.actions` so the webhook can credit the account safely.
    pub async fn create_checkout_session(
        &self,
        user_id: uuid::Uuid,
        user_email: Option<&str>,
        bundle: &ActionBundle,
    ) -> AppResult<String> {
        let price_id = Self::resolve_price_id(bundle)?;

        // Stripe expects `application/x-www-form-urlencoded` with
        // bracketed array notation — they explicitly do NOT accept JSON
        // for this endpoint.
        let mut form: Vec<(String, String)> = vec![
            ("mode".into(), "payment".into()),
            ("line_items[0][price]".into(), price_id),
            ("line_items[0][quantity]".into(), "1".into()),
            ("success_url".into(), format!("{}?session_id={{CHECKOUT_SESSION_ID}}", self.success_url)),
            ("cancel_url".into(), self.cancel_url.clone()),
            ("client_reference_id".into(), user_id.to_string()),
            ("metadata[user_id]".into(), user_id.to_string()),
            ("metadata[actions]".into(), bundle.actions.to_string()),
            ("metadata[bundle]".into(), bundle.key.to_string()),
            // Allow Stripe to handle the email — pre-fill if we have one.
            ("payment_intent_data[metadata][user_id]".into(), user_id.to_string()),
            ("payment_intent_data[metadata][bundle]".into(), bundle.key.to_string()),
        ];
        if let Some(email) = user_email {
            form.push(("customer_email".into(), email.to_string()));
        }

        let resp = self
            .http
            .post("https://api.stripe.com/v1/checkout/sessions")
            .basic_auth(&self.secret_key, Some(""))
            .form(&form)
            .send()
            .await
            .map_err(|e| AppError::internal(format!("Stripe request failed: {e}")))?;

        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| AppError::internal(format!("Stripe body read failed: {e}")))?;

        if !status.is_success() {
            tracing::error!("Stripe checkout error {status}: {body}");
            return Err(AppError::internal(format!(
                "Stripe checkout failed: {status}"
            )));
        }

        #[derive(Deserialize)]
        struct CheckoutSession {
            url: Option<String>,
        }
        let session: CheckoutSession = serde_json::from_str(&body)
            .map_err(|e| AppError::internal(format!("Stripe parse failed: {e}")))?;

        session
            .url
            .ok_or_else(|| AppError::internal("Stripe returned no checkout URL"))
    }

    /// Verify the `Stripe-Signature` header against the raw request body.
    /// Returns `Ok(())` only if at least one `v1=…` signature in the
    /// header matches HMAC-SHA256(`timestamp.body`) using the configured
    /// webhook secret, AND the timestamp is within the 5-minute window.
    pub fn verify_webhook_signature(&self, payload: &[u8], header: &str) -> AppResult<()> {
        if self.webhook_secret.is_empty() {
            return Err(AppError::internal(
                "STRIPE_WEBHOOK_SECRET is not configured",
            ));
        }

        // Header format: "t=1614012345,v1=hex,v1=hex,v0=…"
        let mut timestamp: Option<&str> = None;
        let mut signatures: Vec<&str> = Vec::new();
        for part in header.split(',') {
            let mut kv = part.splitn(2, '=');
            match (kv.next(), kv.next()) {
                (Some("t"), Some(v)) => timestamp = Some(v),
                (Some("v1"), Some(v)) => signatures.push(v),
                _ => {}
            }
        }
        let timestamp = timestamp.ok_or_else(|| {
            AppError::authentication("Missing timestamp in Stripe-Signature header")
        })?;
        if signatures.is_empty() {
            return Err(AppError::authentication(
                "No v1 signatures in Stripe-Signature header",
            ));
        }

        // Replay-attack window: reject events older than 5 minutes.
        let ts: i64 = timestamp
            .parse()
            .map_err(|_| AppError::authentication("Bad Stripe timestamp"))?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        if (now - ts).abs() > 300 {
            return Err(AppError::authentication(
                "Stripe webhook timestamp outside tolerance window",
            ));
        }

        // Compute expected HMAC-SHA256 over "timestamp.body".
        let mut mac = HmacSha256::new_from_slice(self.webhook_secret.as_bytes())
            .map_err(|_| AppError::internal("HMAC key error"))?;
        mac.update(timestamp.as_bytes());
        mac.update(b".");
        mac.update(payload);
        let expected = mac.finalize().into_bytes();

        // Constant-time compare against every supplied v1 signature.
        for sig_hex in signatures {
            if let Ok(sig_bytes) = hex::decode(sig_hex) {
                if sig_bytes.len() == expected.len()
                    && sig_bytes.ct_eq(expected.as_slice()).into()
                {
                    return Ok(());
                }
            }
        }
        Err(AppError::authentication("Stripe signature mismatch"))
    }
}

// ════════════════════════════════════════════════════════════════════════════
// Webhook event types — only the subset we care about.
// ════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct WebhookEvent {
    pub id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: WebhookEventData,
}

#[derive(Debug, Deserialize)]
pub struct WebhookEventData {
    pub object: serde_json::Value,
}

/// Strongly-typed view of the bits of `checkout.session` we need.
#[derive(Debug, Deserialize)]
pub struct CheckoutSessionObject {
    pub id: String,
    pub client_reference_id: Option<String>,
    pub payment_status: Option<String>,
    pub metadata: Option<serde_json::Map<String, serde_json::Value>>,
}

impl CheckoutSessionObject {
    pub fn user_id(&self) -> Option<uuid::Uuid> {
        // Prefer client_reference_id (set by us at checkout creation),
        // fall back to metadata.user_id for safety.
        self.client_reference_id
            .as_ref()
            .and_then(|s| uuid::Uuid::parse_str(s).ok())
            .or_else(|| {
                self.metadata
                    .as_ref()
                    .and_then(|m| m.get("user_id"))
                    .and_then(|v| v.as_str())
                    .and_then(|s| uuid::Uuid::parse_str(s).ok())
            })
    }

    pub fn actions_count(&self) -> Option<i32> {
        self.metadata
            .as_ref()
            .and_then(|m| m.get("actions"))
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
    }

    pub fn bundle_key(&self) -> Option<&str> {
        self.metadata
            .as_ref()
            .and_then(|m| m.get("bundle"))
            .and_then(|v| v.as_str())
    }

    pub fn is_paid(&self) -> bool {
        self.payment_status.as_deref() == Some("paid")
    }
}
