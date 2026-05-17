// ── JS: HUD overlay — particle stats, shape label, sliders, formation buttons ─────
// Стили читаются из window.__modalTheme (design/tokens.rs).

pub const JS: &str = r##"
      // ── HUD overlay (top-right) ─────────────────────────────────
      const __T = window.__modalTheme;
      const __C = __T.COLORS;
      let hud = document.getElementById('gpu-hud');
      if (!hud) {
        hud = document.createElement('div');
        hud.id = 'gpu-hud';
        __T.applyHudStyle(hud, { top: '72px', right: '356px', zIndex: '9999' });
        __T.makeDraggable(hud, hud);
        __T.blockCanvasEvents(hud);
        document.body.appendChild(hud);
      }
      function toggleHud() {
        hud.style.display = hud.style.display === 'none' ? 'block' : 'none';
        if (hud.style.display === 'block') updateHud(0);
      }
      window.__toggleHud = toggleHud;
      function fmtN(n) {
        if (n >= 1000000) return (n / 1000000).toFixed(n % 1000000 === 0 ? 0 : 1) + 'M';
        if (n >= 1000)    return (n / 1000).toFixed(n % 1000 === 0 ? 0 : 1) + 'K';
        return String(n);
      }
      function updateHud(fps) {
        const density = NUM_SPHERES / CLOUD_VOLUME;
        const dStr = density >= 1000
          ? (density / 1000).toFixed(1) + 'K/u3'
          : density.toFixed(0) + '/u3';
        const r    = shape.roundness;
        const nExp = shapeExponent(r).toFixed(1);
        const shapeLabel =
          r >= 0.92 ? 'sphere'       :
          r >= 0.65 ? 'rounded-octa' :
          r >= 0.45 ? 'octahedron'   :
          r >= 0.25 ? 'squircle'     :
          r >  0.05 ? 'rounded-cube' :
                      'super cube';

        let cellInfo = '';
        if (formation.mode === 'cube') {
          const side     = NUM_SPHERES <= 1 ? 1 : Math.max(2, Math.floor(Math.cbrt(NUM_SPHERES)));
          const surface  = NUM_SPHERES <= 1 ? 1 : 6 * side * side - 12 * side + 8;
          const drawn    = side * side * side;
          const interior = drawn - surface;
          const colorNames = ['normal','normals-RGB','mask-color'];
          cellInfo =
            '<div style="margin-top:4px;padding-top:4px;border-top:1px dashed ' + __C.panelSolid + '">' +
            '<span style="color:' + __C.mute + '">formation</span> <b style="color:' + __C.info + '">Cube ' + side + 'x' + side + 'x' + side + '</b></div>' +
            '<div><span style="color:' + __C.mute + '">render</span> <b style="color:' + (cellSdf.on?__C.ok:__C.mute) + '">' + (cellSdf.on?'Cell SDF':'Imposter') + '</b>' +
            ' <span style="color:' + __C.mute + '">r</span> <b style="color:' + __C.warn + '">' + cellSdf.radius.toFixed(2) + '</b></div>' +
            '<div><span style="color:' + __C.mute + '">surface</span> <b style="color:' + __C.violet + '">' + fmtN(surface) + '</b>' +
            ' <span style="color:' + __C.mute + '">interior</span> <b style="color:' + __C.dimDark + '">' + fmtN(Math.max(0, interior)) + '</b></div>' +
            (cellSdf.colorMode > 0 || cellSdf.hideLow
              ? '<div><span style="color:' + __C.mute + '">debug</span> ' +
                '<b style="color:' + __C.warn + '">' + colorNames[cellSdf.colorMode] + '</b>' +
                (cellSdf.hideLow ? ' <b style="color:' + __C.pink + '">+hide-low</b>' : '') + '</div>'
              : '');
        }

        hud.innerHTML =
          '<div style="color:' + __C.info + ';font-weight:600;letter-spacing:.06em">PARTICLE SCENE</div>' +
          '<div style="margin-top:4px"><span style="color:' + __C.mute + '">particles</span> ' +
          '<b style="color:' + __C.violet + '">' + fmtN(NUM_SPHERES) + '</b>' +
          '<span style="color:' + __C.dimDark + '"> / ' + fmtN(MAX_PARTICLES) + '</span></div>' +
          '<div><span style="color:' + __C.mute + '">density</span> <b style="color:' + __C.pink + '">' + dStr + '</b>' +
          ' <span style="color:' + __C.mute + '">vol</span> <b>' + CLOUD_VOLUME.toFixed(0) + 'u3</b></div>' +
          '<div><span style="color:' + __C.mute + '">shape</span> <b style="color:' + __C.warn + '">' + shapeLabel + '</b>' +
          ' <span style="color:' + __C.dimDark + '">n=' + nExp + '</span></div>' +
          '<div style="pointer-events:auto;margin:4px 0">' +
          '<input id="gpu-shape-slider" type="range" min="0" max="1" step="0.01" value="' + r + '" ' +
          'style="width:100%;accent-color:' + __C.warn + '"></div>' +
          cellInfo +
          (formation.mode === 'cube'
            ? '<div style="pointer-events:auto;margin:4px 0">' +
              '<input id="gpu-cell-r" type="range" min="0" max="0.5" step="0.01" value="' + cellSdf.radius + '" ' +
              'style="width:100%;accent-color:' + __C.info + '"></div>'
            : '') +
          '<div><span style="color:' + __C.mute + '">fps</span> <b style="color:' + (fps>50?__C.ok:fps>25?__C.warn:__C.danger) + '">' + fps.toFixed(0) + '</b>' +
          ' <span style="color:' + __C.mute + '">dist</span> <b style="color:' + __C.warn2 + '">' + cam.dist.toFixed(2) + 'm</b>' +
          ' <span style="color:' + __C.mute + '">obj [X:' + cam.target[0].toFixed(1) + ' Y:' + cam.target[1].toFixed(1) + ' Z:' + cam.target[2].toFixed(1) + ']</span></div>' +
          '<div style="margin-top:6px;display:flex;gap:4px;pointer-events:auto">' +
            ['cloud','cube','wall'].map(function(m) {
              return '<button data-form="' + m + '" style="flex:1;' +
              'background:' + (formation.mode===m?__C.panelActive:__C.panelSolid) + ';' +
              'border:1px solid ' + (formation.mode===m?__C.info:__C.panelBorder) + ';' +
              'color:' + (formation.mode===m?'#ecfeff':__C.fg) + ';' +
              'padding:4px 6px;border-radius:6px;cursor:pointer;font-size:11px;font-weight:600;text-transform:uppercase">' + m + '</button>';
            }).join('') +
          '</div>' +
          '<div style="margin-top:6px;display:flex;gap:6px;pointer-events:auto">' +
          '<button onclick="if(!window.__gpuBench||!window.__gpuBench.running)window.__gpuRunBench&&window.__gpuRunBench()" style="flex:1;' +
          'background:' + __C.panelSolid + ';border:1px solid ' + __C.panelBorder + ';' +
          'color:' + __C.warn + ';padding:4px 8px;border-radius:6px;' +
          'cursor:pointer;font-size:11px;font-weight:600">BENCH (B)</button></div>' +
          '<div style="margin-top:4px;color:' + __C.dim + ';font-size:11px">' +
          '1-5 count · C/V/W form · S cellSDF · [/] r · N normals · M mask · I hide-low · F preset · B hide</div>';

        const slider = document.getElementById('gpu-shape-slider');
        if (slider) slider.oninput = function(e) { shape.roundness = parseFloat(e.target.value); };
        const cellSlider = document.getElementById('gpu-cell-r');
        if (cellSlider) cellSlider.oninput = function(e) { cellSdf.radius = parseFloat(e.target.value); };
        hud.querySelectorAll('button[data-form]').forEach(function(b) {
          b.onclick = function() { setFormation(b.dataset.form); };
        });
      }
"##;
