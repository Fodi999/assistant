// ── JS: Constraint Solver — HORIZONTAL / VERTICAL / EQUAL_LENGTH ─────────────
//
// Exposes:
//   window.__applyConstraint(constraintObj)  → applies one constraint via WASM/backend
//   window.__solveConstraints()              → solves all sketch.constraints
//   window.__addConstraint(type, targetType, targetId, value?)  → add + solve
//
// Strategy:
//   1. Try WASM (synchronous, instant)
//   2. On WASM miss, try backend POST /api/matter/sketch/solve-constraints
//   3. On backend error, show message and leave sketch unchanged
//
// After a successful solve: patches sketchState points/edges/constraints,
// calls __recomputeProfiles(), __recomputeValidation(), __redrawSketch().

pub const JS: &str = r##"
(function registerConstraintSolver() {

  // ── Helpers ────────────────────────────────────────────────────────────────
  function sketchJSON() {
    if (window.__sketchToJSON) return window.__sketchToJSON();
    if (!window.sketchState) return null;
    const ss = window.sketchState;
    return {
      schema: 'sketch_graph', version: 1,
      workingPlane: ss.workingPlane || 'XZ',
      gridSize: (ss.precision && ss.precision.displayGridStepM) || 0.01,
      points: ss.points || [],
      edges:  ss.edges  || [],
      constraints: ss.constraints || [],
    };
  }

  function patchSketch(solved) {
    if (!solved || !solved.ok || !solved.sketch) return;
    const ss = window.sketchState;
    if (!ss) return;
    // Patch points (only update coords, preserve other fields)
    const byId = {};
    for (const p of (solved.sketch.points || [])) byId[p.id] = p;
    for (let i = 0; i < ss.points.length; i++) {
      const upd = byId[ss.points[i].id];
      if (upd) {
        ss.points[i].gx = upd.gx; ss.points[i].gy = upd.gy; ss.points[i].gz = upd.gz;
        ss.points[i].x  = upd.x;  ss.points[i].y  = upd.y;  ss.points[i].z  = upd.z;
      }
    }
    console.log('[WASM SOLVE APPLIED] points updated:', JSON.parse(JSON.stringify(ss.points)));
    // Trigger recompute
    if (window.__recomputeProfiles)   window.__recomputeProfiles();
    if (window.__recomputeValidation) window.__recomputeValidation();
    if (window.__redrawSketch)        window.__redrawSketch();
    if (window.__updateSketchInspector) window.__updateSketchInspector();
  }

  function status(msg, color) {
    if (window.__setStatusMessage) window.__setStatusMessage(msg);
    console.log('[constraint]', msg);
  }

  // ── WASM path ───────────────────────────────────────────────────────────────
  function tryWasm(payload) {
    try {
      const wasm = window.__wasmModule;
      if (wasm && typeof wasm.wasm_solve_constraints === 'function') {
        console.log('[WASM SOLVE INPUT]', JSON.stringify(payload, null, 2));
        const raw = wasm.wasm_solve_constraints(JSON.stringify(payload));
        console.log('[WASM SOLVE RAW]', raw);
        const res = JSON.parse(raw);
        console.log('[WASM SOLVE OUTPUT]', res);
        return res;
      } else {
        console.warn('[WASM SOLVE] __wasmModule.wasm_solve_constraints not available');
      }
    } catch (e) {
      console.warn('[constraint] WASM error:', e);
    }
    return null;
  }

  // ── Backend path ────────────────────────────────────────────────────────────
  async function tryBackend(payload) {
    try {
      const resp = await fetch('/api/matter/sketch/solve-constraints', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload),
      });
      if (!resp.ok) {
        const err = await resp.json().catch(() => ({}));
        throw new Error(err.error || resp.statusText);
      }
      return await resp.json();
    } catch (e) {
      console.warn('[constraint] backend error:', e.message);
      return null;
    }
  }

  // ── Core apply ──────────────────────────────────────────────────────────────
  async function applyPayload(payload) {
    // 1. WASM
    let result = tryWasm(payload);
    if (result && result.ok) {
      patchSketch(result);
      const moved = (result.results || []).flatMap(r => r.moved_points || []).length;
      status(`✓ Constraint applied (${moved} pts moved)`);
      return result;
    }
    // 2. Backend
    result = await tryBackend(payload);
    if (result && result.ok) {
      patchSketch(result);
      const moved = (result.results || []).flatMap(r => r.moved_points || []).length;
      status(`✓ Constraint applied via backend (${moved} pts moved)`);
      return result;
    }
    // 3. Failure
    const err = result && result.results && result.results[0]
      ? result.results[0].message : 'Constraint solve failed';
    status('⚠ ' + err);
    return result || { ok: false };
  }

  // ── Public API ───────────────────────────────────────────────────────────────

  /// Apply a single constraint object (preview or one-shot).
  /// constraintObj = { type, targetType, targetId, value? }
  window.__applyConstraint = async function(constraintObj) {
    const sketch = sketchJSON();
    if (!sketch) return;
    return applyPayload({ sketch, constraint: constraintObj });
  };

  /// Solve ALL constraints currently in sketchState.constraints.
  window.__solveConstraints = async function() {
    const sketch = sketchJSON();
    if (!sketch) return;
    if (!sketch.constraints || sketch.constraints.length === 0) {
      status('No constraints to solve');
      return;
    }
    return applyPayload({ sketch });
  };

  /// Convenience: add constraint to sketchState + immediately solve.
  /// type       : 'HORIZONTAL' | 'VERTICAL' | 'EQUAL_LENGTH'
  /// targetType : 'edge' | 'point'
  /// targetId   : edge id (or "edgeA,edgeB" for EQUAL_LENGTH)
  /// value?     : optional numeric value (e.g. fixed length in mm)
  window.__addConstraint = async function(type, targetType, targetId, value) {
    const ss = window.sketchState;
    if (!ss) return;
    if (!ss.constraints) ss.constraints = [];
    // Normalize type to uppercase (solver expects HORIZONTAL, VERTICAL etc.)
    const normalizedType = (type || '').toUpperCase();
    const cid = normalizedType + '_' + targetId + '_' + Date.now();
    const c = { id: cid, type: normalizedType, targetType, targetId };
    if (value !== undefined) c.value = value;
    // Avoid duplicates
    const already = ss.constraints.find(x => x.type === normalizedType && x.targetId === targetId);
    if (already) {
      status('Constraint already exists: ' + normalizedType + ' on ' + targetId);
      return;
    }
    ss.constraints.push(c);
    const result = await window.__applyConstraint(c);
    if (!result || !result.ok) {
      // Rollback
      ss.constraints = ss.constraints.filter(x => x.id !== cid);
    }
    return result;
  };

  /// Remove a constraint by id and re-solve remaining.
  window.__removeConstraint = async function(constraintId) {
    const ss = window.sketchState;
    if (!ss || !ss.constraints) return;
    ss.constraints = ss.constraints.filter(c => c.id !== constraintId);
    await window.__solveConstraints();
  };

})();
"##;
