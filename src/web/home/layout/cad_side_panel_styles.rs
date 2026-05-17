// ── CAD Side Panel — Styles ──────────────────────────────────────────────────

pub fn cad_side_panel_styles() -> &'static str {
    r##"
    /* ═══════════════════════════════════════════════
       CAD SIDE PANEL  (#cad-side-panel)
       ═══════════════════════════════════════════════ */

    #cad-side-panel {
      position: absolute;
      top: 12px;
      right: 12px;
      width: 264px;
      max-height: calc(100vh - 80px);
      display: flex;
      flex-direction: column;
      border-radius: 16px;
      background: rgba(14, 16, 22, 0.93);
      backdrop-filter: blur(20px);
      -webkit-backdrop-filter: blur(20px);
      border: 1px solid rgba(255, 255, 255, 0.07);
      box-shadow: 0 8px 40px rgba(0, 0, 0, 0.65), 0 1px 0 rgba(255,255,255,0.04) inset;
      z-index: 30;
      pointer-events: auto;
      overflow: hidden;
      font: 500 12px/1.5 -apple-system, "SF Pro Display", system-ui, monospace;
      color: #c8d0dc;
      user-select: none;
    }

    /* ── Top tabs ── */
    .csp-tabs {
      display: grid;
      grid-template-columns: repeat(5, 1fr);
      gap: 5px;
      padding: 8px 8px 4px;
      flex-shrink: 0;
    }

    .csp-tab {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      gap: 3px;
      height: 50px;
      border-radius: 10px;
      background: rgba(255, 255, 255, 0.05);
      border: 1px solid rgba(255, 255, 255, 0.06);
      color: #7a8594;
      cursor: pointer;
      transition: background 130ms, color 130ms, border-color 130ms;
      padding: 0;
    }
    .csp-tab:hover {
      background: rgba(255, 255, 255, 0.10);
      color: #c8d0dc;
      border-color: rgba(255, 255, 255, 0.14);
    }
    .csp-tab.active {
      background: rgba(103, 232, 249, 0.12);
      border-color: rgba(103, 232, 249, 0.35);
      color: #67e8f9;
    }
    .csp-tab-icon {
      font-size: 16px;
      line-height: 1;
    }
    .csp-tab-label {
      font-size: 9px;
      font-weight: 700;
      letter-spacing: 0.06em;
      text-transform: uppercase;
      line-height: 1;
    }

    /* thin line between tabs and body */
    .csp-tabs::after {
      content: '';
      display: block;
      grid-column: 1 / -1;
      height: 1px;
      background: rgba(255,255,255,0.07);
      margin: 4px -2px 0;
    }

    /* ── Panel body (scrollable) ── */
    .csp-body {
      overflow-y: auto;
      overflow-x: hidden;
      flex: 1 1 auto;
      padding: 6px 0 10px;
    }
    .csp-body::-webkit-scrollbar { width: 4px; }
    .csp-body::-webkit-scrollbar-thumb { background: rgba(148,163,184,0.20); border-radius: 2px; }

    /* ── Accordion section ── */
    .csp-section {
      margin: 4px 8px;
      border-radius: 12px;
      background: rgba(0, 0, 0, 0.38);
      border: 1px solid rgba(255, 255, 255, 0.05);
      overflow: hidden;
    }

    .csp-section-hdr {
      width: 100%;
      height: 40px;
      padding: 0 14px;
      display: flex;
      align-items: center;
      justify-content: space-between;
      background: none;
      border: none;
      color: #8e9aaa;
      font: 700 10px/1 -apple-system, system-ui, monospace;
      letter-spacing: 0.10em;
      text-transform: uppercase;
      cursor: pointer;
      transition: color 120ms;
    }
    .csp-section-hdr:hover { color: #c8d0dc; }
    .csp-section-hdr.open  { color: #e2e8f0; border-bottom: 1px solid rgba(255,255,255,0.06); }

    .csp-caret {
      font-size: 10px;
      transition: transform 200ms;
    }
    .csp-section-hdr.open .csp-caret { transform: rotate(0deg); }
    .csp-section-hdr:not(.open) .csp-caret { transform: rotate(-90deg); }

    .csp-section-body {
      padding: 10px 14px 12px;
      display: flex;
      flex-direction: column;
      gap: 7px;
    }

    /* ── Row ── */
    .csp-row {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 8px;
      min-height: 24px;
    }

    .csp-lbl {
      color: #64748b;
      font-size: 11px;
      flex-shrink: 0;
    }
    .csp-val {
      color: #e2e8f0;
      font-size: 11px;
      font-weight: 600;
      text-align: right;
    }
    .csp-val.csp-status-ok   { color: #34d399; }
    .csp-val.csp-status-warn { color: #fbbf24; }
    .csp-val.csp-status-err  { color: #f87171; }

    /* ── Checkbox rows ── */
    .csp-check {
      justify-content: flex-start;
      gap: 8px;
      cursor: pointer;
    }
    .csp-check input[type="checkbox"] {
      width: 14px;
      height: 14px;
      accent-color: #67e8f9;
      cursor: pointer;
    }

    /* ── Radio group ── */
    .csp-radio-group {
      display: flex;
      flex-direction: column;
      gap: 5px;
    }
    .csp-radio {
      display: flex;
      align-items: center;
      gap: 7px;
      cursor: pointer;
      font-size: 11px;
      color: #94a3b8;
    }
    .csp-radio input[type="radio"] {
      accent-color: #67e8f9;
      cursor: pointer;
    }
    .csp-radio:has(input:checked) { color: #e2e8f0; }

    /* ── Stepper (− value + unit +) ── */
    .csp-stepper {
      display: flex;
      align-items: center;
      gap: 3px;
    }
    .csp-step-btn {
      width: 22px;
      height: 22px;
      border-radius: 6px;
      background: rgba(255,255,255,0.07);
      border: 1px solid rgba(255,255,255,0.10);
      color: #94a3b8;
      font-size: 14px;
      line-height: 1;
      cursor: pointer;
      display: flex;
      align-items: center;
      justify-content: center;
      transition: background 100ms;
    }
    .csp-step-btn:hover { background: rgba(103,232,249,0.15); color: #67e8f9; }

    .csp-num-input {
      width: 56px;
      background: rgba(15, 23, 42, 0.75);
      border: 1px solid rgba(148,163,184,0.20);
      border-radius: 6px;
      color: #e2e8f0;
      font: inherit;
      font-size: 11px;
      padding: 3px 6px;
      text-align: right;
      transition: border-color 120ms;
    }
    .csp-num-input:focus {
      outline: none;
      border-color: rgba(103,232,249,0.55);
    }
    .csp-unit {
      font-size: 10px;
      color: #64748b;
      min-width: 18px;
    }

    /* ── Buttons ── */
    .csp-btn-row {
      display: flex;
      gap: 5px;
      flex-wrap: wrap;
    }
    .csp-btn-sm {
      flex: 1 1 auto;
      padding: 5px 8px;
      border-radius: 7px;
      background: rgba(255,255,255,0.06);
      border: 1px solid rgba(255,255,255,0.09);
      color: #94a3b8;
      font: 600 10px/1.4 -apple-system, system-ui, monospace;
      letter-spacing: 0.03em;
      cursor: pointer;
      transition: background 100ms, color 100ms;
      text-align: center;
    }
    .csp-btn-sm:hover {
      background: rgba(103,232,249,0.12);
      border-color: rgba(103,232,249,0.30);
      color: #67e8f9;
    }
    .csp-btn-full { width: 100%; flex: none; margin-top: 3px; }

    /* ── Divider ── */
    .csp-divider {
      height: 1px;
      background: rgba(255,255,255,0.07);
      margin: 2px 0;
    }

    /* ── Error list ── */
    .csp-error-list {
      font-size: 11px;
      color: #f87171;
      background: rgba(248,113,113,0.08);
      border-radius: 6px;
      padding: 6px 8px;
      line-height: 1.6;
    }

    /* ── JSON preview ── */
    .csp-json-pre {
      background: rgba(0,0,0,0.45);
      border-radius: 6px;
      padding: 7px 9px;
      font: 10px/1.5 "JetBrains Mono", monospace;
      color: #67e8f9;
      overflow: auto;
      max-height: 180px;
      white-space: pre-wrap;
      word-break: break-all;
      margin: 0;
    }
"##
}
