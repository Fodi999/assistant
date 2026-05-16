// ── Dimension editor popup (Phase 17 v2) ────────────────────────────────
//
// Professional CAD-style editor opened by clicking an edge length dimension
// label. Provides:
//   • Length input + 3 fix modes (fixA_moveB / fixB_moveA / center_moveBoth)
//   • Point A coords (X / Y / Z) in mm
//   • Point B coords (X / Y / Z) in mm
//   • Apply / Cancel / Enter / Esc
//   • Change detection: coord edits override length edits.
//   • On error: popup stays open, error message shown inline.
//
// Entry: window.__openDimensionEditor(hit, px, py)
//   hit = { kind:'edge_length_dimension', edgeId, aPointId, bPointId, valueMm }

pub const JS: &str = r##"
      (function registerDimensionEditor() {

        const COLORS = {
          bg:      'rgba(15,23,42,0.97)',
          panel:   'rgba(30,41,59,0.9)',
          border:  'rgba(148,163,184,0.35)',
          frame:   'rgba(226,232,240,0.25)',
          fg:      '#e2e8f0',
          mute:    '#94a3b8',
          dim:     '#64748b',
          input:   '#f1f5f9',
          accent:  'rgba(99,102,241,0.85)',
          accent2: 'rgba(99,102,241,1.0)',
          danger:  '#f87171',
        };

        // ── Build / reuse popup ──────────────────────────────────────────
        function buildPopup() {
          const el = document.createElement('div');
          el.id = '__dim-editor';
          Object.assign(el.style, {
            position:      'fixed',
            zIndex:        '9999',
            display:       'none',
            background:    COLORS.bg,
            border:        '1px solid ' + COLORS.frame,
            borderRadius:  '8px',
            padding:       '12px 14px',
            boxShadow:     '0 8px 32px rgba(0,0,0,0.7)',
            fontFamily:    '"JetBrains Mono", system-ui, monospace',
            fontSize:      '12px',
            color:         COLORS.fg,
            minWidth:      '260px',
            pointerEvents: 'all',
            userSelect:    'none',
          });

          el.innerHTML = `
            <div class="dim-section">
              <div class="dim-label">Length (mm)</div>
              <input class="dim-input" data-k="len" type="text" />
              <div class="dim-modes">
                <label><input type="radio" name="dim-mode" value="fixA_moveB" checked> Fix A · move B</label>
                <label><input type="radio" name="dim-mode" value="fixB_moveA"> Fix B · move A</label>
                <label><input type="radio" name="dim-mode" value="center_moveBoth"> Center · move both</label>
              </div>
              <div class="dim-align-label">Align</div>
              <div class="dim-align">
                <button class="dim-axis-btn dim-axis-active" type="button" data-axis="free">Free</button>
                <button class="dim-axis-btn"                type="button" data-axis="X">X</button>
                <button class="dim-axis-btn"                type="button" data-axis="Y">Y</button>
                <button class="dim-axis-btn"                type="button" data-axis="Z">Z</button>
              </div>
            </div>

            <div class="dim-sep"></div>

            <div class="dim-section">
              <div class="dim-label">Point A (mm)</div>
              <div class="dim-row">
                <span class="dim-axis">X</span><input class="dim-input" data-k="ax" type="text" />
                <span class="dim-axis">Y</span><input class="dim-input" data-k="ay" type="text" />
                <span class="dim-axis">Z</span><input class="dim-input" data-k="az" type="text" />
              </div>
            </div>

            <div class="dim-section">
              <div class="dim-label">Point B (mm)</div>
              <div class="dim-row">
                <span class="dim-axis">X</span><input class="dim-input" data-k="bx" type="text" />
                <span class="dim-axis">Y</span><input class="dim-input" data-k="by" type="text" />
                <span class="dim-axis">Z</span><input class="dim-input" data-k="bz" type="text" />
              </div>
            </div>

            <div class="dim-error" data-k="err"></div>

            <div class="dim-buttons">
              <button class="dim-btn dim-cancel" type="button">Cancel</button>
              <button class="dim-btn dim-apply"  type="button">Apply</button>
            </div>

            <div class="dim-hint">Enter = Apply · Esc = Cancel</div>
          `;

          // Inject scoped styles once.
          if (!document.getElementById('__dim-editor-style')) {
            const css = document.createElement('style');
            css.id = '__dim-editor-style';
            css.textContent = `
              #__dim-editor .dim-section { margin-bottom:8px; }
              #__dim-editor .dim-sep {
                height:1px; background:${COLORS.frame};
                margin:8px 0;
              }
              #__dim-editor .dim-label {
                font-size:10px; color:${COLORS.mute};
                margin-bottom:4px; text-transform:uppercase;
                letter-spacing:0.5px;
              }
              #__dim-editor .dim-modes {
                display:flex; flex-direction:column; gap:2px;
                margin-top:6px; font-size:11px; color:${COLORS.fg};
              }
              #__dim-editor .dim-modes label {
                display:flex; align-items:center; gap:6px;
                cursor:pointer;
              }
              #__dim-editor .dim-modes input[type=radio] {
                accent-color:${COLORS.accent2};
              }
              #__dim-editor .dim-align-label {
                font-size:10px; color:${COLORS.mute};
                margin:8px 0 4px; text-transform:uppercase;
                letter-spacing:0.5px;
              }
              #__dim-editor .dim-align {
                display:grid;
                grid-template-columns: repeat(4, 1fr);
                gap:4px;
              }
              #__dim-editor .dim-axis-btn {
                background:${COLORS.panel};
                border:1px solid ${COLORS.border};
                border-radius:4px;
                color:${COLORS.fg};
                font-family:inherit; font-size:11px;
                padding:3px 0; cursor:pointer;
              }
              #__dim-editor .dim-axis-btn:hover { filter:brightness(1.2); }
              #__dim-editor .dim-axis-btn.dim-axis-active {
                background:${COLORS.accent}; border-color:${COLORS.accent2};
                color:#fff;
              }
              #__dim-editor .dim-row {
                display:grid;
                grid-template-columns: 14px 1fr 14px 1fr 14px 1fr;
                gap:4px; align-items:center;
              }
              #__dim-editor .dim-axis {
                color:${COLORS.mute}; font-size:11px; text-align:center;
              }
              #__dim-editor .dim-input {
                background:${COLORS.panel};
                border:1px solid ${COLORS.border};
                border-radius:4px;
                padding:3px 6px;
                color:${COLORS.input};
                font-family:inherit; font-size:12px;
                outline:none;
                width:100%; box-sizing:border-box;
                min-width:0;
              }
              #__dim-editor .dim-input:focus {
                border-color:${COLORS.accent2};
              }
              #__dim-editor .dim-section > .dim-input { width:90px; }
              #__dim-editor .dim-error {
                color:${COLORS.danger};
                font-size:11px;
                min-height:14px;
                margin:4px 0;
              }
              #__dim-editor .dim-buttons {
                display:flex; gap:6px; justify-content:flex-end;
                margin-top:6px;
              }
              #__dim-editor .dim-btn {
                background:${COLORS.panel};
                border:1px solid ${COLORS.border};
                border-radius:4px;
                color:${COLORS.fg};
                font-family:inherit; font-size:12px;
                padding:4px 12px; cursor:pointer;
              }
              #__dim-editor .dim-btn.dim-apply {
                background:${COLORS.accent}; border-color:${COLORS.accent2};
                color:#fff;
              }
              #__dim-editor .dim-btn:hover { filter:brightness(1.15); }
              #__dim-editor .dim-hint {
                margin-top:6px; font-size:9px; color:${COLORS.dim};
              }
            `;
            document.head.appendChild(css);
          }

          document.body.appendChild(el);

          // Block global click/pointer handlers from closing the editor or
          // hitting the canvas behind it.
          // IMPORTANT: pointerdown/mousedown use capture=true so the canvas
          // orbit handler (which runs in bubble phase) never sees them.
          // click/dblclick/contextmenu use bubble phase (capture=false) so
          // stopPropagation() only prevents them going UP to the canvas —
          // the event still descends normally to the Apply/Cancel buttons.
          ['pointerdown','mousedown'].forEach((evt) => {
            el.addEventListener(evt, (e) => { e.stopPropagation(); }, true);
          });
          ['click','dblclick','contextmenu'].forEach((evt) => {
            el.addEventListener(evt, (e) => { e.stopPropagation(); }, false);
          });

          return el;
        }

        function getPopup() {
          return document.getElementById('__dim-editor') || buildPopup();
        }

        function $(el, sel) { return el.querySelector(sel); }
        function getInput(el, key) { return el.querySelector('[data-k="'+key+'"]'); }
        function setError(el, msg) { el.querySelector('[data-k="err"]').textContent = msg || ''; }

        function closeEditor() {
          const el = document.getElementById('__dim-editor');
          if (el) {
            el.style.display = 'none';
            el.__state = null;
          }
        }

        // Compare two mm values with sub-step tolerance (half of 0.01 mm).
        function approxEq(a, b) {
          return Math.abs((Number(a)||0) - (Number(b)||0)) < 0.005;
        }

        // Read all input fields → parsed numbers.
        function readForm(el) {
          const num = (k) => window.__parseCadNumber(getInput(el, k).value);
          const mode = el.querySelector('input[name="dim-mode"]:checked')?.value || 'fixA_moveB';
          return {
            len: num('len'),
            mode,
            a: { x: num('ax'), y: num('ay'), z: num('az') },
            b: { x: num('bx'), y: num('by'), z: num('bz') },
          };
        }

        function validate(form) {
          const fields = [
            ['len', form.len, true],
            ['ax', form.a.x], ['ay', form.a.y], ['az', form.a.z],
            ['bx', form.b.x], ['by', form.b.y], ['bz', form.b.z],
          ];
          for (const [name, v, positive] of fields) {
            if (!isFinite(v)) return 'Invalid number in ' + name.toUpperCase();
            if (positive && v <= 0) return 'Length must be > 0';
          }
          return null;
        }

        // ── Axis align (preview) ─────────────────────────────────────────
        // Internal CAD step is 0.01 mm (see CAD_INTERNAL_STEP_MM).
        const STEP_MM = 0.01;
        function mmToGrid(mm) { return Math.round((Number(mm) || 0) / STEP_MM); }
        function gridToMm(g)  { return g * STEP_MM; }

        function getAxisSign(axis, a, b) {
          if (axis === 'X') return Math.sign(b.gx - a.gx) || 1;
          if (axis === 'Y') return Math.sign(b.gy - a.gy) || 1;
          if (axis === 'Z') return Math.sign(b.gz - a.gz) || 1;
          return 1;
        }

        function writePointAInputsMm(el, g) {
          const fmt = window.__formatCadNumberMm || ((n) => String(n));
          getInput(el, 'ax').value = fmt(gridToMm(g.gx), 2);
          getInput(el, 'ay').value = fmt(gridToMm(g.gy), 2);
          getInput(el, 'az').value = fmt(gridToMm(g.gz), 2);
        }
        function writePointBInputsMm(el, g) {
          const fmt = window.__formatCadNumberMm || ((n) => String(n));
          getInput(el, 'bx').value = fmt(gridToMm(g.gx), 2);
          getInput(el, 'by').value = fmt(gridToMm(g.gy), 2);
          getInput(el, 'bz').value = fmt(gridToMm(g.gz), 2);
        }

        // Apply axis preview to inputs. Returns true on success.
        function applyAxisPreview(el, axis) {
          if (axis === 'free') {
            el.__direction = 'free';
            setError(el, '');
            window.__setStatusMessage?.('Free direction');
            return true;
          }
          const form = readForm(el);
          if (!isFinite(form.len) || form.len <= 0) {
            setError(el, 'Invalid length'); return false;
          }
          const aG = { gx: mmToGrid(form.a.x), gy: mmToGrid(form.a.y), gz: mmToGrid(form.a.z) };
          const bG = { gx: mmToGrid(form.b.x), gy: mmToGrid(form.b.y), gz: mmToGrid(form.b.z) };
          const len = Math.round(form.len / STEP_MM);
          if (len <= 0) { setError(el, 'Length must be > 0'); return false; }
          const sign = getAxisSign(axis, aG, bG);

          let newA = { ...aG };
          let newB = { ...bG };

          if (form.mode === 'fixA_moveB') {
            newA = { ...aG };
            newB = { gx: aG.gx, gy: aG.gy, gz: aG.gz };
            if (axis === 'X') newB.gx = aG.gx + sign * len;
            if (axis === 'Y') newB.gy = aG.gy + sign * len;
            if (axis === 'Z') newB.gz = aG.gz + sign * len;
          } else if (form.mode === 'fixB_moveA') {
            newB = { ...bG };
            newA = { gx: bG.gx, gy: bG.gy, gz: bG.gz };
            if (axis === 'X') newA.gx = bG.gx - sign * len;
            if (axis === 'Y') newA.gy = bG.gy - sign * len;
            if (axis === 'Z') newA.gz = bG.gz - sign * len;
          } else { // center_moveBoth
            const cx = Math.round((aG.gx + bG.gx) / 2);
            const cy = Math.round((aG.gy + bG.gy) / 2);
            const cz = Math.round((aG.gz + bG.gz) / 2);
            const halfA = Math.floor(len / 2);
            const halfB = len - halfA;
            newA = { gx: cx, gy: cy, gz: cz };
            newB = { gx: cx, gy: cy, gz: cz };
            if (axis === 'X') { newA.gx = cx - sign * halfA; newB.gx = cx + sign * halfB; }
            if (axis === 'Y') { newA.gy = cy - sign * halfA; newB.gy = cy + sign * halfB; }
            if (axis === 'Z') { newA.gz = cz - sign * halfA; newB.gz = cz + sign * halfB; }
          }

          writePointAInputsMm(el, newA);
          writePointBInputsMm(el, newB);
          el.__direction = axis;
          setError(el, '');
          window.__setStatusMessage?.('Preview: aligned ' + axis);
          console.log('[DimEditor] align', axis, { mode: form.mode, sign, newA, newB });
          return true;
        }

        function updateAxisButtons(el, axis) {
          el.querySelectorAll('.dim-axis-btn').forEach((b) => {
            b.classList.toggle('dim-axis-active', b.dataset.axis === axis);
          });
        }

        // ── Apply ────────────────────────────────────────────────────────
        async function applyEdit(el) {
          console.log('[DimEditor] Apply clicked');
          const st = el.__state;
          if (!st) { console.warn('[DimEditor] no state on popup'); return; }
          const form = readForm(el);
          const verr = validate(form);
          if (verr) {
            console.warn('[DimEditor] validation error:', verr);
            setError(el, verr); return;
          }
          setError(el, '');

          // Detect changes.
          const aChanged =
               !approxEq(form.a.x, st.initial.a.x)
            || !approxEq(form.a.y, st.initial.a.y)
            || !approxEq(form.a.z, st.initial.a.z);
          const bChanged =
               !approxEq(form.b.x, st.initial.b.x)
            || !approxEq(form.b.y, st.initial.b.y)
            || !approxEq(form.b.z, st.initial.b.z);
          const lenChanged = !approxEq(form.len, st.initial.len);

          console.log('[DimEditor] parsed values', {
            edgeId:    st.edgeId,
            aPointId:  st.aPointId,
            bPointId:  st.bPointId,
            lengthMm:  form.len,
            mode:      form.mode,
            aCoords:   form.a,
            bCoords:   form.b,
            initial:   st.initial,
            aChanged, bChanged, lenChanged,
          });

          // Coord changes take priority over length.
          if (aChanged || bChanged) {
            if (aChanged) {
              const r = await window.__setPointCoordsMm(
                st.aPointId, form.a.x, form.a.y, form.a.z);
              console.log('[DimEditor] setPointCoordsMm(A) →', r);
              if (!r || !r.ok) {
                console.warn('[DimEditor] apply failed (A)', r);
                setError(el, 'A: ' + ((r && r.error) || 'move failed'));
                return;
              }
            }
            if (bChanged) {
              const r = await window.__setPointCoordsMm(
                st.bPointId, form.b.x, form.b.y, form.b.z);
              console.log('[DimEditor] setPointCoordsMm(B) →', r);
              if (!r || !r.ok) {
                console.warn('[DimEditor] apply failed (B)', r);
                setError(el, 'B: ' + ((r && r.error) || 'move failed'));
                return;
              }
            }
            window.__setStatusMessage?.(
              (el.__direction && el.__direction !== 'free'
                ? ('Edge aligned ' + el.__direction + ' · ')
                : '')
              + 'Point coords updated');
            closeEditor();
            return;
          }

          if (lenChanged) {
            const r = await window.__setEdgeLengthMm(
              st.edgeId, form.len, { mode: form.mode });
            console.log('[DimEditor] setEdgeLengthMm →', r);
            if (!r || !r.ok) {
              console.warn('[DimEditor] apply failed (length)', r);
              setError(el, (r && r.error) || 'length update failed');
              return;
            }
            window.__setStatusMessage?.(
              'Edge length → ' + window.__formatCadNumberMm(form.len) + ' mm');
            closeEditor();
            return;
          }

          // Nothing changed.
          console.log('[DimEditor] no changes — closing');
          window.__setStatusMessage?.('No changes');
          closeEditor();
        }

        // ── Open ─────────────────────────────────────────────────────────
        window.__openDimensionEditor = function(hit, px, py) {
          if (!hit || hit.kind !== 'edge_length_dimension' || !hit.edgeId) {
            window.__setStatusMessage?.('Dimension type not editable: '
              + (hit && hit.kind));
            return;
          }
          const el = getPopup();

          // Resolve current numeric mm coords for both endpoints.
          let aMm = null, bMm = null;
          try {
            if (typeof window.__pointMmById === 'function') {
              aMm = window.__pointMmById(hit.aPointId);
              bMm = window.__pointMmById(hit.bPointId);
            }
          } catch (_) { /* noop */ }

          // Prefer freshly computed length from the engine; fall back to hit.valueMm.
          let lenMm = (typeof hit.valueMm === 'number' && isFinite(hit.valueMm))
            ? hit.valueMm : 0;
          try {
            if (typeof window.__edgeMmById === 'function') {
              const em = window.__edgeMmById(hit.edgeId);
              if (em && isFinite(em.lengthMm)) lenMm = em.lengthMm;
            }
          } catch (_) { /* noop */ }

          if (!aMm) aMm = { x: 0, y: 0, z: 0 };
          if (!bMm) bMm = { x: 0, y: 0, z: 0 };

          // Cache initial values for change detection.
          el.__state = {
            edgeId:    hit.edgeId,
            aPointId:  hit.aPointId,
            bPointId:  hit.bPointId,
            initial: {
              len: lenMm,
              a:   { x: aMm.x, y: aMm.y, z: aMm.z },
              b:   { x: bMm.x, y: bMm.y, z: bMm.z },
            },
          };

          const fmt = window.__formatCadNumberMm || ((n) => String(n));
          getInput(el, 'len').value = fmt(lenMm, 2);
          getInput(el, 'ax').value  = fmt(aMm.x, 2);
          getInput(el, 'ay').value  = fmt(aMm.y, 2);
          getInput(el, 'az').value  = fmt(aMm.z, 2);
          getInput(el, 'bx').value  = fmt(bMm.x, 2);
          getInput(el, 'by').value  = fmt(bMm.y, 2);
          getInput(el, 'bz').value  = fmt(bMm.z, 2);
          // Default mode.
          const radio = el.querySelector('input[name="dim-mode"][value="fixA_moveB"]');
          if (radio) radio.checked = true;
          // Default direction.
          el.__direction = 'free';
          updateAxisButtons(el, 'free');
          setError(el, '');

          // Position near click, keep within viewport.
          el.style.display = 'block';
          const vw = window.innerWidth, vh = window.innerHeight;
          const w = el.offsetWidth  || 280;
          const h = el.offsetHeight || 260;
          let left = px + 12, top = py - 12;
          if (left + w > vw - 8) left = px - w - 12;
          if (top  + h > vh - 8) top  = vh - h - 8;
          if (left < 8) left = 8;
          if (top  < 8) top  = 8;
          el.style.left = left + 'px';
          el.style.top  = top  + 'px';

          // Focus length input by default.
          const lenInput = getInput(el, 'len');
          lenInput.focus();
          lenInput.select();

          // Wire keyboard once per popup lifetime.
          if (!el.__keyboardWired) {
            el.__keyboardWired = true;
            el.addEventListener('keydown', (e) => {
              if (e.key === 'Enter') {
                e.preventDefault(); e.stopPropagation();
                applyEdit(el);
              } else if (e.key === 'Escape') {
                e.preventDefault(); e.stopPropagation();
                closeEditor();
              }
            });
            const applyBtn  = $(el, '.dim-apply');
            const cancelBtn = $(el, '.dim-cancel');
            console.log('[DimEditor] wiring buttons', {
              applyBtn:  !!applyBtn,
              cancelBtn: !!cancelBtn,
            });
            applyBtn.addEventListener('click', (e) => {
              console.log('[DimEditor] apply button click');
              e.preventDefault(); e.stopPropagation();
              applyEdit(el);
            });
            cancelBtn.addEventListener('click', (e) => {
              e.preventDefault(); e.stopPropagation();
              closeEditor();
            });
            // Wire Align buttons (Free / X / Y / Z).
            el.querySelectorAll('.dim-axis-btn').forEach((btn) => {
              btn.addEventListener('click', (e) => {
                e.preventDefault(); e.stopPropagation();
                const axis = btn.dataset.axis || 'free';
                if (applyAxisPreview(el, axis)) updateAxisButtons(el, axis);
              });
            });
          }
        };

        // Legacy entry point (back-compat with any callers that may still
        // dispatch __applyDimensionEdit directly).
        window.__applyDimensionEdit = async function(hit, newValueMm) {
          if (!hit || hit.kind !== 'edge_length_dimension' || !hit.edgeId) return;
          const r = await window.__setEdgeLengthMm(hit.edgeId, newValueMm);
          if (!r || !r.ok) {
            window.__setStatusMessage?.('Edit failed: ' + ((r && r.error) || '?'));
          }
        };

        // Close on outside pointerdown (events inside the popup are
        // stopPropagation()'d above and never reach this handler).
        document.addEventListener('pointerdown', (e) => {
          const el = document.getElementById('__dim-editor');
          if (el && el.style.display !== 'none' && !el.contains(e.target)) {
            closeEditor();
          }
        }, true);

      })();
"##;
