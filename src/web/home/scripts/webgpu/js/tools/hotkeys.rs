// ── Hotkeys + Constraints ─────────────────────────────────────────────────────
// Handles:
//   __handleSketchKey  — all keyboard shortcuts
//   __keyDimension     — D: edge length constraint
//   __keyHorizontal    — H: horizontal constraint
//   __keyVertical      — V: vertical constraint
//   __keyFixToggle     — F: fix/unfix points
//   __updateLinePreview — line tool preview

pub const JS: &str = r##"
      // ─────────────────────────────────────────────────────────
      // Constraint helpers
      // ─────────────────────────────────────────────────────────
      function __requireSingleEdge() {
        if (sketchState.selectedEdgeIds.size !== 1 || sketchState.selectedPointIds.size > 0) {
          window.__setStatusMessage('Выберите ровно одно ребро');
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
        if (!isFinite(v) || v <= 0) { window.__setStatusMessage('Некорректная длина'); return; }
        window.__pushHistory();
        const ok = window.__applyEdgeLength(edge, v);
        if (ok) {
          window.__addConstraint('edge_length', 'edge', edge.id, v);
          window.__notifySketchChanged();
          window.__setStatusMessage('Длина = ' + v.toFixed(3));
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
          window.__setStatusMessage('Ребро горизонтально');
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
          window.__setStatusMessage('Ребро вертикально');
        }
      }

      function __keyFixToggle() {
        if (sketchState.selectedPointIds.size === 0) {
          window.__setStatusMessage('F: сначала выберите точки');
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
        window.__setStatusMessage('Зафиксировано ' + nFixed + ' · Снято ' + nUnfixed);
      }

      // ─────────────────────────────────────────────────────────
      // __applyOrthoSnap(a, h, plane) → snapped hover point
      // TRUE axis-lock: dominant axis wins → line becomes exactly
      // 0° (horizontal) or 90° (vertical). No 45° — pure CAD ortho.
      //   |dx| > |dz|  →  lock Z  (horizontal line)
      //   |dz| >= |dx| →  lock X  (vertical line)
      // After axis lock the endpoint is also re-snapped to grid.
      // ─────────────────────────────────────────────────────────
      window.__applyOrthoSnap = function(a, h, plane) {
        const gs = sketchState.gridSize || 0.00001;
        const r  = Object.assign({}, h);

        let dx, dy;
        if      (plane === 'XY') { dx = h.x - a.x; dy = h.y - a.y; }
        else if (plane === 'YZ') { dx = h.y - a.y; dy = h.z - a.z; }
        else /* XZ default */    { dx = h.x - a.x; dy = h.z - a.z; }

        if (Math.abs(dx) >= Math.abs(dy)) {
          // Horizontal: lock the secondary axis to start point
          if      (plane === 'XY') { r.y = a.y; }
          else if (plane === 'YZ') { r.z = a.z; }
          else /* XZ */            { r.z = a.z; }
          // store which axis was locked for badge display
          r._orthoAxis = plane === 'YZ' ? 'ORTHO Y' : 'ORTHO X';
        } else {
          // Vertical: lock the primary axis to start point
          if      (plane === 'XY') { r.x = a.x; }
          else if (plane === 'YZ') { r.y = a.y; }
          else /* XZ */            { r.x = a.x; }
          r._orthoAxis = plane === 'YZ' ? 'ORTHO Z' : 'ORTHO Z';
        }

        // Snap the locked point back to grid
        r.gx = Math.round(r.x / gs);
        r.gy = Math.round(r.y / gs);
        r.gz = Math.round(r.z / gs);
        // Re-align world coords to grid (removes sub-grid drift)
        r.x  = r.gx * gs;
        r.y  = r.gy * gs;
        r.z  = r.gz * gs;
        return r;
      };

      // ─────────────────────────────────────────────────────────
      // __toggleOrthoLock() — O key or ORTHO button
      // ─────────────────────────────────────────────────────────
      window.__toggleOrthoLock = function() {
        sketchState.orthoLock = !sketchState.orthoLock;
        // Update toolbar button visual state
        const btn = document.getElementById('btn-ortho');
        if (btn) btn.classList.toggle('active', sketchState.orthoLock);
        // Show status
        if (window.__setStatusMessage)
          window.__setStatusMessage(sketchState.orthoLock ? 'Ортогональность ВКЛ  (0° 45° 90°)' : 'Ортогональность ВЫКЛ');
        if (window.__updateLinePreview) window.__updateLinePreview();
      };


      window.__handleSketchKey = function(e) {
        const k    = e.key.toLowerCase();
        const meta = e.metaKey || e.ctrlKey;

        if (e.target && (e.target.tagName === 'INPUT' || e.target.tagName === 'SELECT' || e.target.tagName === 'TEXTAREA')) return false;

        // Undo / Redo
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

        // Grab mode hotkeys
        if (sketchState.grab.active) {
          if (k === 'escape') {
            if (window.__gizmoCancel) window.__gizmoCancel();
            else window.__cancelGrab();
            return true;
          }
          if (k === 'enter')  { window.__confirmGrab().catch(e => console.warn('[hotkeys] confirmGrab err', e)); return true; }
          // Numeric input: digits, sign, decimal point
          if (/^[0-9]$/.test(k) || (k === '-' && !(sketchState.grab.numericInput||'').includes('-')) || k === '.') {
            sketchState.grab.numericInput = (sketchState.grab.numericInput || '') + e.key;
            if (window.__updateSketchInspector) window.__updateSketchInspector();
            e.preventDefault();
            return true;
          }
          if (k === 'backspace') {
            const ni = sketchState.grab.numericInput || '';
            sketchState.grab.numericInput = ni.slice(0, -1);
            if (window.__updateSketchInspector) window.__updateSketchInspector();
            e.preventDefault();
            return true;
          }
          function __setGrabAxisLock(axis) {
            const grab = sketchState.grab;
            grab.axisLock = (grab.axisLock === axis) ? null : axis;
            grab.numericInput = '';   // reset numeric on axis change
            grab.screenAcc = { x: 0, y: 0, z: 0 };
            const byId = new Map(sketchState.points.map(p => [p.id, p]));
            grab.dragBase = new Map();
            let _kcx=0, _kcy=0, _kcz=0, _kcn=0;
            for (const id of grab.pointIds) {
              const p = byId.get(id);
              if (p) {
                grab.dragBase.set(id, { x: p.x, y: p.y, z: p.z });
                _kcx+=p.x; _kcy+=p.y; _kcz+=p.z; _kcn++;
              }
            }
            // Re-anchor drag plane center + reset startDragPoint for new axis
            grab.startCenter = _kcn ? { x:_kcx/_kcn, y:_kcy/_kcn, z:_kcz/_kcn } : grab.startCenter;
            grab.startDragPoint = null; // will be re-set on next pointermove
            const lockName = grab.axisLock || 'free';
            window.__setStatusMessage('⤢ Захват · ось ' + lockName.toUpperCase() + ' — тяни · Enter ✓ · Esc ✗');
            console.log('[Grab] axis lock: ' + grab.axisLock);
          }
          if (k === 'x') { __setGrabAxisLock('X'); return true; }
          if (k === 'y') { __setGrabAxisLock('Y'); return true; }
          if (k === 'z') { __setGrabAxisLock('Z'); return true; }
          return true;
        }

        // Copy Connect hotkeys
        if (sketchState.copy.active) {
          if (k === 'escape') { window.__cancelCopyConnect(); return true; }
          if (k === 'enter')  { window.__confirmCopyConnect(); return true; }
          if (k === 'x') { window.__copyAxisToggle('X'); return true; }
          if (k === 'y') { window.__copyAxisToggle('Y'); return true; }
          if (k === 'z') { window.__copyAxisToggle('Z'); return true; }
          return true;
        }

        // Working plane
        if (k === '1') { window.__setWorkingPlane('XZ'); return true; }
        if (k === '2') { window.__setWorkingPlane('XY'); return true; }
        if (k === '3') { window.__setWorkingPlane('YZ'); return true; }

        // J — projection draft mode
        if (k === 'j') {
          if (window.__toggleDraftMode) window.__toggleDraftMode();
          return true;
        }

        // N — toggle inspector panel
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
          if (sketchState.line.active || sketchState.line.startPointId) {
            if (window.__finishLineChain) window.__finishLineChain('cancelled');
            else {
              sketchState.line = { active: false, startPointId: null, startWorld: null };
              sketchState.phase = 'idle';
              window.__setStatusMessage('Линия отменена');
            }
          } else {
            sketchState.selectedPointIds.clear();
            sketchState.selectedEdgeIds.clear();
          }
          if (window.__updateSketchInspector) window.__updateSketchInspector();
          return true;
        }

        if (k === 'enter') {
          if (sketchState.line.active || sketchState.line.startPointId) {
            if (window.__finishLineChain) window.__finishLineChain();
            else {
              sketchState.line = { active: false, startPointId: null, startWorld: null };
              sketchState.phase = 'idle';
              window.__setStatusMessage('Режим линии завершён');
              if (window.__updateSketchInspector) window.__updateSketchInspector();
            }
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

        // Tool switches
        if (k === 's') { window.__setSketchTool && window.__setSketchTool('select'); return true; }
        if (k === 'o') { window.__toggleOrthoLock && window.__toggleOrthoLock(); return true; }
        if (k === 'p' && e.shiftKey) { if (window.__togglePerfHud) window.__togglePerfHud(); return true; }
        if (k === 'p') { window.__setSketchTool && window.__setSketchTool('point'); return true; }
        if (k === 'l') { window.__setSketchTool && window.__setSketchTool('line');  return true; }

        // Shift+G → Copy Connect
        if (k === 'g' && e.shiftKey) {
          window.__startCopyConnect();
          return true;
        }

        // G → Grab
        if (k === 'g') {
          if (!sketchState.selectedPointIds.size &&
              !sketchState.selectedEdgeIds.size &&
              !sketchState.selectedProfileId) {
            window.__setStatusMessage('G: сначала выберите точки, рёбра или профиль');
            return true;
          }
          window.__startGrab();
          return true;
        }

        // M — Mirror stub
        if (k === 'm' && !e.shiftKey && !meta) {
          window.__setStatusMessage('M: Зеркало — скоро');
          return true;
        }

        // Constraints
        if (k === 'd') { __keyDimension();  if (window.__updateSketchInspector) window.__updateSketchInspector(); return true; }
        if (k === 'f') { __keyFixToggle();  if (window.__updateSketchInspector) window.__updateSketchInspector(); return true; }
        if (k === 'h') { __keyHorizontal(); if (window.__updateSketchInspector) window.__updateSketchInspector(); return true; }
        if (k === 'v') {
          if (e.shiftKey) {
            sketchState.showValidation = !sketchState.showValidation;
            window.__setStatusMessage('Проверка: ' + (sketchState.showValidation ? 'вкл' : 'выкл'));
          } else {
            __keyVertical();
          }
          if (window.__updateSketchInspector) window.__updateSketchInspector();
          return true;
        }

        return false;
      };

      // ─────────────────────────────────────────────────────────
      // __updateLinePreview() — line tool ghost preview
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
        let h = sketchState.hoverWorld;
        // ── Ortho / Angle lock: snap to nearest 45° from start point ──
        if (sketchState.orthoLock) {
          h = window.__applyOrthoSnap(a, h, sketchState.workingPlane || 'XZ');
        }
        line.previewPoint  = { x: h.x, y: h.y, z: h.z, gx: h.gx, gy: h.gy, gz: h.gz,
                               _orthoAxis: h._orthoAxis || null };
        line.previewLength = Math.hypot(h.x - a.x, h.y - a.y, h.z - a.z);
        const samePos        = (a.gx === h.gx && a.gy === h.gy && a.gz === h.gz);
        const targetExisting = window.__findPointAtGrid(h.gx, h.gy, h.gz);
        const dupEdge        = targetExisting && window.__findEdgeBetween(a.id, targetExisting.id);
        line.previewValid = !samePos && !dupEdge;
      };
"##;
