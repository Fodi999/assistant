//! Standardized API response envelope.
//!
//! Every tool returns `ToolResponse<T>` with:
//! - `data`:  the actual result
//! - `meta`:  engine, tool, timing, cache hints
//! - `seo`:   optional SEO metadata (title, description, canonical)
//!
//! Old endpoints return raw JSON for backward compat.
//! The new `/tools/run` endpoint wraps them in this envelope.

use serde::Serialize;
use crate::domain::engines::types::{EngineKind, ToolId};

// ── Response envelope ────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ToolResponse<T: Serialize> {
    pub ok:   bool,
    pub data: T,
    pub meta: ResponseMeta,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seo:  Option<SeoMeta>,
}

#[derive(Debug, Serialize)]
pub struct ResponseMeta {
    pub engine:    EngineKind,
    pub tool:      String,
    pub version:   &'static str,
    /// Processing time in milliseconds
    pub timing_ms: u64,
    /// Cache TTL hint for the client (seconds). 0 = don't cache.
    pub cache_ttl: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SeoMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title:       Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical:   Option<String>,
}

// ── Builder ──────────────────────────────────────────────────────────────────

impl<T: Serialize> ToolResponse<T> {
    pub fn ok(tool: ToolId, data: T, timing_ms: u64) -> Self {
        Self {
            ok: true,
            data,
            meta: ResponseMeta {
                engine:    tool.engine(),
                tool:      tool.path().to_string(),
                version:   "2.0",
                timing_ms,
                cache_ttl: tool.cache_ttl_secs(),
            },
            seo: None,
        }
    }

    pub fn with_seo(mut self, seo: SeoMeta) -> Self {
        self.seo = Some(seo);
        self
    }
}

// ── Error response (no generic needed) ───────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ToolError {
    pub ok:      bool,
    pub error:   String,
    pub code:    String,
    pub tool:    Option<String>,
    pub engine:  Option<EngineKind>,
}

impl ToolError {
    pub fn not_found(tool: &str) -> Self {
        Self {
            ok: false,
            error: format!("Tool '{}' not found", tool),
            code: "TOOL_NOT_FOUND".to_string(),
            tool: Some(tool.to_string()),
            engine: None,
        }
    }

    pub fn bad_request(tool: ToolId, msg: impl Into<String>) -> Self {
        Self {
            ok: false,
            error: msg.into(),
            code: "BAD_REQUEST".to_string(),
            tool: Some(tool.path().to_string()),
            engine: Some(tool.engine()),
        }
    }

    pub fn internal(tool: ToolId, msg: impl Into<String>) -> Self {
        Self {
            ok: false,
            error: msg.into(),
            code: "INTERNAL_ERROR".to_string(),
            tool: Some(tool.path().to_string()),
            engine: Some(tool.engine()),
        }
    }
}
