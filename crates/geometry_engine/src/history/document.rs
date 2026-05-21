//! Parametric document = ordered list of operations.
#![allow(dead_code, unused_variables, unused_imports)]
use super::{operation::Operation, operation_id::OperationId};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Document {
    pub operations: Vec<OperationId>,
    pub store: HashMap<OperationId, Operation>,
}

