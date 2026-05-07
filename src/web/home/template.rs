pub fn template(styles: &str, scripts: &str) -> String {
    let matter = crate::web::home::matter_lab::matter_lab_section();
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

  <!-- ── Render Screen (Matter Lab) ── -->
"##;

    let tail = r##"
</body>
</html>"##;

    [head, styles, mid, matter, scripts, tail].concat()
}
