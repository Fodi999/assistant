pub fn template(styles: &str, scripts: &str) -> String {
    let matter = crate::web::home::layout::matter_lab::matter_lab_section();
    let head = r##"<!doctype html>
<!-- ChefOS Interactive Engine — v2: fullscreen render mode -->
<html lang="ru">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>ChefOS Interactive Engine</title>
  <meta name="description" content="Интерактивная операционная система для шеф-повара: рецепты, склад, ингредиенты, себестоимость и лаборатория в одном игровом интерфейсе." />
  <!-- Preload WASM module so the browser fetches it in parallel with other resources -->
  <link rel="modulepreload" href="/wasm/sketch_engine/sketch_engine.js" />
  <link rel="preload" href="/wasm/sketch_engine/sketch_engine_bg.wasm" as="fetch" type="application/wasm" crossorigin="anonymous" />
  <style>"##;

    let mid = r##"</style>
</head>

<body class="engine-open">

  <!-- ── Render Screen (Matter Lab) ── -->
"##;

    let tail = r##"
</body>
</html>"##;

    [head, styles, mid, &matter, scripts, tail].concat()
}
