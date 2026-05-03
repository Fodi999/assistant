//! Inventory scene service — orchestrator only.
//!
//! Responsibilities live in sibling files:
//!   * `inventory_layout`     — zone placement, slot grid
//!   * `inventory_prefabs`    — `PrefabKey` + asset / emoji dispatch
//!   * `inventory_materials`  — severity → theme, category → accent
//!   * `inventory_mechanics`  — pulse / glow per severity
//!   * `inventory_actions`    — allowed `EntityAction`s per severity
//!
//! This file just glues data → entities → SceneState. Frontend never has
//! to know about layout, severity mapping, or HUD formatting.

use std::sync::Arc;
use time::OffsetDateTime;

use super::inventory_actions::actions_for_theme;
use super::inventory_layout::{
    infer_zone, product_position_in_zone, ZoneKey, CARD_COLS, ROOM_CORNER_RADIUS, ROOM_SIZE,
    ROOM_WALL_HEIGHT, ROOM_WALL_THICKNESS,
};
use super::inventory_materials::{product_material, severity_to_theme, zone_material};
use super::inventory_mechanics::{mechanics_for_theme, zone_mechanics};
use super::inventory_prefabs::{category_emoji, infer_asset_key, product_prefab, zone_prefab};
use crate::application::inventory::{InventoryService, InventoryView};
use crate::domain::scene::{
    CameraPreset, DomainKind, EntityContent, EntityDataRef, EntityGameplay, EntityGeometry,
    EntityType, GeometryKind, MaterialTheme, SceneCamera, SceneEntity, SceneEnvironment, SceneHud,
    SceneMode, SceneState, Transform,
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

#[derive(Clone, Copy)]
struct ItemSlot {
    theme: MaterialTheme,
    zone: ZoneKey,
    index_in_zone: usize,
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

/// Build a `StorageRoom` entity for the given zone.
fn build_zone_entity(zone: ZoneKey, count: usize) -> SceneEntity {
    let meta = zone.meta();
    SceneEntity {
        id: format!("zone_{}", zone.id()),
        entity_type: EntityType::StorageZone,
        prefab: Some(zone_prefab(zone)),
        transform: Transform::at(meta.center),
        geometry: EntityGeometry::new(GeometryKind::StorageRoom)
            .with_size(ROOM_SIZE)
            .with_walls(ROOM_WALL_HEIGHT, ROOM_WALL_THICKNESS)
            .with_corner_radius(ROOM_CORNER_RADIUS),
        material: zone_material(zone),
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
        mechanics: zone_mechanics(meta.theme),
        data: None,
    }
}

/// Build a `ProductCard` entity for the given inventory item.
fn build_product_entity(item: &InventoryView, slot: ItemSlot) -> SceneEntity {
    let position = product_position_in_zone(slot.zone, slot.index_in_zone);
    let asset_key = infer_asset_key(&item.product.category);
    let emoji = category_emoji(&item.product.category);
    let expiry = expiry_label(item.expires_at);
    let subtitle = format!(
        "{:.2} {} · {}",
        item.remaining_quantity, item.product.base_unit, expiry
    );
    let item_id = item.id.to_string();

    SceneEntity {
        id: format!("product_{}", item_id),
        entity_type: EntityType::InventoryProduct,
        prefab: Some(product_prefab()),
        transform: Transform::at(position),
        geometry: EntityGeometry::new(GeometryKind::ProductCard)
            .with_size([1.82, 0.82, 0.06])
            .with_corner_radius(0.055),
        material: product_material(slot.theme, &item.product.category),
        content: Some(EntityContent {
            title: Some(item.product.name.clone()),
            subtitle: Some(subtitle),
            asset_key: Some(asset_key.to_string()),
            image_url: item.product.image_url.clone(),
            fallback_icon: Some(emoji.to_string()),
            badges: vec![item.product.category.clone()],
        }),
        gameplay: Some(EntityGameplay {
            selectable: true,
            hoverable: true,
            actions: actions_for_theme(slot.theme),
            linked_entity_id: Some(item_id.clone()),
        }),
        mechanics: mechanics_for_theme(slot.theme),
        data: Some(EntityDataRef {
            domain: DomainKind::Inventory,
            entity_id: item_id,
        }),
    }
}

fn build_environment() -> SceneEnvironment {
    SceneEnvironment {
        ambient_intensity: Some(0.55),
        ambient_color: Some("#ffffff".to_string()),
        key_light_intensity: Some(0.9),
        background: Some("#06070a".to_string()),
        fog_color: Some("#06070a".to_string()),
        fog_density: Some(0.012),
    }
}

/// Pure scene builder. Public for unit tests.
pub fn build_scene_from_items(
    items: &[InventoryView],
    selected_entity_id: Option<String>,
) -> SceneState {
    // Bucket by zone so per-zone indices are stable.
    let mut bucket_counts: [(ZoneKey, usize); 4] = [
        (ZoneKey::Cold, 0),
        (ZoneKey::Dry, 0),
        (ZoneKey::Freezer, 0),
        (ZoneKey::Risk, 0),
    ];

    let mut slots: Vec<ItemSlot> = Vec::with_capacity(items.len());
    for item in items.iter() {
        let theme = severity_to_theme(item.severity);
        let zone = infer_zone(item, theme);
        let bucket = bucket_counts
            .iter_mut()
            .find(|(z, _)| *z == zone)
            .expect("zone exists");
        let index_in_zone = bucket.1;
        bucket.1 += 1;
        slots.push(ItemSlot {
            theme,
            zone,
            index_in_zone,
        });
    }

    let mut entities: Vec<SceneEntity> = Vec::with_capacity(items.len() + ZoneKey::ORDER.len());

    // Zone entities (rooms)
    for zone in ZoneKey::ORDER {
        let count = bucket_counts
            .iter()
            .find(|(z, _)| *z == zone)
            .map(|(_, c)| *c)
            .unwrap_or(0);
        entities.push(build_zone_entity(zone, count));
    }

    // Product entities
    for (i, item) in items.iter().enumerate() {
        entities.push(build_product_entity(item, slots[i]));
    }

    // HUD aggregations
    let expiring_count = slots
        .iter()
        .filter(|s| s.theme == MaterialTheme::Warning)
        .count();
    let low_stock_count = items
        .iter()
        .filter(|it| {
            it.product.min_stock_threshold > 0.0
                && it.remaining_quantity <= it.product.min_stock_threshold
        })
        .count();
    let risk_count = slots
        .iter()
        .filter(|s| matches!(s.theme, MaterialTheme::Critical | MaterialTheme::Expired))
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
            position: [0.0, 8.0, 13.0],
            target: [0.0, 1.5, 0.0],
            fov: 58.0,
        },
        environment: Some(build_environment()),
        hud: SceneHud {
            total_value_label: Some(total_value_label),
            items_label: Some(items.len().to_string()),
            expiring_label: Some(expiring_count.to_string()),
            low_stock_label: Some(low_stock_count.to_string()),
            risk_label: if risk_count > 0 {
                Some(risk_count.to_string())
            } else {
                None
            },
        },
        entities,
        mechanics: vec![],
        selected_entity_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_inventory_still_builds_zones() {
        let scene = build_scene_from_items(&[], None);
        assert_eq!(scene.entities.len(), ZoneKey::ORDER.len());
        assert!(scene.entities[0].prefab.is_some());
        assert!(scene.environment.is_some());
    }

    #[test]
    fn card_grid_first_row_z_constant() {
        let pos_first = product_position_in_zone(ZoneKey::Dry, 0);
        let pos_last_in_row = product_position_in_zone(ZoneKey::Dry, CARD_COLS - 1);
        assert!((pos_first[2] - pos_last_in_row[2]).abs() < 0.001);
    }
}
