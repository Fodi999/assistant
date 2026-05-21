// ── Selection store ──────────────────────────────────────────────────────
//
//  Central state container for solid selection. Replaces scattered globals
//  (`window.__solidSelected`, `__solidSelFaceId`, etc.) with a single
//  observable store.
//
//  API:
//    const sel = window.CadInteraction.selection;
//
//    sel.snapshot()                         → { selected, mode, faceId, sourceFaceId, hoverFaceId, hoverSourceFaceId }
//    sel.set({ selected, mode, faceId, sourceFaceId })
//    sel.setHover({ faceId, sourceFaceId })
//    sel.clear()                            // selected = false, all 0
//    sel.subscribe(cb)                      // cb(snapshot) on every change → unsub fn
//
//  Highlight bridge syncs snapshot → legacy globals (read by render_loop_ubo
//  until UBO writer is migrated to read directly from store).

pub const JS: &str = r##"
(function registerSelectionStore() {
  var _state = {
    selected:          false,
    mode:              0,     // OBJECT
    faceId:            0,     // logical face id (1..N from face_metadata)
    sourceFaceId:      0,     // kernel face_id (for shader p.cellMask compare)
    hoverFaceId:       0,
    hoverSourceFaceId: 0,
  };
  var _listeners = [];

  function _notify() {
    var snap = Object.assign({}, _state);
    for (var i = 0; i < _listeners.length; i++) {
      try { _listeners[i](snap); } catch (e) { console.warn('[selection] listener err', e); }
    }
  }

  window.CadInteraction.selection = {
    snapshot: function() { return Object.assign({}, _state); },

    set: function(patch) {
      if (!patch) return;
      var changed = false;
      ['selected','mode','faceId','sourceFaceId'].forEach(function(k) {
        if (patch[k] !== undefined && _state[k] !== patch[k]) {
          _state[k] = patch[k];
          changed = true;
        }
      });
      if (changed) _notify();
    },

    setHover: function(patch) {
      if (!patch) return;
      var changed = false;
      var fid  = patch.faceId       !== undefined ? patch.faceId       : 0;
      var sfid = patch.sourceFaceId !== undefined ? patch.sourceFaceId : 0;
      if (_state.hoverFaceId       !== fid)  { _state.hoverFaceId       = fid;  changed = true; }
      if (_state.hoverSourceFaceId !== sfid) { _state.hoverSourceFaceId = sfid; changed = true; }
      if (changed) _notify();
    },

    clear: function() {
      _state.selected          = false;
      _state.mode              = 0;
      _state.faceId            = 0;
      _state.sourceFaceId      = 0;
      _state.hoverFaceId       = 0;
      _state.hoverSourceFaceId = 0;
      _notify();
    },

    subscribe: function(cb) {
      if (typeof cb !== 'function') return function(){};
      _listeners.push(cb);
      // Fire once with current state so subscribers init correctly
      try { cb(Object.assign({}, _state)); } catch (e) {}
      return function unsub() {
        var i = _listeners.indexOf(cb);
        if (i >= 0) _listeners.splice(i, 1);
      };
    },
  };

  console.log('[CadInteraction.selection] store ready');
})();
"##;
