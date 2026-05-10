// ── JS: Matter Lab UI binding — sync state ↔ DOM, action handlers ────────────
// Domain: Presentation — wires DOM events of the new Matter Lab panels into
// existing engine objects (cam, shape, formation, cellSdf, setParticleCount).

pub const JS: &str = r##"
      // ── 8b. Matter Lab UI ───────────────────────────────────────

      // ── DOM sync ──
      function syncMatterUi() {
        if (sceneState) {
          const isSel = sceneState.selected;
          const pos = isSel ? sceneState.objectPosition : [0, 0, 0];
          const scl = isSel ? sceneState.objectScale : [0, 0, 0];
          const dim = isSel ? sceneState.baseMeshDim : [0, 0, 0];

          const x = document.getElementById("tf-loc-x"); if (x !== document.activeElement) x.value = pos[0].toFixed(3);
          const y = document.getElementById("tf-loc-y"); if (y !== document.activeElement) y.value = pos[1].toFixed(3);
          const z = document.getElementById("tf-loc-z"); if (z !== document.activeElement) z.value = pos[2].toFixed(3);

          const sx = document.getElementById("tf-scale-x"); if (sx !== document.activeElement) sx.value = scl[0].toFixed(3);
          const sy = document.getElementById("tf-scale-y"); if (sy !== document.activeElement) sy.value = scl[1].toFixed(3);
          const sz = document.getElementById("tf-scale-z"); if (sz !== document.activeElement) sz.value = scl[2].toFixed(3);

          // dimensions = scale * baseMeshDim
          const dx = document.getElementById("tf-dim-x"); if (dx !== document.activeElement) dx.value = (scl[0] * dim[0]).toFixed(3);
          const dy = document.getElementById("tf-dim-y"); if (dy !== document.activeElement) dy.value = (scl[1] * dim[1]).toFixed(3);
          const dz = document.getElementById("tf-dim-z"); if (dz !== document.activeElement) dz.value = (scl[2] * dim[2]).toFixed(3);

        }


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

        const makeDraggable = (input) => {

          // Style wrapper to allow custom arrows
          let parentEl = input.parentElement;
          const btnDec = document.createElement("button");
          btnDec.innerText = "◀";
          btnDec.style.cssText = "background:transparent; border:none; color:#475569; font-size:8px; padding:0 4px; cursor:pointer;";
          const btnInc = document.createElement("button");
          btnInc.innerText = "▶";
          btnInc.style.cssText = "background:transparent; border:none; color:#475569; font-size:8px; padding:0 8px 0 4px; cursor:pointer;";
          
          btnDec.addEventListener("click", () => {
            let v = parseFloat(input.value) || 0;
            let step = parseFloat(input.getAttribute("step")) || 0.1;
            input.value = (v - step).toFixed(3);
            input.dispatchEvent(new Event("input"));
          });
          btnInc.addEventListener("click", () => {
            let v = parseFloat(input.value) || 0;
            let step = parseFloat(input.getAttribute("step")) || 0.1;
            input.value = (v + step).toFixed(3);
            input.dispatchEvent(new Event("input"));
          });
          
          // Insert arrows after input
          parentEl.appendChild(btnDec);
          parentEl.appendChild(btnInc);

          input.style.cursor = "ew-resize";
          let isDragging = false;
          let startX = 0;
          let startVal = 0;
          let hasDragged = false;
          let baseStep = parseFloat(input.getAttribute("step")) || 0.1;
          
          input.addEventListener("mousedown", (e) => {
            startX = e.clientX;
            startVal = parseFloat(input.value) || 0;
            isDragging = true;
            hasDragged = false;
            
            const onMove = (me) => {
              if (!isDragging) return;
              const dx = me.clientX - startX;
              if (Math.abs(dx) > 2) hasDragged = true;
              if (hasDragged) {
                let speed = baseStep * 0.25;
                if (me.shiftKey) speed *= 0.1;
                if (me.altKey) speed *= 10.0;
                
                const newVal = startVal + dx * speed;
                input.value = newVal.toFixed(3);
                input.dispatchEvent(new Event("input"));
                
                if (document.activeElement !== input) {
                  input.focus();
                }
              }
            };
            
            const onUp = (ue) => {
              isDragging = false;
              window.removeEventListener("mousemove", onMove);
              window.removeEventListener("mouseup", onUp);
              if (hasDragged) {
                ue.preventDefault();
                input.blur();
              }
            };
            
            window.addEventListener("mousemove", onMove);
            window.addEventListener("mouseup", onUp);
          });
        };

        const bindInput = (id, cb) => {
          const el = document.getElementById(id);
          if (!el) return;
          makeDraggable(el);
          el.addEventListener("input", (e) => cb(parseFloat(e.target.value) || 0.0));
        };
        bindInput("tf-loc-x", v => sceneState.objectPosition[0] = v);
        bindInput("tf-loc-y", v => sceneState.objectPosition[1] = v);
        bindInput("tf-loc-z", v => sceneState.objectPosition[2] = v);

        bindInput("tf-rot-x", v => sceneState.objectRotation[0] = v);
        bindInput("tf-rot-y", v => sceneState.objectRotation[1] = v);
        bindInput("tf-rot-z", v => sceneState.objectRotation[2] = v);

        bindInput("tf-scale-x", v => sceneState.objectScale[0] = Math.max(0.001, v));
        bindInput("tf-scale-y", v => sceneState.objectScale[1] = Math.max(0.001, v));
        bindInput("tf-scale-z", v => sceneState.objectScale[2] = Math.max(0.001, v));

        // dimensions editing changes scale under the hood (Blender-like)
        bindInput("tf-dim-x", v => sceneState.objectScale[0] = Math.max(0.001, v / sceneState.baseMeshDim[0]));
        bindInput("tf-dim-y", v => sceneState.objectScale[1] = Math.max(0.001, v / sceneState.baseMeshDim[1]));
        bindInput("tf-dim-z", v => sceneState.objectScale[2] = Math.max(0.001, v / sceneState.baseMeshDim[2]));

        const btnApplyScale = document.getElementById("btn-apply-scale");
        if (btnApplyScale) btnApplyScale.addEventListener("click", () => {
          sceneState.baseMeshDim[0] *= sceneState.objectScale[0];
          sceneState.baseMeshDim[1] *= sceneState.objectScale[1];
          sceneState.baseMeshDim[2] *= sceneState.objectScale[2];
          sceneState.objectScale[0] = 1.0;
          sceneState.objectScale[1] = 1.0;
          sceneState.objectScale[2] = 1.0;
        });


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
            const isRender = sceneState.engineMode === "PARTICLES";
            const mode = isRender ? "CAD" : "PARTICLES";
            btn.querySelector(".mode-label").textContent = isRender ? "RENDER" : "SOLID";
            btn.querySelector(".mode-icon").textContent = isRender ? "⬡" : "◈";
            btn.dataset.mode = mode;
            if (isRender) {
                btn.classList.remove("active");
            } else {
            }
            
            sceneState.engineMode = mode;
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
