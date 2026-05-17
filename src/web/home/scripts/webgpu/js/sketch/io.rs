// ── JS: Sketch I/O — Export / Import JSON + backend payload preview ─────────
// Domain: Sketch — clipboard copy, file download, file load, panel refresh.

pub const JS: &str = r##"
      // ── Pretty printer (stable key order) ──────────────────────
      function __sketchPrettyJSON(obj) {
        try { return JSON.stringify(obj, null, 2); }
        catch (_) { return '{}'; }
      }

      // ── Panel state ────────────────────────────────────────────
      // Modes: 'full' (sketchToJSON) | 'payload' (sketchExportPayload).
      window.__sketchIO = { mode: 'full' };

      window.__refreshSketchIOPanel = function() {
        const pre = document.getElementById('sio-preview');
        if (!pre) return;
        const obj = (window.__sketchIO.mode === 'payload')
          ? window.__sketchExportPayload()
          : window.__sketchToJSON();
        const txt = __sketchPrettyJSON(obj);
        pre.textContent = txt;
        const meta = document.getElementById('sio-meta');
        if (meta) {
          const bytes = new Blob([txt]).size;
          const pts = (obj.points || []).length;
          const eds = (obj.edges  || []).length;
          const cns = (obj.constraints || []).length;
          meta.textContent = pts + ' pts · ' + eds + ' edges · ' + cns + ' constraints · ' + bytes + ' B';
        }
        document.querySelectorAll('.sio-tab[data-mode]').forEach(btn => {
          btn.classList.toggle('active', btn.dataset.mode === window.__sketchIO.mode);
        });
      };

      window.__copySketchJSON = async function() {
        const obj = (window.__sketchIO.mode === 'payload')
          ? window.__sketchExportPayload()
          : window.__sketchToJSON();
        const txt = __sketchPrettyJSON(obj);
        try {
          if (navigator.clipboard && navigator.clipboard.writeText) {
            await navigator.clipboard.writeText(txt);
          } else {
            const ta = document.createElement('textarea');
            ta.value = txt;
            ta.style.position = 'fixed'; ta.style.opacity = '0';
            document.body.appendChild(ta); ta.select();
            document.execCommand('copy'); document.body.removeChild(ta);
          }
          window.__setStatusMessage('Скопировано ' + new Blob([txt]).size + ' Б JSON в буфер');
        } catch (err) {
          window.__setStatusMessage('Ошибка копирования: ' + (err && err.message ? err.message : err));
        }
      };

      window.__downloadSketchJSON = function() {
        const obj = (window.__sketchIO.mode === 'payload')
          ? window.__sketchExportPayload()
          : window.__sketchToJSON();
        const txt = __sketchPrettyJSON(obj);
        const blob = new Blob([txt], { type: 'application/json' });
        const url  = URL.createObjectURL(blob);
        const a    = document.createElement('a');
        const ts   = new Date().toISOString().replace(/[:.]/g, '-');
        const suffix = (window.__sketchIO.mode === 'payload') ? 'payload' : 'full';
        a.href = url;
        a.download = 'sketch-' + suffix + '-' + ts + '.json';
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        setTimeout(() => URL.revokeObjectURL(url), 1000);
        window.__setStatusMessage('Загружено ' + a.download);
      };

      window.__loadSketchJSON = function(text) {
        let obj;
        try { obj = JSON.parse(text); }
        catch (e) {
          window.__setStatusMessage('Ошибка импорта: некорректный JSON');
          return { ok:false, error:'invalid JSON: ' + e.message };
        }
        const res = window.__sketchFromJSON(obj);
        if (!res.ok) {
          window.__setStatusMessage('Ошибка импорта: ' + res.error);
        } else if (window.__updateSketchInspector) {
          window.__updateSketchInspector();
        }
        window.__refreshSketchIOPanel();
        return res;
      };

      window.__triggerSketchFileLoad = function() {
        const inp = document.getElementById('sio-file-input');
        if (inp) inp.click();
      };

      function __bindSketchIO() {
        const copyBtn = document.getElementById('sio-copy');
        if (copyBtn) copyBtn.addEventListener('click', window.__copySketchJSON);
        const dlBtn = document.getElementById('sio-download');
        if (dlBtn)   dlBtn.addEventListener('click', window.__downloadSketchJSON);
        const loadBtn = document.getElementById('sio-load');
        if (loadBtn) loadBtn.addEventListener('click', window.__triggerSketchFileLoad);
        const refBtn = document.getElementById('sio-refresh');
        if (refBtn)  refBtn.addEventListener('click', window.__refreshSketchIOPanel);

        document.querySelectorAll('.sio-tab[data-mode]').forEach(btn => {
          btn.addEventListener('click', () => {
            window.__sketchIO.mode = btn.dataset.mode;
            window.__refreshSketchIOPanel();
          });
        });

        const fileInput = document.getElementById('sio-file-input');
        if (fileInput) {
          fileInput.addEventListener('change', (ev) => {
            const file = ev.target.files && ev.target.files[0];
            if (!file) return;
            const reader = new FileReader();
            reader.onload = () => {
              const txt = String(reader.result || '');
              window.__loadSketchJSON(txt);
              fileInput.value = '';
            };
            reader.onerror = () => {
              window.__setStatusMessage('Ошибка чтения файла');
              fileInput.value = '';
            };
            reader.readAsText(file);
          });
        }

        const toggle = document.getElementById('sio-toggle');
        const panel  = document.getElementById('sketch-io-panel');
        if (toggle && panel) {
          toggle.addEventListener('click', () => {
            panel.classList.toggle('collapsed');
            toggle.textContent = panel.classList.contains('collapsed') ? '▸' : '▾';
            if (!panel.classList.contains('collapsed')) window.__refreshSketchIOPanel();
          });
        }

        window.__refreshSketchIOPanel();
      }
      window.__bindSketchIO = __bindSketchIO;

      // Auto-refresh panel periodically (only when N-panel + IO section are both visible).
      setInterval(() => {
        const inspector = document.getElementById('sketch-inspector');
        const panel     = document.getElementById('sketch-io-panel');
        if (inspector && inspector.classList.contains('open') &&
            panel && !panel.classList.contains('collapsed')) {
          window.__refreshSketchIOPanel();
        }
      }, 750);
"##;
