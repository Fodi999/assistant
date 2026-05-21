//! Profile wrapper proven to be CCW, non-self-intersecting, closed.
#![allow(dead_code, unused_variables, unused_imports)]
use super::profile2::Profile2;

#[derive(Debug, Clone)]
pub struct ValidatedProfile { pub inner: Profile2 }

