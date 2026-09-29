//! Per-request database scope for row-level security.
//!
//! Every request that touches tenant data runs inside a transaction whose
//! `app.user_id` / `app.business_id` settings tell PostgreSQL who is asking and
//! for which business (see the `identity_and_tenancy` migration). The settings
//! are transaction-local, so they can never leak to another request that reuses
//! the same pooled connection. With no scope set, RLS returns no rows.

use crate::shared::{AppResult, BusinessId, UserId};
use sqlx::{PgConnection, PgPool, Postgres, Transaction};

/// Who is asking, and for which business.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DbScope {
    pub user_id: Option<UserId>,
    pub business_id: Option<BusinessId>,
}

impl DbScope {
    /// No user, no business: RLS hides everything.
    pub fn anonymous() -> Self {
        Self::default()
    }

    /// A signed-in user outside any business (e.g. listing "my businesses").
    pub fn user(user_id: UserId) -> Self {
        Self {
            user_id: Some(user_id),
            business_id: None,
        }
    }

    /// A signed-in user working inside one business. The caller must have
    /// verified the membership before building this scope.
    pub fn business(user_id: UserId, business_id: BusinessId) -> Self {
        Self {
            user_id: Some(user_id),
            business_id: Some(business_id),
        }
    }
}

/// Sets the transaction-local RLS context on an open transaction.
pub async fn apply_scope(conn: &mut PgConnection, scope: DbScope) -> AppResult<()> {
    sqlx::query(
        "SELECT set_config('app.user_id', $1, true), set_config('app.business_id', $2, true)",
    )
    .bind(scope.user_id.map(|id| id.to_string()).unwrap_or_default())
    .bind(
        scope
            .business_id
            .map(|id| id.to_string())
            .unwrap_or_default(),
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// Starts a transaction with the given scope already applied.
pub async fn begin_scoped(
    pool: &PgPool,
    scope: DbScope,
) -> AppResult<Transaction<'static, Postgres>> {
    let mut tx = pool.begin().await?;
    apply_scope(&mut tx, scope).await?;
    Ok(tx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_constructors() {
        let user = UserId::new();
        let business = BusinessId::new();

        assert_eq!(DbScope::anonymous(), DbScope::default());
        assert_eq!(DbScope::user(user).business_id, None);

        let scoped = DbScope::business(user, business);
        assert_eq!(scoped.user_id, Some(user));
        assert_eq!(scoped.business_id, Some(business));
    }
}
