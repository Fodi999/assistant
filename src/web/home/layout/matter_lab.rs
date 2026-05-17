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
        <canvas id="axis-gizmo" width="96" height="96" title="Клик по оси — привязать вид"></canvas>

        <!-- Performance HUD (Shift+P toggle) -->
        <div id="perf-hud" class="perf-hud">
          <div id="perf-hud-header" class="perf-hud-header" title="Клик — свернуть · Shift+P — вкл/выкл">
            <span id="perf-hud-caret">▾</span>
            <span class="perf-hud-title">PERF</span>
          </div>
          <div id="perf-hud-body" class="perf-hud-body">
            <div class="perf-row"><span class="perf-key">FPS</span>      <span id="perf-fps"      class="perf-val">0</span></div>
            <div class="perf-row"><span class="perf-key">Кадр</span>     <span id="perf-frame"    class="perf-val">0 мс</span></div>
            <div class="perf-row"><span class="perf-key">Рендер</span>   <span id="perf-render"   class="perf-val">0 мс</span></div>
            <div class="perf-row"><span class="perf-key">Оверлей</span>  <span id="perf-overlay"  class="perf-val">0 мс</span></div>
            <div class="perf-row"><span class="perf-key">Пикинг</span>   <span id="perf-pick"     class="perf-val">0 мс</span></div>
            <div class="perf-row"><span class="perf-key">Бэкенд</span>   <span id="perf-backend"  class="perf-val">—</span></div>
            <div class="perf-sep"></div>
            <div class="perf-row"><span class="perf-key">Режим</span>    <span id="perf-mode"     class="perf-val">backend</span></div>
            <div class="perf-row"><span class="perf-key">WASM мс</span>  <span id="perf-wasm-ms"  class="perf-val">—</span></div>
            <div class="perf-row"><span class="perf-key">BE мс</span>    <span id="perf-be-ms"    class="perf-val">—</span></div>
            <div class="perf-sep"></div>
            <div class="perf-row"><span class="perf-key">Точки</span>    <span id="perf-pts"      class="perf-val">0</span></div>
            <div class="perf-row"><span class="perf-key">Рёбра</span>    <span id="perf-edges"    class="perf-val">0</span></div>
            <div class="perf-row"><span class="perf-key">Профили</span>  <span id="perf-profiles" class="perf-val">0</span></div>
            <div class="perf-row"><span class="perf-key">Выбр</span>     <span id="perf-selected" class="perf-val">0</span></div>
            <div class="perf-sep"></div>
            <div class="perf-row"><span class="perf-key">DPR</span>      <span id="perf-dpr"      class="perf-val">1.00</span></div>
            <div class="perf-row"><span class="perf-key">Холст</span>    <span id="perf-canvas"   class="perf-val">—</span></div>
            <div class="perf-row"><span class="perf-key">Масштаб</span>  <span id="perf-scale"    class="perf-val">1.00</span></div>
          </div>
        </div>

        <!-- Mini command bar (top center) — MODE · TOOL · PLANE · SNAP only -->
        <div id="mini-bar">
          <span class="mb-cell"><b>Режим</b> <span id="mini-mode">Свободный 3D</span></span>
          <span class="mb-sep">·</span>
          <span class="mb-cell"><b>Инструмент</b> <span id="mini-tool">ВЫБОР</span></span>
          <span class="mb-sep">·</span>
          <span class="mb-cell"><b>Плоскость</b> <span id="mini-plane">XZ</span></span>
          <span class="mb-sep">·</span>
          <span class="mb-cell"><b>Привязка</b> <span id="mini-snap">—</span></span>
          <!-- Shortcuts toggle button -->
          <button id="shortcuts-toggle" title="Горячие клавиши (?)">?</button>
        </div>

        <!-- Shortcuts overlay (hidden by default, toggled by ? button or ? key) -->
        <div id="shortcuts-overlay" style="display:none">
          <div class="sco-title">Горячие клавиши <button id="shortcuts-close">✕</button></div>
          <div class="sco-grid">
            <span class="sco-key">S</span><span>Выбор</span>
            <span class="sco-key">P</span><span>Точка</span>
            <span class="sco-key">L</span><span>Линия</span>
            <span class="sco-key">G</span><span>Захват</span>
            <span class="sco-key">⇧G</span><span>Копировать</span>
            <span class="sco-key">D</span><span>Размер</span>
            <span class="sco-key">F</span><span>Зафиксировать</span>
            <span class="sco-key">H</span><span>Горизонталь</span>
            <span class="sco-key">V</span><span>Вертикаль</span>
            <span class="sco-key">J</span><span>Проекция</span>
            <span class="sco-key">O</span><span>Ортогональность</span>
            <span class="sco-key">1/2/3</span><span>Сменить плоскость</span>
            <span class="sco-key">Space</span><span>Центровать сцену</span>
            <span class="sco-key">⌫</span><span>Удалить</span>
            <span class="sco-key">⌘Z</span><span>Отменить</span>
            <span class="sco-key">⇧P</span><span>Счётчик FPS</span>
            <span class="sco-key">Esc</span><span>Отмена</span>
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
          <button class="plane-pill active" data-plane="XZ" title="Горизонтальная плоскость (1)">XZ</button>
          <button class="plane-pill"        data-plane="XY" title="Фронтальная плоскость (2)">XY</button>
          <button class="plane-pill"        data-plane="YZ" title="Боковая плоскость (3)">YZ</button>
        </div>

        <!-- Universal Toolbar — 5 sketch tools + Ortho Lock -->
        <nav id="universal-toolbar" aria-label="Инструменты эскиза">
          <button class="utb-btn active" data-tool="select" title="Выбор (S)">↖<span class="utb-label">Выбор</span></button>
          <button class="utb-btn"        data-tool="point"  title="Точка (P)">•<span class="utb-label">Точка</span></button>
          <button class="utb-btn"        data-tool="line"   title="Линия (L)">╱<span class="utb-label">Линия</span></button>
          <button class="utb-btn"        data-tool="grab"   title="Захват (G)">✥<span class="utb-label">Захват</span></button>
          <button class="utb-btn"        data-tool="delete" title="Удалить (⌫)">⌫<span class="utb-label">Удалить</span></button>
          <div class="utb-sep"></div>
          <button class="utb-btn" id="btn-ortho" data-toggle="ortho"
                  title="Ортогональность — привязка к 0° 45° 90° (O)"
                  onclick="if(window.__toggleOrthoLock) window.__toggleOrthoLock()">
            ⊾<span class="utb-label">Ортогон.</span>
          </button>
        </nav>
"##;

    let cad_panel = crate::web::home::layout::cad_side_panel::cad_side_panel_html();

    let after_panel = r##"
        <!-- Status bar -->
        <footer class="status-bar">
          <div>
            <span class="online-dot"></span>
            Движок онлайн
            <span class="muted">Rust · WebGPU · Ядро эскиза</span>
            <span id="webgpu-status" class="webgpu-status">⏳ проверка WebGPU…</span>
          </div>
          <div class="perf">
            <span>FPS <b id="fpsValue">—</b></span>
            <span>Кадр <b id="frameValue">—</b></span>
          </div>
        </footer>

      </section>
    </main>
  </section>
"##;

    [before_panel, cad_panel, after_panel].concat()
}
