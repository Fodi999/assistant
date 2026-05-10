pub fn profile_panel() -> &'static str {
    r#"
        <aside class="matter-panel-right collapsed" id="profile-panel">
          <div class="panel-resizer" id="profile-resizer" title="Drag to resize"></div>
          <button class="panel-toggle-btn tab-m" id="profile-toggle" title="Профиль (M)">Профиль</button>
          <div class="panel-header">Профиль (M)</div>
          <div class="panel-body">
            <div class="prop-section">
              <div class="prop-title">Chef Status</div>
              <div class="prop-value">Online</div>
            </div>
            <div class="prop-section">
              <div class="prop-title">Role</div>
              <div class="prop-value text-muted">Admin</div>
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
                <div class="prop-row"><div class="prop-row-label x" style="width:50px;">Width</div><input type="number" id="tf-dim-x" value="2.0" step="0.1"><span style="color:#64748b;font-size:12px;padding-right:8px;">m</span></div>
                <div class="prop-row"><div class="prop-row-label y" style="width:50px;">Height</div><input type="number" id="tf-dim-y" value="2.0" step="0.1"><span style="color:#64748b;font-size:12px;padding-right:8px;">m</span></div>
                <div class="prop-row"><div class="prop-row-label z" style="width:50px;">Depth</div><input type="number" id="tf-dim-z" value="2.0" step="0.1"><span style="color:#64748b;font-size:12px;padding-right:8px;">m</span></div>
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
