//! Inventory scene builder — port of
//! `blog/components/visual/builders/inventorySceneBuilder.ts`.
//!
//! Responsibilities (do NOT leak any of these to the frontend):
//!   • severity → MaterialTheme mapping
//!   • storage zone derivation (cold/dry/freezer/risk) from category
//!   • per-zone product layout (4-column grid)
//!   • emissive level per severity
//!   • allowed `EntityAction`s per severity
//!   • HUD strings (totals, expiring count, low-stock count) — pre-formatted
//!
//! The frontend just calls `GET /api/scenes/inventory` and renders.

use std::sync::Arc;
use time::OffsetDateTime;

use crate::application::inventory::{InventoryService, InventoryView};
use crate::domain::inventory::ExpirationSeverity;
use crate::domain::scene::{
    CameraPreset, DomainKind, EntityAction, EntityContent, EntityDataRef, EntityGameplay,
    EntityGeometry, EntityMaterial, EntityType, GeometryKind, MaterialTheme, SceneCamera,
    SceneEntity, SceneHud, SceneMode, SceneState, Transform,
};
use crate::shared::{AppResult, Language, TenantId, UserId};

#[derive(Clone)]
pub struct InventorySceneService {
    inventory: InventoryService,
}

impl InventorySceneService {
    pub fn new(inventory: InventoryService) -> Self {
        Self { inventory }
    }

    pub fn shared(inventory: InventoryService) -> Arc<Self> {
        Arc::new(Self::new(inventory))
    }

    /// Build a full SceneState snapshot for the given tenant.
    pub async fn build_scene(
        &self,
        user_id: UserId,
        tenant_id: TenantId,
        language: Language,
        selected_entity_id: Option<String>,
    ) -> AppResult<SceneState> {
        let items = self
            .inventory
            .list_products_with_details(user_id, tenant_id, language)
            .await?;
        Ok(build_scene_from_items(&items, selected_entity_id))
    }
}

// ── Pure builder (no DB) — easy to unit test ────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum ZoneKey {
    Cold,
    Dry,
    Freezer,
    Risk,
}

impl ZoneKey {
    const ORDER: [ZoneKey; 4] = [Self::Cold, Self::Dry, Self::Freezer, Self::Risk];

    fn id(self) -> &'static str {
        match self {
            Self::Cold => "cold",
            Self::Dry => "dry",
            Self::Freezer => "freezer",
            Self::Risk => "risk",
        }
    }

    fn meta(self) -> ZoneMeta {
        match self {
            Self::Cold => ZoneMeta {
                label: "Cold Storage",
                subtitle: "0–4°C",
                theme: MaterialTheme::Cold,
                pos: [-6.2, 0.0, 3.5],
            },
            Self::Dry => ZoneMeta {
                label: "Dry Storage",
                subtitle: "15–20°C",
                theme: MaterialTheme::Dry,
                pos: [6.2, 0.0, 3.5],
            },
            Self::Freezer => ZoneMeta {
                label: "Freezer",
                subtitle: "-18°C",
                theme: MaterialTheme::Freezer,
                pos: [-6.2, 0.0, -3.5],
            },
            Self::Risk => ZoneMeta {
                label: "⚠ Risk Zone",
                subtitle: "Attention required",
                theme: MaterialTheme::Risk,
                pos: [6.2, 0.0, -3.5],
            },
        }
    }
}

struct ZoneMeta {
    label: &'static str,
    subtitle: &'static str,
    theme: MaterialTheme,
    pos: [f32; 3],
}

fn severity_to_theme(s: ExpirationSeverity) -> MaterialTheme {
    match s {
        ExpirationSeverity::Expired => MaterialTheme::Expired,
        ExpirationSeverity::Critical => MaterialTheme::Critical,
        ExpirationSeverity::Warning => MaterialTheme::Warning,
        ExpirationSeverity::Ok | ExpirationSeverity::NoExpiration => MaterialTheme::Ok,
    }
}

fn infer_zone(item: &InventoryView, theme: MaterialTheme) -> ZoneKey {
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

fn infer_asset_key(category: &str) -> &'static str {
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

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| haystack.contains(n))
}

/// Per-category accent color (overrides the severity theme for the card tint).
/// This makes cards visually distinct by product type even within the same zone.
fn category_color(category: &str) -> Option<&'static str> {
    let c = category.to_lowercase();
    if contains_any(&c, &["meat", "chicken", "beef", "pork", "мяс", "курин"]) {
        return Some("#f87171"); // red-400
    }
    if contains_any(&c, &["fish", "рыб", "seafood"]) {
        return Some("#38bdf8"); // sky-400
    }
    if contains_any(&c, &["dairy", "cheese", "milk", "молоч", "сыр"]) {
        return Some("#a78bfa"); // violet-400
    }
    if contains_any(&c, &["egg", "яйц"]) {
        return Some("#fde68a"); // amber-200
    }
    if contains_any(&c, &["veg", "tomato", "salad", "зелен", "овощ"]) {
        return Some("#4ade80"); // green-400
    }
    if contains_any(&c, &["fruit", "apple", "berry", "фрукт", "ягод"]) {
        return Some("#fb923c"); // orange-400
    }
    if contains_any(&c, &["grain", "rice", "pasta", "flour", "круп", "мук"]) {
        return Some("#d4a574"); // tan
    }
    if contains_any(&c, &["spice", "herb", "special", "спец"]) {
        return Some("#f472b6"); // pink-400
    }
    if contains_any(&c, &["oil", "масл"]) {
        return Some("#facc15"); // yellow-400
    }
    if contains_any(&c, &["drink", "water", "juice", "напит", "сок"]) {
        return Some("#67e8f9"); // cyan-300
    }
    if contains_any(&c, &["frozen", "мороз", "ice"]) {
        return Some("#93c5fd"); // blue-300
    }
    None
}

/// Category → emoji fallback icon.
fn category_emoji(category: &str) -> &'static str {
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

/// Format the expiry countdown as a short human-readable string.
fn expiry_label(expires_at: OffsetDateTime) -> String {
    let now = OffsetDateTime::now_utc();
    let diff = expires_at - now;
    let days = diff.whole_days();
    if days < 0 {
        return "EXPIRED".to_string();
    }
    if days == 0 {
        let hours = diff.whole_hours();
        if hours <= 0 {
            return "EXPIRED".to_string();
        }
        return format!("{}h left", hours);
    }
    if days == 1 {
        return "tomorrow".to_string();
    }
    format!("{}d left", days)
}

fn product_position_in_zone(zone: ZoneKey, index_in_zone: usize) -> [f32; 3] {
    const COLS: usize = 5;
    const STEP_X: f32 = 1.6;
    const STEP_Z: f32 = 1.3;
    let base = zone.meta().pos;
    let col = (index_in_zone % COLS) as f32;
    let row = (index_in_zone / COLS) as f32;
    [
        base[0] + (col - (COLS as f32 - 1.0) / 2.0) * STEP_X,
        base[1],
        base[2] + (row - 1.0) * STEP_Z,
    ]
}

fn actions_for_theme(theme: MaterialTheme) -> Vec<EntityAction> {
    match theme {
        MaterialTheme::Expired | MaterialTheme::Critical => {
            vec![EntityAction::WriteOff, EntityAction::OpenDetails]
        }
        MaterialTheme::Warning => vec![
            EntityAction::UseToday,
            EntityAction::WriteOff,
            EntityAction::OpenDetails,
        ],
        _ => vec![
            EntityAction::UseToday,
            EntityAction::OpenDetails,
            EntityAction::WriteOff,
        ],
    }
}

fn emissive_for_theme(theme: MaterialTheme) -> f32 {
    match theme {
        MaterialTheme::Ok => 0.10,
        MaterialTheme::Warning => 0.28,
        MaterialTheme::Critical => 0.40,
        MaterialTheme::Expired => 0.55,
        _ => 0.12,
    }
}

/// Pure scene builder. Public for unit tests.
pub fn build_scene_from_items(
    items: &[InventoryView],
    selected_entity_id: Option<String>,
) -> SceneState {
    // Bucket by zone so per-zone indices are stable.
    let mut buckets: [(ZoneKey, Vec<usize>); 4] = [
        (ZoneKey::Cold, Vec::new()),
        (ZoneKey::Dry, Vec::new()),
        (ZoneKey::Freezer, Vec::new()),
        (ZoneKey::Risk, Vec::new()),
    ];

    let mut item_meta: Vec<(MaterialTheme, ZoneKey, usize)> = Vec::with_capacity(items.len());

    for (i, item) in items.iter().enumerate() {
        let theme = severity_to_theme(item.severity);
        let zone = infer_zone(item, theme);
        let zone_slot = buckets
            .iter_mut()
            .find(|(z, _)| *z == zone)
            .expect("zone exists");
        let idx_in_zone = zone_slot.1.len();
        zone_slot.1.push(i);
        item_meta.push((theme, zone, idx_in_zone));
    }

    let mut entities: Vec<SceneEntity> = Vec::with_capacity(items.len() + ZoneKey::ORDER.len());

    // Zone entities
    for zone in ZoneKey::ORDER {
        let meta = zone.meta();
        let count = buckets
            .iter()
            .find(|(z, _)| *z == zone)
            .map(|(_, v)| v.len())
            .unwrap_or(0);
        entities.push(SceneEntity {
            id: format!("zone_{}", zone.id()),
            entity_type: EntityType::StorageZone,
            transform: Transform::at(meta.pos),
            geometry: EntityGeometry {
                kind: GeometryKind::StorageRoom,
            },
            material: EntityMaterial {
                theme: meta.theme,
                color: None,
                emissive: 0.18,
                opacity: 1.0,
            },
            content: Some(EntityContent {
                title: Some(meta.label.to_string()),
                subtitle: Some(meta.subtitle.to_string()),
                badges: vec![count.to_string()],
                ..Default::default()
            }),
            gameplay: Some(EntityGameplay {
                selectable: false,
                hoverable: false,
                actions: vec![],
                linked_entity_id: None,
            }),
            data: None,
        });
    }

    // Product entities
    for (i, item) in items.iter().enumerate() {
        let (theme, zone, idx_in_zone) = item_meta[i];
        let position = product_position_in_zone(zone, idx_in_zone);
        let asset_key = infer_asset_key(&item.product.category);
        let emoji = category_emoji(&item.product.category);
        let expiry = expiry_label(item.expires_at);
        let short_qty = format!("{:.2} {} · {}", item.remaining_quantity, item.product.base_unit, expiry);
        let item_id = item.id.to_string();
        // Category color overrides severity theme — cards look distinct by type.
        // For expired/critical we keep severity red to keep the warning prominent.
        let card_color = if matches!(theme, MaterialTheme::Expired | MaterialTheme::Critical) {
            None
        } else {
            category_color(&item.product.category).map(|s| s.to_string())
        };

        entities.push(SceneEntity {
            id: format!("product_{}", item_id),
            entity_type: EntityType::InventoryProduct,
            transform: Transform::at(position),
            geometry: EntityGeometry {
                kind: GeometryKind::ProductCard,
            },
            material: EntityMaterial {
                theme,
                color: card_color,
                emissive: emissive_for_theme(theme),
                opacity: 1.0,
            },
            content: Some(EntityContent {
                title: Some(item.product.name.clone()),
                subtitle: Some(short_qty),
                asset_key: Some(asset_key.to_string()),
                image_url: item.product.image_url.clone(),
                fallback_icon: Some(emoji.to_string()),
                badges: vec![item.product.category.clone()],
            }),
            gameplay: Some(EntityGameplay {
                selectable: true,
                hoverable: true,
                actions: actions_for_theme(theme),
                linked_entity_id: Some(item_id.clone()),
            }),
            data: Some(EntityDataRef {
                domain: DomainKind::Inventory,
                entity_id: item_id,
            }),
        });
    }

    // HUD aggregations
    let expiring_count = item_meta
        .iter()
        .filter(|(t, _, _)| *t == MaterialTheme::Warning)
        .count();
    let low_stock_count = items
        .iter()
        .filter(|it| {
            it.product.min_stock_threshold > 0.0
                && it.remaining_quantity <= it.product.min_stock_threshold
        })
        .count();
    let total_value_cents: i128 = items
        .iter()
        .map(|it| (it.remaining_quantity * it.price_per_unit_cents as f64) as i128)
        .sum();

    let total_value_label = format!("{:.2} PLN", (total_value_cents as f64) / 100.0);
    let now = OffsetDateTime::now_utc();
    let tick = (now.unix_timestamp_nanos() / 1_000_000) as i64;
    let generated_at = now
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| String::from("1970-01-01T00:00:00Z"));

    SceneState {
        scene_id: "inventory-main".to_string(),
        mode: SceneMode::Inventory,
        tick,
        generated_at,
        camera: SceneCamera {
            preset: CameraPreset::Overview,
            position: [0.0, 13.0, 17.0],
            target: [0.0, 0.0, 0.0],
            fov: 50.0,
        },
        hud: SceneHud {
            total_value_label: Some(total_value_label),
            items_label: Some(items.len().to_string()),
            expiring_label: Some(expiring_count.to_string()),
            low_stock_label: Some(low_stock_count.to_string()),
            risk_label: None,
        },
        entities,
        selected_entity_id,
    }
}
