use crate::domain::ProcessingState;

// ──────────────────────────────────────────────────────────────
// Product groups — determines which coefficient table to use
// ──────────────────────────────────────────────────────────────

/// Product group classification for nutrition transform coefficients.
/// Different food groups respond very differently to the same cooking method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductGroup {
    /// Fruits, vegetables, leafy greens, salads (water 80–96%)
    WateryProduce,
    /// Potatoes, root vegetables (water 60–80%, starchy)
    DensePlant,
    /// Nuts, seeds (water 2–6%)
    NutsSeeds,
    /// Meat, poultry, fish, seafood (water 60–80%, protein-rich)
    MeatFish,
    /// Grains, cereals, flour, pasta, legumes (water 10–13%)
    DryGoods,
    /// Oils, butter, fats (water 0–16%)
    OilsFats,
    /// Spices, dried herbs (water ~10%)
    Spices,
    /// Dairy, eggs, sauces, condiments, etc.
    Other,
}

/// Classify product_type string into a ProductGroup.
/// product_type comes from catalog_ingredients.product_type column.
pub fn classify_group(product_type: &str) -> ProductGroup {
    match product_type.to_lowercase().as_str() {
        "fruit" | "berry" | "melon" => ProductGroup::WateryProduce,
        "vegetable" | "leafy" | "salad" | "mushroom" => ProductGroup::WateryProduce,
        "root" | "tuber" => ProductGroup::DensePlant,
        "nut" | "seed" => ProductGroup::NutsSeeds,
        "meat" | "poultry" | "fish" | "seafood" => ProductGroup::MeatFish,
        "grain" | "cereal" | "flour" | "pasta" | "bread" | "legume" => ProductGroup::DryGoods,
        "oil" | "fat" | "butter" => ProductGroup::OilsFats,
        "spice" | "herb" | "seasoning" => ProductGroup::Spices,
        _ => ProductGroup::Other, // dairy, egg, sauce, condiment, sweetener, beverage, etc.
    }
}

// ──────────────────────────────────────────────────────────────
// Base & Transformed nutrition structs
// ──────────────────────────────────────────────────────────────

/// Base nutrition values from the raw ingredient (per 100g)
#[derive(Debug, Clone)]
pub struct BaseNutrition {
    pub calories: f64,
    pub protein: f64,
    pub fat: f64,
    pub carbs: f64,
    pub fiber: f64,
    pub water_percent: f64,
}

/// Transformed nutrition for a specific processing state
#[derive(Debug, Clone)]
pub struct TransformedNutrition {
    pub calories_per_100g: f64,
    pub protein_per_100g: f64,
    pub fat_per_100g: f64,
    pub carbs_per_100g: f64,
    pub fiber_per_100g: f64,
    pub water_percent: f64,
}

// ──────────────────────────────────────────────────────────────
// Coefficient tables per ProductGroup × ProcessingState
// ──────────────────────────────────────────────────────────────

/// Transformation coefficients applied to base (raw) nutrition.
/// `fat_add` is an absolute addition (g per 100g) for frying, not a multiplier.
#[derive(Debug, Clone)]
struct TransformCoefficients {
    calories: f64,
    protein: f64,
    fat: f64,
    carbs: f64,
    fiber: f64,
    fat_add: f64,      // absolute fat addition (g/100g) — mainly for frying
    water_delta: f64,  // absolute change in water % (negative = loss)
}

impl TransformCoefficients {
    /// Identity — no change (raw / frozen for most groups)
    const fn identity() -> Self {
        Self {
            calories: 1.0,
            protein: 1.0,
            fat: 1.0,
            carbs: 1.0,
            fiber: 1.0,
            fat_add: 0.0,
            water_delta: 0.0,
        }
    }
}

/// Get coefficients based on product group AND processing state.
/// Values based on USDA / McCance & Widdowson food composition data.
fn get_coefficients(group: ProductGroup, state: ProcessingState) -> TransformCoefficients {
    match (group, state) {
        // ── RAW: always identity ──
        (_, ProcessingState::Raw) => TransformCoefficients::identity(),

        // ── FROZEN: negligible change for all groups ──
        (_, ProcessingState::Frozen) => TransformCoefficients::identity(),

        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        // WATERY PRODUCE (fruits, vegetables, leafy, mushrooms)
        // High water content → big concentration on drying
        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        (ProductGroup::WateryProduce, ProcessingState::Boiled) => TransformCoefficients {
            calories: 0.95, protein: 0.90, fat: 0.95, carbs: 0.95, fiber: 0.90,
            fat_add: 0.0, water_delta: 3.0,
        },
        (ProductGroup::WateryProduce, ProcessingState::Steamed) => TransformCoefficients {
            calories: 0.98, protein: 0.95, fat: 1.0, carbs: 0.98, fiber: 0.95,
            fat_add: 0.0, water_delta: 1.0,
        },
        (ProductGroup::WateryProduce, ProcessingState::Baked) => TransformCoefficients {
            calories: 1.10, protein: 1.0, fat: 1.0, carbs: 1.08, fiber: 1.0,
            fat_add: 0.0, water_delta: -10.0,
        },
        (ProductGroup::WateryProduce, ProcessingState::Grilled) => TransformCoefficients {
            calories: 1.08, protein: 1.0, fat: 0.90, carbs: 1.05, fiber: 1.0,
            fat_add: 0.0, water_delta: -12.0,
        },
        (ProductGroup::WateryProduce, ProcessingState::Fried) => TransformCoefficients {
            calories: 1.30, protein: 0.95, fat: 1.0, carbs: 1.05, fiber: 0.90,
            fat_add: 8.0, water_delta: -20.0,
        },
        (ProductGroup::WateryProduce, ProcessingState::Smoked) => TransformCoefficients {
            calories: 1.15, protein: 1.10, fat: 1.05, carbs: 1.10, fiber: 1.0,
            fat_add: 0.0, water_delta: -20.0,
        },
        // Dried: water-based concentration — calculated dynamically below
        (ProductGroup::WateryProduce, ProcessingState::Dried) => TransformCoefficients {
            // placeholder — overridden by water-based drying logic
            calories: 1.0, protein: 1.0, fat: 1.0, carbs: 1.0, fiber: 1.0,
            fat_add: 0.0, water_delta: 0.0,
        },
        (ProductGroup::WateryProduce, ProcessingState::Pickled) => TransformCoefficients {
            calories: 0.90, protein: 0.85, fat: 1.0, carbs: 1.15, fiber: 0.85,
            fat_add: 0.0, water_delta: 3.0,
        },

        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        // DENSE PLANT (potato, root vegetables)
        // Moderate water, starchy → absorbs oil well when fried
        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        (ProductGroup::DensePlant, ProcessingState::Boiled) => TransformCoefficients {
            calories: 1.02, protein: 0.95, fat: 0.95, carbs: 1.0, fiber: 0.95,
            fat_add: 0.0, water_delta: 5.0,
        },
        (ProductGroup::DensePlant, ProcessingState::Steamed) => TransformCoefficients {
            calories: 1.01, protein: 0.98, fat: 1.0, carbs: 1.0, fiber: 0.98,
            fat_add: 0.0, water_delta: 2.0,
        },
        (ProductGroup::DensePlant, ProcessingState::Baked) => TransformCoefficients {
            calories: 1.12, protein: 1.05, fat: 1.0, carbs: 1.10, fiber: 1.0,
            fat_add: 0.0, water_delta: -12.0,
        },
        (ProductGroup::DensePlant, ProcessingState::Grilled) => TransformCoefficients {
            calories: 1.08, protein: 1.05, fat: 0.90, carbs: 1.05, fiber: 1.0,
            fat_add: 0.0, water_delta: -14.0,
        },
        (ProductGroup::DensePlant, ProcessingState::Fried) => TransformCoefficients {
            calories: 1.35, protein: 1.0, fat: 1.0, carbs: 1.05, fiber: 0.95,
            fat_add: 10.0, water_delta: -25.0,
        },
        (ProductGroup::DensePlant, ProcessingState::Smoked) => TransformCoefficients {
            calories: 1.20, protein: 1.15, fat: 1.10, carbs: 1.10, fiber: 1.0,
            fat_add: 0.0, water_delta: -22.0,
        },
        (ProductGroup::DensePlant, ProcessingState::Dried) => TransformCoefficients {
            // placeholder — overridden by water-based drying
            calories: 1.0, protein: 1.0, fat: 1.0, carbs: 1.0, fiber: 1.0,
            fat_add: 0.0, water_delta: 0.0,
        },
        (ProductGroup::DensePlant, ProcessingState::Pickled) => TransformCoefficients {
            calories: 0.95, protein: 0.90, fat: 1.0, carbs: 1.10, fiber: 0.90,
            fat_add: 0.0, water_delta: 3.0,
        },

        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        // NUTS & SEEDS (water 2–6%)
        // Already very dry → drying barely changes anything
        // Very high fat → frying adds little relative change
        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        (ProductGroup::NutsSeeds, ProcessingState::Boiled) => TransformCoefficients {
            calories: 1.02, protein: 0.98, fat: 1.0, carbs: 1.0, fiber: 0.98,
            fat_add: 0.0, water_delta: 2.0,
        },
        (ProductGroup::NutsSeeds, ProcessingState::Steamed) => TransformCoefficients {
            calories: 1.0, protein: 1.0, fat: 1.0, carbs: 1.0, fiber: 1.0,
            fat_add: 0.0, water_delta: 1.0,
        },
        (ProductGroup::NutsSeeds, ProcessingState::Baked) => TransformCoefficients {
            // Roasting nuts: slight water loss, flavor concentration
            calories: 1.05, protein: 1.02, fat: 1.02, carbs: 1.0, fiber: 1.0,
            fat_add: 0.0, water_delta: -2.0,
        },
        (ProductGroup::NutsSeeds, ProcessingState::Grilled) => TransformCoefficients {
            calories: 1.05, protein: 1.02, fat: 1.0, carbs: 1.0, fiber: 1.0,
            fat_add: 0.0, water_delta: -2.0,
        },
        (ProductGroup::NutsSeeds, ProcessingState::Fried) => TransformCoefficients {
            // Nuts already have 50-65g fat; frying adds minimal extra
            calories: 1.10, protein: 1.0, fat: 1.0, carbs: 1.02, fiber: 0.98,
            fat_add: 3.0, water_delta: -2.0,
        },
        (ProductGroup::NutsSeeds, ProcessingState::Smoked) => TransformCoefficients {
            calories: 1.10, protein: 1.05, fat: 1.05, carbs: 1.0, fiber: 1.0,
            fat_add: 0.0, water_delta: -2.0,
        },
        (ProductGroup::NutsSeeds, ProcessingState::Dried) => TransformCoefficients {
            // Already dry (4% water) → almost no change
            calories: 1.02, protein: 1.02, fat: 1.01, carbs: 1.02, fiber: 1.02,
            fat_add: 0.0, water_delta: -2.0,
        },
        (ProductGroup::NutsSeeds, ProcessingState::Pickled) => TransformCoefficients {
            calories: 1.03, protein: 0.95, fat: 1.0, carbs: 1.05, fiber: 0.95,
            fat_add: 0.0, water_delta: 2.0,
        },

        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        // MEAT & FISH (water 60–80%, protein-rich)
        // Cooking drives off water → concentrates protein
        // Grilling drips fat, frying adds fat
        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        (ProductGroup::MeatFish, ProcessingState::Boiled) => TransformCoefficients {
            calories: 1.05, protein: 1.05, fat: 0.90, carbs: 1.0, fiber: 1.0,
            fat_add: 0.0, water_delta: 2.0,
        },
        (ProductGroup::MeatFish, ProcessingState::Steamed) => TransformCoefficients {
            calories: 1.03, protein: 1.03, fat: 0.95, carbs: 1.0, fiber: 1.0,
            fat_add: 0.0, water_delta: 1.0,
        },
        (ProductGroup::MeatFish, ProcessingState::Baked) => TransformCoefficients {
            calories: 1.15, protein: 1.15, fat: 1.05, carbs: 1.0, fiber: 1.0,
            fat_add: 0.0, water_delta: -15.0,
        },
        (ProductGroup::MeatFish, ProcessingState::Grilled) => TransformCoefficients {
            calories: 1.10, protein: 1.20, fat: 0.80, carbs: 1.0, fiber: 1.0,
            fat_add: 0.0, water_delta: -18.0,
        },
        (ProductGroup::MeatFish, ProcessingState::Fried) => TransformCoefficients {
            calories: 1.25, protein: 1.10, fat: 1.0, carbs: 1.0, fiber: 1.0,
            fat_add: 8.0, water_delta: -22.0,
        },
        (ProductGroup::MeatFish, ProcessingState::Smoked) => TransformCoefficients {
            calories: 1.30, protein: 1.25, fat: 1.15, carbs: 1.0, fiber: 1.0,
            fat_add: 0.0, water_delta: -25.0,
        },
        (ProductGroup::MeatFish, ProcessingState::Dried) => TransformCoefficients {
            // placeholder — overridden by water-based drying
            calories: 1.0, protein: 1.0, fat: 1.0, carbs: 1.0, fiber: 1.0,
            fat_add: 0.0, water_delta: 0.0,
        },
        (ProductGroup::MeatFish, ProcessingState::Pickled) => TransformCoefficients {
            calories: 0.95, protein: 0.95, fat: 1.0, carbs: 1.05, fiber: 1.0,
            fat_add: 0.0, water_delta: 3.0,
        },

        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        // DRY GOODS (grains, cereals, flour, pasta, legumes — water 10–13%)
        // Boiling absorbs a LOT of water (weight_ratio ≈ 2.5)
        // Drying barely changes already-dry product
        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        (ProductGroup::DryGoods, ProcessingState::Boiled) => TransformCoefficients {
            // weight_ratio ≈ 2.5 → coeff = 1/2.5 = 0.40
            // Rice 360 raw / 2.5 = 144 kcal cooked ≈ USDA 130 kcal ✓
            calories: 0.40, protein: 0.40, fat: 0.30, carbs: 0.40, fiber: 0.40,
            fat_add: 0.0, water_delta: 55.0,
        },
        (ProductGroup::DryGoods, ProcessingState::Steamed) => TransformCoefficients {
            // weight_ratio ≈ 2.2 for steaming (slightly less water)
            calories: 0.45, protein: 0.42, fat: 0.35, carbs: 0.45, fiber: 0.45,
            fat_add: 0.0, water_delta: 50.0,
        },
        (ProductGroup::DryGoods, ProcessingState::Baked) => TransformCoefficients {
            calories: 1.05, protein: 1.02, fat: 1.0, carbs: 1.05, fiber: 1.0,
            fat_add: 0.0, water_delta: -3.0,
        },
        (ProductGroup::DryGoods, ProcessingState::Grilled) => TransformCoefficients {
            calories: 1.05, protein: 1.0, fat: 0.95, carbs: 1.05, fiber: 1.0,
            fat_add: 0.0, water_delta: -3.0,
        },
        (ProductGroup::DryGoods, ProcessingState::Fried) => TransformCoefficients {
            calories: 1.20, protein: 1.0, fat: 1.0, carbs: 1.05, fiber: 0.95,
            fat_add: 6.0, water_delta: -5.0,
        },
        (ProductGroup::DryGoods, ProcessingState::Smoked) => TransformCoefficients {
            calories: 1.08, protein: 1.05, fat: 1.02, carbs: 1.05, fiber: 1.0,
            fat_add: 0.0, water_delta: -5.0,
        },
        (ProductGroup::DryGoods, ProcessingState::Dried) => TransformCoefficients {
            // Already dry → negligible change
            calories: 1.02, protein: 1.02, fat: 1.0, carbs: 1.02, fiber: 1.02,
            fat_add: 0.0, water_delta: -3.0,
        },
        (ProductGroup::DryGoods, ProcessingState::Pickled) => TransformCoefficients {
            calories: 0.95, protein: 0.90, fat: 1.0, carbs: 1.05, fiber: 0.90,
            fat_add: 0.0, water_delta: 5.0,
        },

        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        // OILS & FATS (water 0–16%)
        // Cooking barely changes pure fats
        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        (ProductGroup::OilsFats, ProcessingState::Boiled)  => TransformCoefficients::identity(),
        (ProductGroup::OilsFats, ProcessingState::Steamed) => TransformCoefficients::identity(),
        (ProductGroup::OilsFats, ProcessingState::Baked)   => TransformCoefficients {
            calories: 1.0, protein: 1.0, fat: 1.0, carbs: 1.0, fiber: 1.0,
            fat_add: 0.0, water_delta: -5.0,
        },
        (ProductGroup::OilsFats, ProcessingState::Grilled) => TransformCoefficients::identity(),
        (ProductGroup::OilsFats, ProcessingState::Fried)   => TransformCoefficients::identity(),
        (ProductGroup::OilsFats, ProcessingState::Smoked)  => TransformCoefficients {
            calories: 1.02, protein: 1.0, fat: 1.02, carbs: 1.0, fiber: 1.0,
            fat_add: 0.0, water_delta: -5.0,
        },
        (ProductGroup::OilsFats, ProcessingState::Dried)   => TransformCoefficients {
            calories: 1.0, protein: 1.0, fat: 1.0, carbs: 1.0, fiber: 1.0,
            fat_add: 0.0, water_delta: -10.0,
        },
        (ProductGroup::OilsFats, ProcessingState::Pickled) => TransformCoefficients::identity(),

        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        // SPICES (water ~10%, used in small quantities)
        // Minimal change — spices are flavoring, not bulk
        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        (ProductGroup::Spices, ProcessingState::Boiled)  => TransformCoefficients {
            calories: 0.95, protein: 0.90, fat: 1.0, carbs: 0.95, fiber: 0.90,
            fat_add: 0.0, water_delta: 5.0,
        },
        (ProductGroup::Spices, ProcessingState::Steamed) => TransformCoefficients::identity(),
        (ProductGroup::Spices, ProcessingState::Baked)   => TransformCoefficients {
            calories: 1.05, protein: 1.0, fat: 1.0, carbs: 1.05, fiber: 1.0,
            fat_add: 0.0, water_delta: -3.0,
        },
        (ProductGroup::Spices, ProcessingState::Grilled) => TransformCoefficients {
            calories: 1.05, protein: 1.0, fat: 1.0, carbs: 1.05, fiber: 1.0,
            fat_add: 0.0, water_delta: -3.0,
        },
        (ProductGroup::Spices, ProcessingState::Fried)   => TransformCoefficients {
            calories: 1.10, protein: 1.0, fat: 1.0, carbs: 1.05, fiber: 0.95,
            fat_add: 2.0, water_delta: -3.0,
        },
        (ProductGroup::Spices, ProcessingState::Smoked)  => TransformCoefficients {
            calories: 1.05, protein: 1.0, fat: 1.0, carbs: 1.05, fiber: 1.0,
            fat_add: 0.0, water_delta: -5.0,
        },
        (ProductGroup::Spices, ProcessingState::Dried)   => TransformCoefficients {
            // Already mostly dry
            calories: 1.05, protein: 1.05, fat: 1.0, carbs: 1.05, fiber: 1.05,
            fat_add: 0.0, water_delta: -5.0,
        },
        (ProductGroup::Spices, ProcessingState::Pickled) => TransformCoefficients {
            calories: 0.95, protein: 0.90, fat: 1.0, carbs: 1.10, fiber: 0.90,
            fat_add: 0.0, water_delta: 5.0,
        },

        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        // OTHER (dairy, eggs, sauce, condiment, sweetener, beverage)
        // Moderate / generic coefficients
        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        (ProductGroup::Other, ProcessingState::Boiled) => TransformCoefficients {
            calories: 1.02, protein: 0.95, fat: 0.95, carbs: 1.0, fiber: 0.95,
            fat_add: 0.0, water_delta: 3.0,
        },
        (ProductGroup::Other, ProcessingState::Steamed) => TransformCoefficients {
            calories: 1.01, protein: 0.98, fat: 1.0, carbs: 1.0, fiber: 0.98,
            fat_add: 0.0, water_delta: 1.0,
        },
        (ProductGroup::Other, ProcessingState::Baked) => TransformCoefficients {
            calories: 1.10, protein: 1.05, fat: 1.02, carbs: 1.08, fiber: 1.0,
            fat_add: 0.0, water_delta: -10.0,
        },
        (ProductGroup::Other, ProcessingState::Grilled) => TransformCoefficients {
            calories: 1.08, protein: 1.05, fat: 0.90, carbs: 1.05, fiber: 1.0,
            fat_add: 0.0, water_delta: -12.0,
        },
        (ProductGroup::Other, ProcessingState::Fried) => TransformCoefficients {
            calories: 1.25, protein: 1.0, fat: 1.0, carbs: 1.05, fiber: 0.95,
            fat_add: 7.0, water_delta: -15.0,
        },
        (ProductGroup::Other, ProcessingState::Smoked) => TransformCoefficients {
            calories: 1.15, protein: 1.10, fat: 1.05, carbs: 1.05, fiber: 1.0,
            fat_add: 0.0, water_delta: -15.0,
        },
        (ProductGroup::Other, ProcessingState::Dried) => TransformCoefficients {
            // placeholder — overridden by water-based drying
            calories: 1.0, protein: 1.0, fat: 1.0, carbs: 1.0, fiber: 1.0,
            fat_add: 0.0, water_delta: 0.0,
        },
        (ProductGroup::Other, ProcessingState::Pickled) => TransformCoefficients {
            calories: 0.95, protein: 0.90, fat: 1.0, carbs: 1.10, fiber: 0.90,
            fat_add: 0.0, water_delta: 3.0,
        },
    }
}

// ──────────────────────────────────────────────────────────────
// Drying logic — water-based concentration
// ──────────────────────────────────────────────────────────────

/// Calculate drying concentration factor based on actual water content.
///
/// Principle: when water is removed, the remaining solids concentrate.
/// concentration = (100 - water_before) stays the same mass, but now
/// occupies a larger fraction of the dried weight.
///
/// Example: tomato (94% water) → dried to 10% water:
///   solid_before = 6%, solid_after = 90% → factor = 90/6 = 15×
///   But capped to avoid absurd values.
///
/// Example: walnut (4% water) → dried to 2% water:
///   solid_before = 96%, solid_after = 98% → factor = 98/96 ≈ 1.02×
fn drying_factor(water_percent: f64) -> f64 {
    let water_before = water_percent.clamp(0.0, 99.0);
    // Target water after drying: 8% for very wet, down to (water-2)% for dry
    let water_after = if water_before > 50.0 {
        8.0  // standard dried product moisture
    } else if water_before > 20.0 {
        6.0  // drier target for moderate-water products
    } else {
        // Already quite dry — remove just a couple percent
        (water_before - 2.0).max(1.0)
    };

    let solid_before = 100.0 - water_before;
    let solid_after = 100.0 - water_after;

    if solid_before < 1.0 {
        // Edge case: nearly pure water (e.g. beverage)
        return 1.0;
    }

    let factor = solid_after / solid_before;
    // Cap at 8× to prevent absurd values
    factor.clamp(1.0, 8.0)
}

/// Calculate water_percent after drying
fn dried_water_percent(water_percent: f64) -> f64 {
    let water_before = water_percent.clamp(0.0, 99.0);
    if water_before > 50.0 {
        8.0
    } else if water_before > 20.0 {
        6.0
    } else {
        (water_before - 2.0).max(1.0)
    }
}

// ──────────────────────────────────────────────────────────────
// Post-validation: ensure macros don't exceed physical limits
// ──────────────────────────────────────────────────────────────

/// Validate and normalize nutrition values to ensure physical plausibility.
///
/// Rules:
/// - protein, fat, carbs each ≤ 100g per 100g
/// - protein + fat + carbs ≤ 100g per 100g (solids can't exceed total)
/// - calories recalculated if deviation > 25% from Atwater formula
fn validate_nutrition(mut n: TransformedNutrition) -> TransformedNutrition {
    // 1. Clamp individual macros
    n.protein_per_100g = n.protein_per_100g.min(100.0).max(0.0);
    n.fat_per_100g = n.fat_per_100g.min(100.0).max(0.0);
    n.carbs_per_100g = n.carbs_per_100g.min(100.0).max(0.0);
    n.fiber_per_100g = n.fiber_per_100g.min(100.0).max(0.0);

    // 2. Normalize if sum exceeds 100g (solids can't exceed total mass)
    let macro_sum = n.protein_per_100g + n.fat_per_100g + n.carbs_per_100g;
    if macro_sum > 100.0 {
        let ratio = 100.0 / macro_sum;
        n.protein_per_100g = round2(n.protein_per_100g * ratio);
        n.fat_per_100g = round2(n.fat_per_100g * ratio);
        n.carbs_per_100g = round2(n.carbs_per_100g * ratio);
    }

    // 3. Safety guard: calories ≈ protein*4 + carbs*4 + fat*9
    //    If deviation > 25%, recalculate from Atwater factors
    let atwater = n.protein_per_100g * 4.0 + n.carbs_per_100g * 4.0 + n.fat_per_100g * 9.0;
    if atwater > 0.0 && n.calories_per_100g > 0.0 {
        let deviation = (n.calories_per_100g - atwater).abs() / atwater;
        if deviation > 0.25 {
            n.calories_per_100g = round2(atwater);
        }
    } else if n.calories_per_100g <= 0.0 && atwater > 0.0 {
        n.calories_per_100g = round2(atwater);
    }

    // 4. Clamp water
    n.water_percent = n.water_percent.clamp(0.0, 100.0);

    // Round everything
    n.calories_per_100g = round2(n.calories_per_100g);
    n.protein_per_100g = round2(n.protein_per_100g);
    n.fat_per_100g = round2(n.fat_per_100g);
    n.carbs_per_100g = round2(n.carbs_per_100g);
    n.fiber_per_100g = round2(n.fiber_per_100g);
    n.water_percent = round2(n.water_percent);

    n
}

// ──────────────────────────────────────────────────────────────
// Main transform function
// ──────────────────────────────────────────────────────────────

/// Transform base nutrition values according to product group + processing state.
///
/// For drying: uses water-based concentration instead of fixed multipliers.
/// For frying: uses fat_add (absolute grams) instead of fat multiplier.
/// All results are post-validated for physical plausibility.
pub fn transform_nutrition(
    base: &BaseNutrition,
    group: ProductGroup,
    state: ProcessingState,
) -> TransformedNutrition {
    // Special handling for drying — water-based concentration
    if state == ProcessingState::Dried {
        // For groups that are already dry (NutsSeeds, DryGoods, Spices, OilsFats),
        // we still use the coefficient table (minimal change).
        // For wet groups, we compute dynamically from water content.
        let use_water_based = matches!(
            group,
            ProductGroup::WateryProduce | ProductGroup::DensePlant | ProductGroup::MeatFish | ProductGroup::Other
        );

        if use_water_based {
            let factor = drying_factor(base.water_percent);
            let water_after = dried_water_percent(base.water_percent);

            let raw = TransformedNutrition {
                calories_per_100g: round2(base.calories * factor),
                protein_per_100g: round2(base.protein * factor),
                fat_per_100g: round2(base.fat * factor),
                carbs_per_100g: round2(base.carbs * factor),
                fiber_per_100g: round2(base.fiber * factor),
                water_percent: round2(water_after),
            };
            return validate_nutrition(raw);
        }
    }

    let coeff = get_coefficients(group, state);

    let water = (base.water_percent + coeff.water_delta).clamp(0.0, 100.0);

    // Fat: multiplier + absolute addition (for frying)
    let fat = base.fat * coeff.fat + coeff.fat_add;

    let raw = TransformedNutrition {
        calories_per_100g: round2(base.calories * coeff.calories),
        protein_per_100g: round2(base.protein * coeff.protein),
        fat_per_100g: round2(fat),
        carbs_per_100g: round2(base.carbs * coeff.carbs),
        fiber_per_100g: round2(base.fiber * coeff.fiber),
        water_percent: round2(water),
    };

    validate_nutrition(raw)
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

// ──────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_potato() -> BaseNutrition {
        BaseNutrition {
            calories: 77.0,
            protein: 2.0,
            fat: 0.1,
            carbs: 17.0,
            fiber: 2.2,
            water_percent: 79.0,
        }
    }

    fn sample_walnut() -> BaseNutrition {
        BaseNutrition {
            calories: 654.0,
            protein: 15.2,
            fat: 65.2,
            carbs: 13.7,
            fiber: 6.7,
            water_percent: 4.0,
        }
    }

    fn sample_tomato() -> BaseNutrition {
        BaseNutrition {
            calories: 18.0,
            protein: 0.9,
            fat: 0.2,
            carbs: 3.9,
            fiber: 1.2,
            water_percent: 94.0,
        }
    }

    fn sample_salmon() -> BaseNutrition {
        BaseNutrition {
            calories: 208.0,
            protein: 20.4,
            fat: 13.4,
            carbs: 0.0,
            fiber: 0.0,
            water_percent: 64.0,
        }
    }

    fn sample_rice() -> BaseNutrition {
        BaseNutrition {
            calories: 360.0,
            protein: 6.6,
            fat: 0.6,
            carbs: 80.0,
            fiber: 1.3,
            water_percent: 12.0,
        }
    }

    // ── Product group classification ──

    #[test]
    fn classify_groups_correctly() {
        assert_eq!(classify_group("fruit"), ProductGroup::WateryProduce);
        assert_eq!(classify_group("vegetable"), ProductGroup::WateryProduce);
        assert_eq!(classify_group("nut"), ProductGroup::NutsSeeds);
        assert_eq!(classify_group("fish"), ProductGroup::MeatFish);
        assert_eq!(classify_group("grain"), ProductGroup::DryGoods);
        assert_eq!(classify_group("oil"), ProductGroup::OilsFats);
        assert_eq!(classify_group("spice"), ProductGroup::Spices);
        assert_eq!(classify_group("dairy"), ProductGroup::Other);
        assert_eq!(classify_group("unknown_type"), ProductGroup::Other);
    }

    // ── Raw is always unchanged ──

    #[test]
    fn raw_is_unchanged() {
        let base = sample_potato();
        let result = transform_nutrition(&base, ProductGroup::DensePlant, ProcessingState::Raw);
        assert_eq!(result.calories_per_100g, 77.0);
        assert_eq!(result.protein_per_100g, 2.0);
    }

    // ── Frozen is always unchanged ──

    #[test]
    fn frozen_is_mostly_unchanged() {
        let base = sample_potato();
        let result = transform_nutrition(&base, ProductGroup::DensePlant, ProcessingState::Frozen);
        assert_eq!(result.calories_per_100g, 77.0);
        assert_eq!(result.protein_per_100g, 2.0);
    }

    // ── Potato fried: fat should increase but stay reasonable ──

    #[test]
    fn potato_fried_reasonable_fat() {
        let base = sample_potato();
        let result = transform_nutrition(&base, ProductGroup::DensePlant, ProcessingState::Fried);
        // Potato: 0.1g fat + 10g absorbed oil = ~10.1g fat
        assert!(result.fat_per_100g > 5.0, "fried potato should gain fat");
        assert!(result.fat_per_100g < 20.0, "fried potato fat should be < 20g, got {}", result.fat_per_100g);
        assert!(result.calories_per_100g > base.calories, "frying increases calories");
    }

    // ── Walnut dried: should barely change (already 4% water) ──

    #[test]
    fn walnut_dried_barely_changes() {
        let base = sample_walnut();
        let result = transform_nutrition(&base, ProductGroup::NutsSeeds, ProcessingState::Dried);
        // Should be very close to raw values
        assert!(result.fat_per_100g < 70.0, "walnut dried fat should be < 70g, got {}", result.fat_per_100g);
        assert!(result.fat_per_100g > 60.0, "walnut dried fat should be > 60g, got {}", result.fat_per_100g);
        assert!(result.calories_per_100g < 700.0, "walnut dried cal < 700, got {}", result.calories_per_100g);
    }

    // ── Tomato dried: should concentrate a lot (94% water → ~8%) ──

    #[test]
    fn tomato_dried_concentrates_properly() {
        let base = sample_tomato();
        let result = transform_nutrition(&base, ProductGroup::WateryProduce, ProcessingState::Dried);
        // factor = (100-8)/(100-94) = 92/6 ≈ 15.3, but capped at 8×
        assert!(result.calories_per_100g > base.calories * 5.0, "tomato dried should concentrate a lot");
        assert!(result.calories_per_100g <= base.calories * 8.5, "tomato dried capped at 8×, got {}", result.calories_per_100g);
        assert!(result.water_percent < 10.0, "dried tomato should be < 10% water");
        // Macros should not exceed 100
        assert!(result.protein_per_100g + result.fat_per_100g + result.carbs_per_100g <= 100.0,
            "macro sum should be ≤ 100g");
    }

    // ── Salmon grilled: reasonable protein concentration ──

    #[test]
    fn salmon_grilled_reasonable() {
        let base = sample_salmon();
        let result = transform_nutrition(&base, ProductGroup::MeatFish, ProcessingState::Grilled);
        assert!(result.protein_per_100g > base.protein, "grilled salmon should have more protein per 100g");
        assert!(result.fat_per_100g < base.fat, "grilled salmon should lose some fat");
        assert!(result.protein_per_100g < 35.0, "grilled salmon protein < 35g, got {}", result.protein_per_100g);
    }

    // ── Rice boiled: should dilute significantly ──

    #[test]
    fn rice_boiled_dilutes() {
        let base = sample_rice();
        let result = transform_nutrition(&base, ProductGroup::DryGoods, ProcessingState::Boiled);
        // Cooked rice is ~130 kcal/100g vs 360 raw
        assert!(result.calories_per_100g < 200.0, "boiled rice should be < 200 kcal, got {}", result.calories_per_100g);
        assert!(result.calories_per_100g > 100.0, "boiled rice should be > 100 kcal, got {}", result.calories_per_100g);
        assert!(result.water_percent > 60.0, "boiled rice should have high water");
    }

    // ── Validation: macros can't exceed 100g total ──

    #[test]
    fn validate_caps_macros() {
        let extreme = TransformedNutrition {
            calories_per_100g: 900.0,
            protein_per_100g: 50.0,
            fat_per_100g: 40.0,
            carbs_per_100g: 30.0,
            fiber_per_100g: 5.0,
            water_percent: 10.0,
        };
        let v = validate_nutrition(extreme);
        assert!(v.protein_per_100g + v.fat_per_100g + v.carbs_per_100g <= 100.01,
            "macro sum should be ≤ 100g after validation");
    }

    // ── Drying factor calculations ──

    #[test]
    fn drying_factor_tomato() {
        // 94% water → 8% target → factor = 92/6 = 15.3, but capped at 8
        let f = drying_factor(94.0);
        assert_eq!(f, 8.0, "tomato drying capped at 8×");
    }

    #[test]
    fn drying_factor_walnut() {
        // 4% water → target (4-2)=2% → factor = 98/96 ≈ 1.02
        let f = drying_factor(4.0);
        assert!(f < 1.05, "walnut drying factor should be ~1.02, got {}", f);
    }

    #[test]
    fn drying_factor_potato() {
        // 79% water → 8% target → factor = 92/21 ≈ 4.38
        let f = drying_factor(79.0);
        assert!(f > 4.0 && f < 5.0, "potato drying factor should be ~4.4, got {}", f);
    }

    // ── Calorie safety guard ──

    #[test]
    fn calorie_safety_guard_recalculates() {
        let bad = TransformedNutrition {
            calories_per_100g: 1000.0, // way too high
            protein_per_100g: 10.0,
            fat_per_100g: 5.0,
            carbs_per_100g: 20.0,
            fiber_per_100g: 3.0,
            water_percent: 60.0,
        };
        let v = validate_nutrition(bad);
        // Atwater: 10*4 + 20*4 + 5*9 = 40+80+45 = 165
        // 1000 vs 165 → deviation ~5× → should recalculate
        assert!(v.calories_per_100g < 200.0, "calories should be recalculated to ~165, got {}", v.calories_per_100g);
    }
}
