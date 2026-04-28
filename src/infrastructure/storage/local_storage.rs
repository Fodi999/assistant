//! Local filesystem [`StorageAdapter`].
//!
//! Files are written under `root_dir/<key>` and served back via a
//! `ServeDir` mounted at `public_url_prefix`. The adapter is intentionally
//! oblivious to *what* it stores — it just writes bytes and returns a URL.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use tokio::fs;
use tokio::io::AsyncWriteExt;

use super::StorageAdapter;
use crate::shared::AppError;

#[derive(Clone)]
pub struct LocalStorageAdapter {
    root_dir: PathBuf,
    /// URL prefix that `ServeDir(root_dir)` is mounted under, **without**
    /// a trailing slash, e.g. `"/static"`.
    public_url_prefix: String,
}

impl LocalStorageAdapter {
    pub fn new(root_dir: impl Into<PathBuf>, public_url_prefix: impl Into<String>) -> Self {
        Self {
            root_dir: root_dir.into(),
            public_url_prefix: public_url_prefix.into().trim_end_matches('/').to_string(),
        }
    }

    fn full_path(&self, key: &str) -> PathBuf {
        // Defensive: refuse keys that try to escape root.
        let safe = key.trim_start_matches('/');
        self.root_dir.join(safe)
    }
}

#[async_trait]
impl StorageAdapter for LocalStorageAdapter {
    async fn put_bytes(
        &self,
        key: &str,
        bytes: Vec<u8>,
        _content_type: &str,
    ) -> Result<String, AppError> {
        if key.contains("..") || key.starts_with('/') {
            return Err(AppError::validation(format!(
                "storage: invalid key `{key}`"
            )));
        }

        let path = self.full_path(key);
        if let Some(parent) = path.parent() {
            ensure_dir(parent).await?;
        }

        let mut file = fs::File::create(&path)
            .await
            .map_err(|e| AppError::internal(format!("storage: create {path:?}: {e}")))?;
        file.write_all(&bytes)
            .await
            .map_err(|e| AppError::internal(format!("storage: write {path:?}: {e}")))?;
        file.flush()
            .await
            .map_err(|e| AppError::internal(format!("storage: flush {path:?}: {e}")))?;

        Ok(format!(
            "{}/{}",
            self.public_url_prefix,
            key.trim_start_matches('/')
        ))
    }
}

async fn ensure_dir(dir: &Path) -> Result<(), AppError> {
    fs::create_dir_all(dir)
        .await
        .map_err(|e| AppError::internal(format!("storage: mkdir {dir:?}: {e}")))
}
