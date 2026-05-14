// ── Select Tool ──────────────────────────────────────────────────────────────
// Handles:
//   __handleSketchClick   — click dispatch for select / point / line / delete
//   __handleSketchDoubleClick — double-click profile / edge selection
//
// Loaded before sketch_tools.rs (which still owns grab/copy/constraints).

pub const JS: &str = r##"
      // ─────────────────────────────────────────────────────────
      // __handleSketchClick(ndcX, ndcY, shiftKey)
      // ─────────────────────────────────────────────────────────
      window.__handleSketchClick = function(ndcX, ndcY, shiftKey) {
        // Ignore clicks that are the tail end of a wheel/pinch zoom gesture.
        if (window.__wheelZoomActive) return;

        const SM   = window.SelectionMode;
        const tool = sketchState.activeTool;

        if (sketchState.grab.active) { window.__confirmGrab(); return; }
        if (sketchState.copy.active) { window.__confirmCopyConnect(); return; }

        if (tool === SM.SELECT) {
          const pId = window.__pickPointAt(ndcX, ndcY);
          const eId = pId ? null : window.__pickEdgeAt(ndcX, ndcY);
          if (!shiftKey) {
            sketchState.selectedPointIds.clear();
            sketchState.selectedEdgeIds.clear();
            sketchState.selectedProfileId = null;
          }
          if (pId) {
            if (sketchState.selectedPointIds.has(pId)) sketchState.selectedPointIds.delete(pId);
            else                                       sketchState.selectedPointIds.add(pId);
          } else if (eId) {
            if (sketchState.selectedEdgeIds.has(eId)) sketchState.selectedEdgeIds.delete(eId);
            else                                      sketchState.selectedEdgeIds.add(eId);
          } else {
            // Profile picking — only on the active working plane.
            const hit = window.__raycastSketchPlane(ndcX, ndcY);
            if (hit) {
              const profId = window.__pickProfileAtWorld(hit.freeX, hit.freeY, hit.freeZ);
              if (profId) {
                window.__selectProfile(profId);
              } else if (!shiftKey) {
                sketchState.selectedProfileId = null;
              }
            }
          }
          if (window.__updateSketchInspector) window.__updateSketchInspector();
          return;
        }

        // ── Resolve current snap target (single source of truth for geometry creation) ──
        // Used by POINT and LINE tools. Refreshed at click time so it works even
        // on touch / first-frame click where pointermove never fired.
        function __resolveClickSnap() {
          // Priority 0: fresh pick at click-time NDC — most reliable, avoids
          // timing gaps between pointermove and pointerup clearing hoverPointId.
          const freshPickId = window.__pickPointAt ? window.__pickPointAt(ndcX, ndcY) : null;
          const freshPickData = freshPickId ? sketchState.points.find(p => p.id === freshPickId) : null;
          if (freshPickId && freshPickData) {
            return {
              kind: 'point',
              pointId: freshPickId,
              gx: freshPickData.gx, gy: freshPickData.gy, gz: freshPickData.gz,
              x: freshPickData.x, y: freshPickData.y, z: freshPickData.z,
              valid: true,
            };
          }

          // Priority 1: use the already-highlighted hover point (set by pointermove
          // point-snap priority). This is what the user sees as the yellow dot.
          if (sketchState.hoverPointId && sketchState.hoverWorld) {
            const hw = sketchState.hoverWorld;
            return {
              kind: 'point',
              pointId: sketchState.hoverPointId,
              gx: hw.gx, gy: hw.gy, gz: hw.gz,
              x: hw.x, y: hw.y, z: hw.z,
              valid: true,
            };
          }

          // Priority 2: snap from plane raycast + resolveSnapTarget
          const canvasEl = document.getElementById('matterCanvas');
          const mpx = canvasEl
            ? { x: (ndcX + 1) * 0.5 * canvasEl.width, y: (1 - ndcY) * 0.5 * canvasEl.height }
            : { x: 0, y: 0 };

          const hit = window.__raycastSketchPlane(ndcX, ndcY);

          // Fallback: even if the ray misses the working plane (oblique camera),
          // snap to a nearby existing point — so connecting lines always works.
          if (!hit) {
            if (!window.__resolveSnapTarget) return null;
            // Use camera-origin projected onto working plane as a dummy free pos.
            const dummy = { x: 0, y: 0, z: 0 };
            const result = window.__resolveSnapTarget(dummy, mpx, { force: true });
            if (result && result.kind === 'point') return result;
            return null;
          }

          if (!window.__resolveSnapTarget) {
            return {
              kind: 'grid', pointId: null,
              gx: hit.gx, gy: hit.gy, gz: hit.gz,
              x: hit.x, y: hit.y, z: hit.z, valid: true,
            };
          }
          return window.__resolveSnapTarget(
            { x: hit.freeX, y: hit.freeY, z: hit.freeZ },
            mpx,
            { force: true },
          );
        }

        if (tool === SM.POINT) {
          const snap = __resolveClickSnap();
          if (!snap || !snap.valid) return;
          if (snap.kind === 'point') return;
          const mode = sketchState.engineMode || 'backend';
          if (mode === 'wasm' || mode === 'hybrid') {
            window.__pushHistory();
            window.__wasmAddPointAndApply(snap.gx, snap.gy, snap.gz).then(() => {
              if (window.__updateSketchInspector) window.__updateSketchInspector();
            });
            return;
          }
          if (mode === 'backend') {
            window.__pushHistory();
            window.__backendAddPoint(snap.gx, snap.gy, snap.gz).then(() => {
              if (window.__updateSketchInspector) window.__updateSketchInspector();
            });
            return;
          }
          window.__pushHistory();
          window.__addPoint(snap.gx, snap.gy, snap.gz);
          if (window.__updateSketchInspector) window.__updateSketchInspector();
          return;
        }

        if (tool === SM.LINE || tool === 'line') {
          if (window.__lineClick) window.__lineClick(ndcX, ndcY);
          return;
        }

        if (tool === SM.DELETE) {
          const pId = window.__pickPointAt(ndcX, ndcY);
          if (pId) {
            window.__pushHistory();
            sketchState.selectedPointIds = new Set([pId]);
            sketchState.selectedEdgeIds  = new Set();
            window.__deleteSelected();
          } else {
            const eId = window.__pickEdgeAt(ndcX, ndcY);
            if (eId) {
              window.__pushHistory();
              sketchState.selectedEdgeIds  = new Set([eId]);
              sketchState.selectedPointIds = new Set();
              window.__deleteSelected();
            }
          }
          if (window.__updateSketchInspector) window.__updateSketchInspector();
          return;
        }
      };

      // ─────────────────────────────────────────────────────────
      // __lineClick(ndcX, ndcY) — Line Tool click handler
      //
      // Click 1 on point/grid → save startPointId + startWorld
      // Click 2 on point/grid → create edge A→B, reset line state
      // Enter → finish mode (no edge)
      // ─────────────────────────────────────────────────────────
      window.__lineClick = function(ndcX, ndcY) {
        // Resolve snap at click time (same priority-0 fresh-pick logic)
        const freshPickId   = window.__pickPointAt ? window.__pickPointAt(ndcX, ndcY) : null;
        const freshPickData = freshPickId ? sketchState.points.find(p => p.id === freshPickId) : null;

        let snap = null;
        if (freshPickId && freshPickData) {
          snap = {
            kind: 'point', pointId: freshPickId,
            gx: freshPickData.gx, gy: freshPickData.gy, gz: freshPickData.gz,
            x: freshPickData.x, y: freshPickData.y, z: freshPickData.z, valid: true,
          };
        } else {
          // fallback: hoverWorld from pointermove
          const hw = sketchState.hoverWorld;
          if (hw) {
            const snapPtId = (sketchState.snap && sketchState.snap.kind === 'point') ? sketchState.snap.pointId : null;
            snap = {
              kind: snapPtId ? 'point' : (hw.snapKind || 'grid'),
              pointId: snapPtId || null,
              gx: hw.gx, gy: hw.gy, gz: hw.gz,
              x: hw.x, y: hw.y, z: hw.z, valid: true,
            };
          } else {
            // Last resort: raycast
            const hit = window.__raycastSketchPlane && window.__raycastSketchPlane(ndcX, ndcY);
            if (hit && window.__resolveSnapTarget) {
              const canvasEl = document.getElementById('matterCanvas');
              const mpx = canvasEl
                ? { x: (ndcX+1)*0.5*canvasEl.width, y: (1-ndcY)*0.5*canvasEl.height }
                : { x: 0, y: 0 };
              snap = window.__resolveSnapTarget({ x: hit.freeX, y: hit.freeY, z: hit.freeZ }, mpx, { force: true });
            }
          }
        }

        if (!snap || !snap.valid) {
          window.__setStatusMessage('Line: no snap target');
          return;
        }

        console.log('[Line] click snap=', snap, 'line=', sketchState.line);

        const line = sketchState.line;

        // ── FIRST CLICK ──
        if (!line.active) {
          sketchState.line = {
            active: true,
            startPointId: snap.kind === 'point' ? snap.pointId : null,
            startWorld: { x: snap.x, y: snap.y, z: snap.z, gx: snap.gx, gy: snap.gy, gz: snap.gz },
          };
          const label = snap.kind === 'point' ? 'point · click second point' : 'grid · click second point';
          window.__setStatusMessage('Line start set · ' + label);
          return;
        }

        // ── SECOND CLICK ──
        window.__finishLineClick(snap);
      };

      window.__finishLineClick = async function(snap) {
        const line = sketchState.line;
        if (!line || !line.active || !line.startWorld) return;

        const g = sketchState.gridSize || 1.0;
        const mode = sketchState.engineMode || 'backend';

        let aId = line.startPointId;
        let bId = snap.kind === 'point' ? snap.pointId : null;

        window.__pushHistory();

        // Create start point if it was on grid (no existing point)
        if (!aId) {
          const a = line.startWorld;
          const gx = Math.round(a.x / g), gy = Math.round(a.y / g), gz = Math.round(a.z / g);
          if (mode === 'backend') {
            const r = await window.__backendAddPoint(gx, gy, gz);
            aId = r?.pointId || null;
          } else if (mode === 'wasm' || mode === 'hybrid') {
            const r = await window.__wasmAddPointAndApply(gx, gy, gz);
            aId = r?.ok ? r.pointId : null;
          } else {
            aId = window.__addPoint(gx, gy, gz)?.id || null;
          }
        }

        // Create end point if it was on grid (no existing point)
        if (!bId) {
          const gx = Math.round(snap.x / g), gy = Math.round(snap.y / g), gz = Math.round(snap.z / g);
          if (mode === 'backend') {
            const r = await window.__backendAddPoint(gx, gy, gz);
            bId = r?.pointId || null;
          } else if (mode === 'wasm' || mode === 'hybrid') {
            const r = await window.__wasmAddPointAndApply(gx, gy, gz);
            bId = r?.ok ? r.pointId : null;
          } else {
            bId = window.__addPoint(gx, gy, gz)?.id || null;
          }
        }

        if (!aId || !bId) {
          window.__setStatusMessage('Line: failed to resolve points');
          sketchState.line = { active: false, startPointId: null, startWorld: null };
          return;
        }
        if (aId === bId) {
          window.__setStatusMessage('Line: same point — click a different location');
          sketchState.line = { active: false, startPointId: null, startWorld: null };
          return;
        }

        if (mode === 'backend') {
          await window.__backendAddEdge({ pointId: aId }, { pointId: bId });
        } else if (mode === 'wasm' || mode === 'hybrid') {
          await window.__wasmAddEdgeAndApply({ pointId: aId }, { pointId: bId });
        } else {
          window.__addEdge(aId, bId);
        }

        sketchState.line = { active: false, startPointId: null, startWorld: null };
        sketchState.phase = 'idle';

        if (window.__notifySketchChanged) window.__notifySketchChanged();
        if (window.__updateSketchInspector) window.__updateSketchInspector();
        window.__setStatusMessage('Line created · click to start new line');
      };

      // ─────────────────────────────────────────────────────────
      // __handleSketchDoubleClick(ndcX, ndcY)
      // Select tool: double-click edge → select profile or endpoints.
      // ─────────────────────────────────────────────────────────
      window.__handleSketchDoubleClick = function(ndcX, ndcY) {
        if (sketchState.activeTool !== "select") return;
        const eId = window.__pickEdgeAt(ndcX, ndcY);
        if (!eId) {
          const hit = window.__raycastSketchPlane(ndcX, ndcY);
          if (hit) {
            const profId = window.__pickProfileAtWorld(hit.freeX, hit.freeY, hit.freeZ);
            if (profId) {
              const prof = window.__selectProfile(profId);
              if (prof) {
                sketchState.selectedPointIds = new Set(prof.pointIds);
                sketchState.selectedEdgeIds  = new Set(prof.edgeIds);
              }
              if (window.__updateSketchInspector) window.__updateSketchInspector();
            }
          }
          return;
        }
        const profs = window.__getProfilesForEdge(eId);
        let prof = null;
        if (profs.length === 1) prof = profs[0];
        else if (profs.length > 1) {
          prof = profs
            .map(p => ({ p, a: window.__profileArea(p) }))
            .filter(h => h.a > 0)
            .sort((a, b) => a.a - b.a)[0]?.p || profs[0];
        }
        if (prof) {
          if (sketchState.selectedProfileId === prof.id) {
            sketchState.selectedPointIds = new Set(prof.pointIds);
            sketchState.selectedEdgeIds  = new Set(prof.edgeIds);
          } else {
            window.__selectProfile(prof.id);
            sketchState.selectedPointIds = new Set(prof.pointIds);
            sketchState.selectedEdgeIds  = new Set(prof.edgeIds);
          }
        } else {
          const e = sketchState.edges.find(x => x.id === eId);
          if (!e) return;
          sketchState.selectedPointIds = new Set([e.a, e.b]);
          sketchState.selectedEdgeIds  = new Set([eId]);
        }
        if (window.__updateSketchInspector) window.__updateSketchInspector();
      };
"##;
