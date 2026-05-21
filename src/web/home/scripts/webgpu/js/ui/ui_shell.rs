// ── UI Shell v1 — central facade window.CAD.ui ──────────────────────────────
//
// Goals:
//   * Provide a single, defensive namespace `window.CAD.ui` that every new UI
//     module (top_bar, bottom_toolbar, right_inspector, scene_tree, dev_mode)
//     can rely on without poking sketchState / geometryState directly.
//   * Stay 100 % additive: never overwrite globals, only read or wrap.
//   * Tiny event-bus so panels re-render on tool / selection / dev changes.
//
// All getters are SAFE: if the underlying state is missing they return a
// neutral fallback (null / [] / 'select' / 'XZ'…). Never throws.

pub const JS: &str = r##"
(function() {
  if (window.CAD && window.CAD.ui && window.CAD.ui.__v >= 1) return;
  window.CAD = window.CAD || {};

  // ── tiny event bus ─────────────────────────────────────────────
  const _subs = Object.create(null);
  function on(ev, fn)   { (_subs[ev] = _subs[ev] || []).push(fn); return () => off(ev, fn); }
  function off(ev, fn)  { const a = _subs[ev]; if (!a) return; const i = a.indexOf(fn); if (i >= 0) a.splice(i, 1); }
  function emit(ev, p)  { const a = _subs[ev]; if (!a) return; for (const fn of a.slice()) { try { fn(p); } catch (e) { console.warn('[CAD.ui]', ev, e); } } }

  // ── selection adapter ──────────────────────────────────────────
  // Returns one of: 'none' | 'point' | 'edge' | 'face' | 'body' | 'profile' | 'sketch' | 'extrude'
  function getSelectionType() {
    const ss = window.sketchState;
    if (!ss) return 'none';

    // 1) Active extrude gizmo (preview) — highest priority.
    //    Real state lives in `window.__solidExtrudeState` (set by
    //    `tools/solid_extrude_gizmo.rs`). Fallback to legacy `ss.extrude`.
    const ex = window.__solidExtrudeState;
    if (ex && ex.active) return 'extrude';
    if (ss.extrude && (ss.extrude.active || ss.extrude.preview)) return 'extrude';

    // 2) Solid body
    const bodyIds = ss.selectedBodyIds;
    if (bodyIds && (bodyIds.size ? bodyIds.size : (bodyIds.length || 0)) > 0) return 'body';

    // 3) Solid face
    const faceIds = ss.selectedFaceIds;
    if (faceIds && (faceIds.size ? faceIds.size : (faceIds.length || 0)) > 0) return 'face';

    // 4) Closed profile
    if (ss.selectedProfileId != null) return 'profile';

    // 5) Single sketch edge / point
    const eIds = ss.selectedEdgeIds;
    if (eIds && (eIds.size ? eIds.size : (eIds.length || 0)) > 0) return 'edge';
    const pIds = ss.selectedPointIds;
    if (pIds && (pIds.size ? pIds.size : (pIds.length || 0)) > 0) return 'point';

    return 'none';
  }

  function _sizeOf(s) { return !s ? 0 : (s.size != null ? s.size : (s.length || 0)); }
  function _firstOf(s) {
    if (!s) return null;
    if (s.size != null) { for (const v of s) return v; return null; }
    return (s[0] != null ? s[0] : null);
  }

  // Build a structured payload the right inspector can render.
  function getSelectionData() {
    const ss = window.sketchState;
    const type = getSelectionType();
    const out = { type, plane: (ss && ss.workingPlane) || 'XZ' };

    if (!ss) return out;

    switch (type) {
      case 'extrude': {
        // Real state from solid_extrude_gizmo.rs; fallback to legacy ss.extrude.
        const xs = window.__solidExtrudeState;
        const ex = (xs && xs.active) ? xs : (ss.extrude || {});
        out.feature = {
          id:        ex.id || ex.featureId || 'extrude_preview',
          depthMm:   typeof ex.depthMm === 'number' ? ex.depthMm
                   : (typeof ex.depth === 'number' ? ex.depth * 1000 : 10),
          direction: ex.direction || ex._direction || 'positive',  // positive | negative | symmetric
          operation: ex.operation || ex._operation || 'new_body',  // new_body | join | cut
          profileId: ex.profileId || ss.selectedProfileId || null,
          plane:     ex.plane     || ss.workingPlane || 'XZ',
        };
        break;
      }
      case 'body': {
        const id = _firstOf(ss.selectedBodyIds);
        // Prefer document store, fallback to sketchState.bodies.
        const docBody = (window.CAD && window.CAD.document)
                      ? window.CAD.document.findBody(id) : null;
        const body = docBody || (ss.bodies || []).find(b => b.id === id) || {};
        const docSel = (window.CAD && window.CAD.document && window.CAD.document.selection) || null;
        const selMeta = (docSel && docSel.type === 'body' && docSel.id === id) ? (docSel.meta || null) : null;
        out.body = {
          id,
          type:    body.type || 'solid',
          source:  body.sourceFeatureId || body.source || '—',
          vertCount: body.vertexCount || (body.mesh && body.mesh.vertices ? body.mesh.vertices.length / 3 : 0),
          faceCount: body.faceCount   || (body.faces ? body.faces.length : 0),
          visible: body.visible !== false,
          selectedFaceId:       selMeta ? selMeta.faceId       : null,
          selectedGlobalFaceId: selMeta ? selMeta.globalFaceId : null,
          selectedSourceFaceId: selMeta ? selMeta.sourceFaceId : null,
        };
        break;
      }
      case 'face': {
        const id = _firstOf(ss.selectedFaceIds);
        out.face = { id };
        break;
      }
      case 'profile': {
        const pid = ss.selectedProfileId;
        const prof = (ss.profiles || []).find(p => p.id === pid) || {};
        out.profile = {
          id: pid,
          closed: prof.closed !== false,
          plane:  prof.plane || ss.workingPlane || 'XZ',
          area:   typeof prof.areaMm2 === 'number' ? prof.areaMm2 : (typeof prof.area === 'number' ? prof.area : null),
          edgeCount: (prof.edgeIds && prof.edgeIds.length) || (prof.edges && prof.edges.length) || 0,
        };
        break;
      }
      case 'edge': {
        const id = _firstOf(ss.selectedEdgeIds);
        const e = (ss.edges || []).find(x => x.id === id);
        out.edge = { id, count: _sizeOf(ss.selectedEdgeIds), info: e ? { a: e.a, b: e.b } : null };
        break;
      }
      case 'point': {
        const id = _firstOf(ss.selectedPointIds);
        out.point = { id, count: _sizeOf(ss.selectedPointIds) };
        break;
      }
      default: break;
    }

    // Sketch summary always available
    out.sketch = {
      id:       ss.sketchId || 'sketch_active',
      plane:    ss.workingPlane || 'XZ',
      points:   (ss.points  || []).length,
      edges:    (ss.edges   || []).length,
      profiles: (ss.profiles|| []).length,
      visible:  ss.visible !== false,
    };

    return out;
  }

  // ── scene tree adapter ─────────────────────────────────────────
  // Always returns an array, even if state is empty.
  function getSceneTreeData() {
    const ss = window.sketchState || {};
    const docu = (window.CAD && window.CAD.document) ? window.CAD.document : null;
    const nodes = [];

    // Active sketch (single sketch in current state model)
    const selProfile = ss.selectedProfileId != null;
    nodes.push({
      id:       'sketch_active',
      type:     'sketch',
      name:     'Sketch (' + (ss.workingPlane || 'XZ') + ')',
      visible:  ss.visible !== false,
      selected: !selProfile && _sizeOf(ss.selectedBodyIds) === 0,
    });

    // Profiles — prefer document store (clean snapshot), fallback to live.
    const profiles = docu ? docu.profiles : (ss.profiles || []);
    profiles.forEach((p, i) => {
      nodes.push({
        id:       'profile_' + (p.id != null ? p.id : i),
        type:     'profile',
        name:     'Profile #' + (p.id != null ? p.id : i) + (p.closed === false ? ' (open)' : ''),
        visible:  p.visible !== false,
        selected: ss.selectedProfileId === p.id,
      });
    });

    // Bodies — prefer document store (persisted across syncs).
    const bodies = docu ? docu.bodies : (ss.bodies || []);
    bodies.forEach((b, i) => {
      const id = b.id != null ? b.id : i;
      const sel = ss.selectedBodyIds && (ss.selectedBodyIds.has ? ss.selectedBodyIds.has(id) : ss.selectedBodyIds.indexOf(id) >= 0);
      nodes.push({
        id:       String(id).startsWith('body_') ? String(id) : ('body_' + id),
        type:     'body',
        name:     b.name || ('Body #' + id),
        visible:  b.visible !== false,
        selected: !!sel,
      });
    });

    // Features (parametric history — populated by document.recordExtrude).
    const features = docu ? docu.features : (ss.features || []);
    features.forEach((f, i) => {
      nodes.push({
        id:       String(f.id || '').startsWith('extrude_') || String(f.id || '').startsWith('feature_')
                   ? String(f.id) : ('feature_' + (f.id != null ? f.id : i)),
        type:     'feature',
        name:     f.name || (f.type ? (f.type + ' #' + i) : ('Feature #' + i)),
        visible:  f.suppressed !== true,
        selected: false,
      });
    });

    return nodes;
  }

  // ── ambient state getters ──────────────────────────────────────
  function getActiveTool() { return (window.sketchState && window.sketchState.activeTool) || 'select'; }
  function getMode()       {
    const ss = window.sketchState;
    if (!ss) return 'free3d';
    const ex = window.__solidExtrudeState;
    if (ex && ex.active) return 'solid_edit';
    if (ss.extrude && (ss.extrude.active || ss.extrude.preview)) return 'solid_edit';
    return 'sketch';
  }
  function getPlane()      { return (window.sketchState && window.sketchState.workingPlane) || 'XZ'; }
  function getOrthoLock()  { return !!(window.sketchState && window.sketchState.orthoLock); }
  function getGridMm()     {
    const pr = window.sketchState && window.sketchState.precision;
    if (!pr) return 10;
    if (typeof pr.snapStepMm === 'number') return pr.snapStepMm;
    if (typeof pr.displayGridStepM === 'number') return Math.round(pr.displayGridStepM * 1000);
    return 10;
  }
  function getSnapInfo() {
    const pr = window.sketchState && window.sketchState.precision;
    if (!pr) return { on: false, label: 'off' };
    const on  = pr.gridSnap !== false || pr.pointSnap !== false;
    const parts = [];
    if (pr.gridSnap  !== false) parts.push('grid');
    if (pr.pointSnap !== false) parts.push('pt');
    if (pr.midSnap)             parts.push('mid');
    return { on, label: parts.join('+') || 'off' };
  }

  // ── dev mode flag — actual hide/show logic lives in dev_mode.rs ─
  let _devMode = false;
  function getDevMode() { return _devMode; }
  function setDevMode(on) {
    on = !!on;
    if (_devMode === on) return;
    _devMode = on;
    document.body.classList.toggle('cad-dev-mode', on);
    emit('dev:changed', on);
  }
  function toggleDevMode() { setDevMode(!_devMode); }

  // ── lightweight polling — emits selection/tool changes ─────────
  // No deep refs to sketchState internals; just snapshot a small fingerprint.
  let _last = { tool: '', sel: '', mode: '', plane: '', ortho: false, grid: 0, snap: '' };
  function _fingerprint() {
    const ss = window.sketchState || {};
    const xs = window.__solidExtrudeState || {};
    const docu = (window.CAD && window.CAD.document) ? window.CAD.document : null;
    const sel =
      getSelectionType() + '|' +
      (_firstOf(ss.selectedPointIds) || '') + '|' +
      (_firstOf(ss.selectedEdgeIds)  || '') + '|' +
      (_firstOf(ss.selectedFaceIds)  || '') + '|' +
      (_firstOf(ss.selectedBodyIds)  || '') + '|' +
      (ss.selectedProfileId || '') + '|' +
      (xs.active ? '1' : '0') + '|' + (xs.depthMm || '') + '|' +
      (docu ? (docu.bodies.length + '/' + docu.features.length) : '0/0') + '|' +
      (docu && docu.selection && docu.selection.meta ? (docu.selection.meta.globalFaceId || '') : '');
    return sel;
  }
  function _tick() {
    const tool  = getActiveTool();
    const mode  = getMode();
    const plane = getPlane();
    const ortho = getOrthoLock();
    const grid  = getGridMm();
    const snap  = getSnapInfo().label;
    const sel   = _fingerprint();
    if (tool  !== _last.tool ) { _last.tool  = tool;  emit('tool:changed',  tool);  }
    if (mode  !== _last.mode ) { _last.mode  = mode;  emit('mode:changed',  mode);  }
    if (plane !== _last.plane) { _last.plane = plane; emit('plane:changed', plane); }
    if (ortho !== _last.ortho) { _last.ortho = ortho; emit('ortho:changed', ortho); }
    if (grid  !== _last.grid ) { _last.grid  = grid;  emit('grid:changed',  grid);  }
    if (snap  !== _last.snap ) { _last.snap  = snap;  emit('snap:changed',  snap);  }
    if (sel   !== _last.sel  ) { _last.sel   = sel;   emit('selection:changed', getSelectionData()); }
  }
  setInterval(_tick, 120);   // 8 Hz — cheap & responsive
  setTimeout(_tick, 50);     // first paint

  // ── public namespace ───────────────────────────────────────────
  window.CAD.ui = {
    __v: 1,
    on, off, emit,
    // adapters
    getSelectionType, getSelectionData, getSceneTreeData,
    // ambient state
    getActiveTool, getMode, getPlane, getOrthoLock, getGridMm, getSnapInfo,
    // dev
    get devMode() { return _devMode; },
    set devMode(v) { setDevMode(v); },
    setDevMode, toggleDevMode, getDevMode,
    // helpers for other modules
    _firstOf, _sizeOf,
  };

  console.log('[CAD.ui] shell v1 ready');
})();
"##;
