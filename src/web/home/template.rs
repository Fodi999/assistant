pub fn template(styles: &str, scripts: &str) -> String {
    let head = r##"<!doctype html>
<!-- ChefOS Interactive Engine — v2: fullscreen render mode -->
<html lang="ru">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>ChefOS Interactive Engine</title>
  <meta name="description" content="Интерактивная операционная система для шеф-повара: рецепты, склад, ингредиенты, себестоимость и лаборатория в одном игровом интерфейсе." />
  <style>"##;

    let mid = r##"</style>
</head>

<body>
  <div class="grid"></div>

  <div class="shell">
    <header>
      <div class="brand">
        <div class="logo"></div>
        <div>ChefOS Interactive</div>
      </div>
      <nav>
        <span>Ingredients</span>
        <span>Tools</span>
        <span>Laboratory</span>
        <span>Inventory</span>
      </nav>
    </header>

    <main>
      <section class="hero">
        <h1>Сайт как игровой движок для шеф-повара.</h1>
        <p>
          Интерактивная платформа, где ингредиенты, рецепты, склад, себестоимость
          и лаборатория превращаются в рабочие сцены, объекты и действия.
        </p>
        <div class="actions">
          <button class="button" id="open-chefos">Открыть ChefOS</button>
          <a class="button secondary" href="/public/tools/categories">Проверить API</a>
        </div>
      </section>

      <section class="viewport-card" aria-label="ChefOS viewport preview">
        <div class="fake-viewport"></div>
        <div class="object"></div>
        <div class="toolbar">
          <div class="tool">↖</div>
          <div class="tool">＋</div>
          <div class="tool">□</div>
          <div class="tool">⛶</div>
        </div>
        <div class="panel">
          <h3>Selected Object</h3>
          <div class="row"><span>Type</span><strong>Ingredient Card</strong></div>
          <div class="row"><span>Mode</span><strong>Preview</strong></div>
          <div class="row"><span>Engine</span><strong>Rust</strong></div>
          <div class="row"><span>Render</span><strong>WebGPU soon</strong></div>
        </div>
        <div class="status">
          <span><span class="dot"></span>Backend online</span>
          <span>Axum · Rust · Koyeb Ready</span>
        </div>
      </section>
    </main>

    <footer>
      ChefOS Interactive Engine · Rust backend + future WebGPU frontend
    </footer>
  </div>

  <!-- ── Render Screen ── -->
  <section id="render-screen">
    <canvas id="webgpu-canvas"></canvas>
    <div class="render-gradient"></div>
    <div class="render-grid"></div>

    <div id="gpu-diag" style="
      display:none; position:absolute; top:80px; left:50%; transform:translateX(-50%);
      background:rgba(8,14,28,0.92); border:1px solid rgba(52,211,153,0.4);
      border-radius:14px; padding:18px 28px; z-index:999; font:13px monospace;
      color:#e2e8f0; min-width:320px; pointer-events:none; line-height:1.9;
    "></div>

    <div class="island island-top">
      <strong>ChefOS Render Scene</strong>
      <span id="scene-counter">— · Catalog</span>
      <div style="display:flex;align-items:center;gap:14px;justify-content:flex-end">
        <span id="webgpu-status" style="font-size:11px;font-family:monospace;color:rgba(148,163,184,0.5);transition:color .4s">⏳ probing WebGPU…</span>
        <button id="close-chefos">← Назад</button>
      </div>
    </div>

    <div class="island island-tools">
      <button class="tool-button active">Select</button>
      <button class="tool-button">Move</button>
      <button class="tool-button">Object</button>
      <button class="tool-button">Recipe</button>
    </div>

    <!-- hidden data source for scene cards -->
    <div class="scene-stage" id="scene-stage" style="display:none"></div>

    <div class="island island-inspector">
      <h3>Particle Scene</h3>
      <div class="inspector-row">
        <span>Objects</span>
        <strong id="selected-name">1M частиц · cohesive cloud</strong>
      </div>
      <div class="inspector-row">
        <span>Renderer</span>
        <strong id="selected-type">WebGPU Ray-3D</strong>
      </div>
      <div class="inspector-row">
        <span>Shading</span>
        <strong id="selected-kcal">Sphere · Superquadric · PBR</strong>
      </div>
      <p id="selected-info">Free orbit · 1-5 = 1K…1M · [ ] = cube↔sphere · sand cursor</p>
    </div>

    <div class="island island-command">
      <div>
        <span class="command-label">Current Tool</span>
        <strong id="current-tool-label">Select mode</strong>
      </div>
      <div>
        <span class="command-label">Selected Action</span>
        <strong id="selected-action-title">Mozzarella cheese · add to recipe</strong>
      </div>
      <div class="command-actions">
        <button>Add to recipe</button>
        <button>Add to inventory</button>
        <button>Nutrition</button>
        <button>Food cost</button>
      </div>
    </div>
  </section>
"##;

    let tail = r##"
</body>
</html>"##;

    [head, styles, mid, scripts, tail].concat()
}
