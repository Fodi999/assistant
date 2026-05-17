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
    shader: true, camera: false,
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

// ── Grid step stepper ──────────────────────────────
window.__cadStepGrid = function(dir) {
  const inp = document.getElementById('csp-grid-step');
  if (!inp) return;
  const steps = [0.01, 0.05, 0.1, 0.5, 1, 2, 5, 10, 25, 50, 100, 250, 500, 1000];
  const cur = parseFloat(inp.value) || 1;
  let idx = steps.findIndex(s => s >= cur);
  if (idx < 0) idx = steps.length - 1;
  const next = steps[Math.min(Math.max(idx + dir, 0), steps.length - 1)];
  inp.value = next;
  inp.dispatchEvent(new Event('change'));
};

window.__cadResetGrid = function() {
  const gs  = document.getElementById('csp-grid-step');
  const gsi = document.getElementById('csp-grid-size');
  const gm  = document.getElementById('csp-grid-major');
  if (gs)  { gs.value  = 1;   gs.dispatchEvent(new Event('change')); }
  if (gsi) { gsi.value = 120; gsi.dispatchEvent(new Event('change')); }
  if (gm)  { gm.value  = 10;  gm.dispatchEvent(new Event('change')); }
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
  // Grid step → snap step input
  const cspStep = document.getElementById('csp-grid-step');
  if (cspStep) cspStep.addEventListener('change', () => {
    const v = parseFloat(cspStep.value);
    if (!isFinite(v) || v <= 0) return;
    const siSnap = document.getElementById('si-snap-step');
    if (siSnap) { siSnap.value = v; siSnap.dispatchEvent(new Event('change')); }
    if (sketchState && sketchState.precision) sketchState.precision.snapStepMm = v;
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
"#;
