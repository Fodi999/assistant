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

        // Grab command mode: click outside gizmo handles does NOT auto-confirm.
        // Confirm only happens via: gizmo pointerup (after real drag), Enter key, or numeric input.
        // In command mode (dragging=false) a click outside should be silently ignored.
        if (sketchState.grab?.active) {
          if (sketchState.grab.dragging) {
            window.__confirmGrab().catch(e => console.warn('[select] confirmGrab err', e));
          }
          // else: command mode — swallow the click, keep grab open
          return;
        }
        if (sketchState.copy.active) { window.__confirmCopyConnect(); return; }

        if (tool === SM.SELECT) {
          const gsm = sketchState.geomSelMode || 'edge';
          console.log('[SelectClick] gsm =', gsm);

          if (!shiftKey) {
            sketchState.selectedPointIds.clear();
            sketchState.selectedEdgeIds.clear();
            sketchState.selectedFaceIds.clear();
            sketchState.selectedBodyIds.clear();
            sketchState.selectedProfileId = null;
          }

          if (gsm === 'vertex') {
            // ── Vertex mode: only pick points ──────────────────────────────
            const pId = window.__pickPointAt(ndcX, ndcY);
            if (pId) {
              if (sketchState.selectedPointIds.has(pId)) sketchState.selectedPointIds.delete(pId);
              else                                       sketchState.selectedPointIds.add(pId);
            }
            if (window.__updateSketchInspector) window.__updateSketchInspector();
            return;
          }

          if (gsm === 'edge') {
            // ── Edge mode: points first, then edges (original behavior) ────
            const pId = window.__pickPointAt(ndcX, ndcY);
            const eId = pId ? null : window.__pickEdgeAt(ndcX, ndcY);
            if (pId) {
              if (sketchState.selectedPointIds.has(pId)) sketchState.selectedPointIds.delete(pId);
              else                                       sketchState.selectedPointIds.add(pId);
            } else if (eId) {
              if (sketchState.selectedEdgeIds.has(eId)) sketchState.selectedEdgeIds.delete(eId);
              else                                      sketchState.selectedEdgeIds.add(eId);
            } else {
              const hit = window.__raycastSketchPlane(ndcX, ndcY);
              if (hit) {
                const profId = window.__pickProfileAtWorld(hit.freeX, hit.freeY, hit.freeZ);
                if (profId) window.__selectProfile(profId);
                else if (!shiftKey) sketchState.selectedProfileId = null;
              }
            }
            if (window.__updateSketchInspector) window.__updateSketchInspector();
            return;
          }

          if (gsm === 'face') {
            // ── Face mode: pick profiles / wall surfaces ────────────────────
            // Highlights the profile fill + all its edges (orange outline).
            const hit = window.__raycastSketchPlane(ndcX, ndcY);
            if (hit) {
              const profId = window.__pickProfileAtWorld(hit.freeX, hit.freeY, hit.freeZ);
              if (profId) {
                if (sketchState.selectedFaceIds.has(profId)) {
                  // Deselect — remove face + edges
                  sketchState.selectedFaceIds.delete(profId);
                  sketchState.selectedEdgeIds.clear();
                  sketchState.selectedProfileId = null;
                } else {
                  sketchState.selectedFaceIds.add(profId);
                  const prof = window.__selectProfile(profId);  // sets selectedProfileId
                  // Highlight outline: add all edges of this face to selectedEdgeIds
                  if (prof && prof.edgeIds) {
                    for (const eid of prof.edgeIds) sketchState.selectedEdgeIds.add(eid);
                  }
                }
              }
            }
            if (window.__updateSketchInspector) window.__updateSketchInspector();
            return;
          }

          if (gsm === 'body') {
            // ── Body mode: select all edges+points of a profile ─────────────
            const hit = window.__raycastSketchPlane(ndcX, ndcY);
            if (hit) {
              const profId = window.__pickProfileAtWorld(hit.freeX, hit.freeY, hit.freeZ);
              if (profId) {
                if (sketchState.selectedBodyIds.has(profId)) {
                  sketchState.selectedBodyIds.delete(profId);
                  sketchState.selectedPointIds.clear();
                  sketchState.selectedEdgeIds.clear();
                  sketchState.selectedProfileId = null;
                } else {
                  sketchState.selectedBodyIds.add(profId);
                  const prof = window.__selectProfile(profId);
                  if (prof) {
                    for (const id of prof.pointIds) sketchState.selectedPointIds.add(id);
                    for (const id of prof.edgeIds)  sketchState.selectedEdgeIds.add(id);
                  }
                }
              }
            }
            if (window.__updateSketchInspector) window.__updateSketchInspector();
            return;
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
          const canvasEl = document.getElementById('webgpu-canvas');
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

        if (tool === 'rect') {
          if (window.__rectClick) window.__rectClick(ndcX, ndcY);
          return;
        }

        if (tool === 'circle') {
          if (window.__circleClick) window.__circleClick(ndcX, ndcY);
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
      // __handleSketchDoubleClick(ndcX, ndcY, px, py)
      // Select tool: double-click profile/edge → open Profile popup or select endpoints.
      // px, py = CSS pixel position from the original pointer event.
      // ─────────────────────────────────────────────────────────
      window.__handleSketchDoubleClick = function(ndcX, ndcY, px, py) {
        if (sketchState.activeTool !== "select") return;
        console.log('[DblClick] fired, tool=select, ndcX=', ndcX.toFixed(3), 'ndcY=', ndcY.toFixed(3));
        const eId = window.__pickEdgeAt(ndcX, ndcY);
        if (!eId) {
          const hit = window.__raycastSketchPlane(ndcX, ndcY);
          if (hit) {
            const profId = window.__pickProfileAtWorld(hit.freeX, hit.freeY, hit.freeZ);
            if (profId) {
              console.log('[DblClick] hit profile via raycast:', profId);
              const prof = window.__selectProfile(profId);
              if (prof) {
                sketchState.selectedPointIds = new Set(prof.pointIds);
                sketchState.selectedEdgeIds  = new Set(prof.edgeIds);
              }
              if (window.__updateSketchInspector) window.__updateSketchInspector();
              // Open Profile Check popup at click position.
              if (window.__openProfilePopup) window.__openProfilePopup(px || 200, py || 200);
            }
          }
          return;
        }
        const profs = window.__getProfilesForEdge(eId);
        console.log('[DblClick] hit edge:', eId, '→ profiles:', profs.map(p=>p.id));
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
            // Already selected — open popup.
            if (window.__openProfilePopup) window.__openProfilePopup(px || 200, py || 200);
          } else {
            window.__selectProfile(prof.id);
            sketchState.selectedPointIds = new Set(prof.pointIds);
            sketchState.selectedEdgeIds  = new Set(prof.edgeIds);
            // First selection — open popup immediately too.
            if (window.__openProfilePopup) window.__openProfilePopup(px || 200, py || 200);
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
