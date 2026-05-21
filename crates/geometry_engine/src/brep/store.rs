//! Arena storage for all topology entities.
//! BrepStore owns every entity and hands out typed IDs as stable keys.
//! All mutation goes through the `add_*` / `get_*_mut` methods so that
//! future change-notification hooks can be added in one place.
#![allow(dead_code, unused_variables, unused_imports)]
use std::collections::HashMap;
use crate::topology::*;

#[derive(Debug, Default)]
pub struct BrepStore {
    pub vertices: HashMap<VertexId, Vertex>,
    pub edges:    HashMap<EdgeId, Edge>,
    pub coedges:  HashMap<CoEdgeId, CoEdge>,
    pub loops:    HashMap<LoopId, Loop>,
    pub faces:    HashMap<FaceId, Face>,
    pub shells:   HashMap<ShellId, Shell>,
    pub solids:   HashMap<SolidId, Solid>,
    pub bodies:   HashMap<BodyId, Body>,
}

impl BrepStore {
    pub fn new() -> Self {
        Self::default()
    }

    // ── Vertex ────────────────────────────────────────────────────────────
    pub fn add_vertex(&mut self, v: Vertex) -> VertexId {
        let id = VertexId::fresh();
        self.vertices.insert(id, v);
        id
    }
    pub fn get_vertex(&self, id: VertexId) -> Option<&Vertex> { self.vertices.get(&id) }
    pub fn get_vertex_mut(&mut self, id: VertexId) -> Option<&mut Vertex> { self.vertices.get_mut(&id) }

    // ── Edge ──────────────────────────────────────────────────────────────
    pub fn add_edge(&mut self, e: Edge) -> EdgeId {
        let id = EdgeId::fresh();
        self.edges.insert(id, e);
        id
    }
    pub fn get_edge(&self, id: EdgeId) -> Option<&Edge> { self.edges.get(&id) }
    pub fn get_edge_mut(&mut self, id: EdgeId) -> Option<&mut Edge> { self.edges.get_mut(&id) }

    // ── CoEdge ────────────────────────────────────────────────────────────
    pub fn add_coedge(&mut self, ce: CoEdge) -> CoEdgeId {
        let id = CoEdgeId::fresh();
        self.coedges.insert(id, ce);
        id
    }
    pub fn get_coedge(&self, id: CoEdgeId) -> Option<&CoEdge> { self.coedges.get(&id) }
    pub fn get_coedge_mut(&mut self, id: CoEdgeId) -> Option<&mut CoEdge> { self.coedges.get_mut(&id) }

    // ── Loop ─────────────────────────────────────────────────────────────
    pub fn add_loop(&mut self, lp: Loop) -> LoopId {
        let id = LoopId::fresh();
        self.loops.insert(id, lp);
        id
    }
    pub fn get_loop(&self, id: LoopId) -> Option<&Loop> { self.loops.get(&id) }
    pub fn get_loop_mut(&mut self, id: LoopId) -> Option<&mut Loop> { self.loops.get_mut(&id) }

    // ── Face ─────────────────────────────────────────────────────────────
    pub fn add_face(&mut self, f: Face) -> FaceId {
        let id = FaceId::fresh();
        self.faces.insert(id, f);
        id
    }
    pub fn get_face(&self, id: FaceId) -> Option<&Face> { self.faces.get(&id) }
    pub fn get_face_mut(&mut self, id: FaceId) -> Option<&mut Face> { self.faces.get_mut(&id) }

    // ── Shell ─────────────────────────────────────────────────────────────
    pub fn add_shell(&mut self, s: Shell) -> ShellId {
        let id = ShellId::fresh();
        self.shells.insert(id, s);
        id
    }
    pub fn get_shell(&self, id: ShellId) -> Option<&Shell> { self.shells.get(&id) }
    pub fn get_shell_mut(&mut self, id: ShellId) -> Option<&mut Shell> { self.shells.get_mut(&id) }

    // ── Solid ─────────────────────────────────────────────────────────────
    pub fn add_solid(&mut self, s: Solid) -> SolidId {
        let id = SolidId::fresh();
        self.solids.insert(id, s);
        id
    }
    pub fn get_solid(&self, id: SolidId) -> Option<&Solid> { self.solids.get(&id) }
    pub fn get_solid_mut(&mut self, id: SolidId) -> Option<&mut Solid> { self.solids.get_mut(&id) }

    // ── Body ──────────────────────────────────────────────────────────────
    pub fn add_body(&mut self, b: Body) -> BodyId {
        let id = BodyId::fresh();
        self.bodies.insert(id, b);
        id
    }
    pub fn get_body(&self, id: BodyId) -> Option<&Body> { self.bodies.get(&id) }
    pub fn get_body_mut(&mut self, id: BodyId) -> Option<&mut Body> { self.bodies.get_mut(&id) }

    // ── Statistics ────────────────────────────────────────────────────────
    pub fn entity_counts(&self) -> BrepCounts {
        BrepCounts {
            vertices: self.vertices.len(),
            edges:    self.edges.len(),
            coedges:  self.coedges.len(),
            loops:    self.loops.len(),
            faces:    self.faces.len(),
            shells:   self.shells.len(),
            solids:   self.solids.len(),
            bodies:   self.bodies.len(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BrepCounts {
    pub vertices: usize,
    pub edges:    usize,
    pub coedges:  usize,
    pub loops:    usize,
    pub faces:    usize,
    pub shells:   usize,
    pub solids:   usize,
    pub bodies:   usize,
}


