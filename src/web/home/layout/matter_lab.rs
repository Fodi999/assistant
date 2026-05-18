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
          <div class="sco-title" id="sco-drag-handle"><span class="sco-grip">⠿</span>Горячие клавиши <button id="shortcuts-close">✕</button></div>
          <input id="sco-search" type="text" placeholder="Поиск…" autocomplete="off"
                 oninput="(function(v){var rows=document.querySelectorAll('#shortcuts-overlay .sco-row');rows.forEach(function(r){r.style.display=r.dataset.kw.indexOf(v.toLowerCase())>=0?'contents':'none';});})(this.value)"
                 style="width:100%;box-sizing:border-box;margin-bottom:8px;padding:4px 8px;background:rgba(255,255,255,0.07);border:1px solid rgba(56,189,248,0.25);border-radius:6px;color:#e2e8f0;font:500 12px/1.5 'JetBrains Mono',monospace;outline:none;">
          <div class="sco-grid" id="sco-list">
            <span class="sco-key sco-row" data-kw="s выбор select">S</span><span class="sco-row" data-kw="s выбор select">Выбор</span>
            <span class="sco-key sco-row" data-kw="p точка point">P</span><span class="sco-row" data-kw="p точка point">Точка</span>
            <span class="sco-key sco-row" data-kw="l линия line">L</span><span class="sco-row" data-kw="l линия line">Линия</span>
            <span class="sco-key sco-row" data-kw="g захват grab">G</span><span class="sco-row" data-kw="g захват grab">Захват</span>
            <span class="sco-key sco-row" data-kw="shift g копировать copy">⇧G</span><span class="sco-row" data-kw="shift g копировать copy">Копировать</span>
            <span class="sco-key sco-row" data-kw="d размер dimension">D</span><span class="sco-row" data-kw="d размер dimension">Размер</span>
            <span class="sco-key sco-row" data-kw="w разбить ребро split edge">W</span><span class="sco-row" data-kw="w разбить ребро split edge">Разбить ребро (Split Edge)</span>
            <span class="sco-key sco-row" data-kw="f зафиксировать fix">F</span><span class="sco-row" data-kw="f зафиксировать fix">Зафиксировать</span>
            <span class="sco-key sco-row" data-kw="h горизонталь horizontal">H</span><span class="sco-row" data-kw="h горизонталь horizontal">Горизонталь</span>
            <span class="sco-key sco-row" data-kw="v вертикаль vertical">V</span><span class="sco-row" data-kw="v вертикаль vertical">Вертикаль</span>
            <span class="sco-key sco-row" data-kw="j проекция project">J</span><span class="sco-row" data-kw="j проекция project">Проекция</span>
            <span class="sco-key sco-row" data-kw="o ортогональность ortho">O</span><span class="sco-row" data-kw="o ортогональность ortho">Ортогональность</span>
            <span class="sco-key sco-row" data-kw="1 2 3 плоскость plane">1/2/3</span><span class="sco-row" data-kw="1 2 3 плоскость plane">Сменить плоскость</span>
            <span class="sco-key sco-row" data-kw="space пробел центр center">Space</span><span class="sco-row" data-kw="space пробел центр center">Центровать сцену</span>
            <span class="sco-key sco-row" data-kw="del delete удалить">⌫</span><span class="sco-row" data-kw="del delete удалить">Удалить</span>
            <span class="sco-key sco-row" data-kw="ctrl z отменить undo">⌘Z</span><span class="sco-row" data-kw="ctrl z отменить undo">Отменить</span>
            <span class="sco-key sco-row" data-kw="shift p fps счётчик">⇧P</span><span class="sco-row" data-kw="shift p fps счётчик">Счётчик FPS</span>
            <span class="sco-key sco-row" data-kw="esc отмена cancel">Esc</span><span class="sco-row" data-kw="esc отмена cancel">Отмена</span>
          </div>
        </div>

        <!-- Shortcuts overlay drag logic -->
        <script>
          (function() {
            function initScoDrag() {
              var overlay = document.getElementById('shortcuts-overlay');
              var handle  = document.getElementById('sco-drag-handle');
              var closeBtn = document.getElementById('shortcuts-close');
              if (!overlay || !handle) return;
              var dragging = false, ox = 0, oy = 0;

              // ── Close button ──────────────────────────────────────────────
              if (closeBtn) {
                closeBtn.addEventListener('click', function(e) {
                  overlay.style.display = 'none';
                  e.stopPropagation();
                  e.preventDefault();
                }, true);
              }

              // Block canvas orbit/pick from firing through the overlay.
              // Use bubble phase (false) — capture=true would block child elements (handle, close btn).
              // Overlay and canvas are siblings, so bubble never reaches canvas anyway.
              ['pointerdown','mousedown','pointermove','mousemove','pointerup','mouseup'].forEach(function(ev) {
                overlay.addEventListener(ev, function(e) { e.stopPropagation(); }, false);
              });
              ['click','dblclick','contextmenu'].forEach(function(ev) {
                overlay.addEventListener(ev, function(e) { e.stopPropagation(); }, false);
              });

              handle.addEventListener('pointerdown', function(e) {
                if (e.button !== 0) return;
                if (e.target.closest('input, button')) return;
                dragging = true;
                var r = overlay.getBoundingClientRect();
                ox = e.clientX - r.left;
                oy = e.clientY - r.top;
                overlay.style.transform = 'none';
                overlay.style.left = r.left + 'px';
                overlay.style.top  = r.top  + 'px';
                handle.setPointerCapture(e.pointerId);
                handle.style.cursor = 'grabbing';
                e.preventDefault();
                e.stopPropagation();
              }, false);

              handle.addEventListener('pointermove', function(e) {
                if (!dragging) return;
                var vw = window.innerWidth, vh = window.innerHeight;
                var left = Math.max(0, Math.min(vw - overlay.offsetWidth,  e.clientX - ox));
                var top  = Math.max(0, Math.min(vh - overlay.offsetHeight, e.clientY - oy));
                overlay.style.left = left + 'px';
                overlay.style.top  = top  + 'px';
                e.stopPropagation();
              }, false);

              handle.addEventListener('pointerup', function(e) {
                if (!dragging) return;
                dragging = false;
                handle.style.cursor = 'grab';
                e.stopPropagation();
              }, false);

              handle.addEventListener('pointercancel', function() {
                dragging = false; handle.style.cursor = 'grab';
              });
            }
            if (document.readyState === 'loading') {
              document.addEventListener('DOMContentLoaded', initScoDrag);
            } else { initScoDrag(); }
          })();
        </script>

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
          <div class="utb-sep"></div>
          <button class="utb-btn" id="btn-help" title="Справка по клавишам"
                  onclick="if(window.__toggleShortcutsOverlay)window.__toggleShortcutsOverlay();else{var o=document.getElementById('shortcuts-overlay');o.style.display=(o.style.display==='none'?'':'none');}">
            ?<span class="utb-label">Справка</span>
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
