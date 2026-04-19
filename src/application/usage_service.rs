use crate::domain::usage::{
    ActionBalance, ActionResult, ActionSource, ActionType, BatchActionItem, BatchActionResult,
    DailyUsage, DenyReason, ServerLimits, UsageSnapshot,
};
use crate::shared::{AppError, AppResult, UserId};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

// ============================================================================
// UsageService — all monetization logic (server = truth)
// Fixes: transaction locks, idempotency, server-driven limits, batch
// ============================================================================

#[derive(Clone)]
pub struct UsageService {
    pool: PgPool,
}

impl UsageService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ────────────────────────────────────────────────────────────
    // Load server-driven limits from DB
    // ────────────────────────────────────────────────────────────

    async fn load_limits(&self) -> AppResult<ServerLimits> {
        let row = sqlx::query_as::<_, ServerLimitsRow>(
            "SELECT plans, recipes, scans, optimize, chats, cost_plan, cost_recipe, cost_scan, cost_optimize, cost_chat FROM usage_limits WHERE id = 1"
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(match row {
            Some(r) => ServerLimits {
                plans: r.plans, recipes: r.recipes, scans: r.scans,
                optimize: r.optimize, chats: r.chats,
                cost_plan: r.cost_plan, cost_recipe: r.cost_recipe,
                cost_scan: r.cost_scan, cost_optimize: r.cost_optimize,
                cost_chat: r.cost_chat,
            },
            None => ServerLimits::default(),
        })
    }

    pub async fn get_limits(&self) -> AppResult<ServerLimits> {
        self.load_limits().await
    }

    // ────────────────────────────────────────────────────────────
    // GET /api/usage/today
    // ────────────────────────────────────────────────────────────

    pub async fn get_today(&self, user_id: UserId) -> AppResult<(DailyUsage, ActionBalance)> {
        let today = OffsetDateTime::now_utc().date();
        let uid = *user_id.as_uuid();
        let usage = self.get_or_create_daily(uid, today).await?;
        let balance = self.get_or_create_balance(uid).await?;
        Ok((usage, balance))
    }

    // ────────────────────────────────────────────────────────────
    // Idempotency
    // ────────────────────────────────────────────────────────────

    pub async fn check_idempotency(&self, key: &str, user_id: Uuid) -> AppResult<Option<String>> {
        let _ = sqlx::query("DELETE FROM idempotency_keys WHERE created_at < NOW() - INTERVAL '24 hours'")
            .execute(&self.pool).await;

        let row = sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT response FROM idempotency_keys WHERE key = $1 AND user_id = $2"
        )
        .bind(key).bind(user_id)
        .fetch_optional(&self.pool).await?;

        Ok(row.map(|v| v.to_string()))
    }

    pub async fn store_idempotency(&self, key: &str, user_id: Uuid, response: &str) -> AppResult<()> {
        let json: serde_json::Value = serde_json::from_str(response)
            .unwrap_or_else(|_| serde_json::Value::String(response.to_string()));
        sqlx::query("INSERT INTO idempotency_keys (key, user_id, response) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING")
            .bind(key).bind(user_id).bind(json)
            .execute(&self.pool).await?;
        Ok(())
    }

    // ────────────────────────────────────────────────────────────
    // POST /api/usage/action — transaction + FOR UPDATE
    // ────────────────────────────────────────────────────────────

    pub async fn perform_action(
        &self,
        user_id: UserId,
        action: ActionType,
    ) -> AppResult<ActionResult> {
        let today = OffsetDateTime::now_utc().date();
        let uid = *user_id.as_uuid();
        let limits = self.load_limits().await?;

        // Ensure rows exist outside tx
        self.get_or_create_daily(uid, today).await?;
        self.get_or_create_balance(uid).await?;

        // === BEGIN TRANSACTION with row locks ===
        let mut tx = self.pool.begin().await?;

        let usage_row = sqlx::query_as::<_, DailyUsageRow>(
            "SELECT user_id, date, plans_used, recipes_used, scans_used, optimize_used, chats_used \
             FROM user_usage WHERE user_id = $1 AND date = $2 FOR UPDATE"
        )
        .bind(uid).bind(today)
        .fetch_one(&mut *tx).await?;

        let balance_row = sqlx::query_as::<_, ActionBalanceRow>(
            "SELECT user_id, purchased_actions, total_purchased, total_spent, welcome_bonus, last_weekly_bonus \
             FROM user_action_balance WHERE user_id = $1 FOR UPDATE"
        )
        .bind(uid)
        .fetch_one(&mut *tx).await?;

        let usage = daily_from_row(&usage_row);
        let balance = balance_from_row(&balance_row);
        let used = usage.used_count(action);
        let limit = limits.daily_limit(action);
        let cost = limits.action_cost(action);
        let usage_col = action.column();

        let result;

        if used < limit {
            // Free tier
            sqlx::query(&format!(
                "UPDATE user_usage SET {} = {} + 1, updated_at = NOW() WHERE user_id = $1 AND date = $2",
                usage_col, usage_col
            ))
            .bind(uid).bind(today)
            .execute(&mut *tx).await?;

            let remaining_free = limit - used - 1;
            let warning = remaining_free <= 1;

            let mut snap = build_snapshot(&usage, &limits);
            snap.set_for_action(action, remaining_free);
            snap.purchased_actions = balance.purchased_actions;

            result = ActionResult {
                allowed: true,
                source: ActionSource::FreeTier,
                reason: None,
                remaining_free,
                purchased_actions_left: balance.purchased_actions,
                warning,
                message: if remaining_free == 0 {
                    Some(format!("Last free {} for today", action.as_str()))
                } else if warning {
                    Some(format!("Only {} free {} left today", remaining_free, action.as_str()))
                } else { None },
                usage: snap,
            };
        } else if balance.purchased_actions >= cost {
            // Purchased
            sqlx::query(&format!(
                "UPDATE user_usage SET {} = {} + 1, updated_at = NOW() WHERE user_id = $1 AND date = $2",
                usage_col, usage_col
            ))
            .bind(uid).bind(today)
            .execute(&mut *tx).await?;

            sqlx::query(
                "UPDATE user_action_balance SET purchased_actions = purchased_actions - $2, \
                 total_spent = total_spent + $2, updated_at = NOW() WHERE user_id = $1"
            )
            .bind(uid).bind(cost)
            .execute(&mut *tx).await?;

            let new_purchased = balance.purchased_actions - cost;
            let mut snap = build_snapshot(&usage, &limits);
            snap.purchased_actions = new_purchased;

            result = ActionResult {
                allowed: true,
                source: ActionSource::Purchased,
                reason: None,
                remaining_free: 0,
                purchased_actions_left: new_purchased,
                warning: new_purchased < cost,
                message: Some(format!("Used {} purchased actions", cost)),
                usage: snap,
            };
        } else {
            // Denied
            let snap = UsageSnapshot {
                plans_left: usage.free_remaining_with(ActionType::GeneratePlan, &limits),
                recipes_left: usage.free_remaining_with(ActionType::CreateRecipe, &limits),
                scans_left: usage.free_remaining_with(ActionType::ScanReceipt, &limits),
                optimize_left: usage.free_remaining_with(ActionType::OptimizeDay, &limits),
                chats_left: usage.free_remaining_with(ActionType::AiChat, &limits),
                purchased_actions: balance.purchased_actions,
            };

            result = ActionResult {
                allowed: false,
                source: ActionSource::Denied,
                reason: Some(if used >= limit { DenyReason::DailyLimitReached } else { DenyReason::InsufficientActions }),
                remaining_free: 0,
                purchased_actions_left: balance.purchased_actions,
                warning: false,
                message: Some(format!("No free {}s left and not enough actions (need {})", action.as_str(), cost)),
                usage: snap,
            };
        }

        tx.commit().await?;
        Ok(result)
    }

    // ────────────────────────────────────────────────────────────
    // POST /api/usage/actions — batch (single transaction)
    // ────────────────────────────────────────────────────────────

    pub async fn perform_batch(
        &self,
        user_id: UserId,
        actions: Vec<ActionType>,
    ) -> AppResult<BatchActionResult> {
        let today = OffsetDateTime::now_utc().date();
        let uid = *user_id.as_uuid();
        let limits = self.load_limits().await?;

        self.get_or_create_daily(uid, today).await?;
        self.get_or_create_balance(uid).await?;

        let mut tx = self.pool.begin().await?;

        let mut usage_row = sqlx::query_as::<_, DailyUsageRow>(
            "SELECT user_id, date, plans_used, recipes_used, scans_used, optimize_used, chats_used \
             FROM user_usage WHERE user_id = $1 AND date = $2 FOR UPDATE"
        )
        .bind(uid).bind(today).fetch_one(&mut *tx).await?;

        let mut balance_row = sqlx::query_as::<_, ActionBalanceRow>(
            "SELECT user_id, purchased_actions, total_purchased, total_spent, welcome_bonus, last_weekly_bonus \
             FROM user_action_balance WHERE user_id = $1 FOR UPDATE"
        )
        .bind(uid).fetch_one(&mut *tx).await?;

        let mut items = Vec::with_capacity(actions.len());

        for action in &actions {
            let used = get_used_from_row(&usage_row, *action);
            let limit = limits.daily_limit(*action);
            let cost = limits.action_cost(*action);

            if used < limit {
                increment_row(&mut usage_row, *action);
                items.push(BatchActionItem {
                    action: action.as_str().to_string(),
                    allowed: true, source: ActionSource::FreeTier,
                    reason: None, message: None,
                });
            } else if balance_row.purchased_actions >= cost {
                increment_row(&mut usage_row, *action);
                balance_row.purchased_actions -= cost;
                balance_row.total_spent += cost;
                items.push(BatchActionItem {
                    action: action.as_str().to_string(),
                    allowed: true, source: ActionSource::Purchased,
                    reason: None, message: Some(format!("Used {} actions", cost)),
                });
            } else {
                items.push(BatchActionItem {
                    action: action.as_str().to_string(),
                    allowed: false, source: ActionSource::Denied,
                    reason: Some(if used >= limit { DenyReason::DailyLimitReached } else { DenyReason::InsufficientActions }),
                    message: Some(format!("Need {} actions", cost)),
                });
            }
        }

        sqlx::query(
            "UPDATE user_usage SET plans_used=$3, recipes_used=$4, scans_used=$5, \
             optimize_used=$6, chats_used=$7, updated_at=NOW() WHERE user_id=$1 AND date=$2"
        )
        .bind(uid).bind(today)
        .bind(usage_row.plans_used).bind(usage_row.recipes_used)
        .bind(usage_row.scans_used).bind(usage_row.optimize_used)
        .bind(usage_row.chats_used)
        .execute(&mut *tx).await?;

        sqlx::query(
            "UPDATE user_action_balance SET purchased_actions=$2, total_spent=$3, updated_at=NOW() WHERE user_id=$1"
        )
        .bind(uid).bind(balance_row.purchased_actions).bind(balance_row.total_spent)
        .execute(&mut *tx).await?;

        tx.commit().await?;

        let usage = daily_from_row(&usage_row);
        let snap = UsageSnapshot {
            plans_left: usage.free_remaining_with(ActionType::GeneratePlan, &limits),
            recipes_left: usage.free_remaining_with(ActionType::CreateRecipe, &limits),
            scans_left: usage.free_remaining_with(ActionType::ScanReceipt, &limits),
            optimize_left: usage.free_remaining_with(ActionType::OptimizeDay, &limits),
            chats_left: usage.free_remaining_with(ActionType::AiChat, &limits),
            purchased_actions: balance_row.purchased_actions,
        };

        Ok(BatchActionResult { results: items, usage: snap })
    }

    // ────────────────────────────────────────────────────────────
    // POST /api/usage/purchase
    // ────────────────────────────────────────────────────────────

    pub async fn record_purchase(
        &self,
        user_id: UserId,
        actions: i32,
        source: &str,
        receipt_id: Option<&str>,
    ) -> AppResult<ActionBalance> {
        let uid = *user_id.as_uuid();
        if actions <= 0 {
            return Err(AppError::validation("Actions must be positive"));
        }
        self.get_or_create_balance(uid).await?;

        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "UPDATE user_action_balance SET purchased_actions = purchased_actions + $2, \
             total_purchased = total_purchased + $2, updated_at = NOW() WHERE user_id = $1"
        ).bind(uid).bind(actions).execute(&mut *tx).await?;

        sqlx::query(
            "INSERT INTO usage_purchases (user_id, actions, source, receipt_id) VALUES ($1, $2, $3, $4)"
        ).bind(uid).bind(actions).bind(source).bind(receipt_id)
        .execute(&mut *tx).await?;

        tx.commit().await?;
        self.get_or_create_balance(uid).await
    }

    // ────────────────────────────────────────────────────────────
    // Welcome bonus (+20, once per user, idempotent)
    // ────────────────────────────────────────────────────────────

    pub async fn grant_welcome_bonus(&self, user_id: UserId) -> AppResult<ActionBalance> {
        let uid = *user_id.as_uuid();
        let balance = self.get_or_create_balance(uid).await?;
        if balance.welcome_bonus { return Ok(balance); }

        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "UPDATE user_action_balance SET purchased_actions = purchased_actions + 20, \
             total_purchased = total_purchased + 20, welcome_bonus = TRUE, updated_at = NOW() \
             WHERE user_id = $1 AND welcome_bonus = FALSE"
        ).bind(uid).execute(&mut *tx).await?;

        sqlx::query(
            "INSERT INTO usage_purchases (user_id, actions, source) VALUES ($1, 20, 'welcome_bonus')"
        ).bind(uid).execute(&mut *tx).await?;

        tx.commit().await?;
        self.get_or_create_balance(uid).await
    }

    /// Auto-grant on login (fire-and-forget, idempotent)
    pub async fn auto_welcome_bonus_if_needed(&self, user_id: UserId) -> AppResult<()> {
        let _ = self.grant_welcome_bonus(user_id).await;
        Ok(())
    }

    // ────────────────────────────────────────────────────────────
    // Weekly bonus (+5, Mondays, idempotent)
    // ────────────────────────────────────────────────────────────

    pub async fn check_weekly_bonus(&self, user_id: UserId) -> AppResult<Option<ActionBalance>> {
        let uid = *user_id.as_uuid();
        let today = OffsetDateTime::now_utc().date();
        if today.weekday() != time::Weekday::Monday { return Ok(None); }

        let balance = self.get_or_create_balance(uid).await?;
        if let Some(last) = balance.last_weekly_bonus {
            if last == today { return Ok(None); }
        }

        let mut tx = self.pool.begin().await?;
        let rows = sqlx::query(
            "UPDATE user_action_balance SET purchased_actions = purchased_actions + 5, \
             total_purchased = total_purchased + 5, last_weekly_bonus = $2, updated_at = NOW() \
             WHERE user_id = $1 AND (last_weekly_bonus IS NULL OR last_weekly_bonus < $2)"
        ).bind(uid).bind(today).execute(&mut *tx).await?;

        if rows.rows_affected() > 0 {
            sqlx::query(
                "INSERT INTO usage_purchases (user_id, actions, source) VALUES ($1, 5, 'weekly_bonus')"
            ).bind(uid).execute(&mut *tx).await?;
        }

        tx.commit().await?;
        let updated = self.get_or_create_balance(uid).await?;
        Ok(Some(updated))
    }

    // ════════════════════════════════════════════════════════════
    // Private helpers
    // ════════════════════════════════════════════════════════════

    async fn get_or_create_daily(&self, user_id: Uuid, date: Date) -> AppResult<DailyUsage> {
        sqlx::query("INSERT INTO user_usage (user_id, date) VALUES ($1, $2) ON CONFLICT (user_id, date) DO NOTHING")
            .bind(user_id).bind(date).execute(&self.pool).await?;

        let row = sqlx::query_as::<_, DailyUsageRow>(
            "SELECT user_id, date, plans_used, recipes_used, scans_used, optimize_used, chats_used \
             FROM user_usage WHERE user_id = $1 AND date = $2"
        ).bind(user_id).bind(date).fetch_one(&self.pool).await?;

        Ok(daily_from_row(&row))
    }

    async fn get_or_create_balance(&self, user_id: Uuid) -> AppResult<ActionBalance> {
        sqlx::query("INSERT INTO user_action_balance (user_id) VALUES ($1) ON CONFLICT (user_id) DO NOTHING")
            .bind(user_id).execute(&self.pool).await?;

        let row = sqlx::query_as::<_, ActionBalanceRow>(
            "SELECT user_id, purchased_actions, total_purchased, total_spent, welcome_bonus, last_weekly_bonus \
             FROM user_action_balance WHERE user_id = $1"
        ).bind(user_id).fetch_one(&self.pool).await?;

        Ok(balance_from_row(&row))
    }
}

// ════════════════════════════════════════════════════════════
// Row mappings + helpers
// ════════════════════════════════════════════════════════════

#[derive(sqlx::FromRow, Clone)]
struct DailyUsageRow {
    user_id: Uuid,
    date: Date,
    plans_used: i32,
    recipes_used: i32,
    scans_used: i32,
    optimize_used: i32,
    chats_used: i32,
}

#[derive(sqlx::FromRow, Clone)]
struct ActionBalanceRow {
    user_id: Uuid,
    purchased_actions: i32,
    total_purchased: i32,
    total_spent: i32,
    welcome_bonus: bool,
    last_weekly_bonus: Option<Date>,
}

#[derive(sqlx::FromRow)]
struct ServerLimitsRow {
    plans: i32,
    recipes: i32,
    scans: i32,
    optimize: i32,
    chats: i32,
    cost_plan: i32,
    cost_recipe: i32,
    cost_scan: i32,
    cost_optimize: i32,
    cost_chat: i32,
}

fn daily_from_row(r: &DailyUsageRow) -> DailyUsage {
    DailyUsage {
        user_id: r.user_id, date: r.date,
        plans_used: r.plans_used, recipes_used: r.recipes_used,
        scans_used: r.scans_used, optimize_used: r.optimize_used,
        chats_used: r.chats_used,
    }
}

fn balance_from_row(r: &ActionBalanceRow) -> ActionBalance {
    ActionBalance {
        user_id: r.user_id, purchased_actions: r.purchased_actions,
        total_purchased: r.total_purchased, total_spent: r.total_spent,
        welcome_bonus: r.welcome_bonus, last_weekly_bonus: r.last_weekly_bonus,
    }
}

fn get_used_from_row(r: &DailyUsageRow, action: ActionType) -> i32 {
    match action {
        ActionType::GeneratePlan => r.plans_used,
        ActionType::CreateRecipe => r.recipes_used,
        ActionType::ScanReceipt => r.scans_used,
        ActionType::OptimizeDay => r.optimize_used,
        ActionType::AiChat => r.chats_used,
    }
}

fn increment_row(r: &mut DailyUsageRow, action: ActionType) {
    match action {
        ActionType::GeneratePlan => r.plans_used += 1,
        ActionType::CreateRecipe => r.recipes_used += 1,
        ActionType::ScanReceipt => r.scans_used += 1,
        ActionType::OptimizeDay => r.optimize_used += 1,
        ActionType::AiChat => r.chats_used += 1,
    }
}

fn build_snapshot(usage: &DailyUsage, limits: &ServerLimits) -> UsageSnapshot {
    UsageSnapshot {
        plans_left: usage.free_remaining_with(ActionType::GeneratePlan, limits),
        recipes_left: usage.free_remaining_with(ActionType::CreateRecipe, limits),
        scans_left: usage.free_remaining_with(ActionType::ScanReceipt, limits),
        optimize_left: usage.free_remaining_with(ActionType::OptimizeDay, limits),
        chats_left: usage.free_remaining_with(ActionType::AiChat, limits),
        purchased_actions: 0,
    }
}

impl UsageSnapshot {
    fn set_for_action(&mut self, action: ActionType, remaining: i32) {
        match action {
            ActionType::GeneratePlan => self.plans_left = remaining,
            ActionType::CreateRecipe => self.recipes_left = remaining,
            ActionType::ScanReceipt => self.scans_left = remaining,
            ActionType::OptimizeDay => self.optimize_left = remaining,
            ActionType::AiChat => self.chats_left = remaining,
        }
    }
}
