use crate::shared::{AppError, AppResult, Language};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Catalog category ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CatalogCategoryId(Uuid);

impl CatalogCategoryId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for CatalogCategoryId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for CatalogCategoryId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Catalog category - for organizing ingredients
#[derive(Debug, Clone)]
pub struct CatalogCategory {
    pub id: CatalogCategoryId,
    
    // Multilingual names
    pub name_pl: String,
    pub name_en: String,
    pub name_uk: String,
    pub name_ru: String,
    
    // Display order
    pub sort_order: i32,
}

impl CatalogCategory {
    /// Get the name in the user's language
    pub fn name(&self, language: Language) -> &str {
        match language {
            Language::Pl => &self.name_pl,
            Language::En => &self.name_en,
            Language::Uk => &self.name_uk,
            Language::Ru => &self.name_ru,
        }
    }

    /// Create a new catalog category
    pub fn new(
        name_pl: String,
        name_en: String,
        name_uk: String,
        name_ru: String,
        sort_order: i32,
    ) -> Self {
        Self {
            id: CatalogCategoryId::new(),
            name_pl,
            name_en,
            name_uk,
            name_ru,
            sort_order,
        }
    }

    /// Reconstruct from database parts
    pub fn from_parts(
        id: CatalogCategoryId,
        name_pl: String,
        name_en: String,
        name_uk: String,
        name_ru: String,
        sort_order: i32,
    ) -> Self {
        Self {
            id,
            name_pl,
            name_en,
            name_uk,
            name_ru,
            sort_order,
        }
    }
}

/// Catalog ingredient ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CatalogIngredientId(Uuid);

impl CatalogIngredientId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for CatalogIngredientId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for CatalogIngredientId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unit of measurement
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Unit {
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

impl Unit {
    pub fn from_str(s: &str) -> AppResult<Self> {
        match s.to_lowercase().as_str() {
            "gram" | "g" => Ok(Self::Gram),
            "kilogram" | "kg" => Ok(Self::Kilogram),
            "liter" | "l" => Ok(Self::Liter),
            "milliliter" | "ml" => Ok(Self::Milliliter),
            "piece" | "pcs" => Ok(Self::Piece),
            "bunch" => Ok(Self::Bunch),
            "can" => Ok(Self::Can),
            "bottle" => Ok(Self::Bottle),
            "package" | "pkg" => Ok(Self::Package),
            _ => Err(AppError::validation(format!("Invalid unit: {}", s))),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Gram => "gram",
            Self::Kilogram => "kilogram",
            Self::Liter => "liter",
            Self::Milliliter => "milliliter",
            Self::Piece => "piece",
            Self::Bunch => "bunch",
            Self::Can => "can",
            Self::Bottle => "bottle",
            Self::Package => "package",
        }
    }
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Allergen types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Allergen {
    Milk,
    Eggs,
    Fish,
    Shellfish,
    TreeNuts,
    Peanuts,
    Wheat,
    Soybeans,
    Sesame,
    Celery,
    Mustard,
    Sulfites,
    Lupin,
    Molluscs,
}

impl Allergen {
    pub fn from_str(s: &str) -> AppResult<Self> {
        match s.to_lowercase().as_str() {
            "milk" => Ok(Self::Milk),
            "eggs" => Ok(Self::Eggs),
            "fish" => Ok(Self::Fish),
            "shellfish" => Ok(Self::Shellfish),
            "treenuts" => Ok(Self::TreeNuts),
            "peanuts" => Ok(Self::Peanuts),
            "wheat" => Ok(Self::Wheat),
            "soybeans" => Ok(Self::Soybeans),
            "sesame" => Ok(Self::Sesame),
            "celery" => Ok(Self::Celery),
            "mustard" => Ok(Self::Mustard),
            "sulfites" => Ok(Self::Sulfites),
            "lupin" => Ok(Self::Lupin),
            "molluscs" => Ok(Self::Molluscs),
            _ => Err(AppError::validation(format!("Invalid allergen: {}", s))),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Milk => "Milk",
            Self::Eggs => "Eggs",
            Self::Fish => "Fish",
            Self::Shellfish => "Shellfish",
            Self::TreeNuts => "TreeNuts",
            Self::Peanuts => "Peanuts",
            Self::Wheat => "Wheat",
            Self::Soybeans => "Soybeans",
            Self::Sesame => "Sesame",
            Self::Celery => "Celery",
            Self::Mustard => "Mustard",
            Self::Sulfites => "Sulfites",
            Self::Lupin => "Lupin",
            Self::Molluscs => "Molluscs",
        }
    }
}

/// Season availability
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Season {
    Spring,
    Summer,
    Autumn,
    Winter,
    AllYear,
}

impl Season {
    pub fn from_str(s: &str) -> AppResult<Self> {
        match s.to_lowercase().as_str() {
            "spring" => Ok(Self::Spring),
            "summer" => Ok(Self::Summer),
            "autumn" | "fall" => Ok(Self::Autumn),
            "winter" => Ok(Self::Winter),
            "allyear" | "all_year" => Ok(Self::AllYear),
            _ => Err(AppError::validation(format!("Invalid season: {}", s))),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Spring => "Spring",
            Self::Summer => "Summer",
            Self::Autumn => "Autumn",
            Self::Winter => "Winter",
            Self::AllYear => "AllYear",
        }
    }
}

/// Catalog ingredient - the master product catalog
#[derive(Debug, Clone)]
pub struct CatalogIngredient {
    pub id: CatalogIngredientId,
    
    // Category reference
    pub category_id: CatalogCategoryId,
    
    // Multilingual names
    pub name_pl: String,
    pub name_en: String,
    pub name_uk: String,
    pub name_ru: String,
    
    // Core properties
    pub default_unit: Unit,
    pub default_shelf_life_days: Option<i32>,
    
    // Nutritional and metadata
    pub allergens: Vec<Allergen>,
    pub calories_per_100g: Option<i32>,
    pub seasons: Vec<Season>,
    
    // UX
    pub image_url: Option<String>,
}

impl CatalogIngredient {
    /// Get the name in the user's language
    pub fn name(&self, language: Language) -> &str {
        match language {
            Language::Pl => &self.name_pl,
            Language::En => &self.name_en,
            Language::Uk => &self.name_uk,
            Language::Ru => &self.name_ru,
        }
    }

    /// Create a new catalog ingredient
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        category_id: CatalogCategoryId,
        name_pl: String,
        name_en: String,
        name_uk: String,
        name_ru: String,
        default_unit: Unit,
        default_shelf_life_days: Option<i32>,
        allergens: Vec<Allergen>,
        calories_per_100g: Option<i32>,
        seasons: Vec<Season>,
        image_url: Option<String>,
    ) -> Self {
        Self {
            id: CatalogIngredientId::new(),
            category_id,
            name_pl,
            name_en,
            name_uk,
            name_ru,
            default_unit,
            default_shelf_life_days,
            allergens,
            calories_per_100g,
            seasons,
            image_url,
        }
    }

    /// Reconstruct from database parts
    #[allow(clippy::too_many_arguments)]
    pub fn from_parts(
        id: CatalogIngredientId,
        category_id: CatalogCategoryId,
        name_pl: String,
        name_en: String,
        name_uk: String,
        name_ru: String,
        default_unit: Unit,
        default_shelf_life_days: Option<i32>,
        allergens: Vec<Allergen>,
        calories_per_100g: Option<i32>,
        seasons: Vec<Season>,
        image_url: Option<String>,
    ) -> Self {
        Self {
            id,
            category_id,
            name_pl,
            name_en,
            name_uk,
            name_ru,
            default_unit,
            default_shelf_life_days,
            allergens,
            calories_per_100g,
            seasons,
            image_url,
        }
    }
}
