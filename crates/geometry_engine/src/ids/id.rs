//! Stable monotonically-increasing IDs.
#![allow(dead_code, unused_variables, unused_imports)]
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Id(pub u64);

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

impl Id {
    pub fn fresh() -> Self { Self(NEXT_ID.fetch_add(1, Ordering::Relaxed)) }
    pub const fn from_raw(raw: u64) -> Self { Self(raw) }
    pub const fn raw(self) -> u64 { self.0 }
}

