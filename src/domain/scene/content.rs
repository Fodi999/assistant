//! Visible content of an entity — title, subtitle, image, badges.
//!
//! Pure presentation: locale-formatted strings are baked here so the
//! frontend never has to know about i18n number / date rules.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    /// Stable asset key resolved against the frontend's `assetRegistry`.
    /// Use this to swap photo/icon without changing the data layer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    /// Emoji / icon fallback when no `image_url`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_icon: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub badges: Vec<String>,
}
