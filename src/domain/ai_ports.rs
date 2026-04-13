//! AI Ports — domain-level traits для AI abstraction
//!
//! DDD: Domain не знает о Groq, OpenAI, etc.
//! Application layer работает через эти трейты.
//!
//! Это даёт:
//! ✔ Unit tests с mock AI
//! ✔ Fallback models (Groq → OpenAI → local)
//! ✔ Cheap mode (10% cost — smaller model)
//! ✔ Dry-run режим для CI

use crate::shared::AppResult;
use async_trait::async_trait;

/// Core AI generation port — domain doesn't care about provider
#[async_trait]
pub trait AiClient: Send + Sync {
    /// Send a prompt, get raw text response
    /// Used for: autofill, SEO, audit, pairings
    async fn generate(&self, prompt: &str, max_tokens: u32) -> AppResult<String>;

    /// Generate with specific model preference
    /// `quality`: "fast" (8b) | "balanced" (70b) | "best" (70b + retry)
    async fn generate_with_quality(
        &self,
        prompt: &str,
        max_tokens: u32,
        quality: AiQuality,
    ) -> AppResult<String>;
}

/// AI quality tier — controls model selection & cost
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiQuality {
    /// Fast & cheap: gemini-3-flash-preview
    /// Good for: translations, simple classification
    Fast,

    /// Balanced: gemini-3-flash-preview
    /// Good for: autofill, SEO, pairings, chat AI brain
    Balanced,

    /// Best: gemini-3.1-pro-preview + structured retry on parse failure
    /// Good for: audit, complex analysis
    Best,
}

impl Default for AiQuality {
    fn default() -> Self {
        Self::Balanced
    }
}

impl std::fmt::Display for AiQuality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AiQuality::Fast => write!(f, "fast"),
            AiQuality::Balanced => write!(f, "balanced"),
            AiQuality::Best => write!(f, "best"),
        }
    }
}
