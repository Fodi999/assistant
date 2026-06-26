use serde::Deserialize;
use uuid::Uuid;

pub const CHURCH_SITE_ID: Uuid = Uuid::from_u128(0x00000000000000000000000000000101);
pub const CONSTRUCTION_SITE_ID: Uuid = Uuid::from_u128(0x00000000000000000000000000000102);
pub const KITCHEN_SITE_ID: Uuid = Uuid::from_u128(0x00000000000000000000000000000103);

#[derive(Debug, Deserialize)]
pub struct SiteQuery {
    pub site_id: Option<Uuid>,
    pub site: Option<String>,
}

pub fn resolve_site_id(query: &SiteQuery, default_site_id: Uuid) -> Uuid {
    query.site_id.unwrap_or_else(|| {
        query
            .site
            .as_deref()
            .and_then(site_id_from_alias)
            .unwrap_or(default_site_id)
    })
}

pub fn site_id_from_alias(value: &str) -> Option<Uuid> {
    match value.trim().to_ascii_lowercase().as_str() {
        "church" | "icons" => Some(CHURCH_SITE_ID),
        "construction" | "almabuild" => Some(CONSTRUCTION_SITE_ID),
        "kitchen" | "culinary" => Some(KITCHEN_SITE_ID),
        _ => None,
    }
}

pub fn canonical_site_key(site_id: Uuid) -> &'static str {
    if site_id == CHURCH_SITE_ID {
        "church"
    } else if site_id == CONSTRUCTION_SITE_ID {
        "construction"
    } else {
        "kitchen"
    }
}
