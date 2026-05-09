//! Culinary Intelligence Platform — Engine layer
//!
//! 5 domain engines that encapsulate all business logic:
//! - ConversionEngine: unit conversion, density-aware cross-group, scaling
//! - NutritionEngine:  nutrition lookup, comparison, scoring
//! - SeasonalityEngine: seasonal calendar, regions, product seasonality
//! - RecipeEngine:     recipe analysis, rule diagnostics, sharing
//! - KitchenEngine:    yield, cost, equivalents, suggestions
//!
//! Each engine is a trait that defines pure operations.
//! Implementations live in the application layer (with DB access).

pub mod conversion;
pub mod kitchen;
pub mod nutrition;
pub mod recipe;
pub mod registry;
pub mod response;
pub mod seasonality;
pub mod types;
