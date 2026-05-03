//! Inventory layout engine — decides positions, sizes, zone placement.
//!
//! Pure functions, no I/O. Returns a `LayoutPlan` that the entity factory
//! consumes. Keep `inventory_prefabs.rs` / `inventory_materials.rs` etc.
//! free of any positioning concerns — they only translate themes.

use crate::application::inventory::InventoryView;
use crate::domain::scene::MaterialTheme;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ZoneKey {
    Cold,
    Dry,
    Freezer,
    Risk,
}

impl ZoneKey {
    pub const ORDER: [ZoneKey; 4] = [Self::Cold, Self::Dry, Self::Freezer, Self::Risk];

    pub fn id(self) -> &'static str {
        match self {
            Self::Cold => "cold",
            Self::Dry => "dry",
            Self::Freezer => "freezer",
            Self::Risk => "risk",
        }
    }

    pub fn meta(self) -> ZoneMeta {
        match self {
            Self::Cold => ZoneMeta {
                label: "Cold Storage",
                subtitle: "0–4°C",
                theme: MaterialTheme::Cold,
                center: [-6.2, 0.0, 3.5],
            },
            Self::Dry => ZoneMeta {
                label: "Dry Storage",
                subtitle: "15–20°C",
                theme: MaterialTheme::Dry,
                center: [6.2, 0.0, 3.5],
            },
            Self::Freezer => ZoneMeta {
                label: "Freezer",
                subtitle: "-18°C",
                theme: MaterialTheme::Freezer,
                center: [-6.2, 0.0, -3.5],
            },
            Self::Risk => ZoneMeta {
                label: "⚠ Risk Zone",
                subtitle: "Attention required",
                theme: MaterialTheme::Risk,
                center: [6.2, 0.0, -3.5],
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ZoneMeta {
    pub label: &'static str,
    pub subtitle: &'static str,
    pub theme: MaterialTheme,
    pub center: [f32; 3],
}

/// Per-zone room footprint (used by the storage-room geometry).
pub const ROOM_SIZE: [f32; 3] = [9.5, 0.35, 5.0];
pub const ROOM_WALL_HEIGHT: f32 = 0.85;
pub const ROOM_WALL_THICKNESS: f32 = 0.10;
pub const ROOM_CORNER_RADIUS: f32 = 0.22;

/// Y-level of the inner floor surface inside a storage room.
/// Must match frontend: baseH(0.28) + inner floor thickness(0.05) = 0.33
pub const ROOM_FLOOR_Y: f32 = 0.33;

/// Card grid configuration inside a zone.
/// Cartridge size: W=1.15, D=0.18 → step X=1.35, Z=0.40 (tight slot grid).
pub const CARD_COLS: usize = 5;
pub const CARD_STEP_X: f32 = 1.35;
pub const CARD_STEP_Z: f32 = 0.42;

/// Resolved layout for a single product card.
#[derive(Debug, Clone, Copy)]
pub struct CardSlot {
    pub zone: ZoneKey,
    pub index_in_zone: usize,
    pub position: [f32; 3],
    pub theme: MaterialTheme,
}

/// Decide which zone an item lives in. Risk items override category.
pub fn infer_zone(item: &InventoryView, theme: MaterialTheme) -> ZoneKey {
    if matches!(theme, MaterialTheme::Expired | MaterialTheme::Critical) {
        return ZoneKey::Risk;
    }
    let c = item.product.category.to_lowercase();
    if contains_any(&c, &["frozen", "мороз", "ice"]) {
        return ZoneKey::Freezer;
    }
    if contains_any(
        &c,
        &[
            "meat", "fish", "dairy", "cheese", "мяс", "рыб", "молоч", "сыр", "яйц", "egg",
        ],
    ) {
        return ZoneKey::Cold;
    }
    ZoneKey::Dry
}

pub fn product_position_in_zone(zone: ZoneKey, index_in_zone: usize) -> [f32; 3] {
    let base = zone.meta().center;
    let col = (index_in_zone % CARD_COLS) as f32;
    let row = (index_in_zone / CARD_COLS) as f32;
    [
        base[0] + (col - (CARD_COLS as f32 - 1.0) / 2.0) * CARD_STEP_X,
        ROOM_FLOOR_Y,   // ← sit ON the inner floor, not floating at y=0
        base[2] + (row - 1.0) * CARD_STEP_Z,
    ]
}

pub(crate) fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| haystack.contains(n))
}
