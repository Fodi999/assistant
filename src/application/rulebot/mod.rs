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

pub mod orchestrator;
pub mod chat_engine;
pub mod chat_response;
pub mod intent_keywords;
pub mod goal_modifier;
pub mod intent_router;
pub mod response_builder;
pub mod response_templates;
pub mod session_context;
pub mod chef_coach;
pub mod meal_builder;
pub mod cooking_rules;
pub mod food_pairing;
pub mod recipe_engine;
pub mod dish_schema;
pub mod ingredient_resolver;
pub mod nutrition_math;
pub mod display_name;
pub mod user_constraints;
pub mod constraint_policy;
pub mod goal_engine;
pub mod adaptation_engine;
pub mod recipe_validation;
pub mod auto_fix;
pub mod ai_brain;
pub mod flavor_engine;
