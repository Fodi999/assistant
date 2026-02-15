// Recipe domain model V2 - with translation support
// Clean architecture: simple structs that match database schema

use crate::shared::{AppError, AppResult, Language, TenantId, UserId};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

// ========== ID Types ==========

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RecipeId(pub Uuid);

impl RecipeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl From<Uuid> for RecipeId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RecipeIngredientId(pub Uuid);

impl RecipeIngredientId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

// ========== Enums ==========

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecipeStatus {
    Draft,
    Published,
    Archived,
}

impl RecipeStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            RecipeStatus::Draft => "draft",
            RecipeStatus::Published => "published",
            RecipeStatus::Archived => "archived",
        }
    }

    pub fn from_str(s: &str) -> AppResult<Self> {
        match s {
            "draft" => Ok(RecipeStatus::Draft),
            "published" => Ok(RecipeStatus::Published),
            "archived" => Ok(RecipeStatus::Archived),
            _ => Err(AppError::validation("Invalid recipe status")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TranslationSource {
    AI,
    Human,
}

impl TranslationSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            TranslationSource::AI => "ai",
            TranslationSource::Human => "human",
        }
    }

    pub fn from_str(s: &str) -> AppResult<Self> {
        match s {
            "ai" => Ok(TranslationSource::AI),
            "human" => Ok(TranslationSource::Human),
            _ => Err(AppError::validation("Invalid translation source")),
        }
    }
}

// ========== Main Entities ==========

/// Recipe with translation support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub id: RecipeId,
    pub user_id: UserId,
    pub tenant_id: TenantId,
    
    // Default language content
    pub name_default: String,
    pub instructions_default: String,
    pub language_default: Language,
    
    // Recipe details
    pub servings: i32,
    pub total_cost_cents: Option<i32>,
    pub cost_per_serving_cents: Option<i32>,
    
    // Publishing
    pub status: RecipeStatus,
    pub is_public: bool,
    pub published_at: Option<OffsetDateTime>,
    
    // Timestamps
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// Recipe ingredient (links recipe to catalog ingredient)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeIngredient {
    pub id: RecipeIngredientId,
    pub recipe_id: RecipeId,
    pub catalog_ingredient_id: Uuid,
    pub quantity: Decimal,
    pub unit: String,
    
    // Cost snapshot (captures cost at time of recipe creation)
    pub cost_at_use_cents: Option<i32>,
    
    // Name snapshot (for historical reference if ingredient deleted)
    pub catalog_ingredient_name_snapshot: Option<String>,
    
    pub created_at: OffsetDateTime,
}

/// Recipe translation (AI or human translated content)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeTranslation {
    pub id: Uuid,
    pub recipe_id: RecipeId,
    pub language: Language,
    pub name: String,
    pub instructions: String,
    pub translated_by: TranslationSource,
    pub created_at: OffsetDateTime,
}

impl RecipeTranslation {
    pub fn new(
        recipe_id: RecipeId,
        language: Language,
        name: String,
        instructions: String,
        translated_by: TranslationSource,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            recipe_id,
            language,
            name,
            instructions,
            translated_by,
            created_at: OffsetDateTime::now_utc(),
        }
    }
}
