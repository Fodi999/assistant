// ── Matter Lab History Panel ─────────────────────────────────────
// Single History tab, slides from the LEFT edge.
// One tab-toggle button sticks out to the RIGHT of the panel body.

pub fn outliner_panel() -> &'static str {
    r##"
        <!-- Left sliding panel: History only -->
        <aside class="outliner-panel collapsed" id="outliner-panel">

          <!-- Single tab button sticking out to the right -->
          <button class="ol-tab-btn tab-ol-history" id="ol-history-toggle" title="History (H)">History</button>

          <!-- Content area fills the whole panel -->
          <div class="outliner-inner">

              <!-- HISTORY -->
              <div class="outliner-body" id="outliner-tab-history">
                <div class="outliner-toolbar">
                  <button class="outliner-mini-btn" id="btn-history-clear">✕ Clear</button>
                  <span style="color:#64748b;font-size:11px;margin-left:auto;" id="history-count">0 entries</span>
                </div>
                <div class="outliner-history" id="outliner-history-list"></div>
              </div>

          </div><!-- /outliner-inner -->
        </aside>
    "##
}

pub fn outliner_styles() -> &'static str {
    r##"
    /* ─── Left sliding panel (mirrors .matter-panel-right) ─ */
    .outliner-panel {
      position: absolute;
      top: 15px;
      left: 15px;
      bottom: 50px;
      width: 304px;
      background: rgba(30, 30, 32, 0.85);
      backdrop-filter: blur(12px);
      border: 1px solid rgba(80, 80, 85, 0.4);
      border-radius: 8px;
      z-index: 15;
      display: flex;
      flex-direction: column;
      transform: translateX(0);
      transition: transform 0.3s cubic-bezier(0.16, 1, 0.3, 1);
      box-shadow: 4px 0 15px rgba(0, 0, 0, 0.25);
      pointer-events: auto;
    }
    .outliner-panel.collapsed {
      transform: translateX(calc(-100% - 15px));
      z-index: 16;
    }

    /* Tab button — sticks out to the RIGHT (mirror of right panel's left-sticking tabs) */
    .ol-tab-btn {
      position: absolute;
      right: -32px;
      top: 15px;
      width: 32px;
      height: 90px;
      background: rgba(24, 24, 26, 0.95);
      border: 1px solid rgba(80, 80, 85, 0.4);
      border-right: none;          /* mirror: right panel cuts border-right */
      border-radius: 6px 0 0 6px;  /* mirror: right panel has 6px 0 0 6px on left side */
      color: rgba(148, 163, 184, 0.85);
      cursor: pointer;
      font-size: 11px;
      font-weight: 500;
      letter-spacing: 0.5px;
      transition: all 0.15s ease;
      display: flex; align-items: center; justify-content: center;
      writing-mode: vertical-rl;
      padding: 0;
      box-sizing: border-box;
      outline: none;
      z-index: 10;
    }
    .ol-tab-btn:hover {
      background: rgba(50, 50, 55, 0.95);
      color: #fff;
    }
    .ol-tab-btn.active {
      background: rgba(30, 30, 32, 0.85); /* seamlessly matches panel body — same as right panel */
      color: #fff;
      border-right: 2px solid #38bdf8;    /* blue highlight line — mirror of right panel's border-left */
    }

    /* Inner layout */
    .outliner-inner {
      flex: 1;
      display: flex;
      flex-direction: column;
      overflow: hidden;
      border-radius: 8px;
    }
    .outliner-body {
      flex: 1;
      overflow-y: auto;
      padding: 8px;
    }
    .outliner-toolbar {
      display: flex; gap: 4px; align-items: center;
      padding: 4px 2px 8px;
      border-bottom: 1px solid rgba(255,255,255,0.05);
      margin-bottom: 6px;
    }
    .outliner-mini-btn {
      background: rgba(15,23,42,0.55);
      border: 1px solid rgba(148,163,184,0.18);
      color: #cbd5e1;
      padding: 4px 8px;
      border-radius: 4px;
      cursor: pointer;
      font-size: 11px;
      transition: all .12s ease;
    }
    .outliner-mini-btn:hover {
      border-color: rgba(56,189,248,0.5);
      color: #fff;
    }

    /* History */
    .outliner-history {
      display: flex; flex-direction: column-reverse;
      gap: 2px;
    }
    .history-entry {
      display: flex; align-items: center; gap: 6px;
      padding: 4px 6px;
      border-radius: 3px;
      color: #cbd5e1;
      font-size: 11px;
      border-left: 2px solid rgba(56,189,248,0.4);
      background: rgba(15,23,42,0.4);
    }
    .history-entry-icon  { color: #38bdf8; }
    .history-entry-label { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
    .history-entry-time  { color: #64748b; font-size: 10px; font-family: monospace; }
    "##
}

pub fn outliner_js() -> &'static str {
    r##"
    // ─── History Panel Wiring ────────────────────────────────────────
    (function initOutliner() {
      function boot() {
        if (window.__outlinerBooted) return;
        const root = document.getElementById('outliner-panel');
        if (!root) { return setTimeout(boot, 50); }
        window.__outlinerBooted = true;

        // ── History data ─────────────────────────────────────────
        window.cadHistory = window.cadHistory || { entries: [], max: 200, _seq: 0 };
        const hist = window.cadHistory;

        function escapeHtml(s) {
          return String(s).replace(/[&<>"']/g, c =>
            ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
        }

        window.cadHistoryAdd = function(action, label, payload) {
          const e = { id: 'h-' + (++hist._seq), action,
            label: label || action, timestamp: Date.now(), payload: payload || null };
          hist.entries.push(e);
          if (hist.entries.length > hist.max)
            hist.entries.splice(0, hist.entries.length - hist.max);
          renderHistory(); return e;
        };
        window.cadHistoryClear = function() { hist.entries.length = 0; renderHistory(); };

        // ── Panel open / close ────────────────────────────────────
        function openPanel()   { root.classList.remove('collapsed');
                                 const btn = document.getElementById('ol-history-toggle');
                                 if (btn) btn.classList.add('active'); }
        function closePanel()  { root.classList.add('collapsed');
                                 const btn = document.getElementById('ol-history-toggle');
                                 if (btn) btn.classList.remove('active'); }
        function togglePanel() { root.classList.contains('collapsed') ? openPanel() : closePanel(); }

        const tabBtn = document.getElementById('ol-history-toggle');
        if (tabBtn) tabBtn.addEventListener('click', togglePanel);

        // Keyboard shortcut H: toggle History panel
        document.addEventListener('keydown', e => {
          if (e.target && (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA')) return;
          if (e.key === 'h' || e.key === 'H') togglePanel();
        });

        // ── Render history list ───────────────────────────────────
        const histEl  = document.getElementById('outliner-history-list');
        const histCnt = document.getElementById('history-count');
        function renderHistory() {
          if (!histEl) return;
          let html = '';
          for (const e of hist.entries) {
            const t = new Date(e.timestamp);
            const ts = String(t.getHours()).padStart(2,'0') + ':'
                     + String(t.getMinutes()).padStart(2,'0') + ':'
                     + String(t.getSeconds()).padStart(2,'0');
            html += '<div class="history-entry">'
              + '<span class="history-entry-icon">&#9658;</span>'
              + '<span class="history-entry-label">' + escapeHtml(e.label) + '</span>'
              + '<span class="history-entry-time">' + ts + '</span>'
              + '</div>';
          }
          histEl.innerHTML = html;
          if (histCnt) histCnt.textContent = hist.entries.length + ' entries';
        }

        const btnClearHist = document.getElementById('btn-history-clear');
        if (btnClearHist) btnClearHist.addEventListener('click', () => window.cadHistoryClear());

        // ── Primitive buttons in bottom toolbar ──────────────────
        document.querySelectorAll('.prim-btn').forEach(btn => {
          btn.addEventListener('click', () => {
            const a = btn.dataset.asset; if (!a) return;
            window.cadHistoryAdd('asset.add', 'Add: ' + a);
            if (window.__setEditorMode && (a.includes('sketch') || a.includes('rect') || a.includes('circle') || a.includes('poly')))
              window.__setEditorMode('sketch');
          });
        });

        // ── Sketch lifecycle hook ─────────────────────────────────
        const origUpdateUI = window.__updateSketchUI;
        let lastPts = 0, lastClosed = false;
        window.__updateSketchUI = function() {
          if (origUpdateUI) origUpdateUI.apply(this, arguments);
          const s = window.sketchState; if (!s) return;
          const n = (s.points || []).length;
          if (n > lastPts) {
            const tool = (window.editorState && window.editorState.activeSketchTool) || 'line';
            if (tool === 'line')      window.cadHistoryAdd('sketch.addPoint',  n === 1 ? 'Add Line (start)' : 'Add Segment');
            if (tool === 'rectangle') window.cadHistoryAdd('sketch.addRect',   'Add Rectangle');
            if (tool === 'circle')    window.cadHistoryAdd('sketch.addCircle', 'Add Circle');
          }
          if (!lastClosed && s.closed) window.cadHistoryAdd('profile.close', 'Close Profile');
          if (lastClosed && !s.closed && n === 0) window.cadHistoryAdd('sketch.reset', 'Reset Sketch');
          lastPts = n; lastClosed = s.closed;
        };

        // ── Extrude button hooks ──────────────────────────────────
        const btnPrev   = document.getElementById('btn-sketch-extrude-preview');
        const btnCreate = document.getElementById('btn-sketch-extrude-create');
        const btnCancel = document.getElementById('btn-sketch-extrude-cancel');
        if (btnPrev)   btnPrev.addEventListener('click',   () => window.cadHistoryAdd('extrude.preview', 'Preview Extrude'));
        if (btnCreate) btnCreate.addEventListener('click', () => window.cadHistoryAdd('solid.create',    'Create Solid'));
        if (btnCancel) btnCancel.addEventListener('click', () => window.cadHistoryAdd('extrude.cancel',  'Cancel Extrude'));

        // ── Initial render ────────────────────────────────────────
        renderHistory();
        window.cadHistoryAdd('session.start', 'Session started');
      }

      if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', boot);
      } else {
        boot();
      }
    })();
    "##
}
