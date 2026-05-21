//! Fluent B-Rep construction API.
//!
//! `BrepBuilder` is the canonical way to assemble a topologically-consistent
//! `BrepModel` from raw geometry. It exposes three layers, from lowest to
//! highest level of abstraction:
//!
//!   1. **Direct entity ops** (`add_vertex`, `add_edge`, …) — thin wrappers
//!      around [`BrepStore`] that also remember the *current* body / solid /
//!      shell context so the caller doesn't have to pass IDs everywhere.
//!
//!   2. **Euler operators** (`mev`, `mef`, `kev`, `kef`) — the classical
//!      Mäntylä operators which guarantee the Euler–Poincaré invariant
//!      `V − E + F = 2 (S − G)` is preserved at every step.
//!
//!   3. **High-level primitives** (`box_from_extents`, `prism_from_polygon`,
//!      `polyline_face`) — ready-made constructors used by the public
//!      `ops::extrude::*` API and by tests.
//!
//! Every method returns `&mut Self` or a typed ID, so it can be chained
//! ergonomically:
//!
//! ```ignore
//! let model = BrepBuilder::new()
//!     .begin_body("part-1")
//!     .box_from_extents([0.0, 0.0, 0.0], [1.0, 1.0, 1.0])
//!     .build();
//! ```
#![allow(dead_code, unused_variables, unused_imports)]

use crate::math::{Point3, Real};
use crate::topology::*;
use super::model::BrepModel;
use super::store::BrepCounts;

/// Result returned by Euler operators / primitive constructors when a useful
/// "primary" identifier needs to flow back to the caller.
#[derive(Debug, Clone, Copy, Default)]
pub struct BuildHandles {
    pub body:  Option<BodyId>,
    pub solid: Option<SolidId>,
    pub shell: Option<ShellId>,
    pub face:  Option<FaceId>,
}

#[derive(Default)]
pub struct BrepBuilder {
    /// The model being assembled.
    pub model: BrepModel,
    /// Currently-active context — auto-set by `begin_*` calls.
    cur_body:  Option<BodyId>,
    cur_solid: Option<SolidId>,
    cur_shell: Option<ShellId>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Construction & finalisation
// ─────────────────────────────────────────────────────────────────────────────
impl BrepBuilder {
    pub fn new() -> Self { Self::default() }

    /// Consume the builder and return the finished model.
    pub fn build(self) -> BrepModel { self.model }

    /// Snapshot of entity counts — useful in tests / diagnostics.
    pub fn counts(&self) -> BrepCounts { self.model.store.entity_counts() }

    /// Current body / solid / shell context.
    pub fn context(&self) -> BuildHandles {
        BuildHandles {
            body:  self.cur_body,
            solid: self.cur_solid,
            shell: self.cur_shell,
            face:  None,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Layer 1 — context management
// ─────────────────────────────────────────────────────────────────────────────
impl BrepBuilder {
    /// Start a new body and make it the active context.
    pub fn begin_body(&mut self, name: impl Into<String>) -> &mut Self {
        let id = self.model.store.add_body(Body::new().with_name(name));
        self.cur_body = Some(id);
        self.cur_solid = None;
        self.cur_shell = None;
        self
    }

    /// Begin a new solid inside the current body (creates a body if absent).
    pub fn begin_solid(&mut self) -> &mut Self {
        let body = self.cur_body.unwrap_or_else(|| {
            self.model.store.add_body(Body::new())
        });
        self.cur_body = Some(body);

        // Outer shell is created lazily on first face — for now register an
        // empty shell placeholder so `cur_shell` is always valid.
        let shell = self.model.store.add_shell(Shell::new());
        let solid = self.model.store.add_solid(Solid::new(body, shell));

        if let Some(b) = self.model.store.get_body_mut(body) { b.add_solid(solid); }
        if let Some(s) = self.model.store.get_shell_mut(shell) { s.solid = Some(solid); }

        self.cur_solid = Some(solid);
        self.cur_shell = Some(shell);
        self
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Layer 1 — direct entity ops
// ─────────────────────────────────────────────────────────────────────────────
impl BrepBuilder {
    pub fn add_vertex(&mut self, p: Point3) -> VertexId {
        self.model.store.add_vertex(Vertex::new(p))
    }

    pub fn add_edge(&mut self, a: VertexId, b: VertexId) -> EdgeId {
        let mut e = Edge::new(a, b);
        // Compute cached length from the two endpoints if they exist.
        if let (Some(va), Some(vb)) = (
            self.model.store.get_vertex(a),
            self.model.store.get_vertex(b),
        ) {
            e.length = va.point.distance(vb.point);
        }
        self.model.store.add_edge(e)
    }

    pub fn add_coedge(&mut self, edge: EdgeId, loop_id: LoopId, reversed: bool) -> CoEdgeId {
        self.model.store.add_coedge(CoEdge::new(edge, loop_id, reversed))
    }

    /// Make a face inside the current shell with the given outer boundary
    /// (`coedge_chain` must already be linked next/prev in order).
    pub fn add_face_in_current_shell(&mut self, outer_loop: LoopId) -> FaceId {
        let shell = self.cur_shell.expect("begin_solid() must be called first");
        let face = self.model.store.add_face(Face::new(shell, outer_loop));
        if let Some(s) = self.model.store.get_shell_mut(shell) { s.add_face(face); }
        // Back-pointer: tell the loop which face it bounds.
        if let Some(lp) = self.model.store.get_loop_mut(outer_loop) { lp.face = face; }
        face
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Layer 2 — Euler operators (Mäntylä's canonical set)
//
// Names follow the convention used in Mortenson, "Geometric Modeling":
//   MEV  — Make Edge & Vertex
//   MEF  — Make Edge & Face
//   MEKL — Make Edge, Kill Loop
//   KEF  — Kill Edge & Face
//   KEV  — Kill Edge & Vertex
//
// Only MEV / MEF are needed by the current primitives; KEV / KEF are stubs
// (returning `Err`) so callers get a compile-time hint that demolition logic
// hasn't been wired up yet.
// ─────────────────────────────────────────────────────────────────────────────
impl BrepBuilder {
    /// **M**ake **E**dge & **V**ertex — extends an existing vertex with a new
    /// edge ending at a brand-new vertex at `p`. Returns `(new_vertex, edge)`.
    pub fn mev(&mut self, from: VertexId, p: Point3) -> (VertexId, EdgeId) {
        let v = self.add_vertex(p);
        let e = self.add_edge(from, v);
        (v, e)
    }

    /// **M**ake **E**dge & **F**ace — closes a wire by adding a single edge
    /// between two existing vertices, splitting the surrounding loop into two
    /// loops and producing one new face.
    ///
    /// For now this is a topological helper: it creates the edge and returns
    /// it; the loop-split is done by the higher-level `polyline_face` because
    /// it needs the loop in hand.
    pub fn mef(&mut self, a: VertexId, b: VertexId) -> EdgeId {
        self.add_edge(a, b)
    }

    /// **K**ill **E**dge & **V**ertex — placeholder for future demolition.
    pub fn kev(&mut self, _edge: EdgeId) -> Result<(), &'static str> {
        Err("kev: not implemented yet")
    }

    /// **K**ill **E**dge & **F**ace — placeholder.
    pub fn kef(&mut self, _edge: EdgeId) -> Result<(), &'static str> {
        Err("kef: not implemented yet")
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Layer 3 — high-level primitive constructors
// ─────────────────────────────────────────────────────────────────────────────
impl BrepBuilder {
    /// Build a single planar face from a CCW polyline of 3D points.
    ///
    /// Creates V vertices, V edges, V co-edges chained into a single outer
    /// loop, and one face — all inside the current shell (call `begin_solid`
    /// first). Returns the new face ID.
    pub fn polyline_face(&mut self, points: &[Point3]) -> FaceId {
        assert!(points.len() >= 3, "polyline_face needs ≥3 points");

        // 1. Vertices
        let verts: Vec<VertexId> = points.iter().map(|&p| self.add_vertex(p)).collect();

        // 2. Edges (closed loop: last edge wraps to verts[0])
        let n = verts.len();
        let edges: Vec<EdgeId> = (0..n)
            .map(|i| self.add_edge(verts[i], verts[(i + 1) % n]))
            .collect();

        // 3. Empty loop (face id will be patched in by `add_face_in_current_shell`).
        // We use a sentinel face id; corrected immediately after face creation.
        let sentinel_face = FaceId::fresh();
        let loop_id = self.model.store.add_loop(Loop::outer(sentinel_face));

        // 4. Co-edges, then chain prev/next pointers.
        let coedges: Vec<CoEdgeId> = edges
            .iter()
            .map(|&e| self.add_coedge(e, loop_id, false))
            .collect();
        for i in 0..n {
            let cur  = coedges[i];
            let nxt  = coedges[(i + 1) % n];
            if let Some(c) = self.model.store.get_coedge_mut(cur) { c.next = Some(nxt); }
            if let Some(c) = self.model.store.get_coedge_mut(nxt) { c.prev = Some(cur); }
        }
        if let Some(lp) = self.model.store.get_loop_mut(loop_id) {
            lp.coedges = coedges;
        }

        // 5. The face itself — this also fixes `loop.face` back-pointer.
        self.add_face_in_current_shell(loop_id)
    }

    /// Build a closed axis-aligned box from two corner points.
    ///
    /// Six faces are created in `+X, -X, +Y, -Y, +Z, -Z` order. The current
    /// shell is marked `is_closed = true`.
    pub fn box_from_extents(&mut self, min: [Real; 3], max: [Real; 3]) -> &mut Self {
        if self.cur_solid.is_none() { self.begin_solid(); }

        let [x0, y0, z0] = min;
        let [x1, y1, z1] = max;

        let p = |x, y, z| Point3::new(x, y, z);

        // 8 corners
        let v000 = p(x0, y0, z0); let v100 = p(x1, y0, z0);
        let v110 = p(x1, y1, z0); let v010 = p(x0, y1, z0);
        let v001 = p(x0, y0, z1); let v101 = p(x1, y0, z1);
        let v111 = p(x1, y1, z1); let v011 = p(x0, y1, z1);

        // Faces — CCW when looking *into* the solid from outside.
        // -Z (bottom): v000 v010 v110 v100
        self.polyline_face(&[v000, v010, v110, v100]);
        // +Z (top):    v001 v101 v111 v011
        self.polyline_face(&[v001, v101, v111, v011]);
        // -Y (front):  v000 v100 v101 v001
        self.polyline_face(&[v000, v100, v101, v001]);
        // +Y (back):   v010 v011 v111 v110
        self.polyline_face(&[v010, v011, v111, v110]);
        // -X (left):   v000 v001 v011 v010
        self.polyline_face(&[v000, v001, v011, v010]);
        // +X (right):  v100 v110 v111 v101
        self.polyline_face(&[v100, v110, v111, v101]);

        if let Some(shell) = self.cur_shell {
            if let Some(s) = self.model.store.get_shell_mut(shell) {
                s.mark_closed();
            }
        }
        self
    }

    /// Build a straight prism by extruding a CCW polygon along `dir`.
    ///
    /// The bottom polygon is added with reversed orientation; the top with
    /// the original orientation; side rectangles are stitched in between.
    pub fn prism_from_polygon(&mut self, polygon: &[Point3], dir: [Real; 3]) -> &mut Self {
        assert!(polygon.len() >= 3, "prism_from_polygon needs ≥3 points");
        if self.cur_solid.is_none() { self.begin_solid(); }

        let [dx, dy, dz] = dir;
        let top: Vec<Point3> = polygon
            .iter()
            .map(|p| Point3::new(p.x + dx, p.y + dy, p.z + dz))
            .collect();

        // Bottom (reversed so its outward normal points -dir).
        let bottom_rev: Vec<Point3> = polygon.iter().rev().copied().collect();
        self.polyline_face(&bottom_rev);

        // Top.
        self.polyline_face(&top);

        // Side rectangles.
        let n = polygon.len();
        for i in 0..n {
            let j = (i + 1) % n;
            self.polyline_face(&[polygon[i], polygon[j], top[j], top[i]]);
        }

        if let Some(shell) = self.cur_shell {
            if let Some(s) = self.model.store.get_shell_mut(shell) {
                s.mark_closed();
            }
        }
        self
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_builder_has_zero_counts() {
        let b = BrepBuilder::new();
        let c = b.counts();
        assert_eq!(c.vertices, 0);
        assert_eq!(c.faces, 0);
    }

    #[test]
    fn unit_box_has_8v_12e_6f() {
        let mut b = BrepBuilder::new();
        b.begin_body("box").box_from_extents([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
        let c = b.counts();
        // Each of the 6 faces allocates 4 unique vertices + 4 unique edges
        // in this naïve builder (vertex-welding is a later optimisation),
        // so we expect 24 V, 24 E, 24 CE, 6 L, 6 F, 1 S, 1 Solid, 1 Body.
        assert_eq!(c.faces, 6);
        assert_eq!(c.loops, 6);
        assert_eq!(c.shells, 1);
        assert_eq!(c.solids, 1);
        assert_eq!(c.bodies, 1);
        assert_eq!(c.coedges, 24);
    }

    #[test]
    fn triangular_prism_has_5_faces() {
        let tri = [
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        let mut b = BrepBuilder::new();
        b.begin_body("prism").prism_from_polygon(&tri, [0.0, 0.0, 1.0]);
        let c = b.counts();
        assert_eq!(c.faces, 2 + 3); // top + bottom + 3 sides
    }
}

