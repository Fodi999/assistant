pub mod admin_auth;
pub mod admin_catalog;
pub mod admin_nutrition;
pub mod ai_sous_chef; // AI Sous Chef — deterministic state generation
pub mod analytics;
pub mod assistant_service;
pub mod auth;
pub mod catalog;
pub mod chat_events_service; // 🆕 Chat telemetry (Step 4)
pub mod cms_service;
pub mod cook_suggestions; // 🆕 Smart recipe suggestions from inventory
pub mod copilot;
pub mod dish;
pub mod intent_pages; // 🆕 Intent pages CRUD + batch + publish pipeline
pub mod inventory;
pub mod inventory_alert;
pub mod laboratory; // 🆕 Food-tech Laboratory — analysis projects on top of catalog
pub mod menu_engineering;
pub mod preferences_service; // ChefOS user preferences
pub mod public_nutrition;
pub mod public_seo_content; // 🆕 AI SEO content for programmatic pages
pub mod purchase_draft; // 🆕 Purchase drafts (Copilot)
pub mod recipe;
pub mod recipe_ai_insights_service; // AI insights service
pub mod recipe_translation_service; // V2 translation service
pub mod recipe_v2_service; // V2 recipe service
pub mod recipe_validator; // Rule-based validator
pub mod report;
pub mod rulebot; // 🆕 RuleBot orchestrator — Culinary Intelligence Platform
pub mod smart_parse; // 🆕 SmartParse — deterministic text → ingredient parser
pub mod smart_service; // 🆕 SmartService — intelligent ingredient aggregator
pub mod sous_chef; // 🆕 Sous Chef — AI meal planner
pub mod tenant_ingredient;
pub mod usage_service; // ChefOS iOS usage tracking
pub mod user; // 🆕 Copilot — главный LLM Brain над всеми ботами

pub use admin_auth::*;
pub use admin_catalog::*;
pub use admin_nutrition::*;
pub use analytics::*;
pub use assistant_service::*;
pub use auth::*;
pub use catalog::*;
pub use dish::*;
pub use inventory::*;
pub use inventory_alert::*;
pub use menu_engineering::*;
pub use recipe::*;
pub use recipe_ai_insights_service::*;
pub use recipe_validator::*;
pub use tenant_ingredient::*;
pub use user::*;
