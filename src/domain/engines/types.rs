//! Shared types for the Engine layer.
//!
//! Strongly-typed tool identifiers, engine tags, and common enums.

use serde::{Deserialize, Serialize};
use std::fmt;

// ── Engine identifier ────────────────────────────────────────────────────────

/// Which engine handles a given tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineKind {
    Conversion,
    Nutrition,
    Seasonality,
    Recipe,
    Kitchen,
}

impl fmt::Display for EngineKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Conversion  => write!(f, "conversion"),
            Self::Nutrition   => write!(f, "nutrition"),
            Self::Seasonality => write!(f, "seasonality"),
            Self::Recipe      => write!(f, "recipe"),
            Self::Kitchen     => write!(f, "kitchen"),
        }
    }
}

// ── Tool identifier ──────────────────────────────────────────────────────────

/// Every tool in the platform has a unique string ID.
/// This enum provides compile-time safety — no typos, no missing matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToolId {
    // ── Conversion Engine (7 tools) ──
    Convert,
    ListUnits,
    IngredientScale,
    IngredientConvert,
    SeoIngredientConvert,
    MeasureConversion,
    IngredientMeasures,

    // ── Nutrition Engine (4 tools) ──
    Nutrition,
    IngredientsDb,
    CompareFoods,
    ResolveSlug,

    // ── Seasonality Engine (9 tools) ──
    FishSeason,
    FishSeasonTable,
    SeasonalCalendar,
    InSeasonNow,
    ProductSeasonality,
    BestInSeason,
    ProductsByMonth,
    BestRightNow,
    ProductSearch,
    ListRegions,

    // ── Recipe Engine (4 tools) ──
    RecipeAnalyze,
    RecipeNutrition,
    RecipeCost,
    ShareRecipe,
    GetSharedRecipe,

    // ── Kitchen Engine (6 tools) ──
    Scale,
    Yield,
    IngredientEquivalents,
    FoodCost,
    IngredientSuggestions,
    PopularConversions,

    // ── Meta ──
    ToolsCatalog,
}

impl ToolId {
    /// The engine that owns this tool.
    pub fn engine(&self) -> EngineKind {
        use ToolId::*;
        match self {
            Convert | ListUnits | IngredientScale |
            IngredientConvert | SeoIngredientConvert |
            MeasureConversion | IngredientMeasures
                => EngineKind::Conversion,

            Nutrition | IngredientsDb | CompareFoods | ResolveSlug
                => EngineKind::Nutrition,

            FishSeason | FishSeasonTable | SeasonalCalendar |
            InSeasonNow | ProductSeasonality | BestInSeason |
            ProductsByMonth | BestRightNow | ProductSearch | ListRegions
                => EngineKind::Seasonality,

            RecipeAnalyze | RecipeNutrition | RecipeCost |
            ShareRecipe | GetSharedRecipe
                => EngineKind::Recipe,

            Scale | Yield | IngredientEquivalents |
            FoodCost | IngredientSuggestions | PopularConversions
                => EngineKind::Kitchen,

            ToolsCatalog => EngineKind::Kitchen, // meta, doesn't matter
        }
    }

    /// REST path fragment (after /public/tools/)
    pub fn path(&self) -> &'static str {
        use ToolId::*;
        match self {
            Convert                => "convert",
            ListUnits              => "units",
            IngredientScale        => "ingredient-scale",
            IngredientConvert      => "ingredient-convert",
            SeoIngredientConvert   => ":from_to/:slug",
            MeasureConversion      => "measure-conversion",
            IngredientMeasures     => "ingredient-measures",
            Nutrition              => "nutrition",
            IngredientsDb          => "ingredients",
            CompareFoods           => "compare",
            ResolveSlug            => "resolve-slug",
            FishSeason             => "fish-season",
            FishSeasonTable        => "fish-season-table",
            SeasonalCalendar       => "seasonal-calendar",
            InSeasonNow            => "in-season-now",
            ProductSeasonality     => "product-seasonality",
            BestInSeason           => "best-in-season",
            ProductsByMonth        => "products-by-month",
            BestRightNow           => "best-right-now",
            ProductSearch          => "product-search",
            ListRegions            => "regions",
            RecipeAnalyze          => "recipe-analyze",
            RecipeNutrition        => "recipe-nutrition",
            RecipeCost             => "recipe-cost",
            ShareRecipe            => "share-recipe",
            GetSharedRecipe        => "shared-recipe/:slug",
            Scale                  => "scale",
            Yield                  => "yield",
            IngredientEquivalents  => "ingredient-equivalents",
            FoodCost               => "food-cost",
            IngredientSuggestions  => "ingredient-suggestions",
            PopularConversions     => "popular-conversions",
            ToolsCatalog           => "catalog",
        }
    }

    /// Human-readable description (EN).
    pub fn description(&self) -> &'static str {
        use ToolId::*;
        match self {
            Convert                => "Universal unit converter (mass & volume)",
            ListUnits              => "List all supported units with i18n labels",
            IngredientScale        => "Scale an ingredient between portion sizes",
            IngredientConvert      => "Density-aware cross-group ingredient converter",
            SeoIngredientConvert   => "SEO-friendly ingredient conversion (e.g. cup-to-grams/flour)",
            MeasureConversion      => "How many grams in a cup/tbsp/tsp of an ingredient",
            IngredientMeasures     => "Full cup/tbsp/tsp grams table for an ingredient",
            Nutrition              => "Nutrition calculator for any ingredient + amount",
            IngredientsDb          => "Search ingredients database with nutrition data",
            CompareFoods           => "Compare nutrition of two ingredients side-by-side",
            ResolveSlug            => "Resolve ingredient name to canonical slug",
            FishSeason             => "Fish seasonality calendar (single fish)",
            FishSeasonTable        => "Full fish seasonality table with catalog data",
            SeasonalCalendar       => "Universal seasonal calendar by product type",
            InSeasonNow            => "What products are in season right now",
            ProductSeasonality     => "Seasonality data for a single product",
            BestInSeason           => "Best products in season for a given month",
            ProductsByMonth        => "All products with their status for a given month",
            BestRightNow           => "Best products in season right now (SEO powerhouse)",
            ProductSearch          => "Search products with seasonality data",
            ListRegions            => "List available seasonality regions",
            RecipeAnalyze          => "Full recipe analysis: nutrition + flavor + diet + rules",
            RecipeNutrition        => "Calculate total nutrition for a list of ingredients",
            RecipeCost             => "Calculate recipe cost from ingredient weights",
            ShareRecipe            => "Share a recipe and get a shareable slug",
            GetSharedRecipe        => "Retrieve a shared recipe by slug",
            Scale                  => "Recipe portion scaler",
            Yield                  => "Cooking yield & waste calculator",
            IngredientEquivalents  => "Convert ingredient to all units via density",
            FoodCost               => "Food cost, margin & markup calculator",
            IngredientSuggestions  => "Suggest ingredients by volume unit with grams",
            PopularConversions     => "Curated popular cooking conversions (SEO)",
            ToolsCatalog           => "List all available tools with metadata",
        }
    }

    /// HTTP method: GET or POST.
    pub fn method(&self) -> &'static str {
        use ToolId::*;
        match self {
            RecipeAnalyze | RecipeNutrition | RecipeCost | ShareRecipe => "POST",
            _ => "GET",
        }
    }

    /// Cache TTL hint in seconds. 0 = no cache.
    pub fn cache_ttl_secs(&self) -> u64 {
        use ToolId::*;
        match self {
            // Seasonality data changes rarely — 24 hours
            FishSeason | FishSeasonTable | SeasonalCalendar |
            InSeasonNow | ProductSeasonality | BestInSeason |
            ProductsByMonth | BestRightNow | ListRegions
                => 86_400,

            // Nutrition / catalog — 1 hour
            Nutrition | IngredientsDb | CompareFoods | ResolveSlug |
            IngredientMeasures | MeasureConversion | ProductSearch
                => 3_600,

            // Static conversions — 7 days
            Convert | ListUnits | PopularConversions
                => 604_800,

            // Dynamic / user-specific — no cache
            RecipeAnalyze | RecipeNutrition | RecipeCost |
            ShareRecipe | GetSharedRecipe
                => 0,

            // Moderate — 1 hour
            _ => 3_600,
        }
    }

    /// All tool IDs in registration order.
    pub fn all() -> &'static [ToolId] {
        use ToolId::*;
        &[
            Convert, ListUnits, IngredientScale, IngredientConvert,
            SeoIngredientConvert, MeasureConversion, IngredientMeasures,
            Nutrition, IngredientsDb, CompareFoods, ResolveSlug,
            FishSeason, FishSeasonTable, SeasonalCalendar, InSeasonNow,
            ProductSeasonality, BestInSeason, ProductsByMonth, BestRightNow,
            ProductSearch, ListRegions,
            RecipeAnalyze, RecipeNutrition, RecipeCost, ShareRecipe, GetSharedRecipe,
            Scale, Yield, IngredientEquivalents, FoodCost,
            IngredientSuggestions, PopularConversions,
            ToolsCatalog,
        ]
    }
}

impl fmt::Display for ToolId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.path())
    }
}
