// ── Matter Lab template — engine-screen markup ──────────
// The outer `#render-screen` wrapper is shown by `body.engine-open` (see styles.rs).
// Inside lives the full Matter Lab UI: topbar / left-tools / canvas / matter-panel /
// action-bar / status-bar — all positioned absolutely over the canvas.

pub fn matter_lab_section() -> String {
    let profile_panel = crate::web::home::matter_panels::profile_panel();
    let properties_panel = crate::web::home::matter_panels::properties_panel();
    let shape_panel = crate::web::home::matter_panels::shape_panel();
    let material_panel = crate::web::home::matter_panels::material_panel();
    let nodes_panel = crate::web::home::matter_panels::nodes_panel();
    let history_panel = crate::web::home::matter_panels::history_panel();
    let ai_panel = crate::web::home::matter_panels::ai_panel();
    let sketch_panel = crate::web::home::matter_panels::sketch_panel();
    let outliner_panel = crate::web::home::matter_outliner::outliner_panel();

    format!(
        r##"
  <!-- ── Engine Screen (Matter Lab) ── -->
  <section id="render-screen">
    <main class="matter-lab-shell">

      <!-- Header / Topbar removed completely -->

      <!-- Stage layer ──────────────────────────────────────── -->
      <section class="matter-stage">
        
        <!-- Floating Close Button -->
        <button id="close-chefos" class="close-engine-btn" title="Выйти на главную">✕</button>

        <!-- Canvas (z=0) -->
        <canvas id="webgpu-canvas"></canvas>
        <canvas id="sketch-canvas" style="position: absolute; top: 0; left: 0; pointer-events: none; z-index: 1; display: none;"></canvas>

        <!-- Axis gizmo (top-left, z=20) — click axis to snap camera view -->
        <canvas id="axis-gizmo" width="96" height="96" title="Click axis to snap view"></canvas>

        <!-- Engine Mode Switcher ── PARTICLE ↔ CAD ── -->
        <div class="engine-mode-switcher" id="engine-mode-switcher">
          <button class="mode-btn" data-mode="PARTICLES" title="Mode Switch">
            <span class="mode-icon">⬡</span>
            <span class="mode-label">RENDER</span>
          </button>
        </div>

        <!-- Bottom toolbar island: mode switcher + primitives -->
        <div class="selection-mode-switcher" id="selection-mode-switcher">
          <!-- Mode buttons -->
          <button class="sel-btn active" data-sel="0" title="Object Mode"><span style="font-weight:900;">◼</span> Object</button>
          <button class="sel-btn" data-sel="4" title="Sketch Mode"><span style="font-weight:900;">✎</span> Sketch</button>
          <button class="sel-btn" data-sel="1" title="Face Mode"><span>▨</span> Face</button>
          <button class="sel-btn" data-sel="2" title="Edge Mode"><span>◰</span> Edge</button>
          <button class="sel-btn" data-sel="3" title="Vertex Mode"><span style="font-size:10px;">⬤</span> Vertex</button>
          <!-- Divider -->
          <div class="toolbar-sep"></div>
          <!-- Primitives -->
          <button class="prim-btn" data-asset="box"            title="Add Box">▢<span class="prim-label">Box</span></button>
          <button class="prim-btn" data-asset="sphere"         title="Add Sphere">◉<span class="prim-label">Sphere</span></button>
          <button class="prim-btn" data-asset="cylinder"       title="Add Cylinder">▮<span class="prim-label">Cylinder</span></button>
          <div class="toolbar-sep"></div>
          <button class="prim-btn" data-asset="rect-sketch"    title="Rectangle Sketch">▭<span class="prim-label">Rect</span></button>
          <button class="prim-btn" data-asset="circle-sketch"  title="Circle Sketch">○<span class="prim-label">Circle</span></button>
          <button class="prim-btn" data-asset="polygon-sketch" title="Polygon Sketch">⬡<span class="prim-label">Poly</span></button>
        </div>

        <!-- Sketch Tools (Visible only in Sketch Mode) -->
        <div class="sketch-tools-switcher" id="sketch-tools-switcher" style="display:none; position: absolute; left: 50%; top: 60px; transform: translateX(-50%); z-index: 20; background: rgba(30,30,35,0.85); border: 1px solid rgba(255,255,255,0.1); border-radius: 8px; padding: 4px; display: flex; gap: 4px; backdrop-filter: blur(10px);">
          <button class="sketch-tool-btn active" data-tool="line" style="background: rgba(255,255,255,0.1); color: #fff; border: none; padding: 4px 12px; border-radius: 4px; cursor: pointer; font-size: 13px;">Line</button>
          <button class="sketch-tool-btn" data-tool="rectangle" style="background: transparent; color: #cbd5e1; border: none; padding: 4px 12px; border-radius: 4px; cursor: pointer; font-size: 13px;">Rect</button>
          <button class="sketch-tool-btn" data-tool="circle" style="background: transparent; color: #cbd5e1; border: none; padding: 4px 12px; border-radius: 4px; cursor: pointer; font-size: 13px;">Circle</button>
          <button class="sketch-tool-btn" data-tool="dimension" style="background: transparent; color: #cbd5e1; border: none; padding: 4px 12px; border-radius: 4px; cursor: pointer; font-size: 13px;">Dim</button>
        </div>

        <div id="sketch-info-overlay" style="display:none; position: absolute; left: 24px; bottom: 48px; z-index: 10; font-family: monospace; font-size: 11px; color: #64748b; line-height: 1.5; pointer-events: none;">
          <div style="font-weight: bold; color: #cbd5e1; margin-bottom: 4px;">MODE: <span id="sketch-info-mode">Sketch</span></div>
          <div>PLANE: <span id="sketch-info-plane">XZ Top</span></div>
          <div>TOOL: <span id="sketch-info-tool">line</span></div>
          <div>SNAP: <span id="sketch-info-grid">10 cm</span></div>
          <div>MOUSE: <span id="sketch-info-mouse">—</span></div>
        </div>

        <!-- Right Properties Panel (Blender N-panel analog) -->
        {}

        <!-- Shape Panel -->
        {}

        <!-- Material Panel -->
        {}

        <!-- Nodes Panel -->
        {}

        <!-- History Panel -->
        {}

        <!-- AI Panel -->
        {}

        <!-- Right Profile Panel (M-panel) -->
        {}

        <!-- Sketch Inspector Panel -->
        {}

        <!-- Left Outliner + Tool Shelf -->
        {}

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
"##,
        properties_panel, shape_panel, material_panel, nodes_panel, history_panel, ai_panel, profile_panel, sketch_panel, outliner_panel
    )
}
