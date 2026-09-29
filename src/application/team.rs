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
}
