pub mod admin;
pub mod ai_ports;  // 🆕 AI abstraction traits (AiClient, AiQuality)
pub mod assistant;
pub mod auth;
pub mod catalog;
pub mod classification_rules; // 🆕 Added classification rules
pub mod dish;
pub mod engines; // 🆕 Culinary Intelligence Platform — 5 engine traits + registry
pub mod inventory;
pub mod menu_engineering;
pub mod processing_state; // 🆕 Product states (raw, boiled, fried, etc.)
pub mod recipe;
pub mod recipe_ai_insights; // AI-generated insights
pub mod recipe_v2; // V2 with translation support
pub mod report;
pub mod tenant;
pub mod tenant_ingredient;
pub mod tools; // 🆕 Chef tools domain (unit converter, yield, scale)
pub mod usage; // ChefOS iOS usage tracking + monetization
pub mod user;
pub mod user_preferences; // ChefOS user health/diet/lifestyle preferences

pub use admin::*;
pub use ai_ports::*;
pub use assistant::*;
pub use auth::*;
pub use catalog::*;
pub use dish::*;
pub use inventory::*;
pub use menu_engineering::*;
pub use recipe::*;
pub use recipe_ai_insights::*;
pub use processing_state::*;
pub use tenant::*;
pub use tenant_ingredient::*;
pub use user::*;
