//! AI Use Cases — каждый файл = один бизнес use-case
//!
//! Clean Architecture: Application → Use Cases → Domain + Ports
//!
//! Каждый use-case:
//! 1. Получает входные данные от handler (Interfaces layer)
//! 2. Оркестрирует вызовы через AiClient trait (domain port)
//! 3. Кеширует результат через AiCacheRepository
//! 4. Возвращает результат

pub mod autofill;
pub mod create_product_draft;
pub mod generate_seo;
pub mod audit;
pub mod pairing;
