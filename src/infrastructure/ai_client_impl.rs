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
                // Use flash model for speed
                self.groq_raw_request_with_model(prompt, max_tokens, "gemini-2.5-flash").await
            }
            AiQuality::Balanced => {
                // Default pro model
                self.groq_raw_request(prompt, max_tokens).await
            }
            AiQuality::Best => {
                // Pro model with retry on failure
                match self.groq_raw_request(prompt, max_tokens).await {
                    Ok(result) => Ok(result),
                    Err(_first_err) => {
                        tracing::warn!("🔄 AI Best quality: retrying after first failure");
                        self.groq_raw_request(prompt, max_tokens).await
                    }
                }
            }
        }
    }
}
