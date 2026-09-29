//! Platform moderation: which businesses may appear in the public catalog.
//!
//! Only users listed in `platform_admin` may call this. A new business starts
//! `pending`; an admin approves or rejects it and may later suspend an
//! approved one. Every decision is recorded with its note.

use crate::application::auth::clean_optional;
use crate::application::schedule::format_utc;
use crate::infrastructure::{begin_scoped, DbScope};
use crate::shared::{AppError, AppResult, BusinessId, UserId};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

const DEFAULT_LIMIT: i64 = 50;
const MAX_LIMIT: i64 = 100;

#[derive(Debug, Deserialize)]
pub struct AdminListQuery {
    /// `pending`, `approved`, `rejected` or `suspended`; all when absent.
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Default, Deserialize)]
pub struct DecisionInput {
    /// Required when rejecting or suspending.
    pub note: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AdminBusinessView {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub city: Option<String>,
    pub headline: Option<String>,
    pub is_published: bool,
    pub moderation_status: String,
    pub created_at: String,
    pub owner_email: Option<String>,
    pub owner_name: Option<String>,
}

type Row = (
    Uuid,
    String,
    String,
    Option<String>,
    Option<String>,
    bool,
    String,
    OffsetDateTime,
    Option<String>,
    Option<String>,
);

fn view(row: Row) -> AdminBusinessView {
    AdminBusinessView {
        id: row.0,
        name: row.1,
        slug: row.2,
        city: row.3,
        headline: row.4,
        is_published: row.5,
        moderation_status: row.6,
        created_at: format_utc(row.7),
        owner_email: row.8,
        owner_name: row.9,
    }
}

const SELECT: &str = "SELECT b.id, b.name, b.slug, b.city, b.headline, b.is_published,
        b.moderation_status, b.created_at, u.email, u.display_name
     FROM business b
     LEFT JOIN membership m ON m.business_id = b.id AND m.role = 'owner' AND m.status = 'active'
     LEFT JOIN users u ON u.id = m.user_id";

const STATUSES: [&str; 4] = ["pending", "approved", "rejected", "suspended"];

#[derive(Clone)]
pub struct AdminService {
    pool: PgPool,
}

impl AdminService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn ensure_admin(&self, user_id: UserId) -> AppResult<()> {
        let mut tx = begin_scoped(&self.pool, DbScope::user(user_id)).await?;
        let admin: bool = sqlx::query_scalar("SELECT is_platform_admin()")
            .fetch_one(&mut *tx)
            .await?;
        if admin {
            Ok(())
        } else {
            Err(AppError::authorization("Platform administrators only"))
        }
    }

    pub async fn list(
        &self,
        admin: UserId,
        query: AdminListQuery,
    ) -> AppResult<Vec<AdminBusinessView>> {
        self.ensure_admin(admin).await?;
        if let Some(status) = query.status.as_deref() {
            if !STATUSES.contains(&status) {
                return Err(AppError::validation(
                    "status must be pending, approved, rejected or suspended",
                ));
            }
        }
        let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
        let offset = query.offset.unwrap_or(0).max(0);
        let mut tx = begin_scoped(&self.pool, DbScope::user(admin)).await?;
        let rows = sqlx::query_as::<_, Row>(&format!(
            "{SELECT}
             WHERE b.deleted_at IS NULL AND ($1::text IS NULL OR b.moderation_status = $1)
             ORDER BY b.created_at DESC, b.id
             LIMIT $2 OFFSET $3"
        ))
        .bind(query.status)
        .bind(limit)
        .bind(offset)
        .fetch_all(&mut *tx)
        .await?;
        Ok(rows.into_iter().map(view).collect())
    }

    /// `approved`, `rejected` or `suspended`. Repeating the current decision
    /// changes nothing.
    pub async fn decide(
        &self,
        admin: UserId,
        business_id: Uuid,
        target: &'static str,
        input: DecisionInput,
    ) -> AppResult<AdminBusinessView> {
        self.ensure_admin(admin).await?;
        let note = clean_optional(input.note, 1000, "note")?;
        if matches!(target, "rejected" | "suspended") && note.is_none() {
            return Err(AppError::validation(
                "A note is required when rejecting or suspending",
            ));
        }

        let business = BusinessId::from_uuid(business_id);
        let mut tx = begin_scoped(&self.pool, DbScope::business(admin, business)).await?;
        let current: String = sqlx::query_scalar(
            "SELECT moderation_status FROM business WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(business_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::not_found("Business not found"))?;

        if current == target {
            return load_one(&mut tx, business_id).await;
        }
        match (target, current.as_str()) {
            ("rejected", "pending") | ("suspended", "approved") => {}
            ("approved", _) => {}
            ("rejected", _) => {
                return Err(AppError::conflict(
                    "Only a pending business can be rejected",
                ));
            }
            _ => {
                return Err(AppError::conflict(
                    "Only an approved business can be suspended",
                ));
            }
        }

        sqlx::query(
            "UPDATE business SET moderation_status = $2, moderated_at = now() WHERE id = $1",
        )
        .bind(business_id)
        .bind(target)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO business_moderation (business_id, status, note, decided_by)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(business_id)
        .bind(target)
        .bind(&note)
        .bind(admin.as_uuid())
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO audit_log (business_id, actor_user_id, action, entity, entity_id)
             VALUES ($1, $2, $3, 'business', $1)",
        )
        .bind(business_id)
        .bind(admin.as_uuid())
        .bind(format!("business.moderation.{target}"))
        .execute(&mut *tx)
        .await?;
        let view = load_one(&mut tx, business_id).await?;
        tx.commit().await?;
        Ok(view)
    }
}

async fn load_one(conn: &mut PgConnection, business_id: Uuid) -> AppResult<AdminBusinessView> {
    sqlx::query_as::<_, Row>(&format!("{SELECT} WHERE b.id = $1"))
        .bind(business_id)
        .fetch_optional(&mut *conn)
        .await?
        .map(view)
        .ok_or_else(|| AppError::not_found("Business not found"))
}
