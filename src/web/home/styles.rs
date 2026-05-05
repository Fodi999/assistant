pub fn styles() -> &'static str {
    r##"
    :root {
      --bg: #050812;
      --panel: rgba(15, 23, 42, 0.78);
      --panel-border: rgba(148, 163, 184, 0.18);
      --text: #e5e7eb;
      --muted: #94a3b8;
      --accent: #38bdf8;
      --accent-2: #a78bfa;
      --good: #22c55e;
    }

    * {
      box-sizing: border-box;
    }

    body {
      margin: 0;
      min-height: 100vh;
      font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      color: var(--text);
      background:
        radial-gradient(circle at 20% 20%, rgba(56, 189, 248, 0.16), transparent 28%),
        radial-gradient(circle at 80% 30%, rgba(167, 139, 250, 0.18), transparent 30%),
        linear-gradient(180deg, #020617 0%, var(--bg) 100%);
      overflow-x: hidden;
    }

    .grid {
      position: fixed;
      inset: 0;
      background-image:
        linear-gradient(rgba(148, 163, 184, 0.08) 1px, transparent 1px),
        linear-gradient(90deg, rgba(148, 163, 184, 0.08) 1px, transparent 1px);
      background-size: 44px 44px;
      mask-image: linear-gradient(to bottom, black, transparent 82%);
      pointer-events: none;
    }

    .shell {
      position: relative;
      z-index: 1;
      min-height: 100vh;
      display: grid;
      grid-template-rows: auto 1fr auto;
    }

    header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 24px clamp(20px, 5vw, 72px);
    }

    .brand {
      display: flex;
      align-items: center;
      gap: 12px;
      font-weight: 800;
      letter-spacing: -0.03em;
    }

    .logo {
      width: 36px;
      height: 36px;
      border-radius: 12px;
      background: linear-gradient(135deg, var(--accent), var(--accent-2));
      box-shadow: 0 0 40px rgba(56, 189, 248, 0.35);
    }

    nav {
      display: flex;
      gap: 18px;
      color: var(--muted);
      font-size: 14px;
    }

    main {
      display: grid;
      grid-template-columns: minmax(0, 1.05fr) minmax(320px, 0.95fr);
      gap: 36px;
      align-items: center;
      padding: 32px clamp(20px, 5vw, 72px) 64px;
    }

    .hero h1 {
      margin: 0;
      max-width: 850px;
      font-size: clamp(46px, 8vw, 92px);
      line-height: 0.92;
      letter-spacing: -0.075em;
    }

    .hero p {
      margin: 28px 0 0;
      max-width: 680px;
      color: var(--muted);
      font-size: clamp(17px, 2vw, 22px);
      line-height: 1.55;
    }

    .actions {
      display: flex;
      flex-wrap: wrap;
      gap: 14px;
      margin-top: 34px;
    }

    .button {
      appearance: none;
      border: 0;
      border-radius: 16px;
      padding: 15px 20px;
      font-weight: 800;
      color: #020617;
      background: linear-gradient(135deg, var(--accent), #67e8f9);
      cursor: pointer;
      text-decoration: none;
      box-shadow: 0 18px 60px rgba(56, 189, 248, 0.25);
    }

    .button.secondary {
      color: var(--text);
      background: rgba(15, 23, 42, 0.75);
      border: 1px solid var(--panel-border);
      box-shadow: none;
    }

    .viewport-card {
      position: relative;
      min-height: 540px;
      border: 1px solid var(--panel-border);
      border-radius: 32px;
      background:
        linear-gradient(180deg, rgba(15, 23, 42, 0.92), rgba(2, 6, 23, 0.78)),
        radial-gradient(circle at 50% 20%, rgba(56, 189, 248, 0.24), transparent 36%);
      box-shadow: 0 28px 110px rgba(0, 0, 0, 0.45);
      overflow: hidden;
    }

    .fake-viewport {
      position: absolute;
      inset: 0;
      background-image:
        linear-gradient(rgba(56, 189, 248, 0.12) 1px, transparent 1px),
        linear-gradient(90deg, rgba(56, 189, 248, 0.12) 1px, transparent 1px);
      background-size: 34px 34px;
      transform: perspective(900px) rotateX(58deg) translateY(80px) scale(1.25);
      transform-origin: center bottom;
      opacity: 0.55;
    }

    .object {
      position: absolute;
      left: 50%;
      top: 43%;
      width: 180px;
      height: 180px;
      transform: translate(-50%, -50%) rotateX(58deg) rotateZ(45deg);
      border-radius: 28px;
      background: linear-gradient(135deg, rgba(56, 189, 248, 0.92), rgba(167, 139, 250, 0.92));
      box-shadow:
        0 0 0 1px rgba(255,255,255,0.24) inset,
        0 28px 80px rgba(56, 189, 248, 0.34);
    }

    .panel {
      position: absolute;
      right: 22px;
      top: 22px;
      width: 210px;
      padding: 16px;
      border-radius: 22px;
      border: 1px solid var(--panel-border);
      background: rgba(2, 6, 23, 0.72);
      backdrop-filter: blur(16px);
    }

    .panel h3 {
      margin: 0 0 12px;
      font-size: 14px;
    }

    .row {
      display: flex;
      justify-content: space-between;
      gap: 12px;
      padding: 8px 0;
      color: var(--muted);
      font-size: 13px;
      border-top: 1px solid rgba(148, 163, 184, 0.12);
    }

    .toolbar {
      position: absolute;
      left: 22px;
      top: 22px;
      display: grid;
      gap: 10px;
    }

    .tool {
      width: 44px;
      height: 44px;
      display: grid;
      place-items: center;
      border-radius: 14px;
      background: rgba(2, 6, 23, 0.74);
      border: 1px solid var(--panel-border);
      color: var(--text);
      font-weight: 800;
    }

    .status {
      position: absolute;
      left: 22px;
      right: 22px;
      bottom: 22px;
      display: flex;
      justify-content: space-between;
      gap: 14px;
      padding: 14px 16px;
      border-radius: 18px;
      background: rgba(2, 6, 23, 0.72);
      border: 1px solid var(--panel-border);
      color: var(--muted);
      font-size: 13px;
    }

    .dot {
      display: inline-block;
      width: 9px;
      height: 9px;
      margin-right: 8px;
      border-radius: 999px;
      background: var(--good);
      box-shadow: 0 0 20px rgba(34, 197, 94, 0.8);
    }

    footer {
      padding: 24px clamp(20px, 5vw, 72px);
      color: var(--muted);
      font-size: 13px;
    }

    /* ── Render Screen ── */
    #render-screen {
      position: fixed;
      inset: 0;
      display: none;
      overflow: hidden;
      background: #020617;
      z-index: 100;
    }

    body.gpu-active #render-screen { background: transparent; }

    body.engine-open .shell {
      display: none;
    }

    body.engine-open #render-screen {
      display: block;
    }

    #webgpu-canvas {
      position: absolute;
      inset: 0;
      width: 100vw;
      height: 100vh;
      display: block;
      z-index: 0;
    }

    .render-gradient {
      position: absolute;
      inset: 0;
      z-index: 1;
      pointer-events: none;
      background:
        radial-gradient(circle at 28% 38%, rgba(56, 189, 248, 0.2), transparent 30%),
        radial-gradient(circle at 70% 50%, rgba(167, 139, 250, 0.22), transparent 34%),
        linear-gradient(180deg, rgba(2, 6, 23, 0.2), rgba(2, 6, 23, 0.96));
    }

    .render-grid {
      position: absolute;
      left: 8%;
      right: 8%;
      bottom: -18%;
      height: 55%;
      z-index: 2;
      pointer-events: none;
      background-image:
        linear-gradient(rgba(56, 189, 248, 0.1) 1px, transparent 1px),
        linear-gradient(90deg, rgba(56, 189, 248, 0.1) 1px, transparent 1px);
      background-size: 44px 44px;
      transform: perspective(900px) rotateX(62deg);
      transform-origin: center bottom;
      mask-image: linear-gradient(to top, black, transparent);
      opacity: 0.75;
    }

    /* ── Islands ── */
    .island {
      position: absolute;
      z-index: 20;
      border-radius: 22px;
      background: rgba(8, 14, 28, 0.62);
      border: 1px solid rgba(148, 163, 184, 0.18);
      backdrop-filter: blur(18px);
      -webkit-backdrop-filter: blur(18px);
      box-shadow:
        0 18px 70px rgba(0, 0, 0, 0.34),
        inset 0 1px 0 rgba(255, 255, 255, 0.045);
    }

    .island-top {
      left: 16px;
      right: 16px;
      top: 10px;
      min-height: 48px;
      display: grid;
      grid-template-columns: 1fr auto 1fr;
      align-items: center;
      gap: 18px;
      padding: 8px 12px 8px 18px;
      border-radius: 16px;
    }

    .island-top strong {
      color: #f8fafc;
      font-size: 14px;
    }

    .island-top span {
      color: #64748b;
      font-size: 12px;
      font-weight: 900;
      letter-spacing: 0.08em;
      text-align: center;
    }

    #close-chefos {
      justify-self: end;
      min-height: 34px;
      padding: 0 14px;
      border-radius: 10px;
      border: 1px solid rgba(148, 163, 184, 0.2);
      background: rgba(15, 23, 42, 0.8);
      color: #cbd5e1;
      font-size: 13px;
      cursor: pointer;
      appearance: none;
    }

    #close-chefos:hover {
      color: #f8fafc;
      border-color: rgba(148, 163, 184, 0.4);
    }

    .island-tools {
      left: 20px;
      top: 72px;
      width: 92px;
      display: grid;
      gap: 10px;
      padding: 12px;
    }

    .tool-button {
      min-height: 42px;
      border-radius: 13px;
      border: 1px solid rgba(148, 163, 184, 0.18);
      color: #94a3b8;
      background: rgba(15, 23, 42, 0.72);
      font-weight: 900;
      font-size: 12px;
      cursor: pointer;
      appearance: none;
    }

    .tool-button:hover {
      color: #e2e8f0;
      border-color: rgba(56, 189, 248, 0.4);
    }

    .tool-button.active {
      color: #020617;
      background: linear-gradient(135deg, #38bdf8, #67e8f9);
      border-color: transparent;
    }

    .island-inspector {
      right: 20px;
      top: 72px;
      width: 310px;
      padding: 18px;
    }

    .island-inspector h3 {
      margin: 0 0 12px;
      color: #f8fafc;
      font-size: 14px;
    }

    .inspector-row {
      display: flex;
      justify-content: space-between;
      gap: 16px;
      padding: 10px 0;
      border-top: 1px solid rgba(148, 163, 184, 0.14);
    }

    .inspector-row span {
      color: #64748b;
      font-size: 12px;
      font-weight: 900;
    }

    .inspector-row strong {
      color: #38bdf8;
      font-size: 13px;
      text-align: right;
    }

    #selected-info {
      margin: 8px 0 0;
      color: #94a3b8;
      font-size: 13px;
      line-height: 1.45;
    }

    .island-command {
      left: 50%;
      bottom: 22px;
      transform: translateX(-50%);
      width: min(980px, calc(100vw - 44px));
      display: grid;
      grid-template-columns: 180px 1fr auto;
      gap: 18px;
      align-items: center;
      padding: 14px 16px;
    }

    .command-label {
      display: block;
      margin-bottom: 4px;
      color: #64748b;
      font-size: 11px;
      font-weight: 900;
      text-transform: uppercase;
      letter-spacing: 0.08em;
    }

    .island-command strong {
      color: #e5e7eb;
      font-size: 14px;
    }

    .command-actions {
      display: flex;
      gap: 8px;
    }

    .command-actions button {
      min-height: 38px;
      padding: 0 13px;
      border-radius: 12px;
      border: 1px solid rgba(148, 163, 184, 0.18);
      color: #dbeafe;
      background: rgba(15, 23, 42, 0.78);
      font-weight: 900;
      font-size: 13px;
      cursor: pointer;
      appearance: none;
    }

    .command-actions button:hover {
      color: #020617;
      background: linear-gradient(135deg, #38bdf8, #67e8f9);
    }

    /* ── GPU Card Overlay — 3D unified objects ── */
    .gpu-card-overlay {
      position: absolute;
      inset: 0;
      z-index: 8;
      pointer-events: none;
      perspective: 1400px;
      perspective-origin: 50% 42%;
    }

    .gpu-card {
      position: absolute;
      left: var(--x);
      top: var(--y);
      width: 138px;
      min-height: 210px;
      transform: translate(-50%, -50%)
                 rotateY(var(--ry, 0deg))
                 translateZ(var(--tz, 0px));
      transform-style: preserve-3d;
      display: grid;
      grid-template-rows: 1fr auto auto;
      gap: 8px;
      padding: 10px;
      border-radius: 20px;
      border: 1px solid rgba(148, 163, 184, 0.15);
      background: rgba(8, 16, 34, 0.52);
      color: #e5e7eb;
      pointer-events: auto;
      cursor: pointer;
      box-shadow:
        0 2px 0 rgba(255,255,255,0.04) inset,
        0 28px 80px rgba(0, 0, 0, 0.5);
      backdrop-filter: blur(14px);
      -webkit-backdrop-filter: blur(14px);
      transition: transform 220ms cubic-bezier(.2,.8,.3,1),
                  border-color 180ms ease,
                  box-shadow 180ms ease;
      appearance: none;
      text-align: left;
    }

    .gpu-card.active {
      transform: translate(-50%, -58%)
                 rotateY(0deg)
                 translateZ(calc(var(--tz, 0px) + 48px));
      border-color: rgba(56, 189, 248, 0.9);
      background: rgba(4, 18, 42, 0.72);
      box-shadow:
        0 0 0 1px rgba(56, 189, 248, 0.35),
        0 0 40px rgba(56, 189, 248, 0.22),
        0 40px 100px rgba(0, 0, 0, 0.55);
    }

    .gpu-card-image {
      display: grid;
      place-items: center;
      min-height: 110px;
      border-radius: 14px;
      background: rgba(248, 250, 252, 0.96);
      overflow: hidden;
    }

    .gpu-card-image img   { width:100%; height:100%; object-fit:cover; }
    .gpu-card-image span  { font-size: 54px; line-height:1; }
    .gpu-card strong      { font-size: 13px; line-height: 1.2; color: #f1f5f9; }
    .gpu-card small       { color: #64748b; font-size: 11px; font-weight: 700; }

    .scene-stage { display: none !important; }

    .gallery-stage {
      position: relative;
      width: 0;
      height: 0;
      transform-style: preserve-3d;
      transition: transform 0.55s cubic-bezier(0.25, 0.46, 0.45, 0.94);
    }

    .ingredient-card {
      position: absolute;
      width: 130px;
      margin-left: -65px;
      margin-top: -80px;
      padding-bottom: 12px;
      border-radius: 20px;
      background: rgba(10, 18, 40, 0.88);
      border: 1px solid rgba(148, 163, 184, 0.16);
      backdrop-filter: blur(10px);
      cursor: pointer;
      user-select: none;
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 6px;
      transition: border-color 0.2s, box-shadow 0.2s;
      will-change: transform;
    }

    .ingredient-card:hover {
      border-color: rgba(56, 189, 248, 0.45);
    }

    .ingredient-card.active {
      border-color: rgba(56, 189, 248, 0.8);
      box-shadow:
        0 0 0 2px rgba(56, 189, 248, 0.35),
        0 20px 60px rgba(56, 189, 248, 0.28);
    }

    .card-img {
      width: 106px;
      height: 100px;
      border-radius: 14px;
      object-fit: cover;
      display: block;
      margin-top: 12px;
      background: rgba(15, 23, 42, 0.6);
    }

    @media (max-width: 920px) {
      .island-command {
        grid-template-columns: 1fr;
      }
      .command-actions {
        flex-wrap: wrap;
      }
    }

    @media (max-width: 980px) {
      main {
        grid-template-columns: 1fr;
      }

      .viewport-card {
        min-height: 460px;
      }

      nav {
        display: none;
      }
    }
"##
}
