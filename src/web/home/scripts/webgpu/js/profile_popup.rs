// ── Profile Check popup — floating panel, opens on double-click inside profile
//
// window.__openProfilePopup(px, py)
//   px, py — CSS pixel position (e.g. from pointer event clientX/Y)
//
// The popup auto-runs Analyze on open, then lets the user click
// Make Rectangle / Make Square / Equalize Edges without touching the N-panel.
//
// Designed to match the dimension editor visual language.

pub const JS: &str = r##"
      (function registerProfilePopup() {

        // Используем единую тему — меняй цвета в components/modal_theme.rs
        const T = window.__modalTheme;
        const C = T.COLORS;

        function buildPopup() {
          const el = document.createElement('div');
          el.id = '__profile-popup';
          T.applyPopupStyle(el, { zIndex: '9998', minWidth: '240px', maxWidth: '290px' });

          el.innerHTML = `
            <div class="cad-popup-titlebar" data-k="titlebar">
              <span class="cad-popup-title">Profile Check</span>
              <button data-k="close" type="button" class="cad-popup-close">✕</button>
            </div>

            <dl class="cad-popup-grid">
              <dt>Id</dt>     <dd data-k="pid">—</dd>
              <dt>Type</dt>   <dd data-k="type">—</dd>
              <dt>Width</dt>  <dd data-k="width">—</dd>
              <dt>Height</dt> <dd data-k="height">—</dd>
              <dt>Status</dt> <dd data-k="status" style="font-weight:700;">—</dd>
            </dl>

            <ul data-k="errors" style="list-style:none; padding:0; margin:6px 0 4px;
                max-height:110px; overflow-y:auto; font-size:11px;"></ul>

            <div class="cad-popup-sep"></div>

            <div style="display:grid; grid-template-columns:1fr 1fr; gap:6px; margin-top:6px;">
              <button data-k="analyze"   class="cad-popup-btn cad-popup-btn-accent" type="button" style="grid-column:1/-1;">⟳ Analyze</button>
              <button data-k="rectangle" class="cad-popup-btn" type="button">Make Rectangle</button>
              <button data-k="square"    class="cad-popup-btn" type="button">Make Square</button>
              <button data-k="equalize"  class="cad-popup-btn" type="button" style="grid-column:1/-1;">Equalize Edges</button>
            </div>

            <div data-k="msg" class="cad-popup-msg"></div>
            <div class="cad-popup-hint">Esc = close · drag title to move</div>
          `;

          document.body.appendChild(el);

          // Перетаскивание за заголовок и блокировка canvas событий
          T.makeDraggable(el, el.querySelector('[data-k="titlebar"]'));
          T.blockCanvasEvents(el);

          return el;
        }

        function getPopup() {
          return document.getElementById('__profile-popup') || buildPopup();
        }

        function q(el, k) { return el.querySelector('[data-k="' + k + '"]'); }

        function closePopup() {
          const el = document.getElementById('__profile-popup');
          if (el) el.style.display = 'none';
        }

        // ── Render report into popup ────────────────────────────────────
        function renderReport(el, prof, rep) {
          q(el,'pid').textContent    = prof ? prof.id : '—';
          q(el,'type').textContent   = rep ? (rep.type || '—') : '—';
          q(el,'width').textContent  = (rep && isFinite(rep.widthMm))
            ? rep.widthMm.toFixed(2) + ' mm' : '—';
          q(el,'height').textContent = (rep && isFinite(rep.heightMm))
            ? rep.heightMm.toFixed(2) + ' mm' : '—';

          const statusEl = q(el, 'status');
          if (!rep) {
            statusEl.textContent = '—';
            statusEl.style.color = C.mute;
          } else {
            const errN  = (rep.errors || []).filter(e => e.severity === 'error').length;
            const warnN = (rep.errors || []).filter(e => e.severity === 'warn').length;
            if (errN > 0) {
              statusEl.textContent = 'errors (' + errN + ')';
              statusEl.style.color = C.danger;
            } else if (warnN > 0) {
              statusEl.textContent = 'warnings (' + warnN + ')';
              statusEl.style.color = C.warn;
            } else {
              statusEl.textContent = 'ok ✓';
              statusEl.style.color = C.ok;
            }
          }

          const errList = q(el, 'errors');
          errList.innerHTML = '';
          for (const er of ((rep && rep.errors) || [])) {
            const li = document.createElement('li');
            li.style.padding = '1px 0';
            li.style.color   = er.severity === 'error' ? '#fca5a5' : '#fde68a';
            if (er.kind === 'not_axis_aligned') {
              li.textContent = er.edgeId + ' ' + er.orient
                + ' drift ' + er.driftMm.toFixed(2) + ' mm';
            } else if (er.kind === 'length_mismatch') {
              li.textContent = er.edgeId + ' len '
                + er.actualMm.toFixed(2) + ' vs '
                + er.expectedMm.toFixed(2) + ' mm';
            } else if (er.kind === 'angle_not_90') {
              li.textContent = er.vertexPointId + ' angle '
                + er.angleDeg.toFixed(1) + '°';
            } else {
              li.textContent = er.kind;
            }
            errList.appendChild(li);
          }
        }

        function setMsg(el, txt, color) {
          const m = q(el, 'msg');
          m.textContent  = txt || '';
          m.style.color  = color || C.mute;
        }

        // ── Public: open at (px, py) ────────────────────────────────────
        window.__openProfilePopup = function(px, py) {
          const prof = window.__getSelectedProfile && window.__getSelectedProfile();
          if (!prof) return;

          const el = getPopup();

          // Auto-run analyze.
          const rep = window.__analyzeProfile && window.__analyzeProfile(prof);
          if (window.__profileCheckState)
            window.__profileCheckState = { report: rep, profileId: prof.id };
          renderReport(el, prof, rep);
          setMsg(el, '');

          T.positionNear(el, px, py);

          // Wire buttons once.
          if (!el.__wired) {
            el.__wired = true;

            q(el,'close').addEventListener('click', () => closePopup());

            el.addEventListener('keydown', e => {
              if (e.key === 'Escape') { e.preventDefault(); closePopup(); }
            });

            q(el,'analyze').addEventListener('click', async () => {
              const p = window.__getSelectedProfile && window.__getSelectedProfile();
              if (!p) { setMsg(el, 'No profile selected', C.danger); return; }
              const r = window.__analyzeProfile && window.__analyzeProfile(p);
              if (window.__profileCheckState)
                window.__profileCheckState = { report: r, profileId: p.id };
              renderReport(el, p, r);
              if (window.__updateSketchInspector) window.__updateSketchInspector();
              setMsg(el, r && r.ok ? 'Profile ok' : 'Issues found', r && r.ok ? C.ok : C.warn);
            });

            q(el,'rectangle').addEventListener('click', async () => {
              setMsg(el, 'Working…', C.mute);
              const r = await window.__makeSelectedProfileRectangle();
              if (!r || !r.ok) {
                setMsg(el, r && r.error ? r.error : 'Failed', C.danger); return;
              }
              // Re-analyze after fix.
              const p = window.__getSelectedProfile && window.__getSelectedProfile();
              const rep2 = window.__analyzeProfile && window.__analyzeProfile(p);
              if (window.__profileCheckState && p)
                window.__profileCheckState = { report: rep2, profileId: p.id };
              renderReport(el, p, rep2);
              if (window.__updateSketchInspector) window.__updateSketchInspector();
              setMsg(el, 'Made rectangle ✓', C.ok);
            });

            q(el,'square').addEventListener('click', async () => {
              setMsg(el, 'Working…', C.mute);
              const r = await window.__makeSelectedProfileSquare();
              if (!r || !r.ok) {
                setMsg(el, r && r.error ? r.error : 'Failed', C.danger); return;
              }
              const p = window.__getSelectedProfile && window.__getSelectedProfile();
              const rep2 = window.__analyzeProfile && window.__analyzeProfile(p);
              if (window.__profileCheckState && p)
                window.__profileCheckState = { report: rep2, profileId: p.id };
              renderReport(el, p, rep2);
              if (window.__updateSketchInspector) window.__updateSketchInspector();
              setMsg(el, 'Made square ✓', C.ok);
            });

            q(el,'equalize').addEventListener('click', async () => {
              setMsg(el, 'Working…', C.mute);
              const r = await window.__equalizeSelectedEdges();
              if (!r || !r.ok) {
                setMsg(el, r && r.error ? r.error : 'Failed', C.danger); return;
              }
              const p = window.__getSelectedProfile && window.__getSelectedProfile();
              const rep2 = window.__analyzeProfile && window.__analyzeProfile(p);
              if (window.__profileCheckState && p)
                window.__profileCheckState = { report: rep2, profileId: p.id };
              renderReport(el, p, rep2);
              if (window.__updateSketchInspector) window.__updateSketchInspector();
              setMsg(el, 'Equalized → ' + (r.avgMm || 0).toFixed(2) + ' mm ✓', C.ok);
            });
          } else {
            // Already wired — just re-render with current profile.
            // (buttons close over `getSelectedProfile` so they always see current)
          }
        };

        // Close when clicking outside the popup.
        document.addEventListener('pointerdown', e => {
          const el = document.getElementById('__profile-popup');
          if (el && el.style.display !== 'none' && !el.contains(e.target)) {
            closePopup();
          }
        }, true);

      })();
"##;
