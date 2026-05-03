//! Inventory material configuration — maps severity / category to the
//! visual `EntityMaterial` the renderer reads.

use super::inventory_layout::{contains_any, ZoneKey};
use crate::domain::inventory::ExpirationSeverity;
use crate::domain::scene::{EntityMaterial, MaterialTheme};

pub fn severity_to_theme(s: ExpirationSeverity) -> MaterialTheme {
    match s {
        ExpirationSeverity::Expired => MaterialTheme::Expired,
        ExpirationSeverity::Critical => MaterialTheme::Critical,
        ExpirationSeverity::Warning => MaterialTheme::Warning,
        ExpirationSeverity::Ok | ExpirationSeverity::NoExpiration => MaterialTheme::Ok,
    }
}

pub fn emissive_for_theme(theme: MaterialTheme) -> f32 {
    match theme {
        MaterialTheme::Ok => 0.10,
        MaterialTheme::Warning => 0.28,
        MaterialTheme::Critical => 0.40,
        MaterialTheme::Expired => 0.55,
        _ => 0.12,
    }
}

/// Per-category accent color (overrides severity tint for the card).
/// Risk severities keep red so warnings stay loud.
pub fn category_color(category: &str) -> Option<&'static str> {
    let c = category.to_lowercase();
    if contains_any(&c, &["meat", "chicken", "beef", "pork", "мяс", "курин"]) {
        return Some("#f87171");
    }
    if contains_any(&c, &["fish", "рыб", "seafood"]) {
        return Some("#38bdf8");
    }
    if contains_any(&c, &["dairy", "cheese", "milk", "молоч", "сыр"]) {
        return Some("#a78bfa");
    }
    if contains_any(&c, &["egg", "яйц"]) {
        return Some("#fde68a");
    }
    if contains_any(&c, &["veg", "tomato", "salad", "зелен", "овощ"]) {
        return Some("#4ade80");
    }
    if contains_any(&c, &["fruit", "apple", "berry", "фрукт", "ягод"]) {
        return Some("#fb923c");
    }
    if contains_any(&c, &["grain", "rice", "pasta", "flour", "круп", "мук"]) {
        return Some("#d4a574");
    }
    if contains_any(&c, &["spice", "herb", "special", "спец"]) {
        return Some("#f472b6");
    }
    if contains_any(&c, &["oil", "масл"]) {
        return Some("#facc15");
    }
    if contains_any(&c, &["drink", "water", "juice", "напит", "сок"]) {
        return Some("#67e8f9");
    }
    if contains_any(&c, &["frozen", "мороз", "ice"]) {
        return Some("#93c5fd");
    }
    None
}

/// Build the storage-zone material — glass-like for cold/freezer, matte
/// for dry, danger-tinted for risk.
pub fn zone_material(zone: ZoneKey) -> EntityMaterial {
    match zone {
        ZoneKey::Cold => EntityMaterial::new(MaterialTheme::Cold)
            .with_accent("#38bdf8")
            .with_emissive(0.18)
            .with_glass(0.32, 0.25, 0.55),
        ZoneKey::Freezer => EntityMaterial::new(MaterialTheme::Freezer)
            .with_accent("#bae6fd")
            .with_emissive(0.22)
            .with_glass(0.4, 0.2, 0.6),
        ZoneKey::Dry => EntityMaterial::new(MaterialTheme::Dry)
            .with_accent("#f59e0b")
            .with_emissive(0.16),
        ZoneKey::Risk => EntityMaterial::new(MaterialTheme::Risk)
            .with_accent("#ef4444")
            .with_emissive(0.30),
    }
}

/// Build the product-card material from severity + category.
pub fn product_material(theme: MaterialTheme, category: &str) -> EntityMaterial {
    // For expired / critical we keep the red severity tint to keep risk
    // visually loud; otherwise the per-category accent wins.
    let accent = if matches!(theme, MaterialTheme::Expired | MaterialTheme::Critical) {
        None
    } else {
        category_color(category)
    };
    let mut mat = EntityMaterial::new(theme).with_emissive(emissive_for_theme(theme));
    if let Some(c) = accent {
        mat = mat.with_accent(c);
    }
    mat
}
