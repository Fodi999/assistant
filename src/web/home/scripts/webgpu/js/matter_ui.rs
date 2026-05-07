// ── JS: Matter Lab UI binding — sync state ↔ DOM, action handlers ────────────
// Domain: Presentation — wires DOM events of the new Matter Lab panels into
// existing engine objects (cam, shape, formation, cellSdf, setParticleCount).

pub const JS: &str = r##"
      // ── 8b. Matter Lab UI ───────────────────────────────────────
      const FORMATIONS = ['cube', 'wall', 'cloud'];
      const SHAPES     = ['super-cube', 'octa', 'super-sphere'];

      // map UI shape → shape.roundness used by the WGSL pipeline
      function shapeToRoundness(s) {
        if (s === 'super-sphere') return 0.0;
        if (s === 'octa')         return 0.5;
        return 1.0;                   // super-cube
      }
      function roundnessToShape(r) {
        if (r < 0.25) return 'super-sphere';
        if (r < 0.75) return 'octa';
        return 'super-cube';
      }

      // ── DOM sync ──
      function syncMatterUi() {
        const m = engineState.matter;

        // mirror engine → state
        m.formation  = formation.mode;
        m.shape      = roundnessToShape(shape.roundness);
        m.particlesM = +(NUM_SPHERES / 1_000_000).toFixed(2);
        recomputeMatterCounts();
        engineState.performance.fps     = __matterPerf.fps;
        engineState.performance.frameMs = __matterPerf.frameMs;

        const setText = (id, v) => { const el = document.getElementById(id); if (el) el.textContent = v; };

        setText('particlesValue', fmtMillions(m.particlesM));
        const slider = document.getElementById('particleSlider');
        if (slider) {
          slider.max   = m.maxParticlesM;
          slider.value = Math.max(1, Math.round(m.particlesM));
        }

        setText('formationValue', capitalize(m.formation));
        setText('shapeValue',     capitalizeWords(m.shape));

        setText('densityValue',  m.density.toFixed(1) + 'k/u³');
        setText('noiseValue',    m.noise.toFixed(1));
        setText('cohesionValue', m.cohesion.toFixed(2));
        setText('surfaceValue',  fmtThousands(m.surface));
        setText('interiorValue', fmtThousands(m.interior));

        setText('currentTool',     capitalize(engineState.tool) + ' mode');
        setText('selectedAction',  describeAction(engineState.action.selected));

        setText('fpsValue',       engineState.performance.fps.toFixed(0));
        setText('frameValue',     engineState.performance.frameMs.toFixed(1) + 'ms');
        setText('particlesHud',   fmtMillions(m.particlesM));
      }

      function describeAction(a) {
        switch (a) {
          case 'configure-formation': return 'Configure matter formation';
          case 'compress':            return 'Compressing structure';
          case 'expand':              return 'Expanding structure';
          case 'scatter':             return 'Scattering particles';
          case 'smooth':              return 'Smoothing morphology';
          case 'freeze':              return engineState.matter.frozen ? 'Frozen · paused' : 'Unfrozen · live';
          case 'reset':               return 'Reset to defaults';
          default:                    return '—';
        }
      }

      // ── tools ──
      function setTool(tool) {
        engineState.tool = tool;
        document.querySelectorAll('.tool-btn').forEach(b => {
          b.classList.toggle('active', b.dataset.tool === tool);
        });
        syncMatterUi();
      }

      // ── action handlers ──
      function runMatterAction(name) {
        const m = engineState.matter;
        engineState.action.selected = name;
        engineState.action.last     = name;
        switch (name) {
          case 'compress':
            m.density  = Math.min(30, m.density + 0.4);
            m.cohesion = Math.min(1,  m.cohesion + 0.05);
            cellSdf.radius = Math.max(0.05, cellSdf.radius - 0.02);
            break;
          case 'expand':
            m.density  = Math.max(0.1, m.density - 0.4);
            m.cohesion = Math.max(0,   m.cohesion - 0.05);
            cellSdf.radius = Math.min(0.5, cellSdf.radius + 0.02);
            break;
          case 'scatter':
            setFormation('cloud');
            m.noise = Math.min(50, m.noise + 8);
            break;
          case 'smooth':
            m.noise = Math.max(0, m.noise - 6);
            break;
          case 'freeze':
            m.frozen = !m.frozen;
            cam.autoRotate = !m.frozen ? cam.autoRotate : false;
            break;
          case 'reset':
            resetMatter();
            break;
        }
        syncMatterUi();
      }

      function resetMatter() {
        setParticleCount(1_000_000);
        setFormation('cube');
        shape.roundness = 1.0;
        cellSdf.on = false;
        cellSdf.radius = 0.25;
        const m = engineState.matter;
        m.density = 1.4; m.noise = 22.0; m.cohesion = 0.75; m.frozen = false;
      }

      // ── bind once on startup ──
      function bindMatterUi() {
        // tool buttons
        document.querySelectorAll('.tool-btn').forEach(btn => {
          btn.addEventListener('click', () => setTool(btn.dataset.tool));
        });

        // action buttons
        document.querySelectorAll('.action-bar button[data-action]').forEach(btn => {
          btn.addEventListener('click', () => runMatterAction(btn.dataset.action));
        });

        // particle slider
        const slider = document.getElementById('particleSlider');
        if (slider) {
          slider.addEventListener('input', e => {
            const m = +e.target.value;
            setParticleCount(m * 1_000_000);
            syncMatterUi();
          });
        }

        // formation cycle (click row)
        const formRow = document.querySelector('[data-cycle="formation"]');
        if (formRow) {
          formRow.addEventListener('click', () => {
            const cur = FORMATIONS.indexOf(formation.mode);
            const nxt = FORMATIONS[(cur + 1) % FORMATIONS.length];
            setFormation(nxt);
            syncMatterUi();
          });
        }

        // shape cycle (click row)
        const shapeRow = document.querySelector('[data-cycle="shape"]');
        if (shapeRow) {
          shapeRow.addEventListener('click', () => {
            const cur = SHAPES.indexOf(roundnessToShape(shape.roundness));
            const nxt = SHAPES[(cur + 1) % SHAPES.length];
            shape.roundness = shapeToRoundness(nxt);
            syncMatterUi();
          });
        }

        // grid scale buttons
        const gridScaleBtns = document.querySelectorAll('#ui-grid-scale button');
        if (gridScaleBtns.length > 0) {
          gridScaleBtns.forEach(btn => {
            btn.addEventListener('click', () => {
              gridScaleBtns.forEach(b => b.classList.remove('active'));
              btn.classList.add('active');
              
              const val = btn.dataset.val;
              if (val === 'mm') floorGrid.scale = 1000.0;
              if (val === 'cm') floorGrid.scale = 100.0;
              if (val === 'm')  floorGrid.scale = 1.0;
            });
          });
        }

        // particles scale buttons
        const particleScaleBtns = document.querySelectorAll('#ui-particles-scale button');
        if (particleScaleBtns.length > 0) {
          particleScaleBtns.forEach(btn => {
            btn.addEventListener('click', () => {
              particleScaleBtns.forEach(b => b.classList.remove('active'));
              btn.classList.add('active');
              
              const count = parseInt(btn.dataset.val, 10);
              if (!isNaN(count)) {
                setParticleCount(count);
                syncMatterUi();
              }
            });
          });
        }

        // density slider
        const ds = document.getElementById('densitySlider');
        if (ds) ds.addEventListener('input', e => {
          engineState.matter.density = (+e.target.value) / 10;
          cellSdf.radius = Math.max(0.05, 0.5 - engineState.matter.density * 0.15);
          syncMatterUi();
        });

        // noise slider
        const ns = document.getElementById('noiseSlider');
        if (ns) ns.addEventListener('input', e => {
          engineState.matter.noise = +e.target.value;
          syncMatterUi();
        });

        // cohesion slider
        const cs = document.getElementById('cohesionSlider');
        if (cs) cs.addEventListener('input', e => {
          engineState.matter.cohesion = (+e.target.value) / 100;
          syncMatterUi();
        });

        // close button → return to landing
        const close = document.getElementById('close-chefos');
        if (close) close.addEventListener('click', () => {
          document.body.classList.remove('engine-open');
        });

        // periodic refresh for fps / frame
        setInterval(syncMatterUi, 250);
        syncMatterUi();
      }

      // expose for render_loop perf hook
      globalThis.__matterPerf = __matterPerf;

      // delay binding until DOM is ready (script is at bottom but be safe)
      if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', bindMatterUi);
      } else {
        bindMatterUi();
      }

"##;
