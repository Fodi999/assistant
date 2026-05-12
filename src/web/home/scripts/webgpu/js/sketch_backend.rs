// ── JS: Backend precision sketch commands (Phase 7) ───────────────────────
// Domain: Sketch — POSTs to Rust backend for point/edge creation.
//
// Backend is the source of geometric truth. Frontend preview remains instant;
// these helpers only fire on **confirm** (click), never on hover/move.

pub const JS: &str = r##"
      // ─────────────────────────────────────────────────────────
      // Apply a SketchCommandResult.sketch to local sketchState.
      // Mirrors __sketchFromJSON but without the history push (the
      // caller already pushed before sending the command).
      // ─────────────────────────────────────────────────────────
      window.__applyBackendSketchResult = function(result) {
        if (!result || !result.sketch) return false;
        const g = result.sketch;
        const gridSize = (g.gridSize == null) ? sketchState.gridSize : g.gridSize;
        sketchState.gridSize     = gridSize;
        sketchState.workingPlane = g.workingPlane || sketchState.workingPlane;
        sketchState.plane        = sketchState.workingPlane;

        sketchState.points = (g.points || []).map(p => ({
          id: p.id,
          gx: p.gx, gy: p.gy, gz: p.gz,
          x: (typeof p.x === 'number') ? p.x : p.gx * gridSize,
          y: (typeof p.y === 'number') ? p.y : p.gy * gridSize,
          z: (typeof p.z === 'number') ? p.z : p.gz * gridSize,
        }));
        sketchState.edges = (g.edges || []).map(e => ({ id: e.id, a: e.a, b: e.b }));
        sketchState.constraints = (g.constraints || []).map(c => ({
          id: c.id || ('c_be_' + Date.now()),
          type: c.type, targetType: c.targetType, targetId: c.targetId,
          value: (c.value == null) ? null : c.value,
        }));
        sketchState.profiles = (g.profiles || []).map(pf => ({
          id: pf.id,
          pointIds: Array.isArray(pf.pointIds) ? [...pf.pointIds] : [],
          edgeIds:  Array.isArray(pf.edgeIds)  ? [...pf.edgeIds]  : [],
          plane: pf.plane || sketchState.workingPlane,
          closed: !!pf.closed,
        }));

        sketchState.backendStatus = {
          ok: !!result.ok,
          message: result.message || null,
          lastValidation: result.validation || null,
        };

        if (window.__notifySketchChanged) window.__notifySketchChanged();
        return true;
      };

      // ─────────────────────────────────────────────────────────
      // POST helper — returns { ok, json } or { ok:false, error }.
      // Network failures are swallowed so callers can fall back gracefully.
      // ─────────────────────────────────────────────────────────
      async function __postSketchCommand(path, body) {
        try {
          const res = await fetch(path, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(body),
          });
          if (!res.ok) {
            return { ok: false, error: 'HTTP ' + res.status };
          }
          const json = await res.json();
          return { ok: true, json };
        } catch (e) {
          return { ok: false, error: String(e && e.message || e) };
        }
      }

      // ─────────────────────────────────────────────────────────
      // __backendAddPoint(gx, gy, gz) -> Promise<{ok, pointId, created, message}>
      // ─────────────────────────────────────────────────────────
      window.__backendAddPoint = async function(gx, gy, gz) {
        const payload = {
          sketch:       window.__sketchToJSON(),
          workingPlane: sketchState.workingPlane,
          gridSize:     sketchState.gridSize,
          gx: gx | 0, gy: gy | 0, gz: gz | 0,
        };
        const r = await __postSketchCommand('/api/matter/sketch/add-point', payload);
        if (!r.ok) {
          sketchState.backendStatus = { ok: false, message: 'Backend unavailable', lastValidation: null };
          window.__setStatusMessage('Backend unavailable — using local sketch mode');
          return { ok: false, error: r.error };
        }
        const result = r.json;
        if (result.ok) {
          window.__applyBackendSketchResult(result);
          const pid = result.createdPointId || result.reusedPointId;
          window.__setStatusMessage(result.message || ('Backend point ' + pid));
          return {
            ok: true,
            pointId: pid,
            created: !!result.createdPointId,
            message: result.message,
          };
        }
        sketchState.backendStatus = {
          ok: false,
          message: result.message || 'Backend rejected request',
          lastValidation: result.validation || null,
        };
        window.__setStatusMessage(result.message || 'Backend rejected request');
        return { ok: false, error: result.message };
      };

      // ─────────────────────────────────────────────────────────
      // __backendAddEdge(startRef, endRef)
      //   refs: { pointId } OR { gx, gy, gz }
      // ─────────────────────────────────────────────────────────
      window.__backendAddEdge = async function(startRef, endRef) {
        const payload = {
          sketch:       window.__sketchToJSON(),
          workingPlane: sketchState.workingPlane,
          gridSize:     sketchState.gridSize,
          start: startRef,
          end:   endRef,
        };
        const r = await __postSketchCommand('/api/matter/sketch/add-edge', payload);
        if (!r.ok) {
          sketchState.backendStatus = { ok: false, message: 'Backend unavailable', lastValidation: null };
          window.__setStatusMessage('Backend unavailable — using local sketch mode');
          return { ok: false, error: r.error };
        }
        const result = r.json;
        // Even when ok=false (duplicate, self-loop), apply the returned sketch
        // because the backend may have inserted new endpoint points already.
        if (result.sketch) window.__applyBackendSketchResult(result);
        if (result.ok) {
          window.__setStatusMessage(result.message || 'Backend created edge');
          return {
            ok: true,
            edgeId: result.createdEdgeId,
            createdPointId: result.createdPointId,
            message: result.message,
          };
        }
        window.__setStatusMessage(result.message || 'Backend rejected edge');
        return { ok: false, error: result.message };
      };

      // ─────────────────────────────────────────────────────────
      // Toggle Backend on/off (also exposed in inspector UI).
      // ─────────────────────────────────────────────────────────
      window.__setUseBackendCommands = function(v) {
        sketchState.useBackendCommands = !!v;
        window.__setStatusMessage('Backend commands: ' + (v ? 'ON' : 'OFF'));
        if (window.__updateSketchInspector) window.__updateSketchInspector();
      };
"##;
