//! RuleBot — Culinary Intelligence Platform.
//!
//! DDD Architecture:
//!   - `intent_keywords`    → keyword data tables (WHAT words mean)
//!   - `goal_modifier`      → health modifier detection (WHICH goal)
//!   - `intent_router`      → intent scoring + routing (the BRAIN)
//!   - `chat_engine`        → routes `POST /public /chat` to handlers
//!   - `chat_response`      → ChatResponse / Card / Suggestion types
//!   - `response_builder`   → card assembly + suggestion generation
//!   - `response_templates` → localized human-readable text
//!   - `session_context`    → multi-turn session state
//!   - `chef_coach`         → motivational sous-chef messages
//!   - `meal_builder`       → dynamic meal combo assembler
//!   - `cooking_rules`      → DDD dish rules as data
//!   - `food_pairing`       → ingredient compatibility filter
//!   - `recipe_engine`      → recipe resolution engine (orchestrator + types)
//!   - `dish_schema`        → Gemini LLM call + JSON parsing
//!   - `ingredient_resolver`→ slug resolution + implicit ingredients
//!   - `nutrition_math`     → portions, yields, КБЖУ, allergens, diet tags
//!   - `display_name`       → multilingual grammar + display names
//!   - `user_constraints`   → dietary constraint parsing from user text
//!   - `constraint_policy`  → enforce constraints on resolved ingredients
//!   - `goal_engine`        → nutritional target profiles (GoalProfile + GoalStrategy)
//!   - `adaptation_engine`  → smart rebalancing after constraint removal
//!   - `recipe_validation`  → post-build recipe coherence checks
//!   - `auto_fix`           → automatic repair of validation issues
//!   - `ai_brain`           → Layer 2 LLM fallback with tool calling

pub mod adaptation_engine;
pub mod ai_brain;
pub mod auto_fix;
pub mod category_filter;
pub mod chat_engine;
pub mod chat_response;
pub mod chef_coach;
pub mod constraint_policy;
pub mod cooking_rules;
pub mod culinary_base_layer;
pub mod dish_schema;
pub mod display_name;
pub mod flavor_engine;
pub mod food_pairing;
pub mod goal_engine;
pub mod goal_modifier;
pub mod ingredient_resolver;
pub mod intent_keywords;
pub mod intent_router;
pub mod meal_builder;
pub mod nutrition_math;
pub mod orchestrator;
pub mod preference_resolver;
pub mod recipe_engine;
pub mod recipe_validation;
pub mod response_builder;
pub mod response_templates;
pub mod session_context;
pub mod user_constraints;
