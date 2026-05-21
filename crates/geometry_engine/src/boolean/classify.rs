//! Classify split faces as inside / outside / on for boolean.
#![allow(dead_code, unused_variables, unused_imports)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Classification { Inside, Outside, OnBoundary, Coplanar }

