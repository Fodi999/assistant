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

        <!-- Axis gizmo (top-left, z=20) — click axis to snap camera view -->
        <canvas id="axis-gizmo" width="96" height="96" title="Click axis to snap view"></canvas>

        <!-- Engine Mode Switcher ── PARTICLE ↔ CAD ── -->
        <div class="engine-mode-switcher" id="engine-mode-switcher">
          <button class="mode-btn active" data-mode="PARTICLES" title="Particle / Morph Mode">
            <span class="mode-icon">⬡</span>
            <span class="mode-label">PARTICLE</span>
          </button>
          <button class="mode-btn" data-mode="CAD" title="Solid / CAD Mode">
            <span class="mode-icon">◈</span>
            <span class="mode-label">SOLID</span>
          </button>
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
        properties_panel, shape_panel, material_panel, nodes_panel, history_panel, ai_panel, profile_panel
    )
}
