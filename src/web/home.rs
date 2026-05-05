use axum::response::{Html, IntoResponse};

pub async fn home_page() -> impl IntoResponse {
    Html(r#"<!doctype html>
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
          <a class="button" href="/app/laboratory">Открыть лабораторию</a>
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
</body>
</html>"#)
}
