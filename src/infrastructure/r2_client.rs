use bytes::Bytes;
use reqwest::Client;
use crate::shared::AppError;

/// Cloudflare R2 Client (direct HTTP implementation, lightweight)
#[derive(Clone)]
pub struct R2Client {
    http_client: Client,
    endpoint_url: String,
    access_key_id: String,
    secret_access_key: String,
    bucket_name: String,
    public_url_base: String,
}

impl R2Client {
    pub async fn new(
        account_id: String,
        access_key_id: String,
        secret_access_key: String,
        bucket_name: String,
    ) -> Self {
        let http_client = Client::builder()
            .build()
            .expect("Failed to create HTTP client");

        let endpoint_url = format!("https://{}.r2.cloudflarestorage.com", account_id);
        
        // Public URL base (first 8 chars of account_id)
        let account_prefix = account_id.chars().take(8).collect::<String>();
        let public_url_base = format!("https://pub-{}.r2.dev", account_prefix);

        Self {
            http_client,
            endpoint_url,
            access_key_id,
            secret_access_key,
            bucket_name,
            public_url_base,
        }
    }

    /// Upload image to R2 using S3-compatible PUT
    /// Returns public URL
    pub async fn upload_image(
        &self,
        key: &str,
        content: Bytes,
        content_type: &str,
    ) -> Result<String, AppError> {
        let url = format!("{}/{}/{}", self.endpoint_url, self.bucket_name, key);
        
        // Try simple PUT first (some R2 buckets allow unsigned uploads)
        let response = self.http_client
            .put(&url)
            .header("Content-Type", content_type)
            .header("x-amz-acl", "public-read")
            .basic_auth(&self.access_key_id, Some(&self.secret_access_key))
            .body(content)
            .send()
            .await
            .map_err(|e| AppError::internal(format!("Failed to upload to R2: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("R2 upload failed: {} - {}", status, body);
            return Err(AppError::internal(format!("R2 upload failed ({}): {}", status, body)));
        }

        // Return public URL
        let public_url = format!("{}/{}", self.public_url_base, key);
        Ok(public_url)
    }

    /// Delete image from R2
    pub async fn delete_image(&self, key: &str) -> Result<(), AppError> {
        let url = format!("{}/{}/{}", self.endpoint_url, self.bucket_name, key);
        
        // Simple DELETE request (R2 allows unsigned deletes for existing keys)
        let response = self.http_client
            .delete(&url)
            .send()
            .await
            .map_err(|e| AppError::internal(format!("Failed to delete from R2: {}", e)))?;

        if !response.status().is_success() && response.status().as_u16() != 404 {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::internal(format!("R2 delete failed ({}): {}", status, body)));
        }

        Ok(())
    }

    /// Check if object exists
    pub async fn object_exists(&self, key: &str) -> bool {
        let url = format!("{}/{}/{}", self.endpoint_url, self.bucket_name, key);
        
        self.http_client
            .head(&url)
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }
}
