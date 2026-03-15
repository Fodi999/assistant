use crate::domain::ProcessingState;

/// Storage requirements for a given processing state
#[derive(Debug, Clone)]
pub struct StorageRule {
    pub shelf_life_hours: i32,
    pub storage_temp_c: i32,
    pub texture: &'static str,
    /// Weight change during processing (negative = loss, positive = gain)
    pub weight_change_percent: f64,
    /// Classification: "raw", "heat", or "preserved"
    pub state_type: &'static str,
    /// Oil absorbed per 100g raw product (mainly fried)
    pub oil_absorption_g: f64,
    /// Water lost during processing (0–100%)
    pub water_loss_percent: f64,
    /// Specific cooking method (cooking_method_enum)
    pub cooking_method: &'static str,
}

/// Whether a processing state is applicable for a given product type.
/// Returns: "normal", "rare" (uncommon but possible), or "skip" (should not generate).
pub fn state_applicability(product_type: &str, state: ProcessingState) -> &'static str {
    match (product_type, state) {
        // Nuts & seeds: boiling and steaming are very rare
        ("nut", ProcessingState::Boiled) => "rare",
        ("nut", ProcessingState::Steamed) => "rare",
        ("nut", ProcessingState::Pickled) => "rare",
        ("seed", ProcessingState::Boiled) => "rare",
        ("seed", ProcessingState::Steamed) => "rare",

        // Oils: most cooking methods don't apply
        ("oil", ProcessingState::Boiled) => "skip",
        ("oil", ProcessingState::Steamed) => "skip",
        ("oil", ProcessingState::Grilled) => "skip",
        ("oil", ProcessingState::Baked) => "skip",
        ("oil", ProcessingState::Smoked) => "skip",
        ("oil", ProcessingState::Dried) => "skip",
        ("oil", ProcessingState::Pickled) => "skip",

        // Butter/fat: limited methods
        ("butter", ProcessingState::Grilled) => "skip",
        ("butter", ProcessingState::Smoked) => "skip",
        ("butter", ProcessingState::Dried) => "skip",
        ("butter", ProcessingState::Pickled) => "skip",
        ("fat", ProcessingState::Grilled) => "skip",
        ("fat", ProcessingState::Smoked) => "skip",
        ("fat", ProcessingState::Dried) => "skip",
        ("fat", ProcessingState::Pickled) => "skip",

        // Spices: most heat methods are rare
        ("spice", ProcessingState::Boiled) => "rare",
        ("spice", ProcessingState::Steamed) => "rare",
        ("spice", ProcessingState::Grilled) => "rare",
        ("spice", ProcessingState::Smoked) => "rare",
        ("spice", ProcessingState::Pickled) => "rare",

        // Beverages: only frozen makes sense
        ("beverage", ProcessingState::Fried) => "skip",
        ("beverage", ProcessingState::Baked) => "skip",
        ("beverage", ProcessingState::Grilled) => "skip",
        ("beverage", ProcessingState::Smoked) => "skip",
        ("beverage", ProcessingState::Dried) => "skip",

        // Dairy: smoking and grilling are rare
        ("dairy", ProcessingState::Smoked) => "rare",
        ("dairy", ProcessingState::Grilled) => "rare",
        ("dairy", ProcessingState::Dried) => "rare",

        // Everything else is normal
        _ => "normal",
    }
}

/// Get storage rules for a given processing state.
///
/// These are general defaults — specific products may override.
/// Based on food safety guidelines (HACCP, FDA, EU regulations).
pub fn get_storage_rule(state: ProcessingState) -> StorageRule {
    match state {
        ProcessingState::Raw => StorageRule {
            shelf_life_hours: 72,    // 3 days typical for fresh produce
            storage_temp_c: 4,       // standard fridge
            texture: "natural",
            weight_change_percent: 0.0,
            state_type: "raw",
            oil_absorption_g: 0.0,
            water_loss_percent: 0.0,
            cooking_method: "raw",
        },
        ProcessingState::Boiled => StorageRule {
            shelf_life_hours: 48,    // 2 days in fridge
            storage_temp_c: 4,
            texture: "soft",
            weight_change_percent: 5.0,   // absorbs water → gains ~5%
            state_type: "heat",
            oil_absorption_g: 0.0,
            water_loss_percent: 0.0,      // actually gains water
            cooking_method: "boiled",
        },
        ProcessingState::Fried => StorageRule {
            shelf_life_hours: 24,    // consume same day ideally
            storage_temp_c: 4,
            texture: "crispy",
            weight_change_percent: -25.0, // loses water, absorbs oil → net loss ~25%
            state_type: "heat",
            oil_absorption_g: 10.0,       // ~10g oil per 100g product
            water_loss_percent: 30.0,
            cooking_method: "pan_fried",
        },
        ProcessingState::Baked => StorageRule {
            shelf_life_hours: 48,
            storage_temp_c: 4,
            texture: "firm",
            weight_change_percent: -15.0, // moderate water loss
            state_type: "heat",
            oil_absorption_g: 0.0,
            water_loss_percent: 15.0,
            cooking_method: "baked",
        },
        ProcessingState::Grilled => StorageRule {
            shelf_life_hours: 24,
            storage_temp_c: 4,
            texture: "charred",
            weight_change_percent: -20.0, // fat drips + water evaporates
            state_type: "heat",
            oil_absorption_g: 0.0,
            water_loss_percent: 18.0,
            cooking_method: "grilled",
        },
        ProcessingState::Steamed => StorageRule {
            shelf_life_hours: 48,
            storage_temp_c: 4,
            texture: "tender",
            weight_change_percent: -3.0,  // minimal loss
            state_type: "heat",
            oil_absorption_g: 0.0,
            water_loss_percent: 3.0,
            cooking_method: "steamed",
        },
        ProcessingState::Smoked => StorageRule {
            shelf_life_hours: 168,   // 7 days — smoking preserves
            storage_temp_c: 4,
            texture: "firm-smoky",
            weight_change_percent: -30.0, // significant water loss
            state_type: "preserved",
            oil_absorption_g: 0.0,
            water_loss_percent: 25.0,
            cooking_method: "smoked",
        },
        ProcessingState::Frozen => StorageRule {
            shelf_life_hours: 2160,  // 90 days
            storage_temp_c: -18,     // standard freezer
            texture: "frozen",
            weight_change_percent: 0.0,   // no change
            state_type: "preserved",
            oil_absorption_g: 0.0,
            water_loss_percent: 0.0,
            cooking_method: "frozen",
        },
        ProcessingState::Dried => StorageRule {
            shelf_life_hours: 4320,  // 180 days
            storage_temp_c: 20,      // room temperature OK
            texture: "brittle",
            weight_change_percent: -70.0, // extreme water loss
            state_type: "preserved",
            oil_absorption_g: 0.0,
            water_loss_percent: 70.0,
            cooking_method: "dried",
        },
        ProcessingState::Pickled => StorageRule {
            shelf_life_hours: 720,   // 30 days
            storage_temp_c: 4,
            texture: "crunchy-sour",
            weight_change_percent: 5.0,   // absorbs brine
            state_type: "preserved",
            oil_absorption_g: 0.0,
            water_loss_percent: 0.0,
            cooking_method: "pickled",
        },
    }
}

/// Product-type specific overrides for shelf life.
/// Some products have different storage needs.
pub fn override_shelf_life(product_type: &str, state: ProcessingState) -> Option<i32> {
    match (product_type, state) {
        // Fish & seafood — shorter shelf life
        ("fish", ProcessingState::Raw) => Some(24),      // 1 day
        ("fish", ProcessingState::Boiled) => Some(24),
        ("fish", ProcessingState::Fried) => Some(12),
        ("seafood", ProcessingState::Raw) => Some(24),
        ("seafood", ProcessingState::Boiled) => Some(24),

        // Meat — moderate
        ("meat", ProcessingState::Raw) => Some(48),       // 2 days
        ("meat", ProcessingState::Boiled) => Some(48),
        ("meat", ProcessingState::Smoked) => Some(336),   // 14 days

        // Dairy — short
        ("dairy", ProcessingState::Raw) => Some(120),     // 5 days
        ("dairy", ProcessingState::Baked) => Some(72),

        // Fruit — varies
        ("fruit", ProcessingState::Raw) => Some(120),     // 5 days
        ("fruit", ProcessingState::Dried) => Some(8640),  // 360 days

        // Vegetables — moderate
        ("vegetable", ProcessingState::Raw) => Some(168), // 7 days
        ("vegetable", ProcessingState::Frozen) => Some(4320), // 180 days

        // Grains — long shelf life
        ("grain", ProcessingState::Raw) => Some(8640),    // 360 days
        ("grain", ProcessingState::Boiled) => Some(72),

        // Nuts — very long
        ("nut", ProcessingState::Raw) => Some(4320),      // 180 days

        // Spices — very long
        ("spice", ProcessingState::Raw) => Some(17520),   // 2 years
        ("spice", ProcessingState::Dried) => Some(17520),

        _ => None, // use default from get_storage_rule
    }
}
