//! The current time, injectable so rules that depend on "now" (minimum notice,
//! booking horizon, hold expiry) can be tested with a fixed instant.

use std::sync::Arc;
use time::OffsetDateTime;

#[derive(Clone)]
pub struct Clock(Arc<dyn Fn() -> OffsetDateTime + Send + Sync>);

impl Clock {
    pub fn system() -> Self {
        Self(Arc::new(OffsetDateTime::now_utc))
    }

    /// A clock that always returns `instant` (tests).
    pub fn fixed(instant: OffsetDateTime) -> Self {
        Self(Arc::new(move || instant))
    }

    pub fn now(&self) -> OffsetDateTime {
        (self.0)()
    }
}

impl Default for Clock {
    fn default() -> Self {
        Self::system()
    }
}

impl std::fmt::Debug for Clock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Clock")
    }
}
