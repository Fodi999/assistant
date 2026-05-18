// ── Top-Left HUD: Selection Mode ────────────────────────────────────────────
// Компактный блок в левом верхнем углу: [V] [E] [F] [B]
// Клавиши 1/2/3/4 в инструменте Select.

pub const JS: &str = r##"
  (function() {
    if (window.__selModeHudInited) return;
    window.__selModeHudInited = true;

    const MONO = 'monospace';

    function _btn(label, title) {
      const b = document.createElement('button');
      b.textContent = label;
      b.title = title || label;
      Object.assign(b.style, {
        height:       '26px',
        minWidth:     '28px',
        padding:      '0 7px',
        borderRadius: '5px',
        border:       '1px solid rgba(255,255,255,0.13)',
        background:   'rgba(255,255,255,0.07)',
        color:        'rgba(255,255,255,0.72)',
        fontSize:     '12px',
        fontWeight:   '600',
        fontFamily:   MONO,
        cursor:       'pointer',
        transition:   'background 0.12s, color 0.12s',
        outline:      'none',
        userSelect:   'none',
        lineHeight:   '1',
        flexShrink:   '0',
      });
      b.addEventListener('mouseenter', () => { if (!b._active) b.style.background = 'rgba(255,255,255,0.14)'; });
      b.addEventListener('mouseleave', () => { if (!b._active) b.style.background = 'rgba(255,255,255,0.07)'; });
      return b;
    }

    // ── Обёртка ──────────────────────────────────────────────────
    const hud = document.createElement('div');
    hud.id = '__sel-mode-hud';
    Object.assign(hud.style, {
      position:       'absolute',
      top:            '12px',
      left:           '12px',
      display:        'flex',
      flexDirection:  'row',
      gap:            '4px',
      padding:        '6px 8px',
      background:     'rgba(14,17,22,0.88)',
      borderRadius:   '9px',
      border:         '1px solid rgba(255,255,255,0.10)',
      backdropFilter: 'blur(8px)',
      zIndex:         '9998',
      pointerEvents:  'auto',
      userSelect:     'none',
    });

    // ── Кнопки режима ─────────────────────────────────────────────
    const MODES = [
      { id: 'vertex', label: 'V', title: 'Вершины  [1]' },
      { id: 'edge',   label: 'E', title: 'Рёбра    [2]' },
      { id: 'face',   label: 'F', title: 'Грани    [3]' },
      { id: 'body',   label: 'B', title: 'Тела     [4]' },
    ];
    const selBtns = {};

    MODES.forEach(m => {
      const b = _btn(m.label, m.title);
      b.addEventListener('pointerdown', e => {
        e.stopPropagation();
        e.preventDefault();
        window.__setGeomSelMode(m.id);
      });
      selBtns[m.id] = b;
      hud.appendChild(b);
    });

    // ── Монтирование ──────────────────────────────────────────────
    function _mount() {
      const canvas = document.getElementById('webgpu-canvas');
      const parent = canvas ? canvas.parentElement : document.body;
      if (parent && !parent.contains(hud)) parent.appendChild(hud);
    }
    _mount();
    setTimeout(_mount, 800);

    // ── __setGeomSelMode ──────────────────────────────────────────
    window.__setGeomSelMode = function(mode) {
      if (!window.GeomSelMode) { console.warn('[GeomSel] GeomSelMode not defined yet'); return; }
      if (!Object.values(window.GeomSelMode).includes(mode)) { console.warn('[GeomSel] unknown mode:', mode); return; }
      console.log('[GeomSel] mode →', mode);
      sketchState.geomSelMode = mode;
      if (mode === 'vertex') {
        sketchState.selectedEdgeIds.clear();
        sketchState.selectedFaceIds.clear();
        sketchState.selectedBodyIds.clear();
        sketchState.selectedProfileId = null;
      } else if (mode === 'edge') {
        sketchState.selectedFaceIds.clear();
        sketchState.selectedBodyIds.clear();
      } else if (mode === 'face') {
        sketchState.selectedPointIds.clear();
        sketchState.selectedEdgeIds.clear();
        sketchState.selectedBodyIds.clear();
      } else if (mode === 'body') {
        sketchState.selectedFaceIds.clear();
      }
      _refresh();
      if (window.__updateSketchInspector) window.__updateSketchInspector();
    };

    // ── Обновление визуала ────────────────────────────────────────
    const ACT  = { background:'rgba(255,140,30,0.88)', color:'#fff', borderColor:'rgba(255,165,55,0.9)' };
    const IDLE = { background:'rgba(255,255,255,0.07)', color:'rgba(255,255,255,0.72)', borderColor:'rgba(255,255,255,0.13)' };

    function _refresh() {
      const cur = (window.sketchState && sketchState.geomSelMode) || 'edge';
      MODES.forEach(m => {
        const b = selBtns[m.id];
        b._active = (m.id === cur);
        Object.assign(b.style, b._active ? ACT : IDLE);
      });
    }

    setInterval(_refresh, 150);
    _refresh();

  })();
"##;

