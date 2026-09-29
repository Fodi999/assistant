//! Use cases. Handlers stay thin: they parse the request and call a service.

pub mod access;
pub mod auth;
pub mod availability;
pub mod business;
pub mod catalog;
pub mod schedule;
pub mod team;

pub use access::{BusinessAccess, Role};
pub use auth::AuthService;
pub use availability::AvailabilityService;
pub use business::BusinessService;
pub use catalog::CatalogService;
pub use schedule::ScheduleService;
pub use team::TeamService;
