// ── CAD Side Panel — JS logic ────────────────────────────────────────────────
// Tab switching, accordion toggle, live binding to sketchState.

pub const JS: &str = r#"
// ═══════════════════════════════════════════════════
//  CAD SIDE PANEL  — state + controller
// ═══════════════════════════════════════════════════

window.__cadPanel = {
  activeTab: 'grid',
  openSections: {
    grid: true, units: true,
    snapping: true,
    shader: true, camera: false, helpers: true,
    selection: true, analyze: false,
    engine: true, devjson: false, devsnapstate: false,
  }
};

// ── Tab switch ──────────────────────────────────────
window.__cadPanelTab = function(tab) {
  window.__cadPanel.activeTab = tab;
  // hide all pages, show target
  document.querySelectorAll('#cad-side-panel .csp-page').forEach(p => {
    p.style.display = p.dataset.page === tab ? '' : 'none';
  });
  // update tab button active state
  document.querySelectorAll('#cad-side-panel .csp-tab').forEach(b => {
    b.classList.toggle('active', b.dataset.tab === tab);
  });
  // sync ortho checkbox when opening snap tab
  if (tab === 'snap') window.__cadPanelSyncSnap();
};

// ── Accordion toggle ────────────────────────────────
window.__cadPanelToggleSection = function(name) {
  const st = window.__cadPanel.openSections;
  st[name] = !st[name];
  const section = document.querySelector('#cad-side-panel .csp-section[data-section="' + name + '"]');
  if (!section) return;
  const hdr  = section.querySelector('.csp-section-hdr');
  const body = section.querySelector('.csp-section-body');
  if (!hdr || !body) return;
  if (st[name]) {
    hdr.classList.add('open');
    body.style.display = '';
  } else {
    hdr.classList.remove('open');
    body.style.display = 'none';
  }
};

// ── Grid step — central setter (mm → displayGridStepM in meters) ──
window.__cadSetGrid = function(mm) {
  mm = Math.min(100, Math.max(1, Math.round(mm)));
  // update number input
  const num = document.getElementById('csp-grid-step-num');
  if (num) num.value = mm;
  // update slider
  const sl = document.getElementById('csp-grid-step-slider');
  if (sl) sl.value = mm;
  // apply to sketchState
  if (window.sketchState && sketchState.precision) {
    sketchState.precision.displayGridStepM = mm / 1000;
    sketchState.precision.snapStepMm       = mm;
  }
  // highlight active preset button
  document.querySelectorAll('#cad-side-panel .csp-preset-btn').forEach(b => {
    const v = parseInt(b.textContent);
    b.classList.toggle('csp-preset-active', v === mm);
  });

  // ── Auto-zoom: smoothly bring cam.dist to a comfortable view of this grid ──
  // Target = 15 cells wide on screen (dist ≈ gridM * 15).
  // Only animate if we are way outside the "good" range for this grid size.
  if (window.cam) {
    const gridM   = mm / 1000;
    const minDist = gridM * 5;   // floor: at least 5 cells → grid always visible
    const tgtDist = gridM * 15;  // comfortable: ~15 cells on screen
    const maxDist = gridM * 200; // ceiling: not so zoomed out cells are invisible

    // Decide if a zoom adjustment is needed
    let newDist = null;
    if (cam.dist < minDist)        newDist = minDist;  // too close → pull out
    else if (cam.dist > maxDist)   newDist = tgtDist;  // too far   → zoom in

    if (newDist !== null) {
      const d0 = cam.dist;
      const FRAMES = 30;
      let f = 0;
      (function _step() {
        f++;
        const ease = 1 - Math.pow(1 - f / FRAMES, 3);
        cam.dist = d0 + (newDist - d0) * ease;
        if (f < FRAMES) requestAnimationFrame(_step);
      })();
      if (window.__setStatusMessage)
        window.__setStatusMessage('Grid ' + mm + ' mm — zoom adjusted');
    }
  }
};

// ── Grid step stepper (kept for backward compat) ──────────────────
window.__cadStepGrid = function(dir) {
  const sl = document.getElementById('csp-grid-step-slider');
  const cur = sl ? parseInt(sl.value) || 10 : 10;
  const steps = [1, 2, 5, 10, 25, 50, 100];
  let idx = steps.findIndex(s => s >= cur);
  if (idx < 0) idx = steps.length - 1;
  window.__cadSetGrid(steps[Math.min(Math.max(idx + dir, 0), steps.length - 1)]);
};

window.__cadResetGrid = function() {
  window.__cadSetGrid(10);
};

// ── Sync snap tab checkboxes → sketchState.precision ───
window.__cadPanelSyncSnap = function() {
  const pr = sketchState && sketchState.precision;
  if (!pr) return;
  const set = (id, val) => { const el = document.getElementById(id); if (el) el.checked = !!val; };
  set('csp-snap-to-grid',  pr.gridSnap !== false);
  set('csp-snap-to-point', pr.pointSnap !== false);
  set('csp-snap-endpoint', pr.pointSnap !== false);
  set('csp-snap-midpoint', !!pr.midSnap);
  set('csp-snap-ortho',    !!(sketchState.orthoLock));
};

// ── Wire snap checkboxes → sketchState + si-* checkboxes ──
(function() {
  function _wire(id, onchange) {
    const el = document.getElementById(id);
    if (el) el.addEventListener('change', onchange);
  }
  _wire('csp-snap-to-grid',  e => {
    if (sketchState && sketchState.precision) sketchState.precision.gridSnap  = e.target.checked;
    const si = document.getElementById('si-snap-grid'); if (si) si.checked = e.target.checked;
  });
  _wire('csp-snap-to-point', e => {
    if (sketchState && sketchState.precision) sketchState.precision.pointSnap = e.target.checked;
    const si = document.getElementById('si-snap-point'); if (si) si.checked = e.target.checked;
  });
  _wire('csp-snap-midpoint', e => {
    if (sketchState && sketchState.precision) sketchState.precision.midSnap   = e.target.checked;
    const si = document.getElementById('si-snap-mid'); if (si) si.checked = e.target.checked;
  });
  _wire('csp-snap-ortho', e => {
    if (e.target.checked !== !!(sketchState && sketchState.orthoLock)) {
      if (window.__toggleOrthoLock) window.__toggleOrthoLock();
    }
  });
  // Grid slider ↔ number input sync
  const cspSlider = document.getElementById('csp-grid-step-slider');
  const cspNum    = document.getElementById('csp-grid-step-num');
  if (cspSlider) cspSlider.addEventListener('input', () => {
    window.__cadSetGrid(parseInt(cspSlider.value));
  });
  if (cspNum) cspNum.addEventListener('change', () => {
    window.__cadSetGrid(parseInt(cspNum.value));
  });
  // Show grid checkbox
  _wire('csp-show-grid', e => {
    if (sketchState && sketchState.precision) sketchState.precision.showGrid = e.target.checked;
  });
})();

// ── Live update: Object tab selection info ──────────
window.__cadPanelUpdateSelection = function() {
  const ss = sketchState;
  if (!ss) return;
  const fmtL = window.__fmtLength || (v => (v * 1000).toFixed(1) + ' mm');
  let type = '—', len = '—', angle = '—';
  const plane = ss.workingPlane || 'XZ';

  if (ss.selectedEdgeIds && ss.selectedEdgeIds.length === 1) {
    const eid = ss.selectedEdgeIds[0];
    const edge = ss.edges && ss.edges.find(e => e.id === eid);
    if (edge) {
      const a = ss.points && ss.points.find(p => p.id === edge.a);
      const b = ss.points && ss.points.find(p => p.id === edge.b);
      type = 'Edge';
      if (a && b) {
        const l = Math.hypot(b.x - a.x, b.y - a.y, b.z - a.z);
        len = fmtL(l);
        const dx = b.x - a.x, dz = b.z - a.z;
        angle = (Math.atan2(Math.abs(dz), Math.abs(dx)) * 180 / Math.PI).toFixed(1) + '°';
      }
    }
  } else if (ss.selectedPointIds && ss.selectedPointIds.length === 1) {
    type = 'Point';
  } else if ((ss.selectedPointIds && ss.selectedPointIds.length > 1) ||
             (ss.selectedEdgeIds  && ss.selectedEdgeIds.length  > 1)) {
    type = 'Multi';
  }

  const set = (id, v) => { const el = document.getElementById(id); if (el) el.textContent = v; };
  set('csp-sel-type',  type);
  set('csp-sel-len',   len);
  set('csp-sel-angle', angle);
  set('csp-sel-plane', plane);
};

// ── Live update: Dev tab engine status ──────────────
window.__cadPanelUpdateDev = function() {
  const ss = sketchState;
  if (!ss) return;
  const set = (id, v) => { const el = document.getElementById(id); if (el) el.textContent = v; };
  // Mirror from existing si-* elements
  const mirror = (from, to) => {
    const src = document.getElementById(from);
    const dst = document.getElementById(to);
    if (src && dst) dst.textContent = src.textContent;
  };
  mirror('si-wasm-status', 'csp-wasm-status');
  mirror('si-sync-status', 'csp-be-status');
  mirror('si-last-wasm-ms','csp-wasm-ms');
  mirror('si-last-be-ms',  'csp-be-ms');
  mirror('si-cad-pending', 'csp-pending');

  // Snap debug (Dev tab)
  if (ss.snap) {
    const k = ss.snap.kind || '—';
    set('csp-snap-kind',  k);
    set('csp-snap-gxyz',  ss.snap.gx + ' / ' + ss.snap.gy + ' / ' + ss.snap.gz);
  }
  if (ss.hoverWorld) {
    const hw = ss.hoverWorld;
    const f2 = v => Number(v).toFixed(2);
    set('csp-snap-world', f2(hw.x) + ' / ' + f2(hw.y) + ' / ' + f2(hw.z));
  }

  // Ortho checkbox sync
  const chk = document.getElementById('csp-snap-ortho');
  if (chk) chk.checked = !!(ss.orthoLock);
};

// ── Tick: called from render loop (or requestAnimationFrame) ──
window.__cadPanelTick = function() {
  const tab = window.__cadPanel.activeTab;
  if (tab === 'object') window.__cadPanelUpdateSelection();
  if (tab === 'dev')    window.__cadPanelUpdateDev();
};

// ── Init: apply default grid step once sketchState is ready ──
(function _initGrid() {
  if (window.sketchState && sketchState.precision) {
    window.__cadSetGrid(10); // 10 mm default
  } else {
    setTimeout(_initGrid, 200);
  }
})();

// ── Helper guide toggles (VIEW tab) ─────────────────
(function _initHelperGuideToggles() {
  function _wire() {
    const orbitChk   = document.getElementById('csp-show-orbit-guide');
    const projChk    = document.getElementById('csp-show-projection-guide');
    const fadeChk    = document.getElementById('csp-fade-bg-helpers');
    if (!orbitChk || !projChk || !fadeChk) { setTimeout(_wire, 200); return; }

    // Sync initial state → DOM
    orbitChk.checked = !!window.__showOrbitGuide;
    projChk.checked  = !!window.__showProjectionGuide;
    fadeChk.checked  = !!window.__fadeBackgroundHelpers;

    orbitChk.addEventListener('change', () => {
      window.__showOrbitGuide = orbitChk.checked;
    });
    projChk.addEventListener('change', () => {
      window.__showProjectionGuide = projChk.checked;
      // Also sync underlying sketchState projection.showGuides
      if (window.sketchState && window.sketchState.projection) {
        window.sketchState.projection.showGuides = projChk.checked;
      }
    });
    fadeChk.addEventListener('change', () => {
      window.__fadeBackgroundHelpers = fadeChk.checked;
    });
  }
  _wire();
})();
"#;
