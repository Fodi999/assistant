//! Aggregated diagnostic report.
#![allow(dead_code, unused_variables, unused_imports)]
use super::error::Diagnostic;

#[derive(Debug, Default, Clone)]
pub struct Report { pub diagnostics: Vec<Diagnostic> }

impl Report {
    pub fn ok() -> Self { Self::default() }
    pub fn is_ok(&self) -> bool { self.diagnostics.is_empty() }
}

