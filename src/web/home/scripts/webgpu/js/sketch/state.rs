// ── JS: Sketch data model — wireframe core + constraints + profiles ─────────
// Domain: Sketch — id-based SketchGraph + helpers + history + validation.

pub const JS: &str = r##"
      // ── Selection mode constants ───────────────────────────────
      window.SelectionMode = Object.freeze({
        SELECT: "select", POINT: "point", LINE: "line",
        GRAB:   "grab",   DELETE: "delete",
      });

      // ── Sketch State ───────────────────────────────────────────
      const sketchState = {
        points: [], edges: [],
        constraints: [],
        profiles: [],
        validation: { isolatedIds: [], openEndIds: [] },
        showValidation: true,
        statusMessage: null,

        selectedPointIds: new Set(),
        selectedEdgeIds:  new Set(),

        hoverPointId: null,
        hoverEdgeId:  null,
        hoverWorld:   null,
        hoverGridFree: null,

        activeTool: "select",
        phase:      "idle",

        // ── Line tool state ──
        line: {
          startPointId: null,
          previewPoint: null,
          previewLength: 0,
          previewValid: true,
        },

        // ── Ortho / Angle Lock (0° 45° 90° …) ──
        orthoLock: false,   // toggled by O key or ORTHO button

        // ── Precision cursor mode (Alt held) ──
        precisionMode: false,

        // ── Cursor display settings ──
        cursorSettings: {
          showCoords:    true,
          showSnapMarker: true,
          showLength:    true,
          autoFlipLabel: true,
        },

        // ── Snap status ──
        snap: { kind: "grid", pointId: null, gx: 0, gy: 0, gz: 0 },

        // ── Grid / plane ──
        // gridSize == precision.internalStepM (0.01 mm). It is the *engine*
        // grid step — what the CAD core (WASM + backend) uses to convert
        // (gx,gy,gz) ↔ world meters. Do not edit directly; use
        // sketchState.precision.{internalStepM, snapStepM, displayGridStepM}.
        gridSize: 0.00001,
        workingPlane: "XZ",
        showGrid: true,
        plane: "XZ",

        // ── Grab snapshot ──
        grab: {
          active: false, pointIds: [],
          startMouseWorld: null,
          originalPoints: new Map(),
          axisLock: null,
        },

        // ── Gizmo drag state (set active only when pointer is down on a handle) ──
        gizmoDrag: { active: false, axis: null, pointerId: null },

        // ── Undo/redo ──
        _history: { undo: [], redo: [] },
        _historyLimit: 100,

        // ── Backend precision commands (Phase 7) ──
        useBackendCommands: true,
        backendStatus: { ok: null, message: null, lastValidation: null },

        // ── Engine Mode (Phase 11) ── 'backend' | 'wasm' | 'hybrid'
        engineMode: 'backend',
        lastBackendMs: 0,
        lastWasmMs: 0,
        lastSyncStatus: '—',   // 'ok' | 'diff' | 'pending' | 'err' | '—'
        lastCommandMsg: '—',   // last engine result string (shown as "Last result")

        // ── Profile selection (Phase 8) ──
        selectedProfileId: null,
        hoverProfileId: null,

        // ── Wall Surfaces (Edge Extrude) ──
        wallSurfaces: [],

        // ── Extrude tool state ──
        extrude: {
          active:      false,
          heightInput: '',
          edgeIds:     [],
        },

        // ── Precision / Snap (Phase 12 + Phase 15: split precision) ──
        coordPrecision: 3,
        precision: {
          // ── Three-tier precision model ──
          // internalStepM  = engine resolution (0.01 mm). Drives gx/gy/gz.
          // snapStepM      = visible snap step in world meters (1 mm default).
          // displayGridStepM = rendered grid line spacing in world meters.
          internalStepM:    0.00001,  // 0.01 mm
          snapStepM:        0.001,    // 1 mm
          displayGridStepM: 0.001,    // 1 mm

          touchpadMode: true,
          snapEnabled: true,
          gridSnap:    true,
          pointSnap:   true,
          midpointSnap: false,
          freeMode:    false,

          pointSnapRadiusPx: 14,
          edgeSnapRadiusPx:  8,
          snapLockMs:        180,

          // Last resolved samples (set every pointermove).
          lastFreeWorld:    null,
          lastSnappedWorld: null,
          lastGrid:         null,
          lastMouseScreen:  null,

          // Modifier state (true while Shift is held).
          shiftHeld: false,

          // Touchpad anti-jitter lock.
          snapLock: {
            active: false,
            key: null,        // pointId or "g:gx,gy,gz"
            kind: null,       // 'point' | 'grid'
            expiresAt: 0,
            screenX: 0,
            screenY: 0,
          },
        },

        // ── Projection Drafting (Phase 13) ──
        // draftMode 'free3d' → ordinary 3D sketch.
        // draftMode 'projection' → engineering-style projection drafting
        // (top / front / side views on the three working planes).
        draftMode: 'free3d',
        projection: {
          // Default box dimensions for "Create projection box".
          boxWidth:  6,   // X
          boxHeight: 5,   // Y
          boxDepth:  4,   // Z
          // Show dashed projection guide-lines through hovered / selected points.
          showGuides: false,  // controlled by window.__showProjectionGuide (VIEW panel)
        },

        // ── Copy Connect (Phase 14) ──
        // Plasticity-style "pull copy": duplicate selected geometry, move with
        // cursor, then auto-connect originals → copies with bridge edges.
        copy: {
          active:          false,
          source:          null,         // 'profile' | 'edges' | 'points'
          pointIds:        [],           // original points being duplicated
          edges:           [],           // [[aId, bId, kind], …] inner edges to clone
          originals:       new Map(),    // id → { x, y, z }
          startScreen:     null,         // canvas px at the moment Shift+G was pressed
          delta:           { dx: 0, dy: 0, dz: 0 }, // grid-snapped offset
          axisLock:        null,         // 'X' | 'Y' | 'Z' | null
        },

        // ── Drafting Overlay (Phase 16) ─────────────────────────
        // Pure visual layer drawn on top of geometry. Does not alter the
        // sketch — only renders engineering-drawing decorations.
        drafting: {
          showDimensions:    true,   // dim line + arrows + label for selected edges & profile
          showEdgeLengths:   false,  // length label on every edge (or only hovered/selected)
          showPointLabels:   false,  // coord labels on hovered/selected points
          showGridNumbers:   false,  // ruler-style numbers along viewport edges
          showCenterlines:   false,  // dashed centerlines through profile centroid
          unit:              'mm',
          decimals:          1,
          dimensionOffsetPx: 20,
          arrowSizePx:       7,
          textGapPx:         6,
        },
        draftingHitLabels: [], // populated each frame by drafting overlay
      };
      window.sketchState = sketchState;

      // ── 3D Helper Guide flags ──────────────────────────────────
      // Controlled by VIEW panel toggles.
      // __showOrbitGuide     — draw dashed orbit-pivot ring while orbiting
      // __showProjectionGuide — draw dashed guide lines in projection mode
      // __fadeBackgroundHelpers — draw background helpers at reduced opacity
      // __orbitActive        — true while the user is actively dragging to orbit
      window.__showOrbitGuide       = false;
      window.__showProjectionGuide  = false;
      window.__fadeBackgroundHelpers = true;
      window.__orbitActive          = false;

      // ── Id generation ──────────────────────────────────────────
      let __pointCounter = 0, __edgeCounter = 0, __constraintCounter = 0, __profileCounter = 0;
      window.__nextPointId      = () => "p_" + (++__pointCounter);
      window.__nextEdgeId       = () => "e_" + (++__edgeCounter);
      window.__nextConstraintId = () => "c_" + (++__constraintCounter);

      // ── Status message (auto-clearing) ─────────────────────────
      let __statusTimer = null;
      window.__setStatusMessage = function(msg, ttl) {
        sketchState.statusMessage = msg || null;
        if (__statusTimer) clearTimeout(__statusTimer);
        if (msg) {
          __statusTimer = setTimeout(() => { sketchState.statusMessage = null; }, ttl || 2500);
          if (typeof log === 'function') log(msg, '#fbbf24');
        }
      };

      // ── Coordinate helpers ─────────────────────────────────────
      // Internal step = engine grid resolution (0.01 mm). Snap step = visible
      // user-facing snap quantum (e.g. 1 mm). Snap rounds in world space first,
      // then the snapped world value is converted to internal grid coords.
      function __internalStep() {
        const pr = sketchState.precision;
        const s = (pr && pr.internalStepM) || sketchState.gridSize || 1.0;
        return (isFinite(s) && s > 0) ? s : 1.0;
      }
      function __snapStep() {
        const pr = sketchState.precision;
        const s = pr && pr.snapStepM;
        return (isFinite(s) && s > 0) ? s : __internalStep();
      }
      window.__gridToWorld = function(gx, gy, gz) {
        const g = __internalStep();
        return { x: gx * g, y: gy * g, z: gz * g };
      };
      window.__worldToGrid = function(x, y, z) {
        const g = __internalStep();
        return { gx: Math.round(x / g), gy: Math.round(y / g), gz: Math.round(z / g) };
      };
      window.__snapWorldToGrid = function(world, plane) {
        const internal = __internalStep();
        const snap     = __snapStep();
        const pl = plane || sketchState.workingPlane || "XZ";
        // 1. Round world coords to the visible snap step.
        const sx = Math.round(world.x / snap) * snap;
        const sy = Math.round(world.y / snap) * snap;
        const sz = Math.round(world.z / snap) * snap;
        // 2. Convert to internal grid coords (always integers at 0.01 mm).
        let gx = Math.round(sx / internal);
        let gy = Math.round(sy / internal);
        let gz = Math.round(sz / internal);
        if (pl === "XZ") gy = 0;
        if (pl === "XY") gz = 0;
        if (pl === "YZ") gx = 0;
        return { gx, gy, gz, x: gx * internal, y: gy * internal, z: gz * internal };
      };

      // ── Precision helpers (Phase 12) ───────────────────────────
      // Formats a coordinate value using sketchState.coordPrecision.
      window.__fmtCoord = function(v) {
        const n = Number(v);
        if (!isFinite(n)) return '—';
        const p = sketchState.coordPrecision || 3;
        // Trim trailing zeros (1.500 → 1.5, 1.000 → 1) when integer.
        return n.toFixed(p);
      };
      window.__fmtLength = function(v) {
        // v is in world meters (1 world unit = 1 m = 1000 mm).
        const n = Number(v);
        if (!isFinite(n)) return '—';
        const mm = n * 1000.0;
        if (mm >= 1000.0) return (mm / 1000.0).toFixed(3) + ' m';
        const dp = (sketchState.drafting && typeof sketchState.drafting.decimals === 'number')
                   ? sketchState.drafting.decimals : 1;
        return mm.toFixed(dp) + ' mm';
      };
      // __fmtArea(worldArea) → "1000.0 mm²" | "1.000 m²"
      window.__fmtArea = function(v) {
        const n = Number(v);
        if (!isFinite(n)) return '—';
        const mm2 = n * 1e6;
        if (mm2 >= 1e6) return (mm2 / 1e6).toFixed(3) + ' m\u00b2';
        return mm2.toFixed(1) + ' mm\u00b2';
      };

      // ── Drafting helpers (Phase 16) ────────────────────────────
      // True engineering measurements are derived from integer gx/gy/gz so
      // they are exact and unit-stable (1 internal step = 0.01 mm).
      // __edgeLengthMm(a, b) → length in millimetres.
      window.__edgeLengthMm = function(a, b) {
        if (!a || !b) return 0;
        const internalMm = ((sketchState.precision && sketchState.precision.internalStepM) || 0.00001) * 1000;
        const dx = (b.gx - a.gx);
        const dy = (b.gy - a.gy);
        const dz = (b.gz - a.gz);
        return Math.hypot(dx, dy, dz) * internalMm;
      };
      // __formatDim(valueMm) → "10,0" (locale comma, configurable decimals).
      window.__formatDim = function(valueMm) {
        const d  = (sketchState.drafting && sketchState.drafting.decimals);
        const dp = (typeof d === 'number' && d >= 0) ? d : 1;
        const n  = Number(valueMm);
        if (!isFinite(n)) return '—';
        return n.toFixed(dp).replace('.', ',');
      };
      // __pointCoordsMm(p) → { x:"10,0", y:"0,0", z:"-30,0" }.
      window.__pointCoordsMm = function(p) {
        const internalMm = ((sketchState.precision && sketchState.precision.internalStepM) || 0.00001) * 1000;
        const fmt = window.__formatDim;
        return {
          x: fmt(p.gx * internalMm),
          y: fmt(p.gy * internalMm),
          z: fmt(p.gz * internalMm),
        };
      };

      // __hitDraftingLabel(pxCss, pyCss) → hit descriptor | null.
      //
      // The drafting overlay (#sketch-canvas) has its backing-buffer sized to
      // canvas.width/height which equals the WebGPU canvas backing buffer
      // (DPR-scaled = device pixels). The hit rects in
      // sketchState.draftingHitLabels are therefore stored in *device pixels*.
      // Pointer events arrive in CSS pixels (clientX/Y - rect.left/top).
      // We convert CSS → device by multiplying with (backingW / cssRectW).
      window.__hitDraftingLabel = function(pxCss, pyCss) {
        const labels = sketchState.draftingHitLabels;
        if (!labels || !labels.length) return null;
        const sk = document.getElementById('sketch-canvas');
        let sx = 1, sy = 1;
        if (sk) {
          const r = sk.getBoundingClientRect();
          if (r.width  > 0) sx = sk.width  / r.width;
          if (r.height > 0) sy = sk.height / r.height;
        }
        const px = pxCss * sx;
        const py = pyCss * sy;
        for (const lbl of labels) {
          const r = lbl.rect;
          if (px >= r.x && px <= r.x + r.w && py >= r.y && py <= r.y + r.h) {
            return lbl;
          }
        }
        return null;
      };

      // ── CAD mm/internal conversion helpers ─────────────────────
      // 1 internal step = 0.01 mm (internalStepM = 0.00001 m).
      window.__cadInternalStepMm = function() {
        return ((sketchState.precision && sketchState.precision.internalStepM) || 0.00001) * 1000;
      };
      window.__parseCadNumber = function(v) {
        if (typeof v !== 'string') return Number(v);
        return Number(v.trim().replace(',', '.'));
      };
      window.__formatCadNumberMm = function(valueMm, decimals) {
        const dp = (typeof decimals === 'number') ? decimals : 1;
        const n  = Number(valueMm);
        if (!isFinite(n)) return '—';
        return n.toFixed(dp).replace('.', ',');
      };
      window.__gridToMm = function(g) {
        return g * window.__cadInternalStepMm();
      };
      window.__mmToGrid = function(mm) {
        return Math.round(mm / window.__cadInternalStepMm());
      };
      window.__pointToMm = function(p) {
        const s = window.__cadInternalStepMm();
        return { x: p.gx * s, y: p.gy * s, z: p.gz * s };
      };
      // __pointMmById(id) → { x, y, z } numeric mm | null.
      window.__pointMmById = function(id) {
        const p = (sketchState.points || []).find(q => q.id === id);
        return p ? window.__pointToMm(p) : null;
      };
      // __edgeMmById(id) → { a:{x,y,z}, b:{x,y,z}, lengthMm } | null.
      window.__edgeMmById = function(id) {
        const e = (sketchState.edges || []).find(q => q.id === id);
        if (!e) return null;
        const a = window.__pointMmById(e.a);
        const b = window.__pointMmById(e.b);
        if (!a || !b) return null;
        const dx = a.x - b.x, dy = a.y - b.y, dz = a.z - b.z;
        return { a, b, lengthMm: Math.hypot(dx, dy, dz) };
      };

      // __setEdgeLengthMm(edgeId, lengthMm, options)
      //   options.mode: 'fixA_moveB' (default) | 'fixB_moveA' | 'center_moveBoth'
      // Returns { ok: bool, error?: string }.
      window.__setEdgeLengthMm = async function(edgeId, lengthMm, options) {
        const mode = (options && options.mode) || 'fixA_moveB';
        const edge = (sketchState.edges || []).find(e => e.id === edgeId);
        if (!edge) return { ok: false, error: 'Edge not found: ' + edgeId };
        const byId = new Map(sketchState.points.map(p => [p.id, p]));
        const a = byId.get(edge.a);
        const b = byId.get(edge.b);
        if (!a || !b) return { ok: false, error: 'Endpoints not found' };

        const stepMm = window.__cadInternalStepMm();
        const dx = b.gx - a.gx, dy = b.gy - a.gy, dz = b.gz - a.gz;
        const oldLen = Math.hypot(dx, dy, dz);
        if (oldLen <= 0) return { ok: false, error: 'Zero-length edge cannot be resized' };

        const newLenInternal = Math.round(lengthMm / stepMm);
        if (newLenInternal <= 0) return { ok: false, error: 'Length too small' };

        const scale = newLenInternal / oldLen;
        console.log('[setEdgeLengthMm]', {
          edgeId, lengthMm, mode,
          a: { id: a.id, gx: a.gx, gy: a.gy, gz: a.gz },
          b: { id: b.id, gx: b.gx, gy: b.gy, gz: b.gz },
          stepMm, oldLen, newLenInternal, scale,
        });

        if (mode === 'fixA_moveB') {
          const nx = a.gx + Math.round(dx * scale);
          const ny = a.gy + Math.round(dy * scale);
          const nz = a.gz + Math.round(dz * scale);
          console.log('[setEdgeLengthMm] moving B →', { nx, ny, nz });
          const r = await window.__movePointViaEngine(b.id, nx, ny, nz);
          console.log('[setEdgeLengthMm] move B result', r);
          if (!r || !r.ok) return { ok: false, error: (r && r.error) || 'move B failed' };
          return { ok: true };
        }
        if (mode === 'fixB_moveA') {
          const nx = b.gx - Math.round(dx * scale);
          const ny = b.gy - Math.round(dy * scale);
          const nz = b.gz - Math.round(dz * scale);
          console.log('[setEdgeLengthMm] moving A →', { nx, ny, nz });
          const r = await window.__movePointViaEngine(a.id, nx, ny, nz);
          console.log('[setEdgeLengthMm] move A result', r);
          if (!r || !r.ok) return { ok: false, error: (r && r.error) || 'move A failed' };
          return { ok: true };
        }
        if (mode === 'center_moveBoth') {
          const cx = (a.gx + b.gx) / 2;
          const cy = (a.gy + b.gy) / 2;
          const cz = (a.gz + b.gz) / 2;
          const hdx = (dx * scale) / 2;
          const hdy = (dy * scale) / 2;
          const hdz = (dz * scale) / 2;
          const nAx = Math.round(cx - hdx);
          const nAy = Math.round(cy - hdy);
          const nAz = Math.round(cz - hdz);
          const nBx = Math.round(cx + hdx);
          const nBy = Math.round(cy + hdy);
          const nBz = Math.round(cz + hdz);
          console.log('[setEdgeLengthMm] center mode', { nAx, nAy, nAz, nBx, nBy, nBz });
          const ra = await window.__movePointViaEngine(a.id, nAx, nAy, nAz);
          if (!ra || !ra.ok) return { ok: false, error: (ra && ra.error) || 'move A failed' };
          const rb = await window.__movePointViaEngine(b.id, nBx, nBy, nBz);
          if (!rb || !rb.ok) return { ok: false, error: (rb && rb.error) || 'move B failed' };
          return { ok: true };
        }
        return { ok: false, error: 'Unknown fix mode: ' + mode };
      };

      // __setPointCoordsMm(pointId, xMm, yMm, zMm) → { ok, error? }.
      window.__setPointCoordsMm = async function(pointId, xMm, yMm, zMm) {
        const gx = window.__mmToGrid(xMm);
        const gy = window.__mmToGrid(yMm);
        const gz = window.__mmToGrid(zMm);
        console.log('[setPointCoordsMm]', { pointId, xMm, yMm, zMm, gx, gy, gz });
        const r = await window.__movePointViaEngine(pointId, gx, gy, gz);
        console.log('[setPointCoordsMm] result', r);
        if (!r || !r.ok) return { ok: false, error: (r && r.error) || 'move failed' };
        return { ok: true, gx, gy, gz };
      };

      // __setSnapMode(key, on) — toggles one of: gridSnap | pointSnap | midpointSnap | freeMode.
      window.__setSnapMode = function(key, on) {
        const pr = sketchState.precision;
        if (!(key in pr)) return;
        pr[key] = !!on;
        // freeMode implies snapping disabled overall.
        if (key === 'freeMode' && on) {
          pr.snapEnabled = false;
        } else if (key === 'freeMode' && !on) {
          pr.snapEnabled = true;
        }
        if (window.__setStatusMessage) {
          window.__setStatusMessage('Привязка · ' + key + ' = ' + (on ? 'вкл' : 'выкл'));
        }
      };

      // __cycleGridSize(direction) — halve (-1) or double (+1) the visible
      // snap step (precision.snapStepM). Internal engine step is unaffected.
      // Clamps to [0.0001 m, 1 m] (0.1 mm … 1 m).
      window.__cycleGridSize = function(direction) {
        const pr = sketchState.precision;
        const cur = (pr && pr.snapStepM) || 0.001;
        const next = (direction > 0) ? cur * 2 : cur / 2;
        const clamped = Math.max(0.0001, Math.min(1.0, next));
        pr.snapStepM = clamped;
        if (window.__setStatusMessage) {
          window.__setStatusMessage('Шаг привязки: ' + (clamped * 1000).toFixed(3) + ' мм');
        }
        if (window.__notifySketchChanged) window.__notifySketchChanged();
      };

      // ── __resolveSnapTarget(freeWorld, mousePx, options) → snap target ──
      // Priority:
      //   1. existing point   (if pointSnap and within pointSnapRadiusPx)
      //   2. grid             (if gridSnap)
      //   3. free             (if freeMode or shift held)
      //   4. grid fallback
      // Returns {kind, pointId, gx, gy, gz, x, y, z, screenDistancePx, key, valid}.
      window.__resolveSnapTarget = function(freeWorld, mousePx, opts) {
        const pr   = sketchState.precision;
        const plane = sketchState.workingPlane || 'XZ';
        const now  = Date.now();
        const shift = pr.shiftHeld;

        // Persist last free sample.
        pr.lastFreeWorld   = { x: freeWorld.x, y: freeWorld.y, z: freeWorld.z };
        pr.lastMouseScreen = { x: mousePx.x,   y: mousePx.y };

        // Free mode override (explicit or via Shift).
        const wantFree = pr.freeMode || shift;

        // ── Snap lock check (touchpad anti-jitter) ──
        const lock = pr.snapLock;
        if (pr.touchpadMode && lock.active && now < lock.expiresAt && !wantFree) {
          const dx = mousePx.x - lock.screenX;
          const dy = mousePx.y - lock.screenY;
          const moved = Math.hypot(dx, dy);
          if (moved < 8) {
            // Still locked — return last snapshot if target still exists.
            if (lock.kind === 'point') {
              const p = sketchState.points.find(pp => pp.id === lock.key);
              if (p) {
                return {
                  kind: 'point', pointId: p.id,
                  gx: p.gx, gy: p.gy, gz: p.gz,
                  x: p.x, y: p.y, z: p.z,
                  screenDistancePx: moved, key: 'p:' + p.id, valid: true,
                };
              }
            }
          }
          lock.active = false;
        }

        // ── 1. Existing point snap ──
        if (pr.snapEnabled && pr.pointSnap && !wantFree && window.__worldToScreenPx) {
          const radius = pr.pointSnapRadiusPx || 14;
          let best = null, bestD = Infinity;
          for (const p of sketchState.points) {
            const s = window.__worldToScreenPx(p.x, p.y, p.z);
            if (!s) continue;
            const d = Math.hypot(s.x - mousePx.x, s.y - mousePx.y);
            if (d < radius && d < bestD) { bestD = d; best = p; }
          }
          if (best) {
            // Acquire lock.
            if (pr.touchpadMode) {
              lock.active    = true;
              lock.kind      = 'point';
              lock.key       = best.id;
              lock.expiresAt = now + (pr.snapLockMs || 180);
              lock.screenX   = mousePx.x;
              lock.screenY   = mousePx.y;
            }
            pr.lastSnappedWorld = { x: best.x, y: best.y, z: best.z };
            pr.lastGrid = { gx: best.gx, gy: best.gy, gz: best.gz };
            return {
              kind: 'point', pointId: best.id,
              gx: best.gx, gy: best.gy, gz: best.gz,
              x: best.x, y: best.y, z: best.z,
              screenDistancePx: bestD, key: 'p:' + best.id, valid: true,
            };
          }
        }

        // ── 2. Free mode (no snap visually; still emit nearest grid for engine) ──
        if (wantFree) {
          let fx = freeWorld.x, fy = freeWorld.y, fz = freeWorld.z;
          if (plane === 'XZ') fy = 0;
          if (plane === 'XY') fz = 0;
          if (plane === 'YZ') fx = 0;
          // Compute nearest grid coords so engine submission still works.
          const ng = window.__snapWorldToGrid({ x: fx, y: fy, z: fz }, plane);
          pr.lastSnappedWorld = { x: fx, y: fy, z: fz };
          pr.lastGrid = { gx: ng.gx, gy: ng.gy, gz: ng.gz };
          return {
            kind: 'free', pointId: null,
            gx: ng.gx, gy: ng.gy, gz: ng.gz,
            x: fx, y: fy, z: fz,
            screenDistancePx: 0,
            key: 'f:' + fx.toFixed(4) + ',' + fy.toFixed(4) + ',' + fz.toFixed(4),
            valid: true,
          };
        }

        // ── 3. Grid snap (fallback / primary when no point) ──
        const snapped = window.__snapWorldToGrid(freeWorld, plane);
        const gridKey = 'g:' + snapped.gx + ',' + snapped.gy + ',' + snapped.gz;
        if (pr.touchpadMode) {
          lock.active    = true;
          lock.kind      = 'grid';
          lock.key       = gridKey;
          lock.expiresAt = now + (pr.snapLockMs || 180);
          lock.screenX   = mousePx.x;
          lock.screenY   = mousePx.y;
        }
        pr.lastSnappedWorld = { x: snapped.x, y: snapped.y, z: snapped.z };
        pr.lastGrid = { gx: snapped.gx, gy: snapped.gy, gz: snapped.gz };
        return {
          kind: 'grid', pointId: null,
          gx: snapped.gx, gy: snapped.gy, gz: snapped.gz,
          x: snapped.x,   y: snapped.y,   z: snapped.z,
          screenDistancePx: 0, key: gridKey, valid: true,
        };
      };

      window.__findPointAtGrid = function(gx, gy, gz) {
        for (const p of sketchState.points) {
          if (p.gx === gx && p.gy === gy && p.gz === gz) return p;
        }
        return null;
      };
      window.__findEdgeBetween = function(aId, bId) {
        for (const e of sketchState.edges) {
          if ((e.a === aId && e.b === bId) || (e.a === bId && e.b === aId)) return e;
        }
        return null;
      };
      window.__edgeLength = function(edge) {
        const byId = new Map(sketchState.points.map(p => [p.id, p]));
        const a = byId.get(edge.a), b = byId.get(edge.b);
        if (!a || !b) return 0;
        return Math.hypot(b.x - a.x, b.y - a.y, b.z - a.z);
      };
      window.__pointDegree = function(pointId) {
        let n = 0;
        for (const e of sketchState.edges) if (e.a === pointId || e.b === pointId) n++;
        return n;
      };
      window.__edgesAtPoint = function(pointId) {
        const out = [];
        for (const e of sketchState.edges) if (e.a === pointId || e.b === pointId) out.push(e.id);
        return out;
      };

      // ── Constraint helpers ─────────────────────────────────────
      window.__getConstraintsForTarget = function(targetId) {
        return sketchState.constraints.filter(c => c.targetId === targetId);
      };
      window.__getConstraintForTarget = function(type, targetId) {
        for (const c of sketchState.constraints) {
          if (c.type === type && c.targetId === targetId) return c;
        }
        return null;
      };
      window.__addConstraint = function(type, targetType, targetId, value) {
        // Replace existing of same type+target.
        const existing = window.__getConstraintForTarget(type, targetId);
        if (existing) { existing.value = (value === undefined) ? existing.value : value; return existing; }
        const c = { id: window.__nextConstraintId(), type, targetType, targetId, value: value == null ? null : value };
        sketchState.constraints.push(c);
        return c;
      };
      window.__removeConstraint = function(constraintId) {
        const before = sketchState.constraints.length;
        sketchState.constraints = sketchState.constraints.filter(c => c.id !== constraintId);
        return sketchState.constraints.length !== before;
      };
      window.__removeConstraintsForTarget = function(targetId) {
        const before = sketchState.constraints.length;
        sketchState.constraints = sketchState.constraints.filter(c => c.targetId !== targetId);
        return sketchState.constraints.length !== before;
      };
      window.__isPointFixed = function(pointId) {
        return !!window.__getConstraintForTarget('fixed_point', pointId);
      };
      window.__getEdgeLengthConstraint = function(edgeId) {
        return window.__getConstraintForTarget('edge_length', edgeId);
      };
      window.__hasHorizontalConstraint = function(edgeId) {
        return !!window.__getConstraintForTarget('horizontal', edgeId);
      };
      window.__hasVerticalConstraint = function(edgeId) {
        return !!window.__getConstraintForTarget('vertical', edgeId);
      };

      // ── Geometry mutators applied by constraints ───────────────
      function __planeClamp(point) {
        const pl = sketchState.workingPlane || 'XZ';
        if (pl === 'XZ') point.y = 0;
        if (pl === 'XY') point.z = 0;
        if (pl === 'YZ') point.x = 0;
      }
      function __refreshGridCoords(point) {
        const g = (sketchState.precision && sketchState.precision.internalStepM)
                  || sketchState.gridSize || 1.0;
        point.gx = Math.round(point.x / g);
        point.gy = Math.round(point.y / g);
        point.gz = Math.round(point.z / g);
      }

      // Apply explicit edge length. Returns true on success.
      window.__applyEdgeLength = function(edge, length) {
        const byId = new Map(sketchState.points.map(p => [p.id, p]));
        const a = byId.get(edge.a), b = byId.get(edge.b);
        if (!a || !b) return false;
        const aFixed = window.__isPointFixed(a.id);
        const bFixed = window.__isPointFixed(b.id);
        if (aFixed && bFixed) {
          window.__setStatusMessage('Нельзя задать размер: обе точки зафиксированы');
          return false;
        }
        let dx = b.x - a.x, dy = b.y - a.y, dz = b.z - a.z;
        let len = Math.hypot(dx, dy, dz);
        if (len < 1e-6) {
          // Fallback direction: plane-horizontal axis.
          const pl = sketchState.workingPlane || 'XZ';
          if (pl === 'YZ') { dx = 0; dy = 0; dz = 1; }
          else             { dx = 1; dy = 0; dz = 0; }
          len = 1;
        }
        const ux = dx / len, uy = dy / len, uz = dz / len;
        const moveB = !bFixed;
        const target = moveB ? b : a;
        const anchor = moveB ? a : b;
        const sign   = moveB ? 1 : -1;
        target.x = anchor.x + sign * ux * length;
        target.y = anchor.y + sign * uy * length;
        target.z = anchor.z + sign * uz * length;
        __planeClamp(target);
        __refreshGridCoords(target);
        return true;
      };

      // Align edge along a world axis ('x' | 'y' | 'z') by moving one endpoint.
      function __applyAxisAlign(edge, axis) {
        const byId = new Map(sketchState.points.map(p => [p.id, p]));
        const a = byId.get(edge.a), b = byId.get(edge.b);
        if (!a || !b) return false;
        const aFixed = window.__isPointFixed(a.id);
        const bFixed = window.__isPointFixed(b.id);
        if (aFixed && bFixed) {
          window.__setStatusMessage('Нельзя ограничить: обе точки зафиксированы');
          return false;
        }
        const moveB = !bFixed;
        const target = moveB ? b : a;
        const anchor = moveB ? a : b;
        if (axis === 'x') { target.y = anchor.y; target.z = anchor.z; }
        if (axis === 'y') { target.x = anchor.x; target.z = anchor.z; }
        if (axis === 'z') { target.x = anchor.x; target.y = anchor.y; }
        __planeClamp(target);
        __refreshGridCoords(target);
        return true;
      }
      window.__applyHorizontal = function(edge) {
        const pl = sketchState.workingPlane || 'XZ';
        // horizontal axis: XZ→X, XY→X, YZ→Z
        const axis = (pl === 'YZ') ? 'z' : 'x';
        return __applyAxisAlign(edge, axis);
      };
      window.__applyVertical = function(edge) {
        const pl = sketchState.workingPlane || 'XZ';
        // vertical axis: XZ→Z, XY→Y, YZ→Y
        const axis = (pl === 'XZ') ? 'z' : 'y';
        return __applyAxisAlign(edge, axis);
      };

      // ── Validation ─────────────────────────────────────────────
      window.__countOpenEnds = function() {
        return window.__getOpenEndPointIds().length;
      };
      window.__countIsolatedPoints = function() {
        return window.__getIsolatedPointIds().length;
      };
      window.__getOpenEndPointIds = function() {
        const deg = new Map();
        for (const e of sketchState.edges) {
          deg.set(e.a, (deg.get(e.a) || 0) + 1);
          deg.set(e.b, (deg.get(e.b) || 0) + 1);
        }
        const out = [];
        for (const p of sketchState.points) {
          if ((deg.get(p.id) || 0) === 1) out.push(p.id);
        }
        return out;
      };
      window.__getIsolatedPointIds = function() {
        const deg = new Map();
        for (const e of sketchState.edges) {
          deg.set(e.a, (deg.get(e.a) || 0) + 1);
          deg.set(e.b, (deg.get(e.b) || 0) + 1);
        }
        const out = [];
        for (const p of sketchState.points) {
          if ((deg.get(p.id) || 0) === 0) out.push(p.id);
        }
        return out;
      };
      window.__recomputeValidation = function() {
        sketchState.validation.isolatedIds = window.__getIsolatedPointIds();
        sketchState.validation.openEndIds  = window.__getOpenEndPointIds();
      };

      // ── Profile detection (simple cycles, length 3..20) ────────
      function __detectProfilePlane(pointIds) {
        const byId = new Map(sketchState.points.map(p => [p.id, p]));
        const pts  = pointIds.map(id => byId.get(id)).filter(Boolean);
        if (!pts.length) return 'unknown';
        const eps = 1e-4;
        const sameY = pts.every(p => Math.abs(p.y - pts[0].y) < eps);
        if (sameY) return 'XZ';
        const sameZ = pts.every(p => Math.abs(p.z - pts[0].z) < eps);
        if (sameZ) return 'XY';
        const sameX = pts.every(p => Math.abs(p.x - pts[0].x) < eps);
        if (sameX) return 'YZ';
        return 'unknown';
      }

      function __findSimpleCycles() {
        const N = sketchState.points.length;
        if (N < 3) return [];
        const idIdx = new Map();
        sketchState.points.forEach((p, i) => idIdx.set(p.id, i));
        const adj = Array.from({ length: N }, () => []);
        const edgeBetween = new Map();
        for (const e of sketchState.edges) {
          const ai = idIdx.get(e.a), bi = idIdx.get(e.b);
          if (ai == null || bi == null || ai === bi) continue;
          adj[ai].push(bi);
          adj[bi].push(ai);
          const k = ai < bi ? (ai + '-' + bi) : (bi + '-' + ai);
          edgeBetween.set(k, e.id);
        }
        const seen = new Set();
        const cycles = [];
        const MAX_LEN  = 20;
        const MAX_CYCLES = 64;
        function dfs(start, u, path, onPath) {
          if (cycles.length >= MAX_CYCLES) return;
          if (path.length > MAX_LEN) return;
          const neighbors = adj[u];
          for (let i = 0; i < neighbors.length; i++) {
            const v = neighbors[i];
            if (v < start) continue;
            if (v === start && path.length >= 3) {
              const second = path[1], last = path[path.length - 1];
              const canonical = (second <= last) ? path.slice() : [path[0], ...path.slice(1).reverse()];
              const key = canonical.join(',');
              if (!seen.has(key)) { seen.add(key); cycles.push(canonical); }
              continue;
            }
            if (onPath[v]) continue;
            onPath[v] = 1;
            path.push(v);
            dfs(start, v, path, onPath);
            path.pop();
            onPath[v] = 0;
          }
        }
        for (let i = 0; i < N; i++) {
          if (cycles.length >= MAX_CYCLES) break;
          const onPath = new Uint8Array(N);
          onPath[i] = 1;
          dfs(i, i, [i], onPath);
        }
        // Map index cycles → point/edge id cycles.
        return cycles.map(cyc => {
          const pointIds = cyc.map(idx => sketchState.points[idx].id);
          const edgeIds  = [];
          for (let i = 0; i < cyc.length; i++) {
            const a = cyc[i], b = cyc[(i + 1) % cyc.length];
            const k = a < b ? (a + '-' + b) : (b + '-' + a);
            const eid = edgeBetween.get(k);
            if (eid) edgeIds.push(eid);
          }
          return { pointIds, edgeIds };
        });
      }

      window.__recomputeProfiles = function() {
        __profileCounter = 0;
        const raw = __findSimpleCycles();
        sketchState.profiles = raw.map(cyc => ({
          id: 'profile_' + (++__profileCounter),
          pointIds: cyc.pointIds,
          edgeIds:  cyc.edgeIds,
          plane:    __detectProfilePlane(cyc.pointIds),
          closed:   true,
        }));
      };

      window.__getProfileForEdge = function(edgeId) {
        for (const prof of sketchState.profiles) {
          if (prof.edgeIds.indexOf(edgeId) !== -1) return prof;
        }
        return null;
      };

      // ── Profile selection helpers (Phase 8) ───────────────────
      window.__getProfileById = function(profileId) {
        if (!profileId) return null;
        return sketchState.profiles.find(p => p.id === profileId) || null;
      };

      window.__getProfilesForEdge = function(edgeId) {
        return sketchState.profiles.filter(p => p.edgeIds.indexOf(edgeId) !== -1);
      };

      window.__clearProfileSelection = function() {
        sketchState.selectedProfileId = null;
      };

      window.__selectProfile = function(profileId) {
        const prof = window.__getProfileById(profileId);
        if (!prof) { sketchState.selectedProfileId = null; return null; }
        sketchState.selectedProfileId = prof.id;
        window.__setStatusMessage('Выбран ' + prof.id + ' (' + prof.edgeIds.length + ' рёбер)');
        return prof;
      };

      window.__setHoverProfile = function(profileId) {
        sketchState.hoverProfileId = profileId || null;
      };

      window.__getSelectedProfile = function() {
        return window.__getProfileById(sketchState.selectedProfileId);
      };

      // Project a point (x,y,z) onto the plane's 2D (u,v).
      //   XZ → (x, z)
      //   XY → (x, y)
      //   YZ → (z, y)
      window.__projectToPlane2D = function(plane, x, y, z) {
        if (plane === 'XY') return { u: x, v: y };
        if (plane === 'YZ') return { u: z, v: y };
        return { u: x, v: z }; // XZ
      };

      // Shoelace area in active plane projection. Always positive.
      window.__profileArea = function(prof) {
        if (!prof || !prof.pointIds || prof.pointIds.length < 3) return 0;
        const pById = new Map(sketchState.points.map(p => [p.id, p]));
        const ring = prof.pointIds.map(id => pById.get(id)).filter(Boolean);
        if (ring.length < 3) return 0;
        const plane = prof.plane || sketchState.workingPlane || 'XZ';
        const uv = ring.map(p => window.__projectToPlane2D(plane, p.x, p.y, p.z));
        let s = 0;
        for (let i = 0; i < uv.length; i++) {
          const a = uv[i], b = uv[(i + 1) % uv.length];
          s += a.u * b.v - b.u * a.v;
        }
        return Math.abs(s * 0.5);
      };

      // Point-in-polygon (ray casting) in plane 2D projection.
      window.__profileContainsWorld = function(prof, x, y, z) {
        if (!prof || !prof.pointIds || prof.pointIds.length < 3) return false;
        const pById = new Map(sketchState.points.map(p => [p.id, p]));
        const ring = prof.pointIds.map(id => pById.get(id)).filter(Boolean);
        if (ring.length < 3) return false;
        const plane = prof.plane || sketchState.workingPlane || 'XZ';
        const uv = ring.map(p => window.__projectToPlane2D(plane, p.x, p.y, p.z));
        const q  = window.__projectToPlane2D(plane, x, y, z);
        let inside = false;
        for (let i = 0, j = uv.length - 1; i < uv.length; j = i++) {
          const a = uv[i], b = uv[j];
          const intersect = ((a.v > q.v) !== (b.v > q.v))
            && (q.u < (b.u - a.u) * (q.v - a.v) / ((b.v - a.v) || 1e-12) + a.u);
          if (intersect) inside = !inside;
        }
        return inside;
      };

      // Smallest containing profile at world (x,y,z) on active plane.
      window.__pickProfileAtWorld = function(x, y, z) {
        const hits = sketchState.profiles
          .filter(p => p.plane === sketchState.workingPlane)
          .filter(p => window.__profileContainsWorld(p, x, y, z))
          .map(p => ({ p, area: window.__profileArea(p) }))
          .filter(h => h.area > 0)
          .sort((a, b) => a.area - b.area);
        return hits.length ? hits[0].p.id : null;
      };

      // Build the payload for future extrude.
      window.__selectedProfileToPayload = function() {
        const prof = window.__getSelectedProfile();
        if (!prof) return null;
        const pById = new Map(sketchState.points.map(p => [p.id, p]));
        const points = prof.pointIds.map(id => {
          const p = pById.get(id);
          return p ? { id: p.id, gx: p.gx, gy: p.gy, gz: p.gz, x: p.x, y: p.y, z: p.z } : null;
        }).filter(Boolean);
        return {
          profileId: prof.id,
          plane:     prof.plane,
          pointIds:  [...prof.pointIds],
          edgeIds:   [...prof.edgeIds],
          points,
          area:      window.__profileArea(prof),
        };
      };

      window.__isSelectedProfileExtrudable = function() {
        const prof = window.__getSelectedProfile();
        if (!prof) return false;
        if (!prof.closed) return false;
        if (!prof.plane || prof.plane === 'unknown') return false;
        if (!prof.edgeIds || prof.edgeIds.length < 3) return false;
        return window.__profileArea(prof) > 0;
      };

      window.__notifySketchChanged = function() {
        window.__recomputeValidation();
        window.__recomputeProfiles();
        // Phase 8: invalidate stale profile selection.
        if (sketchState.selectedProfileId
            && !sketchState.profiles.some(p => p.id === sketchState.selectedProfileId)) {
          sketchState.selectedProfileId = null;
        }
        if (sketchState.hoverProfileId
            && !sketchState.profiles.some(p => p.id === sketchState.hoverProfileId)) {
          sketchState.hoverProfileId = null;
        }
      };

      // ── Mutations ──────────────────────────────────────────────
      window.__addPoint = function(gx, gy, gz) {
        const existing = window.__findPointAtGrid(gx, gy, gz);
        if (existing) return existing;
        const g = __internalStep();
        const p = { id: window.__nextPointId(), gx, gy, gz, x: gx * g, y: gy * g, z: gz * g };
        sketchState.points.push(p);
        window.__notifySketchChanged();
        return p;
      };
      window.__addEdge = function(aId, bId, kind) {
        if (!aId || !bId || aId === bId) return null;
        const existing = window.__findEdgeBetween(aId, bId);
        if (existing) {
          // Upgrade kind if a stricter (visible) variant was requested.
          if (kind && kind !== existing.kind) existing.kind = kind;
          return existing;
        }
        const e = { id: window.__nextEdgeId(), a: aId, b: bId, kind: kind || 'normal' };
        sketchState.edges.push(e);
        window.__notifySketchChanged();
        return e;
      };
      window.__deleteSelected = function() {
        const sp = sketchState.selectedPointIds;
        const se = sketchState.selectedEdgeIds;
        if (sp.size === 0 && se.size === 0) return false;
        // Determine edges to remove (selected + incident on removed points).
        const removedEdges = new Set();
        for (const e of sketchState.edges) {
          if (se.has(e.id) || sp.has(e.a) || sp.has(e.b)) removedEdges.add(e.id);
        }
        sketchState.edges  = sketchState.edges.filter(e => !removedEdges.has(e.id));
        sketchState.points = sketchState.points.filter(p => !sp.has(p.id));
        // Cascade constraint removal.
        sketchState.constraints = sketchState.constraints.filter(c => {
          if (c.targetType === 'point' && sp.has(c.targetId)) return false;
          if (c.targetType === 'edge'  && removedEdges.has(c.targetId)) return false;
          return true;
        });
        sketchState.selectedPointIds = new Set();
        sketchState.selectedEdgeIds  = new Set();
        if (sketchState.hoverPointId && !sketchState.points.some(p => p.id === sketchState.hoverPointId)) sketchState.hoverPointId = null;
        if (sketchState.hoverEdgeId  && !sketchState.edges.some(e  => e.id === sketchState.hoverEdgeId )) sketchState.hoverEdgeId  = null;
        if (sketchState.line.startPointId && !sketchState.points.some(p => p.id === sketchState.line.startPointId)) {
          sketchState.line.startPointId = null;
        }
        window.__notifySketchChanged();
        return true;
      };

      // ── Working plane ──────────────────────────────────────────
      window.__setWorkingPlane = function(plane) {
        if (!['XZ','XY','YZ'].includes(plane)) return;
        sketchState.workingPlane = plane;
        sketchState.plane         = plane;
        sketchState.hoverWorld    = null;
        const el = document.getElementById('si-plane');
        if (el) el.textContent = plane;
        const sb = document.getElementById('mini-plane');
        if (sb) sb.textContent = plane;
        if (typeof log === 'function') log('◇ Working plane → ' + plane, '#67e8f9');
      };

      // ── Projection Drafting Mode (Phase 13) ────────────────────
      // Plane label helper — in projection mode returns Top/Front/Side,
      // in free3d returns raw plane name.
      window.__planeLabel = function(plane) {
        const pl = plane || sketchState.workingPlane || 'XZ';
        if (sketchState.draftMode !== 'projection') return pl;
        if (pl === 'XZ') return 'Top';
        if (pl === 'XY') return 'Front';
        if (pl === 'YZ') return 'Side';
        return pl;
      };
      // Full descriptive label, e.g. "XZ · Horizontal / Top".
      window.__planeDescriptor = function(plane) {
        const pl = plane || sketchState.workingPlane || 'XZ';
        if (pl === 'XZ') return 'XZ · Horizontal / Top';
        if (pl === 'XY') return 'XY · Front';
        if (pl === 'YZ') return 'YZ · Profile / Side';
        return pl;
      };

      // Map a 3D point onto the 2D coordinates visible in a given plane.
      // Returns { hAxis, vAxis, hiddenAxis, h, v, hidden } where h/v are the
      // visible projected coordinates and `hidden` is the axis that collapses.
      window.__projectionCoordsForPlane = function(point, plane) {
        const pl = plane || sketchState.workingPlane || 'XZ';
        if (!point) return null;
        if (pl === 'XZ') return { hAxis:'X', vAxis:'Z', hiddenAxis:'Y', h:point.x, v:point.z, hidden:point.y };
        if (pl === 'XY') return { hAxis:'X', vAxis:'Y', hiddenAxis:'Z', h:point.x, v:point.y, hidden:point.z };
        if (pl === 'YZ') return { hAxis:'Z', vAxis:'Y', hiddenAxis:'X', h:point.z, v:point.y, hidden:point.x };
        return null;
      };

      // Compose a 3D point from two projection pairs.
      // Stub for future "auto-reconstruct from two views" feature.
      // planeA/planeB are 'XZ' | 'XY' | 'YZ'; hA/vA/hB/vB are the projected
      // coordinates on each plane. Returns {x,y,z} or null on conflict.
      window.__pointFromProjectionPair = function(planeA, hA, vA, planeB, hB, vB) {
        if (!planeA || !planeB || planeA === planeB) return null;
        const set = {};
        function put(axis, val) {
          if (set[axis] === undefined) { set[axis] = val; return true; }
          return Math.abs(set[axis] - val) < 1e-6;
        }
        function feed(plane, h, v) {
          const map = window.__projectionCoordsForPlane({x:0,y:0,z:0}, plane);
          if (!map) return false;
          return put(map.hAxis, h) && put(map.vAxis, v);
        }
        if (!feed(planeA, hA, vA)) return null;
        if (!feed(planeB, hB, vB)) return null;
        return { x: set.X || 0, y: set.Y || 0, z: set.Z || 0 };
      };

      // Toggle / set draft mode.
      window.__setDraftMode = function(mode) {
        if (mode !== 'free3d' && mode !== 'projection') return;
        if (sketchState.draftMode === mode) return;
        sketchState.draftMode = mode;
        if (window.__setStatusMessage) {
          window.__setStatusMessage('Режим черчения: ' + (mode === 'projection' ? 'Проекция' : 'Свободный 3D'));
        }
        if (window.__updateSketchInspector) window.__updateSketchInspector();
      };
      window.__toggleDraftMode = function() {
        window.__setDraftMode(sketchState.draftMode === 'projection' ? 'free3d' : 'projection');
      };

      // ── Engine-aware bulk point creation ──
      // Adds a point through the currently active engine and returns the new
      // point id (string) via a Promise. Falls back to the legacy local model
      // when neither backend nor wasm engines are available.
      window.__createPointViaEngine = async function(gx, gy, gz) {
        const existing = window.__findPointAtGrid(gx, gy, gz);
        if (existing) return existing.id;
        const mode = sketchState.engineMode || 'backend';
        if ((mode === 'wasm' || mode === 'hybrid') && window.__wasmAddPointAndApply) {
          const r = await window.__wasmAddPointAndApply(gx, gy, gz);
          if (r && r.ok) return r.pointId;
        }
        if (mode === 'backend' && window.__backendAddPoint) {
          const r = await window.__backendAddPoint(gx, gy, gz);
          if (r && r.ok) return r.pointId;
        }
        const p = window.__addPoint(gx, gy, gz);
        return p ? p.id : null;
      };
      window.__createEdgeViaEngine = async function(aId, bId, kind) {
        if (!aId || !bId || aId === bId) return null;
        const mode = sketchState.engineMode || 'backend';
        if ((mode === 'wasm' || mode === 'hybrid') && window.__wasmAddEdgeAndApply) {
          await window.__wasmAddEdgeAndApply({ pointId: aId }, { pointId: bId });
        } else if (mode === 'backend' && window.__backendAddEdge) {
          await window.__backendAddEdge({ pointId: aId }, { pointId: bId });
        } else {
          window.__addEdge(aId, bId, kind);
          return;
        }
        // After backend/wasm sync, stamp the kind on the freshly-added edge.
        const e = window.__findEdgeBetween(aId, bId);
        if (e && kind && kind !== 'normal') e.kind = kind;
      };

      // ── Projection box ──
      // Creates an axis-aligned wireframe box (12 edges) at the origin with the
      // given grid-unit dimensions. Uses the active engine.
      window.__createProjectionBox = async function(w, h, d) {
        const W = Math.max(1, Math.round(w || sketchState.projection.boxWidth));
        const H = Math.max(1, Math.round(h || sketchState.projection.boxHeight));
        const D = Math.max(1, Math.round(d || sketchState.projection.boxDepth));
        if (window.__pushHistory) window.__pushHistory();
        const A  = await window.__createPointViaEngine(0, 0, 0);
        const B  = await window.__createPointViaEngine(W, 0, 0);
        const C  = await window.__createPointViaEngine(W, 0, D);
        const D1 = await window.__createPointViaEngine(0, 0, D);
        const A2 = await window.__createPointViaEngine(0, H, 0);
        const B2 = await window.__createPointViaEngine(W, H, 0);
        const C2 = await window.__createPointViaEngine(W, H, D);
        const D2 = await window.__createPointViaEngine(0, H, D);
        const edges = [
          [A,B],[B,C],[C,D1],[D1,A],
          [A2,B2],[B2,C2],[C2,D2],[D2,A2],
          [A,A2],[B,B2],[C,C2],[D1,D2],
        ];
        for (const [u, v] of edges) {
          await window.__createEdgeViaEngine(u, v, 'normal');
        }
        if (window.__setStatusMessage) {
          window.__setStatusMessage('Блок проекции ' + W + '×' + H + '×' + D + ' создан');
        }
        if (window.__updateSketchInspector) window.__updateSketchInspector();
      };

      // ── Sample sloped block (engineering drafting example) ──
      // Wireframe only — does NOT generate solid geometry.
      window.__createSampleSlopedBlock = async function() {
        if (window.__pushHistory) window.__pushHistory();
        const P  = {};
        P.p1  = await window.__createPointViaEngine(0, 0, 0);
        P.p2  = await window.__createPointViaEngine(6, 0, 0);
        P.p3  = await window.__createPointViaEngine(6, 0, 4);
        P.p4  = await window.__createPointViaEngine(0, 0, 4);
        P.p5  = await window.__createPointViaEngine(0, 2, 0);
        P.p6  = await window.__createPointViaEngine(6, 2, 0);
        P.p7  = await window.__createPointViaEngine(2, 5, 0);
        P.p8  = await window.__createPointViaEngine(6, 5, 0);
        P.p9  = await window.__createPointViaEngine(2, 5, 4);
        P.p10 = await window.__createPointViaEngine(6, 5, 4);
        P.p11 = await window.__createPointViaEngine(0, 2, 4);
        P.p12 = await window.__createPointViaEngine(6, 2, 4);
        const edges = [
          // base
          [P.p1, P.p2], [P.p2, P.p3], [P.p3, P.p4], [P.p4, P.p1],
          // verticals up to lower shoulder
          [P.p1, P.p5], [P.p2, P.p6], [P.p4, P.p11], [P.p3, P.p12],
          // upper sloped top
          [P.p5, P.p7], [P.p7, P.p8], [P.p8, P.p6],
          [P.p11, P.p9], [P.p9, P.p10], [P.p10, P.p12],
          // top-side connectors
          [P.p5, P.p11], [P.p7, P.p9], [P.p8, P.p10], [P.p6, P.p12],
        ];
        for (const [u, v] of edges) {
          await window.__createEdgeViaEngine(u, v, 'normal');
        }
        if (window.__setStatusMessage) {
          window.__setStatusMessage('Образец наклонного блока создан (каркас)');
        }
        if (window.__updateSketchInspector) window.__updateSketchInspector();
      };

      // ── History (undo / redo) ──────────────────────────────────
      function __cloneSnapshot() {
        return {
          points: sketchState.points.map(p => ({ id:p.id, gx:p.gx, gy:p.gy, gz:p.gz, x:p.x, y:p.y, z:p.z })),
          edges:  sketchState.edges.map(e => ({ id:e.id, a:e.a, b:e.b })),
          constraints: sketchState.constraints.map(c => ({ id:c.id, type:c.type, targetType:c.targetType, targetId:c.targetId, value:c.value })),
          selPts: [...sketchState.selectedPointIds],
          selEds: [...sketchState.selectedEdgeIds],
          pCtr:   __pointCounter,
          eCtr:   __edgeCounter,
          cCtr:   __constraintCounter,
        };
      }
      function __applySnapshot(s) {
        sketchState.points = s.points.map(p => ({ ...p }));
        sketchState.edges  = s.edges.map(e => ({ ...e }));
        sketchState.constraints = (s.constraints || []).map(c => ({ ...c }));
        sketchState.selectedPointIds = new Set(s.selPts);
        sketchState.selectedEdgeIds  = new Set(s.selEds);
        __pointCounter      = s.pCtr;
        __edgeCounter       = s.eCtr;
        __constraintCounter = s.cCtr || 0;
        sketchState.hoverPointId = null;
        sketchState.hoverEdgeId  = null;
        sketchState.line.startPointId = null;
        window.__notifySketchChanged();
      }
      window.__pushHistory = function() {
        sketchState._history.undo.push(__cloneSnapshot());
        if (sketchState._history.undo.length > sketchState._historyLimit) sketchState._history.undo.shift();
        sketchState._history.redo.length = 0;
      };
      window.__undo = function() {
        const hist = sketchState._history;
        if (!hist.undo.length) return false;
        hist.redo.push(__cloneSnapshot());
        __applySnapshot(hist.undo.pop());
        if (typeof log === 'function') log('↶ undo', '#a78bfa');
        return true;
      };
      window.__redo = function() {
        const hist = sketchState._history;
        if (!hist.redo.length) return false;
        hist.undo.push(__cloneSnapshot());
        __applySnapshot(hist.redo.pop());
        if (typeof log === 'function') log('↷ redo', '#a78bfa');
        return true;
      };

      // ── Serialisation ──────────────────────────────────────────
      // SketchGraph v1 — canonical front-end ↔ Rust backend contract.
      window.__SKETCH_SCHEMA_VERSION = 1;

      window.__sketchToJSON = function() {
        return {
          schema:       'sketch_graph',
          version:      window.__SKETCH_SCHEMA_VERSION,
          workingPlane: sketchState.workingPlane,
          gridSize:     sketchState.gridSize,
          points: sketchState.points.map(p => ({ id:p.id, gx:p.gx, gy:p.gy, gz:p.gz, x:p.x, y:p.y, z:p.z })),
          edges:  sketchState.edges.map(e => ({ id:e.id, a:e.a, b:e.b })),
          constraints: sketchState.constraints.map(c => ({
            id: c.id, type: c.type, targetType: c.targetType, targetId: c.targetId, value: c.value
          })),
          profiles: (sketchState.profiles || []).map(pf => ({
            id: pf.id, pointIds: [...pf.pointIds], edgeIds: [...pf.edgeIds],
            plane: pf.plane, closed: !!pf.closed,
          })),
        };
      };

      // Backend-compatible payload — slim shape, no derived fields.
      // Profiles & validation are omitted (they are recomputable downstream).
      window.__sketchExportPayload = function() {
        return {
          schema:       'sketch_graph',
          version:      window.__SKETCH_SCHEMA_VERSION,
          workingPlane: sketchState.workingPlane,
          gridSize:     sketchState.gridSize,
          points: sketchState.points.map(p => ({ id:p.id, gx:p.gx, gy:p.gy, gz:p.gz })),
          edges:  sketchState.edges.map(e => ({ id:e.id, a:e.a, b:e.b })),
          constraints: sketchState.constraints.map(c => ({
            type: c.type, targetType: c.targetType, targetId: c.targetId, value: c.value
          })),
        };
      };

      // ── Validation ─────────────────────────────────────────────
      // Returns { ok:true } on success, or { ok:false, error:string }.
      window.__validateSketchJSON = function(obj) {
        if (!obj || typeof obj !== 'object')                 return { ok:false, error:'Not a JSON object' };
        if (obj.schema && obj.schema !== 'sketch_graph')     return { ok:false, error:'Wrong schema tag: ' + obj.schema };
        if (!Array.isArray(obj.points))                      return { ok:false, error:'Missing "points" array' };
        if (!Array.isArray(obj.edges))                       return { ok:false, error:'Missing "edges" array' };
        const constraints = obj.constraints || [];
        if (!Array.isArray(constraints))                     return { ok:false, error:'"constraints" must be an array' };
        const plane = obj.workingPlane || 'XZ';
        if (!['XZ','XY','YZ'].includes(plane))               return { ok:false, error:'Invalid workingPlane: ' + plane };
        const gridSize = (obj.gridSize == null) ? 1.0 : obj.gridSize;
        if (!isFinite(gridSize) || gridSize <= 0)            return { ok:false, error:'gridSize must be a positive number' };

        // Points: id uniqueness, fields present, grid uniqueness.
        const ids = new Set();
        const gridKeys = new Set();
        for (let i = 0; i < obj.points.length; i++) {
          const p = obj.points[i];
          if (!p || typeof p.id !== 'string')                return { ok:false, error:'points[' + i + '].id missing' };
          if (ids.has(p.id))                                 return { ok:false, error:'Duplicate point id: ' + p.id };
          ids.add(p.id);
          for (const k of ['gx','gy','gz']) {
            if (!Number.isInteger(p[k]))                     return { ok:false, error:'points[' + i + '].' + k + ' must be integer' };
          }
          const gk = p.gx + ',' + p.gy + ',' + p.gz;
          if (gridKeys.has(gk))                              return { ok:false, error:'Duplicate point grid coords: (' + gk + ')' };
          gridKeys.add(gk);
        }

        // Edges: id uniqueness, endpoints exist, no self-loops.
        const edgeIds = new Set();
        for (let i = 0; i < obj.edges.length; i++) {
          const e = obj.edges[i];
          if (!e || typeof e.id !== 'string')                return { ok:false, error:'edges[' + i + '].id missing' };
          if (edgeIds.has(e.id))                             return { ok:false, error:'Duplicate edge id: ' + e.id };
          edgeIds.add(e.id);
          if (!ids.has(e.a))                                 return { ok:false, error:'Edge ' + e.id + ' references unknown point a=' + e.a };
          if (!ids.has(e.b))                                 return { ok:false, error:'Edge ' + e.id + ' references unknown point b=' + e.b };
          if (e.a === e.b)                                   return { ok:false, error:'Edge ' + e.id + ' is a self-loop' };
        }

        // Constraints: types & targets valid.
        const validC = new Set(['fixed_point','edge_length','horizontal','vertical']);
        for (let i = 0; i < constraints.length; i++) {
          const c = constraints[i];
          if (!c || !validC.has(c.type))                     return { ok:false, error:'constraints[' + i + '] invalid type: ' + (c && c.type) };
          if (c.type === 'fixed_point') {
            if (c.targetType !== 'point' || !ids.has(c.targetId))
              return { ok:false, error:'fixed_point on unknown point: ' + c.targetId };
          } else {
            if (c.targetType !== 'edge' || !edgeIds.has(c.targetId))
              return { ok:false, error:c.type + ' on unknown edge: ' + c.targetId };
            if (c.type === 'edge_length' && (!isFinite(c.value) || c.value <= 0))
              return { ok:false, error:'edge_length value must be positive: ' + c.value };
          }
        }
        return { ok:true };
      };

      // Import a SketchGraph object. Pushes history snapshot first.
      // Returns { ok:true, stats:{points,edges,constraints} } or { ok:false, error }.
      window.__sketchFromJSON = function(obj) {
        const v = window.__validateSketchJSON(obj);
        if (!v.ok) return v;

        // Snapshot current state for undo.
        window.__pushHistory();

        const g = (obj.gridSize == null) ? 1.0 : obj.gridSize;
        sketchState.gridSize     = g;
        // Keep three-tier precision in sync: imported gridSize *is* the
        // internal engine step. snap/display steps are preserved from the
        // current session (or fall back to sensible defaults).
        if (sketchState.precision) {
          sketchState.precision.internalStepM = g;
          if (!isFinite(sketchState.precision.snapStepM) || sketchState.precision.snapStepM <= 0) {
            sketchState.precision.snapStepM = 0.001;
          }
          if (!isFinite(sketchState.precision.displayGridStepM) || sketchState.precision.displayGridStepM <= 0) {
            sketchState.precision.displayGridStepM = 0.001;
          }
        }
        sketchState.workingPlane = obj.workingPlane || 'XZ';
        sketchState.plane        = sketchState.workingPlane;

        // Rebuild points — derive world coords from grid + gridSize.
        sketchState.points = obj.points.map(p => ({
          id: p.id,
          gx: p.gx, gy: p.gy, gz: p.gz,
          x: (typeof p.x === 'number') ? p.x : p.gx * g,
          y: (typeof p.y === 'number') ? p.y : p.gy * g,
          z: (typeof p.z === 'number') ? p.z : p.gz * g,
        }));
        sketchState.edges       = obj.edges.map(e => ({ id: e.id, a: e.a, b: e.b }));
        sketchState.constraints = (obj.constraints || []).map(c => ({
          id: c.id || ('c_imp_' + (++__constraintCounter)),
          type: c.type, targetType: c.targetType, targetId: c.targetId,
          value: (c.value == null) ? null : c.value,
        }));

        // Re-seed id counters past the highest imported numeric suffix.
        const bump = (prefix, current) => {
          let max = current;
          const re = new RegExp('^' + prefix + '_(\\d+)$');
          const scan = (arr) => arr.forEach(it => {
            const m = re.exec(it.id || '');
            if (m) { const n = parseInt(m[1], 10); if (n > max) max = n; }
          });
          scan(sketchState.points.filter(x => prefix === 'p'));
          scan(sketchState.edges.filter(x => prefix === 'e'));
          scan(sketchState.constraints.filter(x => prefix === 'c'));
          return max;
        };
        __pointCounter      = bump('p', __pointCounter);
        __edgeCounter       = bump('e', __edgeCounter);
        __constraintCounter = bump('c', __constraintCounter);

        // Reset transient state.
        sketchState.selectedPointIds = new Set();
        sketchState.selectedEdgeIds  = new Set();
        sketchState.hoverPointId = null;
        sketchState.hoverEdgeId  = null;
        sketchState.hoverWorld   = null;
        sketchState.line.startPointId = null;
        sketchState.line.previewPoint = null;
        sketchState.grab.active = false;

        // Recompute derived sets.
        window.__notifySketchChanged();
        window.__setStatusMessage('Импортировано ' + sketchState.points.length + ' точек · '
          + sketchState.edges.length + ' рёбер · '
          + sketchState.constraints.length + ' ограничений');
        return {
          ok: true,
          stats: {
            points: sketchState.points.length,
            edges:  sketchState.edges.length,
            constraints: sketchState.constraints.length,
          },
        };
      };
"##;
