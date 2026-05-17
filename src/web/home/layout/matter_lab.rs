// ── Matter Lab template — engine-screen markup (sketch + constraints) ──────
// Wireframe editor with dimensions / fixed / horizontal-vertical / profiles.

pub fn matter_lab_section() -> String {
    let before_panel = r##"
  <!-- ── Engine Screen (Matter Lab — 3D Sketch core + constraints) ── -->
  <section id="render-screen">
    <main class="matter-lab-shell">
      <section class="matter-stage">

        <canvas id="webgpu-canvas"></canvas>
        <canvas id="sketch-canvas" style="position:absolute;top:0;left:0;pointer-events:none;z-index:1;"></canvas>

        <!-- Axis gizmo -->
        <canvas id="axis-gizmo" width="96" height="96" title="Click axis to snap view"></canvas>

        <!-- Performance HUD (Shift+P toggle) -->
        <div id="perf-hud" class="perf-hud">
          <div id="perf-hud-header" class="perf-hud-header" title="Click to collapse · Shift+P toggle">
            <span id="perf-hud-caret">▾</span>
            <span class="perf-hud-title">PERF</span>
          </div>
          <div id="perf-hud-body" class="perf-hud-body">
            <div class="perf-row"><span class="perf-key">FPS</span>      <span id="perf-fps"      class="perf-val">0</span></div>
            <div class="perf-row"><span class="perf-key">Frame</span>    <span id="perf-frame"    class="perf-val">0 ms</span></div>
            <div class="perf-row"><span class="perf-key">Render</span>   <span id="perf-render"   class="perf-val">0 ms</span></div>
            <div class="perf-row"><span class="perf-key">Overlay</span>  <span id="perf-overlay"  class="perf-val">0 ms</span></div>
            <div class="perf-row"><span class="perf-key">Pick</span>     <span id="perf-pick"     class="perf-val">0 ms</span></div>
            <div class="perf-row"><span class="perf-key">Backend</span>  <span id="perf-backend"  class="perf-val">—</span></div>
            <div class="perf-sep"></div>
            <div class="perf-row"><span class="perf-key">Mode</span>     <span id="perf-mode"     class="perf-val">backend</span></div>
            <div class="perf-row"><span class="perf-key">WASM ms</span>  <span id="perf-wasm-ms"  class="perf-val">—</span></div>
            <div class="perf-row"><span class="perf-key">BE ms</span>    <span id="perf-be-ms"    class="perf-val">—</span></div>
            <div class="perf-sep"></div>
            <div class="perf-row"><span class="perf-key">Pts</span>      <span id="perf-pts"      class="perf-val">0</span></div>
            <div class="perf-row"><span class="perf-key">Edges</span>    <span id="perf-edges"    class="perf-val">0</span></div>
            <div class="perf-row"><span class="perf-key">Profiles</span> <span id="perf-profiles" class="perf-val">0</span></div>
            <div class="perf-row"><span class="perf-key">Sel</span>      <span id="perf-selected" class="perf-val">0</span></div>
            <div class="perf-sep"></div>
            <div class="perf-row"><span class="perf-key">DPR</span>      <span id="perf-dpr"      class="perf-val">1.00</span></div>
            <div class="perf-row"><span class="perf-key">Canvas</span>   <span id="perf-canvas"   class="perf-val">—</span></div>
            <div class="perf-row"><span class="perf-key">Scale</span>    <span id="perf-scale"    class="perf-val">1.00</span></div>
          </div>
        </div>

        <!-- Mini command bar (top center) — MODE · TOOL · PLANE · SNAP only -->
        <div id="mini-bar">
          <span class="mb-cell"><b>Mode</b> <span id="mini-mode">Free 3D</span></span>
          <span class="mb-sep">·</span>
          <span class="mb-cell"><b>Tool</b> <span id="mini-tool">SELECT</span></span>
          <span class="mb-sep">·</span>
          <span class="mb-cell"><b>Plane</b> <span id="mini-plane">XZ</span></span>
          <span class="mb-sep">·</span>
          <span class="mb-cell"><b>Snap</b> <span id="mini-snap">—</span></span>
          <!-- Shortcuts toggle button -->
          <button id="shortcuts-toggle" title="Keyboard shortcuts (?)">?</button>
        </div>

        <!-- Shortcuts overlay (hidden by default, toggled by ? button or ? key) -->
        <div id="shortcuts-overlay" style="display:none">
          <div class="sco-title">Keyboard Shortcuts <button id="shortcuts-close">✕</button></div>
          <div class="sco-grid">
            <span class="sco-key">S</span><span>Select</span>
            <span class="sco-key">P</span><span>Point</span>
            <span class="sco-key">L</span><span>Line</span>
            <span class="sco-key">G</span><span>Grab</span>
            <span class="sco-key">⇧G</span><span>Copy</span>
            <span class="sco-key">D</span><span>Dimension</span>
            <span class="sco-key">F</span><span>Fix</span>
            <span class="sco-key">H</span><span>Horizontal</span>
            <span class="sco-key">V</span><span>Vertical</span>
            <span class="sco-key">J</span><span>Project</span>
            <span class="sco-key">O</span><span>Ortho lock</span>
            <span class="sco-key">1/2/3</span><span>Switch plane</span>
            <span class="sco-key">Space</span><span>Centre scene</span>
            <span class="sco-key">⌫</span><span>Delete</span>
            <span class="sco-key">⌘Z</span><span>Undo</span>
            <span class="sco-key">⇧P</span><span>Perf HUD</span>
            <span class="sco-key">Esc</span><span>Cancel</span>
          </div>
        </div>

        <!-- Floating cursor measurement HUD (shown only when __cursorInfoVisible=true) -->
        <div id="cursor-hud" style="display:none">
          <div class="chud-row"><span class="chud-lbl">X</span><span id="chud-x">—</span></div>
          <div class="chud-row"><span class="chud-lbl">Y</span><span id="chud-y">—</span></div>
          <div class="chud-row"><span class="chud-lbl">Z</span><span id="chud-z">—</span></div>
          <div class="chud-sep"></div>
          <div class="chud-row chud-len"><span class="chud-lbl">L</span><span id="chud-len">—</span></div>
          <div class="chud-row chud-ang" style="display:none"><span class="chud-lbl">∠</span><span id="chud-ang">—</span></div>
          <div class="chud-sep"></div>
          <div class="chud-row chud-snap-row"><span class="chud-lbl">⊙</span><span id="chud-snap">—</span></div>
        </div>

        <!-- Working plane pills (top-left) -->
        <div id="plane-switch">
          <button class="plane-pill active" data-plane="XZ" title="Top plane (1)">XZ</button>
          <button class="plane-pill"        data-plane="XY" title="Front plane (2)">XY</button>
          <button class="plane-pill"        data-plane="YZ" title="Right plane (3)">YZ</button>
        </div>

        <!-- Universal Toolbar — 5 sketch tools + Ortho Lock -->
        <nav id="universal-toolbar" aria-label="Sketch tools">
          <button class="utb-btn active" data-tool="select" title="Select (S)">↖<span class="utb-label">Select</span></button>
          <button class="utb-btn"        data-tool="point"  title="Point (P)">•<span class="utb-label">Point</span></button>
          <button class="utb-btn"        data-tool="line"   title="Line (L)">╱<span class="utb-label">Line</span></button>
          <button class="utb-btn"        data-tool="grab"   title="Grab (G)">✥<span class="utb-label">Grab</span></button>
          <button class="utb-btn"        data-tool="delete" title="Delete (⌫)">⌫<span class="utb-label">Delete</span></button>
          <div class="utb-sep"></div>
          <button class="utb-btn" id="btn-ortho" data-toggle="ortho"
                  title="Ortho Lock — snap to 0° 45° 90° (O)"
                  onclick="if(window.__toggleOrthoLock) window.__toggleOrthoLock()">
            ⊾<span class="utb-label">Ortho</span>
          </button>
        </nav>
"##;

    let cad_panel = crate::web::home::layout::cad_side_panel::cad_side_panel_html();

    let after_panel = r##"
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
"##;

    [before_panel, cad_panel, after_panel].concat()
}
