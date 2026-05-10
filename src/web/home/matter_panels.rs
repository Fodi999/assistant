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
              <div class="prop-title">Selection</div>
              <div class="prop-value text-muted">No object selected</div>
            </div>
            
            <div class="prop-section">
              <div class="prop-title">Active Tool</div>
              <div class="prop-value">Select</div>
            </div>
            
            <div class="prop-section">
              <div class="prop-title">Mode</div>
              <div class="prop-value">Object Mode</div>
            </div>
            
            <div class="prop-section">
              <div class="prop-title">Actions</div>
              <div class="prop-actions">
                <button class="prop-btn">Add Cube</button>
                <button class="prop-btn">Add Form</button>
                <button class="prop-btn highlight">Open AI Command</button>
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
            <div class="prop-section">
              <div class="prop-title">Mesh Data</div>
              <div class="prop-value text-muted">No mesh</div>
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
