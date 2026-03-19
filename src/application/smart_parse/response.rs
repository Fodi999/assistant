use serde::Serialize;

/// How the token was matched.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchType {
    /// slug = token  (priority 1)
    Exact,
    /// LOWER(name_*) = token  (priority 2–6)
    Name,
    /// ILIKE '%token%'  (priority 7–9)
    Ilike,
    /// pg_trgm similarity ≥ 0.25  (priority 10)
    Fuzzy,
}

/// A matched ingredient (slug + localized display name + confidence).
#[derive(Debug, Clone, Serialize)]
pub struct IngredientShort {
    pub slug: String,
    pub name: String,
    /// Match confidence 0.0–1.0.
    pub confidence: f32,
    /// How the match was found.
    pub match_type: MatchType,
}

/// Timing / stats metadata.
#[derive(Debug, Clone, Serialize)]
pub struct ParseMeta {
    /// Total tokens extracted from input text.
    pub tokens: usize,
    /// Number of tokens that matched a catalog ingredient.
    pub matched: usize,
    /// Number of tokens that did NOT match.
    pub unmatched: usize,
    /// Wall-clock time of the entire parse pipeline (ms).
    pub timing_ms: u128,
    /// Whether the result was served from cache.
    pub cache: bool,
}

/// Top-level response for `POST /api/smart/parse`.
#[derive(Debug, Clone, Serialize)]
pub struct SmartParseResponse {
    /// Ingredients successfully resolved from the catalog.
    pub found: Vec<IngredientShort>,
    /// Tokens that could not be resolved.
    pub unknown: Vec<String>,
    /// Pipeline metadata.
    pub meta: ParseMeta,
}
