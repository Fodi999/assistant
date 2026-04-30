//! Audit log — все действия Copilot-а записываются в copilot_action_log.
//!
//! Статусы:
//!   planned              — план создан
//!   awaiting_confirmation — ждёт подтверждения пользователя
//!   executed             — выполнен
//!   cancelled            — отменён пользователем
//!   failed               — ошибка при выполнении

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::shared::{AppResult, TenantId, UserId};

use super::actions::ActionPlan;
use super::context::CopilotScreen;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "text")]
#[serde(rename_all = "snake_case")]
pub enum AuditStatus {
    Planned,
    AwaitingConfirmation,
    Executed,
    Cancelled,
    Failed,
}

/// Запись в аудит-логе.
#[derive(Debug, Serialize)]
pub struct CopilotAuditEntry {
    pub id: Uuid,
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub screen: String,
    pub input_message: String,
    pub intent: String,
    pub used_tools: Vec<String>,
    pub action_payload: serde_json::Value,
    pub status: AuditStatus,
    pub requires_confirmation: bool,
    pub confirmed_at: Option<time::OffsetDateTime>,
    pub executed_at: Option<time::OffsetDateTime>,
    pub created_at: time::OffsetDateTime,
}

pub struct CopilotAuditService {
    pool: PgPool,
}

impl CopilotAuditService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Записать новый запрос в лог.
    pub async fn record(
        &self,
        user_id: UserId,
        tenant_id: TenantId,
        screen: &CopilotScreen,
        message: &str,
        intent: &str,
        used_tools: &[String],
        action_plan: &Option<ActionPlan>,
        requires_confirmation: bool,
    ) -> AppResult<Uuid> {
        // Use action_plan.id so confirm works
        let id = action_plan.as_ref().map(|p| p.id).unwrap_or_else(Uuid::new_v4);
        let uid = *user_id.as_uuid();
        let tid = *tenant_id.as_uuid();
        let tools_json = serde_json::to_value(used_tools).unwrap_or_default();
        let payload = action_plan.as_ref()
            .map(|p| serde_json::to_value(p).unwrap_or_default())
            .unwrap_or(serde_json::Value::Null);
        let status = if requires_confirmation {
            "awaiting_confirmation"
        } else {
            "planned"
        };
        let screen_str = format!("{:?}", screen).to_lowercase();

        sqlx::query(
            "INSERT INTO copilot_action_log \
             (id, user_id, tenant_id, screen, input_message, intent, used_tools, action_payload, status, requires_confirmation) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"
        )
        .bind(id)
        .bind(uid)
        .bind(tid)
        .bind(&screen_str)
        .bind(message)
        .bind(intent)
        .bind(tools_json)
        .bind(payload)
        .bind(status)
        .bind(requires_confirmation)
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    /// Получить запись по action_id (для confirm endpoint).
    pub async fn get(&self, action_id: Uuid, user_id: UserId) -> AppResult<Option<CopilotAuditEntry>> {
        let uid = *user_id.as_uuid();
        let row = sqlx::query_as::<_, CopilotAuditRow>(
            r#"SELECT id, user_id, tenant_id, screen, input_message, intent,
                      used_tools, action_payload, status,
                      requires_confirmation, confirmed_at, executed_at, created_at
               FROM copilot_action_log
               WHERE id = $1 AND user_id = $2"#,
        )
        .bind(action_id)
        .bind(uid)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(row_to_entry))
    }

    /// Обновить статус на executed.
    pub async fn mark_executed(&self, action_id: Uuid) -> AppResult<()> {
        sqlx::query(
            "UPDATE copilot_action_log \
             SET status = 'executed', executed_at = NOW() \
             WHERE id = $1"
        )
        .bind(action_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Обновить статус на confirmed (перед выполнением).
    pub async fn mark_confirmed(&self, action_id: Uuid) -> AppResult<()> {
        sqlx::query(
            "UPDATE copilot_action_log \
             SET status = 'planned', confirmed_at = NOW() \
             WHERE id = $1"
        )
        .bind(action_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Отменить action.
    pub async fn mark_cancelled(&self, action_id: Uuid, user_id: UserId) -> AppResult<()> {
        let uid = *user_id.as_uuid();
        sqlx::query(
            "UPDATE copilot_action_log \
             SET status = 'cancelled' \
             WHERE id = $1 AND user_id = $2"
        )
        .bind(action_id)
        .bind(uid)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Отметить ошибку.
    pub async fn mark_failed(&self, action_id: Uuid, _error: &str) -> AppResult<()> {
        sqlx::query(
            "UPDATE copilot_action_log SET status = 'failed' WHERE id = $1"
        )
        .bind(action_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

// ── DB row types ─────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct CopilotAuditRow {
    id: Uuid,
    user_id: Uuid,
    tenant_id: Uuid,
    screen: String,
    input_message: String,
    intent: String,
    used_tools: serde_json::Value,
    action_payload: serde_json::Value,
    status: String,
    requires_confirmation: bool,
    confirmed_at: Option<time::OffsetDateTime>,
    executed_at: Option<time::OffsetDateTime>,
    created_at: time::OffsetDateTime,
}

fn row_to_entry(r: CopilotAuditRow) -> CopilotAuditEntry {
    let status = match r.status.as_str() {
        "awaiting_confirmation" => AuditStatus::AwaitingConfirmation,
        "executed"              => AuditStatus::Executed,
        "cancelled"             => AuditStatus::Cancelled,
        "failed"                => AuditStatus::Failed,
        _                       => AuditStatus::Planned,
    };
    let used_tools: Vec<String> = r.used_tools.as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    CopilotAuditEntry {
        id: r.id,
        user_id: r.user_id,
        tenant_id: r.tenant_id,
        screen: r.screen,
        input_message: r.input_message,
        intent: r.intent,
        used_tools,
        action_payload: r.action_payload,
        status,
        requires_confirmation: r.requires_confirmation,
        confirmed_at: r.confirmed_at,
        executed_at: r.executed_at,
        created_at: r.created_at,
    }
}
