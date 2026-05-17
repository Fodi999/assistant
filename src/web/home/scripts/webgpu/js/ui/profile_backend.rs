// ── Profile Backend Bridge ────────────────────────────────────────────────
// Connects Profile Check popup to backend CAD engine endpoints:
//   POST /api/matter/sketch/profile/analyze
//   POST /api/matter/sketch/profile/repair
//
// API contract:
//   - All coordinates sent in mm (x_mm, y_mm, z_mm)
//   - Backend errors highlight edge_id on canvas (window.__highlightEdgeError)
//   - Falls back to local analyze if backend unreachable (network error / 5xx)
//   - Never called on mousemove — only on explicit user action (button click)

pub const JS: &str = r##"
(function() {

  // ── Serialise sketchState to backend sketch wire format ──────────────
  function _sketchPayload() {
    const ss = window.sketchState;
    if (!ss) return null;
    return {
      schema: 'sketch_graph',
      version: 1,
      workingPlane: ss.workingPlane || 'XZ',
      gridSize: (ss.precision && ss.precision.displayGridStepM) || 0.01,
      points: (ss.points || []).map(p => ({
        id: p.id,
        gx: p.gx || 0, gy: p.gy || 0, gz: p.gz || 0,
        x: p.x, y: p.y, z: p.z,
      })),
      edges: (ss.edges || []).map(e => ({
        id: e.id,
        a: e.a, b: e.b,
        kind: e.kind || 'line',
      })),
      constraints: (ss.constraints || []),
    };
  }

  // ── POST helper with timeout + fallback ──────────────────────────────
  async function _post(url, body) {
    const ctrl = new AbortController();
    const tid  = setTimeout(() => ctrl.abort(), 5000); // 5 s timeout
    try {
      const resp = await fetch(url, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
        signal: ctrl.signal,
      });
      clearTimeout(tid);
      if (!resp.ok) {
        const text = await resp.text().catch(() => '');
        return { __httpError: resp.status, message: text };
      }
      return await resp.json();
    } catch (err) {
      clearTimeout(tid);
      return { __networkError: true, message: String(err) };
    }
  }

  // ── Highlight problem edges on canvas ────────────────────────────────
  function _highlightEdgeErrors(issues) {
    if (!window.__highlightEdgeError) return;
    // Clear previous highlights first
    if (window.__clearEdgeHighlights) window.__clearEdgeHighlights();
    for (const issue of (issues || [])) {
      if (issue.edge_id && issue.severity === 'error') {
        window.__highlightEdgeError(issue.edge_id, '#ef4444'); // red
      } else if (issue.edge_id && issue.severity === 'warn') {
        window.__highlightEdgeError(issue.edge_id, '#f59e0b'); // amber
      }
    }
  }

  // ── Apply repaired points to sketchState ─────────────────────────────
  function _applyRepairedPoints(repairedPoints) {
    const ss = window.sketchState;
    if (!ss || !ss.points) return 0;
    let count = 0;
    for (const rp of (repairedPoints || [])) {
      const pt = ss.points.find(p => p.id === rp.id);
      if (!pt) continue;
      pt.x = rp.x; pt.y = rp.y; pt.z = rp.z;
      // Keep grid coords in sync if possible
      const gridM = (ss.precision && ss.precision.displayGridStepM) || 0.001;
      pt.gx = Math.round(rp.x / gridM);
      pt.gy = Math.round(rp.y / gridM);
      pt.gz = Math.round(rp.z / gridM);
      count++;
    }
    return count;
  }

  function _redrawAll() {
    if (window.__recomputeProfiles)  window.__recomputeProfiles();
    if (window.__recomputeValidation) window.__recomputeValidation();
    if (window.__requestRedraw)      window.__requestRedraw();
    else if (window.__rafDirty !== undefined) window.__rafDirty = true;
  }

  // ── window.__backendAnalyzeProfile(profile) ──────────────────────────
  // Returns a report object compatible with the existing __analyzeProfile format.
  // Falls back to local __analyzeProfile on network/server error.
  window.__backendAnalyzeProfile = async function(profile) {
    const sketch = _sketchPayload();
    if (!sketch || !profile) {
      return window.__analyzeProfile ? window.__analyzeProfile(profile) : null;
    }

    const result = await _post('/api/matter/sketch/profile/analyze', {
      sketch,
      profile_id: profile.id,
    });

    // Network / server error → fall back to local
    if (result.__networkError || result.__httpError) {
      console.warn('[ProfileBackend] analyze fallback (local):', result.message);
      return window.__analyzeProfile ? window.__analyzeProfile(profile) : null;
    }

    // Map backend response to popup report format
    _highlightEdgeErrors(result.issues);

    return {
      ok: result.error_count === 0,
      source: 'backend',
      type: result.profile_type || '—',
      widthMm:  result.width_mm  || 0,
      heightMm: result.height_mm || 0,
      areaMm2:  result.area_mm2  || 0,
      perimeter: result.perimeter_mm || 0,
      errors: (result.issues || []).map(i => ({
        kind:          i.kind,
        severity:      i.severity,
        edgeId:        i.edge_id,
        vertexPointId: i.vertex_point_id,
        driftMm:       i.drift_mm,
        actualMm:      i.actual_mm,
        expectedMm:    i.expected_mm,
        angleDeg:      i.angle_deg,
        orient:        i.orient,
        message:       i.message,
      })),
    };
  };

  // ── window.__backendRepairProfile(profile, repairType) ───────────────
  // repairType: "FIX_RECTANGLE" | "FIX_SQUARE" | "EQUALIZE_EDGES"
  // Returns { ok, avgMm?, widthMm?, heightMm?, error? }
  window.__backendRepairProfile = async function(profile, repairType) {
    const sketch = _sketchPayload();
    if (!sketch || !profile) {
      return { ok: false, error: 'No sketch / profile' };
    }

    const result = await _post('/api/matter/sketch/profile/repair', {
      sketch,
      profile_id: profile.id,
      repair_type: repairType,
    });

    if (result.__networkError || result.__httpError) {
      console.warn('[ProfileBackend] repair fallback (local):', result.message);
      // Fall back to local repair functions
      if (repairType === 'FIX_RECTANGLE' && window.__makeSelectedProfileRectangle) {
        return window.__makeSelectedProfileRectangle();
      }
      if (repairType === 'FIX_SQUARE' && window.__makeSelectedProfileSquare) {
        return window.__makeSelectedProfileSquare();
      }
      if (repairType === 'EQUALIZE_EDGES' && window.__equalizeSelectedEdges) {
        return window.__equalizeSelectedEdges();
      }
      return { ok: false, error: result.message };
    }

    if (!result.ok) {
      return { ok: false, error: result.error || 'Repair failed' };
    }

    // Apply repaired coords to sketchState
    const count = _applyRepairedPoints(result.repaired_points);
    _redrawAll();

    return {
      ok: true,
      count,
      avgMm:   result.avg_mm,
      widthMm: result.width_mm,
      heightMm: result.height_mm,
    };
  };

})();
"##;
