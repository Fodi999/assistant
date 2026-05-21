// ── Debug face overlay ───────────────────────────────────────────────────
//
//  Bottom-left panel showing the logical face decomposition of the current
//  solid. Subscribes to the selection store so the highlighted entry tracks
//  the selected/hovered face automatically.
//
//  API:
//    window.CadInteraction.overlays.debug.show()
//    window.CadInteraction.overlays.debug.hide()
//    window.CadInteraction.overlays.debug.setFaces(faces)
//    window.CadInteraction.overlays.debug.toggle()

pub const JS: &str = r##"
(function registerDebugOverlay() {
  var sel = window.CadInteraction && window.CadInteraction.selection;
  if (!sel) return;

  var _faces = [];

  function _ensureEl() {
    var el = document.getElementById('__cad_face_overlay');
    if (el) return el;

    var T = window.__modalTheme;
    var C = T ? T.COLORS : { panel: '#1e293b', border: 'rgba(56,189,248,.3)', input: '#f1f5f9' };
    var L = T ? T.LAYOUT : { font: "'JetBrains Mono',monospace", borderRadius: '8px' };

    el = document.createElement('div');
    el.id = '__cad_face_overlay';
    Object.assign(el.style, {
      display:      'none',
      position:     'fixed',
      left:         '20px',
      top:          '50%',
      transform:    'translateY(-50%)',
      zIndex:       '10030',
      background:   C.panel,
      border:       '1px solid ' + C.border,
      borderRadius: L.borderRadius,
      padding:      '10px 12px',
      fontFamily:   L.font,
      fontSize:     '11px',
      color:        C.input,
      minWidth:     '190px',
      boxShadow:    '0 4px 20px rgba(0,0,0,.6)',
      userSelect:   'none',
    });

    el.innerHTML =
      '<div style="font-size:10px;letter-spacing:.6px;text-transform:uppercase;' +
        'color:rgba(56,189,248,.7);margin-bottom:6px;">🧱 Face Debug</div>' +
      '<div id="__cfo_count" style="color:#94a3b8;margin-bottom:3px;">Faces: –</div>' +
      '<div id="__cfo_sel"   style="color:#38bdf8;margin-bottom:4px;">Selected: (none)</div>' +
      '<div style="border-top:1px solid rgba(56,189,248,.15);padding-top:6px;" id="__cfo_list"></div>' +
      '<div style="margin-top:6px;">' +
        '<button id="__cfo_close" style="width:100%;padding:4px 0;font-family:inherit;' +
          'font-size:10px;background:rgba(255,255,255,.06);border:1px solid rgba(56,189,248,.2);' +
          'border-radius:4px;color:#64748b;cursor:pointer;">✕ close</button>' +
      '</div>';

    document.body.appendChild(el);
    el.querySelector('#__cfo_close').onclick = function() {
      window.CadInteraction.overlays.debug.hide();
    };
    return el;
  }

  function _render(snap) {
    var el = document.getElementById('__cad_face_overlay');
    if (!el || el.style.display === 'none') return;

    var activeId = snap.hoverFaceId || snap.faceId || null;
    var countEl  = document.getElementById('__cfo_count');
    var selEl    = document.getElementById('__cfo_sel');
    var listEl   = document.getElementById('__cfo_list');

    if (countEl) countEl.textContent = 'Faces: ' + _faces.length;
    if (selEl)   selEl.textContent   = 'Selected: ' + (snap.faceId ? 'F' + snap.faceId : '(none)');
    if (!listEl) return;

    var html = '';
    for (var i = 0; i < _faces.length; i++) {
      var f      = _faces[i];
      var isAct  = (f.face_id === activeId);
      var bg     = isAct ? 'rgba(56,189,248,.18)' : 'transparent';
      var brd    = isAct ? 'rgba(56,189,248,.5)'  : 'transparent';
      var n      = f.normal;
      html +=
        '<div data-fid="' + f.face_id + '" style="' +
          'padding:2px 4px;border-radius:3px;cursor:pointer;' +
          'background:' + bg + ';border:1px solid ' + brd + ';margin-bottom:2px;">' +
          '<span style="color:#38bdf8;">F' + f.face_id + '</span> ' +
          '<span style="color:#64748b;font-size:10px;">' +
            'src=' + f.source_face_id + ' n=[' +
            n[0].toFixed(2) + ',' + n[1].toFixed(2) + ',' + n[2].toFixed(2) + ']' +
          '</span>' +
        '</div>';
    }
    listEl.innerHTML = html;

    listEl.querySelectorAll('[data-fid]').forEach(function(div) {
      div.addEventListener('click', function() {
        var fid  = parseInt(this.getAttribute('data-fid'));
        var face = _faces.find(function(x){ return x.face_id === fid; });
        if (!face) return;
        sel.set({ selected: true, mode: 0, faceId: face.face_id, sourceFaceId: face.source_face_id });
      });
    });
  }

  window.CadInteraction.overlays.debug = {
    show: function() {
      var el = _ensureEl();
      el.style.display = 'block';
      _render(sel.snapshot());
    },
    hide: function() {
      var el = document.getElementById('__cad_face_overlay');
      if (el) el.style.display = 'none';
    },
    toggle: function() {
      var el = document.getElementById('__cad_face_overlay');
      if (!el || el.style.display === 'none') this.show();
      else                                    this.hide();
    },
    setFaces: function(faces) {
      _faces = faces || [];
      _render(sel.snapshot());
    },
  };

  // Re-render whenever the selection store changes
  sel.subscribe(_render);

  console.log('[CadInteraction.overlays] debug overlay ready');
})();
"##;
