use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::fmt;

/// Processing state of an ingredient (raw, boiled, fried, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[sqlx(type_name = "processing_state", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ProcessingState {
    Raw,
    Boiled,
    Fried,
    Baked,
    Grilled,
    Steamed,
    Smoked,
    Frozen,
    Dried,
    Pickled,
}

impl ProcessingState {
    /// All possible states
    pub const ALL: &'static [ProcessingState] = &[
        Self::Raw,
        Self::Boiled,
        Self::Fried,
        Self::Baked,
        Self::Grilled,
        Self::Steamed,
        Self::Smoked,
        Self::Frozen,
        Self::Dried,
        Self::Pickled,
    ];

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "raw" => Some(Self::Raw),
            "boiled" => Some(Self::Boiled),
            "fried" => Some(Self::Fried),
            "baked" => Some(Self::Baked),
            "grilled" => Some(Self::Grilled),
            "steamed" => Some(Self::Steamed),
            "smoked" => Some(Self::Smoked),
            "frozen" => Some(Self::Frozen),
            "dried" => Some(Self::Dried),
            "pickled" => Some(Self::Pickled),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Raw => "raw",
            Self::Boiled => "boiled",
            Self::Fried => "fried",
            Self::Baked => "baked",
            Self::Grilled => "grilled",
            Self::Steamed => "steamed",
            Self::Smoked => "smoked",
            Self::Frozen => "frozen",
            Self::Dried => "dried",
            Self::Pickled => "pickled",
        }
    }

    /// English display name
    pub fn label_en(&self) -> &'static str {
        match self {
            Self::Raw => "Raw",
            Self::Boiled => "Boiled",
            Self::Fried => "Fried",
            Self::Baked => "Baked",
            Self::Grilled => "Grilled",
            Self::Steamed => "Steamed",
            Self::Smoked => "Smoked",
            Self::Frozen => "Frozen",
            Self::Dried => "Dried",
            Self::Pickled => "Pickled",
        }
    }
}

impl fmt::Display for ProcessingState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
