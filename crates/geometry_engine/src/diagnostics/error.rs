//! Diagnostic severity + message.
#![allow(dead_code, unused_variables, unused_imports)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity { Info, Warning, Error }

#[derive(Debug, Clone)]
pub struct Diagnostic { pub severity: Severity, pub message: String }

