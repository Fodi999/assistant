//! User-facing selection set.
#![allow(dead_code, unused_variables, unused_imports)]
use crate::topology::*;
use std::collections::HashSet;

#[derive(Debug, Default)]
pub struct SelectionSet {
    pub vertices: HashSet<VertexId>,
    pub edges:    HashSet<EdgeId>,
    pub faces:    HashSet<FaceId>,
}

