// ── JS: Matter Lab UI binding — sync state ↔ DOM, action handlers ────────────
// Domain: Presentation — wires DOM events of the new Matter Lab panels into
// existing engine objects (cam, shape, formation, cellSdf, setParticleCount).

pub const JS: &str = r##"
      // ── 8b. Matter Lab UI ───────────────────────────────────────

      // ── DOM sync ──
      function syncMatterUi() {
        engineState.performance.fps     = __matterPerf.fps;
        engineState.performance.frameMs = __matterPerf.frameMs;

        const setText = (id, v) => { const el = document.getElementById(id); if (el) el.textContent = v; };

        setText('fpsValue',       engineState.performance.fps.toFixed(0));
        setText('frameValue',     engineState.performance.frameMs.toFixed(1) + 'ms');
        
      }

      function resetMatter() {
        // No-op for now
      }

      // ── bind once on startup ──
      function bindMatterUi() {
        // close button → return to landing
        const close = document.getElementById('close-chefos');
        if (close) close.addEventListener('click', () => {
          document.body.classList.remove('engine-open');
        });

        // === Right Panels Logic (N and M) ===
        const toggleNPanel = document.getElementById('properties-toggle');
        const toggleMPanel = document.getElementById('profile-toggle');
        
        const propertiesPanel = document.getElementById('properties-panel');
        const profilePanel = document.getElementById('profile-panel');

        function updateBodyPanelState() {
          const isAnyOpen = (propertiesPanel && !propertiesPanel.classList.contains('collapsed')) || 
                            (profilePanel && !profilePanel.classList.contains('collapsed'));
          if (isAnyOpen) {
            document.body.classList.add('panel-open');
          } else {
            document.body.classList.remove('panel-open');
          }
        }

        function openPanel(idToOpen) {
          // Open requested
          if (idToOpen === 'N' && propertiesPanel) {
            propertiesPanel.classList.remove('collapsed');
            toggleNPanel.classList.add('active');
            
            // Close other
            if (profilePanel) {
               profilePanel.classList.add('collapsed');
               if(toggleMPanel) toggleMPanel.classList.remove('active');
            }
          } 
          else if (idToOpen === 'M' && profilePanel) {
            profilePanel.classList.remove('collapsed');
            if(toggleMPanel) toggleMPanel.classList.add('active');
            
            // Close other
            if (propertiesPanel) {
               propertiesPanel.classList.add('collapsed');
               toggleNPanel.classList.remove('active');
            }
          }
          
          updateBodyPanelState();
        }

        function closeAllPanels() {
          if (propertiesPanel) propertiesPanel.classList.add('collapsed');
          if (profilePanel) profilePanel.classList.add('collapsed');
          if (toggleNPanel) toggleNPanel.classList.remove('active');
          if (toggleMPanel) toggleMPanel.classList.remove('active');
          
          updateBodyPanelState();
        }

        if (toggleNPanel && propertiesPanel) {
          toggleNPanel.addEventListener('click', () => {
            if (propertiesPanel.classList.contains('collapsed')) {
              openPanel('N');
            } else {
              closeAllPanels();
            }
          });
        }
        
        if (toggleMPanel && profilePanel) {
          toggleMPanel.addEventListener('click', () => {
            if (profilePanel.classList.contains('collapsed')) {
              openPanel('M');
            } else {
              closeAllPanels();
            }
          });
        }
          
        // Allow toggling with hotkeys N and M
        window.addEventListener('keydown', (e) => {
          if (!e.ctrlKey && !e.metaKey && e.target.tagName !== 'INPUT') {
            const key = e.key.toLowerCase();
            
            if (key === 'n') {
              if (propertiesPanel && propertiesPanel.classList.contains('collapsed')) {
                openPanel('N');
              } else {
                closeAllPanels();
              }
            } 
            else if (key === 'm') {
              if (profilePanel && profilePanel.classList.contains('collapsed')) {
                openPanel('M');
              } else {
                closeAllPanels();
              }
            }
          }
        });

        // Drag to resize panel logic
        const resizerN = document.getElementById('properties-resizer');
        const resizerM = document.getElementById('profile-resizer');
        let isResizing = false;
        
        const attachResizer = (resizerEl) => {
          if (!resizerEl) return;
          resizerEl.addEventListener('mousedown', (e) => {
            isResizing = true;
            document.body.style.cursor = 'ew-resize';
            e.preventDefault();
          });
        };
        
        attachResizer(resizerN);
        attachResizer(resizerM);
        
        window.addEventListener('mousemove', (e) => {
          if (!isResizing) return;
          
          let newWidth = window.innerWidth - e.clientX - 15;
          if (newWidth < 320) newWidth = 320;
          if (newWidth > 720) newWidth = 720;
              
              // We set a CSS variable on the body, 
              // so both the panel width and gizmo offset update in real-time
              document.body.style.setProperty('--panel-width', `${newWidth}px`);
              // Temporarily disable transition during drag for 60fps immediate response
              document.body.classList.add('is-resizing');
            });
            
            window.addEventListener('mouseup', () => {
              if (isResizing) {
                isResizing = false;
                document.body.style.cursor = '';
                document.body.classList.remove('is-resizing');
              }
            });

        // periodic refresh for fps / frame
        setInterval(syncMatterUi, 250);
        syncMatterUi();
      }

      // expose for render_loop perf hook
      globalThis.__matterPerf = __matterPerf;

      // delay binding until DOM is ready (script is at bottom but be safe)
      if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', bindMatterUi);
      } else {
        bindMatterUi();
      }

"##;
