// ── Matter Lab CSS — UI overlay above WebGPU canvas ──────────────────────────────

pub fn matter_lab_styles() -> &'static str {
    r##"
    /* === Matter Lab shell ============================================ */
    .matter-lab-shell {
      position: absolute; inset: 0;
      display: block;
      color: #e5e7eb;
      font-family: Inter, ui-sans-serif, system-ui, "Segoe UI", sans-serif;
      pointer-events: none;            /* canvas сквозит, дочерние сами включают события */
    }
    .matter-lab-shell button,
    .matter-lab-shell input,
    .matter-lab-shell .tool-btn,
    .matter-lab-shell .matter-panel,
    .matter-lab-shell .matter-topbar,
    .matter-lab-shell .left-tools,
    .matter-lab-shell .action-bar,
    .matter-lab-shell .status-bar { pointer-events: auto; }

    /* canvas itself must receive pointer events for orbit/zoom/push */
    .matter-stage > #webgpu-canvas { pointer-events: auto; }
    /* stage is transparent to clicks — panels above it handle themselves */
    .matter-stage { pointer-events: none; }

    /* === Top bar ===================================================== */
    .matter-topbar {
      position: absolute; top: 0; left: 0; right: 0;
      height: 56px;
      display: grid;
      grid-template-columns: auto 1fr auto;
      align-items: center;
      padding: 0 22px;
      background: rgba(7, 11, 22, 0.72);
      backdrop-filter: blur(14px);
      border-bottom: 1px solid rgba(148, 163, 184, 0.10);
      z-index: 20;
    }
    .matter-topbar .brand {
      display: flex; align-items: center; gap: 10px;
      font-weight: 600; font-size: 14px; letter-spacing: 0.2px;
    }
    .matter-topbar .brand-icon {
      width: 22px; height: 22px; border-radius: 6px;
      background: linear-gradient(135deg, #38bdf8, #a78bfa);
      box-shadow: 0 0 14px rgba(56, 189, 248, 0.5);
    }
    .matter-topbar .top-title {
      text-align: center;
      font-size: 12px;
      color: rgba(148, 163, 184, 0.75);
      letter-spacing: 0.4px;
    }
    .matter-topbar .top-nav {
      display: flex; align-items: center; gap: 6px;
      justify-self: end;
    }
    .matter-topbar .top-nav button {
      background: transparent; border: 1px solid transparent;
      color: rgba(226, 232, 240, 0.78);
      font: 500 12px/1 Inter, system-ui, sans-serif;
      padding: 8px 12px; border-radius: 8px;
      cursor: pointer; transition: all .15s ease;
    }
    .matter-topbar .top-nav button:hover {
      background: rgba(255, 255, 255, 0.05);
      border-color: rgba(148, 163, 184, 0.18);
      color: #fff;
    }
    .matter-topbar .top-nav .back-btn {
      width: 32px; height: 32px; padding: 0;
      border: 1px solid rgba(148, 163, 184, 0.22);
      background: rgba(15, 23, 42, 0.6);
      font-size: 14px;
    }
    .matter-topbar .user-badge {
      width: 32px; height: 32px; border-radius: 50%;
      display: grid; place-items: center;
      background: linear-gradient(135deg, #38bdf8, #a78bfa);
      color: #0b1120; font: 700 12px/1 Inter; letter-spacing: 0.4px;
      margin-left: 6px;
    }

    /* === Stage (full-bleed canvas area) ============================== */
    .matter-stage {
      position: absolute; inset: 56px 0 0 0;   /* leave room for topbar */
      overflow: hidden;
    }
"##
}

pub fn matter_tools_styles() -> &'static str {
    r##"
    /* === Left tools rail ============================================ */
    .left-tools {
      position: absolute;
      top: 24px; left: 22px;
      display: flex; flex-direction: column; gap: 8px;
      padding: 10px;
      background: rgba(8, 14, 28, 0.78);
      backdrop-filter: blur(14px);
      border: 1px solid rgba(148, 163, 184, 0.14);
      border-radius: 16px;
      z-index: 15;
      box-shadow: 0 18px 40px rgba(0, 0, 0, 0.45);
    }
    .tool-btn {
      width: 96px; padding: 10px 12px;
      background: rgba(15, 23, 42, 0.55);
      border: 1px solid rgba(148, 163, 184, 0.14);
      border-radius: 10px;
      color: rgba(226, 232, 240, 0.82);
      font: 500 12px/1.2 Inter, system-ui, sans-serif;
      letter-spacing: 0.3px;
      text-align: left;
      cursor: pointer;
      transition: all .18s ease;
    }
    .tool-btn:hover {
      border-color: rgba(56, 189, 248, 0.35);
      color: #fff;
      transform: translateX(2px);
    }
    .tool-btn.active {
      background: linear-gradient(135deg, rgba(56, 189, 248, 0.22), rgba(167, 139, 250, 0.22));
      border-color: rgba(56, 189, 248, 0.6);
      color: #fff;
      box-shadow: 0 0 0 1px rgba(56, 189, 248, 0.25), 0 12px 28px rgba(56, 189, 248, 0.18);
    }
"##
}

pub fn matter_panel_styles() -> &'static str {
    // ── Matter Panel Removed ──
    r##""##
}

pub fn matter_action_bar_styles() -> &'static str {
    r##"
    /* === Bottom action bar ========================================== */
    .action-bar {
      position: absolute;
      left: 50%; transform: translateX(-50%);
      bottom: 56px;
      display: flex; align-items: center; gap: 10px;
      padding: 12px 16px;
      background: rgba(8, 14, 28, 0.86);
      backdrop-filter: blur(16px);
      border: 1px solid rgba(148, 163, 184, 0.16);
      border-radius: 16px;
      z-index: 15;
      box-shadow: 0 26px 50px rgba(0, 0, 0, 0.55);
    }
    .action-bar .action-meta {
      display: flex; flex-direction: column; gap: 2px;
      padding: 0 12px;
      border-right: 1px solid rgba(148, 163, 184, 0.12);
    }
    .action-bar .action-meta:nth-of-type(2) {
      border-right: 1px solid rgba(148, 163, 184, 0.12);
      padding-right: 16px;
    }
    .action-bar .action-meta small {
      font-size: 9px; letter-spacing: 0.6px; text-transform: uppercase;
      color: rgba(148, 163, 184, 0.7);
    }
    .action-bar .action-meta strong {
      font-size: 12px; color: #fff; font-weight: 600;
    }
    .action-bar button[data-action] {
      padding: 9px 14px;
      background: rgba(15, 23, 42, 0.55);
      border: 1px solid rgba(148, 163, 184, 0.18);
      border-radius: 10px;
      color: rgba(226, 232, 240, 0.9);
      font: 500 12px/1 Inter; cursor: pointer;
      letter-spacing: 0.3px;
      transition: all .15s ease;
    }
    .action-bar button[data-action]:hover {
      background: linear-gradient(135deg, rgba(56, 189, 248, 0.22), rgba(167, 139, 250, 0.22));
      border-color: rgba(56, 189, 248, 0.5);
      color: #fff;
      transform: translateY(-1px);
    }
    .action-bar button[data-action="reset"]:hover {
      background: rgba(248, 113, 113, 0.18);
      border-color: rgba(248, 113, 113, 0.5);
    }
"##
}

pub fn matter_status_styles() -> &'static str {
    r##"
    /* === Status bar ================================================= */
    .status-bar {
      position: absolute;
      left: 0; right: 0; bottom: 0;
      height: 32px;
      display: flex; align-items: center; justify-content: space-between;
      padding: 0 22px;
      background: rgba(5, 8, 18, 0.78);
      border-top: 1px solid rgba(148, 163, 184, 0.10);
      font-size: 11px;
      color: rgba(226, 232, 240, 0.85);
      z-index: 14;
    }
    .status-bar .online-dot {
      display: inline-block; width: 7px; height: 7px;
      background: #22c55e; border-radius: 50%;
      box-shadow: 0 0 10px rgba(34, 197, 94, 0.7);
      margin-right: 6px;
      animation: matterPulse 1.6s ease-in-out infinite;
    }
    .status-bar .muted {
      margin-left: 12px;
      color: rgba(148, 163, 184, 0.6);
    }
    .status-bar .webgpu-status {
      margin-left: 12px;
      font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
      color: rgba(148, 163, 184, 0.65);
      transition: color .4s ease;
    }
    .status-bar .perf {
      display: flex; gap: 16px;
      font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
      color: rgba(148, 163, 184, 0.85);
    }
    .status-bar .perf b {
      color: #38bdf8; font-weight: 600;
      margin-left: 4px;
    }

    @keyframes matterPulse {
      0%, 100% { opacity: 1; }
      50%      { opacity: 0.5; }
    }

    /* canvas in matter stage fills the area completely */
    .matter-stage > #webgpu-canvas {
      position: absolute; inset: 0;
      width: 100%; height: 100%;
      z-index: 0;
      display: block;
    }

    /* ── Axis gizmo ─────────────────────────────── */
    #axis-gizmo {
      position: absolute;
      top: 16px; right: 16px;
      width: 96px; height: 96px;
      z-index: 20;
      cursor: pointer;
      border-radius: 50%;
      background: rgba(8, 14, 28, 0.55);
      border: 1px solid rgba(148, 163, 184, 0.14);
      backdrop-filter: blur(8px);
      pointer-events: auto;
      transition: border-color .2s;
    }
    #axis-gizmo:hover {
      border-color: rgba(56, 189, 248, 0.45);
    }
"##
}
