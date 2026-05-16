// ── Dimension editor popup (Phase 17) ─────────────────────────────────────
//
// Provides:
//   window.__openDimensionEditor(hit, px, py)  — shows an input popup near
//     the clicked dimension label with the current value pre-filled.
//   window.__applyDimensionEdit(hit, newValueMm) — commits the new value by
//     dispatching to __setEdgeLengthMm (or future profile-dim handlers).
//
// The popup is a single <div id="__dim-editor"> that is reused each time.

pub const JS: &str = r##"
      (function registerDimensionEditor() {

        // ── Build / reuse the popup element ──────────────────────────────
        function getOrCreatePopup() {
          let el = document.getElementById('__dim-editor');
          if (el) return el;
          el = document.createElement('div');
          el.id = '__dim-editor';
          Object.assign(el.style, {
            position:      'fixed',
            zIndex:        '9999',
            display:       'none',
            background:    'rgba(15,23,42,0.97)',
            border:        '1px solid rgba(226,232,240,0.25)',
            borderRadius:  '6px',
            padding:       '8px 10px',
            boxShadow:     '0 4px 24px rgba(0,0,0,0.6)',
            fontFamily:    '"JetBrains Mono", system-ui, monospace',
            fontSize:      '13px',
            color:         '#e2e8f0',
            minWidth:      '120px',
            pointerEvents: 'all',
          });

          const label = document.createElement('div');
          label.style.cssText = 'margin-bottom:5px;font-size:10px;color:#94a3b8;user-select:none;';
          label.textContent = 'Length (mm)';
          el.appendChild(label);

          const row = document.createElement('div');
          row.style.cssText = 'display:flex;gap:5px;align-items:center;';

          const input = document.createElement('input');
          input.id = '__dim-editor-input';
          Object.assign(input.style, {
            background:   'rgba(30,41,59,0.9)',
            border:       '1px solid rgba(148,163,184,0.35)',
            borderRadius: '4px',
            padding:      '3px 7px',
            color:        '#f1f5f9',
            fontFamily:   'inherit',
            fontSize:     '13px',
            width:        '80px',
            outline:      'none',
          });

          const btn = document.createElement('button');
          btn.textContent = '✓';
          Object.assign(btn.style, {
            background:   'rgba(99,102,241,0.8)',
            border:       'none',
            borderRadius: '4px',
            color:        '#fff',
            fontFamily:   'inherit',
            fontSize:     '13px',
            padding:      '3px 8px',
            cursor:       'pointer',
          });

          row.appendChild(input);
          row.appendChild(btn);
          el.appendChild(row);

          const hint = document.createElement('div');
          hint.style.cssText = 'margin-top:4px;font-size:9px;color:#64748b;user-select:none;';
          hint.textContent = 'Enter = apply · Esc = cancel';
          el.appendChild(hint);

          document.body.appendChild(el);
          return el;
        }

        // ── Close popup ───────────────────────────────────────────────────
        function closeEditor() {
          const el = document.getElementById('__dim-editor');
          if (el) el.style.display = 'none';
        }

        // ── Apply edit ────────────────────────────────────────────────────
        window.__applyDimensionEdit = async function(hit, newValueMm) {
          closeEditor();
          if (!isFinite(newValueMm) || newValueMm <= 0) {
            window.__setStatusMessage?.('Invalid dimension value');
            return;
          }
          if (hit.kind === 'edge_length_dimension' && hit.edgeId) {
            await window.__setEdgeLengthMm?.(hit.edgeId, newValueMm);
          } else {
            window.__setStatusMessage?.('Dimension type not editable: ' + hit.kind);
          }
        };

        // ── Open popup ────────────────────────────────────────────────────
        window.__openDimensionEditor = function(hit, px, py) {
          const el = getOrCreatePopup();
          const input = document.getElementById('__dim-editor-input');

          // Pre-fill with current value (accept comma or dot).
          const currentMm = (typeof hit.valueMm === 'number' && isFinite(hit.valueMm))
            ? hit.valueMm.toFixed(2).replace('.', ',')
            : '';
          input.value = currentMm;

          // Position near click, keep within viewport.
          const vw = window.innerWidth, vh = window.innerHeight;
          let left = px + 8, top = py - 12;
          el.style.display = 'block';
          const w = el.offsetWidth  || 160;
          const h = el.offsetHeight || 80;
          if (left + w > vw - 8) left = px - w - 8;
          if (top + h > vh - 8)  top  = vh - h - 8;
          if (top < 4) top = 4;
          el.style.left = left + 'px';
          el.style.top  = top  + 'px';

          // Store hit descriptor on element for event handlers.
          el.__hit = hit;

          input.focus();
          input.select();

          // ── Keyboard events ───────────────────────────────────────────
          function parseInput() {
            // Allow European comma decimal: "10,5" → 10.5
            return parseFloat(input.value.replace(',', '.'));
          }

          // Remove old listeners by replacing element clone trick avoided —
          // use a single named function stored on element.
          if (el.__keyhandler) input.removeEventListener('keydown', el.__keyhandler);
          el.__keyhandler = function(e) {
            if (e.key === 'Enter') {
              e.preventDefault();
              e.stopPropagation();
              const v = parseInput();
              window.__applyDimensionEdit(el.__hit, v);
            } else if (e.key === 'Escape') {
              e.preventDefault();
              closeEditor();
            }
          };
          input.addEventListener('keydown', el.__keyhandler);

          // Button click.
          const btn = el.querySelector('button');
          if (btn) {
            btn.onclick = () => {
              const v = parseInput();
              window.__applyDimensionEdit(el.__hit, v);
            };
          }
        };

        // Close on outside click.
        document.addEventListener('pointerdown', (e) => {
          const el = document.getElementById('__dim-editor');
          if (el && el.style.display !== 'none' && !el.contains(e.target)) {
            closeEditor();
          }
        }, true);

      })();
"##;
