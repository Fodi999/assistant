use crate::shared::{AppError, AppResult, Language, TenantId, UserId};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub tenant_id: TenantId,
    pub email: Email,
    pub password_hash: String,
    pub display_name: Option<DisplayName>,
    pub avatar_url: Option<String>,
    pub role: UserRole,
    pub language: Language,
    pub created_at: OffsetDateTime,
}

impl User {
    pub fn new(
        tenant_id: TenantId,
        email: Email,
        password_hash: String,
        display_name: Option<DisplayName>,
        role: UserRole,
        language: Language,
    ) -> Self {
        Self {
            id: UserId::new(),
            tenant_id,
            email,
            password_hash,
            display_name,
            avatar_url: None,
            role,
            language,
            created_at: OffsetDateTime::now_utc(),
        }
    }

    pub fn from_parts(
        id: UserId,
        tenant_id: TenantId,
        email: Email,
        password_hash: String,
        display_name: Option<DisplayName>,
        avatar_url: Option<String>,
        role: UserRole,
        language: Language,
        created_at: OffsetDateTime,
    ) -> Self {
        Self {
            id,
            tenant_id,
            email,
            password_hash,
            display_name,
            avatar_url,
            role,
            language,
            created_at,
        }
    }

    pub fn is_owner(&self) -> bool {
        matches!(self.role, UserRole::Owner)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    Owner,
    Manager,
    Staff,
}

impl UserRole {
    pub fn as_str(&self) -> &str {
        match self {
            UserRole::Owner => "owner",
            UserRole::Manager => "manager",
            UserRole::Staff => "staff",
        }
    }

    pub fn from_str(s: &str) -> AppResult<Self> {
        match s.to_lowercase().as_str() {
            "owner" => Ok(UserRole::Owner),
            "manager" => Ok(UserRole::Manager),
            "staff" => Ok(UserRole::Staff),
            _ => Err(AppError::validation(format!("Invalid user role: {}", s))),
        }
    }
}

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Email(String);

impl Email {
    pub fn new(email: String) -> AppResult<Self> {
        let normalized = email.trim().to_lowercase();
        
        if normalized.is_empty() {
            return Err(AppError::validation("Email cannot be empty"));
        }
        
        // Simple email validation
        if !normalized.contains('@') || !normalized.contains('.') {
            return Err(AppError::validation("Invalid email format"));
        }
        
        Ok(Self(normalized))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl AsRef<str> for Email {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Email {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayName(String);

impl DisplayName {
    pub fn new(name: String) -> AppResult<Self> {
        let trimmed = name.trim().to_string();
        
        if trimmed.is_empty() {
            return Err(AppError::validation("Display name cannot be empty"));
        }
        
        if trimmed.len() > 255 {
            return Err(AppError::validation("Display name cannot exceed 255 characters"));
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

impl AsRef<str> for DisplayName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for DisplayName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct Password(String);

impl Password {
    pub fn new(password: String) -> AppResult<Self> {
        if password.len() < 8 {
            return Err(AppError::validation("Password must be at least 8 characters long"));
        }
        
        if password.len() > 128 {
            return Err(AppError::validation("Password cannot exceed 128 characters"));
        }
        
        // Check for at least one letter and one number
        let has_letter = password.chars().any(|c| c.is_alphabetic());
        let has_digit = password.chars().any(|c| c.is_numeric());
        
        if !has_letter || !has_digit {
            return Err(AppError::validation(
                "Password must contain at least one letter and one number"
            ));
        }
        
        Ok(Self(password))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}
