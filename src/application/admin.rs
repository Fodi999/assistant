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
use serde_json::Value;
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

#[derive(Debug, Serialize)]
pub struct AdminOwner {
    pub email: Option<String>,
    pub name: Option<String>,
    pub phone: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AdminStaff {
    pub id: Uuid,
    pub display_name: String,
    pub bio: Option<String>,
    pub is_bookable: bool,
}

#[derive(Debug, Serialize)]
pub struct AdminVariant {
    pub id: Uuid,
    pub name: Option<Value>,
    pub duration_min: i32,
    pub price_minor: i64,
    pub price_type: String,
    pub currency: String,
    pub is_active: bool,
}

#[derive(Debug, Serialize)]
pub struct AdminServiceItem {
    pub id: Uuid,
    pub name: Value,
    pub description: Option<Value>,
    pub is_active: bool,
    pub variants: Vec<AdminVariant>,
}

#[derive(Debug, Serialize)]
pub struct AdminModerationEntry {
    pub status: String,
    pub note: Option<String>,
    pub decided_by: Option<String>,
    pub created_at: String,
}

/// Everything an operator needs to decide on one business. Read-only.
#[derive(Debug, Serialize)]
pub struct AdminBusinessDetail {
    #[serde(flatten)]
    pub business: AdminBusinessView,
    pub about: Option<String>,
    pub instagram: Option<String>,
    pub owner: AdminOwner,
    pub staff: Vec<AdminStaff>,
    pub services: Vec<AdminServiceItem>,
    pub moderation_history: Vec<AdminModerationEntry>,
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

    /// Read-only moderation card. Runs in the business scope so the tenant
    /// policies on services and staff apply; the admin check comes first.
    pub async fn detail(&self, admin: UserId, business_id: Uuid) -> AppResult<AdminBusinessDetail> {
        self.ensure_admin(admin).await?;
        let business = BusinessId::from_uuid(business_id);
        let mut tx = begin_scoped(&self.pool, DbScope::business(admin, business)).await?;
        let base = load_one(&mut tx, business_id).await?;
        let (about, instagram): (Option<String>, Option<String>) =
            sqlx::query_as("SELECT about, instagram FROM business WHERE id = $1")
                .bind(business_id)
                .fetch_one(&mut *tx)
                .await?;
        let phone: Option<String> = sqlx::query_scalar(
            "SELECT u.phone_e164 FROM membership m JOIN users u ON u.id = m.user_id
             WHERE m.business_id = $1 AND m.role = 'owner' AND m.status = 'active' LIMIT 1",
        )
        .bind(business_id)
        .fetch_optional(&mut *tx)
        .await?
        .flatten();
        let staff = sqlx::query_as::<_, (Uuid, String, Option<String>, bool)>(
            "SELECT id, display_name, bio, is_bookable FROM staff_member
             WHERE business_id = $1 ORDER BY sort_order, created_at",
        )
        .bind(business_id)
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(|r| AdminStaff {
            id: r.0,
            display_name: r.1,
            bio: r.2,
            is_bookable: r.3,
        })
        .collect();

        let service_rows = sqlx::query_as::<_, (Uuid, String, Option<String>, bool)>(
            "SELECT id, name::text, description::text, is_active FROM service
             WHERE business_id = $1 AND deleted_at IS NULL ORDER BY sort_order, created_at",
        )
        .bind(business_id)
        .fetch_all(&mut *tx)
        .await?;
        let variant_rows =
            sqlx::query_as::<_, (Uuid, Uuid, Option<String>, i32, i64, String, String, bool)>(
                "SELECT id, service_id, name::text, duration_min, price_minor, price_type,
                    currency::text, is_active
             FROM service_variant
             WHERE business_id = $1 AND deleted_at IS NULL ORDER BY sort_order, created_at",
            )
            .bind(business_id)
            .fetch_all(&mut *tx)
            .await?;
        let mut services = Vec::with_capacity(service_rows.len());
        for (id, name, description, is_active) in service_rows {
            let mut variants = Vec::new();
            for v in variant_rows.iter().filter(|v| v.1 == id) {
                variants.push(AdminVariant {
                    id: v.0,
                    name: parse_json_opt(v.2.as_deref())?,
                    duration_min: v.3,
                    price_minor: v.4,
                    price_type: v.5.clone(),
                    currency: v.6.clone(),
                    is_active: v.7,
                });
            }
            services.push(AdminServiceItem {
                id,
                name: parse_json(&name)?,
                description: parse_json_opt(description.as_deref())?,
                is_active,
                variants,
            });
        }

        let moderation_history =
            sqlx::query_as::<_, (String, Option<String>, Option<String>, OffsetDateTime)>(
                "SELECT bm.status, bm.note, u.email, bm.created_at
             FROM business_moderation bm LEFT JOIN users u ON u.id = bm.decided_by
             WHERE bm.business_id = $1 ORDER BY bm.created_at DESC",
            )
            .bind(business_id)
            .fetch_all(&mut *tx)
            .await?
            .into_iter()
            .map(|r| AdminModerationEntry {
                status: r.0,
                note: r.1,
                decided_by: r.2,
                created_at: format_utc(r.3),
            })
            .collect();

        Ok(AdminBusinessDetail {
            about,
            instagram,
            owner: AdminOwner {
                email: base.owner_email.clone(),
                name: base.owner_name.clone(),
                phone,
            },
            business: base,
            staff,
            services,
            moderation_history,
        })
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

fn parse_json(text: &str) -> AppResult<Value> {
    serde_json::from_str(text).map_err(|_| AppError::internal("stored JSON is invalid"))
}

fn parse_json_opt(text: Option<&str>) -> AppResult<Option<Value>> {
    text.map(parse_json).transpose()
}

async fn load_one(conn: &mut PgConnection, business_id: Uuid) -> AppResult<AdminBusinessView> {
    sqlx::query_as::<_, Row>(&format!("{SELECT} WHERE b.id = $1"))
        .bind(business_id)
        .fetch_optional(&mut *conn)
        .await?
        .map(view)
        .ok_or_else(|| AppError::not_found("Business not found"))
}
