use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TenantId(pub Uuid);

impl TenantId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for TenantId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TenantId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for TenantId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UserId(pub Uuid);

impl UserId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for UserId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for UserId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RefreshTokenId(pub Uuid);

impl RefreshTokenId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for RefreshTokenId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for RefreshTokenId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for RefreshTokenId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

/// Unit Type - maps to PostgreSQL ENUM unit_type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "unit_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum UnitType {
    Gram,
    Kilogram,
    Liter,
    Milliliter,
    Piece,
    Bunch,
    Can,
    Bottle,
    Package,
}

impl fmt::Display for UnitType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            UnitType::Gram => "gram",
            UnitType::Kilogram => "kilogram",
            UnitType::Liter => "liter",
            UnitType::Milliliter => "milliliter",
            UnitType::Piece => "piece",
            UnitType::Bunch => "bunch",
            UnitType::Can => "can",
            UnitType::Bottle => "bottle",
            UnitType::Package => "package",
        };
        write!(f, "{}", s)
    }
}

impl UnitType {
    /// Конвертировать строку в UnitType
    /// Используется для AI классификации
    pub fn from_string(s: &str) -> Result<Self, crate::shared::AppError> {
        match s.trim().to_lowercase().as_str() {
            "gram" => Ok(UnitType::Gram),
            "kilogram" | "kg" => Ok(UnitType::Kilogram),
            "liter" | "litre" => Ok(UnitType::Liter),
            "milliliter" | "ml" => Ok(UnitType::Milliliter),
            "piece" | "штука" => Ok(UnitType::Piece),
            "bunch" | "пучок" => Ok(UnitType::Bunch),
            "can" | "банка" => Ok(UnitType::Can),
            "bottle" | "бутылка" => Ok(UnitType::Bottle),
            "package" | "упаковка" => Ok(UnitType::Package),
            _ => Err(crate::shared::AppError::validation(
                &format!("Unknown unit type: {}", s)
            )),
        }
    }
}
