//! Strongly-typed topology IDs.
#![allow(dead_code, unused_variables, unused_imports)]
use crate::ids::EntityId;

#[derive(Debug, Clone, Copy)] pub struct VertexTag;
#[derive(Debug, Clone, Copy)] pub struct EdgeTag;
#[derive(Debug, Clone, Copy)] pub struct CoEdgeTag;
#[derive(Debug, Clone, Copy)] pub struct LoopTag;
#[derive(Debug, Clone, Copy)] pub struct FaceTag;
#[derive(Debug, Clone, Copy)] pub struct ShellTag;
#[derive(Debug, Clone, Copy)] pub struct SolidTag;
#[derive(Debug, Clone, Copy)] pub struct BodyTag;

pub type VertexId = EntityId<VertexTag>;
pub type EdgeId   = EntityId<EdgeTag>;
pub type CoEdgeId = EntityId<CoEdgeTag>;
pub type LoopId   = EntityId<LoopTag>;
pub type FaceId   = EntityId<FaceTag>;
pub type ShellId  = EntityId<ShellTag>;
pub type SolidId  = EntityId<SolidTag>;
pub type BodyId   = EntityId<BodyTag>;
