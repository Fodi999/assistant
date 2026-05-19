// ── CAD Side Panel — Plasticity-style floating right panel ──────────────────
// Tabs: View · Grid · Snap · Object · Dev
// Each tab contains accordion sections.

pub fn cad_side_panel_html() -> &'static str {
    r##"
        <!-- ═══════════════════════════════════════════════════════
             CAD SIDE PANEL  (floating right, tab + accordion)
             ═══════════════════════════════════════════════════ -->
        <div id="cad-side-panel">

          <!-- ── Top icon tabs ── -->
          <nav class="csp-tabs" role="tablist">
            <button class="csp-tab active" data-tab="grid"   title="Сетка и единицы"   onclick="window.__cadPanelTab('grid')">
              <span class="csp-tab-icon">▦</span>
              <span class="csp-tab-label">Сетка</span>
            </button>
            <button class="csp-tab" data-tab="snap"   title="Привязка и ортогональность" onclick="window.__cadPanelTab('snap')">
              <span class="csp-tab-icon">⌘</span>
              <span class="csp-tab-label">Привязка</span>
            </button>
            <button class="csp-tab" data-tab="view"   title="Вид и отображение" onclick="window.__cadPanelTab('view')">
              <span class="csp-tab-icon">◐</span>
              <span class="csp-tab-label">Вид</span>
            </button>
            <button class="csp-tab" data-tab="object" title="Выделение и объекты" onclick="window.__cadPanelTab('object')">
              <span class="csp-tab-icon">⬚</span>
              <span class="csp-tab-label">Объект</span>
            </button>
            <button class="csp-tab" data-tab="dev"    title="Разработка и отладка" onclick="window.__cadPanelTab('dev')">
              <span class="csp-tab-icon">⚙</span>
              <span class="csp-tab-label">Dev</span>
            </button>
          </nav>

          <!-- ── Tab body ── -->
          <div class="csp-body">

            <!-- ════ TAB: GRID ════ -->
            <div class="csp-page" data-page="grid">

              <!-- GRID section -->
              <div class="csp-section" data-section="grid">
                <button class="csp-section-hdr open" onclick="window.__cadPanelToggleSection('grid')">
                  <span>СЕТКА</span>
                  <span class="csp-caret">▾</span>
                </button>
                <div class="csp-section-body">

                  <!-- toggles -->
                  <label class="csp-row csp-check">
                    <input type="checkbox" id="csp-show-grid" checked>
                    <span>Показать сетку</span>
                  </label>
                  <label class="csp-row csp-check">
                    <input type="checkbox" id="csp-snap-grid" checked>
                    <span>Привязка к сетке</span>
                  </label>

                  <!-- slider row -->
                  <div class="csp-row" style="flex-direction:column; gap:6px; align-items:stretch;">
                    <div style="display:flex; justify-content:space-between; align-items:center;">
                      <span class="csp-lbl">Шаг сетки</span>
                      <div style="display:flex; align-items:center; gap:4px;">
                        <input id="csp-grid-step-num"
                               type="number" min="1" max="100" step="1" value="10"
                               style="width:52px; background:rgba(0,0,0,0.5); color:#e2e8f0;
                                      border:1px solid rgba(103,232,249,0.25); border-radius:6px;
                                      padding:2px 6px; font:inherit; text-align:right; font-size:12px;">
                        <span class="csp-unit">мм</span>
                      </div>
                    </div>
                    <input id="csp-grid-step-slider"
                           type="range" min="1" max="100" step="1" value="10"
                           style="width:100%; accent-color:#67e8f9; cursor:pointer;">
                    <div style="display:flex; gap:4px; flex-wrap:wrap;">
                      <button class="csp-preset-btn" onclick="window.__cadSetGrid(1)">1</button>
                      <button class="csp-preset-btn" onclick="window.__cadSetGrid(5)">5</button>
                      <button class="csp-preset-btn csp-preset-active" onclick="window.__cadSetGrid(10)">10</button>
                      <button class="csp-preset-btn" onclick="window.__cadSetGrid(25)">25</button>
                      <button class="csp-preset-btn" onclick="window.__cadSetGrid(50)">50</button>
                      <button class="csp-preset-btn" onclick="window.__cadSetGrid(100)">100</button>
                    </div>
                  </div>

                  <button class="csp-btn-sm" style="margin-top:4px;" onclick="window.__cadResetGrid()">↺ Сбросить</button>
                </div>
              </div>

            </div><!-- /page:grid -->

            <!-- ════ TAB: SNAP ════ -->
            <div class="csp-page" data-page="snap" style="display:none;">

              <div class="csp-section" data-section="snapping">
                <button class="csp-section-hdr open" onclick="window.__cadPanelToggleSection('snapping')">
                  <span>ПРИВЯЗКА</span>
                  <span class="csp-caret">▾</span>
                </button>
                <div class="csp-section-body">
                  <label class="csp-row csp-check">
                    <input type="checkbox" id="csp-snap-to-grid"  checked>
                    <span>Сетка</span>
                  </label>
                  <label class="csp-row csp-check">
                    <input type="checkbox" id="csp-snap-to-point" checked>
                    <span>Точка</span>
                  </label>
                  <label class="csp-row csp-check">
                    <input type="checkbox" id="csp-snap-endpoint" checked>
                    <span>Конечная точка</span>
                  </label>
                  <label class="csp-row csp-check">
                    <input type="checkbox" id="csp-snap-midpoint">
                    <span>Средняя точка</span>
                  </label>
                  <label class="csp-row csp-check">
                    <input type="checkbox" id="csp-snap-center">
                    <span>Центр</span>
                  </label>
                  <label class="csp-row csp-check">
                    <input type="checkbox" id="csp-snap-intersect">
                    <span>Пересечение</span>
                  </label>
                  <div class="csp-divider"></div>
                  <label class="csp-row csp-check">
                    <input type="checkbox" id="csp-snap-ortho">
                    <span>Ортогональность (O)</span>
                  </label>
                  <div class="csp-row">
                    <span class="csp-lbl">Точность</span>
                    <span class="csp-val">Alt</span>
                  </div>
                </div>
              </div>

            </div><!-- /page:snap -->

            <!-- ════ TAB: VIEW ════ -->
            <div class="csp-page" data-page="view" style="display:none;">

              <div class="csp-section" data-section="shader">
                <button class="csp-section-hdr open" onclick="window.__cadPanelToggleSection('shader')">
                  <span>РЕЖИМ ОТОБРАЖЕНИЯ</span>
                  <span class="csp-caret">▾</span>
                </button>
                <div class="csp-section-body">
                  <div class="csp-radio-group" id="csp-shader-group">
                    <label class="csp-radio"><input type="radio" name="csp-shader" value="wireframe"> Каркас</label>
                    <label class="csp-radio"><input type="radio" name="csp-shader" value="solid" checked> Твёрдое тело</label>
                    <label class="csp-radio"><input type="radio" name="csp-shader" value="rendered"> Рендер</label>
                  </div>
                </div>
              </div>

              <div class="csp-section" data-section="camera">
                <button class="csp-section-hdr" onclick="window.__cadPanelToggleSection('camera')">
                  <span>КАМЕРА</span>
                  <span class="csp-caret">▸</span>
                </button>
                <div class="csp-section-body" style="display:none;">
                  <div class="csp-radio-group">
                    <label class="csp-radio"><input type="radio" name="csp-proj" value="perspective" checked> Перспектива</label>
                    <label class="csp-radio"><input type="radio" name="csp-proj" value="ortho"> Ортографическая</label>
                  </div>
                  <div class="csp-divider"></div>
                  <div class="csp-btn-row">
                    <button class="csp-btn-sm" onclick="window.__snapView && window.__snapView('front')">Спереди</button>
                    <button class="csp-btn-sm" onclick="window.__snapView && window.__snapView('top')">Сверху</button>
                    <button class="csp-btn-sm" onclick="window.__snapView && window.__snapView('right')">Справа</button>
                    <button class="csp-btn-sm" onclick="window.__snapView && window.__snapView('iso')">Изо</button>
                  </div>
                  <button class="csp-btn-sm csp-btn-full" onclick="window.__resetCamera && window.__resetCamera()">Сбросить камеру</button>
                </div>
              </div>

              <div class="csp-section" data-section="helpers">
                <button class="csp-section-hdr open" onclick="window.__cadPanelToggleSection('helpers')">
                  <span>ПОМОЩНИКИ</span>
                  <span class="csp-caret">▾</span>
                </button>
                <div class="csp-section-body">
                  <label class="csp-row csp-check">
                    <input type="checkbox" id="csp-show-orbit-guide">
                    <span>Кольцо орбиты</span>
                  </label>
                  <label class="csp-row csp-check">
                    <input type="checkbox" id="csp-show-projection-guide">
                    <span>Направляющие проекции</span>
                  </label>
                  <label class="csp-row csp-check">
                    <input type="checkbox" id="csp-fade-bg-helpers" checked>
                    <span>Затухание помощников</span>
                  </label>
                </div>
              </div>

            </div><!-- /page:view -->

            <!-- ════ TAB: OBJECT ════ -->
            <div class="csp-page" data-page="object" style="display:none;">

              <div class="csp-section" data-section="selection">
                <button class="csp-section-hdr open" onclick="window.__cadPanelToggleSection('selection')">
                  <span>ВЫДЕЛЕНИЕ</span>
                  <span class="csp-caret">▾</span>
                </button>
                <div class="csp-section-body">
                  <div class="csp-row"><span class="csp-lbl">Тип</span>    <span id="csp-sel-type"  class="csp-val">—</span></div>
                  <div class="csp-row"><span class="csp-lbl">Длина</span>  <span id="csp-sel-len"   class="csp-val">—</span></div>
                  <div class="csp-row"><span class="csp-lbl">Угол</span>   <span id="csp-sel-angle" class="csp-val">—</span></div>
                  <div class="csp-row"><span class="csp-lbl">Плоскость</span> <span id="csp-sel-plane" class="csp-val">XZ</span></div>
                </div>
              </div>

              <div class="csp-section" data-section="analyze">
                <button class="csp-section-hdr" onclick="window.__cadPanelToggleSection('analyze')">
                  <span>АНАЛИЗ</span>
                  <span class="csp-caret">▸</span>
                </button>
                <div class="csp-section-body" style="display:none;">
                  <div class="csp-row">
                    <span class="csp-lbl">Статус</span>
                    <span id="csp-analyze-status" class="csp-val csp-status-ok">OK</span>
                  </div>
                  <div id="csp-analyze-errors" class="csp-error-list" style="display:none;"></div>
                  <div class="csp-divider"></div>
                  <div class="csp-btn-row">
                    <button class="csp-btn-sm" onclick="window.__cadAction && window.__cadAction('make_rect')">Прямоугольник</button>
                    <button class="csp-btn-sm" onclick="window.__cadAction && window.__cadAction('equalize')">Уравнять</button>
                  </div>
                  <div class="csp-btn-row">
                    <button class="csp-btn-sm" onclick="window.__cadAction && window.__cadAction('snap_to_grid')">На сетку</button>
                    <button class="csp-btn-sm" onclick="window.__cadAction && window.__cadAction('lock_constraints')">Зафиксировать</button>
                  </div>
                </div>
              </div>

            </div><!-- /page:object -->

            <!-- ════ TAB: DEV ════ -->
            <div class="csp-page" data-page="dev" style="display:none;">

              <div class="csp-section" data-section="engine">
                <button class="csp-section-hdr open" onclick="window.__cadPanelToggleSection('engine')">
                  <span>CAD ДВИЖОК</span>
                  <span class="csp-caret">▾</span>
                </button>
                <div class="csp-section-body">
                  <div class="csp-row"><span class="csp-lbl">WASM</span>       <span id="csp-wasm-status" class="csp-val">—</span></div>
                  <div class="csp-row"><span class="csp-lbl">Бэкенд</span>     <span id="csp-be-status"   class="csp-val">—</span></div>
                  <div class="csp-row"><span class="csp-lbl">WASM мс</span>    <span id="csp-wasm-ms"    class="csp-val">—</span></div>
                  <div class="csp-row"><span class="csp-lbl">BE мс</span>      <span id="csp-be-ms"      class="csp-val">—</span></div>
                  <div class="csp-row"><span class="csp-lbl">В очереди</span>  <span id="csp-pending"    class="csp-val">0</span></div>
                </div>
              </div>

              <div class="csp-section" data-section="solver">
                <button class="csp-section-hdr" onclick="window.__cadPanelToggleSection('solver')">
                  <span>РЕШАТЕЛЬ v2</span>
                  <span class="csp-caret">▸</span>
                </button>
                <div class="csp-section-body" style="display:none;">
                  <div class="csp-row">
                    <button class="csp-btn-sm csp-btn-full" onclick="if(window.__solveSketchWasm)window.__solveSketchWasm();">⚙ Решить (Shift+S)</button>
                  </div>
                  <div class="csp-divider"></div>
                  <div class="csp-row"><span class="csp-lbl">Статус</span>      <span id="csp-solve-status"  class="csp-val">—</span></div>
                  <div class="csp-row"><span class="csp-lbl">Итерации</span>    <span id="csp-solve-iter"    class="csp-val">—</span></div>
                  <div class="csp-row"><span class="csp-lbl">Макс. ошибка</span><span id="csp-solve-maxerr"  class="csp-val">—</span></div>
                  <div class="csp-row"><span class="csp-lbl">Сум. ошибка</span> <span id="csp-solve-toterr"  class="csp-val">—</span></div>
                  <div class="csp-divider"></div>
                  <div class="csp-row"><span class="csp-lbl">DOF</span>         <span id="csp-solve-dof"     class="csp-val">—</span></div>
                  <div class="csp-row"><span class="csp-lbl">DOF статус</span>  <span id="csp-solve-dofst"   class="csp-val">—</span></div>
                  <div class="csp-row"><span class="csp-lbl">Неудовл.</span>    <span id="csp-solve-unsat"   class="csp-val">—</span></div>
                </div>
              </div>

              <div class="csp-section" data-section="devjson">
                <button class="csp-section-hdr" onclick="window.__cadPanelToggleSection('devjson')">
                  <span>ЭКСПОРТ JSON</span>
                  <span class="csp-caret">▸</span>
                </button>
                <div class="csp-section-body" style="display:none;">
                  <div class="csp-btn-row">
                    <button class="csp-btn-sm" onclick="window.__sioRefresh && window.__sioRefresh()">↻ Обновить</button>
                    <button class="csp-btn-sm" onclick="window.__sioCopy   && window.__sioCopy()">⧉ Копировать</button>
                    <button class="csp-btn-sm" onclick="window.__sioDownload && window.__sioDownload()">⬇ Сохранить</button>
                  </div>
                  <pre id="csp-json-preview" class="csp-json-pre">{}</pre>
                </div>
              </div>

              <div class="csp-section" data-section="devsnapstate">
                <button class="csp-section-hdr" onclick="window.__cadPanelToggleSection('devsnapstate')">
                  <span>ОТЛАДКА ПРИВЯЗКИ</span>
                  <span class="csp-caret">▸</span>
                </button>
                <div class="csp-section-body" style="display:none;">
                  <div class="csp-row"><span class="csp-lbl">Тип</span>   <span id="csp-snap-kind"  class="csp-val">—</span></div>
                  <div class="csp-row"><span class="csp-lbl">Сетка</span> <span id="csp-snap-gxyz"  class="csp-val">—</span></div>
                  <div class="csp-row"><span class="csp-lbl">Мир</span>   <span id="csp-snap-world" class="csp-val">—</span></div>
                </div>
              </div>

            </div><!-- /page:dev -->

          </div><!-- /.csp-body -->
        </div><!-- /#cad-side-panel -->
"##
}
