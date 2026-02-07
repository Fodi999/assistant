use crate::shared::{AppError, AppResult, TenantId};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: TenantId,
    pub name: TenantName,
    pub created_at: OffsetDateTime,
}

impl Tenant {
    pub fn new(name: TenantName) -> Self {
        Self {
            id: TenantId::new(),
            name,
            created_at: OffsetDateTime::now_utc(),
        }
    }

    pub fn from_parts(id: TenantId, name: TenantName, created_at: OffsetDateTime) -> Self {
        Self {
            id,
            name,
            created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantName(String);

impl TenantName {
    pub fn new(name: String) -> AppResult<Self> {
        let trimmed = name.trim().to_string();
        
        if trimmed.is_empty() {
            return Err(AppError::validation("Tenant name cannot be empty"));
        }
        
        if trimmed.len() > 255 {
            return Err(AppError::validation("Tenant name cannot exceed 255 characters"));
        }
        
        Ok(Self(trimmed))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl AsRef<str> for TenantName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for TenantName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
