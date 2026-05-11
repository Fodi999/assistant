// ── JS: Sketch data model — points, profile, dimensions, snap settings ────────
// Domain: Sketch — all mutable sketch data and phase transitions.

pub const JS: &str = r##"
      // ── Sketch State (data model) ──────────────────────────────
      const sketchState = {
        points: [],   // { x, y, z }
        closed: false,
        plane: 'XZ',  // 'XZ' (y=0), 'XY' (z=0), 'YZ' (x=0)
        showGrid: true,
        // ── State machine ──────────────────────────────────────────
        //  'drawing'         → can add points/lines, Extrude disabled
        //  'closed_profile'  → profile locked, Extrude enabled
        //  'extrude_preview' → preview shown, no new points allowed
        //  'solid_created'   → sketch is ghost/reference (read-only)
        phase: 'drawing',
        // Tool state: for Rect/Circle 2-click workflow
        pendingTool:  null,  // 'rectangle' | 'circle' | 'dimension' | null
        pendingStart: null,  // {x,y,z} first corner / center
        dimensions:   [],    // committed dimension annotations
        // Snap visualization (filled by render_loop)
        hover: null,         // {x,y,z, snapType: 'grid'|'point'|'origin'|'first'|'align'}
        // Drag state (filled by pointerdown in state.rs)
        draggedPtIndex: undefined,
      };
      window.sketchState = sketchState;

      // ── Phase transition helper (central place for all transitions) ──
      window.__setSketchPhase = function(next, reason) {
        const prev = sketchState.phase;
        if (prev === next) return;
        sketchState.phase = next;
        log(`◇ sketch phase: ${prev} → ${next}${reason ? ' (' + reason + ')' : ''}`, '#a78bfa');
        if (window.__updateSketchUI) window.__updateSketchUI();
      };

      // ── Extrude Preview state (frontend only, no backend yet) ──
      window.extrudePreview = {
        active:    false,
        profileId: null,
        plane:     'XZ',
        direction: [0, 1, 0],  // unit vector
        distance:  1.0,
        points:    [],         // snapshot of base-profile points
      };

      // ── Grid snap helpers ──────────────────────────────────────
      window.sketchGridSnap = 0.1;
      function snapToGrid(value) {
        const s = window.sketchGridSnap;
        if (!s || s <= 0) return value;
        return Math.round(value / s) * s;
      }

      // ── UI Settings Sync ───────────────────────────────────────
      const planeSel = document.getElementById('sketch-plane-select');
      if (planeSel) {
        planeSel.addEventListener('change', e => {
          if (window.__setSketchPlane) window.__setSketchPlane(e.target.value);
          else sketchState.plane = e.target.value;
        });
      }
      const gridSnapSel = document.getElementById('sketch-grid-snap-select');
      if (gridSnapSel) {
        gridSnapSel.addEventListener('change', e => {
          window.sketchGridSnap = parseFloat(e.target.value);
          const elG = document.getElementById('sketch-info-grid');
          if (elG) {
            const snap = window.sketchGridSnap;
            elG.textContent = snap >= 1 ? snap + ' m' : (snap * 100).toFixed(snap < 0.01 ? 1 : 0) + ' cm';
          }
        });
      }
      const gridToggle = document.getElementById('sketch-grid-toggle');
      if (gridToggle) {
        gridToggle.addEventListener('change', e => { sketchState.showGrid = e.target.checked; });
      }
"##;
