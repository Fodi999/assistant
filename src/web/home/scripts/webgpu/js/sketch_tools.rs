// ── JS: Sketch tools — select / point / line / grab / delete + constraints ──
// Domain: Sketch — click dispatch, hotkeys, line chain, grab, history.

pub const JS: &str = r##"
      // ─────────────────────────────────────────────────────────
      // __handleSketchClick(ndcX, ndcY, shiftKey)
      // ─────────────────────────────────────────────────────────
      window.__handleSketchClick = function(ndcX, ndcY, shiftKey) {
        const SM   = window.SelectionMode;
        const tool = sketchState.activeTool;

        if (sketchState.grab.active) { __confirmGrab(); return; }

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

        if (tool === SM.POINT) {
          const hit = window.__raycastSketchPlane(ndcX, ndcY);
          if (!hit) return;
          const existing = window.__findPointAtGrid(hit.gx, hit.gy, hit.gz);
          if (existing) return;
          // ── Backend precision path (Phase 7) ──
          if (sketchState.useBackendCommands) {
            window.__pushHistory();
            window.__backendAddPoint(hit.gx, hit.gy, hit.gz).then(() => {
              if (window.__updateSketchInspector) window.__updateSketchInspector();
            });
            return;
          }
          window.__pushHistory();
          window.__addPoint(hit.gx, hit.gy, hit.gz);
          if (window.__updateSketchInspector) window.__updateSketchInspector();
          return;
        }

        if (tool === SM.LINE) {
          const hoveredId = window.__pickPointAt(ndcX, ndcY);
          // ── Backend precision path ──
          if (sketchState.useBackendCommands) {
            (async () => {
              window.__pushHistory();
              let targetId = hoveredId;
              if (!targetId) {
                const hit = window.__raycastSketchPlane(ndcX, ndcY);
                if (!hit) return;
                const r = await window.__backendAddPoint(hit.gx, hit.gy, hit.gz);
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
            const hit = window.__raycastSketchPlane(ndcX, ndcY);
            if (!hit) return;
            const existing = window.__findPointAtGrid(hit.gx, hit.gy, hit.gz);
            if (existing) targetId = existing.id;
            else { window.__pushHistory(); targetId = window.__addPoint(hit.gx, hit.gy, hit.gz).id; createdPoint = true; }
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
          if (k === 'x') { sketchState.grab.axisLock = 'X'; return true; }
          if (k === 'y') { sketchState.grab.axisLock = 'Y'; return true; }
          if (k === 'z') { sketchState.grab.axisLock = 'Z'; return true; }
          return true;
        }

        // Working plane.
        if (k === '1') { window.__setWorkingPlane('XZ'); return true; }
        if (k === '2') { window.__setWorkingPlane('XY'); return true; }
        if (k === '3') { window.__setWorkingPlane('YZ'); return true; }

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
        if (k === 'p') { window.__setSketchTool && window.__setSketchTool('point');  return true; }
        if (k === 'l') { window.__setSketchTool && window.__setSketchTool('line');   return true; }
        if (k === 'g') {
          if (!sketchState.selectedPointIds.size) { window.__setStatusMessage('G: select points first'); return true; }
          __startGrab();
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
        sketchState.grab = {
          active: true, pointIds: moveIds,
          startMouseWorld: startWorld,
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
        let dx = hoverWorld.x - grab.startMouseWorld.x;
        let dy = hoverWorld.y - grab.startMouseWorld.y;
        let dz = hoverWorld.z - grab.startMouseWorld.z;
        if (grab.axisLock === 'X') { dy = 0; dz = 0; }
        if (grab.axisLock === 'Y') { dx = 0; dz = 0; }
        if (grab.axisLock === 'Z') { dx = 0; dy = 0; }
        const g = sketchState.gridSize || 1.0;
        const sdx = Math.round(dx / g) * g;
        const sdy = Math.round(dy / g) * g;
        const sdz = Math.round(dz / g) * g;
        const byId = new Map(sketchState.points.map(p => [p.id, p]));
        for (const id of grab.pointIds) {
          const orig = grab.originalPoints.get(id);
          const p = byId.get(id);
          if (!orig || !p) continue;
          p.x  = orig.x + sdx; p.y  = orig.y + sdy; p.z  = orig.z + sdz;
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
