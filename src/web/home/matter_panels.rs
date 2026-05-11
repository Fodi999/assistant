pub fn profile_panel() -> &'static str {
    r#"
        <aside class="matter-panel-right collapsed" id="profile-panel">
          <div class="panel-resizer" id="profile-resizer" title="Drag to resize"></div>
          <button class="panel-toggle-btn tab-m" id="profile-toggle" title="Профиль (M)">Профиль</button>
          <div class="panel-header">Пользователь (Профиль)</div>
          <div class="panel-body">
            <div class="prop-section">
              <div class="prop-title">Chef Status</div>
              <div class="prop-value">Online</div>
            </div>
            <div class="prop-section">
              <div class="prop-title">Role</div>
              <div class="prop-value text-muted">Admin</div>
            </div>
            <hr style="border: 0; border-top: 1px solid rgba(255,255,255,0.1); margin: 16px 0;">
            <div class="prop-section">
              <div class="prop-title" style="margin-bottom: 8px;">Производство / CAD Экспорт</div>
              <div style="font-size: 11px; color:#9ca3af; margin-bottom: 12px; line-height: 1.4;">
                Экспортируйте физическую 3D-модель с учетом точных миллиметровых размеров для использования в сторонних программах (Blender, ЧПУ).
              </div>
              <div class="prop-actions" style="display:flex; flex-direction:column; gap:8px;">
                <button id="btn-export-obj" class="prop-btn highlight" style="width:100%; text-align:center; display:flex; justify-content:center; align-items:center; gap:8px;">
                  <span style="font-size:14px;">⬇️</span> Скачать .OBJ / .MTL файл
                </button>
              </div>
            </div>
          </div>
        </aside>
    "#
}

pub fn properties_panel() -> &'static str {
    r#"
        <aside class="matter-panel-right collapsed" id="properties-panel">
          <div class="panel-resizer" id="properties-resizer" title="Drag to resize"></div>
          <button class="panel-toggle-btn tab-n" id="properties-toggle" title="Свойства (N)">Свойства</button>
          <div class="panel-header">Свойства (N)</div>
          <div class="panel-body">
            
            <div class="prop-section">
              <div class="prop-title" style="margin-bottom:-4px;">Transform</div>
              <hr style="border: 0; border-top: 1px solid rgba(255,255,255,0.1); margin: 8px 0 12px 0;">
              
              <!-- Location -->
              <div class="prop-title" style="text-transform:none; font-weight:normal; margin-bottom: 4px; color: #cbd5e1;">Location:</div>
              <div class="prop-vector">
                <div class="prop-row"><div class="prop-row-label x">X</div><input type="number" id="tf-loc-x" value="0.000" step="0.1"></div>
                <div class="prop-row"><div class="prop-row-label y">Y</div><input type="number" id="tf-loc-y" value="0.000" step="0.1"></div>
                <div class="prop-row"><div class="prop-row-label z">Z</div><input type="number" id="tf-loc-z" value="0.000" step="0.1"></div>
              </div>

              <!-- Rotation -->
              <div class="prop-title" style="text-transform:none; font-weight:normal; margin-bottom: 4px; color: #cbd5e1;">Rotation:</div>
              <div class="prop-vector">
                <div class="prop-row"><div class="prop-row-label x">X</div><input type="number" id="tf-rot-x" value="0" step="1"><span style="color:#64748b;font-size:12px;padding-right:8px;">°</span></div>
                <div class="prop-row"><div class="prop-row-label y">Y</div><input type="number" id="tf-rot-y" value="0" step="1"><span style="color:#64748b;font-size:12px;padding-right:8px;">°</span></div>
                <div class="prop-row"><div class="prop-row-label z">Z</div><input type="number" id="tf-rot-z" value="0" step="1"><span style="color:#64748b;font-size:12px;padding-right:8px;">°</span></div>
              </div>

              <!-- Scale -->
              <div style="display:flex; justify-content:space-between; align-items:center; margin-bottom: 4px;">
                <div class="prop-title" style="text-transform:none; font-weight:normal; color: #cbd5e1; margin-bottom: 0;">Scale:</div>
                <button id="btn-apply-scale" style="background:#334155; border:none; color:#cbd5e1; font-size:11px; padding:2px 6px; border-radius:4px; cursor:pointer;" title="Apply Scale">Apply Scale</button>
              </div>
              <div class="prop-vector" style="margin-bottom: 12px;">
                <div class="prop-row"><div class="prop-row-label x">X</div><input type="number" id="tf-scale-x" value="1.000" step="0.1"></div>
                <div class="prop-row"><div class="prop-row-label y">Y</div><input type="number" id="tf-scale-y" value="1.000" step="0.1"></div>
                <div class="prop-row"><div class="prop-row-label z">Z</div><input type="number" id="tf-scale-z" value="1.000" step="0.1"></div>
              </div>
            </div>
          </div>
        </aside>
    "#
}

pub fn shape_panel() -> &'static str {
    r#"
        <aside class="matter-panel-right collapsed" id="shape-panel">
          <div class="panel-resizer" id="shape-resizer" title="Drag to resize"></div>
          <button class="panel-toggle-btn tab-shape" id="shape-toggle" title="Форма">Форма</button>
          <div class="panel-header">Форма</div>
          <div class="panel-body">
            <!-- Dimensions -->
            <div class="prop-section">
              <div class="prop-title" style="text-transform:none; font-weight:bold; margin-bottom: 4px; color: #f8fafc;">Dimensions</div>
              <div class="prop-vector">
                <div class="prop-row"><div class="prop-row-label x" style="width:50px;">Width</div><input type="number" id="tf-dim-x" value="2.000" step="0.010"><span style="color:#64748b;font-size:12px;padding-right:8px;">m</span></div>
                <div class="prop-row"><div class="prop-row-label y" style="width:50px;">Height</div><input type="number" id="tf-dim-y" value="2.000" step="0.010"><span style="color:#64748b;font-size:12px;padding-right:8px;">m</span></div>
                <div class="prop-row"><div class="prop-row-label z" style="width:50px;">Depth</div><input type="number" id="tf-dim-z" value="2.000" step="0.010"><span style="color:#64748b;font-size:12px;padding-right:8px;">m</span></div>
              </div>
            </div>

            <!-- Geometry -->
            <div class="prop-section" style="margin-top: 16px;">
              <div class="prop-title" style="text-transform:none; font-weight:bold; margin-bottom: 4px; color: #f8fafc;">Geometry</div>
              <div class="prop-vector">
                <div class="prop-row"><div class="prop-row-label" style="width:70px; color:#cbd5e1;">Bevel</div><input type="number" id="tf-geom-bevel" value="0.040" step="0.010" min="0"><span style="color:#64748b;font-size:12px;padding-right:8px;">m</span></div>
                <div class="prop-row"><div class="prop-row-label" style="width:70px; color:#cbd5e1;">Segments</div><input type="number" id="tf-geom-segments" value="10" step="1" min="1"><span style="color:#64748b;font-size:12px;padding-right:8px;">&nbsp;</span></div>
                <div class="prop-row"><div class="prop-row-label" style="width:70px; color:#cbd5e1;">Roundness</div><input type="number" id="tf-geom-roundness" value="0" step="1" min="0"><span style="color:#64748b;font-size:12px;padding-right:8px;">%</span></div>
              </div>
            </div>
          </div>
        </aside>
    "#
}

pub fn material_panel() -> &'static str {
    r#"
        <aside class="matter-panel-right collapsed" id="material-panel">
          <div class="panel-resizer" id="material-resizer" title="Drag to resize"></div>
          <button class="panel-toggle-btn tab-material" id="material-toggle" title="Материал">Материал</button>
          <div class="panel-header">Материал</div>
          <div class="panel-body">
            <div class="prop-section">
              <div class="prop-title">Active Material</div>
              <div class="prop-value text-muted">None</div>
            </div>
          </div>
        </aside>
    "#
}

pub fn nodes_panel() -> &'static str {
    r#"
        <aside class="matter-panel-right collapsed" id="nodes-panel">
          <div class="panel-resizer" id="nodes-resizer" title="Drag to resize"></div>
          <button class="panel-toggle-btn tab-nodes" id="nodes-toggle" title="Ноды">Ноды</button>
          <div class="panel-header">Ноды</div>
          <div class="panel-body">
            <div class="prop-section">
              <div class="prop-title">Node Tree</div>
              <div class="prop-value text-muted">Empty</div>
            </div>
          </div>
        </aside>
    "#
}

pub fn history_panel() -> &'static str {
    r#"
        <aside class="matter-panel-right collapsed" id="history-panel">
          <div class="panel-resizer" id="history-resizer" title="Drag to resize"></div>
          <button class="panel-toggle-btn tab-history" id="history-toggle" title="История">История</button>
          <div class="panel-header">История</div>
          <div class="panel-body">
            <div class="prop-section">
              <div class="prop-title">Undo History</div>
              <div class="prop-value text-muted">Initial State</div>
            </div>
          </div>
        </aside>
    "#
}

pub fn ai_panel() -> &'static str {
    r#"
        <aside class="matter-panel-right collapsed" id="ai-panel">
          <div class="panel-resizer" id="ai-resizer" title="Drag to resize"></div>
          <button class="panel-toggle-btn tab-ai" id="ai-toggle" title="AI">AI</button>
          <div class="panel-header">AI Assistant</div>
          <div class="panel-body">
            <div class="prop-section">
              <div class="prop-title">Prompt</div>
              <div class="prop-actions">
                <button class="prop-btn highlight">Generate</button>
              </div>
            </div>
          </div>
        </aside>
    "#
}

pub fn sketch_panel() -> &'static str {
    r#"
        <aside class="matter-panel-right collapsed" id="sketch-panel">
          <div class="panel-resizer" id="sketch-resizer" title="Drag to resize"></div>
          <button class="panel-toggle-btn tab-sketch" id="sketch-toggle" title="Скетч">Скетч</button>
          <div class="panel-header">Sketch Inspector</div>
          <div class="panel-body">
            
            <!-- STATUS -->
            <div class="prop-section" style="margin-bottom:16px;">
              <div class="prop-title" style="color:#f8fafc; margin-bottom:8px;">STATUS</div>
              <div class="prop-title" style="margin-bottom: 4px; text-transform:none; display:flex; justify-content:space-between; align-items:center;">
                Plane: 
                <select id="sketch-plane-select" style="background:#1e1e24; color:#cbd5e1; border:1px solid rgba(255,255,255,0.1); padding:2px 4px; border-radius:4px; font-size:11px;">
                  <option value="XZ" selected>XZ (Top)</option>
                  <option value="XY">XY (Front)</option>
                  <option value="YZ">YZ (Right)</option>
                </select>
              </div>
              <div class="prop-title" style="margin-bottom: 4px; text-transform:none;">View: <span style="font-weight:normal; color:#cbd5e1;">Orthographic</span></div>
              <div class="prop-title" style="margin-bottom: 4px; text-transform:none;">Profile: <span id="sketch-ui-closed" style="font-weight:normal; color:#f87171;">Open</span></div>
              <div class="prop-title" style="margin-bottom: 4px; text-transform:none;">Points: <span id="sketch-ui-points" style="font-weight:normal; color:#cbd5e1;">0</span></div>
              <div class="prop-title" style="margin-bottom: 4px; text-transform:none;">Segments: <span id="sketch-ui-segments" style="font-weight:normal; color:#cbd5e1;">0</span></div>
            </div>

            <!-- TOOLS -->
            <div class="prop-section" style="margin-bottom:16px;">
              <div class="prop-title" style="color:#f8fafc; margin-bottom:8px;">TOOLS</div>
              <div class="prop-title" style="margin-bottom: 4px; text-transform:none;">Active Tool: <span id="sketch-ui-tool" style="font-weight:normal; color:#38bdf8;">Line</span></div>
              <div class="prop-title" style="margin-bottom: 4px; text-transform:none;">Snap: <span style="font-weight:normal; color:#10b981;">ON</span></div>
              <div class="prop-title" style="margin-bottom: 4px; text-transform:none; display:flex; justify-content:space-between; align-items:center;">
                Grid Snap: 
                <select id="sketch-grid-snap-select" style="background:#1e1e24; color:#cbd5e1; border:1px solid rgba(255,255,255,0.1); padding:2px 4px; border-radius:4px; font-size:11px;">
                  <option value="1.0">1 m</option>
                  <option value="0.5">50 cm</option>
                  <option value="0.1" selected>10 cm</option>
                  <option value="0.05">5 cm</option>
                  <option value="0.01">1 cm</option>
                  <option value="0.005">5 mm</option>
                  <option value="0.001">1 mm</option>
                  <option value="0">Off</option>
                </select>
              </div>
              <div class="prop-title" style="margin-bottom: 4px; text-transform:none; display:flex; justify-content:space-between; align-items:center;">
                Show Grid: 
                <input type="checkbox" id="sketch-grid-toggle" checked style="cursor:pointer;" />
              </div>
              <div class="prop-title" style="margin-bottom: 4px; text-transform:none;">Angle Snap: <span style="font-weight:normal; color:#cbd5e1;">15°</span></div>
            </div>

            <!-- DIMENSIONS (Calculated dynamically) -->
            <div class="prop-section" id="sketch-ui-dimensions-panel" style="margin-bottom:16px; opacity: 0.5;">
              <div class="prop-title" style="color:#f8fafc; margin-bottom:8px;">DIMENSIONS</div>
              <div class="prop-title" style="margin-bottom: 4px; text-transform:none;"><span id="sketch-ui-width-label">X Size</span>: <span id="sketch-ui-width" style="font-weight:normal; color:#cbd5e1;">0.000 m</span></div>
              <div class="prop-title" style="margin-bottom: 4px; text-transform:none;"><span id="sketch-ui-depth-label">Z Size</span>: <span id="sketch-ui-depth" style="font-weight:normal; color:#cbd5e1;">0.000 m</span></div>
              <div class="prop-title" style="margin-bottom: 4px; text-transform:none;">Area: <span id="sketch-ui-area" style="font-weight:normal; color:#cbd5e1;">0.000 m²</span></div>
              <div class="prop-title" style="margin-bottom: 4px; text-transform:none;">Perimeter: <span id="sketch-ui-perimeter" style="font-weight:normal; color:#cbd5e1;">0.000 m</span></div>
            </div>

            <!-- EXTRUDE (visible only when profile is closed) -->
            <div class="prop-section" id="sketch-ui-extrude-panel" style="margin-bottom:16px; display:none;">
              <div class="prop-title" style="color:#f8fafc; margin-bottom:8px;">EXTRUDE</div>
              <div class="prop-title" style="margin-bottom: 4px; text-transform:none;">Direction: <span id="sketch-ui-extrude-dir" style="font-weight:normal; color:#38bdf8;">+Y</span></div>
              <div class="prop-row" style="margin-bottom:8px;">
                <div class="prop-row-label" style="width:70px; color:#cbd5e1;">Distance</div>
                <input type="number" id="sketch-ui-extrude-distance" value="1.000" step="0.05" min="0.001" style="flex:1; background:#1e1e24; color:#cbd5e1; border:1px solid rgba(255,255,255,0.1); padding:2px 6px; border-radius:4px;">
                <span style="color:#64748b;font-size:12px;padding-left:6px;">m</span>
              </div>
              <div class="prop-actions" style="display:flex; flex-direction:column; gap:6px;">
                <button id="btn-sketch-extrude-preview" class="prop-btn" style="width:100%;">Preview Extrude</button>
                <button id="btn-sketch-extrude-create" class="prop-btn highlight" style="width:100%;">Create Solid</button>
                <button id="btn-sketch-extrude-cancel" class="prop-btn" style="width:100%; color:#f87171; border-color:rgba(248,113,113,0.2);">Cancel</button>
              </div>
            </div>

            <!-- CONSTRAINTS -->
            <div class="prop-section" id="sketch-ui-constraints-panel" style="margin-bottom:16px; opacity: 0.5;">
              <div class="prop-title" style="color:#f8fafc; margin-bottom:8px;">CONSTRAINTS</div>
              <div class="prop-title" style="margin-bottom: 4px; text-transform:none;">Horizontal: <span style="font-weight:normal; color:#cbd5e1;">0</span></div>
              <div class="prop-title" style="margin-bottom: 4px; text-transform:none;">Vertical: <span style="font-weight:normal; color:#cbd5e1;">0</span></div>
              <div class="prop-title" style="margin-bottom: 4px; text-transform:none;">Coincident: <span style="font-weight:normal; color:#cbd5e1;">0</span></div>
            </div>
                                                                                                  
            <!-- ACTIONS -->
            <div class="prop-section">
              <div class="prop-title" style="color:#f8fafc; margin-bottom:8px;">ACTIONS</div>
              <div class="prop-actions" style="display:flex; flex-direction:column; gap:8px;">
                <button id="btn-sketch-close-profile" class="prop-btn" style="width:100%;">Close Profile</button>                                                                                                                     
                <button id="btn-sketch-add-dimension" class="prop-btn" style="width:100%;">Add Dimension</button>                                                                                                                     
                <button id="btn-sketch-extrude" class="prop-btn highlight" style="width:100%; opacity: 0.5; pointer-events: none;">Extrude</button>                                                                                   
                <button id="btn-sketch-cancel" class="prop-btn" style="width:100%; color: #f87171; border-color: rgba(248, 113, 113, 0.2);">Cancel Sketch</button>                                                                         
              </div>
            </div>

          </div>
        </aside>
    "#
}
