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

        <!-- Right panel deleted (Object Inspector) -->

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
