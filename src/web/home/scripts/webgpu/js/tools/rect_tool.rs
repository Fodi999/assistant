// ── Rectangle Tool — thin WASM wrapper ──────────────────────────────────────
// Geometry logic lives in geometry_engine::tools::rect (Rust/WASM).
// This file only handles: snap resolution, 2-click FSM, cancel, make-square.
//
// WASM calls: wasm_tool_rect(json) → SketchDelta
//             wasm_make_square(json) → SketchDelta
// Applied via: window.__applySketchDelta(delta)
//
// Hotkey: R
// State:  sketchState.rect = { active: false, startSnap: null }

pub const JS: &str = r##"

      window.__rectClick = async function(ndcX, ndcY) {
        const snap = window.__resolveLineSnap ? window.__resolveLineSnap(ndcX, ndcY) : null;
        if (!snap || !snap.valid) {
          window.__setStatusMessage && window.__setStatusMessage('Rect: no snap target');
          return;
        }
        const rect  = sketchState.rect || { active: false, startSnap: null };
        const g     = sketchState.gridSize || 1.0;
        const plane = sketchState.workingPlane || 'XZ';
        const snapToGrid = (s) => ({
          gx: Math.round(s.gx !== undefined ? s.gx : s.x / g),
          gy: Math.round(s.gy !== undefined ? s.gy : s.y / g),
          gz: Math.round(s.gz !== undefined ? s.gz : s.z / g),
        });

        if (!rect.active) {
          const g1 = snapToGrid(snap);
          sketchState.rect  = { active: true, startSnap: g1 };
          sketchState.phase = 'rect-corner';
          window.__setStatusMessage && window.__setStatusMessage(
            '⬡ Rect corner A · click opposite corner · Esc cancel');
          if (window.__notifySketchChanged) window.__notifySketchChanged();
          return;
        }

        const g2 = snapToGrid(snap);
        const g1 = rect.startSnap;
        const wm = window.__wasmModule;
        if (!wm || typeof wm.wasm_tool_rect !== 'function') {
          window.__setStatusMessage && window.__setStatusMessage('Rect: WASM not loaded');
          sketchState.rect = { active: false, startSnap: null };
          sketchState.phase = 'idle';
          return;
        }

        window.__pushHistory();
        const raw   = wm.wasm_tool_rect(JSON.stringify({
          gx1: g1.gx, gy1: g1.gy, gz1: g1.gz,
          gx2: g2.gx, gy2: g2.gy, gz2: g2.gz,
          plane, id_offset: Date.now() % 1000000000,
        }));
        const delta = JSON.parse(raw);

        if (!delta.ok) {
          window.__setStatusMessage && window.__setStatusMessage('Rect: ' + (delta.error || 'error'));
          sketchState.rect  = { active: false, startSnap: null };
          sketchState.phase = 'idle';
          return;
        }

        window.__applySketchDelta(delta);
        sketchState.rect  = { active: false, startSnap: null };
        sketchState.phase = 'idle';
        if (window.__notifySketchChanged)   window.__notifySketchChanged();
        if (window.__updateSketchInspector) window.__updateSketchInspector();
        window.__setStatusMessage && window.__setStatusMessage(
          '✓ Rectangle (' + delta.new_points.length + ' pts, ' + delta.new_edges.length + ' edges)');
      };

      window.__cancelRectTool = function() {
        const wasActive = sketchState.rect && sketchState.rect.active;
        sketchState.rect  = { active: false, startSnap: null };
        sketchState.phase = 'idle';
        if (wasActive) {
          window.__setStatusMessage && window.__setStatusMessage('Rect cancelled');
          if (window.__notifySketchChanged) window.__notifySketchChanged();
        }
      };

      // Backward compat stub — geometry is now in WASM
      window.__solveRectConstraints = function() {};

      window.__makeSquare = async function(profileId) {
        const ss = window.sketchState;
        if (!ss) return;
        if (window.__recomputeProfiles) window.__recomputeProfiles();

        let prof = null;
        if (profileId) prof = (ss.profiles || []).find(p => p.id === profileId);
        if (!prof && ss.selectedProfileId)
          prof = (ss.profiles || []).find(p => p.id === ss.selectedProfileId);
        if (!prof && ss.profiles && ss.profiles.length) prof = ss.profiles[0];
        if (!prof || prof.edgeIds.length !== 4) {
          window.__setStatusMessage && window.__setStatusMessage(
            '⚠ Квадрат: нужен прямоугольник (4 ребра)'); return;
        }

        const wm = window.__wasmModule;
        if (!wm || typeof wm.wasm_make_square !== 'function') {
          window.__setStatusMessage && window.__setStatusMessage('⚠ WASM wasm_make_square не загружен');
          return;
        }

        window.__pushHistory && window.__pushHistory();
        const byPt = new Map((ss.points || []).map(p => [p.id, p]));
        const ptIds   = prof.pointIds.slice(0, 4);
        const edgeIds = prof.edgeIds.slice(0, 4);

        const raw = wm.wasm_make_square(JSON.stringify({
          pt_ids: ptIds, edge_ids: edgeIds,
          pts_gx: ptIds.map(id => (byPt.get(id) || {}).gx || 0),
          pts_gy: ptIds.map(id => (byPt.get(id) || {}).gy || 0),
          pts_gz: ptIds.map(id => (byPt.get(id) || {}).gz || 0),
          plane: ss.workingPlane || 'XZ',
          id_offset: Date.now() % 1000000000,
        }));
        const delta = JSON.parse(raw);
        if (!delta.ok) {
          window.__setStatusMessage && window.__setStatusMessage('⚠ Квадрат: ' + (delta.error || 'error'));
          return;
        }
        window.__applySketchDelta(delta);
        if (window.__notifySketchChanged)   window.__notifySketchChanged();
        if (window.__solveSketchWasm)       await window.__solveSketchWasm();
        else if (window.__solveConstraints) await window.__solveConstraints();
        window.__setStatusMessage && window.__setStatusMessage('⬛ Квадрат создан');
        if (window.__recomputeProfiles) window.__recomputeProfiles();
        if (window.__redrawSketch)      window.__redrawSketch();
        if (window.__updateDofBadge)    window.__updateDofBadge();
      };

"##;
