//! Object storage abstraction.
//!
//! MVP ships only [`LocalStorageAdapter`] — files are written under
//! `./uploads/...` and served back via Axum's `tower-http::ServeDir`
//! mounted at `/static/*`. The trait keeps the door open for an
//! S3/R2/GCS implementation without touching the application layer.

pub mod local_storage;

use async_trait::async_trait;

use crate::shared::AppError;

pub use local_storage::LocalStorageAdapter;

/// A minimal, synchronous-write object store.
///
/// Implementations must be **thread-safe** (`Send + Sync`) so they can be
/// stored inside `Arc` and cloned cheaply into Axum state.
#[async_trait]
pub trait StorageAdapter: Send + Sync {
    /// Persist `bytes` at the logical `key` (a relative path, e.g.
    /// `"laboratory/images/<uuid>.png"`). Returns a **public URL** the
    /// browser can fetch directly.
    async fn put_bytes(
        &self,
        key: &str,
        bytes: Vec<u8>,
        content_type: &str,
    ) -> Result<String, AppError>;
}
