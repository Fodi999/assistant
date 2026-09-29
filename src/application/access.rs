//! Roles inside a business and the scope a request may use.

use crate::infrastructure::DbScope;
use crate::shared::{AppError, AppResult, BusinessId, UserId};
use serde::Serialize;

/// A member's role inside one business (PRODUCT_SPEC §17). Customers are not
/// members; they are clients of a business.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Owner,
    Manager,
    Reception,
    Employee,
}

impl Role {
    pub fn parse(value: &str) -> AppResult<Self> {
        match value {
            "owner" => Ok(Self::Owner),
            "manager" => Ok(Self::Manager),
            "reception" => Ok(Self::Reception),
            "employee" => Ok(Self::Employee),
            other => Err(AppError::internal(format!(
                "Unknown role in database: {other}"
            ))),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Manager => "manager",
            Self::Reception => "reception",
            Self::Employee => "employee",
        }
    }

    /// Owners and managers may change business settings.
    pub fn can_manage_business(self) -> bool {
        matches!(self, Self::Owner | Self::Manager)
    }
}

/// Proof that a user is an active member of a business, with their role. The
/// only way to obtain a business-scoped [`DbScope`] in handlers.
#[derive(Debug, Clone, Copy)]
pub struct BusinessAccess {
    pub user_id: UserId,
    pub business_id: BusinessId,
    pub role: Role,
}

impl BusinessAccess {
    /// A signed-in customer acting inside a business they are NOT a member of.
    /// It carries the least-privileged member role only so that the shared
    /// booking code can run; the public service must have checked that the
    /// business is public and that every appointment touched is the caller's
    /// own (`BookingService::ensure_owned`) before using it.
    pub(crate) fn for_customer(user_id: UserId, business_id: BusinessId) -> Self {
        Self {
            user_id,
            business_id,
            role: Role::Reception,
        }
    }

    /// No signed-in user: only for computing public availability.
    pub(crate) fn for_public(business_id: BusinessId) -> Self {
        Self {
            user_id: UserId::from_uuid(uuid::Uuid::nil()),
            business_id,
            role: Role::Reception,
        }
    }

    pub fn scope(&self) -> DbScope {
        DbScope::business(self.user_id, self.business_id)
    }

    /// Fails with 403 unless the member's role is one of `allowed`.
    pub fn require(&self, allowed: &[Role]) -> AppResult<()> {
        if allowed.contains(&self.role) {
            Ok(())
        } else {
            Err(AppError::authorization(format!(
                "Role '{}' is not allowed to do this",
                self.role.as_str()
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_roundtrip_and_reject_unknown() {
        for role in [Role::Owner, Role::Manager, Role::Reception, Role::Employee] {
            assert_eq!(Role::parse(role.as_str()).unwrap(), role);
        }
        assert!(Role::parse("root").is_err());
    }

    #[test]
    fn only_owner_and_manager_manage_the_business() {
        assert!(Role::Owner.can_manage_business());
        assert!(Role::Manager.can_manage_business());
        assert!(!Role::Reception.can_manage_business());
        assert!(!Role::Employee.can_manage_business());
    }

    #[test]
    fn require_checks_the_allow_list() {
        let access = BusinessAccess {
            user_id: UserId::new(),
            business_id: BusinessId::new(),
            role: Role::Employee,
        };
        assert!(access.require(&[Role::Employee]).is_ok());
        assert!(matches!(
            access.require(&[Role::Owner, Role::Manager]),
            Err(AppError::Authorization(_))
        ));
        assert_eq!(access.scope().business_id, Some(access.business_id));
    }
}
