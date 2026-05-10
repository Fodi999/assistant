cat << 'EOF' > prop_panel.tmp
pub fn properties_panel() -> &'static str {
    r#"
        <aside class="matter-panel-right collapsed" id="properties-panel">
          <div class="panel-resizer" id="properties-resizer" title="Drag to resize"></div>
          <button class="panel-toggle-btn tab-n" id="properties-toggle" title="Item (N)">Item</button>
          <div class="panel-header">Item (N)</div>
          <div class="panel-body">
            
            <div class="prop-section">
              <div class="prop-title">Transform</div>
              
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
                <div class="prop-row"><div class="prop-row-label x">X</div><input type="number" id="tf-rot-x" value="0°" step="1" title="Currently UI only"></div>
                <div class="prop-row"><div class="prop-row-label y">Y</div><input type="number" id="tf-rot-y" value="0°" step="1" title="Currently UI only"></div>
                <div class="prop-row"><div class="prop-row-label z">Z</div><input type="number" id="tf-rot-z" value="0°" step="1" title="Currently UI only"></div>
              </div>

              <!-- Scale -->
              <div class="prop-title" style="text-transform:none; font-weight:normal; margin-bottom: 4px; color: #cbd5e1;">Scale:</div>
              <div class="prop-vector">
                <div class="prop-row"><div class="prop-row-label x">X</div><input type="number" id="tf-scale-x" value="1.000" step="0.1"></div>
                <div class="prop-row"><div class="prop-row-label y">Y</div><input type="number" id="tf-scale-y" value="1.000" step="0.1"></div>
                <div class="prop-row"><div class="prop-row-label z">Z</div><input type="number" id="tf-scale-z" value="1.000" step="0.1"></div>
              </div>
              
              <!-- Dimensions -->
              <div class="prop-title" style="text-transform:none; font-weight:normal; margin-bottom: 4px; color: #cbd5e1;">Dimensions:</div>
              <div class="prop-vector">
                <div class="prop-row"><div class="prop-row-label x">X</div><input type="number" id="tf-dim-x" value="2.000" step="0.1"></div>
                <div class="prop-row"><div class="prop-row-label y">Y</div><input type="number" id="tf-dim-y" value="2.000" step="0.1"></div>
                <div class="prop-row"><div class="prop-row-label z">Z</div><input type="number" id="tf-dim-z" value="2.000" step="0.1"></div>
              </div>
            </div>

          </div>
        </aside>
    "#
}
EOF
sed -i '' -e '/pub fn properties_panel() -> &'\'static' str {/,/    "#/!b' -e '/    "#/!d' -e '/    "#/r prop_panel.tmp' -e '/    "#/d' src/web/home/matter_panels.rs
