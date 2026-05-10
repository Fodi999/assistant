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
      /* Делаем фон полностью монолитным, чтобы 3D-сцена на 100% перекрывалась */
      background: #0f111a; 
      border-bottom: 1px solid rgba(255, 255, 255, 0.08);
      z-index: 100; /* Гарантированно поверх сцены и гизмо */
      transform: translateY(0);
      transition: transform 0.3s cubic-bezier(0.16, 1, 0.3, 1);
    }
    
    .header-collapsed .matter-topbar {
      transform: translateY(-100%);
    }

    .topbar-toggle-btn {
      position: absolute;
      top: 56px; /* Язычок крепится сразу под панелью */
      left: 50%;
      transform: translateX(-50%);
      width: 48px;
      height: 20px;
      background: #0f111a;
      border: 1px solid rgba(255, 255, 255, 0.08);
      border-top: none;
      border-radius: 0 0 8px 8px;
      color: rgba(226, 232, 240, 0.7);
      cursor: pointer;
      display: flex; align-items: center; justify-content: center;
      font-size: 10px;
      z-index: 100;
      transition: top 0.3s cubic-bezier(0.16, 1, 0.3, 1), background 0.15s, color 0.15s;
    }
    
    .header-collapsed .topbar-toggle-btn {
      top: 0px; /* При закрытой панели язычок остается на самом верху экрана */
      border-top: 1px solid rgba(255, 255, 255, 0.08);
    }
    
    .topbar-toggle-btn:hover {
      background: #1e2233; 
      color: #fff;
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
      position: absolute; inset: 0;   /* теперь canvas идет на весь экран */
      overflow: hidden;
    }

    /* Floating Close Button */
    .close-engine-btn {
      position: absolute;
      top: 16px; left: 16px;
      width: 40px; height: 40px;
      background: rgba(15, 23, 42, 0.65);
      border: 1px solid rgba(148, 163, 184, 0.25);
      backdrop-filter: blur(8px);
      border-radius: 50%; /* Makes it a perfect circle */
      color: rgba(226, 232, 240, 0.95);
      font-size: 16px; /* Bigger cross */
      display: flex; align-items: center; justify-content: center;
      cursor: pointer;
      z-index: 50;
      transition: all .15s ease;
      box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    }
    .close-engine-btn:hover {
      background: rgba(220, 38, 38, 0.8); /* Red on hover */
      color: #fff;
      transform: scale(1.05); /* Slight pop effect */
    }
"##
}

pub fn matter_tools_styles() -> &'static str {
    r##"
    /* === Left tools rail ============================================ */
    .left-tools {
      position: absolute;
      top: 186px;
      left: 16px; 
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
    r##"
    /* === Right Properties Panel (Blender N-panel style) ========================= */
    .matter-panel-right {
      position: absolute;
      top: 15px; 
      right: 15px; 
      bottom: 50px; /* above the bottom row elements */
      width: var(--panel-width, 420px);
      background: rgba(30, 30, 32, 0.85); /* Blender-like dark grey */
      backdrop-filter: blur(12px);
      border: 1px solid rgba(80, 80, 85, 0.4);
      border-radius: 8px;
      z-index: 15;
      display: flex;
      flex-direction: column;
      transform: translateX(0);
      transition: transform 0.3s cubic-bezier(0.16, 1, 0.3, 1);
      box-shadow: -4px 0 15px rgba(0, 0, 0, 0.25);
    }
    
    .panel-resizer {
      position: absolute;
      top: 0;
      bottom: 0;
      left: -4px;
      width: 8px;
      cursor: ew-resize;
      z-index: 20;
    }
    
    .matter-panel-right.collapsed {
      /* Translation precisely hides the panel off-screen, leaving exactly the 32px tab visible flush with the right edge */
      transform: translateX(calc(100% + 15px));
      z-index: 16; /* Ensure collapsed tabs render on top of the open panel body */
    }
    
    .panel-toggle-btn {
      position: absolute;
      left: -32px; 
      width: 32px; 
      height: 90px; /* Enough space for Blender-style vertical text */
      background: rgba(24, 24, 26, 0.95); /* Darker inactive tab background */
      border: 1px solid rgba(80, 80, 85, 0.4);
      border-right: none;
      border-radius: 6px 0 0 6px;
      color: rgba(148, 163, 184, 0.85);
      cursor: pointer;
      font-size: 11px;
      font-weight: 500;
      letter-spacing: 0.5px;
      transition: all 0.15s ease;
      
      /* Blender aesthetic: Top-to-bottom reading, vertically centered */
      display: flex; align-items: center; justify-content: center;
      writing-mode: vertical-rl;
      
      padding: 0;
      box-sizing: border-box;
      outline: none;
      z-index: 10;
    }
    
    .panel-toggle-btn.tab-n {
      top: 15px; 
    }

    .panel-toggle-btn.tab-shape {
      top: 106px; /* 15px + 90px height + 1px gap */
    }

    .panel-toggle-btn.tab-material {
      top: 197px; 
    }

    .panel-toggle-btn.tab-nodes {
      top: 288px; 
    }

    .panel-toggle-btn.tab-history {
      top: 379px; 
    }

    .panel-toggle-btn.tab-ai {
      top: 470px; 
    }

    .panel-toggle-btn.tab-m {
      top: 561px; 
    }
    
    .panel-toggle-btn:hover { 
      background: rgba(50, 50, 55, 0.95); 
      color: #fff; 
    }
    
    .panel-toggle-btn.active {
      background: rgba(30, 30, 32, 0.85); /* Seamlessly matches the active panel body */
      color: #fff;
      border-left: 2px solid #38bdf8; /* Blue highlight line to indicate Active tab (Blender style) */
    }

    .panel-header {
      padding: 12px 16px;
      font-size: 13px;
      font-weight: 600;
      color: #e2e8f0;
      border-bottom: 1px solid rgba(255, 255, 255, 0.08);
      letter-spacing: 0.3px;
    }
    
    .panel-body {
      flex: 1;
      padding: 16px;
      overflow-y: auto;
    }
    
    .prop-section {
      margin-bottom: 20px;
    }
    
    .prop-title {
      font-size: 11px;
      text-transform: uppercase;
      font-weight: 700;
      color: rgba(226, 232, 240, 0.5);
      letter-spacing: 0.8px;
      margin-bottom: 8px;
    }
    
    .prop-value {
      font-size: 13px;
      color: #e2e8f0;
      background: rgba(15, 23, 42, 0.4);
      padding: 8px 12px;
      border-radius: 6px;
      border: 1px solid rgba(255, 255, 255, 0.06);
    }
    
    .prop-value.text-muted {
      color: rgba(226, 232, 240, 0.4);
      font-style: italic;
    }
    
    .prop-actions {
      display: flex;
      flex-direction: column;
      gap: 8px;
    }
    
    .prop-btn {
      width: 100%;
      text-align: center;
      padding: 8px 12px;
      background: rgba(255, 255, 255, 0.08);
      border: 1px solid rgba(255, 255, 255, 0.1);
      border-radius: 6px;
      color: #e2e8f0;
      font-size: 12px;
      font-weight: 500;
      cursor: pointer;
      transition: all 0.15s ease;
    }
    
    .prop-btn:hover {
      background: rgba(255, 255, 255, 0.15);
      border-color: rgba(255, 255, 255, 0.2);
    }
    
    .prop-btn.highlight {
      background: rgba(56, 189, 248, 0.15);
      border-color: rgba(56, 189, 248, 0.4);
      color: #38bdf8;
    }
    
    .prop-btn.highlight:hover {
      background: rgba(56, 189, 248, 0.25);
    }
    "##
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
      top: 16px;
      right: 36px; /* 32px (tabs width) + 4px gap */
      left: auto;
      width: 96px; height: 96px;
      z-index: 20;
      cursor: pointer;
      border-radius: 50%;
      background: rgba(8, 14, 28, 0.55);
      border: 1px solid rgba(148, 163, 184, 0.14);
      backdrop-filter: blur(8px);
      pointer-events: auto;
      transition: right 0.3s cubic-bezier(0.16, 1, 0.3, 1), border-color 0.2s;
    }
    
    /* When a sidebar panel is open, push the gizmo to the left of the panel */
    body.panel-open #axis-gizmo {
      /* 15px (panel right) + panel width + 32px (tabs) + 4px (gap) = 51px */
      right: calc(var(--panel-width, 420px) + 51px);
    }
    
    /* Disable transitions temporarily when user is drag-resizing */
    .is-resizing .matter-panel-right,
    .is-resizing #axis-gizmo {
      transition: none !important;
    }
    
    #axis-gizmo:hover {
      border-color: rgba(56, 189, 248, 0.45);
    }
"##
}
