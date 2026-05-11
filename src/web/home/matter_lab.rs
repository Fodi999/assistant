// ── Matter Lab template — engine-screen markup (sketch + constraints) ──────
// Wireframe editor with dimensions / fixed / horizontal-vertical / profiles.

pub fn matter_lab_section() -> String {
    r##"
  <!-- ── Engine Screen (Matter Lab — 3D Sketch core + constraints) ── -->
  <section id="render-screen">
    <main class="matter-lab-shell">
      <section class="matter-stage">

        <button id="close-chefos" class="close-engine-btn" title="Выйти на главную">✕</button>

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

        <!-- Sketch Inspector (right side) -->
        <aside id="sketch-inspector" class="glass-dark">
          <header class="si-header">Sketch Inspector</header>
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
        </aside>

        <!-- Sketch I/O panel (bottom-right): export/import JSON contract -->
        <aside id="sketch-io-panel" class="glass-dark">
          <header class="sio-header">
            <span class="sio-title">SketchGraph JSON</span>
            <button id="sio-toggle" class="sio-toggle" title="Collapse / expand panel">▾ JSON</button>
          </header>
          <div class="sio-tabs">
            <button class="sio-tab active" data-mode="full"    title="Full sketch (with world coords + profiles)">Full</button>
            <button class="sio-tab"        data-mode="payload" title="Backend-compatible payload (slim)">Payload</button>
            <button id="sio-refresh" class="sio-btn-mini" title="Refresh preview">↻</button>
          </div>
          <pre id="sio-preview" class="sio-preview">{}</pre>
          <div id="sio-meta" class="sio-meta">—</div>
          <div class="sio-actions">
            <button id="sio-copy"     class="sio-btn" title="Copy JSON to clipboard">⧉ Copy</button>
            <button id="sio-download" class="sio-btn" title="Download JSON file">⬇ Download</button>
            <button id="sio-load"     class="sio-btn" title="Load JSON file from disk">⬆ Load</button>
            <input  id="sio-file-input" type="file" accept="application/json,.json" style="display:none;">
          </div>
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
