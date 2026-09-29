//! Use cases. Handlers stay thin: they parse the request and call a service.

pub mod access;
pub mod auth;
pub mod availability;
pub mod booking;
pub mod admin;
pub mod business;
pub mod catalog;
pub mod clients;
pub mod profile;
pub mod public;
pub mod schedule;
pub mod team;

pub use access::{BusinessAccess, Role};
pub use auth::AuthService;
pub use availability::AvailabilityService;
pub use booking::BookingService;
pub use admin::AdminService;
pub use business::BusinessService;
pub use catalog::CatalogService;
pub use clients::ClientService;
pub use profile::ProfileService;
pub use public::PublicApi;
pub use schedule::ScheduleService;
pub use team::TeamService;
