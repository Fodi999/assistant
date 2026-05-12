// ── Matter Lab template — engine-screen markup (sketch + constraints) ──────
// Wireframe editor with dimensions / fixed / horizontal-vertical / profiles.

pub fn matter_lab_section() -> String {
    r##"
  <!-- ── Engine Screen (Matter Lab — 3D Sketch core + constraints) ── -->
  <section id="render-screen">
    <main class="matter-lab-shell">
      <section class="matter-stage">

        <canvas id="webgpu-canvas"></canvas>
        <canvas id="sketch-canvas" style="position:absolute;top:0;left:0;pointer-events:none;z-index:1;"></canvas>

        <!-- Axis gizmo -->
        <canvas id="axis-gizmo" width="96" height="96" title="Click axis to snap view"></canvas>

        <!-- Mini command bar (top center) -->
        <div id="mini-bar">
          <span class="mb-cell"><b>Tool</b> <span id="mini-tool">SELECT</span></span>
          <span class="mb-sep">·</span>
          <span class="mb-cell"><b>Plane</b> <span id="mini-plane">XZ</span></span>
          <span class="mb-sep">·</span>
          <span class="mb-cell"><b>Snap</b> <span id="mini-snap">—</span></span>
          <span class="mb-sep">·</span>
          <span class="mb-cell"><b>Length</b> <span id="mini-length">—</span></span>
        </div>

        <!-- Hotkey strip -->
        <div id="hotkey-strip">
          <span><b>S</b> Select</span>
          <span><b>P</b> Point</span>
          <span><b>L</b> Line</span>
          <span><b>G</b> Grab</span>
          <span><b>D</b> Dim</span>
          <span><b>F</b> Fix</span>
          <span><b>H</b> Horiz</span>
          <span><b>V</b> Vert</span>
          <span><b>⇧V</b> Valid</span>
          <span><b>1/2/3</b> Plane</span>
          <span><b>⌫</b> Del</span>
          <span><b>⌘Z</b> Undo</span>
          <span><b>Esc</b> Cancel</span>
        </div>

        <!-- Working plane pills (top-left) -->
        <div id="plane-switch">
          <button class="plane-pill active" data-plane="XZ" title="Top plane (1)">XZ</button>
          <button class="plane-pill"        data-plane="XY" title="Front plane (2)">XY</button>
          <button class="plane-pill"        data-plane="YZ" title="Right plane (3)">YZ</button>
        </div>

        <!-- Universal Toolbar — 5 sketch tools -->
        <nav id="universal-toolbar" aria-label="Sketch tools">
          <button class="utb-btn active" data-tool="select" title="Select (S)">↖<span class="utb-label">Select</span></button>
          <button class="utb-btn"        data-tool="point"  title="Point (P)">•<span class="utb-label">Point</span></button>
          <button class="utb-btn"        data-tool="line"   title="Line (L)">╱<span class="utb-label">Line</span></button>
          <button class="utb-btn"        data-tool="grab"   title="Grab (G)">✥<span class="utb-label">Grab</span></button>
          <button class="utb-btn"        data-tool="delete" title="Delete (⌫)">⌫<span class="utb-label">Delete</span></button>
        </nav>

        <!-- Sketch Inspector — Blender-style N-panel (slides from right edge) -->
        <button id="si-tab" class="si-edge-tab" title="Toggle Inspector (N)">
          <span class="si-tab-label">N</span>
        </button>
        <aside id="sketch-inspector" class="glass-dark">
          <header class="si-header">
            <span>Sketch Inspector</span>
          </header>
          <div id="si-body">
          <dl class="si-grid">
            <dt>Tool</dt>            <dd id="si-tool">SELECT</dd>
            <dt>Plane</dt>           <dd id="si-plane">XZ</dd>
            <dt>Points</dt>          <dd id="si-points">0</dd>
            <dt>Edges</dt>           <dd id="si-edges">0</dd>
            <dt>Selected</dt>        <dd id="si-selected">0</dd>
            <dt>Open ends</dt>       <dd id="si-open-ends">0</dd>
            <dt>Isolated</dt>        <dd id="si-isolated">0</dd>
            <dt>Closed profiles</dt> <dd id="si-profiles">0</dd>
            <dt>Validation</dt>      <dd id="si-validation">on</dd>
          </dl>

          <div class="si-divider"></div>

          <div id="si-block-none" class="si-block">
            <div class="si-block-title">No selection</div>
            <div class="si-block-hint">Pick a point or edge with Select (S).</div>
          </div>

          <div id="si-block-point" class="si-block" style="display:none;">
            <div class="si-block-title">Point</div>
            <dl class="si-grid">
              <dt>Id</dt>         <dd id="si-pt-id">—</dd>
              <dt>Grid</dt>       <dd id="si-pt-grid">—</dd>
              <dt>World</dt>      <dd id="si-pt-world">—</dd>
              <dt>Edges</dt>      <dd id="si-pt-degree">0</dd>
              <dt>Fixed</dt>      <dd id="si-pt-fixed">no</dd>
              <dt>Validation</dt> <dd id="si-pt-valid">—</dd>
            </dl>
          </div>

          <div id="si-block-edge" class="si-block" style="display:none;">
            <div class="si-block-title">Edge</div>
            <dl class="si-grid">
              <dt>Id</dt>           <dd id="si-eg-id">—</dd>
              <dt>A</dt>            <dd id="si-eg-from">—</dd>
              <dt>B</dt>            <dd id="si-eg-to">—</dd>
              <dt>Length</dt>       <dd id="si-eg-len">—</dd>
              <dt>Dimension</dt>    <dd id="si-eg-dim">—</dd>
              <dt>Orientation</dt>  <dd id="si-eg-orient">—</dd>
              <dt>Profile</dt>      <dd id="si-eg-profile">—</dd>
              <dt>ΔX</dt>           <dd id="si-eg-dx">—</dd>
              <dt>ΔY</dt>           <dd id="si-eg-dy">—</dd>
              <dt>ΔZ</dt>           <dd id="si-eg-dz">—</dd>
            </dl>
          </div>

          <div id="si-block-multi" class="si-block" style="display:none;">
            <div class="si-block-title">Multi-selection</div>
            <dl class="si-grid">
              <dt>Points</dt>         <dd id="si-multi-pts">0</dd>
              <dt>Edges</dt>          <dd id="si-multi-eds">0</dd>
              <dt>Fixed points</dt>   <dd id="si-multi-fixed">0</dd>
              <dt>Constrained</dt>    <dd id="si-multi-constr">0</dd>
              <dt>Total len</dt>      <dd id="si-multi-len">—</dd>
            </dl>
          </div>

          <div class="si-divider"></div>
          <div class="si-hint" id="si-hint">Click to pick · Shift+click to add</div>

          <!-- ── Selected Profile (Phase 8) ── -->
          <div class="si-divider"></div>
          <div id="si-block-profile" class="si-block" style="display:none;">
            <div class="si-block-title">Selected Profile</div>
            <dl class="si-grid">
              <dt>Id</dt>        <dd id="si-pf-id">—</dd>
              <dt>Plane</dt>     <dd id="si-pf-plane">—</dd>
              <dt>Points</dt>    <dd id="si-pf-points">0</dd>
              <dt>Edges</dt>     <dd id="si-pf-edges">0</dd>
              <dt>Area</dt>      <dd id="si-pf-area">—</dd>
              <dt>Extrudable</dt><dd id="si-pf-ready">no</dd>
            </dl>
            <div style="margin-top:8px; display:flex; gap:6px;">
              <button id="si-pf-copy" class="sio-btn" style="flex:1;">⧉ Copy Profile</button>
              <button id="si-pf-clear" class="sio-btn" style="flex:0 0 auto; padding:6px 10px;">✕</button>
            </div>
          </div>

          <div id="si-block-profile-list" class="si-block">
            <div class="si-block-title">Profiles · <span id="si-pf-count">0</span></div>
            <ul id="si-pf-list" style="list-style:none; padding:0; margin:6px 0 0;
                 max-height:140px; overflow-y:auto; font-size:11px;">
            </ul>
          </div>

          <!-- ── Precision (Phase 7) ── -->
          <div class="si-divider"></div>
          <div class="si-block-title">Precision</div>
          <dl class="si-grid">
            <dt>Grid size</dt>
            <dd>
              <input id="si-grid-size" type="number" step="any" min="0.001" max="1000"
                     value="1" style="width:64px; background:rgba(15,23,42,0.7); color:#e2e8f0;
                     border:1px solid rgba(148,163,184,0.25); border-radius:4px; padding:1px 4px;
                     font:inherit; text-align:right;">
            </dd>
            <dt>Backend</dt>
            <dd>
              <label style="display:inline-flex; align-items:center; gap:4px; cursor:pointer;">
                <input id="si-use-backend" type="checkbox" checked
                       style="accent-color:#38bdf8;">
                <span id="si-backend-onoff">ON</span>
              </label>
            </dd>
            <dt>Last result</dt> <dd id="si-backend-last">—</dd>
            <dt>Coord prec.</dt> <dd>3 dec</dd>
          </dl>

          <!-- ── JSON Export / Import ── -->
          <div class="si-divider"></div>
          <div id="sketch-io-panel">
            <header class="sio-header">
              <span class="sio-title">SketchGraph JSON</span>
              <button id="sio-toggle" class="sio-toggle" title="Collapse / expand">▾</button>
            </header>
            <div class="sio-tabs">
              <button class="sio-tab active" data-mode="full"    title="Full sketch">Full</button>
              <button class="sio-tab"        data-mode="payload" title="Backend payload">Payload</button>
              <button id="sio-refresh" class="sio-btn-mini" title="Refresh">↻</button>
            </div>
            <pre id="sio-preview" class="sio-preview">{}</pre>
            <div id="sio-meta" class="sio-meta">—</div>
            <div class="sio-actions">
              <button id="sio-copy"     class="sio-btn" title="Copy JSON">⧉ Copy</button>
              <button id="sio-download" class="sio-btn" title="Download JSON">⬇ Save</button>
              <button id="sio-load"     class="sio-btn" title="Load JSON file">⬆ Load</button>
              <input  id="sio-file-input" type="file" accept="application/json,.json" style="display:none;">
            </div>
          </div>
          </div><!-- /si-body -->
        </aside>

        <!-- Status bar -->
        <footer class="status-bar">
          <div>
            <span class="online-dot"></span>
            Engine online
            <span class="muted">Rust · WebGPU · Sketch core</span>
            <span id="webgpu-status" class="webgpu-status">⏳ probing WebGPU…</span>
          </div>
          <div class="perf">
            <span>FPS <b id="fpsValue">—</b></span>
            <span>Frame <b id="frameValue">—</b></span>
          </div>
        </footer>

      </section>
    </main>
  </section>
"##.to_string()
}
