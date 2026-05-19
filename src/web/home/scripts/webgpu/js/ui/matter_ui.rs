// ── JS: Matter Lab UI — toolbar binding, inspector, status bar, FPS sync ─────
// Domain: Application — UI glue. Includes constraint + validation inspector.

pub const JS: &str = r##"
      // ── Per-tool helper text ──
      const TOOL_HINTS = {
        select: 'Клик — выбор · Shift+клик — переключение · Двойной клик на ребре → точки / профиль · D размер · F фикс · H/V выравнивание',
        point:  'Клик — разместить точку на текущей плоскости · 1/2/3 — сменить плоскость',
        line:   'Клик двух точек — провести ребро · продолжай кликать для цепочки · Enter/Esc — завершить',
        grab:   'Тяни выбранные (незафиксированные) точки · X / Y / Z — ограничить ось · Enter/клик — подтвердить · Esc — отмена',
        delete: 'Клик на точку или ребро — удалить · ⌫/Del удаляет выделение · каскадно удаляет связанные ограничения',
      };
      const TOOL_NAMES_RU = { select: 'ВЫБОР', point: 'ТОЧКА', line: 'ЛИНИЯ', grab: 'ЗАХВАТ', delete: 'УДАЛИТЬ' };

      window.__setSketchTool = function(tool) {
        const valid = ['select', 'point', 'line', 'rect', 'circle', 'grab', 'delete'];
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
          // Tear down any active Rect Tool when switching tools.
          if (sketchState.circle && sketchState.circle.active) {
            if (window.__cancelCircleTool) window.__cancelCircleTool();
            else sketchState.circle = { active: false, centerSnap: null };
          }
          if (sketchState.rect && sketchState.rect.active) {
            if (window.__cancelRectTool) window.__cancelRectTool();
            else sketchState.rect = { active: false, startSnap: null };
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
            window.__setStatusMessage('Захват: сначала выделите точки, рёбра или профиль');
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
        if (window.__hasHorizontalConstraint && window.__hasHorizontalConstraint(edgeId)) return 'Горизонталь';
        if (window.__hasVerticalConstraint   && window.__hasVerticalConstraint(edgeId))   return 'Вертикаль';
        return '—';
      }
      function __validationOfPoint(pointId) {
        const deg = window.__pointDegree ? window.__pointDegree(pointId) : 0;
        if (deg === 0) return 'изолирована';
        if (deg === 1) return 'открыта';
        return 'соединена';
      }

      window.__updateSketchInspector = function() {
        const set = (id, val) => { const el = document.getElementById(id); if (el) el.textContent = val; };
        const showBlock = (id, on) => { const el = document.getElementById(id); if (el) el.style.display = on ? '' : 'none'; };

        const tool = (sketchState.activeTool || 'select');
        set('si-tool',  TOOL_NAMES_RU[tool] || tool.toUpperCase());
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
            set('si-eg-len', window.__fmtLength ? window.__fmtLength(window.__edgeLength(e)) : window.__edgeLength(e).toFixed(3) + ' u');
            const dim = window.__getEdgeLengthConstraint(e.id);
            set('si-eg-dim', dim ? (window.__fmtLength ? window.__fmtLength(dim.value) : dim.value.toFixed(3) + ' u') : '—');
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
          set('si-multi-len', window.__fmtLength ? window.__fmtLength(totalLen) : totalLen.toFixed(3) + ' u');
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

        // ── Ortho button sync ──
        const btnO = document.getElementById('btn-ortho');
        if (btnO) btnO.classList.toggle('active', !!sketchState.orthoLock);

        // ── Mini status bar ──
        const planeLbl = window.__planeLabel
          ? window.__planeLabel(sketchState.workingPlane)
          : (sketchState.workingPlane || 'XZ');
        set('mini-mode',  sketchState.draftMode === 'projection' ? 'Проекция' : 'Свободный 3D');
        set('mini-tool',  TOOL_NAMES_RU[tool] || tool.toUpperCase());
        set('mini-plane', sketchState.draftMode === 'projection'
          ? (sketchState.workingPlane + ' ' + planeLbl)
          : (sketchState.workingPlane || 'XZ'));
        const snap = sketchState.snap || { kind: 'none' };
        const hw   = sketchState.hoverWorld;

        // ── Shared format helpers ─────────────────────────────────────────
        const formatCoord   = window.__fmtCoord  || (v => Number(v).toFixed(3));
        const formatLengthMm = window.__fmtLength || (v => (Number(v) * 1000).toFixed(1) + ' mm');
        // Keep legacy aliases used elsewhere in this function
        const fmtC = formatCoord;
        const fmtL = formatLengthMm;

        let snapTxt;
        if (snap.kind === 'point' && snap.pointId)  snapTxt = 'ТОЧКА ' + snap.pointId;
        else if (snap.kind === 'grid')               snapTxt = 'СЕТКА '  + snap.gx + ',' + snap.gy + ',' + snap.gz;
        else if (snap.kind === 'free' && hw)         snapTxt = 'СВОБОДНО '  + formatCoord(hw.x) + ' ' + formatCoord(hw.y) + ' ' + formatCoord(hw.z);
        else                                          snapTxt = '—';
        set('mini-snap', snapTxt);

        // ── Cursor HUD: coordinates + length — shown only when __cursorInfoVisible ──
        const cursorHud = document.getElementById('cursor-hud');
        // Initialise global toggle state on first call
        if (window.__cursorInfoVisible === undefined) window.__cursorInfoVisible = false;

        // ? button reflects active state
        const shortcutsToggle = document.getElementById('shortcuts-toggle');
        if (shortcutsToggle) shortcutsToggle.classList.toggle('active', !!window.__cursorInfoVisible);

        const isDrawing = (tool === 'line' && sketchState.line && sketchState.line.startPointId)
                        || (tool === 'grab' && sketchState.selectedPointIds && sketchState.selectedPointIds.size > 0);

        if (cursorHud) {
          // Show only when user toggled cursor info ON AND cursor is over the canvas
          if (window.__cursorInfoVisible && (isDrawing || hw)) {
            cursorHud.style.display = '';
            const setH = (id, v) => { const el = document.getElementById(id); if (el) el.textContent = v; };
            if (hw) {
              setH('chud-x', formatCoord(hw.x));
              setH('chud-y', formatCoord(hw.y));
              setH('chud-z', formatCoord(hw.z));
            }
            // Length and angle
            let lenTxt = '—', angTxt = null;
            if (tool === 'line' && sketchState.line.startPointId && sketchState.line.previewPoint) {
              lenTxt = formatLengthMm(sketchState.line.previewLength || 0);
              if (sketchState.line.previewAngleDeg !== undefined)
                angTxt = sketchState.line.previewAngleDeg.toFixed(1) + '°';
            } else if (totalSel === 1 && selEds.length === 1) {
              const e = sketchState.edges.find(x => x.id === selEds[0]);
              if (e) lenTxt = formatLengthMm(window.__edgeLength(e));
            }
            setH('chud-len', lenTxt);
            // Snap type row
            setH('chud-snap', snap.kind ? (({point:'ТОЧКА',grid:'СЕТКА',free:'СВОБОДНО'}[snap.kind] || snap.kind.toUpperCase())) : '—');
            const angRow = cursorHud.querySelector('.chud-ang');
            if (angRow) {
              angRow.style.display = angTxt !== null ? '' : 'none';
              if (angTxt !== null) setH('chud-ang', angTxt);
            }
            // Follow cursor with auto-flip to avoid going off-screen
            if (window._lastMouseX !== undefined) {
              const labW = document.getElementById('matter-lab');
              const labRect = labW ? labW.getBoundingClientRect() : { left: 0, top: 0, right: window.innerWidth, bottom: window.innerHeight };
              const hudW = cursorHud.offsetWidth  || 120;
              const hudH = cursorHud.offsetHeight || 100;
              const ox = window._lastMouseX - labRect.left;
              const oy = window._lastMouseY - labRect.top;
              const flipX = (ox + 24 + hudW) > labRect.width;
              const flipY = (oy + 24 + hudH) > labRect.height;
              cursorHud.style.left = (flipX ? ox - hudW - 8 : ox + 20) + 'px';
              cursorHud.style.top  = (flipY ? oy - hudH - 8 : oy + 20) + 'px';
            }
          } else {
            cursorHud.style.display = 'none';
          }
        }

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

        // ── Drafting overlay sync (Phase 16) ──
        const df = sketchState.drafting || {};
        const dfMap = [
          ['si-df-dims',   'showDimensions'],
          ['si-df-edges',  'showEdgeLengths'],
          ['si-df-points', 'showPointLabels'],
          ['si-df-grid',   'showGridNumbers'],
          ['si-df-center', 'showCenterlines'],
        ];
        for (const [id, key] of dfMap) {
          const el = document.getElementById(id);
          if (el && document.activeElement !== el) el.checked = !!df[key];
        }
        const decEl = document.getElementById('si-df-decimals');
        if (decEl && document.activeElement !== decEl) {
          const v = (typeof df.decimals === 'number') ? df.decimals : 1;
          if (parseInt(decEl.value, 10) !== v) decEl.value = String(v);
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
          set('si-pf-area', window.__fmtArea ? window.__fmtArea(area) : (area * 1e6).toFixed(1) + ' mm\u00b2');
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
            window.__setStatusMessage('Шаг привязки: ' + mm + ' мм');
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
            window.__setStatusMessage('Сетка отображения: ' + mm + ' мм');
            window.__updateSketchInspector();
          });
        }

        // ── Drafting overlay toggles (Phase 16) ──
        if (!sketchState.drafting) {
          sketchState.drafting = {
            showDimensions: true, showEdgeLengths: false,
            showPointLabels: false, showGridNumbers: false, showCenterlines: false,
            unit: 'mm', decimals: 1,
            dimensionOffsetPx: 20, arrowSizePx: 7, textGapPx: 6,
          };
        }
        const dfBindings = [
          ['si-df-dims',   'showDimensions'],
          ['si-df-edges',  'showEdgeLengths'],
          ['si-df-points', 'showPointLabels'],
          ['si-df-grid',   'showGridNumbers'],
          ['si-df-center', 'showCenterlines'],
        ];
        for (const [id, key] of dfBindings) {
          const el = document.getElementById(id);
          if (!el) continue;
          el.checked = !!sketchState.drafting[key];
          el.addEventListener('change', () => {
            sketchState.drafting[key] = el.checked;
            window.__setStatusMessage('Черчение · ' + key + ' = ' + (el.checked ? 'вкл' : 'выкл'));
          });
        }
        const decEl = document.getElementById('si-df-decimals');
        if (decEl) {
          decEl.value = String((typeof sketchState.drafting.decimals === 'number')
                               ? sketchState.drafting.decimals : 1);
          decEl.addEventListener('change', () => {
            let v = parseInt(decEl.value, 10);
            if (!isFinite(v)) v = 1;
            v = Math.max(0, Math.min(3, v));
            decEl.value = String(v);
            sketchState.drafting.decimals = v;
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
            window.__setStatusMessage('Точность тачпада: ' + (tpEl.checked ? 'вкл' : 'выкл'));
          });
        }

        // Engine mode dropdown removed — CAD Engine is now the only unified mode

        // ── Profile selection buttons (Phase 8) ──
        const pfCopy = document.getElementById('si-pf-copy');
        if (pfCopy) {
          pfCopy.addEventListener('click', async () => {
            const payload = window.__selectedProfileToPayload();
            if (!payload) { window.__setStatusMessage('Профиль не выбран'); return; }
            const txt = JSON.stringify(payload, null, 2);
            try {
              await navigator.clipboard.writeText(txt);
              window.__setStatusMessage('Профиль скопирован: ' + payload.profileId);
            } catch (e) {
              window.__setStatusMessage('Ошибка буфера обмена');
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
