// ── JS: HUD overlay — particle stats, shape label, sliders, formation buttons ─────
// Domain: UI — heads-up display assembly, fmtN helper, updateHud.

pub const JS: &str = r##"
      // ── HUD overlay (top-right) ─────────────────────────────────
      let hud = document.getElementById('gpu-hud');
      if (!hud) {
        hud = document.createElement('div');
        hud.id = 'gpu-hud';
        hud.style.cssText = [
          'position:fixed','top:72px','right:356px','z-index:9999',
          'padding:10px 14px','border-radius:10px',
          'background:rgba(2,6,23,.82)','backdrop-filter:blur(10px)',
          '-webkit-backdrop-filter:blur(10px)',
          'border:1px solid rgba(103,232,249,.35)',
          'font:500 12px/1.5 -apple-system,SF Pro Display,system-ui,monospace',
          'color:#cbd5e1','letter-spacing:.02em','pointer-events:auto','user-select:none',
          'box-shadow:0 8px 32px rgba(0,0,0,.5)',
          'display:none',          // hidden by default — press B to reveal
        ].join(';');
        document.body.appendChild(hud);
      }
      // toggle visibility from controls
      function toggleHud() {
        hud.style.display = hud.style.display === 'none' ? 'block' : 'none';
        if (hud.style.display === 'block') updateHud(0);
      }
      window.__toggleHud = toggleHud;
      function fmtN(n) {
        if (n >= 1_000_000) return (n / 1_000_000).toFixed(n % 1_000_000 === 0 ? 0 : 1) + 'M';
        if (n >= 1_000)     return (n / 1_000).toFixed(n % 1_000 === 0 ? 0 : 1) + 'K';
        return String(n);
      }
      function updateHud(fps) {
        const density = NUM_SPHERES / CLOUD_VOLUME;     // particles per unit³
        const dStr = density >= 1000
          ? (density / 1000).toFixed(1) + 'K/u³'
          : density.toFixed(0) + '/u³';
        const r    = shape.roundness;
        const nExp = shapeExponent(r).toFixed(1);
        // r: 0 cube → 0.5 octahedron → 1 sphere  (piecewise n: 22 → 1 → 2)
        const shapeLabel =
          r >= 0.92 ? 'sphere'             :   // n ≈ 2.0
          r >= 0.65 ? 'rounded-octa'       :   // n ∈ (1.3 .. 1.85)
          r >= 0.45 ? 'octahedron'         :   // n ≈ 1.0  ← «треугольник»
          r >= 0.25 ? 'squircle'           :   // n ∈ (5 .. 12)
          r >  0.05 ? 'rounded-cube'       :   // n ∈ (12 .. 20)
                      'super-cube';            // n ≈ 22

        // ── Cell-SDF formation stats (cube only) ──
        let cellInfo = '';
        if (formation.mode === 'cube') {
          const side    = NUM_SPHERES <= 1 ? 1 : Math.max(2, Math.floor(Math.cbrt(NUM_SPHERES)));
          const surface = NUM_SPHERES <= 1 ? 1 : 6 * side * side - 12 * side + 8;       // hollow shell
          const drawn   = side * side * side;                    // solid grid
          const interior = drawn - surface;                       // culled inside
          const colorNames = ['normal','normals-RGB','mask-color'];
          cellInfo =
            `<div style="margin-top:4px;padding-top:4px;border-top:1px dashed #1e293b">` +
            `<span style="color:#94a3b8">formation</span> <b style="color:#67e8f9">Cube ${side}×${side}×${side}</b></div>` +
            `<div><span style="color:#94a3b8">render</span> <b style="color:${cellSdf.on?'#34d399':'#94a3b8'}">${cellSdf.on?'Cell SDF':'Imposter'}</b>` +
            ` <span style="color:#94a3b8">· r</span> <b style="color:#fbbf24">${cellSdf.radius.toFixed(2)}</b></div>` +
            `<div><span style="color:#94a3b8">surface</span> <b style="color:#a78bfa">${fmtN(surface)}</b>` +
            ` <span style="color:#94a3b8">· interior</span> <b style="color:#475569">${fmtN(Math.max(0, interior))}</b></div>` +
            (cellSdf.colorMode > 0 || cellSdf.hideLow
              ? `<div><span style="color:#94a3b8">debug</span> ` +
                `<b style="color:#fbbf24">${colorNames[cellSdf.colorMode]}</b>` +
                (cellSdf.hideLow ? ` <b style="color:#f0abfc">+ hide-low</b>` : '') +
                `</div>`
              : '');
        }

        hud.innerHTML =
          `<div style="color:#67e8f9;font-weight:600;letter-spacing:.06em">PARTICLE SCENE</div>` +
          `<div style="margin-top:4px"><span style="color:#94a3b8">particles</span> `+
          `<b style="color:#a78bfa">${fmtN(NUM_SPHERES)}</b>` +
          `<span style="color:#475569"> / ${fmtN(MAX_PARTICLES)}</span></div>` +
          `<div><span style="color:#94a3b8">density</span> <b style="color:#f0abfc">${dStr}</b>` +
          ` <span style="color:#94a3b8">· vol</span> <b>${CLOUD_VOLUME.toFixed(0)}u³</b></div>` +
          `<div><span style="color:#94a3b8">shape</span> <b style="color:#fbbf24">${shapeLabel}</b>` +
          ` <span style="color:#475569">n=${nExp}</span></div>` +
          `<div style="pointer-events:auto;margin:4px 0">` +
          `<input id="gpu-shape-slider" type="range" min="0" max="1" step="0.01" value="${r}" ` +
          `style="width:100%;accent-color:#fbbf24"></div>` +
          cellInfo +
          (formation.mode === 'cube'
            ? `<div style="pointer-events:auto;margin:4px 0">` +
              `<input id="gpu-cell-r" type="range" min="0" max="0.5" step="0.01" value="${cellSdf.radius}" ` +
              `style="width:100%;accent-color:#67e8f9"></div>`
            : '') +
          `<div><span style="color:#94a3b8">fps</span> <b style="color:${fps>50?'#34d399':fps>25?'#fbbf24':'#f87171'}">${fps.toFixed(0)}</b>`+
          ` <span style="color:#94a3b8">· dist</span> <b style="color:#fcd34d">${cam.dist.toFixed(2)}m</b>` +
          ` <span style="color:#94a3b8">· obj [X: ${cam.target[0].toFixed(1)}, Y: ${cam.target[1].toFixed(1)}, Z: ${cam.target[2].toFixed(1)}]</span>` +
          ` <span style="color:#94a3b8">· cam [X: ${(cam.target[0] - (-Math.sin(cam.yaw) * Math.cos(cam.pitch)) * cam.dist).toFixed(1)}, Y: ${(cam.target[1] - (-Math.sin(cam.pitch)) * cam.dist).toFixed(1)}, Z: ${(cam.target[2] - (Math.cos(cam.yaw) * Math.cos(cam.pitch)) * cam.dist).toFixed(1)}]</b></div>` +
          `<div style="margin-top:6px;display:flex;gap:4px;pointer-events:auto">` +
            ['cloud','cube','wall'].map(m =>
              `<button data-form="${m}" style="flex:1;background:${formation.mode===m?'#0e7490':'#1e293b'};` +
              `border:1px solid ${formation.mode===m?'#67e8f9':'#334155'};color:${formation.mode===m?'#ecfeff':'#cbd5e1'};` +
              `padding:4px 6px;border-radius:6px;cursor:pointer;font-size:11px;font-weight:600;text-transform:uppercase">${m}</button>`
            ).join('') +
          `</div>` +
          `<div style="margin-top:6px;display:flex;gap:6px;pointer-events:auto">` +
          `<button onclick="if(!window.__gpuBench?.running)window.__gpuRunBench?.()" style="flex:1;background:#1e293b;` +
          `border:1px solid #334155;color:#fbbf24;padding:4px 8px;border-radius:6px;` +
          `cursor:pointer;font-size:11px;font-weight:600">🔬 BENCH (B)</button></div>` +
          `<div style="margin-top:4px;color:#64748b;font-size:11px">`+
          `1-5 count · C/V/W form · S cellSDF · [/] r · N normals · M mask · I hide-low · F preset · Shift+B bench · B hide</div>`;
        // wire the slider after each rebuild
        const slider = document.getElementById('gpu-shape-slider');
        if (slider) slider.oninput = (e) => { shape.roundness = parseFloat(e.target.value); };
        const cellSlider = document.getElementById('gpu-cell-r');
        if (cellSlider) cellSlider.oninput = (e) => { cellSdf.radius = parseFloat(e.target.value); };
        // wire formation buttons
        hud.querySelectorAll('button[data-form]').forEach(b => {
          b.onclick = () => setFormation(b.dataset.form);
        });
      }
      // do NOT call updateHud on startup — HUD is hidden until user presses B
"##;
