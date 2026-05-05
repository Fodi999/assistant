use axum::response::{Html, IntoResponse};

pub async fn home_page() -> impl IntoResponse {
    Html(r#"<!doctype html>
<!-- ChefOS Interactive Engine — v2: fullscreen render mode -->
<html lang="ru">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />

  <title>ChefOS Interactive Engine</title>
  <meta name="description" content="Интерактивная операционная система для шеф-повара: рецепты, склад, ингредиенты, себестоимость и лаборатория в одном игровом интерфейсе." />

  <style>
    :root {
      --bg: #050812;
      --panel: rgba(15, 23, 42, 0.78);
      --panel-border: rgba(148, 163, 184, 0.18);
      --text: #e5e7eb;
      --muted: #94a3b8;
      --accent: #38bdf8;
      --accent-2: #a78bfa;
      --good: #22c55e;
    }

    * {
      box-sizing: border-box;
    }

    body {
      margin: 0;
      min-height: 100vh;
      font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      color: var(--text);
      background:
        radial-gradient(circle at 20% 20%, rgba(56, 189, 248, 0.16), transparent 28%),
        radial-gradient(circle at 80% 30%, rgba(167, 139, 250, 0.18), transparent 30%),
        linear-gradient(180deg, #020617 0%, var(--bg) 100%);
      overflow-x: hidden;
    }

    .grid {
      position: fixed;
      inset: 0;
      background-image:
        linear-gradient(rgba(148, 163, 184, 0.08) 1px, transparent 1px),
        linear-gradient(90deg, rgba(148, 163, 184, 0.08) 1px, transparent 1px);
      background-size: 44px 44px;
      mask-image: linear-gradient(to bottom, black, transparent 82%);
      pointer-events: none;
    }

    .shell {
      position: relative;
      z-index: 1;
      min-height: 100vh;
      display: grid;
      grid-template-rows: auto 1fr auto;
    }

    header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 24px clamp(20px, 5vw, 72px);
    }

    .brand {
      display: flex;
      align-items: center;
      gap: 12px;
      font-weight: 800;
      letter-spacing: -0.03em;
    }

    .logo {
      width: 36px;
      height: 36px;
      border-radius: 12px;
      background: linear-gradient(135deg, var(--accent), var(--accent-2));
      box-shadow: 0 0 40px rgba(56, 189, 248, 0.35);
    }

    nav {
      display: flex;
      gap: 18px;
      color: var(--muted);
      font-size: 14px;
    }

    main {
      display: grid;
      grid-template-columns: minmax(0, 1.05fr) minmax(320px, 0.95fr);
      gap: 36px;
      align-items: center;
      padding: 32px clamp(20px, 5vw, 72px) 64px;
    }

    .hero h1 {
      margin: 0;
      max-width: 850px;
      font-size: clamp(46px, 8vw, 92px);
      line-height: 0.92;
      letter-spacing: -0.075em;
    }

    .hero p {
      margin: 28px 0 0;
      max-width: 680px;
      color: var(--muted);
      font-size: clamp(17px, 2vw, 22px);
      line-height: 1.55;
    }

    .actions {
      display: flex;
      flex-wrap: wrap;
      gap: 14px;
      margin-top: 34px;
    }

    .button {
      appearance: none;
      border: 0;
      border-radius: 16px;
      padding: 15px 20px;
      font-weight: 800;
      color: #020617;
      background: linear-gradient(135deg, var(--accent), #67e8f9);
      cursor: pointer;
      text-decoration: none;
      box-shadow: 0 18px 60px rgba(56, 189, 248, 0.25);
    }

    .button.secondary {
      color: var(--text);
      background: rgba(15, 23, 42, 0.75);
      border: 1px solid var(--panel-border);
      box-shadow: none;
    }

    .viewport-card {
      position: relative;
      min-height: 540px;
      border: 1px solid var(--panel-border);
      border-radius: 32px;
      background:
        linear-gradient(180deg, rgba(15, 23, 42, 0.92), rgba(2, 6, 23, 0.78)),
        radial-gradient(circle at 50% 20%, rgba(56, 189, 248, 0.24), transparent 36%);
      box-shadow: 0 28px 110px rgba(0, 0, 0, 0.45);
      overflow: hidden;
    }

    .fake-viewport {
      position: absolute;
      inset: 0;
      background-image:
        linear-gradient(rgba(56, 189, 248, 0.12) 1px, transparent 1px),
        linear-gradient(90deg, rgba(56, 189, 248, 0.12) 1px, transparent 1px);
      background-size: 34px 34px;
      transform: perspective(900px) rotateX(58deg) translateY(80px) scale(1.25);
      transform-origin: center bottom;
      opacity: 0.55;
    }

    .object {
      position: absolute;
      left: 50%;
      top: 43%;
      width: 180px;
      height: 180px;
      transform: translate(-50%, -50%) rotateX(58deg) rotateZ(45deg);
      border-radius: 28px;
      background: linear-gradient(135deg, rgba(56, 189, 248, 0.92), rgba(167, 139, 250, 0.92));
      box-shadow:
        0 0 0 1px rgba(255,255,255,0.24) inset,
        0 28px 80px rgba(56, 189, 248, 0.34);
    }

    .panel {
      position: absolute;
      right: 22px;
      top: 22px;
      width: 210px;
      padding: 16px;
      border-radius: 22px;
      border: 1px solid var(--panel-border);
      background: rgba(2, 6, 23, 0.72);
      backdrop-filter: blur(16px);
    }

    .panel h3 {
      margin: 0 0 12px;
      font-size: 14px;
    }

    .row {
      display: flex;
      justify-content: space-between;
      gap: 12px;
      padding: 8px 0;
      color: var(--muted);
      font-size: 13px;
      border-top: 1px solid rgba(148, 163, 184, 0.12);
    }

    .toolbar {
      position: absolute;
      left: 22px;
      top: 22px;
      display: grid;
      gap: 10px;
    }

    .tool {
      width: 44px;
      height: 44px;
      display: grid;
      place-items: center;
      border-radius: 14px;
      background: rgba(2, 6, 23, 0.74);
      border: 1px solid var(--panel-border);
      color: var(--text);
      font-weight: 800;
    }

    .status {
      position: absolute;
      left: 22px;
      right: 22px;
      bottom: 22px;
      display: flex;
      justify-content: space-between;
      gap: 14px;
      padding: 14px 16px;
      border-radius: 18px;
      background: rgba(2, 6, 23, 0.72);
      border: 1px solid var(--panel-border);
      color: var(--muted);
      font-size: 13px;
    }

    .dot {
      display: inline-block;
      width: 9px;
      height: 9px;
      margin-right: 8px;
      border-radius: 999px;
      background: var(--good);
      box-shadow: 0 0 20px rgba(34, 197, 94, 0.8);
    }

    footer {
      padding: 24px clamp(20px, 5vw, 72px);
      color: var(--muted);
      font-size: 13px;
    }

    /* ── Render Screen ── */
    #render-screen {
      display: none;
      position: fixed;
      inset: 0;
      overflow: hidden;
      background:
        radial-gradient(circle at 30% 35%, rgba(56, 189, 248, 0.18), transparent 28%),
        radial-gradient(circle at 70% 55%, rgba(167, 139, 250, 0.2), transparent 32%),
        linear-gradient(180deg, #020617, #030712);
      z-index: 100;
    }

    body.engine-open .shell {
      display: none;
    }

    body.engine-open #render-screen {
      display: block;
    }

    #webgpu-canvas {
      position: absolute;
      inset: 52px 0 0 0;
      width: 100%;
      height: calc(100% - 52px);
      display: block;
    }

    .scene-floor {
      position: absolute;
      left: 16%;
      right: 12%;
      bottom: -12%;
      height: 58%;
      background-image:
        linear-gradient(rgba(56, 189, 248, 0.13) 1px, transparent 1px),
        linear-gradient(90deg, rgba(56, 189, 248, 0.13) 1px, transparent 1px);
      background-size: 42px 42px;
      transform: perspective(850px) rotateX(62deg);
      transform-origin: center bottom;
      opacity: 0.55;
      mask-image: linear-gradient(to top, black, transparent);
      pointer-events: none;
      z-index: 1;
    }

    .scene-object {
      position: absolute;
      left: var(--x);
      top: var(--y);
      transform: translate(-50%, -50%);
      display: grid;
      justify-items: center;
      gap: 10px;
      cursor: pointer;
      user-select: none;
      z-index: 5;
    }

    .object-cube {
      width: 96px;
      height: 96px;
      display: grid;
      place-items: center;
      border-radius: 24px;
      font-size: 44px;
      background: linear-gradient(135deg, rgba(56,189,248,0.92), rgba(167,139,250,0.92));
      box-shadow: 0 24px 70px rgba(56,189,248,0.24);
      transform: rotateX(58deg) rotateZ(45deg);
      transition: 160ms ease;
    }

    .scene-object span {
      padding: 7px 12px;
      border-radius: 999px;
      background: rgba(2, 6, 23, 0.72);
      border: 1px solid rgba(148, 163, 184, 0.18);
      color: #dbeafe;
      font-size: 13px;
      font-weight: 800;
    }

    .scene-object:hover .object-cube,
    .scene-object.active .object-cube {
      box-shadow:
        0 0 0 2px rgba(56,189,248,0.7),
        0 28px 90px rgba(56,189,248,0.42);
      transform: rotateX(58deg) rotateZ(45deg) translateY(-8px);
    }

    .engine-topbar {
      position: absolute;
      top: 0;
      left: 0;
      right: 0;
      height: 52px;
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 0 20px;
      background: rgba(2, 6, 23, 0.82);
      border-bottom: 1px solid var(--panel-border);
      backdrop-filter: blur(12px);
      z-index: 10;
    }

    .engine-topbar strong {
      font-size: 14px;
      letter-spacing: -0.02em;
    }

    #close-chefos {
      appearance: none;
      border: 1px solid var(--panel-border);
      border-radius: 10px;
      padding: 7px 14px;
      background: rgba(15, 23, 42, 0.75);
      color: var(--muted);
      font-size: 13px;
      cursor: pointer;
    }

    #close-chefos:hover {
      color: var(--text);
      border-color: rgba(148, 163, 184, 0.4);
    }

    .engine-toolbar {
      position: absolute;
      left: 20px;
      top: 72px;
      display: grid;
      gap: 8px;
      z-index: 10;
    }

    .engine-toolbar button {
      appearance: none;
      width: 80px;
      padding: 10px 0;
      border-radius: 12px;
      border: 1px solid var(--panel-border);
      background: rgba(2, 6, 23, 0.74);
      color: var(--muted);
      font-size: 12px;
      font-weight: 600;
      cursor: pointer;
      text-align: center;
    }

    .engine-toolbar button:hover {
      color: var(--text);
      border-color: var(--accent);
    }

    .engine-toolbar button.active {
      color: #020617;
      background: linear-gradient(135deg, #38bdf8, #67e8f9);
      border-color: transparent;
    }

    .engine-inspector {
      position: absolute;
      right: 20px;
      top: 72px;
      width: 220px;
      padding: 18px;
      border-radius: 20px;
      border: 1px solid var(--panel-border);
      background: rgba(2, 6, 23, 0.78);
      backdrop-filter: blur(14px);
      z-index: 10;
    }

    .engine-inspector h3 {
      margin: 0 0 12px;
      font-size: 13px;
      color: var(--text);
    }

    .inspector-value {
      display: flex;
      justify-content: space-between;
      gap: 14px;
      padding: 9px 0;
      border-top: 1px solid rgba(148, 163, 184, 0.14);
    }

    .inspector-value span {
      color: #64748b;
      font-size: 12px;
      font-weight: 800;
    }

    .inspector-value strong {
      color: #38bdf8;
      font-size: 13px;
      text-align: right;
    }

    .engine-inspector p {
      margin: 6px 0;
      font-size: 12px;
      color: var(--muted);
    }

    /* ── Command Bar ── */
    .engine-command-bar {
      position: absolute;
      left: 50%;
      bottom: 24px;
      transform: translateX(-50%);
      min-width: min(920px, calc(100vw - 48px));
      display: grid;
      grid-template-columns: 180px 1fr auto;
      gap: 18px;
      align-items: center;
      padding: 14px 16px;
      border-radius: 20px;
      background: rgba(2, 6, 23, 0.74);
      border: 1px solid rgba(148, 163, 184, 0.18);
      backdrop-filter: blur(18px);
      box-shadow: 0 24px 80px rgba(0, 0, 0, 0.36);
      z-index: 10;
    }

    .command-label {
      display: block;
      margin-bottom: 4px;
      color: #64748b;
      font-size: 11px;
      font-weight: 800;
      text-transform: uppercase;
      letter-spacing: 0.08em;
    }

    .engine-command-bar strong {
      color: #e5e7eb;
      font-size: 14px;
    }

    .command-actions {
      display: flex;
      gap: 8px;
    }

    .command-actions button {
      min-height: 38px;
      padding: 0 13px;
      border-radius: 12px;
      border: 1px solid rgba(148, 163, 184, 0.18);
      color: #dbeafe;
      background: rgba(15, 23, 42, 0.78);
      font-weight: 800;
      cursor: pointer;
      font-size: 13px;
    }

    .command-actions button:hover {
      color: #020617;
      background: linear-gradient(135deg, #38bdf8, #67e8f9);
    }

    @media (max-width: 920px) {
      .engine-command-bar {
        grid-template-columns: 1fr;
      }
      .command-actions {
        flex-wrap: wrap;
      }
    }

    @media (max-width: 980px) {
      main {
        grid-template-columns: 1fr;
      }

      .viewport-card {
        min-height: 460px;
      }

      nav {
        display: none;
      }
    }
  </style>
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

  <!-- ── Fullscreen Render Screen ── -->
  <section id="render-screen">
    <canvas id="webgpu-canvas"></canvas>

    <!-- fake scene -->
    <div class="scene-floor"></div>

    <div
      class="scene-object active"
      data-name="Salmon"
      data-type="Ingredient Card"
      data-kcal="208 kcal"
      data-info="Protein 20g · Fat 13g"
      data-action="Salmon · ready for recipe"
      style="--x: 44%; --y: 44%;"
    >
      <div class="object-cube">🐟</div>
      <span>Salmon</span>
    </div>

    <div
      class="scene-object"
      data-name="Flour"
      data-type="Ingredient Card"
      data-kcal="364 kcal"
      data-info="Baking · Density · Gluten"
      data-action="Flour · calculate grams/cups"
      style="--x: 58%; --y: 50%;"
    >
      <div class="object-cube">🌾</div>
      <span>Flour</span>
    </div>

    <div
      class="scene-object"
      data-name="Strawberry"
      data-type="Ingredient Card"
      data-kcal="32 kcal"
      data-info="Vitamin C · Dessert · Antioxidant"
      data-action="Strawberry · add to dessert"
      style="--x: 36%; --y: 58%;"
    >
      <div class="object-cube">🍓</div>
      <span>Strawberry</span>
    </div>

    <div class="engine-topbar">
      <strong>ChefOS Render Scene</strong>
      <button id="close-chefos">← Назад</button>
    </div>

    <div class="engine-toolbar">
      <button class="active">Select</button>
      <button>Move</button>
      <button>Object</button>
      <button>Recipe</button>
    </div>

    <div class="engine-inspector">
      <h3>Selected Object</h3>

      <div class="inspector-value">
        <span>Name</span>
        <strong id="selected-name">Salmon</strong>
      </div>

      <div class="inspector-value">
        <span>Type</span>
        <strong id="selected-type">Ingredient Card</strong>
      </div>

      <div class="inspector-value">
        <span>Nutrition</span>
        <strong id="selected-kcal">208 kcal</strong>
      </div>

      <p id="selected-info">Protein 20g · Fat 13g</p>
    </div>

    <div class="engine-command-bar">
      <div>
        <span class="command-label">Current Tool</span>
        <strong id="current-tool-label">Select mode</strong>
      </div>

      <div>
        <span class="command-label">Selected Action</span>
        <strong id="selected-action-title">Salmon · ready for recipe</strong>
      </div>

      <div class="command-actions">
        <button>Add to recipe</button>
        <button>Add to inventory</button>
        <button>Nutrition</button>
        <button>Food cost</button>
      </div>
    </div>
  </section>

  <script>
    const openButton = document.getElementById("open-chefos");
    const closeButton = document.getElementById("close-chefos");

    openButton?.addEventListener("click", () => {
      document.body.classList.add("engine-open");
    });

    closeButton?.addEventListener("click", () => {
      document.body.classList.remove("engine-open");
    });

    const selectedName = document.getElementById("selected-name");
    const selectedType = document.getElementById("selected-type");
    const selectedKcal = document.getElementById("selected-kcal");
    const selectedInfo = document.getElementById("selected-info");
    const selectedActionTitle = document.getElementById("selected-action-title");
    const currentToolLabel = document.getElementById("current-tool-label");

    document.querySelectorAll(".scene-object").forEach((object) => {
      object.addEventListener("click", () => {
        document.querySelectorAll(".scene-object").forEach((item) => {
          item.classList.remove("active");
        });

        object.classList.add("active");

        if (selectedName) selectedName.textContent = object.dataset.name || "Object";
        if (selectedType) selectedType.textContent = object.dataset.type || "Scene Object";
        if (selectedKcal) selectedKcal.textContent = object.dataset.kcal || "—";
        if (selectedInfo) selectedInfo.textContent = object.dataset.info || "No data";
        if (selectedActionTitle) selectedActionTitle.textContent = object.dataset.action || "Ready";
      });
    });

    const toolLabels = {
      "Select": "Select mode",
      "Move":   "Move mode · drag objects",
      "Object": "Object mode · edit properties",
      "Recipe": "Recipe mode · build dish"
    };

    document.querySelectorAll(".engine-toolbar button").forEach((button) => {
      button.addEventListener("click", () => {
        document.querySelectorAll(".engine-toolbar button").forEach((item) => {
          item.classList.remove("active");
        });

        button.classList.add("active");

        const label = button.textContent.trim();
        if (currentToolLabel) {
          currentToolLabel.textContent = toolLabels[label] || label;
        }
      });
    });
  </script>
</body>
</html>"#)
}
