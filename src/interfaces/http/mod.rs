pub mod admin_auth;
pub mod admin_catalog;
pub mod admin_cms;
pub mod admin_intent_pages; // 🆕 Intent pages admin handlers
pub mod admin_lab_combos;   // 🆕 Lab combo SEO pages admin handlers
pub mod admin_nutrition;
pub mod admin_states; // Ingredient processing states (AI Sous Chef)
pub mod admin_users;
pub mod assistant;
pub mod auth;
pub mod catalog;
pub mod chef_reference_public;
pub mod dish;
pub mod public;
pub mod error;
pub mod health;
pub mod inventory;
pub mod menu_engineering;
pub mod middleware;
pub mod recipe;
pub mod recipe_ai_insights; // AI insights for recipes
pub mod recipe_v2; // V2 with translations
pub mod report;
pub mod routes;
pub mod smart; // 🆕 SmartService — POST /api/smart/ingredient
pub mod smart_parse; // 🆕 SmartParse — POST /api/smart/parse
pub mod tenant_ingredient;
pub mod user;
