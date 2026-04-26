use crate::application::usage_service::UsageService;
use crate::domain::usage::{ActionType, UsageSnapshot};
use crate::interfaces::http::middleware::AuthUser;
use crate::shared::AppError;
use axum::{extract::State, http::HeaderMap, Json};
use serde::{Deserialize, Serialize};

// ============================================================================
// GET /api/usage/today — full usage snapshot for iOS
// ============================================================================

#[derive(Debug, Serialize)]
pub struct UsageTodayResponse {
    pub plans_left: i32,
    pub recipes_left: i32,
    pub scans_left: i32,
    pub optimize_left: i32,
    pub chats_left: i32,
    pub purchased_actions: i32,
    /// Lifetime total of actions ever credited (purchases + bonuses).
    /// Used by the AI Wallet UI as the denominator of the progress bar.
    pub total_purchased: i32,
    /// Lifetime total of actions consumed from the purchased balance.
    pub total_spent: i32,
    /// Subset of `total_purchased` that came from non-IAP sources
    /// (welcome bonus, weekly bonus, promo codes).
    pub bonus_actions: i32,
    pub daily_limits: LimitsResponse,
    pub costs: CostsResponse,
    pub welcome_bonus_granted: bool,
}

#[derive(Debug, Serialize)]
pub struct LimitsResponse {
    pub plans: i32,
    pub recipes: i32,
    pub scans: i32,
    pub optimize: i32,
    pub chats: i32,
}

#[derive(Debug, Serialize)]
pub struct CostsResponse {
    pub generate_plan: i32,
    pub create_recipe: i32,
    pub scan_receipt: i32,
    pub optimize_day: i32,
    pub ai_chat: i32,
}

pub async fn get_today(
    auth: AuthUser,
    State(service): State<UsageService>,
) -> Result<Json<UsageTodayResponse>, AppError> {
    let _ = service.check_weekly_bonus(auth.user_id).await;
    let (usage, balance) = service.get_today(auth.user_id).await?;
    let limits = service.get_limits().await?;
    let bonus_actions = service.get_bonus_actions(auth.user_id).await.unwrap_or(0);

    Ok(Json(UsageTodayResponse {
        plans_left: usage.free_remaining(ActionType::GeneratePlan, &limits),
        recipes_left: usage.free_remaining(ActionType::CreateRecipe, &limits),
        scans_left: usage.free_remaining(ActionType::ScanReceipt, &limits),
        optimize_left: usage.free_remaining(ActionType::OptimizeDay, &limits),
        chats_left: usage.free_remaining(ActionType::AiChat, &limits),
        purchased_actions: balance.purchased_actions,
        total_purchased: balance.total_purchased,
        total_spent: balance.total_spent,
        bonus_actions,
        daily_limits: LimitsResponse {
            plans: limits.plans,
            recipes: limits.recipes,
            scans: limits.scans,
            optimize: limits.optimize,
            chats: limits.chats,
        },
        costs: CostsResponse {
            generate_plan: limits.cost_plan,
            create_recipe: limits.cost_recipe,
            scan_receipt: limits.cost_scan,
            optimize_day: limits.cost_optimize,
            ai_chat: limits.cost_chat,
        },
        welcome_bonus_granted: balance.welcome_bonus,
    }))
}

// ============================================================================
// POST /api/usage/action — with Idempotency-Key support
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ActionRequest {
    pub action: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActionResponse {
    pub allowed: bool,
    pub source: String,
    pub reason: Option<String>,
    pub remaining_free: i32,
    pub purchased_actions_left: i32,
    pub warning: bool,
    pub message: Option<String>,
    pub usage: UsageSnapshot,
}

pub async fn perform_action(
    auth: AuthUser,
    headers: HeaderMap,
    State(service): State<UsageService>,
    Json(req): Json<ActionRequest>,
) -> Result<Json<ActionResponse>, AppError> {
    let action = ActionType::from_str(&req.action)
        .ok_or_else(|| AppError::validation(format!("Unknown action: {}", req.action)))?;

    let uid = *auth.user_id.as_uuid();

    // Idempotency check
    if let Some(idem_key) = headers.get("idempotency-key").and_then(|v| v.to_str().ok()) {
        if let Some(cached) = service.check_idempotency(idem_key, uid).await? {
            if let Ok(resp) = serde_json::from_str::<ActionResponse>(&cached) {
                return Ok(Json(resp));
            }
        }
    }

    let result = service.perform_action(auth.user_id, action).await?;

    let resp = ActionResponse {
        allowed: result.allowed,
        source: format!("{:?}", result.source).to_lowercase(),
        reason: result.reason.map(|r| format!("{:?}", r).to_lowercase()),
        remaining_free: result.remaining_free,
        purchased_actions_left: result.purchased_actions_left,
        warning: result.warning,
        message: result.message,
        usage: result.usage,
    };

    // Store idempotency
    if let Some(idem_key) = headers.get("idempotency-key").and_then(|v| v.to_str().ok()) {
        if let Ok(json) = serde_json::to_string(&resp) {
            let _ = service.store_idempotency(idem_key, uid, &json).await;
        }
    }

    Ok(Json(resp))
}

// ============================================================================
// POST /api/usage/actions — batch actions (single transaction)
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct BatchRequest {
    pub actions: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct BatchResponse {
    pub results: Vec<BatchItemResponse>,
    pub usage: UsageSnapshot,
}

#[derive(Debug, Serialize)]
pub struct BatchItemResponse {
    pub action: String,
    pub allowed: bool,
    pub source: String,
    pub reason: Option<String>,
    pub message: Option<String>,
}

pub async fn perform_batch(
    auth: AuthUser,
    State(service): State<UsageService>,
    Json(req): Json<BatchRequest>,
) -> Result<Json<BatchResponse>, AppError> {
    if req.actions.is_empty() {
        return Err(AppError::validation("Actions list cannot be empty"));
    }
    if req.actions.len() > 10 {
        return Err(AppError::validation("Max 10 actions per batch"));
    }

    let action_types: Vec<ActionType> = req.actions.iter()
        .map(|s| ActionType::from_str(s).ok_or_else(|| AppError::validation(format!("Unknown action: {}", s))))
        .collect::<Result<Vec<_>, _>>()?;

    let result = service.perform_batch(auth.user_id, action_types).await?;

    let items = result.results.into_iter().map(|r| BatchItemResponse {
        action: r.action,
        allowed: r.allowed,
        source: format!("{:?}", r.source).to_lowercase(),
        reason: r.reason.map(|r| format!("{:?}", r).to_lowercase()),
        message: r.message,
    }).collect();

    Ok(Json(BatchResponse {
        results: items,
        usage: result.usage,
    }))
}

// ============================================================================
// POST /api/usage/purchase
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct PurchaseRequest {
    pub actions: i32,
    pub receipt_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PurchaseResponse {
    pub purchased_actions: i32,
    pub total_purchased: i32,
}

pub async fn record_purchase(
    auth: AuthUser,
    State(service): State<UsageService>,
    Json(req): Json<PurchaseRequest>,
) -> Result<Json<PurchaseResponse>, AppError> {
    let balance = service.record_purchase(auth.user_id, req.actions, "iap", req.receipt_id.as_deref()).await?;
    Ok(Json(PurchaseResponse {
        purchased_actions: balance.purchased_actions,
        total_purchased: balance.total_purchased,
    }))
}

// ============================================================================
// POST /api/usage/welcome-bonus
// ============================================================================

#[derive(Debug, Serialize)]
pub struct BonusResponse {
    pub purchased_actions: i32,
    pub granted: bool,
}

pub async fn grant_welcome_bonus(
    auth: AuthUser,
    State(service): State<UsageService>,
) -> Result<Json<BonusResponse>, AppError> {
    let before = service.get_today(auth.user_id).await?.1;
    let after = service.grant_welcome_bonus(auth.user_id).await?;
    Ok(Json(BonusResponse {
        purchased_actions: after.purchased_actions,
        granted: !before.welcome_bonus && after.welcome_bonus,
    }))
}
