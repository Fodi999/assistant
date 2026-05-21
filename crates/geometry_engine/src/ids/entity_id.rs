//! Typed entity IDs (vertex, edge, face, body) — phantom-typed Id.
#![allow(dead_code, unused_variables, unused_imports)]
use std::marker::PhantomData;
use super::id::Id;

#[derive(Debug)]
pub struct EntityId<T> { pub id: Id, _marker: PhantomData<T> }

impl<T> Clone for EntityId<T> { fn clone(&self) -> Self { *self } }
impl<T> Copy  for EntityId<T> {}
impl<T> PartialEq for EntityId<T> { fn eq(&self, o: &Self) -> bool { self.id == o.id } }
impl<T> Eq for EntityId<T> {}
impl<T> std::hash::Hash for EntityId<T> { fn hash<H: std::hash::Hasher>(&self, s: &mut H) { self.id.hash(s) } }

impl<T> EntityId<T> {
    pub fn fresh() -> Self { Self { id: Id::fresh(), _marker: PhantomData } }
    pub const fn from_id(id: Id) -> Self { Self { id, _marker: PhantomData } }
}

