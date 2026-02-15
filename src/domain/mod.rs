pub mod admin;
pub mod assistant;
pub mod auth;
pub mod catalog;
pub mod dish;
pub mod inventory;
pub mod menu_engineering;
pub mod recipe;
pub mod recipe_v2;  // V2 with translation support
pub mod recipe_ai_insights;  // AI-generated insights
pub mod tenant;
pub mod tenant_ingredient;
pub mod user;

pub use admin::*;
pub use assistant::*;
pub use auth::*;
pub use catalog::*;
pub use tenant_ingredient::*;
pub use dish::*;
pub use inventory::*;
pub use menu_engineering::*;
pub use recipe::*;
pub use recipe_ai_insights::*;
pub use tenant::*;
pub use user::*;
