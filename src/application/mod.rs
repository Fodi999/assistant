//! Use cases. Handlers stay thin: they parse the request and call a service.

pub mod access;
pub mod auth;
pub mod business;
pub mod catalog;

pub use access::{BusinessAccess, Role};
pub use auth::AuthService;
pub use business::BusinessService;
pub use catalog::CatalogService;
