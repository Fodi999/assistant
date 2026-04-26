pub mod ai_client_impl; // 🆕 AiClient trait implementation for LlmAdapter
pub mod cache;          // 🆕 In-memory cache for public endpoints (saves Neon CU)
pub mod config;
pub mod gemini_service; // 🆕 Google Gemini AI (replaces Groq for generation)
pub mod groq_service;   // Legacy — types re-exported by gemini_service
pub mod ingredient_cache; // 🆕 In-memory ingredient catalog for Sous-Chef (0 SQL)
pub mod llm_adapter;
pub mod persistence;
pub mod r2_client;
pub mod security;
pub mod stripe_service; // 🆕 Stripe Checkout + Webhook

pub use cache::AppCache;
pub use config::*;
pub use gemini_service::GeminiService;
pub use groq_service::{GroqService, UnifiedProductResponse};
pub use ingredient_cache::IngredientCache;
pub use llm_adapter::LlmAdapter;
pub use persistence::*;
pub use r2_client::R2Client;
pub use security::*;
pub use stripe_service::StripeService;
