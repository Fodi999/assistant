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

        // === Right Panels Logic ===
        const panelsConfig = [
          { id: 'M', panelId: 'profile-panel', toggleId: 'profile-toggle', resizerId: 'profile-resizer' },
          { id: 'N', panelId: 'properties-panel', toggleId: 'properties-toggle', resizerId: 'properties-resizer' },
          { id: 'SHAPE', panelId: 'shape-panel', toggleId: 'shape-toggle', resizerId: 'shape-resizer' },
          { id: 'MATERIAL', panelId: 'material-panel', toggleId: 'material-toggle', resizerId: 'material-resizer' },
          { id: 'NODES', panelId: 'nodes-panel', toggleId: 'nodes-toggle', resizerId: 'nodes-resizer' },
          { id: 'HISTORY', panelId: 'history-panel', toggleId: 'history-toggle', resizerId: 'history-resizer' },
          { id: 'AI', panelId: 'ai-panel', toggleId: 'ai-toggle', resizerId: 'ai-resizer' }
        ];

        const panelElements = panelsConfig.map(cfg => ({
          ...cfg,
          panel: document.getElementById(cfg.panelId),
          toggle: document.getElementById(cfg.toggleId),
          resizer: document.getElementById(cfg.resizerId)
        })).filter(cfg => cfg.panel && cfg.toggle);

        function updateBodyPanelState() {
          const isAnyOpen = panelElements.some(p => !p.panel.classList.contains('collapsed'));
          if (isAnyOpen) {
            document.body.classList.add('panel-open');
          } else {
            document.body.classList.remove('panel-open');
          }
        }

        function openPanel(idToOpen) {
          panelElements.forEach(p => {
            if (p.id === idToOpen || idToOpen === p.panelId) {
              p.panel.classList.remove('collapsed');
              p.toggle.classList.add('active');
            } else {
              p.panel.classList.add('collapsed');
              p.toggle.classList.remove('active');
            }
          });
          updateBodyPanelState();
        }

        function closeAllPanels() {
          panelElements.forEach(p => {
            p.panel.classList.add('collapsed');
            p.toggle.classList.remove('active');
          });
          updateBodyPanelState();
        }

        panelElements.forEach(p => {
          p.toggle.addEventListener('click', () => {
             if (p.panel.classList.contains('collapsed')) {
               openPanel(p.id);
             } else {
               closeAllPanels();
             }
          });
        });
          
        // Allow toggling with hotkeys N and M
        window.addEventListener('keydown', (e) => {
          if (!e.ctrlKey && !e.metaKey && e.target.tagName !== 'INPUT') {
            const key = e.key.toLowerCase();
            
            if (key === 'n') {
              const p = panelElements.find(p => p.id === 'N');
              if (p) {
                if (p.panel.classList.contains('collapsed')) openPanel('N'); else closeAllPanels();
              }
            } 
            else if (key === 'm') {
              const p = panelElements.find(p => p.id === 'M');
              if (p) {
                if (p.panel.classList.contains('collapsed')) openPanel('M'); else closeAllPanels();
              }
            }
          }
        });

        // Drag to resize panel logic
        let isResizing = false;
        
        const attachResizer = (resizerEl) => {
          if (!resizerEl) return;
          resizerEl.addEventListener('mousedown', (e) => {
            isResizing = true;
            document.body.style.cursor = 'ew-resize';
            e.preventDefault();
          });
        };
        
        panelElements.forEach(p => {
          if (p.resizer) attachResizer(p.resizer);
        });
        
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

        // ── Engine Mode Switcher (PARTICLE ↔ CAD) ─────────────────
        const modeSwitcher = document.getElementById('engine-mode-switcher');
        if (modeSwitcher) {
          modeSwitcher.addEventListener('click', (e) => {
            const btn = e.target.closest('.mode-btn');
            if (!btn) return;
            const mode = btn.dataset.mode;
            if (!mode) return;
            sceneState.engineMode = mode;
            modeSwitcher.querySelectorAll('.mode-btn').forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
          });
        }
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
