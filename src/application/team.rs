//! Team: adding an existing account to a business as a member with a bookable
//! staff card. Invitations by e-mail/link replace this later.
//!
//! Owners add anyone below owner; managers add employees and reception only.

use crate::application::access::{BusinessAccess, Role};
use crate::application::auth::{clean_optional, on_unique};
use crate::infrastructure::begin_scoped;
use crate::shared::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct AddMemberInput {
    pub email: String,
    /// `manager`, `reception` or `employee`.
    pub role: String,
    /// Staff card name; defaults to the account's display name.
    pub display_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MemberView {
    pub membership_id: Uuid,
    pub staff_id: Uuid,
    pub role: Role,
    pub display_name: String,
}

/// One person of the team as the owner sees it.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct MemberDetail {
    pub membership_id: Uuid,
    pub staff_id: Option<Uuid>,
    pub role: String,
    /// `active` or `suspended`.
    pub status: String,
    pub display_name: String,
    pub is_bookable: bool,
    pub is_me: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMemberInput {
    /// `manager`, `reception` or `employee`.
    pub role: Option<String>,
    /// `active` or `suspended`.
    pub status: Option<String>,
}

const MEMBER_SELECT: &str = "SELECT m.id AS membership_id, s.id AS staff_id, m.role, m.status,
        COALESCE(s.display_name, '') AS display_name,
        COALESCE(s.is_bookable, false) AS is_bookable,
        (m.user_id = $2) AS is_me
     FROM membership m
     LEFT JOIN staff_member s ON s.membership_id = m.id AND s.business_id = m.business_id
     WHERE m.business_id = $1 AND m.status IN ('active', 'suspended')";

#[derive(Clone)]
pub struct TeamService {
    pool: PgPool,
}

impl TeamService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn add_member(
        &self,
        access: BusinessAccess,
        input: AddMemberInput,
    ) -> AppResult<MemberView> {
        access.require(&[Role::Owner, Role::Manager])?;
        let role = Role::parse(&input.role)
            .map_err(|_| AppError::validation("role must be manager, reception or employee"))?;
        if role == Role::Owner {
            return Err(AppError::validation(
                "role must be manager, reception or employee",
            ));
        }
        if role == Role::Manager && access.role != Role::Owner {
            return Err(AppError::authorization("Only the owner can add managers"));
        }
        let email = input.email.trim().to_lowercase();
        if email.len() > 254 || !email.contains('@') {
            return Err(AppError::validation("email is not valid"));
        }
        let name = clean_optional(input.display_name, 120, "display_name")?;

        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        let found: Option<(Uuid, Option<String>)> =
            sqlx::query_as("SELECT user_id, display_name FROM find_active_user_by_email($1)")
                .bind(&email)
                .fetch_optional(&mut *tx)
                .await?;
        let (user_id, account_name) = found.ok_or_else(|| {
            AppError::not_found("No account with this e-mail; ask the person to sign up first")
        })?;
        let display_name = name
            .or(account_name)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| email.split('@').next().unwrap_or("Staff").to_string());

        let membership_id: Uuid = sqlx::query_scalar(
            "INSERT INTO membership (business_id, user_id, role, invited_by)
             VALUES ($1, $2, $3, $4)
             RETURNING id",
        )
        .bind(access.business_id.as_uuid())
        .bind(user_id)
        .bind(role.as_str())
        .bind(access.user_id.as_uuid())
        .fetch_one(&mut *tx)
        .await
        .map_err(on_unique("This person is already a member"))?;
        let staff_id: Uuid = sqlx::query_scalar(
            "INSERT INTO staff_member (business_id, membership_id, display_name)
             VALUES ($1, $2, $3)
             RETURNING id",
        )
        .bind(access.business_id.as_uuid())
        .bind(membership_id)
        .bind(&display_name)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(MemberView {
            membership_id,
            staff_id,
            role,
            display_name,
        })
    }

    /// Everyone who works in the business, suspended people included.
    pub async fn list_members(&self, access: BusinessAccess) -> AppResult<Vec<MemberDetail>> {
        access.require(&[Role::Owner, Role::Manager])?;
        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        let rows = sqlx::query_as::<_, MemberDetail>(&format!(
            "{MEMBER_SELECT} ORDER BY (m.role = 'owner') DESC, m.created_at"
        ))
        .bind(access.business_id.as_uuid())
        .bind(access.user_id.as_uuid())
        .fetch_all(&mut *tx)
        .await?;
        Ok(rows)
    }

    /// Changes a member's role or suspends / restores their access.
    ///
    /// The owner can manage everyone but owners; a manager only employees and
    /// reception. Nobody changes their own membership, and the last active
    /// owner can never be changed or removed. Suspending also hides the
    /// person's card from booking; existing appointments are left alone.
    pub async fn update_member(
        &self,
        access: BusinessAccess,
        membership_id: Uuid,
        input: UpdateMemberInput,
    ) -> AppResult<MemberDetail> {
        access.require(&[Role::Owner, Role::Manager])?;
        if input.role.is_none() && input.status.is_none() {
            return Err(AppError::validation("Nothing to change"));
        }
        let new_role = match input.role.as_deref() {
            None => None,
            Some(value) => {
                let role = Role::parse(value).map_err(|_| {
                    AppError::validation("role must be manager, reception or employee")
                })?;
                if role == Role::Owner {
                    return Err(AppError::validation(
                        "role must be manager, reception or employee",
                    ));
                }
                Some(role)
            }
        };
        let new_status = match input.status.as_deref() {
            None => None,
            Some(value @ ("active" | "suspended")) => Some(value.to_string()),
            Some(_) => {
                return Err(AppError::validation("status must be active or suspended"));
            }
        };

        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        let target: Option<(Uuid, String, String)> = sqlx::query_as(
            "SELECT user_id, role, status FROM membership
             WHERE id = $1 AND business_id = $2 AND status IN ('active', 'suspended')
             FOR UPDATE",
        )
        .bind(membership_id)
        .bind(access.business_id.as_uuid())
        .fetch_optional(&mut *tx)
        .await?;
        let (target_user, target_role, target_status) =
            target.ok_or_else(|| AppError::not_found("Member not found"))?;
        let target_role = Role::parse(&target_role)?;

        if target_user == *access.user_id.as_uuid() {
            return Err(AppError::conflict(
                "You cannot change your own role or access",
            ));
        }
        if target_role == Role::Owner {
            let other_owners: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM membership
                 WHERE business_id = $1 AND role = 'owner' AND status = 'active' AND id <> $2",
            )
            .bind(access.business_id.as_uuid())
            .bind(membership_id)
            .fetch_one(&mut *tx)
            .await?;
            if other_owners == 0 {
                return Err(AppError::conflict(
                    "The last owner cannot be changed or removed",
                ));
            }
            if access.role != Role::Owner {
                return Err(AppError::authorization("Only an owner can change an owner"));
            }
        }
        if access.role == Role::Manager {
            if !matches!(target_role, Role::Reception | Role::Employee) {
                return Err(AppError::authorization(
                    "Managers manage only employees and reception",
                ));
            }
            if new_role == Some(Role::Manager) {
                return Err(AppError::authorization(
                    "Only the owner can make someone a manager",
                ));
            }
        }

        sqlx::query(
            "UPDATE membership SET role = COALESCE($3, role), status = COALESCE($4, status)
             WHERE id = $1 AND business_id = $2",
        )
        .bind(membership_id)
        .bind(access.business_id.as_uuid())
        .bind(new_role.map(Role::as_str))
        .bind(&new_status)
        .execute(&mut *tx)
        .await?;
        if let Some(status) = new_status
            .as_deref()
            .filter(|value| *value != target_status)
        {
            sqlx::query(
                "UPDATE staff_member SET is_bookable = $3
                 WHERE membership_id = $1 AND business_id = $2",
            )
            .bind(membership_id)
            .bind(access.business_id.as_uuid())
            .bind(status == "active")
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query(
            "INSERT INTO audit_log (business_id, actor_user_id, action, entity, entity_id)
             VALUES ($1, $2, 'member.update', 'membership', $3)",
        )
        .bind(access.business_id.as_uuid())
        .bind(access.user_id.as_uuid())
        .bind(membership_id)
        .execute(&mut *tx)
        .await?;
        let view = sqlx::query_as::<_, MemberDetail>(&format!("{MEMBER_SELECT} AND m.id = $3"))
            .bind(access.business_id.as_uuid())
            .bind(access.user_id.as_uuid())
            .bind(membership_id)
            .fetch_one(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(view)
    }
}
