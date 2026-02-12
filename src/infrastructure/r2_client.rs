use bytes::Bytes;
use reqwest::Client;
use hmac::{Hmac, Mac};
use sha2::{Sha256, Digest};
use crate::shared::AppError;

type HmacSha256 = Hmac<Sha256>;

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
        
        // Simple S3 signature v4 (AWS4-HMAC-SHA256)
        let date = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
        let date_short = &date[0..8];
        
        // Create string to sign
        let content_sha256 = format!("{:x}", Sha256::digest(&content));
        let canonical_request = format!(
            "PUT\n/{}/{}\n\nhost:{}\nx-amz-content-sha256:{}\nx-amz-date:{}\n\nhost;x-amz-content-sha256;x-amz-date\n{}",
            self.bucket_name, key, 
            format!("{}.r2.cloudflarestorage.com", self.bucket_name),
            content_sha256,
            date,
            content_sha256
        );
        
        let credential_scope = format!("{}/auto/s3/aws4_request", date_short);
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{}\n{}\n{:x}",
            date,
            credential_scope,
            Sha256::digest(canonical_request.as_bytes())
        );
        
        // Calculate signature
        let mut mac = HmacSha256::new_from_slice(format!("AWS4{}", self.secret_access_key).as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(date_short.as_bytes());
        let k_date = mac.finalize().into_bytes();
        
        let mut mac = HmacSha256::new_from_slice(&k_date)
            .expect("HMAC can take key of any size");
        mac.update(b"auto");
        let k_region = mac.finalize().into_bytes();
        
        let mut mac = HmacSha256::new_from_slice(&k_region)
            .expect("HMAC can take key of any size");
        mac.update(b"s3");
        let k_service = mac.finalize().into_bytes();
        
        let mut mac = HmacSha256::new_from_slice(&k_service)
            .expect("HMAC can take key of any size");
        mac.update(b"aws4_request");
        let k_signing = mac.finalize().into_bytes();
        
        let mut mac = HmacSha256::new_from_slice(&k_signing)
            .expect("HMAC can take key of any size");
        mac.update(string_to_sign.as_bytes());
        let signature = format!("{:x}", mac.finalize().into_bytes());
        
        let authorization = format!(
            "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders=host;x-amz-content-sha256;x-amz-date, Signature={}",
            self.access_key_id,
            credential_scope,
            signature
        );

        let response = self.http_client
            .put(&url)
            .header("Authorization", authorization)
            .header("x-amz-date", date)
            .header("x-amz-content-sha256", content_sha256)
            .header("Content-Type", content_type)
            .body(content)
            .send()
            .await
            .map_err(|e| AppError::internal(format!("Failed to upload to R2: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
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
