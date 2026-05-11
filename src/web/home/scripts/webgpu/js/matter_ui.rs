// ── JS: Matter Lab UI — toolbar binding, inspector, status bar, FPS sync ─────
// Domain: Application — UI glue.

pub const JS: &str = r##"
      // ── Per-tool helper text ──
      const TOOL_HINTS = {
        select: 'Click to pick · Shift+click to add/toggle · Double-click edge: select both endpoints',
        point:  'Click to place a point on current plane · 1/2/3 switch plane',
        line:   'Click two points to draw an edge · keep clicking to chain · Enter/Esc to finish',
        grab:   'Drag selected points · X / Y / Z to lock axis · Enter confirm · Esc cancel',
        delete: 'Click a point or edge to delete · ⌫/Del removes current selection',
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
          const ids = [...sketchState.selectedPointIds];
          const byId = new Map(sketchState.points.map(p => [p.id, p]));
          const snapshot = new Map();
          for (const id of ids) {
            const p = byId.get(id);
            if (p) snapshot.set(id, { x: p.x, y: p.y, z: p.z });
          }
          window.__pushHistory();
          sketchState.grab = {
            active: true, pointIds: ids,
            startMouseWorld: sketchState.hoverWorld
              ? { x: sketchState.hoverWorld.x, y: sketchState.hoverWorld.y, z: sketchState.hoverWorld.z }
              : { x: 0, y: 0, z: 0 },
            originalPoints: snapshot,
            axisLock: null,
          };
        }
        document.querySelectorAll('.utb-btn[data-tool]').forEach(btn => {
          btn.classList.toggle('active', btn.dataset.tool === tool);
        });
        if (window.__setCursorForTool) window.__setCursorForTool();
        if (window.__updateSketchInspector) window.__updateSketchInspector();
      };

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

        // ── Per-selection detail blocks ──
        const byId = new Map(sketchState.points.map(p => [p.id, p]));

        // No selection
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
            set('si-pt-degree', String(window.__pointDegree(p.id)));
          }
        } else if (totalSel === 1 && selEds.length === 1) {
          const e = sketchState.edges.find(x => x.id === selEds[0]);
          if (e) {
            const a = byId.get(e.a), b = byId.get(e.b);
            set('si-eg-id', e.id);
            set('si-eg-from', a ? (a.id + ' (' + a.gx + ',' + a.gy + ',' + a.gz + ')') : '—');
            set('si-eg-to',   b ? (b.id + ' (' + b.gx + ',' + b.gy + ',' + b.gz + ')') : '—');
            set('si-eg-len', window.__edgeLength(e).toFixed(3) + ' u');
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
        }

        // ── Open ends ──
        const ends = window.__countOpenEnds ? window.__countOpenEnds() : 0;
        set('si-open-ends', String(ends));

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
        if (window.__setCursorForTool) window.__setCursorForTool();
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
