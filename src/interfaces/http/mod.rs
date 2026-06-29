pub mod admin_ai;
pub mod admin_analytics;
pub mod admin_auth;
pub mod admin_catalog;
pub mod admin_cms;
pub mod admin_intent_pages; // 🆕 Intent pages admin handlers
pub mod admin_nutrition;
pub mod admin_panel;
pub mod admin_search_console;
pub mod admin_states; // Ingredient processing states (AI Sous Chef)
pub mod admin_users;
pub mod admin_version;
pub mod almabuild;
pub mod assistant;
pub mod auth;
pub mod billing; // 🆕 Stripe Checkout + Webhook
pub mod catalog;
pub mod chef_reference_public;
pub mod cook_suggestions; // 🆕 Smart recipe suggestions from inventory
pub mod copilot; // 🆕 Copilot — главный LLM Brain (POST /api/copilot/message)
pub mod dish;
pub mod error;
pub mod health;
pub mod icons_site;
pub mod inventory;
pub mod laboratory; // 🆕 Food-tech Laboratory HTTP handlers
pub mod menu_engineering;
pub mod middleware;
pub mod preferences;
pub mod public;
pub mod recipe;
pub mod recipe_ai_insights; // AI insights for recipes
pub mod recipe_v2; // V2 with translations
pub mod report;
pub mod routes;
pub mod site_context;
pub mod smart; // 🆕 SmartService — POST /api/smart/ingredient
pub mod smart_parse; // 🆕 SmartParse — POST /api/smart/parse
pub mod tenant_ingredient;
pub mod usage; // ChefOS iOS usage endpoints
pub mod user; // ChefOS user preferences endpoints
