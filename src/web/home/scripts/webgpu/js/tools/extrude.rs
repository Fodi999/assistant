// ── Edge Extrude — Blender-style wall surface generator ──────────────────────
// Behaviour:
//   E           → enter extrude mode (validates selection first)
//   type digits → build height in mm (e.g. "2500")
//   Enter       → commit: create top points + wall edges + store wallSurface
//   Esc         → cancel extrude
//
// Selection:
//   - uses selectedEdgeIds (one or many)
//   - falls back to selectedProfileId edges
//   - requires at least 1 valid edge
//
// Direction (per active plane):
//   XZ → Y  (height)
//   XY → Z  (depth)
//   YZ → X  (width)
//
// Data model: sketchState.wallSurfaces[{id,sourceEdgeId,bottomA,bottomB,topA,topB,height}]
// Units: UI mm → world metres (÷ 1000)

pub const JS: &str = r##"
      // ── Capture-phase keyboard guard: intercept keys before any popup ──
      // Registered only once (same startWebGpuScene multi-call guard pattern).
      if (!window.__extrudeKeyInited) {
        window.__extrudeKeyInited = true;
        document.addEventListener('keydown', function(e) {
          if (!window.sketchState?.extrude?.active) return;
          // If focus is inside the modal input, let the input handle it —
          // do NOT stopImmediatePropagation or the input's own keydown fires.
          const inp = document.getElementById('__extrude-modal-input');
          if (inp && document.activeElement === inp) return;
          // Otherwise intercept (e.g. Esc/Enter while canvas has focus)
          if (window.__handleExtrudeKey && window.__handleExtrudeKey(e)) {
            e.stopImmediatePropagation();
          }
        }, true /* capture */);
      }

      // ── Extrude height modal — единый стиль через window.__modalTheme ──
      function __extrudeModalEl() {
        let el = document.getElementById('__extrude-modal');
        if (el) return el;

        const T = window.__modalTheme;
        const C = T.COLORS;
        const L = T.LAYOUT;

        el = document.createElement('div');
        el.id = '__extrude-modal';

        // Базовые стили из темы (те же что у DimEditor / ProfilePopup)
        T.applyPopupStyle(el, { zIndex: '9999', minWidth: '240px', maxWidth: '300px' });
        // Начальная позиция: по центру внизу
        Object.assign(el.style, {
          display:   'none',
          left:      '50%',
          bottom:    '72px',
          top:       'auto',
          transform: 'translateX(-50%)',
          position:  'fixed',
        });

        T.injectBaseCSS();

        el.innerHTML = `
          <div id="__extrude-modal-grip"
            style="
              display:flex; align-items:center; justify-content:space-between;
              margin-bottom:8px; cursor:grab; user-select:none;
              padding:2px 0 6px; border-bottom:1px solid ${C.border};
            ">
            <span style="font-size:10px;font-weight:600;letter-spacing:.6px;
              text-transform:uppercase;color:${C.mute};">
              ⬆ Extrude — высота (мм)
            </span>
            <span style="font-size:14px;color:${C.dim};line-height:1;" title="Перетащить">⠿</span>
          </div>

          <input id="__extrude-modal-input"
            type="number" min="1" step="1" placeholder="2500"
            style="
              display:block; width:100%; box-sizing:border-box;
              text-align:center; font-size:26px; font-weight:700;
              font-family:${L.font}; color:${C.input};
              background:${C.panel}; border:1px solid ${C.border};
              border-radius:${L.borderRadius}; padding:6px 8px;
              outline:none; -moz-appearance:textfield;
            "
          />

          <div class="cad-popup-sep" style="margin:10px 0 8px;"></div>

          <div style="display:flex;gap:6px;">
            <button id="__extrude-cancel-btn" class="cad-popup-btn" style="flex:1;">Esc · Отмена</button>
            <button id="__extrude-apply-btn"  class="cad-popup-btn cad-popup-btn-accent" style="flex:1;">Enter · ОК</button>
          </div>
        `;

        document.body.appendChild(el);

        // ── Drag logic ───────────────────────────────────────────
        const grip = el.querySelector('#__extrude-modal-grip');
        let dragActive = false, ox = 0, oy = 0;

        grip.addEventListener('pointerdown', function(ev) {
          if (ev.button !== 0) return;
          ev.preventDefault();
          ev.stopPropagation();
          dragActive = true;
          // Switch from bottom-anchor to top-anchor so translateX still works
          const rect = el.getBoundingClientRect();
          el.style.top       = rect.top + 'px';
          el.style.bottom    = 'auto';
          el.style.left      = rect.left + 'px';
          el.style.transform = 'none';
          ox = ev.clientX - rect.left;
          oy = ev.clientY - rect.top;
          grip.style.cursor = 'grabbing';
          el.setPointerCapture(ev.pointerId);
        });

        el.addEventListener('pointermove', function(ev) {
          if (!dragActive) return;
          ev.stopPropagation();
          el.style.left = (ev.clientX - ox) + 'px';
          el.style.top  = (ev.clientY - oy) + 'px';
        });

        el.addEventListener('pointerup', function(ev) {
          if (!dragActive) return;
          dragActive = false;
          grip.style.cursor = 'grab';
        });

        // ── Key / button wiring ──────────────────────────────────
        const inp = el.querySelector('#__extrude-modal-input');
        inp.addEventListener('keydown', function(ev) {
          ev.stopPropagation();
          if (ev.key === 'Enter') {
            ev.preventDefault();
            sketchState.extrude.heightInput = inp.value;
            if (window.__commitEdgeExtrude) window.__commitEdgeExtrude();
          } else if (ev.key === 'Escape') {
            ev.preventDefault();
            if (window.__cancelEdgeExtrude) window.__cancelEdgeExtrude();
          }
        });
        inp.addEventListener('input', function() {
          sketchState.extrude.heightInput = inp.value;
        });
        el.querySelector('#__extrude-apply-btn').addEventListener('click', function(ev) {
          ev.stopPropagation();
          sketchState.extrude.heightInput = inp.value;
          if (window.__commitEdgeExtrude) window.__commitEdgeExtrude();
        });
        el.querySelector('#__extrude-cancel-btn').addEventListener('click', function(ev) {
          ev.stopPropagation();
          if (window.__cancelEdgeExtrude) window.__cancelEdgeExtrude();
        });

        return el;
      }

      window.__extrudeModalShow = function(heightInput) {
        const el = __extrudeModalEl();
        el.style.display = 'block';
        const inp = el.querySelector('#__extrude-modal-input');
        if (inp) {
          if (heightInput !== undefined && heightInput !== '') inp.value = heightInput;
          setTimeout(() => { inp.focus(); inp.select(); }, 30);
        }
      };

      window.__extrudeModalHide = function() {
        const el = document.getElementById('__extrude-modal');
        if (el) el.style.display = 'none';
      };

      // ── Extrude direction per working plane ──────────────────
      window.__getExtrudeDir = function(plane) {
        if (plane === 'XY') return { x: 0, y: 0, z: 1 };
        if (plane === 'YZ') return { x: 1, y: 0, z: 0 };
        return { x: 0, y: 1, z: 0 };  // XZ default
      };

      // ── Collect edges to extrude ─────────────────────────────
      window.__collectExtrudeEdges = function() {
        const edges = [];
        const pById = new Map(sketchState.points.map(p => [p.id, p]));

        // 1. selected edges
        if (sketchState.selectedEdgeIds.size > 0) {
          for (const eid of sketchState.selectedEdgeIds) {
            const e = sketchState.edges.find(x => x.id === eid);
            if (e && pById.get(e.a) && pById.get(e.b)) edges.push(e);
          }
        }
        // 2. fallback: selected profile edges
        if (!edges.length && sketchState.selectedProfileId) {
          const prof = window.__getProfileById
            ? window.__getProfileById(sketchState.selectedProfileId)
            : null;
          if (prof && prof.edgeIds) {
            for (const eid of prof.edgeIds) {
              const e = sketchState.edges.find(x => x.id === eid);
              if (e && pById.get(e.a) && pById.get(e.b)) edges.push(e);
            }
          }
        }
        return edges;
      };

      // ── Start extrude mode ───────────────────────────────────
      window.__startEdgeExtrude = function() {
        console.log('[Extrude.__start] вызван', {
          grabActive:   sketchState.grab?.active,
          copyActive:   sketchState.copy?.active,
          activeTool:   sketchState.activeTool,
          lineStarted:  sketchState.line?.startPointId,
          selectedEdgeIds: [...(sketchState.selectedEdgeIds || [])],
          selectedProfileId: sketchState.selectedProfileId,
          totalEdges:   sketchState.edges?.length,
          totalPoints:  sketchState.points?.length,
        });
        if (sketchState.grab?.active || sketchState.copy?.active) {
          console.warn('[Extrude.__start] abort: grab/copy active');
          window.__setStatusMessage('Extrude: завершите текущую операцию сначала');
          return;
        }
        if (sketchState.activeTool === 'line' && sketchState.line?.startPointId) {
          console.warn('[Extrude.__start] abort: line in progress');
          window.__setStatusMessage('Extrude: завершите линию сначала');
          return;
        }
        // Close DimEditor popup if open so it doesn't steal focus/keys
        const dimEl = document.getElementById('__dim-editor');
        if (dimEl) { dimEl.style.display = 'none'; dimEl.__state = null; }
        const edges = window.__collectExtrudeEdges();
        console.log('[Extrude.__start] collectExtrudeEdges →', edges.length, edges.map(e => e.id));
        if (!edges.length) {
          console.warn('[Extrude.__start] abort: no edges collected');
          window.__setStatusMessage('Extrude: выберите линии или профиль');
          return;
        }
        sketchState.extrude.active      = true;
        sketchState.extrude.heightInput = '';
        sketchState.extrude.edgeIds     = edges.map(e => e.id);
        console.log('[Extrude.__start] ✓ active, edgeIds=', sketchState.extrude.edgeIds);
        window.__extrudeModalShow('');
        window.__setStatusMessage(
          'Extrude · ' + edges.length + ' рёбер · введите высоту мм · Enter ✓ · Esc ✗'
        );
        if (window.__updateSketchInspector) window.__updateSketchInspector();
      };

      // ── Cancel extrude mode ──────────────────────────────────
      window.__cancelEdgeExtrude = function() {
        sketchState.extrude.active      = false;
        sketchState.extrude.heightInput = '';
        sketchState.extrude.edgeIds     = [];
        window.__extrudeModalHide();
        window.__setStatusMessage('Extrude отменён');
        if (window.__updateSketchInspector) window.__updateSketchInspector();
      };

      // ── Commit extrude ───────────────────────────────────────
      window.__commitEdgeExtrude = async function() {
        const ex = sketchState.extrude;
        if (!ex.active) return;

        // Always read from the actual <input> element as the source of truth
        const inp = document.getElementById('__extrude-modal-input');
        if (inp && inp.value) ex.heightInput = inp.value;

        const heightMm = parseFloat(ex.heightInput);
        console.log('[Extrude.commit] heightInput=', ex.heightInput, 'heightMm=', heightMm, 'edgeIds=', ex.edgeIds);
        if (!isFinite(heightMm) || heightMm === 0) {
          window.__setStatusMessage('Extrude: введите высоту в мм, например 2500');
          if (inp) inp.focus();
          return;
        }

        const heightM = heightMm / 1000.0;  // mm → metres
        const plane   = sketchState.workingPlane || 'XZ';
        const dir     = window.__getExtrudeDir(plane);
        const pById   = new Map(sketchState.points.map(p => [p.id, p]));
        const gs      = sketchState.gridSize || 0.001;

        const edges = ex.edgeIds
          .map(id => sketchState.edges.find(e => e.id === id))
          .filter(Boolean);

        if (!edges.length) {
          window.__cancelEdgeExtrude();
          return;
        }

        window.__pushHistory();

        // Cache: bottom point id → top point id (shared corners)
        const topPointMap = new Map();

        async function getOrCreateTopPoint(bottomId) {
          if (topPointMap.has(bottomId)) return topPointMap.get(bottomId);
          const bp = pById.get(bottomId);
          if (!bp) return null;
          const tx = bp.x + dir.x * heightM;
          const ty = bp.y + dir.y * heightM;
          const tz = bp.z + dir.z * heightM;
          // Convert world → grid coords
          const tgx = Math.round(tx / gs);
          const tgy = Math.round(ty / gs);
          const tgz = Math.round(tz / gs);
          // Check if top point already exists
          let existing = sketchState.points.find(
            p => p.gx === tgx && p.gy === tgy && p.gz === tgz
          );
          let topId;
          if (existing) {
            topId = existing.id;
          } else {
            topId = await window.__createPointViaEngine(tgx, tgy, tgz);
          }
          topPointMap.set(bottomId, topId);
          return topId;
        }

        const createdWalls = [];

        for (const edge of edges) {
          const bA = pById.get(edge.a);
          const bB = pById.get(edge.b);
          if (!bA || !bB) continue;

          const topAId = await getOrCreateTopPoint(edge.a);
          const topBId = await getOrCreateTopPoint(edge.b);
          if (!topAId || !topBId) continue;

          // Top edge A–B
          const existTopEdge = sketchState.edges.find(
            e => (e.a === topAId && e.b === topBId) || (e.a === topBId && e.b === topAId)
          );
          if (!existTopEdge) {
            await window.__createEdgeViaEngine(topAId, topBId, 'normal');
          }

          // Vertical edge A
          const existVertA = sketchState.edges.find(
            e => (e.a === edge.a && e.b === topAId) || (e.a === topAId && e.b === edge.a)
          );
          if (!existVertA) {
            await window.__createEdgeViaEngine(edge.a, topAId, 'normal');
          }

          // Vertical edge B
          const existVertB = sketchState.edges.find(
            e => (e.a === edge.b && e.b === topBId) || (e.a === topBId && e.b === edge.b)
          );
          if (!existVertB) {
            await window.__createEdgeViaEngine(edge.b, topBId, 'normal');
          }

          // Refresh pById after point creation
          const pByIdFresh = new Map(sketchState.points.map(p => [p.id, p]));
          const tA = pByIdFresh.get(topAId);
          const tB = pByIdFresh.get(topBId);
          if (!tA || !tB) continue;

          // Wall surface record
          const wallId = 'wall_' + Date.now() + '_' + Math.random().toString(36).slice(2, 7);
          createdWalls.push({
            id:           wallId,
            sourceEdgeId: edge.id,
            bottomA:      { x: bA.x, y: bA.y, z: bA.z },
            bottomB:      { x: bB.x, y: bB.y, z: bB.z },
            topA:         { x: tA.x, y: tA.y, z: tA.z },
            topB:         { x: tB.x, y: tB.y, z: tB.z },
            height:       heightM,
            plane:        plane,
            topAId:       topAId,
            topBId:       topBId,
          });
        }

        // Append wall surfaces
        sketchState.wallSurfaces.push(...createdWalls);

        // Reset extrude state
        ex.active      = false;
        ex.heightInput = '';
        ex.edgeIds     = [];

        window.__extrudeModalHide();
        if (window.__notifySketchChanged)   window.__notifySketchChanged();
        if (window.__updateSketchInspector) window.__updateSketchInspector();

        const heightDisplay = Math.abs(heightMm).toFixed(0);
        window.__setStatusMessage(
          '✓ Extrude ' + heightDisplay + ' мм · ' + createdWalls.length + ' стен создано'
        );
        console.log('[Extrude] committed', createdWalls.length, 'walls, h=', heightM, 'm');
      };

      // ── Extrude key handler — fallback when focus is NOT in the modal input ─
      // The modal <input> handles digits+Enter+Esc itself.
      // This fallback covers Esc/Enter from anywhere else (e.g. canvas focus).
      window.__handleExtrudeKey = function(e) {
        if (!sketchState.extrude.active) return false;
        const k = e.key.toLowerCase();

        if (k === 'escape') {
          window.__cancelEdgeExtrude();
          e.preventDefault();
          return true;
        }
        if (k === 'enter') {
          const inp = document.getElementById('__extrude-modal-input');
          if (inp) sketchState.extrude.heightInput = inp.value;
          window.__commitEdgeExtrude();
          e.preventDefault();
          return true;
        }
        // For digit keys — forward focus to input so the user can keep typing
        if (/^[0-9\.\-]$/.test(e.key)) {
          const inp = document.getElementById('__extrude-modal-input');
          if (inp) { inp.focus(); }
          e.preventDefault();
          return true;
        }
        return true; // consume all other keys while extrude active
      };
"##;

