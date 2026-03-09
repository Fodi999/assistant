//! Public tools — split by domain concern.
//!
//! Each sub-module contains handlers for a specific area:
//! - shared:      language parsing, unit labels, guard helpers
//! - units:       convert_units, list_units, ingredient_scale
//! - fish:        fish_season, fish_season_table
//! - nutrition:   nutrition, ingredients_db, compare_foods
//! - kitchen:     scale_recipe, yield_calc, ingredient_equivalents,
//!                food_cost_calc, ingredient_suggestions, popular_conversions
//! - categories:  list_categories, measure_conversion, ingredient_measures
//! - seasonality: seasonal_calendar, in_season_now, product_seasonality,
//!                best_in_season, products_by_month, product_search,
//!                recipe_nutrition, recipe_cost, best_right_now, list_regions

pub mod shared;
pub mod units;
pub mod fish;
pub mod nutrition;
pub mod kitchen;
pub mod categories;
pub mod seasonality;

// ── Re-exports for routes.rs ──────────────────────────────────────────────────

pub use categories::{ingredient_measures, list_categories, measure_conversion};
pub use fish::{fish_season, fish_season_table};
pub use kitchen::{
    food_cost_calc, ingredient_equivalents, ingredient_suggestions, popular_conversions,
    scale_recipe, yield_calc,
};
pub use nutrition::{compare_foods, ingredients_db, nutrition};
pub use seasonality::{
    best_in_season, best_right_now, in_season_now, list_regions, product_search,
    product_seasonality, products_by_month, recipe_cost, recipe_nutrition, seasonal_calendar,
};
pub use units::{convert_units, ingredient_convert, seo_ingredient_convert, ingredient_scale, list_units};
