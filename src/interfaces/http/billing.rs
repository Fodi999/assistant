//! HTTP handlers for Stripe-backed billing.
//!
//!   • `POST /api/billing/checkout` — authenticated. Creates a Stripe
//!     Checkout Session for one of the predefined action bundles and
//!     returns its hosted URL. The user_id comes from the JWT — not
//!     from the request body — so the credit always lands on the
//!     authenticated account.
//!
//!   • `POST /webhooks/stripe` — public, but verified via the
//!     `Stripe-Signature` header (HMAC-SHA256 with the webhook secret).
//!     We listen for `checkout.session.completed` and credit the user's
//!     `purchased_actions` via `UsageService::record_purchase`. The
//!     receipt_id (Stripe session id) is stored UNIQUE so retries are
//!     idempotent — Stripe DOES retry on non-2xx, network blips, and
//!     replays.

use crate::application::usage_service::UsageService;
use crate::infrastructure::stripe_service::{
    find_bundle, CheckoutSessionObject, StripeService, WebhookEvent, BUNDLES,
};
use crate::interfaces::http::middleware::AuthUser;
use crate::shared::{AppError, AppResult, UserId};
use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

// ════════════════════════════════════════════════════════════════════════════
// State bundle for billing routes
// ════════════════════════════════════════════════════════════════════════════

#[derive(Clone)]
pub struct BillingState {
    pub stripe: StripeService,
    pub usage: UsageService,
    pub pool: PgPool,
}

// ════════════════════════════════════════════════════════════════════════════
// GET /api/billing/bundles — public catalog of available bundles
// (no Stripe call, no auth required — used by the pricing page)
// ════════════════════════════════════════════════════════════════════════════

#[derive(Serialize)]
pub struct BundleDto {
    pub key: &'static str,
    pub label: &'static str,
    pub actions: i32,
    pub price_eur_cents: i32,
}

pub async fn list_bundles() -> Json<Vec<BundleDto>> {
    let dtos: Vec<BundleDto> = BUNDLES
        .iter()
        .map(|b| BundleDto {
            key: b.key,
            label: b.label,
            actions: b.actions,
            price_eur_cents: b.display_amount_eur_cents,
        })
        .collect();
    Json(dtos)
}

// ════════════════════════════════════════════════════════════════════════════
// POST /api/billing/checkout — authenticated
// Body: { "bundle": "actions_20" }
// Resp: { "url": "https://checkout.stripe.com/c/pay/cs_test_…" }
// ════════════════════════════════════════════════════════════════════════════

#[derive(Deserialize)]
pub struct CheckoutRequest {
    pub bundle: String,
}

#[derive(Serialize)]
pub struct CheckoutResponse {
    pub url: String,
}

pub async fn create_checkout(
    State(state): State<BillingState>,
    auth: AuthUser,
    Json(req): Json<CheckoutRequest>,
) -> AppResult<Json<CheckoutResponse>> {
    let bundle = find_bundle(&req.bundle)
        .ok_or_else(|| AppError::validation(format!("Unknown bundle '{}'", req.bundle)))?;

    // Pull the user's email for a nicer Checkout pre-fill — non-fatal if
    // missing.
    let email: Option<String> = sqlx::query_scalar::<_, String>(
        "SELECT email FROM users WHERE id = $1",
    )
    .bind(auth.user_id.as_uuid())
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    let url = state
        .stripe
        .create_checkout_session(*auth.user_id.as_uuid(), email.as_deref(), bundle)
        .await?;

    Ok(Json(CheckoutResponse { url }))
}

// ════════════════════════════════════════════════════════════════════════════
// POST /webhooks/stripe — public; HMAC-verified
// Body: raw Stripe event JSON. We MUST receive the raw bytes (no Json
// extractor) because the signature is computed over the exact payload.
// ════════════════════════════════════════════════════════════════════════════

pub async fn stripe_webhook(
    State(state): State<BillingState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, (StatusCode, String)> {
    // 1. Signature verification — fail closed.
    let sig_header = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                "Missing Stripe-Signature header".to_string(),
            )
        })?;

    if let Err(e) = state.stripe.verify_webhook_signature(&body, sig_header) {
        tracing::warn!("Stripe webhook signature rejected: {e}");
        return Err((StatusCode::UNAUTHORIZED, "bad signature".to_string()));
    }

    // 2. Parse event envelope.
    let event: WebhookEvent = serde_json::from_slice(&body).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("invalid event JSON: {e}"),
        )
    })?;

    tracing::info!("Stripe webhook event {} type={}", event.id, event.event_type);

    // 3. Dispatch — only the events we care about right now.
    match event.event_type.as_str() {
        "checkout.session.completed" | "checkout.session.async_payment_succeeded" => {
            let session: CheckoutSessionObject = serde_json::from_value(event.data.object)
                .map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        format!("session parse failed: {e}"),
                    )
                })?;

            // Only credit on confirmed payment. For sync card payments
            // `payment_status` is already "paid" at this stage.
            if !session.is_paid() {
                tracing::info!(
                    "Stripe session {} not yet paid (status={:?}) — skipping credit",
                    session.id,
                    session.payment_status
                );
                return Ok(StatusCode::OK);
            }

            let user_uuid = session.user_id().ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    "missing user_id on session".to_string(),
                )
            })?;
            let actions = session.actions_count().ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    "missing actions metadata on session".to_string(),
                )
            })?;
            let bundle_key = session.bundle_key().unwrap_or("stripe");

            // 4. Idempotent credit. The UNIQUE index on
            // `usage_purchases.receipt_id` (migration 20260426000001)
            // makes the second call a duplicate-key error, which we
            // swallow and return 200 to Stripe.
            match state
                .usage
                .record_purchase(
                    UserId::from(user_uuid),
                    actions,
                    &format!("stripe:{bundle_key}"),
                    Some(&session.id),
                )
                .await
            {
                Ok(_) => {
                    tracing::info!(
                        "Stripe credit OK: user={} +{} actions session={}",
                        user_uuid,
                        actions,
                        session.id
                    );
                }
                Err(AppError::Database(sqlx::Error::Database(db)))
                    if db.constraint() == Some("idx_usage_purchases_receipt_unique") =>
                {
                    tracing::info!(
                        "Stripe webhook duplicate session={} — already credited",
                        session.id
                    );
                }
                Err(e) => {
                    tracing::error!("Stripe credit FAILED: {e}");
                    // Return 500 so Stripe retries.
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "credit failed".to_string(),
                    ));
                }
            }
        }
        other => {
            tracing::debug!("Stripe webhook event type {} ignored", other);
        }
    }

    Ok(StatusCode::OK)
}
