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

        const COLORS = {
          bg:      'rgba(15,23,42,0.97)',
          panel:   'rgba(30,41,59,0.9)',
          border:  'rgba(148,163,184,0.35)',
          frame:   'rgba(226,232,240,0.25)',
          fg:      '#e2e8f0',
          mute:    '#94a3b8',
          dim:     '#64748b',
          input:   '#f1f5f9',
          accent:  'rgba(99,102,241,0.85)',
          accent2: 'rgba(99,102,241,1.0)',
          danger:  '#f87171',
          warn:    '#fbbf24',
          ok:      '#34d399',
        };

        // ── Build popup DOM (once) ──────────────────────────────────────
        function buildPopup() {
          const el = document.createElement('div');
          el.id = '__profile-popup';
          Object.assign(el.style, {
            position:      'fixed',
            zIndex:        '9998',
            display:       'none',
            background:    COLORS.bg,
            border:        '1px solid ' + COLORS.frame,
            borderRadius:  '8px',
            padding:       '12px 14px',
            boxShadow:     '0 8px 32px rgba(0,0,0,0.7)',
            fontFamily:    '"JetBrains Mono", system-ui, monospace',
            fontSize:      '12px',
            color:         COLORS.fg,
            minWidth:      '240px',
            maxWidth:      '290px',
            pointerEvents: 'all',
            userSelect:    'none',
          });

          el.innerHTML = `
            <div style="display:flex; justify-content:space-between; align-items:center;
                 margin-bottom:8px;">
              <span style="font-size:11px; color:${COLORS.mute}; text-transform:uppercase;
                   letter-spacing:0.5px; font-weight:700;">Profile Check</span>
              <button data-k="close" type="button"
                style="background:none; border:none; color:${COLORS.mute};
                       font-size:14px; cursor:pointer; padding:0 2px; line-height:1;">✕</button>
            </div>

            <dl class="pp-grid">
              <dt>Id</dt>     <dd data-k="pid">—</dd>
              <dt>Type</dt>   <dd data-k="type">—</dd>
              <dt>Width</dt>  <dd data-k="width">—</dd>
              <dt>Height</dt> <dd data-k="height">—</dd>
              <dt>Status</dt> <dd data-k="status" style="font-weight:700;">—</dd>
            </dl>

            <ul data-k="errors" style="list-style:none; padding:0; margin:6px 0 4px;
                max-height:110px; overflow-y:auto; font-size:11px;"></ul>

            <div class="pp-sep"></div>

            <div style="display:grid; grid-template-columns:1fr 1fr; gap:6px; margin-top:6px;">
              <button data-k="analyze"   class="pp-btn pp-accent" type="button" style="grid-column:1/-1;">⟳ Analyze</button>
              <button data-k="rectangle" class="pp-btn" type="button">Make Rectangle</button>
              <button data-k="square"    class="pp-btn" type="button">Make Square</button>
              <button data-k="equalize"  class="pp-btn" type="button" style="grid-column:1/-1;">Equalize Edges</button>
            </div>

            <div data-k="msg" style="min-height:14px; margin-top:6px; font-size:11px;
                 color:${COLORS.mute};"></div>
            <div style="margin-top:4px; font-size:9px; color:${COLORS.dim};">Esc = close</div>
          `;

          // Scoped styles.
          if (!document.getElementById('__profile-popup-style')) {
            const css = document.createElement('style');
            css.id = '__profile-popup-style';
            css.textContent = `
              #__profile-popup .pp-grid {
                display:grid; grid-template-columns:60px 1fr;
                gap:2px 8px; margin:0 0 6px; font-size:11px;
              }
              #__profile-popup .pp-grid dt { color:${COLORS.mute}; }
              #__profile-popup .pp-grid dd { color:${COLORS.fg}; margin:0;
                text-align:right; font-weight:600; }
              #__profile-popup .pp-sep {
                height:1px; background:${COLORS.frame}; margin:6px 0;
              }
              #__profile-popup .pp-btn {
                background:${COLORS.panel};
                border:1px solid ${COLORS.border};
                border-radius:4px;
                color:${COLORS.fg};
                font-family:inherit; font-size:11px;
                padding:5px 8px; cursor:pointer;
              }
              #__profile-popup .pp-btn:hover { filter:brightness(1.18); }
              #__profile-popup .pp-accent {
                background:${COLORS.accent}; border-color:${COLORS.accent2};
                color:#fff;
              }
            `;
            document.head.appendChild(css);
          }

          document.body.appendChild(el);

          // Block orbit / canvas events from leaking through.
          ['pointerdown','mousedown'].forEach(ev =>
            el.addEventListener(ev, e => e.stopPropagation(), true));
          ['click','dblclick','contextmenu'].forEach(ev =>
            el.addEventListener(ev, e => e.stopPropagation(), false));

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
            statusEl.style.color = COLORS.mute;
          } else {
            const errN  = (rep.errors || []).filter(e => e.severity === 'error').length;
            const warnN = (rep.errors || []).filter(e => e.severity === 'warn').length;
            if (errN > 0) {
              statusEl.textContent = 'errors (' + errN + ')';
              statusEl.style.color = COLORS.danger;
            } else if (warnN > 0) {
              statusEl.textContent = 'warnings (' + warnN + ')';
              statusEl.style.color = COLORS.warn;
            } else {
              statusEl.textContent = 'ok ✓';
              statusEl.style.color = COLORS.ok;
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
          m.style.color  = color || COLORS.mute;
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

          el.style.display = 'block';

          // Position: keep within viewport.
          const vw = window.innerWidth, vh = window.innerHeight;
          const w  = el.offsetWidth  || 260;
          const h  = el.offsetHeight || 320;
          let left = px + 14, top = py - 14;
          if (left + w > vw - 8) left = px - w - 14;
          if (top  + h > vh - 8) top  = vh - h - 8;
          if (left < 8) left = 8;
          if (top  < 8) top  = 8;
          el.style.left = left + 'px';
          el.style.top  = top  + 'px';

          // Wire buttons once.
          if (!el.__wired) {
            el.__wired = true;

            q(el,'close').addEventListener('click', () => closePopup());

            el.addEventListener('keydown', e => {
              if (e.key === 'Escape') { e.preventDefault(); closePopup(); }
            });

            q(el,'analyze').addEventListener('click', async () => {
              const p = window.__getSelectedProfile && window.__getSelectedProfile();
              if (!p) { setMsg(el, 'No profile selected', COLORS.danger); return; }
              const r = window.__analyzeProfile && window.__analyzeProfile(p);
              if (window.__profileCheckState)
                window.__profileCheckState = { report: r, profileId: p.id };
              renderReport(el, p, r);
              if (window.__updateSketchInspector) window.__updateSketchInspector();
              setMsg(el, r && r.ok ? 'Profile ok' : 'Issues found', r && r.ok ? COLORS.ok : COLORS.warn);
            });

            q(el,'rectangle').addEventListener('click', async () => {
              setMsg(el, 'Working…', COLORS.mute);
              const r = await window.__makeSelectedProfileRectangle();
              if (!r || !r.ok) {
                setMsg(el, r && r.error ? r.error : 'Failed', COLORS.danger); return;
              }
              // Re-analyze after fix.
              const p = window.__getSelectedProfile && window.__getSelectedProfile();
              const rep2 = window.__analyzeProfile && window.__analyzeProfile(p);
              if (window.__profileCheckState && p)
                window.__profileCheckState = { report: rep2, profileId: p.id };
              renderReport(el, p, rep2);
              if (window.__updateSketchInspector) window.__updateSketchInspector();
              setMsg(el, 'Made rectangle ✓', COLORS.ok);
            });

            q(el,'square').addEventListener('click', async () => {
              setMsg(el, 'Working…', COLORS.mute);
              const r = await window.__makeSelectedProfileSquare();
              if (!r || !r.ok) {
                setMsg(el, r && r.error ? r.error : 'Failed', COLORS.danger); return;
              }
              const p = window.__getSelectedProfile && window.__getSelectedProfile();
              const rep2 = window.__analyzeProfile && window.__analyzeProfile(p);
              if (window.__profileCheckState && p)
                window.__profileCheckState = { report: rep2, profileId: p.id };
              renderReport(el, p, rep2);
              if (window.__updateSketchInspector) window.__updateSketchInspector();
              setMsg(el, 'Made square ✓', COLORS.ok);
            });

            q(el,'equalize').addEventListener('click', async () => {
              setMsg(el, 'Working…', COLORS.mute);
              const r = await window.__equalizeSelectedEdges();
              if (!r || !r.ok) {
                setMsg(el, r && r.error ? r.error : 'Failed', COLORS.danger); return;
              }
              const p = window.__getSelectedProfile && window.__getSelectedProfile();
              const rep2 = window.__analyzeProfile && window.__analyzeProfile(p);
              if (window.__profileCheckState && p)
                window.__profileCheckState = { report: rep2, profileId: p.id };
              renderReport(el, p, rep2);
              if (window.__updateSketchInspector) window.__updateSketchInspector();
              setMsg(el, 'Equalized → ' + (r.avgMm || 0).toFixed(2) + ' mm ✓', COLORS.ok);
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
