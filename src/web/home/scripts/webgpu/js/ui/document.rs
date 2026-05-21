// ── CAD Document Store — minimal adapter ────────────────────────────────────
//
//  Provides `window.CAD.document = { sketches, profiles, bodies, features, selection }`.
//
//  Strategy:
//   * `sketches` and `profiles` are **mirrored** every 250 ms from live
//     `window.sketchState` (single active sketch model).
//   * `bodies` and `features` are **accumulated** by wrapping the real
//     `window.__commitSolidExtrude()` from `tools/solid_extrude_gizmo.rs`.
//     After a successful commit we read `window.__lastSolidResult` and push
//     a `{type:'extrude'}` feature + a `{type:'solid'}` body.
//   * Also mirrors `bodies`/`features` into `sketchState.bodies` /
//     `sketchState.features` so that any legacy reader (and the existing UI
//     Shell adapters) finds them via the usual path.
//
//  Defensive: never throws if hooks are missing, polls until gizmo loads.

pub const JS: &str = r##"
(function() {
  if (window.CAD && window.CAD.document) return;
  window.CAD = window.CAD || {};

  const doc = {
    sketches: [],
    profiles: [],
    bodies:   [],
    features: [],
    selection: { type: 'none', id: null },
    _featureSeq: 0,
    _bodySeq:    0,
  };
  window.CAD.document = doc;

  // ── Pull live sketch / profile snapshots ───────────────────────
  function sync() {
    const ss = window.sketchState || {};
    doc.sketches = [{
      id:         'sketch_active',
      plane:      ss.workingPlane || 'XZ',
      pointCount: (ss.points || []).length,
      edgeCount:  (ss.edges  || []).length,
      visible:    ss.visible !== false,
    }];
    doc.profiles = (ss.profiles || []).map(p => ({
      id:        p.id,
      plane:     p.plane || ss.workingPlane || 'XZ',
      closed:    p.closed !== false,
      edgeCount: (p.edgeIds || []).length,
      area:      (typeof p.areaMm2 === 'number') ? p.areaMm2 : null,
      visible:   true,
    }));
    // bodies / features are document-owned — never overwrite from ss
    // (but we DO mirror them into ss after recordExtrude — see below).
  }

  // ── Record a committed extrude ─────────────────────────────────
  function recordExtrude(result, params) {
    if (!result) return null;
    params = params || {};
    const fnum = ++doc._featureSeq;
    const bnum = ++doc._bodySeq;
    const fid = 'extrude_' + fnum;
    const bid = 'body_' + bnum;

    const feature = {
      id:        fid,
      type:      'extrude',
      profileId: params.profileId   || null,
      depthMm:   params.depthMm     || 0,
      plane:     params.plane       || 'XZ',
      direction: params.direction   || 'positive',
      operation: params.operation   || 'new_body',
      bodyId:    bid,
      timestamp: Date.now(),
      suppressed: false,
      name:      'Extrude #' + fnum,
    };
    const body = {
      id:                bid,
      name:              'Body #' + bnum,
      type:              'solid',
      sourceFeatureId:   fid,
      source:            result.kernel || 'geometry-kernel',
      vertexCount:       result.vertex_count || 0,
      faceCount:         result.face_count   || (result.faces ? result.faces.length : 0),
      triangleCount:     result.triangle_count || 0,
      faces:             result.faces || [],
      objData:           result.obj_data || null,
      visible:           true,
    };
    doc.features.push(feature);
    doc.bodies.push(body);

    // Promote the preview mesh (left in CAD._previewBody by the final
    // __uploadSolidToScene call inside __commitSolidExtrude) into a
    // permanent multi-body render slot. If the renderer isn't available,
    // silently skip — the legacy single-body path already drew the mesh.
    try {
      if (window.CAD && window.CAD.renderer && typeof window.CAD.renderer.commitPreviewAsBody === 'function') {
        const renderBody = window.CAD.renderer.commitPreviewAsBody(bid, fid);
        if (renderBody) {
          // Refresh logical face metadata which is populated AFTER upload
          // (CadInteraction.picking.buildFaceMetadata runs post-upload).
          if (result.faces) renderBody.faces = result.faces;
        }
      }
    } catch (e) {
      console.warn('[CAD.document] commitPreviewAsBody failed:', e);
    }

    // Mirror into sketchState so existing readers (UI shell adapters,
    // scene tree fallbacks) find them via the usual path.
    const ss = window.sketchState;
    if (ss) {
      ss.bodies   = ss.bodies   || [];
      ss.features = ss.features || [];
      ss.bodies.push(body);
      ss.features.push(feature);
    }

    console.log('[CAD.document] +extrude', feature.id, '→', body.id,
      body.vertexCount + 'v', body.triangleCount + 't');

    // Notify the rest of the UI shell.
    try { if (window.CAD && window.CAD.ui) window.CAD.ui.emit('document:changed', { kind: 'extrude', feature, body }); } catch (_) {}
    return { feature, body };
  }

  // ── Selection sink (used by scene tree) ────────────────────────
  function setSelection(type, id, meta) {
    doc.selection = { type: type || 'none', id: id || null, meta: meta || null };
    try { if (window.CAD && window.CAD.ui) window.CAD.ui.emit('document:selection', doc.selection); } catch (_) {}
  }

  // ── Find helpers ───────────────────────────────────────────────
  function findBody(id) {
    const s = String(id);
    return doc.bodies.find(b => String(b.id) === s) || null;
  }
  function findFeature(id) {
    const s = String(id);
    return doc.features.find(f => String(f.id) === s) || null;
  }

  doc.sync          = sync;
  doc.recordExtrude = recordExtrude;
  doc.setSelection  = setSelection;
  doc.findBody      = findBody;
  doc.findFeature   = findFeature;

  // ── Wrap __commitSolidExtrude to capture the committed mesh ────
  function _wrapCommit() {
    const orig = window.__commitSolidExtrude;
    if (typeof orig !== 'function') return false;
    if (orig.__wrappedByDoc) return true;
    const wrapped = async function() {
      const st = window.__solidExtrudeState || {};
      const params = {
        profileId: st.profileId,
        depthMm:   st.depthMm,
        plane:     st.plane,
        direction: st._direction || 'positive',
        operation: st._operation || 'new_body',
      };
      const before = window.__lastSolidResult;
      const r = await orig.apply(this, arguments);
      // After commit, __lastSolidResult holds the final mesh.
      const result = window.__lastSolidResult;
      // Avoid double-record if commit didn't actually update the result.
      if (result && result !== before) {
        recordExtrude(result, params);
      } else if (result) {
        // Same object but commit was final → still record.
        recordExtrude(result, params);
      }
      return r;
    };
    wrapped.__wrappedByDoc = true;
    window.__commitSolidExtrude = wrapped;
    console.log('[CAD.document] wrapped __commitSolidExtrude');
    return true;
  }

  if (!_wrapCommit()) {
    let attempts = 0;
    const iv = setInterval(() => {
      if (_wrapCommit() || ++attempts > 80) clearInterval(iv);
    }, 250);
  }

  // ── Wrap __cancelSolidExtrude so the preview body is cleared from
  //    the multi-body renderer when the user aborts the operation.
  function _wrapCancel() {
    const orig = window.__cancelSolidExtrude;
    if (typeof orig !== 'function') return false;
    if (orig.__wrappedByDoc) return true;
    const wrapped = function() {
      const r = orig.apply(this, arguments);
      try {
        if (window.CAD && window.CAD.renderer && typeof window.CAD.renderer.setPreviewBody === 'function') {
          window.CAD.renderer.setPreviewBody(null);
        }
      } catch (_) {}
      return r;
    };
    wrapped.__wrappedByDoc = true;
    window.__cancelSolidExtrude = wrapped;
    return true;
  }
  if (!_wrapCancel()) {
    let attempts = 0;
    const iv = setInterval(() => {
      if (_wrapCancel() || ++attempts > 80) clearInterval(iv);
    }, 250);
  }

  // ── Periodic sketch/profile sync ───────────────────────────────
  setTimeout(sync, 80);
  setInterval(sync, 250);

  console.log('[CAD.document] ready — sketches/profiles/bodies/features store');
})();
"##;
