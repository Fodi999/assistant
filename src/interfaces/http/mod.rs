pub mod admin_auth;
pub mod admin_catalog;
pub mod admin_cms;
pub mod admin_intent_pages; // 🆕 Intent pages admin handlers
pub mod admin_nutrition;
pub mod admin_states; // Ingredient processing states (AI Sous Chef)
pub mod admin_users;
pub mod assistant;
pub mod auth;
pub mod billing; // 🆕 Stripe Checkout + Webhook
pub mod catalog;
pub mod chef_reference_public;
pub mod city; // 🆕 City Engine — GET /api/city/map
pub mod cook_suggestions; // 🆕 Smart recipe suggestions from inventory
pub mod copilot; // 🆕 Copilot — главный LLM Brain (POST /api/copilot/message)
pub mod dish;
pub mod error;
pub mod health;
pub mod inventory;
pub mod laboratory; // 🆕 Food-tech Laboratory HTTP handlers
pub mod laboratory_v2; // 🆕 Laboratory v2 — Photo → 3D Model HTTP handlers
pub mod menu_engineering;
pub mod middleware;
pub mod preferences;
pub mod public;
pub mod recipe;
pub mod recipe_ai_insights; // AI insights for recipes
pub mod recipe_v2; // V2 with translations
pub mod report;
pub mod routes;
pub mod scenes; // 🆕 Game-like SceneState endpoints (GET /api/scenes/inventory)
pub mod smart; // 🆕 SmartService — POST /api/smart/ingredient
pub mod smart_parse; // 🆕 SmartParse — POST /api/smart/parse
pub mod tenant_ingredient;
pub mod usage; // ChefOS iOS usage endpoints
pub mod user; // ChefOS user preferences endpoints
