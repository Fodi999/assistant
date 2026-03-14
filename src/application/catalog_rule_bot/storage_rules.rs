use crate::domain::ProcessingState;

/// Storage requirements for a given processing state
#[derive(Debug, Clone)]
pub struct StorageRule {
    pub shelf_life_hours: i32,
    pub storage_temp_c: i32,
    pub texture: &'static str,
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
        },
        ProcessingState::Boiled => StorageRule {
            shelf_life_hours: 48,    // 2 days in fridge
            storage_temp_c: 4,
            texture: "soft",
        },
        ProcessingState::Fried => StorageRule {
            shelf_life_hours: 24,    // consume same day ideally
            storage_temp_c: 4,
            texture: "crispy",
        },
        ProcessingState::Baked => StorageRule {
            shelf_life_hours: 48,
            storage_temp_c: 4,
            texture: "firm",
        },
        ProcessingState::Grilled => StorageRule {
            shelf_life_hours: 24,
            storage_temp_c: 4,
            texture: "charred",
        },
        ProcessingState::Steamed => StorageRule {
            shelf_life_hours: 48,
            storage_temp_c: 4,
            texture: "tender",
        },
        ProcessingState::Smoked => StorageRule {
            shelf_life_hours: 168,   // 7 days — smoking preserves
            storage_temp_c: 4,
            texture: "firm-smoky",
        },
        ProcessingState::Frozen => StorageRule {
            shelf_life_hours: 2160,  // 90 days
            storage_temp_c: -18,     // standard freezer
            texture: "frozen",
        },
        ProcessingState::Dried => StorageRule {
            shelf_life_hours: 4320,  // 180 days
            storage_temp_c: 20,      // room temperature OK
            texture: "brittle",
        },
        ProcessingState::Pickled => StorageRule {
            shelf_life_hours: 720,   // 30 days
            storage_temp_c: 4,
            texture: "crunchy-sour",
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
