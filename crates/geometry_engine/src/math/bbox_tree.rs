//! BVH tree for accelerated spatial queries.
#![allow(dead_code, unused_variables)]
use crate::math::Aabb;

pub struct BboxTree<T> {
    pub root: Option<usize>,
    pub nodes: Vec<BvhNode<T>>,
}

pub struct BvhNode<T> {
    pub bbox: Aabb,
    pub left: Option<usize>,
    pub right: Option<usize>,
    pub payload: Option<T>,
}

impl<T> Default for BboxTree<T> { fn default() -> Self { Self::new() } }
impl<T> BboxTree<T> {
    pub fn new() -> Self { Self { root: None, nodes: Vec::new() } }
    pub fn build(items: Vec<(Aabb, T)>) -> Self { todo!() }
    pub fn query<F: FnMut(&T)>(&self, bbox: &Aabb, f: F) { todo!() }
}
