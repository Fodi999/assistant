//! Inventory prefab dispatch — maps zones / categories to stable
//! `PrefabKey`s the frontend recognises.

use super::inventory_layout::{contains_any, ZoneKey};
use crate::domain::scene::PrefabKey;

pub fn zone_prefab(zone: ZoneKey) -> PrefabKey {
    match zone {
        ZoneKey::Cold => PrefabKey::GlassFridgeRoom,
        ZoneKey::Dry => PrefabKey::DryStorageRoom,
        ZoneKey::Freezer => PrefabKey::FreezerRoom,
        ZoneKey::Risk => PrefabKey::RiskRoom,
    }
}

pub fn product_prefab() -> PrefabKey {
    // For now every product uses the same card prefab. Later we can branch
    // on category (e.g. `bottlePrefab` for oils, `trayPrefab` for bakery).
    PrefabKey::GlassProductCard
}

/// Frontend `assetRegistry` lookup key — picks the photo / 3D model.
pub fn infer_asset_key(category: &str) -> &'static str {
    let c = category.to_lowercase();
    if contains_any(&c, &["egg", "яйц"]) {
        return "egg";
    }
    if contains_any(&c, &["meat", "chicken", "beef", "pork", "мяс", "курин"]) {
        return "meat";
    }
    if contains_any(&c, &["fish", "рыб", "seafood"]) {
        return "fish";
    }
    if contains_any(&c, &["dairy", "cheese", "milk", "молоч", "сыр"]) {
        return "dairy";
    }
    if contains_any(&c, &["veg", "tomato", "salad", "зелен", "овощ"]) {
        return "vegetable";
    }
    if contains_any(&c, &["fruit", "apple", "berry", "фрукт", "ягод"]) {
        return "fruit";
    }
    if contains_any(&c, &["grain", "rice", "pasta", "flour", "круп", "мук"]) {
        return "grain";
    }
    if contains_any(&c, &["spice", "herb", "special", "спец"]) {
        return "spice";
    }
    if contains_any(&c, &["oil", "масл"]) {
        return "oil";
    }
    if contains_any(&c, &["sauce", "соус"]) {
        return "sauce";
    }
    if contains_any(&c, &["frozen", "мороз", "ice"]) {
        return "frozen";
    }
    if contains_any(&c, &["drink", "water", "juice", "напит", "сок"]) {
        return "drink";
    }
    "generic"
}

/// Emoji fallback when no `image_url` is present on the product.
pub fn category_emoji(category: &str) -> &'static str {
    let c = category.to_lowercase();
    if contains_any(&c, &["meat", "chicken", "beef", "pork", "мяс", "курин"]) {
        return "🥩";
    }
    if contains_any(&c, &["fish", "рыб", "seafood"]) {
        return "🐟";
    }
    if contains_any(&c, &["dairy", "cheese", "milk", "молоч", "сыр"]) {
        return "🧀";
    }
    if contains_any(&c, &["egg", "яйц"]) {
        return "🥚";
    }
    if contains_any(&c, &["veg", "tomato", "salad", "зелен", "овощ"]) {
        return "🥦";
    }
    if contains_any(&c, &["fruit", "apple", "berry", "фрукт", "ягод"]) {
        return "🍎";
    }
    if contains_any(&c, &["grain", "rice", "pasta", "flour", "круп", "мук"]) {
        return "🌾";
    }
    if contains_any(&c, &["spice", "herb", "special", "спец"]) {
        return "🌿";
    }
    if contains_any(&c, &["oil", "масл"]) {
        return "🫙";
    }
    if contains_any(&c, &["sauce", "соус"]) {
        return "🥫";
    }
    if contains_any(&c, &["drink", "water", "juice", "напит", "сок"]) {
        return "🧃";
    }
    if contains_any(&c, &["frozen", "мороз", "ice"]) {
        return "🧊";
    }
    "📦"
}
