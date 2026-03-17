//! Infrastructure implementation of AiClient trait
//!
//! LlmAdapter now implements the domain port AiClient.
//! This bridges Domain ↔ Infrastructure cleanly.

use crate::domain::ai_ports::{AiClient, AiQuality};
use crate::infrastructure::LlmAdapter;
use crate::shared::AppResult;
use async_trait::async_trait;

#[async_trait]
impl AiClient for LlmAdapter {
    async fn generate(&self, prompt: &str, max_tokens: u32) -> AppResult<String> {
        self.groq_raw_request(prompt, max_tokens).await
    }

    async fn generate_with_quality(
        &self,
        prompt: &str,
        max_tokens: u32,
        quality: AiQuality,
    ) -> AppResult<String> {
        match quality {
            AiQuality::Fast => {
                // Use 8b model via translate-level fast path
                self.groq_raw_request_with_model(prompt, max_tokens, "llama-3.1-8b-instant").await
            }
            AiQuality::Balanced => {
                // Default 70b model
                self.groq_raw_request(prompt, max_tokens).await
            }
            AiQuality::Best => {
                // 70b with retry on failure
                match self.groq_raw_request(prompt, max_tokens).await {
                    Ok(result) => Ok(result),
                    Err(_first_err) => {
                        tracing::warn!("🔄 AI Best quality: retrying after first failure");
                        // Retry once
                        self.groq_raw_request(prompt, max_tokens).await
                    }
                }
            }
        }
    }
}
