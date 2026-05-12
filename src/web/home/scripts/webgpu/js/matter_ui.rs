// ── JS: Matter Lab UI — toolbar binding, inspector, status bar, FPS sync ─────
// Domain: Application — UI glue. Includes constraint + validation inspector.

pub const JS: &str = r##"
      // ── Per-tool helper text ──
      const TOOL_HINTS = {
        select: 'Click pick · Shift+click toggle · Dbl-click edge → endpoints / profile · D dim · F fix · H/V align',
        point:  'Click to place point on current plane · 1/2/3 switch plane',
        line:   'Click two points to draw an edge · keep clicking to chain · Enter/Esc to finish',
        grab:   'Drag selected (non-fixed) points · X / Y / Z to lock axis · Enter/click confirm · Esc cancel',
        delete: 'Click a point or edge to delete · ⌫/Del removes current selection · cascades constraints',
      };

      window.__setSketchTool = function(tool) {
        const valid = ['select', 'point', 'line', 'grab', 'delete'];
        if (!valid.includes(tool)) return;
        if (sketchState.activeTool !== tool) {
          sketchState.line.startPointId = null;
          sketchState.line.previewPoint = null;
          sketchState.phase = 'idle';
        }
        sketchState.activeTool = tool;
        if (tool === 'grab' && sketchState.selectedPointIds.size > 0 && !sketchState.grab.active) {
          const allIds  = [...sketchState.selectedPointIds];
          const moveIds = allIds.filter(id => !(window.__isPointFixed && window.__isPointFixed(id)));
          if (!moveIds.length) {
            window.__setStatusMessage('Cannot move fixed point');
          } else {
            const byId = new Map(sketchState.points.map(p => [p.id, p]));
            const snapshot = new Map();
            for (const id of moveIds) {
              const p = byId.get(id);
              if (p) snapshot.set(id, { x: p.x, y: p.y, z: p.z });
            }
            window.__pushHistory();
            sketchState.grab = {
              active: true, pointIds: moveIds,
              startMouseWorld: sketchState.hoverWorld
                ? { x: sketchState.hoverWorld.x, y: sketchState.hoverWorld.y, z: sketchState.hoverWorld.z }
                : { x: 0, y: 0, z: 0 },
              originalPoints: snapshot,
              axisLock: null,
            };
          }
        }
        document.querySelectorAll('.utb-btn[data-tool]').forEach(btn => {
          btn.classList.toggle('active', btn.dataset.tool === tool);
        });
        if (window.__setCursorForTool) window.__setCursorForTool();
        if (window.__updateSketchInspector) window.__updateSketchInspector();
      };

      // ── Helpers for inspector formatting ──
      function __orientationOfEdge(edgeId) {
        if (window.__hasHorizontalConstraint && window.__hasHorizontalConstraint(edgeId)) return 'Horizontal';
        if (window.__hasVerticalConstraint   && window.__hasVerticalConstraint(edgeId))   return 'Vertical';
        return '—';
      }
      function __validationOfPoint(pointId) {
        const deg = window.__pointDegree ? window.__pointDegree(pointId) : 0;
        if (deg === 0) return 'isolated';
        if (deg === 1) return 'open';
        return 'connected';
      }

      window.__updateSketchInspector = function() {
        const set = (id, val) => { const el = document.getElementById(id); if (el) el.textContent = val; };
        const showBlock = (id, on) => { const el = document.getElementById(id); if (el) el.style.display = on ? '' : 'none'; };

        const tool = (sketchState.activeTool || 'select');
        set('si-tool',  tool.toUpperCase());
        set('si-plane', sketchState.workingPlane || 'XZ');
        set('si-points', String(sketchState.points.length));
        set('si-edges',  String(sketchState.edges.length));

        const selPts = [...sketchState.selectedPointIds];
        const selEds = [...sketchState.selectedEdgeIds];
        const totalSel = selPts.length + selEds.length;
        set('si-selected', String(totalSel));

        const byId = new Map(sketchState.points.map(p => [p.id, p]));

        showBlock('si-block-none',  totalSel === 0);
        showBlock('si-block-point', totalSel === 1 && selPts.length === 1);
        showBlock('si-block-edge',  totalSel === 1 && selEds.length === 1);
        showBlock('si-block-multi', totalSel > 1);

        if (totalSel === 1 && selPts.length === 1) {
          const p = byId.get(selPts[0]);
          if (p) {
            set('si-pt-id', p.id);
            set('si-pt-grid', '(' + p.gx + ', ' + p.gy + ', ' + p.gz + ')');
            set('si-pt-world', p.x.toFixed(2) + ', ' + p.y.toFixed(2) + ', ' + p.z.toFixed(2));
            const deg = window.__pointDegree(p.id);
            const edgesAt = window.__edgesAtPoint ? window.__edgesAtPoint(p.id) : [];
            set('si-pt-degree', String(deg) + (edgesAt.length ? ' [' + edgesAt.join(', ') + ']' : ''));
            set('si-pt-fixed', window.__isPointFixed(p.id) ? 'yes' : 'no');
            set('si-pt-valid', __validationOfPoint(p.id));
          }
        } else if (totalSel === 1 && selEds.length === 1) {
          const e = sketchState.edges.find(x => x.id === selEds[0]);
          if (e) {
            const a = byId.get(e.a), b = byId.get(e.b);
            set('si-eg-id', e.id);
            set('si-eg-from', a ? (a.id + ' (' + a.gx + ',' + a.gy + ',' + a.gz + ')') : '—');
            set('si-eg-to',   b ? (b.id + ' (' + b.gx + ',' + b.gy + ',' + b.gz + ')') : '—');
            set('si-eg-len', window.__edgeLength(e).toFixed(3) + ' u');
            const dim = window.__getEdgeLengthConstraint(e.id);
            set('si-eg-dim', dim ? (dim.value.toFixed(3) + ' u') : '—');
            set('si-eg-orient', __orientationOfEdge(e.id));
            const prof = window.__getProfileForEdge(e.id);
            set('si-eg-profile', prof ? prof.id + ' (' + prof.plane + ')' : '—');
            if (a && b) {
              set('si-eg-dx', (b.x - a.x).toFixed(2));
              set('si-eg-dy', (b.y - a.y).toFixed(2));
              set('si-eg-dz', (b.z - a.z).toFixed(2));
            }
          }
        } else if (totalSel > 1) {
          set('si-multi-pts', String(selPts.length));
          set('si-multi-eds', String(selEds.length));
          let totalLen = 0;
          for (const id of selEds) {
            const e = sketchState.edges.find(x => x.id === id);
            if (e) totalLen += window.__edgeLength(e);
          }
          set('si-multi-len', totalLen.toFixed(3) + ' u');
          let fixedN = 0;
          for (const pid of selPts) if (window.__isPointFixed(pid)) fixedN++;
          let constrainedN = 0;
          for (const eid of selEds) {
            if (window.__getEdgeLengthConstraint(eid) ||
                window.__hasHorizontalConstraint(eid) ||
                window.__hasVerticalConstraint(eid)) constrainedN++;
          }
          set('si-multi-fixed', String(fixedN));
          set('si-multi-constr', String(constrainedN));
        }

        // ── Sketch-wide stats (always visible) ──
        const opens     = window.__countOpenEnds ? window.__countOpenEnds() : 0;
        const isolated  = window.__countIsolatedPoints ? window.__countIsolatedPoints() : 0;
        const profilesN = (sketchState.profiles || []).length;
        set('si-open-ends', String(opens));
        set('si-isolated',  String(isolated));
        set('si-profiles',  String(profilesN));
        set('si-validation', sketchState.showValidation ? 'on' : 'off');

        // ── Tool hint ──
        set('si-hint', TOOL_HINTS[tool] || '');

        // ── Mini status bar ──
        set('mini-tool',  tool.toUpperCase());
        set('mini-plane', sketchState.workingPlane || 'XZ');
        let snapTxt;
        if (sketchState.snap.kind === 'point' && sketchState.snap.pointId) snapTxt = 'existing ' + sketchState.snap.pointId;
        else if (sketchState.snap.kind === 'grid')                          snapTxt = 'grid ' + sketchState.snap.gx + ',' + sketchState.snap.gy + ',' + sketchState.snap.gz;
        else                                                                snapTxt = '—';
        set('mini-snap', snapTxt);
        let lenTxt = '—';
        if (tool === 'line' && sketchState.line.startPointId && sketchState.line.previewPoint) {
          lenTxt = (sketchState.line.previewLength || 0).toFixed(2) + ' u';
        } else if (totalSel === 1 && selEds.length === 1) {
          const e = sketchState.edges.find(x => x.id === selEds[0]);
          if (e) lenTxt = window.__edgeLength(e).toFixed(2) + ' u';
        }
        set('mini-length', lenTxt);

        // Plane pill highlight
        document.querySelectorAll('.plane-pill[data-plane]').forEach(btn => {
          btn.classList.toggle('active', btn.dataset.plane === (sketchState.workingPlane || 'XZ'));
        });

        // ── Precision block (Phase 7) ──
        const gridInput = document.getElementById('si-grid-size');
        if (gridInput && document.activeElement !== gridInput) {
          const cur = parseFloat(gridInput.value);
          if (!isFinite(cur) || Math.abs(cur - sketchState.gridSize) > 1e-9) {
            gridInput.value = String(sketchState.gridSize);
          }
        }
        const beChk = document.getElementById('si-use-backend');
        if (beChk) beChk.checked = !!sketchState.useBackendCommands;
        set('si-backend-onoff', sketchState.useBackendCommands ? 'ON' : 'OFF');
        const bs = sketchState.backendStatus || {};
        let last;
        if (bs.ok === true)       last = '✓ ' + (bs.message || 'OK');
        else if (bs.ok === false) last = '✕ ' + (bs.message || 'ERROR');
        else                      last = '—';
        set('si-backend-last', last);
      };

      function bindMatterUi() {
        document.querySelectorAll('.utb-btn[data-tool]').forEach(btn => {
          btn.addEventListener('click', () => window.__setSketchTool(btn.dataset.tool));
        });
        document.querySelectorAll('.plane-pill[data-plane]').forEach(btn => {
          btn.addEventListener('click', () => window.__setWorkingPlane(btn.dataset.plane));
        });
        const closeBtn = document.getElementById('close-chefos');
        if (closeBtn) {
          closeBtn.addEventListener('click', () => {
            document.body.classList.remove('engine-open');
            if (window.__stopWebGpuScene) window.__stopWebGpuScene();
          });
        }
        const siToggle = document.getElementById('si-tab');
        const siPanel  = document.getElementById('sketch-inspector');
        if (siToggle && siPanel) {
          siToggle.addEventListener('click', () => {
            const open = siPanel.classList.toggle('open');
            siToggle.classList.toggle('open', open);
            const stage = document.querySelector('.matter-stage');
            if (stage) stage.classList.toggle('inspector-open', open);
          });
        }
        if (window.__setCursorForTool) window.__setCursorForTool();
        if (window.__notifySketchChanged) window.__notifySketchChanged();
        if (window.__bindSketchIO) window.__bindSketchIO();

        // ── Precision controls (Phase 7) ──
        const gridInput = document.getElementById('si-grid-size');
        if (gridInput) {
          gridInput.addEventListener('change', () => {
            let v = parseFloat(gridInput.value);
            if (!isFinite(v)) v = 1;
            v = Math.max(0.001, Math.min(1000, v));
            sketchState.gridSize = v;
            gridInput.value = String(v);
            window.__setStatusMessage('Grid size: ' + v);
            if (window.__notifySketchChanged) window.__notifySketchChanged();
            window.__updateSketchInspector();
          });
        }
        const beChk = document.getElementById('si-use-backend');
        if (beChk) {
          beChk.addEventListener('change', () => {
            window.__setUseBackendCommands(beChk.checked);
          });
        }

        window.__updateSketchInspector();
        setInterval(() => {
          const fps = (globalThis.__matterPerf && globalThis.__matterPerf.fps) || 0;
          const ms  = (globalThis.__matterPerf && globalThis.__matterPerf.frameMs) || 0;
          const fpsEl = document.getElementById('fpsValue');
          const msEl  = document.getElementById('frameValue');
          if (fpsEl) fpsEl.textContent = fps.toFixed(0);
          if (msEl)  msEl.textContent  = ms.toFixed(1) + ' ms';
          window.__updateSketchInspector();
        }, 120);
      }
      window.bindMatterUi = bindMatterUi;
      if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', bindMatterUi);
      } else {
        bindMatterUi();
      }
"##;
