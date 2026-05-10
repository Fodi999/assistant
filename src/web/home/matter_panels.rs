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
