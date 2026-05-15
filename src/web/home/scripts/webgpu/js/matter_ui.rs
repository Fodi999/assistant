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
          // Tear down any active Line Tool chain when switching tools.
          if (sketchState.line && (sketchState.line.active || sketchState.line.startPointId)) {
            if (window.__finishLineChain) window.__finishLineChain('tool switched');
            else {
              sketchState.line = { active: false, startPointId: null, startWorld: null };
              sketchState.phase = 'idle';
            }
          }
          sketchState.phase = 'idle';
        }
        sketchState.activeTool = tool;
        if (tool === 'grab' && !sketchState.grab.active) {
          const hasSelection = sketchState.selectedPointIds.size > 0
            || sketchState.selectedEdgeIds.size > 0
            || sketchState.selectedProfileId != null;
          if (hasSelection) {
            // Delegate to __startGrab which handles edges + profiles too
            if (window.__startGrab) window.__startGrab();
          } else {
            window.__setStatusMessage('Grab: select points, edges, or a profile first');
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
        const planeLbl = window.__planeLabel
          ? window.__planeLabel(sketchState.workingPlane)
          : (sketchState.workingPlane || 'XZ');
        set('mini-mode',  sketchState.draftMode === 'projection' ? 'Projection' : 'Free 3D');
        set('mini-tool',  tool.toUpperCase());
        set('mini-plane', sketchState.draftMode === 'projection'
          ? (sketchState.workingPlane + ' ' + planeLbl)
          : (sketchState.workingPlane || 'XZ'));
        const snap = sketchState.snap || { kind: 'none' };
        const hw   = sketchState.hoverWorld;
        const fmtC = window.__fmtCoord || (v => Number(v).toFixed(3));
        const fmtL = window.__fmtLength || (v => Number(v).toFixed(3));
        let snapTxt;
        if (snap.kind === 'point' && snap.pointId)  snapTxt = 'POINT ' + snap.pointId;
        else if (snap.kind === 'grid')               snapTxt = 'GRID '  + snap.gx + ',' + snap.gy + ',' + snap.gz;
        else if (snap.kind === 'free' && hw)         snapTxt = 'FREE '  + fmtC(hw.x) + ' ' + fmtC(hw.y) + ' ' + fmtC(hw.z);
        else                                          snapTxt = '—';
        set('mini-snap', snapTxt);
        if (hw)  set('mini-grid', fmtC(hw.x) + ' ' + fmtC(hw.y) + ' ' + fmtC(hw.z));
        else     set('mini-grid', '—');
        let lenTxt = '—';
        if (tool === 'line' && sketchState.line.startPointId && sketchState.line.previewPoint) {
          lenTxt = fmtL(sketchState.line.previewLength || 0) + ' u';
        } else if (totalSel === 1 && selEds.length === 1) {
          const e = sketchState.edges.find(x => x.id === selEds[0]);
          if (e) lenTxt = fmtL(window.__edgeLength(e)) + ' u';
        }
        set('mini-length', lenTxt);

        // Plane pill highlight
        document.querySelectorAll('.plane-pill[data-plane]').forEach(btn => {
          btn.classList.toggle('active', btn.dataset.plane === (sketchState.workingPlane || 'XZ'));
        });

        // ── Precision block (Phase 15: split internal / snap / display) ──
        const pr = sketchState.precision || {};
        // Internal step is a fixed-from-UI read-only label (in mm).
        const internalEl = document.getElementById('si-internal-step');
        if (internalEl) {
          const mm = (pr.internalStepM || 0.00001) * 1000;
          internalEl.textContent = mm.toFixed(3).replace(/\.?0+$/, '') + ' mm';
        }
        // Snap step input (mm).
        const snapInput = document.getElementById('si-snap-step');
        if (snapInput && document.activeElement !== snapInput) {
          const mm  = (pr.snapStepM || 0.001) * 1000;
          const cur = parseFloat(snapInput.value);
          if (!isFinite(cur) || Math.abs(cur - mm) > 1e-6) snapInput.value = String(mm);
        }
        // Display-grid step input (mm).
        const dispInput = document.getElementById('si-display-step');
        if (dispInput && document.activeElement !== dispInput) {
          const mm  = (pr.displayGridStepM || 0.001) * 1000;
          const cur = parseFloat(dispInput.value);
          if (!isFinite(cur) || Math.abs(cur - mm) > 1e-6) dispInput.value = String(mm);
        }
        const beChk = document.getElementById('si-use-backend');
        if (beChk) beChk.checked = !!sketchState.useBackendCommands;
        const onoffEl = document.getElementById('si-backend-onoff');
        if (onoffEl) onoffEl.textContent = sketchState.useBackendCommands ? 'ON' : 'OFF';
        // Last result: use unified lastCommandMsg (set by all engine modes).
        set('si-backend-last', sketchState.lastCommandMsg || '—');

        // ── CAD Engine metrics (unified WASM-first + backend sync) ──
        // WASM status
        const wstatusEl = document.getElementById('si-wasm-status');
        if (wstatusEl && window.cadState) {
          const st = window.cadState.wasmStatus || 'not_loaded';
          wstatusEl.textContent = st;
          wstatusEl.className   = 'si-wasm-status si-wasm-' + st;
        }
        // Backend status (sync)
        const bestatusEl = document.getElementById('si-backend-status');
        if (bestatusEl && window.cadState) {
          const st = window.cadState.backendStatus || 'idle';
          bestatusEl.textContent = st;
          bestatusEl.className = 'si-backend-status si-backend-' + st;
        }
        // Timing metrics
        if (window.cadState) {
          set('si-last-wasm-ms', (window.cadState.lastWasmMs || 0).toFixed(2) + ' ms');
          set('si-last-be-ms',   (window.cadState.lastBackendMs || 0).toFixed(1) + ' ms');
        }
        // Pending operations count
        set('si-cad-pending', window.cadState ? String(window.cadState.pending) : '0');

        // ── Profile selection block (Phase 8) ──
        const profSel = window.__getSelectedProfile && window.__getSelectedProfile();
        showBlock('si-block-profile', !!profSel);
        if (profSel) {
          set('si-pf-id', profSel.id);
          set('si-pf-plane', profSel.plane || '—');
          set('si-pf-points', String(profSel.pointIds.length));
          set('si-pf-edges',  String(profSel.edgeIds.length));
          const area = window.__profileArea(profSel);
          set('si-pf-area', area.toFixed(3) + ' u²');
          set('si-pf-ready', window.__isSelectedProfileExtrudable() ? 'yes' : 'no');
        }

        // Edge inspector — show all profiles if edge belongs to many.
        if (totalSel === 1 && selEds.length === 1) {
          const eid = selEds[0];
          const profs = window.__getProfilesForEdge ? window.__getProfilesForEdge(eid) : [];
          if (profs.length > 1) set('si-eg-profile', profs.map(p => p.id).join(', '));
        }

        // ── Profile list (Phase 8) ──
        const listEl = document.getElementById('si-pf-list');
        const cntEl  = document.getElementById('si-pf-count');
        if (cntEl) cntEl.textContent = String((sketchState.profiles || []).length);
        if (listEl) {
          listEl.innerHTML = '';
          for (const pf of (sketchState.profiles || [])) {
            const li = document.createElement('li');
            li.dataset.profileId = pf.id;
            if (pf.id === sketchState.selectedProfileId) li.classList.add('selected');
            const area = window.__profileArea(pf);
            const left = document.createElement('span');
            left.textContent = pf.id;
            const right = document.createElement('span');
            right.className = 'pf-meta';
            right.textContent = (pf.plane || '?') + ' · ' + pf.edgeIds.length + 'e · ' + area.toFixed(2);
            li.appendChild(left); li.appendChild(right);
            li.addEventListener('click', () => {
              window.__selectProfile(pf.id);
              window.__updateSketchInspector();
            });
            listEl.appendChild(li);
          }
        }
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

        // ── Precision controls (Phase 15: split internal / snap / display) ──
        // Snap step (UI in mm, stored in meters).
        const snapInput = document.getElementById('si-snap-step');
        if (snapInput) {
          snapInput.addEventListener('change', () => {
            let mm = parseFloat(snapInput.value);
            if (!isFinite(mm) || mm <= 0) mm = 1;
            mm = Math.max(0.001, Math.min(1000, mm));
            snapInput.value = String(mm);
            if (!sketchState.precision) sketchState.precision = {};
            sketchState.precision.snapStepM = mm / 1000;
            window.__setStatusMessage('Snap step: ' + mm + ' mm');
            if (window.__notifySketchChanged) window.__notifySketchChanged();
            window.__updateSketchInspector();
          });
        }
        // Display grid step (UI in mm, stored in meters).
        const dispInput = document.getElementById('si-display-step');
        if (dispInput) {
          dispInput.addEventListener('change', () => {
            let mm = parseFloat(dispInput.value);
            if (!isFinite(mm) || mm <= 0) mm = 1;
            mm = Math.max(0.01, Math.min(1000, mm));
            dispInput.value = String(mm);
            if (!sketchState.precision) sketchState.precision = {};
            sketchState.precision.displayGridStepM = mm / 1000;
            window.__setStatusMessage('Display grid: ' + mm + ' mm');
            window.__updateSketchInspector();
          });
        }
        const beChk = document.getElementById('si-use-backend');
        if (beChk) {
          beChk.addEventListener('change', () => {
            window.__setUseBackendCommands(beChk.checked);
          });
        }

        // ── Snap-mode checkboxes (Phase 12 — Touchpad Precision) ──
        const snapBindings = [
          ['si-snap-grid',  'gridSnap'],
          ['si-snap-point', 'pointSnap'],
          ['si-snap-mid',   'midpointSnap'],
          ['si-snap-free',  'freeMode'],
        ];
        for (const [id, key] of snapBindings) {
          const el = document.getElementById(id);
          if (!el) continue;
          // Sync initial DOM state from sketchState.precision.
          el.checked = !!(sketchState.precision && sketchState.precision[key]);
          el.addEventListener('change', () => {
            window.__setSnapMode(key, el.checked);
            window.__updateSketchInspector();
          });
        }
        const tpEl = document.getElementById('si-touchpad-mode');
        if (tpEl) {
          tpEl.checked = !!(sketchState.precision && sketchState.precision.touchpadMode);
          tpEl.addEventListener('change', () => {
            if (sketchState.precision) sketchState.precision.touchpadMode = tpEl.checked;
            window.__setStatusMessage('Touchpad precision: ' + (tpEl.checked ? 'on' : 'off'));
          });
        }

        // Engine mode dropdown removed — CAD Engine is now the only unified mode

        // ── Profile selection buttons (Phase 8) ──
        const pfCopy = document.getElementById('si-pf-copy');
        if (pfCopy) {
          pfCopy.addEventListener('click', async () => {
            const payload = window.__selectedProfileToPayload();
            if (!payload) { window.__setStatusMessage('No profile selected'); return; }
            const txt = JSON.stringify(payload, null, 2);
            try {
              await navigator.clipboard.writeText(txt);
              window.__setStatusMessage('Copied profile ' + payload.profileId);
            } catch (e) {
              window.__setStatusMessage('Clipboard failed');
            }
          });
        }
        const pfClear = document.getElementById('si-pf-clear');
        if (pfClear) {
          pfClear.addEventListener('click', () => {
            window.__clearProfileSelection();
            window.__updateSketchInspector();
          });
        }

        // WASM controls removed — CAD Engine auto-loads WASM at startup

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
