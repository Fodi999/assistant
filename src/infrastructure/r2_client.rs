use aws_sdk_s3::{Client, primitives::ByteStream};
use bytes::Bytes;
use crate::shared::AppError;

/// Cloudflare R2 Client (S3-compatible API)
#[derive(Clone)]
pub struct R2Client {
    client: Client,
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
        // Configure S3 client for R2
        let credentials = aws_sdk_s3::config::Credentials::new(
            access_key_id,
            secret_access_key,
            None,
            None,
            "r2-credentials",
        );

        let endpoint_url = format!("https://{}.r2.cloudflarestorage.com", account_id);
        
        let config = aws_sdk_s3::config::Builder::new()
            .credentials_provider(credentials)
            .endpoint_url(&endpoint_url)
            .region(aws_sdk_s3::config::Region::new("auto"))
            .build();

        let client = Client::from_conf(config);

        // Public URL base (first 8 chars of account_id)
        let account_prefix = account_id.chars().take(8).collect::<String>();
        let public_url_base = format!("https://pub-{}.r2.dev", account_prefix);

        Self {
            client,
            bucket_name,
            public_url_base,
        }
    }

    /// Upload image to R2
    /// Returns public URL
    pub async fn upload_image(
        &self,
        key: &str,
        content: Bytes,
        content_type: &str,
    ) -> Result<String, AppError> {
        let byte_stream = ByteStream::from(content);

        self.client
            .put_object()
            .bucket(&self.bucket_name)
            .key(key)
            .body(byte_stream)
            .content_type(content_type)
            .send()
            .await
            .map_err(|e| AppError::internal(format!("Failed to upload to R2: {}", e)))?;

        // Return public URL
        let public_url = format!("{}/{}", self.public_url_base, key);
        Ok(public_url)
    }

    /// Delete image from R2
    pub async fn delete_image(&self, key: &str) -> Result<(), AppError> {
        self.client
            .delete_object()
            .bucket(&self.bucket_name)
            .key(key)
            .send()
            .await
            .map_err(|e| AppError::internal(format!("Failed to delete from R2: {}", e)))?;

        Ok(())
    }

    /// Check if object exists
    pub async fn object_exists(&self, key: &str) -> bool {
        self.client
            .head_object()
            .bucket(&self.bucket_name)
            .key(key)
            .send()
            .await
            .is_ok()
    }
}
