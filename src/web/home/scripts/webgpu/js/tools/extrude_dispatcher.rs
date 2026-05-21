// ── Context Extrude Dispatcher ────────────────────────────────────────────────
// Single entry point for the E hotkey and toolbar "Extrude" button.
//
// Priority (Plasticity / Fusion 360 style):
//   1. solid-extrude gizmo already active  → commit it
//   2. CAD face selected (mode=FACE)        → face extrude
//   3. sketch edges selected                → edge extrude
//   4. closed sketch profile exists         → solid extrude
//   5. nothing valid                        → status hint
//
// Exports:
//   window.__handleContextExtrude()  — call from E hotkey and any toolbar btn
//   window.__extrudeDispatcherReady  — true after registration
// ─────────────────────────────────────────────────────────────────────────────

pub const JS: &str = r##"
(function registerExtrudeDispatcher() {
  const TAG = '[ExtrudeDispatcher]';

  // ── helpers ──────────────────────────────────────────────────────────────

  function _showHint(msg) {
    console.info(TAG, msg);
    // try status bar if present
    const sb = document.getElementById('status-bar-text')
             || document.getElementById('__status_bar');
    if (sb) {
      sb.textContent = msg;
      clearTimeout(sb.__hintTimer);
      sb.__hintTimer = setTimeout(() => { sb.textContent = ''; }, 3000);
    }
  }

  function _selectionSnapshot() {
    return window.CadInteraction?.selection?.snapshot?.() ?? null;
  }

  function _closedProfile(sketchState) {
    if (!sketchState) return null;
    // explicitly selected profile id
    if (sketchState.selectedProfileId) return sketchState.selectedProfileId;
    // first closed profile in list
    if (Array.isArray(sketchState.profiles)) {
      const p = sketchState.profiles.find(p => p.closed);
      if (p) return p.id;
    }
    return null;
  }

  // ── main dispatcher ───────────────────────────────────────────────────────

  window.__handleContextExtrude = function handleContextExtrude() {
    // 1. Solid-extrude gizmo already active → commit
    if (window.__solidExtrudeState?.active) {
      console.log(TAG, 'source=active-solid → commit');
      window.__commitSolidExtrude?.();
      return;
    }

    const sel   = _selectionSnapshot();
    const SM    = window.CadInteraction?.SelectionMode ?? { OBJECT:0, FACE:1, EDGE:2, VERTEX:3 };
    const sketch = window.sketchState;

    // 2. CAD face selected
    if (sel?.selected && sel.mode === SM.FACE && sel.faceId) {
      if (typeof window.__startFaceExtrude === 'function') {
        console.log(TAG, 'source=face → face extrude, faceId=', sel.faceId);
        window.__startFaceExtrude(sel.faceId);
      } else {
        console.warn(TAG, 'source=face but __startFaceExtrude not loaded — falling through');
        _showHint('Face Extrude not available yet');
      }
      return;
    }

    // 3. Sketch edges selected
    const edgeIds = sketch?.selectedEdgeIds;
    if (Array.isArray(edgeIds) && edgeIds.length > 0) {
      if (typeof window.__startEdgeExtrude === 'function') {
        console.log(TAG, 'source=edge → edge extrude, edges=', edgeIds);
        window.__startEdgeExtrude();
      } else {
        console.warn(TAG, '__startEdgeExtrude not loaded');
        _showHint('Edge Extrude not available');
      }
      return;
    }

    // 4. Closed sketch profile
    const profId = _closedProfile(sketch);
    if (profId) {
      if (typeof window.__startSolidExtrude === 'function') {
        console.log(TAG, 'source=profile → solid extrude, profile=', profId);
        window.__startSolidExtrude(profId);
      } else {
        console.warn(TAG, '__startSolidExtrude not loaded');
        _showHint('Solid Extrude not available');
      }
      return;
    }

    // 5. Nothing valid
    console.log(TAG, 'no valid selection');
    _showHint('Select closed profile, face or edge to extrude');
  };

  window.__extrudeDispatcherReady = true;
  console.log(TAG, 'registered: __handleContextExtrude');
})();
"##;
