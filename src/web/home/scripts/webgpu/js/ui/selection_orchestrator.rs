// ── CAD Selection Orchestrator ──────────────────────────────────────────────
//
//  Single source of truth for selection. Eliminates the race between
//  `tools/select_tool` and `tools/solid_face_meta`, and unifies updates
//  across:
//
//    • CadInteraction.selection      (face id, mode, hover) — shader bridge
//    • sketchState.selectedBodyIds   (legacy entity sets)
//    • CAD.document.selection        (UI shell store — Scene Tree, Inspector)
//    • window.__solidSelFaceId       (read by render_loop_ubo → ubo[60])
//    • window.__solidSelGlobalFaceId (new — globalFaceId mirror)
//    • CAD.ui.emit('selection:changed' / 'document:changed')
//
//  IMPORTANT: in multi-body mode the shader's per-vertex `cellMask`
//  attribute holds the **globalFaceId** (= bodySlot*1000 + localFaceId),
//  so the value pushed to ubo[60] must also be globalFaceId — NOT
//  localFaceId. The orchestrator enforces this invariant.
//
//  Public API (window.CAD.selection):
//    current                       — current selection snapshot
//    selectFace(hitOrGlobalFaceId) — main entry from pickers
//    selectBody(bodyId, meta?)     — programmatic body select (Scene Tree)
//    selectProfile(profileId, meta?)
//    selectSketch(sketchId, meta?)
//    clear()                       — full deselect
//    syncToCadInteraction()
//    syncToSketchState()
//    syncToDocument()
//    syncToShader()
//    emitChanged()
//
//  Debug:
//    window.CAD.debug.dumpSelection()

pub const JS: &str = r##"
(function registerSelectionOrchestrator() {
  if (window.CAD && window.CAD.selection && window.CAD.selection.__v) return;
  window.CAD = window.CAD || {};

  // ── current state ────────────────────────────────────────────────────────
  var current = {
    type:         'none',  // 'none' | 'body' | 'face' | 'profile' | 'sketch'
    bodyId:       null,
    featureId:    null,
    localFaceId:  null,
    globalFaceId: null,
    sourceFaceId: null,
    profileId:    null,
    sketchId:     null,
    point:        null,
  };

  function _reset() {
    current.type         = 'none';
    current.bodyId       = null;
    current.featureId    = null;
    current.localFaceId  = null;
    current.globalFaceId = null;
    current.sourceFaceId = null;
    current.profileId    = null;
    current.sketchId     = null;
    current.point        = null;
  }

  // ── resolve helper ───────────────────────────────────────────────────────
  //   Accepts:
  //     • hit object from CadInteraction.picking.pickFace(): { face, t, point,
  //                bodyId, featureId, localFaceId, globalFaceId }
  //     • number (globalFaceId)
  //   Returns a normalised descriptor or null.
  function _resolve(hitOrGfid) {
    if (hitOrGfid == null) return null;

    var gf = 0, hit = null;
    if (typeof hitOrGfid === 'number') {
      gf = hitOrGfid | 0;
    } else if (typeof hitOrGfid === 'object') {
      hit = hitOrGfid;
      gf = (hit.globalFaceId | 0) || 0;
    }

    // Try resolver first — gives bodyId / localFaceId / face from renderBodies.
    var resolved = null;
    if (gf && window.CAD && window.CAD.renderer
        && typeof window.CAD.renderer.resolveFaceId === 'function') {
      try { resolved = window.CAD.renderer.resolveFaceId(gf); } catch (_) {}
    }

    // Build descriptor merging hit + resolved.
    var d = {
      bodyId:       (hit && hit.bodyId)       || (resolved && resolved.bodyId)       || null,
      featureId:    (hit && hit.featureId)    || (resolved && resolved.featureId)    || null,
      localFaceId:  (hit && hit.localFaceId)  || (resolved && resolved.localFaceId)  || (hit && hit.face && hit.face.face_id) || 0,
      globalFaceId: gf || 0,
      sourceFaceId: (hit && hit.face && hit.face.source_face_id) || (resolved && resolved.face && resolved.face.source_face_id) || 0,
      point:        (hit && hit.point) || null,
      face:         (hit && hit.face) || (resolved && resolved.face) || null,
    };

    // If we still don't have globalFaceId but have hit data, compute fallback.
    if (!d.globalFaceId && hit && d.bodyId) {
      // Find slot in renderBodies → slot*1000 + localFaceId
      var arr = (window.CAD && window.CAD.renderBodies) || [];
      for (var i = 0; i < arr.length; i++) {
        if (arr[i] && arr[i].id === d.bodyId) {
          d.globalFaceId = (i + 1) * 1000 + (d.localFaceId | 0);
          break;
        }
      }
    }

    if (!d.bodyId && !d.globalFaceId) return null;
    return d;
  }

  // ── individual sync layers ──────────────────────────────────────────────

  function syncToCadInteraction() {
    var CI = window.CadInteraction;
    if (!CI || !CI.selection) return;
    if (current.type === 'body' || current.type === 'face') {
      // IMPORTANT: feed shader-facing field `sourceFaceId` with globalFaceId
      // so the per-vertex cellMask (which now stores globalFaceId) matches.
      CI.selection.set({
        selected:     true,
        mode:         0,
        faceId:       current.localFaceId  | 0,
        sourceFaceId: current.globalFaceId | 0,
      });
    } else {
      CI.selection.clear();
    }
  }

  function syncToSketchState() {
    var ss = window.sketchState;
    if (!ss) return;
    if (!(ss.selectedBodyIds  instanceof Set)) ss.selectedBodyIds  = new Set();
    if (!(ss.selectedFaceIds  instanceof Set)) ss.selectedFaceIds  = new Set();
    if (!(ss.selectedEdgeIds  instanceof Set)) ss.selectedEdgeIds  = new Set();
    if (!(ss.selectedPointIds instanceof Set)) ss.selectedPointIds = new Set();

    ss.selectedBodyIds.clear();
    ss.selectedFaceIds.clear();
    ss.selectedEdgeIds.clear();
    ss.selectedPointIds.clear();
    ss.selectedProfileId = null;

    if (current.type === 'body' || current.type === 'face') {
      if (current.bodyId) ss.selectedBodyIds.add(current.bodyId);
    } else if (current.type === 'profile') {
      ss.selectedProfileId = current.profileId;
    }
  }

  function syncToDocument() {
    var doc = window.CAD && window.CAD.document;
    if (!doc || typeof doc.setSelection !== 'function') return;
    if (current.type === 'body' || current.type === 'face') {
      doc.setSelection('body', current.bodyId, {
        faceId:       current.localFaceId,
        globalFaceId: current.globalFaceId,
        sourceFaceId: current.globalFaceId, // unified: shader-id, not kernel-id
        kernelFaceId: current.sourceFaceId, // keep raw kernel face_id under separate key
        featureId:    current.featureId,
        point:        current.point,
      });
    } else if (current.type === 'profile') {
      doc.setSelection('profile', current.profileId, null);
    } else if (current.type === 'sketch') {
      doc.setSelection('sketch', current.sketchId, null);
    } else {
      doc.setSelection('none', null);
    }
  }

  function syncToShader() {
    var gf = current.globalFaceId | 0;
    window.__solidSelGlobalFaceId = gf;
    // Legacy global — render_loop_ubo writes ubo[60] = __solidSelFaceId.
    // Now it carries globalFaceId so it matches the per-vertex cellMask.
    window.__solidSelFaceId       = gf;
    window.__solidSelected        = (current.type === 'body' || current.type === 'face');
    window.__solidSelMode         = 0;
  }

  function emitChanged() {
    try {
      if (window.CAD && window.CAD.ui && typeof window.CAD.ui.emit === 'function') {
        var payload = { selection: Object.assign({}, current) };
        window.CAD.ui.emit('selection:changed', payload);
        window.CAD.ui.emit('document:changed',  { kind: 'selection', selection: payload.selection });
      }
    } catch (e) {
      // CAD.ui might not exist yet during early init — silently skip.
    }
  }

  function _applyAll() {
    syncToCadInteraction();
    syncToSketchState();
    syncToDocument();
    syncToShader();
    emitChanged();
  }

  // ── public selectors ─────────────────────────────────────────────────────

  function selectFace(hitOrGfid) {
    var d = _resolve(hitOrGfid);
    if (!d) { clear(); return null; }

    _reset();
    current.type         = 'body'; // UI treats whole-body selection; meta holds face
    current.bodyId       = d.bodyId;
    current.featureId    = d.featureId;
    current.localFaceId  = d.localFaceId  | 0;
    current.globalFaceId = d.globalFaceId | 0;
    current.sourceFaceId = d.sourceFaceId | 0;
    current.point        = d.point || null;

    _applyAll();
    return Object.assign({}, current);
  }

  function selectBody(bodyId, meta) {
    if (!bodyId) { clear(); return null; }
    _reset();
    current.type      = 'body';
    current.bodyId    = bodyId;
    current.featureId = (meta && meta.featureId) || null;
    if (meta) {
      if (meta.localFaceId  != null) current.localFaceId  = meta.localFaceId  | 0;
      if (meta.globalFaceId != null) current.globalFaceId = meta.globalFaceId | 0;
      if (meta.sourceFaceId != null) current.sourceFaceId = meta.sourceFaceId | 0;
      if (meta.point        != null) current.point        = meta.point;
    }
    _applyAll();
    return Object.assign({}, current);
  }

  function selectProfile(profileId, meta) {
    if (!profileId) { clear(); return null; }
    _reset();
    current.type      = 'profile';
    current.profileId = profileId;
    if (meta && meta.sketchId) current.sketchId = meta.sketchId;
    _applyAll();
    return Object.assign({}, current);
  }

  function selectSketch(sketchId, meta) {
    if (!sketchId) { clear(); return null; }
    _reset();
    current.type     = 'sketch';
    current.sketchId = sketchId;
    _applyAll();
    return Object.assign({}, current);
  }

  function clear() {
    _reset();
    _applyAll();
  }

  // ── debug ────────────────────────────────────────────────────────────────
  window.CAD.debug = window.CAD.debug || {};
  window.CAD.debug.dumpSelection = function() {
    var CI  = window.CadInteraction && window.CadInteraction.selection;
    var doc = window.CAD.document;
    var ss  = window.sketchState;
    var info = {
      'CAD.selection.current': Object.assign({}, current),
      'CadInteraction.selection.snapshot': CI ? CI.snapshot() : null,
      'sketchState.selectedBodyIds': ss && ss.selectedBodyIds
        ? Array.from(ss.selectedBodyIds) : [],
      'CAD.document.selection': doc ? doc.selection : null,
      '__solidSelFaceId':       window.__solidSelFaceId       || 0,
      '__solidSelGlobalFaceId': window.__solidSelGlobalFaceId || 0,
    };
    try { console.table(info); } catch (_) { console.log(info); }
    return info;
  };

  // ── publish ──────────────────────────────────────────────────────────────
  window.CAD.selection = {
    __v: 1,
    current: current,
    selectFace:           selectFace,
    selectBody:           selectBody,
    selectProfile:        selectProfile,
    selectSketch:         selectSketch,
    clear:                clear,
    syncToCadInteraction: syncToCadInteraction,
    syncToSketchState:    syncToSketchState,
    syncToDocument:       syncToDocument,
    syncToShader:         syncToShader,
    emitChanged:          emitChanged,
  };

  console.log('[CAD.selection] orchestrator v1 ready');
})();
"##;
