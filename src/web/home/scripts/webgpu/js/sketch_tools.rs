// ── JS: Sketch tools — select / point / line / grab / delete + constraints ──
// Domain: Sketch — click dispatch, hotkeys, line chain, grab, history.

pub const JS: &str = r##"
      // ─────────────────────────────────────────────────────────
      // __handleSketchClick(ndcX, ndcY, shiftKey)
      // ─────────────────────────────────────────────────────────
      window.__handleSketchClick = function(ndcX, ndcY, shiftKey) {
        // Ignore clicks that are the tail end of a wheel/pinch zoom gesture.
        if (window.__wheelZoomActive) return;

        const SM   = window.SelectionMode;
        const tool = sketchState.activeTool;

        if (sketchState.grab.active) { __confirmGrab(); return; }
        if (sketchState.copy.active) { __confirmCopyConnect(); return; }

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
          const hit = window.__raycastSketchPlane(ndcX, ndcY);
          if (!hit) return null;
          if (!window.__resolveSnapTarget) {
            // Fallback: behave as before — pure grid snap.
            return {
              kind: 'grid', pointId: null,
              gx: hit.gx, gy: hit.gy, gz: hit.gz,
              x: hit.x, y: hit.y, z: hit.z, valid: true,
            };
          }
          const canvas = document.getElementById('matterCanvas');
          const mpx = canvas
            ? { x: (ndcX + 1) * 0.5 * canvas.width, y: (1 - ndcY) * 0.5 * canvas.height }
            : { x: 0, y: 0 };
          return window.__resolveSnapTarget(
            { x: hit.freeX, y: hit.freeY, z: hit.freeZ },
            mpx,
            { force: true },
          );
        }

        if (tool === SM.POINT) {
          const snap = __resolveClickSnap();
          if (!snap || !snap.valid) return;
          // Snapped to an existing point — nothing to create.
          if (snap.kind === 'point') return;
          const mode = sketchState.engineMode || 'backend';
          // ── WASM / Hybrid path ──
          if (mode === 'wasm' || mode === 'hybrid') {
            window.__pushHistory();
            window.__wasmAddPointAndApply(snap.gx, snap.gy, snap.gz).then(() => {
              if (window.__updateSketchInspector) window.__updateSketchInspector();
            });
            return;
          }
          // ── Backend precision path ──
          if (mode === 'backend') {
            window.__pushHistory();
            window.__backendAddPoint(snap.gx, snap.gy, snap.gz).then(() => {
              if (window.__updateSketchInspector) window.__updateSketchInspector();
            });
            return;
          }
          // ── Legacy local fallback ──
          window.__pushHistory();
          window.__addPoint(snap.gx, snap.gy, snap.gz);
          if (window.__updateSketchInspector) window.__updateSketchInspector();
          return;
        }

        if (tool === SM.LINE) {
          const snap = __resolveClickSnap();
          if (!snap || !snap.valid) return;
          const hoveredId = snap.kind === 'point' ? snap.pointId : null;
          const mode = sketchState.engineMode || 'backend';
          // ── WASM / Hybrid path ──
          if (mode === 'wasm' || mode === 'hybrid') {
            (async () => {
              window.__pushHistory();
              let targetId = hoveredId;
              if (!targetId) {
                const r = await window.__wasmAddPointAndApply(snap.gx, snap.gy, snap.gz);
                if (!r.ok) return;
                targetId = r.pointId;
              }
              const startId = sketchState.line.startPointId;
              if (startId && startId !== targetId) {
                await window.__wasmAddEdgeAndApply(
                  { pointId: startId },
                  { pointId: targetId },
                );
              }
              sketchState.line.startPointId = targetId;
              sketchState.phase = "line_pending";
              if (window.__updateSketchInspector) window.__updateSketchInspector();
            })();
            return;
          }
          // ── Backend precision path ──
          if (mode === 'backend') {
            (async () => {
              window.__pushHistory();
              let targetId = hoveredId;
              if (!targetId) {
                const r = await window.__backendAddPoint(snap.gx, snap.gy, snap.gz);
                if (!r.ok) return;
                targetId = r.pointId;
              }
              const startId = sketchState.line.startPointId;
              if (startId && startId !== targetId) {
                await window.__backendAddEdge(
                  { pointId: startId },
                  { pointId: targetId },
                );
              }
              sketchState.line.startPointId = targetId;
              sketchState.phase = "line_pending";
              if (window.__updateSketchInspector) window.__updateSketchInspector();
            })();
            return;
          }
          // ── Legacy local path ──
          let targetId = hoveredId;
          let createdPoint = false;
          if (!targetId) {
            const existing = window.__findPointAtGrid(snap.gx, snap.gy, snap.gz);
            if (existing) targetId = existing.id;
            else {
              window.__pushHistory();
              targetId = window.__addPoint(snap.gx, snap.gy, snap.gz).id;
              createdPoint = true;
            }
          }
          const startId = sketchState.line.startPointId;
          if (startId && startId !== targetId) {
            if (!window.__findEdgeBetween(startId, targetId)) {
              if (!createdPoint) window.__pushHistory();
              window.__addEdge(startId, targetId);
            }
          }
          sketchState.line.startPointId = targetId;
          sketchState.phase = "line_pending";
          if (window.__updateSketchInspector) window.__updateSketchInspector();
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
      // __handleSketchDoubleClick(ndcX, ndcY)
      // Select tool: double-click edge.
      //  - if edge belongs to a profile → select entire profile.
      //  - else → select both endpoints of edge.
      // ─────────────────────────────────────────────────────────
      window.__handleSketchDoubleClick = function(ndcX, ndcY) {
        if (sketchState.activeTool !== "select") return;
        const eId = window.__pickEdgeAt(ndcX, ndcY);
        if (!eId) {
          // No edge under cursor — try profile picking inside an area.
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
        // Edge hit — find all profiles containing this edge, pick smallest by area.
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
          // If profile is already selected, expand to its points/edges.
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

      // ─────────────────────────────────────────────────────────
      // Constraint key handlers (require exactly one edge selected)
      // ─────────────────────────────────────────────────────────
      function __requireSingleEdge() {
        if (sketchState.selectedEdgeIds.size !== 1 || sketchState.selectedPointIds.size > 0) {
          window.__setStatusMessage('Select exactly one edge');
          return null;
        }
        const id = [...sketchState.selectedEdgeIds][0];
        return sketchState.edges.find(e => e.id === id) || null;
      }

      function __keyDimension() {
        const edge = __requireSingleEdge();
        if (!edge) return;
        const cur = window.__edgeLength(edge);
        const input = window.prompt('Edge length:', cur.toFixed(3));
        if (input === null) return;
        const v = parseFloat(input);
        if (!isFinite(v) || v <= 0) { window.__setStatusMessage('Invalid length'); return; }
        window.__pushHistory();
        const ok = window.__applyEdgeLength(edge, v);
        if (ok) {
          window.__addConstraint('edge_length', 'edge', edge.id, v);
          window.__notifySketchChanged();
          window.__setStatusMessage('Length = ' + v.toFixed(3));
        }
      }

      function __keyHorizontal() {
        const edge = __requireSingleEdge();
        if (!edge) return;
        window.__pushHistory();
        const ok = window.__applyHorizontal(edge);
        if (ok) {
          const v = window.__getConstraintForTarget('vertical', edge.id);
          if (v) window.__removeConstraint(v.id);
          window.__addConstraint('horizontal', 'edge', edge.id, null);
          window.__notifySketchChanged();
          window.__setStatusMessage('Edge horizontal');
        }
      }

      function __keyVertical() {
        const edge = __requireSingleEdge();
        if (!edge) return;
        window.__pushHistory();
        const ok = window.__applyVertical(edge);
        if (ok) {
          const h = window.__getConstraintForTarget('horizontal', edge.id);
          if (h) window.__removeConstraint(h.id);
          window.__addConstraint('vertical', 'edge', edge.id, null);
          window.__notifySketchChanged();
          window.__setStatusMessage('Edge vertical');
        }
      }

      function __keyFixToggle() {
        if (sketchState.selectedPointIds.size === 0) {
          window.__setStatusMessage('F: select points first');
          return;
        }
        window.__pushHistory();
        let nFixed = 0, nUnfixed = 0;
        for (const pid of sketchState.selectedPointIds) {
          if (window.__isPointFixed(pid)) {
            const c = window.__getConstraintForTarget('fixed_point', pid);
            if (c) { window.__removeConstraint(c.id); nUnfixed++; }
          } else {
            window.__addConstraint('fixed_point', 'point', pid, null);
            nFixed++;
          }
        }
        window.__setStatusMessage('Fixed ' + nFixed + ' · Unfixed ' + nUnfixed);
      }

      // ─────────────────────────────────────────────────────────
      // __handleSketchKey(e) → bool consumed
      // ─────────────────────────────────────────────────────────
      window.__handleSketchKey = function(e) {
        const k    = e.key.toLowerCase();
        const meta = e.metaKey || e.ctrlKey;

        if (e.target && (e.target.tagName === 'INPUT' || e.target.tagName === 'SELECT' || e.target.tagName === 'TEXTAREA')) return false;

        // Undo / Redo.
        if (meta && k === 'z') {
          if (e.shiftKey) window.__redo();
          else            window.__undo();
          if (window.__updateSketchInspector) window.__updateSketchInspector();
          e.preventDefault();
          return true;
        }
        if (meta && k === 'y') {
          window.__redo();
          if (window.__updateSketchInspector) window.__updateSketchInspector();
          e.preventDefault();
          return true;
        }

        // Grab mode hotkeys (axis lock).
        if (sketchState.grab.active) {
          if (k === 'escape') { __cancelGrab(); return true; }
          if (k === 'enter')  { __confirmGrab(); return true; }
          if (k === 'x') { sketchState.grab.axisLock = (sketchState.grab.axisLock === 'X' ? null : 'X'); return true; }
          if (k === 'y') { sketchState.grab.axisLock = (sketchState.grab.axisLock === 'Y' ? null : 'Y'); return true; }
          if (k === 'z') { sketchState.grab.axisLock = (sketchState.grab.axisLock === 'Z' ? null : 'Z'); return true; }
          return true;
        }

        // Copy Connect hotkeys (Phase 14).
        if (sketchState.copy.active) {
          if (k === 'escape') { __cancelCopyConnect(); return true; }
          if (k === 'enter')  { __confirmCopyConnect(); return true; }
          if (k === 'x') { __copyAxisToggle('X'); return true; }
          if (k === 'y') { __copyAxisToggle('Y'); return true; }
          if (k === 'z') { __copyAxisToggle('Z'); return true; }
          return true;
        }

        // Working plane.
        if (k === '1') { window.__setWorkingPlane('XZ'); return true; }
        if (k === '2') { window.__setWorkingPlane('XY'); return true; }
        if (k === '3') { window.__setWorkingPlane('YZ'); return true; }

        // Projection draft mode toggle (Phase 13).
        if (k === 'j') {
          if (window.__toggleDraftMode) window.__toggleDraftMode();
          return true;
        }

        // N — toggle inspector panel (Blender style).
        if (k === 'n') {
          const tab   = document.getElementById('si-tab');
          const panel = document.getElementById('sketch-inspector');
          const stage = document.querySelector('.matter-stage');
          if (tab && panel) {
            const open = panel.classList.toggle('open');
            tab.classList.toggle('open', open);
            if (stage) stage.classList.toggle('inspector-open', open);
          }
          return true;
        }

        if (k === 'escape') {
          if (sketchState.line.startPointId) {
            sketchState.line.startPointId = null;
            sketchState.phase = "idle";
            window.__setStatusMessage('Line chain cancelled');
          } else {
            sketchState.selectedPointIds.clear();
            sketchState.selectedEdgeIds.clear();
          }
          if (window.__updateSketchInspector) window.__updateSketchInspector();
          return true;
        }

        if (k === 'enter') {
          if (sketchState.line.startPointId) {
            sketchState.line.startPointId = null;
            sketchState.phase = "idle";
            window.__setStatusMessage('Line chain finished');
            if (window.__updateSketchInspector) window.__updateSketchInspector();
          }
          return true;
        }

        if (k === 'backspace' || k === 'delete') {
          if (sketchState.selectedPointIds.size + sketchState.selectedEdgeIds.size > 0) {
            window.__pushHistory();
            window.__deleteSelected();
            if (window.__updateSketchInspector) window.__updateSketchInspector();
          }
          return true;
        }

        // Tools.
        if (k === 's') { window.__setSketchTool && window.__setSketchTool('select'); return true; }
        if (k === 'p' && e.shiftKey) {
          if (window.__togglePerfHud) window.__togglePerfHud();
          return true;
        }
        if (k === 'p') { window.__setSketchTool && window.__setSketchTool('point');  return true; }
        if (k === 'l') { window.__setSketchTool && window.__setSketchTool('line');   return true; }
        if (k === 'g' && e.shiftKey) {
          __startCopyConnect();
          return true;
        }
        if (k === 'g') {
          // If edges are selected but no points — auto-derive their endpoints.
          if (!sketchState.selectedPointIds.size && sketchState.selectedEdgeIds.size > 0) {
            const eById = new Map(sketchState.edges.map(e => [e.id, e]));
            for (const eid of sketchState.selectedEdgeIds) {
              const edge = eById.get(eid);
              if (edge) {
                sketchState.selectedPointIds.add(edge.a);
                sketchState.selectedPointIds.add(edge.b);
              }
            }
          }
          // If a profile is selected — take all its points.
          if (!sketchState.selectedPointIds.size && sketchState.selectedProfileId) {
            const prof = window.__getProfileById
              ? window.__getProfileById(sketchState.selectedProfileId)
              : null;
            if (prof && prof.pointIds) {
              for (const id of prof.pointIds) sketchState.selectedPointIds.add(id);
            }
          }
          if (!sketchState.selectedPointIds.size) {
            window.__setStatusMessage('G: select points, edges, or a profile first');
            return true;
          }
          __startGrab();
          return true;
        }

        // Mirror — reserved for a future tool (currently a no-op stub).
        if (k === 'm' && !e.shiftKey && !meta) {
          window.__setStatusMessage('M: Mirror — coming soon');
          return true;
        }

        // Constraints.
        if (k === 'd') { __keyDimension();  if (window.__updateSketchInspector) window.__updateSketchInspector(); return true; }
        if (k === 'f') { __keyFixToggle();  if (window.__updateSketchInspector) window.__updateSketchInspector(); return true; }
        if (k === 'h') { __keyHorizontal(); if (window.__updateSketchInspector) window.__updateSketchInspector(); return true; }
        if (k === 'v') {
          if (e.shiftKey) {
            sketchState.showValidation = !sketchState.showValidation;
            window.__setStatusMessage('Validation: ' + (sketchState.showValidation ? 'on' : 'off'));
          } else {
            __keyVertical();
          }
          if (window.__updateSketchInspector) window.__updateSketchInspector();
          return true;
        }

        return false;
      };

      // ─────────────────────────────────────────────────────────
      // Grab — skips fixed points.
      // ─────────────────────────────────────────────────────────
      function __startGrab() {
        const allIds   = [...sketchState.selectedPointIds];
        const moveIds  = allIds.filter(id => !window.__isPointFixed(id));
        if (!moveIds.length) {
          window.__setStatusMessage('Cannot move fixed point');
          return;
        }
        window.__pushHistory();
        const byId = new Map(sketchState.points.map(p => [p.id, p]));
        const snapshot = new Map();
        for (const id of moveIds) {
          const p = byId.get(id);
          if (p) snapshot.set(id, { x: p.x, y: p.y, z: p.z });
        }
        const startWorld = sketchState.hoverWorld
          ? { x: sketchState.hoverWorld.x, y: sketchState.hoverWorld.y, z: sketchState.hoverWorld.z }
          : { x: 0, y: 0, z: 0 };
        // Store start screen Y so we can drive the off-plane axis via mouse vertical delta.
        const startScreen = (sketchState.precision && sketchState.precision.lastMouseScreen)
          ? { x: sketchState.precision.lastMouseScreen.x, y: sketchState.precision.lastMouseScreen.y }
          : { x: 0, y: 0 };
        sketchState.grab = {
          active: true, pointIds: moveIds,
          startMouseWorld: startWorld,
          startScreen,
          originalPoints: snapshot,
          axisLock: null,
        };
        const skipped = allIds.length - moveIds.length;
        const msg = '⤢ Grab ' + moveIds.length + ' pt' + (skipped ? ' (' + skipped + ' fixed)' : '') + ' — X/Y/Z lock';
        window.__setStatusMessage(msg);
        if (window.__updateSketchInspector) window.__updateSketchInspector();
      }

      window.__updateGrab = function(hoverWorld) {
        const grab = sketchState.grab;
        if (!grab.active || !hoverWorld || !grab.startMouseWorld) return;
        if (!window.__isPointerDragging || !window.__isPointerDragging()) return;
        if (!grab.dragBase) return;

        let dx = hoverWorld.x - grab.startMouseWorld.x;
        let dy = hoverWorld.y - grab.startMouseWorld.y;
        let dz = hoverWorld.z - grab.startMouseWorld.z;

        // The working plane constrains the raycast, so the axis perpendicular
        // to the plane never moves in hoverWorld.  Instead, drive it from the
        // vertical mouse-screen delta (upward = positive world units).
        const plane = sketchState.workingPlane || 'XZ';
        // Perpendicular axes per plane: XZ→Y,  XY→Z,  YZ→X
        const perpAxis = (plane === 'XZ') ? 'Y' : (plane === 'XY') ? 'Z' : 'X';

        if (grab.axisLock === perpAxis) {
          // Use screen-Y delta to drive the off-plane axis.
          const scr = sketchState.precision && sketchState.precision.lastMouseScreen;
          if (scr && grab.startScreen) {
            const canvas = document.getElementById('matterCanvas');
            const canvasH = canvas ? canvas.height : 600;
            const pixelDelta = grab.startScreen.y - scr.y;   // up = positive
            const scale = (cam.dist * 1.8) / canvasH;        // world-units per pixel
            const raw = pixelDelta * scale;
            if (perpAxis === 'Y') { dx = 0; dy = raw; dz = 0; }
            if (perpAxis === 'Z') { dx = 0; dy = 0;   dz = raw; }
            if (perpAxis === 'X') { dx = raw; dy = 0; dz = 0; }
          }
        } else if (grab.axisLock === 'X') { dy = 0; dz = 0; }
          else if (grab.axisLock === 'Y') { dx = 0; dz = 0; }
          else if (grab.axisLock === 'Z') { dx = 0; dy = 0; }
          else if (grab.axisLock === 'XY') { dz = 0; }
          else if (grab.axisLock === 'YZ') { dx = 0; }
          else if (grab.axisLock === 'XZ') { dy = 0; }
          else if (grab.axisLock === 'FREE') { /* free move on working plane, no axis zeroing */ }

        const g = sketchState.gridSize || 1.0;
        const sdx = Math.round(dx / g) * g;
        const sdy = Math.round(dy / g) * g;
        const sdz = Math.round(dz / g) * g;
        const byId = new Map(sketchState.points.map(p => [p.id, p]));
        for (const id of grab.pointIds) {
          const base = grab.dragBase.get(id);
          const p = byId.get(id);
          if (!base || !p) continue;
          p.x  = base.x + sdx; p.y  = base.y + sdy; p.z  = base.z + sdz;
          p.gx = Math.round(p.x / g);
          p.gy = Math.round(p.y / g);
          p.gz = Math.round(p.z / g);
        }
      };

      function __confirmGrab() {
        const n = sketchState.grab.pointIds.length;
        sketchState.grab = { active: false, pointIds: [], startMouseWorld: null, originalPoints: new Map(), axisLock: null };
        window.__notifySketchChanged();
        window.__setStatusMessage('Grab confirmed (' + n + ' pt)');
        if (window.__updateSketchInspector) window.__updateSketchInspector();
      }

      function __cancelGrab() {
        const grab = sketchState.grab;
        const byId = new Map(sketchState.points.map(p => [p.id, p]));
        const g = sketchState.gridSize || 1.0;
        for (const id of grab.pointIds) {
          const orig = grab.originalPoints.get(id);
          const p = byId.get(id);
          if (!orig || !p) continue;
          p.x = orig.x; p.y = orig.y; p.z = orig.z;
          p.gx = Math.round(p.x / g);
          p.gy = Math.round(p.y / g);
          p.gz = Math.round(p.z / g);
        }
        sketchState.grab = { active: false, pointIds: [], startMouseWorld: null, originalPoints: new Map(), axisLock: null };
        if (sketchState._history.undo.length) sketchState._history.undo.pop();
        window.__setStatusMessage('Grab cancelled');
        if (window.__updateSketchInspector) window.__updateSketchInspector();
      }

      // ─────────────────────────────────────────────────────────
      // Copy Connect (Phase 14) — Plasticity-style pull-copy.
      //   Shift+G   → start
      //   X / Y / Z → toggle axis lock
      //   Enter / click → confirm
      //   Esc       → cancel
      // The copy lives in a preview-only state; nothing is mutated on the
      // sketch model until __confirmCopyConnect() runs.
      // ─────────────────────────────────────────────────────────
      function __collectCopySource() {
        // Returns { source, pointIds:[…], edges:[[a,b,kind], …] } or null.
        const eById = new Map(sketchState.edges.map(e => [e.id, e]));

        if (sketchState.selectedProfileId) {
          const prof = window.__getProfileById
            ? window.__getProfileById(sketchState.selectedProfileId)
            : null;
          if (prof && prof.pointIds && prof.pointIds.length) {
            const edges = (prof.edgeIds || [])
              .map(id => eById.get(id))
              .filter(Boolean)
              .map(e => [e.a, e.b, e.kind || 'normal']);
            return { source: 'profile', pointIds: [...prof.pointIds], edges };
          }
        }
        if (sketchState.selectedEdgeIds && sketchState.selectedEdgeIds.size > 0) {
          const ptSet = new Set();
          const edges = [];
          for (const eid of sketchState.selectedEdgeIds) {
            const e = eById.get(eid);
            if (!e) continue;
            ptSet.add(e.a); ptSet.add(e.b);
            edges.push([e.a, e.b, e.kind || 'normal']);
          }
          if (ptSet.size) return { source: 'edges', pointIds: [...ptSet], edges };
        }
        if (sketchState.selectedPointIds && sketchState.selectedPointIds.size > 0) {
          return { source: 'points', pointIds: [...sketchState.selectedPointIds], edges: [] };
        }
        return null;
      }

      function __startCopyConnect() {
        if (sketchState.grab.active) {
          window.__setStatusMessage('Finish grab first');
          return;
        }
        const src = __collectCopySource();
        if (!src) {
          window.__setStatusMessage('Select profile, edges, or points to copy');
          return;
        }
        const byId = new Map(sketchState.points.map(p => [p.id, p]));
        const originals = new Map();
        const validIds = [];
        for (const id of src.pointIds) {
          const p = byId.get(id);
          if (!p) continue;
          originals.set(id, { x: p.x, y: p.y, z: p.z });
          validIds.push(id);
        }
        if (!validIds.length) {
          window.__setStatusMessage('Copy: nothing valid to copy');
          return;
        }
        const startWorld = sketchState.hoverWorld
          ? { x: sketchState.hoverWorld.x, y: sketchState.hoverWorld.y, z: sketchState.hoverWorld.z }
          : { x: 0, y: 0, z: 0 };
        const cpStartScreen = (sketchState.precision && sketchState.precision.lastMouseScreen)
          ? { x: sketchState.precision.lastMouseScreen.x, y: sketchState.precision.lastMouseScreen.y }
          : { x: 0, y: 0 };
        sketchState.copy = {
          active: true,
          source: src.source,
          pointIds: validIds,
          edges: src.edges,
          originals,
          startMouseWorld: startWorld,
          startScreen: cpStartScreen,
          delta: { dx: 0, dy: 0, dz: 0 },
          axisLock: null,
        };
        const label = src.source === 'profile' ? 'profile'
                    : src.source === 'edges'   ? (src.edges.length + ' edge' + (src.edges.length === 1 ? '' : 's'))
                    : (validIds.length + ' pt');
        window.__setStatusMessage('⎘ Copy Connect ' + label + ' — X/Y/Z lock · Enter confirm · Esc cancel');
        if (window.__updateSketchInspector) window.__updateSketchInspector();
      }

      function __copyAxisToggle(axis) {
        sketchState.copy.axisLock = (sketchState.copy.axisLock === axis) ? null : axis;
      }

      window.__updateCopyConnect = function(hoverWorld) {
        const cp = sketchState.copy;
        if (!cp.active || !hoverWorld || !cp.startMouseWorld) return;
        if (!window.__isPointerDragging || !window.__isPointerDragging()) return;
        if (!cp.baseDelta) return;

        let dx = hoverWorld.x - cp.startMouseWorld.x;
        let dy = hoverWorld.y - cp.startMouseWorld.y;
        let dz = hoverWorld.z - cp.startMouseWorld.z;

        // Same off-plane axis fix as grab.
        const plane = sketchState.workingPlane || 'XZ';
        const perpAxis = (plane === 'XZ') ? 'Y' : (plane === 'XY') ? 'Z' : 'X';

        if (cp.axisLock === perpAxis) {
          const scr = sketchState.precision && sketchState.precision.lastMouseScreen;
          if (scr && cp.startScreen) {
            const canvas = document.getElementById('matterCanvas');
            const canvasH = canvas ? canvas.height : 600;
            const pixelDelta = cp.startScreen.y - scr.y;
            const scale = (cam.dist * 1.8) / canvasH;
            const raw = pixelDelta * scale;
            if (perpAxis === 'Y') { dx = 0; dy = raw; dz = 0; }
            if (perpAxis === 'Z') { dx = 0; dy = 0;   dz = raw; }
            if (perpAxis === 'X') { dx = raw; dy = 0; dz = 0; }
          }
        } else if (cp.axisLock === 'X') { dy = 0; dz = 0; }
          else if (cp.axisLock === 'Y') { dx = 0; dz = 0; }
          else if (cp.axisLock === 'Z') { dx = 0; dy = 0; }
          else if (cp.axisLock === 'XY') { dz = 0; }
          else if (cp.axisLock === 'YZ') { dx = 0; }
          else if (cp.axisLock === 'XZ') { dy = 0; }
          else if (cp.axisLock === 'FREE') { /* no extra zeros */ }

        const g = sketchState.gridSize || 1.0;
        cp.delta.dx = cp.baseDelta.dx + Math.round(dx / g) * g;
        cp.delta.dy = cp.baseDelta.dy + Math.round(dy / g) * g;
        cp.delta.dz = cp.baseDelta.dz + Math.round(dz / g) * g;
      };

      async function __confirmCopyConnect() {
        const cp = sketchState.copy;
        if (!cp.active) return;
        const { dx, dy, dz } = cp.delta;
        if (dx === 0 && dy === 0 && dz === 0) {
          window.__setStatusMessage('Copy: zero offset — move cursor first');
          return;
        }
        // Snapshot for undo; locked while we await engine commits.
        if (window.__pushHistory) window.__pushHistory();
        const g = sketchState.gridSize || 1.0;
        const origToCopy = new Map();   // originalId → copiedId
        const originals = cp.originals;

        // 1) Create copied points.
        for (const id of cp.pointIds) {
          const orig = originals.get(id);
          if (!orig) continue;
          const gx = Math.round((orig.x + dx) / g);
          const gy = Math.round((orig.y + dy) / g);
          const gz = Math.round((orig.z + dz) / g);
          const newId = await window.__createPointViaEngine(gx, gy, gz);
          if (newId) origToCopy.set(id, newId);
        }
        // 2) Mirror the inner edges between copied points.
        for (const [a, b, kind] of cp.edges) {
          const a2 = origToCopy.get(a);
          const b2 = origToCopy.get(b);
          if (a2 && b2 && a2 !== b2) {
            await window.__createEdgeViaEngine(a2, b2, kind || 'normal');
          }
        }
        // 3) Connector edges: original → its copy (skip if collapsed).
        let connectorCount = 0;
        for (const [origId, newId] of origToCopy.entries()) {
          if (!origId || !newId || origId === newId) continue;
          await window.__createEdgeViaEngine(origId, newId, 'normal');
          connectorCount += 1;
        }

        const total = origToCopy.size;
        sketchState.copy = {
          active: false, source: null, pointIds: [], edges: [],
          originals: new Map(), startMouseWorld: null,
          delta: { dx: 0, dy: 0, dz: 0 }, axisLock: null,
        };
        if (window.__notifySketchChanged) window.__notifySketchChanged();
        window.__setStatusMessage(
          '⎘ Copy Connect ✓ ' + total + ' pt · ' + cp.edges.length + ' edge · ' + connectorCount + ' connector'
        );
        if (window.__updateSketchInspector) window.__updateSketchInspector();
      }

      function __cancelCopyConnect() {
        sketchState.copy = {
          active: false, source: null, pointIds: [], edges: [],
          originals: new Map(), startMouseWorld: null,
          delta: { dx: 0, dy: 0, dz: 0 }, axisLock: null,
        };
        window.__setStatusMessage('Copy Connect cancelled');
        if (window.__updateSketchInspector) window.__updateSketchInspector();
      }
      // Expose for UI buttons if needed.
      window.__startCopyConnect    = __startCopyConnect;
      window.__confirmCopyConnect  = __confirmCopyConnect;
      window.__cancelCopyConnect   = __cancelCopyConnect;

      // ─────────────────────────────────────────────────────────
      // __updateLinePreview()
      // ─────────────────────────────────────────────────────────
      window.__updateLinePreview = function() {
        const line = sketchState.line;
        if (sketchState.activeTool !== 'line' || !line.startPointId || !sketchState.hoverWorld) {
          line.previewPoint = null;
          line.previewLength = 0;
          line.previewValid = true;
          return;
        }
        const byId = new Map(sketchState.points.map(p => [p.id, p]));
        const a = byId.get(line.startPointId);
        if (!a) { line.previewPoint = null; return; }
        const h = sketchState.hoverWorld;
        line.previewPoint = { x: h.x, y: h.y, z: h.z, gx: h.gx, gy: h.gy, gz: h.gz };
        line.previewLength = Math.hypot(h.x - a.x, h.y - a.y, h.z - a.z);
        const samePos   = (a.gx === h.gx && a.gy === h.gy && a.gz === h.gz);
        const targetExisting = window.__findPointAtGrid(h.gx, h.gy, h.gz);
        const dupEdge   = targetExisting && window.__findEdgeBetween(a.id, targetExisting.id);
        line.previewValid = !samePos && !dupEdge;
      };
"##;
