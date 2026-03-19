//! SmartParse — deterministic text → ingredient slug parser.
//!
//! `POST /api/smart/parse`
//!
//! Converts free-form text (ingredients or recipe fragments) into
//! a list of known `catalog_ingredients` rows. No LLM, no external APIs.
//!
//! Pipeline:
//! 1. Tokenize (split on whitespace + punctuation)
//! 2. Normalize via `ingredient_dictionary` (RU/UK/PL → EN canonical)
//! 3. Match against `catalog_ingredients` (exact slug → exact name → ILIKE → trigram)
//! 4. Dedup (preserve order, unique slugs)
//! 5. Limit (max 15)
//!
//! Target: <50ms, zero external calls.

pub mod response;
pub mod parser;
pub mod matcher;
pub mod service;

pub use response::SmartParseResponse;
pub use service::SmartParseService;
