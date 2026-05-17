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
            <button class="csp-tab active" data-tab="grid"   title="Grid & Units"   onclick="window.__cadPanelTab('grid')">
              <span class="csp-tab-icon">▦</span>
              <span class="csp-tab-label">Grid</span>
            </button>
            <button class="csp-tab" data-tab="snap"   title="Snapping & Ortho" onclick="window.__cadPanelTab('snap')">
              <span class="csp-tab-icon">⌘</span>
              <span class="csp-tab-label">Snap</span>
            </button>
            <button class="csp-tab" data-tab="view"   title="Viewport & Shading" onclick="window.__cadPanelTab('view')">
              <span class="csp-tab-icon">◐</span>
              <span class="csp-tab-label">View</span>
            </button>
            <button class="csp-tab" data-tab="object" title="Selection & Objects" onclick="window.__cadPanelTab('object')">
              <span class="csp-tab-icon">⬚</span>
              <span class="csp-tab-label">Object</span>
            </button>
            <button class="csp-tab" data-tab="dev"    title="Dev & Debug"       onclick="window.__cadPanelTab('dev')">
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
                  <span>GRID</span>
                  <span class="csp-caret">▾</span>
                </button>
                <div class="csp-section-body">

                  <!-- toggles -->
                  <label class="csp-row csp-check">
                    <input type="checkbox" id="csp-show-grid" checked>
                    <span>Show grid</span>
                  </label>
                  <label class="csp-row csp-check">
                    <input type="checkbox" id="csp-snap-grid" checked>
                    <span>Snap to grid</span>
                  </label>

                  <!-- slider row -->
                  <div class="csp-row" style="flex-direction:column; gap:6px; align-items:stretch;">
                    <div style="display:flex; justify-content:space-between; align-items:center;">
                      <span class="csp-lbl">Grid step</span>
                      <div style="display:flex; align-items:center; gap:4px;">
                        <input id="csp-grid-step-num"
                               type="number" min="1" max="100" step="1" value="10"
                               style="width:52px; background:rgba(0,0,0,0.5); color:#e2e8f0;
                                      border:1px solid rgba(103,232,249,0.25); border-radius:6px;
                                      padding:2px 6px; font:inherit; text-align:right; font-size:12px;">
                        <span class="csp-unit">mm</span>
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

                  <button class="csp-btn-sm" style="margin-top:4px;" onclick="window.__cadResetGrid()">↺ Reset</button>
                </div>
              </div>

            </div><!-- /page:grid -->

            <!-- ════ TAB: SNAP ════ -->
            <div class="csp-page" data-page="snap" style="display:none;">

              <div class="csp-section" data-section="snapping">
                <button class="csp-section-hdr open" onclick="window.__cadPanelToggleSection('snapping')">
                  <span>SNAPPING</span>
                  <span class="csp-caret">▾</span>
                </button>
                <div class="csp-section-body">
                  <label class="csp-row csp-check">
                    <input type="checkbox" id="csp-snap-to-grid"  checked>
                    <span>Grid</span>
                  </label>
                  <label class="csp-row csp-check">
                    <input type="checkbox" id="csp-snap-to-point" checked>
                    <span>Point</span>
                  </label>
                  <label class="csp-row csp-check">
                    <input type="checkbox" id="csp-snap-endpoint" checked>
                    <span>Endpoint</span>
                  </label>
                  <label class="csp-row csp-check">
                    <input type="checkbox" id="csp-snap-midpoint">
                    <span>Midpoint</span>
                  </label>
                  <label class="csp-row csp-check">
                    <input type="checkbox" id="csp-snap-center">
                    <span>Center</span>
                  </label>
                  <label class="csp-row csp-check">
                    <input type="checkbox" id="csp-snap-intersect">
                    <span>Intersection</span>
                  </label>
                  <div class="csp-divider"></div>
                  <label class="csp-row csp-check">
                    <input type="checkbox" id="csp-snap-ortho">
                    <span>Ortho lock (O)</span>
                  </label>
                  <div class="csp-row">
                    <span class="csp-lbl">Precision</span>
                    <span class="csp-val">Alt</span>
                  </div>
                </div>
              </div>

            </div><!-- /page:snap -->

            <!-- ════ TAB: VIEW ════ -->
            <div class="csp-page" data-page="view" style="display:none;">

              <div class="csp-section" data-section="shader">
                <button class="csp-section-hdr open" onclick="window.__cadPanelToggleSection('shader')">
                  <span>SHADER MODE</span>
                  <span class="csp-caret">▾</span>
                </button>
                <div class="csp-section-body">
                  <div class="csp-radio-group" id="csp-shader-group">
                    <label class="csp-radio"><input type="radio" name="csp-shader" value="wireframe"> Wireframe</label>
                    <label class="csp-radio"><input type="radio" name="csp-shader" value="solid" checked> Solid</label>
                    <label class="csp-radio"><input type="radio" name="csp-shader" value="rendered"> Rendered</label>
                  </div>
                </div>
              </div>

              <div class="csp-section" data-section="camera">
                <button class="csp-section-hdr" onclick="window.__cadPanelToggleSection('camera')">
                  <span>CAMERA</span>
                  <span class="csp-caret">▸</span>
                </button>
                <div class="csp-section-body" style="display:none;">
                  <div class="csp-radio-group">
                    <label class="csp-radio"><input type="radio" name="csp-proj" value="perspective" checked> Perspective</label>
                    <label class="csp-radio"><input type="radio" name="csp-proj" value="ortho"> Orthographic</label>
                  </div>
                  <div class="csp-divider"></div>
                  <div class="csp-btn-row">
                    <button class="csp-btn-sm" onclick="window.__snapView && window.__snapView('front')">Front</button>
                    <button class="csp-btn-sm" onclick="window.__snapView && window.__snapView('top')">Top</button>
                    <button class="csp-btn-sm" onclick="window.__snapView && window.__snapView('right')">Right</button>
                    <button class="csp-btn-sm" onclick="window.__snapView && window.__snapView('iso')">Iso</button>
                  </div>
                  <button class="csp-btn-sm csp-btn-full" onclick="window.__resetCamera && window.__resetCamera()">Reset Camera</button>
                </div>
              </div>

              <div class="csp-section" data-section="helpers">
                <button class="csp-section-hdr open" onclick="window.__cadPanelToggleSection('helpers')">
                  <span>HELPERS</span>
                  <span class="csp-caret">▾</span>
                </button>
                <div class="csp-section-body">
                  <label class="csp-row csp-check">
                    <input type="checkbox" id="csp-show-orbit-guide">
                    <span>Show orbit guide</span>
                  </label>
                  <label class="csp-row csp-check">
                    <input type="checkbox" id="csp-show-projection-guide">
                    <span>Show projection guide</span>
                  </label>
                  <label class="csp-row csp-check">
                    <input type="checkbox" id="csp-fade-bg-helpers" checked>
                    <span>Fade background helpers</span>
                  </label>
                </div>
              </div>

            </div><!-- /page:view -->

            <!-- ════ TAB: OBJECT ════ -->
            <div class="csp-page" data-page="object" style="display:none;">

              <div class="csp-section" data-section="selection">
                <button class="csp-section-hdr open" onclick="window.__cadPanelToggleSection('selection')">
                  <span>SELECTION</span>
                  <span class="csp-caret">▾</span>
                </button>
                <div class="csp-section-body">
                  <div class="csp-row"><span class="csp-lbl">Type</span>   <span id="csp-sel-type"  class="csp-val">—</span></div>
                  <div class="csp-row"><span class="csp-lbl">Length</span> <span id="csp-sel-len"   class="csp-val">—</span></div>
                  <div class="csp-row"><span class="csp-lbl">Angle</span>  <span id="csp-sel-angle" class="csp-val">—</span></div>
                  <div class="csp-row"><span class="csp-lbl">Plane</span>  <span id="csp-sel-plane" class="csp-val">XZ</span></div>
                </div>
              </div>

              <div class="csp-section" data-section="analyze">
                <button class="csp-section-hdr" onclick="window.__cadPanelToggleSection('analyze')">
                  <span>ANALYZE</span>
                  <span class="csp-caret">▸</span>
                </button>
                <div class="csp-section-body" style="display:none;">
                  <div class="csp-row">
                    <span class="csp-lbl">Status</span>
                    <span id="csp-analyze-status" class="csp-val csp-status-ok">OK</span>
                  </div>
                  <div id="csp-analyze-errors" class="csp-error-list" style="display:none;"></div>
                  <div class="csp-divider"></div>
                  <div class="csp-btn-row">
                    <button class="csp-btn-sm" onclick="window.__cadAction && window.__cadAction('make_rect')">Make Rect</button>
                    <button class="csp-btn-sm" onclick="window.__cadAction && window.__cadAction('equalize')">Equalize</button>
                  </div>
                  <div class="csp-btn-row">
                    <button class="csp-btn-sm" onclick="window.__cadAction && window.__cadAction('snap_to_grid')">Snap to Grid</button>
                    <button class="csp-btn-sm" onclick="window.__cadAction && window.__cadAction('lock_constraints')">Lock</button>
                  </div>
                </div>
              </div>

            </div><!-- /page:object -->

            <!-- ════ TAB: DEV ════ -->
            <div class="csp-page" data-page="dev" style="display:none;">

              <div class="csp-section" data-section="engine">
                <button class="csp-section-hdr open" onclick="window.__cadPanelToggleSection('engine')">
                  <span>CAD ENGINE</span>
                  <span class="csp-caret">▾</span>
                </button>
                <div class="csp-section-body">
                  <div class="csp-row"><span class="csp-lbl">WASM</span>    <span id="csp-wasm-status" class="csp-val">—</span></div>
                  <div class="csp-row"><span class="csp-lbl">Backend</span> <span id="csp-be-status"   class="csp-val">—</span></div>
                  <div class="csp-row"><span class="csp-lbl">WASM ms</span> <span id="csp-wasm-ms"    class="csp-val">—</span></div>
                  <div class="csp-row"><span class="csp-lbl">BE ms</span>   <span id="csp-be-ms"      class="csp-val">—</span></div>
                  <div class="csp-row"><span class="csp-lbl">Pending</span> <span id="csp-pending"    class="csp-val">0</span></div>
                </div>
              </div>

              <div class="csp-section" data-section="devjson">
                <button class="csp-section-hdr" onclick="window.__cadPanelToggleSection('devjson')">
                  <span>JSON EXPORT</span>
                  <span class="csp-caret">▸</span>
                </button>
                <div class="csp-section-body" style="display:none;">
                  <div class="csp-btn-row">
                    <button class="csp-btn-sm" onclick="window.__sioRefresh && window.__sioRefresh()">↻ Refresh</button>
                    <button class="csp-btn-sm" onclick="window.__sioCopy   && window.__sioCopy()">⧉ Copy</button>
                    <button class="csp-btn-sm" onclick="window.__sioDownload && window.__sioDownload()">⬇ Save</button>
                  </div>
                  <pre id="csp-json-preview" class="csp-json-pre">{}</pre>
                </div>
              </div>

              <div class="csp-section" data-section="devsnapstate">
                <button class="csp-section-hdr" onclick="window.__cadPanelToggleSection('devsnapstate')">
                  <span>SNAP DEBUG</span>
                  <span class="csp-caret">▸</span>
                </button>
                <div class="csp-section-body" style="display:none;">
                  <div class="csp-row"><span class="csp-lbl">Kind</span>  <span id="csp-snap-kind"  class="csp-val">—</span></div>
                  <div class="csp-row"><span class="csp-lbl">Grid</span>  <span id="csp-snap-gxyz"  class="csp-val">—</span></div>
                  <div class="csp-row"><span class="csp-lbl">World</span> <span id="csp-snap-world" class="csp-val">—</span></div>
                </div>
              </div>

            </div><!-- /page:dev -->

          </div><!-- /.csp-body -->
        </div><!-- /#cad-side-panel -->
"##
}
