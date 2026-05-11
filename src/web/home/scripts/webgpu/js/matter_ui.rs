// ── JS: Matter Lab UI binding — sync state ↔ DOM, action handlers ────────────
// Domain: Presentation — wires DOM events of the new Matter Lab panels into
// existing engine objects (cam, shape, formation, cellSdf, setParticleCount).

pub const JS: &str = r##"
      // ── 8b. Matter Lab UI ───────────────────────────────────────

      // ── DOM sync ──
      function syncMatterUi() {
        if (sceneState) {
          const isSel = sceneState.selected;
          const pos = sceneState.objectPosition;
          const scl = sceneState.objectScale;
          const dim = sceneState.baseMeshDim;

          const x = document.getElementById("tf-loc-x"); if (x !== document.activeElement) x.value = pos[0].toFixed(3);
          const y = document.getElementById("tf-loc-y"); if (y !== document.activeElement) y.value = pos[1].toFixed(3);
          const z = document.getElementById("tf-loc-z"); if (z !== document.activeElement) z.value = pos[2].toFixed(3);

          const sx = document.getElementById("tf-scale-x"); if (sx !== document.activeElement) sx.value = scl[0].toFixed(3);
          const sy = document.getElementById("tf-scale-y"); if (sy !== document.activeElement) sy.value = scl[1].toFixed(3);
          const sz = document.getElementById("tf-scale-z"); if (sz !== document.activeElement) sz.value = scl[2].toFixed(3);

          // dimensions bind directly
          const dx = document.getElementById("tf-dim-x"); if (dx !== document.activeElement) dx.value = dim[0].toFixed(3);
          const dy = document.getElementById("tf-dim-y"); if (dy !== document.activeElement) dy.value = dim[1].toFixed(3);
          const dz = document.getElementById("tf-dim-z"); if (dz !== document.activeElement) dz.value = dim[2].toFixed(3);

          const bev = sceneState.objectBevel;
          const seg = sceneState.objectProfile;
          const rnd = sceneState.objectRoundness;

          const gBev = document.getElementById("tf-geom-bevel"); if (gBev && gBev !== document.activeElement) gBev.value = bev.toFixed(3);
          const gSeg = document.getElementById("tf-geom-segments"); if (gSeg && gSeg !== document.activeElement) gSeg.value = seg.toFixed(0);
          const gRnd = document.getElementById("tf-geom-roundness"); if (gRnd && gRnd !== document.activeElement) gRnd.value = rnd.toFixed(0);

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

      window.rebuildSolidMesh = async function() {
        const payload = {
          dimensions: sceneState.baseMeshDim, // В миллиметрах
          bevel: sceneState.objectBevel,
          segments: parseInt(sceneState.objectProfile) || 10
        };

        try {
          const res = await fetch("/api/matter/mesh/generate", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(payload)
          });
          if (!res.ok) throw new Error("Server export returned " + res.status);
          const data = await res.json();
          
          if (data.indices && data.indices.length > 0) {
            const posArr = new Float32Array(data.positions);
            const normArr = new Float32Array(data.normals);
            const faceIdArr = new Uint32Array(data.face_ids);
            const idxArr = new Uint32Array(data.indices);
            
            device.queue.writeBuffer(cadPosBuf, 0, posArr);
            device.queue.writeBuffer(cadNormalBuf, 0, normArr);
            device.queue.writeBuffer(cadFaceIdBuf, 0, faceIdArr);
            device.queue.writeBuffer(cadIndexBuf, 0, idxArr);
            
            cadIndexCount = idxArr.length;
          }
        } catch(e) {
          console.error("Failed to rebuild solid mesh", e);
        }
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

        let rebuildTimeout = null;
        const bindInput = (id, cb) => {
          const el = document.getElementById(id);
          if (!el) return;
          makeDraggable(el);
          el.addEventListener("input", (e) => {
            cb(parseFloat(e.target.value) || 0.0);
            if (sceneState.engineMode === 'CAD') {
              if (rebuildTimeout) clearTimeout(rebuildTimeout);
              rebuildTimeout = setTimeout(() => {
                if (window.rebuildSolidMesh) window.rebuildSolidMesh();
              }, 150);
            }
          });
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

        // dimensions change the actual base form mathematically
        bindInput("tf-dim-x", v => sceneState.baseMeshDim[0] = Math.max(0.001, v));
        bindInput("tf-dim-y", v => sceneState.baseMeshDim[1] = Math.max(0.001, v));
        bindInput("tf-dim-z", v => sceneState.baseMeshDim[2] = Math.max(0.001, v));

        // Geometry changes
        bindInput("tf-geom-bevel", v => sceneState.objectBevel = Math.max(0, v));
        bindInput("tf-geom-segments", v => sceneState.objectProfile = Math.max(1, v));
        bindInput("tf-geom-roundness", v => sceneState.objectRoundness = Math.max(0, v));

        // Export to OBJ
        const btnExportObj = document.getElementById("btn-export-obj");
        if (btnExportObj) {
          btnExportObj.addEventListener("click", async () => {
            try {
              const oldText = btnExportObj.innerHTML;
              btnExportObj.innerHTML = `<span style="font-size:14px;">⏳</span> Генерация ядра...`;
              btnExportObj.style.opacity = '0.7';

              const payload = {
                dimensions: sceneState.baseMeshDim, // В миллиметрах
                bevel: sceneState.objectBevel,
                segments: parseInt(sceneState.objectProfile) || 10
              };

              const res = await fetch("/api/matter/mesh/generate", {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify(payload)
              });

              if (!res.ok) throw new Error("Server export returned " + res.status);
              
              const data = await res.json();
              
              // Helper to download text as file
              const downloadFile = (content, filename) => {
                const blob = new Blob([content], { type: "text/plain" });
                const link = document.createElement("a");
                link.href = URL.createObjectURL(blob);
                link.download = filename;
                document.body.appendChild(link);
                link.click();
                document.body.removeChild(link);
              };

              downloadFile(data.obj_data, "matter_model.obj");
              downloadFile(data.mtl_data, "matter_model.mtl");

              console.log(`%c[WebGPU] 📦 CAD: Настоящий 3D Mesh (${data.vertex_count} вершин, ${data.triangle_count} полигонов) успешно прибыл из Rust бэкенда!`, 'color:#34d399;font-weight:bold');
              if (typeof log === 'function') log('◇ CAD модель прибыла из бэкенда', '#34d399');

              btnExportObj.innerHTML = `<span style="font-size:14px;">✅</span> Успешно!`;
              setTimeout(() => { btnExportObj.innerHTML = oldText; btnExportObj.style.opacity = '1.0'; }, 3000);
            } catch (err) {
              console.error(err);
              btnExportObj.innerHTML = `<span style="font-size:14px;">❌</span> Ошибка экспорта`;
            }
          });
        }

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
          { id: 'AI', panelId: 'ai-panel', toggleId: 'ai-toggle', resizerId: 'ai-resizer' },
          { id: 'SKETCH', panelId: 'sketch-panel', toggleId: 'sketch-toggle', resizerId: 'sketch-resizer' }
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

        // ─────────────────────────────────────────────────────────
        // ── UNIFIED EDITOR MODE CONTROLLER (single source of truth)
        // ─────────────────────────────────────────────────────────
        // editorMode ∈ "object" | "sketch" | "face" | "edge" | "vertex"
        // selectionMode (legacy numeric): 0=Object,1=Face,2=Edge,3=Vertex,4=Sketch
        const MODE_NUM = { object: 0, face: 1, edge: 2, vertex: 3, sketch: 4 };
        const MODE_NAME = ['object','face','edge','vertex','sketch'];
        const MODE_LABEL = { object:'Object', face:'Face', edge:'Edge', vertex:'Vertex', sketch:'Sketch' };

        window.editorState = window.editorState || {
          mode: 'object',
          activeSketchPlane: 'XZ',
          activeSketchTool: 'line',
        };

        function updateModeButtons(mode) {
          const sw = document.getElementById('selection-mode-switcher');
          if (!sw) return;
          sw.querySelectorAll('.sel-btn').forEach(b => {
            const m = MODE_NAME[parseInt(b.dataset.sel)] || 'object';
            b.classList.toggle('active', m === mode);
          });
        }

        function updatePanelForMode(mode) {
          // Right inspector follows mode strictly
          if (mode === 'sketch') openPanel('SKETCH');
          else if (mode === 'object') openPanel('N');
          else openPanel('N'); // Face/Edge/Vertex share Properties panel
        }

        function updateSketchOverlays(mode) {
          const tools = document.getElementById('sketch-tools-switcher');
          const info  = document.getElementById('sketch-info-overlay');
          const sketchCanvas = document.getElementById('sketch-canvas');
          const visible = (mode === 'sketch');
          if (tools) tools.style.display = visible ? 'flex' : 'none';
          if (info)  info.style.display  = visible ? 'block' : 'none';
          if (sketchCanvas) sketchCanvas.style.display = visible ? 'block' : 'none';
        }

        function updateCameraForMode(mode, plane) {
          if (mode === 'sketch') {
            if (window.setCameraProjection) window.setCameraProjection('ortho');
            const preset = (plane === 'XY') ? 'front'
                         : (plane === 'YZ') ? 'right'
                         : 'top';
            if (window.setCameraPreset) window.setCameraPreset(preset);
          } else {
            if (window.setCameraProjection) window.setCameraProjection('persp');
            if (window.setCameraPreset) window.setCameraPreset('iso');
          }
        }

        function updateStatusBar() {
          const s = window.editorState;
          const planeLbl = { XY:'XY Front', XZ:'XZ Top', YZ:'YZ Right' }[s.activeSketchPlane] || s.activeSketchPlane;
          const snap = (window.sketchGridSnap || 0.1);
          const snapLbl = snap >= 1 ? snap+' m' : (snap*100).toFixed(snap<0.01?1:0)+' cm';
          const elP = document.getElementById('sketch-info-plane');
          const elG = document.getElementById('sketch-info-grid');
          const elT = document.getElementById('sketch-info-tool');
          const elM = document.getElementById('sketch-info-mode');
          if (elP) elP.textContent = planeLbl;
          if (elG) elG.textContent = snapLbl;
          if (elT) elT.textContent = s.activeSketchTool;
          if (elM) elM.textContent = MODE_LABEL[s.mode];
        }

        window.__setEditorMode = function(mode, opts) {
          opts = opts || {};
          if (!(mode in MODE_NUM)) mode = 'object';
          const prev = window.editorState.mode;
          if (prev === mode && !opts.force) return;

          // Leaving sketch: optionally extrude closed sketch
          if (prev === 'sketch' && mode !== 'sketch') {
            if (window.doFakeExtrude && sketchState && sketchState.closed) {
              try { window.doFakeExtrude(); } catch (e) {}
            }
          }

          window.editorState.mode = mode;
          sceneState.selectionMode = MODE_NUM[mode];

          // Defaults when entering sketch
          if (mode === 'sketch') {
            window.editorState.activeSketchTool = window.editorState.activeSketchTool || 'line';
            window.editorState.activeSketchPlane = window.editorState.activeSketchPlane || 'XZ';
            sketchState.plane = window.editorState.activeSketchPlane;
            const ps = document.getElementById('sketch-plane-select');
            if (ps) ps.value = sketchState.plane;
          }

          updateModeButtons(mode);
          updatePanelForMode(mode);
          updateSketchOverlays(mode);
          updateCameraForMode(mode, window.editorState.activeSketchPlane);
          updateStatusBar();
          if (window.__updateSketchUI) window.__updateSketchUI();
        };

        window.__setSketchPlane = function(plane) {
          if (!['XY','XZ','YZ'].includes(plane)) return;
          // If user has unfinished sketch, ask
          if (sketchState && sketchState.points.length > 0 && !sketchState.closed) {
            const ok = window.confirm('Discard current unfinished sketch and switch plane?');
            if (!ok) return;
            sketchState.points = [];
            sketchState.closed = false;
          }
          window.editorState.activeSketchPlane = plane;
          sketchState.plane = plane;
          const ps = document.getElementById('sketch-plane-select');
          if (ps) ps.value = plane;
          if (window.editorState.mode === 'sketch') {
            updateCameraForMode('sketch', plane);
          }
          updateStatusBar();
          if (window.__updateSketchUI) window.__updateSketchUI();
        };

        window.__setSketchTool = function(tool) {
          if (!['line','rectangle','circle','dimension'].includes(tool)) return;
          // Cancel any in-progress two-click operation from the previous tool
          if (sketchState && sketchState.pendingStart) {
            sketchState.pendingStart = null;
            sketchState.pendingTool  = null;
          }
          window.editorState.activeSketchTool = tool;
          const sw = document.getElementById('sketch-tools-switcher');
          if (sw) {
            sw.querySelectorAll('.sketch-tool-btn').forEach(b => {
              b.classList.toggle('active', b.dataset.tool === tool);
            });
          }
          updateStatusBar();
        };

        // ─────────────────────────────────────────────────────────
        // ACTIVE SKETCH / PROFILE — single source of truth
        // ─────────────────────────────────────────────────────────
        // `window.activeSketch` is the derived view-model rebuilt from sketchState.
        // Every reader (Inspector, status bar, Extrude button) MUST consume this.
        function fmtLen(m) {
          if (m < 0.01)  return (m*1000).toFixed(1) + ' mm';
          if (m < 1.0)   return (m*100).toFixed(1) + ' cm';
          return m.toFixed(3) + ' m';
        }
        function fmtArea(a) {
          if (a < 0.0001) return (a*1e6).toFixed(1) + ' mm²';
          if (a < 1.0)    return (a*1e4).toFixed(1) + ' cm²';
          return a.toFixed(3) + ' m²';
        }
        function planeAxes(plane) {
          // returns { u, v, n, uLabel, vLabel } picking 2D and normal axis for plane.
          if (plane === 'XY') return { u:'x', v:'y', n:[0,0,1], uLabel:'X Size', vLabel:'Y Size', dirLabel:'+Z' };
          if (plane === 'YZ') return { u:'y', v:'z', n:[1,0,0], uLabel:'Y Size', vLabel:'Z Size', dirLabel:'+X' };
          /* XZ default */    return { u:'x', v:'z', n:[0,1,0], uLabel:'X Size', vLabel:'Z Size', dirLabel:'+Y' };
        }
        function rebuildActiveSketch() {
          const sk = sketchState || {};
          const pts = Array.isArray(sk.points) ? sk.points : [];
          const plane = sk.plane || 'XZ';
          const ax = planeAxes(plane);

          const N = pts.length;
          const closed = !!sk.closed && N >= 3;
          const segCount = closed ? N : Math.max(0, N - 1);

          // Bounding box on 2D axes of the plane
          let minU = Infinity, maxU = -Infinity, minV = Infinity, maxV = -Infinity;
          for (const p of pts) {
            const u = p[ax.u], v = p[ax.v];
            if (u < minU) minU = u; if (u > maxU) maxU = u;
            if (v < minV) minV = v; if (v > maxV) maxV = v;
          }
          const sizeU = N ? (maxU - minU) : 0;
          const sizeV = N ? (maxV - minV) : 0;

          // Perimeter
          let perim = 0;
          for (let i = 1; i < N; i++) {
            const a = pts[i-1], b = pts[i];
            perim += Math.hypot(b.x-a.x, b.y-a.y, b.z-a.z);
          }
          if (closed && N > 1) {
            const a = pts[N-1], b = pts[0];
            perim += Math.hypot(b.x-a.x, b.y-a.y, b.z-a.z);
          }

          // Signed area on 2D plane (shoelace), only meaningful when closed
          let area = 0;
          if (closed) {
            for (let i = 0; i < N; i++) {
              const a = pts[i], b = pts[(i+1) % N];
              area += a[ax.u] * b[ax.v] - b[ax.u] * a[ax.v];
            }
            area = Math.abs(area) * 0.5;
          }

          window.activeSketch = {
            id: 'sketch-0',
            plane,
            axes: ax,
            pointIds:   pts.map((_, i) => 'p' + i),
            segmentIds: Array.from({ length: segCount }, (_, i) => 's' + i),
            points: pts,
            closed,
            bounds: { sizeU, sizeV, minU, maxU, minV, maxV },
            area,
            perimeter: perim,
          };
          return window.activeSketch;
        }

        window.__updateSketchUI = function() {
          const a = rebuildActiveSketch();
          const ax = a.axes;

          // STATUS rows
          const elClosed = document.getElementById('sketch-ui-closed');
          if (elClosed) {
            elClosed.textContent = a.closed ? 'Closed' : 'Open';
            elClosed.style.color = a.closed ? '#10b981' : '#f87171';
          }
          const elPts = document.getElementById('sketch-ui-points');
          if (elPts) elPts.textContent = String(a.pointIds.length);
          const elSeg = document.getElementById('sketch-ui-segments');
          if (elSeg) elSeg.textContent = String(a.segmentIds.length);

          // TOOLS row
          const elTool = document.getElementById('sketch-ui-tool');
          if (elTool) {
            const t = (window.editorState && window.editorState.activeSketchTool) || 'line';
            elTool.textContent = t.charAt(0).toUpperCase() + t.slice(1);
          }

          // DIMENSIONS (plane-aware labels)
          const elWL = document.getElementById('sketch-ui-width-label');
          const elDL = document.getElementById('sketch-ui-depth-label');
          if (elWL) elWL.textContent = ax.uLabel;
          if (elDL) elDL.textContent = ax.vLabel;
          const elW = document.getElementById('sketch-ui-width');
          const elD = document.getElementById('sketch-ui-depth');
          const elA = document.getElementById('sketch-ui-area');
          const elP = document.getElementById('sketch-ui-perimeter');
          if (elW) elW.textContent = fmtLen(a.bounds.sizeU);
          if (elD) elD.textContent = fmtLen(a.bounds.sizeV);
          if (elA) elA.textContent = a.closed ? fmtArea(a.area) : '— (open)';
          if (elP) elP.textContent = fmtLen(a.perimeter);
          const dimsPanel = document.getElementById('sketch-ui-dimensions-panel');
          if (dimsPanel) dimsPanel.style.opacity = (a.points.length > 0) ? '1' : '0.5';

          // EXTRUDE section visibility + button states
          const extPanel = document.getElementById('sketch-ui-extrude-panel');
          const canExtrude = (window.editorState && window.editorState.mode === 'sketch')
                            && a.closed && a.pointIds.length >= 3;
          if (extPanel) extPanel.style.display = canExtrude ? 'block' : 'none';
          const elDir = document.getElementById('sketch-ui-extrude-dir');
          if (elDir) elDir.textContent = ax.dirLabel;

          // Legacy Extrude action button (kept for compat)
          const legacyExt = document.getElementById('btn-sketch-extrude');
          if (legacyExt) {
            if (canExtrude) {
              legacyExt.style.opacity = '1';
              legacyExt.style.pointerEvents = 'auto';
            } else {
              legacyExt.style.opacity = '0.5';
              legacyExt.style.pointerEvents = 'none';
            }
          }
        };

        // ─────────────────────────────────────────────────────────
        // EXTRUDE PREVIEW handlers (frontend-only, no backend yet)
        // ─────────────────────────────────────────────────────────
        function startExtrudePreview() {
          const a = window.activeSketch;
          if (!a || !a.closed || a.pointIds.length < 3) return;
          const distInput = document.getElementById('sketch-ui-extrude-distance');
          const dist = distInput ? Math.max(0.001, parseFloat(distInput.value) || 1.0) : 1.0;
          const n = a.axes.n;
          window.extrudePreview = {
            active:    true,
            profileId: a.id,
            plane:     a.plane,
            direction: [n[0], n[1], n[2]],
            distance:  dist,
            points:    a.points.map(p => ({ x:p.x, y:p.y, z:p.z })),
          };
          log(`▣ Extrude preview: ${a.axes.dirLabel} × ${dist.toFixed(3)} m`, '#a78bfa');
        }
        function cancelExtrudePreview() {
          window.extrudePreview.active = false;
          window.extrudePreview.points = [];
          log('▣ Extrude preview cancelled', '#f87171');
        }
        function commitExtrudePreview() {
          // For now: commit by clearing the sketch and disabling preview.
          // Backend (truck) wiring will replace this later.
          if (!window.extrudePreview.active) startExtrudePreview();
          if (!window.extrudePreview.active) return;
          log('▣ Create Solid: pending backend (truck) — preview kept', '#10b981');
          // Intentionally leave preview active so user can see result.
          // Real solid creation happens when Rust backend is wired.
        }
        const btnPrev   = document.getElementById('btn-sketch-extrude-preview');
        const btnCreate = document.getElementById('btn-sketch-extrude-create');
        const btnCancel = document.getElementById('btn-sketch-extrude-cancel');
        const distInp   = document.getElementById('sketch-ui-extrude-distance');
        if (btnPrev)   btnPrev.addEventListener('click', startExtrudePreview);
        if (btnCreate) btnCreate.addEventListener('click', commitExtrudePreview);
        if (btnCancel) btnCancel.addEventListener('click', cancelExtrudePreview);
        if (distInp)   distInp.addEventListener('input', () => {
          if (window.extrudePreview.active) {
            const d = Math.max(0.001, parseFloat(distInp.value) || 1.0);
            window.extrudePreview.distance = d;
          }
        });
        // "Cancel Sketch" button — full reset
        const btnCancelSketch = document.getElementById('btn-sketch-cancel');
        if (btnCancelSketch) {
          btnCancelSketch.addEventListener('click', () => {
            sketchState.points = [];
            sketchState.closed = false;
            sketchState.pendingStart = null;
            sketchState.pendingTool = null;
            cancelExtrudePreview();
            if (window.__updateSketchUI) window.__updateSketchUI();
            log('✕ Sketch cancelled', '#f87171');
          });
        }

        const toolsSw = document.getElementById('sketch-tools-switcher');
        if (toolsSw) {
          toolsSw.addEventListener('click', (e) => {
            const btn = e.target.closest('.sketch-tool-btn');
            if (!btn) return;
            window.__setSketchTool(btn.dataset.tool);
          });
        }

        // Track mouse coords in sketch plane for status overlay
        const canvasEl = document.getElementById('webgpu-canvas') || document.querySelector('canvas');
        if (canvasEl) {
          canvasEl.addEventListener('pointermove', (e) => {
            if (window.editorState.mode !== 'sketch') return;
            const el = document.getElementById('sketch-info-mouse');
            if (!el || !window.__sketchMouseWorld) return;
            const w = window.__sketchMouseWorld;
            if (!w) return;
            const p = sketchState.plane;
            let a='X', b='Z', av=w.x, bv=w.z;
            if (p === 'XY') { a='X'; b='Y'; av=w.x; bv=w.y; }
            else if (p === 'YZ') { a='Y'; b='Z'; av=w.y; bv=w.z; }
            el.textContent = `${a}: ${av.toFixed(2)}  ${b}: ${bv.toFixed(2)}`;
          });
        }

        // Sketch panel toggle button → switch into Sketch mode automatically
        const sketchPanelToggle = document.getElementById('sketch-toggle');
        if (sketchPanelToggle) {
          sketchPanelToggle.addEventListener('click', () => {
            // Use rAF so existing toggle handler runs first
            requestAnimationFrame(() => {
              const panel = document.getElementById('sketch-panel');
              const isOpen = panel && !panel.classList.contains('collapsed');
              if (isOpen && window.editorState.mode !== 'sketch') {
                window.__setEditorMode('sketch');
              } else if (!isOpen && window.editorState.mode === 'sketch') {
                window.__setEditorMode('object');
              }
            });
          });
        }

        // Other panel toggles → exit sketch mode if open
        ['profile-toggle','properties-toggle','shape-toggle','material-toggle',
         'nodes-toggle','history-toggle','ai-toggle'].forEach(id => {
          const t = document.getElementById(id);
          if (!t) return;
          t.addEventListener('click', () => {
            requestAnimationFrame(() => {
              if (window.editorState.mode === 'sketch') {
                window.__setEditorMode('object');
              }
            });
          });
        });

        // ── Selection Mode Switcher (Object ↔ Sketch ↔ Face ↔ Edge ↔ Vertex) ──
        const selSwitcher = document.getElementById('selection-mode-switcher');
        if (selSwitcher) {
          selSwitcher.addEventListener('click', (e) => {
            const btn = e.target.closest('.sel-btn');
            if (!btn) return;
            const num = parseInt(btn.dataset.sel);
            const modeName = MODE_NAME[num] || 'object';
            window.__setEditorMode(modeName);
          });
        }

        // Apply initial mode (object) so UI is consistent at first paint
        window.__setEditorMode('object', { force: true });

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
        
        // Initial mesh load if CAD mode is default
        setTimeout(() => {
          // Temporarily disabled initial dummy cube loading for Sketch Mode phase 1
          // if (sceneState.engineMode === 'CAD' && window.rebuildSolidMesh) window.rebuildSolidMesh();
          console.log("Skipping initial dummy cube generation for Sketch mode.");
        }, 300);
      }

      window.doFakeExtrude = function() {
        if (!sketchState || !sketchState.closed || sketchState.points.length < 3) return;
        
        let pts = [...sketchState.points];
        // Calculate signed area to enforce consistent CCW winding
        let area = 0;
        for (let i=0; i<pts.length; i++) {
           let nxt = (i+1)%pts.length;
           area += (pts[nxt].x - pts[i].x) * (pts[nxt].y + pts[i].y);
        }
        if (area > 0) pts.reverse(); // force CCW

        let minX = Infinity, maxX = -Infinity;
        let minZ = Infinity, maxZ = -Infinity;
        pts.forEach(p => { 
           minX = Math.min(minX, p.x); maxX = Math.max(maxX, p.x);
           minZ = Math.min(minZ, p.y); maxZ = Math.max(maxZ, p.y);
        });
        
        let cx = (minX + maxX) / 2.0;
        let cz = (minZ + maxZ) / 2.0;
        
        // Генерируем 2D плоскость без толщины (ведь Extrude мы будем делать потом отдельно)
        sceneState.baseMeshDim = [Math.max(0.1, maxX - minX), 0.001, Math.max(0.1, maxZ - minZ)];
        sceneState.objectPosition = [cx, 0.0, cz];
        sceneState.objectRotation = [0, 0, 0];
        sceneState.objectScale = [1, 1, 1];
        
        const pos = [];
        const norm = [];
        const fids = [];
        const uidx = [];
        
        const N = pts.length;
        
        // --- TOP FACE (Плоскость смотрящая вверх) ---
        const vTopCenter = pos.length / 3;
        pos.push(0, 0, 0); // center relative
        norm.push(0, 1, 0); fids.push(0);
        
        const topStart = pos.length / 3;
        for (let i=0; i<N; i++) {
           pos.push((pts[i].x - cx) * 1000.0, 0, (pts[i].y - cz) * 1000.0);
           norm.push(0, 1, 0); fids.push(0);
        }
        for (let i=0; i<N; i++) {
           let nxt = (i + 1) % N;
           uidx.push(vTopCenter, topStart + i, topStart + nxt);
        }
        
        // --- BOTTOM FACE (Плоскость смотрящая вниз, чтобы фигура была видна с двух сторон) ---
        const vBotCenter = pos.length / 3;
        pos.push(0, 0, 0);
        norm.push(0, -1, 0); fids.push(1);
        
        const botStart = pos.length / 3;
        for (let i=0; i<N; i++) {
           pos.push((pts[i].x - cx) * 1000.0, 0, (pts[i].y - cz) * 1000.0);
           norm.push(0, -1, 0); fids.push(1);
        }
        for (let i=0; i<N; i++) {
           let nxt = (i + 1) % N;
           // reversed winding for bottom
           uidx.push(vBotCenter, botStart + nxt, botStart + i);
        }
        
        const posArr = new Float32Array(pos);
        const normArr = new Float32Array(norm);
        const faceIdArr = new Uint32Array(fids);
        const idxArr = new Uint32Array(uidx);
        
        device.queue.writeBuffer(cadPosBuf, 0, posArr);
        device.queue.writeBuffer(cadNormalBuf, 0, normArr);
        device.queue.writeBuffer(cadFaceIdBuf, 0, faceIdArr);
        device.queue.writeBuffer(cadIndexBuf, 0, idxArr);
        cadIndexCount = idxArr.length;
        
        sketchState.points = [];
        sketchState.closed = false;
        openPanel('SKETCH');
        
        const sketchTools = document.getElementById('sketch-tools-switcher');
        if (sketchTools) sketchTools.style.display = 'flex';
        
        const sketchInfo = document.getElementById('sketch-info-overlay');
        if (sketchInfo) sketchInfo.style.display = 'block';
        
        if (window.__updateSketchUI) window.__updateSketchUI();
      };

      // expose for render_loop perf hook
      globalThis.__matterPerf = __matterPerf;

      // delay binding until DOM is ready (script is at bottom but be safe)
      if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', bindMatterUi);
      } else {
        bindMatterUi();
      }

"##;
