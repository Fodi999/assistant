// ── Matter Lab template — engine-screen markup ──────────
// The outer `#render-screen` wrapper is shown by `body.engine-open` (see styles.rs).
// Inside lives the full Matter Lab UI: topbar / left-tools / canvas / matter-panel /
// action-bar / status-bar — all positioned absolutely over the canvas.

pub fn matter_lab_section() -> &'static str {
    r##"
  <!-- ── Engine Screen (Matter Lab) ── -->
  <section id="render-screen">
    <main class="matter-lab-shell">

      <!-- Top bar ──────────────────────────────────────────── -->
      <header class="matter-topbar">
        <div class="brand">
          <div class="brand-icon"></div>
          <span>ChefOS Matter Lab</span>
        </div>
        <div class="top-title">— Matter Laboratory —</div>
        <nav class="top-nav">
          <button>Projects</button>
          <button>Presets</button>
          <button>Docs</button>
          <button id="close-chefos" class="back-btn" title="Назад">←</button>
          <div class="user-badge">CH</div>
        </nav>
      </header>

      <!-- Stage layer ──────────────────────────────────────── -->
      <section class="matter-stage">

        <!-- Canvas (z=0) -->
        <canvas id="webgpu-canvas"></canvas>

        <!-- Axis gizmo (top-right, z=20) — click axis to snap camera view -->
        <canvas id="axis-gizmo" width="96" height="96" title="Click axis to snap view"></canvas>

        <!-- Diagnostics overlay (only visible during probe) -->
        <div id="gpu-diag" style="
          display:none; position:absolute; top:96px; left:50%; transform:translateX(-50%);
          background:rgba(8,14,28,0.92); border:1px solid rgba(52,211,153,0.4);
          border-radius:14px; padding:18px 28px; z-index:25; font:13px monospace;
          color:#e2e8f0; min-width:320px; pointer-events:none; line-height:1.9;
        "></div>

        <!-- Left tools (z=15) -->
        <aside class="left-tools">
          <button class="tool-btn" data-tool="select">Select</button>
          <button class="tool-btn" data-tool="move">Move</button>
          <button class="tool-btn active" data-tool="shape">Shape</button>
          <button class="tool-btn" data-tool="matter">Matter</button>
        </aside>

        <!-- Right panel: Object Inspector (z=15) -->
        <aside class="matter-panel">
          <div class="panel-head">
            <h2>OBJECT</h2>
            <button title="advanced">⚙</button>
          </div>

          <div class="setting-row">
            <span>Type:</span>
            <div class="toggle-group" style="padding: 4px 6px; font-weight: bold; background: none; color: #a78bfa;">Cube Cell</div>
          </div>
          
          <div class="setting-row" style="padding-top: 4px; padding-bottom: 4px;">
            <span style="width:100%">Position: <span style="float:right; color:#cbd5e1" id="ui-obj-pos">X 0.00 / Y 0.05 / Z 0.00</span></span>
          </div>

          <div class="setting-row" style="padding-top: 4px; padding-bottom: 4px;">
            <span>Size:</span>
            <strong style="color: #67e8f9;">100 mm</strong>
          </div>

          <div class="setting-row">
            <span>Render:</span>
            <strong style="color: #34d399;">Mesh</strong>
          </div>

          <div class="setting-row">
            <span>Display:</span>
            <strong style="color: #fcd34d;">Solid</strong>
          </div>

          <div class="setting-row">
            <span>Edges:</span>
            <strong style="color: #fbbf24;">On</strong>
          </div>

          <div class="setting-row" style="margin-top: 8px; border-top: 1px dashed rgba(255,255,255,0.1); padding-top: 8px;">
            <span>CUBE GRID</span>
          </div>

          <div class="setting-row">
            <span>Side:</span>
            <strong style="color: #a78bfa;" id="ui-cube-side">1</strong>
          </div>
          
          <div class="setting-row">
            <span>Cell size:</span>
            <strong style="color: #94a3b8;" id="ui-cube-cell-size">100 mm</strong>
          </div>

          <div class="setting-row">
            <span>Object size:</span>
            <strong style="color: #67e8f9;" id="ui-cube-obj-size">100 mm</strong>
          </div>

          <div class="stats-row">
            <span>Surface</span>
            <strong id="surfaceValue">1</strong>
          </div>
          <div class="stats-row">
            <span>Interior</span>
            <strong id="interiorValue">0</strong>
          </div>

          <div class="setting-row" style="padding-top: 8px; padding-bottom: 8px;">
            <div class="toggle-group" id="ui-cube-sizes-btns" style="width:100%; justify-content: space-between;">
              <button class="active" data-val="1">1&sup3;</button>
              <button data-val="2">2&sup3;</button>
              <button data-val="3">3&sup3;</button>
              <button data-val="5">5&sup3;</button>
              <button data-val="10">10&sup3;</button>
            </div>
          </div>

          <div class="setting-row" style="margin-top: 8px; border-top: 1px dashed rgba(255,255,255,0.1); padding-top: 8px;">
            <span>GRID SCALE</span>
            <div class="toggle-group" id="ui-grid-scale">
              <button data-val="mm">mm</button>
              <button data-val="cm">cm</button>
              <button class="active" data-val="m">m</button>
            </div>
          </div>

          <div class="setting-row">
            <span>PARTICLES</span>
            <div class="toggle-group" id="ui-particles-scale">
              <button data-val="1">1</button>
              <button data-val="1000">1k</button>
              <button data-val="10000">10k</button>
              <button data-val="100000" style="padding:0 2px;">100k</button>
              <button class="active" data-val="1000000">1M</button>
            </div>
          </div>

          <div class="select-row" data-cycle="formation">
            <span>Formation</span>
            <strong id="formationValue">Cube</strong>
          </div>

          <div class="select-row" data-cycle="shape">
            <span>Shape</span>
            <strong id="shapeValue">Super-cube</strong>
          </div>

          <label class="slider-row">
            <span>Density</span>
            <strong id="densityValue">1.4k/u³</strong>
            <input id="densitySlider" type="range" min="1" max="30" value="14">
          </label>

          <label class="slider-row">
            <span>Noise</span>
            <strong id="noiseValue">22.0</strong>
            <input id="noiseSlider" type="range" min="0" max="50" value="22">
          </label>

          <label class="slider-row">
            <span>Cohesion</span>
            <strong id="cohesionValue">0.75</strong>
            <input id="cohesionSlider" type="range" min="0" max="100" value="75">
          </label>

          <div class="stats-row">
            <span>Surface</span>
            <strong id="surfaceValue">58.8K</strong>
          </div>
          <div class="stats-row">
            <span>Interior</span>
            <strong id="interiorValue">941.2K</strong>
          </div>

          <button class="advanced-btn">Advanced Settings ›</button>
        </aside>

        <!-- Bottom action bar (z=15) -->
        <div class="action-bar">
          <div class="action-meta">
            <small>CURRENT TOOL</small>
            <strong id="currentTool">Shape mode</strong>
          </div>
          <div class="action-meta">
            <small>SELECTED ACTION</small>
            <strong id="selectedAction">Configure matter formation</strong>
          </div>
          <button data-action="compress">Compress</button>
          <button data-action="expand">Expand</button>
          <button data-action="scatter">Scatter</button>
          <button data-action="smooth">Smooth</button>
          <button data-action="freeze">Freeze</button>
          <button data-action="reset">Reset</button>
        </div>

        <!-- Status bar (z=15) -->
        <footer class="status-bar">
          <div>
            <span class="online-dot"></span>
            Engine online
            <span class="muted">Rust · WebGPU · Koyeb Ready</span>
            <span id="webgpu-status" class="webgpu-status">⏳ probing WebGPU…</span>
          </div>
          <div class="perf">
            <span>FPS <b id="fpsValue">—</b></span>
            <span>Frame <b id="frameValue">—</b></span>
            <span>Particles <b id="particlesHud">1.0M</b></span>
          </div>
        </footer>

      </section>
    </main>
  </section>
"##
}
