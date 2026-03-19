//! Tool Registry — complete catalog of all 33+ tools.
//!
//! Serves `GET /public/tools/catalog` with full metadata:
//! tool ID, path, engine, method, description, parameters, cache TTL.

use serde::Serialize;
use crate::domain::engines::types::{EngineKind, ToolId};

// ── Registry entry ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ToolRegistryEntry {
    pub id:          String,
    pub path:        String,
    pub full_path:   String,
    pub engine:      EngineKind,
    pub method:      &'static str,
    pub description: &'static str,
    pub cache_ttl:   u64,
    pub parameters:  Vec<ToolParam>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolParam {
    pub name:     &'static str,
    pub r#type:   &'static str,
    pub required: bool,
    pub description: &'static str,
}

// ── Catalog response ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ToolCatalogResponse {
    pub total:   usize,
    pub engines: Vec<EngineSummary>,
    pub tools:   Vec<ToolRegistryEntry>,
}

#[derive(Debug, Serialize)]
pub struct EngineSummary {
    pub engine:      EngineKind,
    pub tool_count:  usize,
    pub description: &'static str,
}

// ── Build catalog ────────────────────────────────────────────────────────────

pub fn build_catalog() -> ToolCatalogResponse {
    let tools: Vec<ToolRegistryEntry> = ToolId::all()
        .iter()
        .map(|&id| ToolRegistryEntry {
            id:          format!("{}", id.path()),
            path:        format!("/tools/{}", id.path()),
            full_path:   format!("/public/tools/{}", id.path()),
            engine:      id.engine(),
            method:      id.method(),
            description: id.description(),
            cache_ttl:   id.cache_ttl_secs(),
            parameters:  params_for(id),
        })
        .collect();

    let engines = vec![
        EngineSummary {
            engine: EngineKind::Conversion,
            tool_count: tools.iter().filter(|t| t.engine == EngineKind::Conversion).count(),
            description: "Unit conversion, scaling, density-aware cross-group math",
        },
        EngineSummary {
            engine: EngineKind::Nutrition,
            tool_count: tools.iter().filter(|t| t.engine == EngineKind::Nutrition).count(),
            description: "Nutrition lookup, comparison, scoring, ingredient database",
        },
        EngineSummary {
            engine: EngineKind::Seasonality,
            tool_count: tools.iter().filter(|t| t.engine == EngineKind::Seasonality).count(),
            description: "Seasonal calendar, fish seasonality, product availability",
        },
        EngineSummary {
            engine: EngineKind::Recipe,
            tool_count: tools.iter().filter(|t| t.engine == EngineKind::Recipe).count(),
            description: "Recipe analysis, nutrition aggregation, flavor profiling, sharing",
        },
        EngineSummary {
            engine: EngineKind::Kitchen,
            tool_count: tools.iter().filter(|t| t.engine == EngineKind::Kitchen).count(),
            description: "Yield calculator, food cost, equivalents, suggestions",
        },
    ];

    ToolCatalogResponse {
        total: tools.len(),
        engines,
        tools,
    }
}

// ── Parameter definitions per tool ───────────────────────────────────────────

fn params_for(tool: ToolId) -> Vec<ToolParam> {
    use ToolId::*;
    match tool {
        Convert => vec![
            ToolParam { name: "value", r#type: "number", required: true, description: "Value to convert" },
            ToolParam { name: "from",  r#type: "string", required: true, description: "Source unit (g, kg, oz, lb, ml, l, cup, tbsp, tsp, ...)" },
            ToolParam { name: "to",    r#type: "string", required: true, description: "Target unit" },
            ToolParam { name: "lang",  r#type: "string", required: false, description: "Language: en, ru, pl, uk" },
        ],
        ListUnits => vec![
            ToolParam { name: "lang", r#type: "string", required: false, description: "Language for labels" },
        ],
        IngredientScale => vec![
            ToolParam { name: "ingredient",    r#type: "string", required: false, description: "Ingredient name or slug" },
            ToolParam { name: "value",         r#type: "number", required: true, description: "Amount" },
            ToolParam { name: "unit",          r#type: "string", required: false, description: "Unit (default: g)" },
            ToolParam { name: "from_portions", r#type: "number", required: true, description: "Original portions" },
            ToolParam { name: "to_portions",   r#type: "number", required: true, description: "Target portions" },
            ToolParam { name: "lang",          r#type: "string", required: false, description: "Language" },
        ],
        IngredientConvert => vec![
            ToolParam { name: "ingredient", r#type: "string", required: true, description: "Ingredient name or slug" },
            ToolParam { name: "value",      r#type: "number", required: true, description: "Value to convert" },
            ToolParam { name: "from",       r#type: "string", required: true, description: "Source unit" },
            ToolParam { name: "to",         r#type: "string", required: true, description: "Target unit" },
            ToolParam { name: "lang",       r#type: "string", required: false, description: "Language" },
        ],
        Nutrition => vec![
            ToolParam { name: "ingredient", r#type: "string", required: true, description: "Ingredient name or slug" },
            ToolParam { name: "amount",     r#type: "number", required: false, description: "Amount (default: 100)" },
            ToolParam { name: "unit",       r#type: "string", required: false, description: "Unit (default: g)" },
            ToolParam { name: "lang",       r#type: "string", required: false, description: "Language" },
        ],
        IngredientsDb => vec![
            ToolParam { name: "q",    r#type: "string", required: false, description: "Search query" },
            ToolParam { name: "lang", r#type: "string", required: false, description: "Language" },
            ToolParam { name: "limit", r#type: "number", required: false, description: "Max results (default: 50)" },
        ],
        CompareFoods => vec![
            ToolParam { name: "a",    r#type: "string", required: true, description: "First ingredient" },
            ToolParam { name: "b",    r#type: "string", required: true, description: "Second ingredient" },
            ToolParam { name: "lang", r#type: "string", required: false, description: "Language" },
        ],
        FishSeason | FishSeasonTable => vec![
            ToolParam { name: "lang",   r#type: "string", required: false, description: "Language" },
            ToolParam { name: "region", r#type: "string", required: false, description: "Region code (default: PL)" },
        ],
        SeasonalCalendar => vec![
            ToolParam { name: "type",   r#type: "string", required: false, description: "Product type (default: seafood)" },
            ToolParam { name: "lang",   r#type: "string", required: false, description: "Language" },
            ToolParam { name: "region", r#type: "string", required: false, description: "Region code" },
        ],
        InSeasonNow => vec![
            ToolParam { name: "type",   r#type: "string", required: false, description: "Product type" },
            ToolParam { name: "lang",   r#type: "string", required: false, description: "Language" },
            ToolParam { name: "region", r#type: "string", required: false, description: "Region code" },
        ],
        ProductSeasonality => vec![
            ToolParam { name: "slug",   r#type: "string", required: true, description: "Product slug" },
            ToolParam { name: "lang",   r#type: "string", required: false, description: "Language" },
            ToolParam { name: "region", r#type: "string", required: false, description: "Region code" },
        ],
        BestInSeason => vec![
            ToolParam { name: "month",  r#type: "number", required: false, description: "Month (1-12, default: current)" },
            ToolParam { name: "type",   r#type: "string", required: false, description: "Product type" },
            ToolParam { name: "lang",   r#type: "string", required: false, description: "Language" },
            ToolParam { name: "region", r#type: "string", required: false, description: "Region code" },
        ],
        ProductsByMonth => vec![
            ToolParam { name: "month",  r#type: "number", required: true, description: "Month (1-12)" },
            ToolParam { name: "type",   r#type: "string", required: false, description: "Product type" },
            ToolParam { name: "lang",   r#type: "string", required: false, description: "Language" },
            ToolParam { name: "region", r#type: "string", required: false, description: "Region code" },
        ],
        BestRightNow => vec![
            ToolParam { name: "type",   r#type: "string", required: false, description: "Product type" },
            ToolParam { name: "lang",   r#type: "string", required: false, description: "Language" },
            ToolParam { name: "region", r#type: "string", required: false, description: "Region code" },
            ToolParam { name: "limit",  r#type: "number", required: false, description: "Max results" },
        ],
        RecipeAnalyze => vec![
            ToolParam { name: "ingredients", r#type: "array", required: true, description: "List of {slug, grams}" },
            ToolParam { name: "portions",    r#type: "number", required: false, description: "Number of portions" },
            ToolParam { name: "lang",        r#type: "string", required: false, description: "Language" },
        ],
        RecipeNutrition => vec![
            ToolParam { name: "ingredients", r#type: "array", required: true, description: "List of {slug, grams}" },
            ToolParam { name: "portions",    r#type: "number", required: false, description: "Number of portions" },
            ToolParam { name: "lang",        r#type: "string", required: false, description: "Language" },
        ],
        RecipeCost => vec![
            ToolParam { name: "ingredients", r#type: "array", required: true, description: "List of {slug, grams}" },
            ToolParam { name: "portions",    r#type: "number", required: false, description: "Number of portions" },
            ToolParam { name: "lang",        r#type: "string", required: false, description: "Language" },
        ],
        Scale => vec![
            ToolParam { name: "value",         r#type: "number", required: true, description: "Amount" },
            ToolParam { name: "from_portions", r#type: "number", required: true, description: "Original portions" },
            ToolParam { name: "to_portions",   r#type: "number", required: true, description: "Target portions" },
        ],
        Yield => vec![
            ToolParam { name: "raw_weight",    r#type: "number", required: true, description: "Raw weight" },
            ToolParam { name: "usable_weight", r#type: "number", required: true, description: "Usable weight after cleaning" },
        ],
        FoodCost => vec![
            ToolParam { name: "price",    r#type: "number", required: true, description: "Price per unit" },
            ToolParam { name: "amount",   r#type: "number", required: true, description: "Amount" },
            ToolParam { name: "portions", r#type: "number", required: false, description: "Number of portions" },
            ToolParam { name: "sell_price", r#type: "number", required: false, description: "Sell price for margin calc" },
        ],
        _ => vec![
            ToolParam { name: "lang", r#type: "string", required: false, description: "Language" },
        ],
    }
}
