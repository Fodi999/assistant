// ── Grab Gizmo (Plasticity-style) ──
// Вызывается из render_loop.rs:  drawGrabGizmo(ctx, sketchState, w2s, sk)
//
// Экспортируется как window.__drawGrabGizmo = drawGrabGizmo;

pub const JS: &str = r#"
  // ── Collect point ids for gizmo (selection or active grab) ──
  window.__collectSelectedPointIdsForGizmo = function() {
    const pts = new Set();
    const sk = sketchState;

    if (sk.grab.active) {
      for (const id of sk.grab.pointIds) pts.add(id);
      return pts;
    }

    // Selected points directly
    for (const id of sk.selectedPointIds) pts.add(id);

    // Points from selected edges
    for (const eId of sk.selectedEdgeIds) {
      const e = sk.edges.find(e => e.id === eId);
      if (e) { pts.add(e.a); pts.add(e.b); }
    }

    // Points from selected profile
    if (sk.selectedProfileId) {
      const prof = sk.profiles && sk.profiles.find(p => p.id === sk.selectedProfileId);
      if (prof && prof.pointIds) for (const id of prof.pointIds) pts.add(id);
      else if (prof && prof.edgeIds) {
        for (const eId of prof.edgeIds) {
          const e = sk.edges.find(e => e.id === eId);
          if (e) { pts.add(e.a); pts.add(e.b); }
        }
      }
    }

    return pts;
  };

  // ── Start grab from gizmo handle click ──
  window.__startGrabFromGizmo = function(axis, clientX, clientY) {
    const ids = window.__collectSelectedPointIdsForGizmo();
    if (!ids.size) return;

    // Collect moveable (non-fixed) ids
    const moveIds = [...ids].filter(id => !window.__isPointFixed || !window.__isPointFixed(id));
    if (!moveIds.length) { window.__setStatusMessage('Cannot move fixed point'); return; }

    window.__pushHistory();
    const byId = new Map(sketchState.points.map(p => [p.id, p]));
    const snapshot = new Map();
    for (const id of moveIds) {
      const p = byId.get(id);
      if (p) snapshot.set(id, { x: p.x, y: p.y, z: p.z });
    }
    const startWorld = sketchState.hoverWorld
      ? { x: sketchState.hoverWorld.x, y: sketchState.hoverWorld.y, z: sketchState.hoverWorld.z }
      : { x: 0, y: 0, z: 0 };
    const canvas = document.getElementById('matterCanvas');
    const rect = canvas ? canvas.getBoundingClientRect() : { left:0, top:0, width:1, height:1 };
    const dpr2 = window.devicePixelRatio || 1;

    // Compute selection center (for drag plane anchor)
    let _scx=0, _scy=0, _scz=0, _scn=0;
    for (const id of moveIds) {
      const p = byId.get(id); if(!p) continue;
      _scx+=p.x; _scy+=p.y; _scz+=p.z; _scn++;
    }
    const startCenter = _scn ? { x:_scx/_scn, y:_scy/_scn, z:_scz/_scn } : { x:0,y:0,z:0 };

    // Compute initial drag point from click NDC position
    const ndcClickX = ((clientX - rect.left) / rect.width)  * 2 - 1;
    const ndcClickY = 1 - ((clientY - rect.top) / rect.height) * 2;

    // startScreen anchored to gizmo center (canvas device-px)
    const _ctr = window.__gizmoCenterScreen;
    const startScreen = _ctr
      ? { x: _ctr.x, y: _ctr.y }
      : { x: (clientX - rect.left) * dpr2, y: (clientY - rect.top) * dpr2 };

    const dragBase = new Map();
    for (const id of moveIds) {
      const p = byId.get(id);
      if (p) dragBase.set(id, { x: p.x, y: p.y, z: p.z });
    }

    sketchState.grab = {
      active: true,
      pointIds: moveIds,
      startMouseWorld: startWorld,
      startScreen,
      startCenter,
      startDragPoint: null,  // set on first __updateGizmoDrag call
      originalPoints: snapshot,
      axisLock: (axis === 'FREE') ? null : axis,
      dragBase,
      screenAcc: { x: 0, y: 0, z: 0 },
      numericInput: '',
      useScreenProjection: true,
    };

    const planeName = sketchState.workingPlane || 'XZ';
    window.__setStatusMessage('⤢ Захват ' + moveIds.length + ' т. · пл.' + planeName + ' · ' + (axis === 'FREE' ? 'свободно' : axis + '-ось') + ' · Enter ✓ · Esc ✗');
    console.log('[Gizmo] start axis ' + axis + ', points: ' + moveIds.length);
    window.__grabIsScreenProjection = true;
    if (window.__resetGrabTracking) window.__resetGrabTracking();
    if (window.__updateSketchInspector) window.__updateSketchInspector();
  };

function drawGrabGizmo(ctx, sketchState, w2s, sk) {
  const isGrabbing = sketchState.grab.active;

  if (!isGrabbing) {
    window.__gizmoHandles   = null;
    window.__gizmoHoverAxis = null;
    return;
  }

  const grab  = sketchState.grab;
  const lock  = grab.axisLock || null;
  const hov   = window.__gizmoHoverAxis || null;
  const numIn = grab.numericInput || '';

  // ── Centroid of selected/grabbed points ──────────────────────────
  const byId = new Map(sketchState.points.map(p => [p.id, p]));
  const gizmoIds = window.__collectSelectedPointIdsForGizmo
    ? window.__collectSelectedPointIdsForGizmo()
    : sketchState.selectedPointIds;

  let cx = 0, cy0 = 0, cz = 0, n = 0;
  for (const id of gizmoIds) {
    const p = byId.get(id); if (!p) continue;
    cx += p.x; cy0 += p.y; cz += p.z; n++;
  }
  if (n > 0) { cx /= n; cy0 /= n; cz /= n; }
  const origin = w2s(cx, cy0, cz);
  if (!origin) { window.__gizmoHandles = null; return; }

  // ── Constants ───────────────────────────────────────────────────
  const ARM   = 90;  // axis arrow length px
  const SHAFT_W_NORM = 2.2;
  const SHAFT_W_ACT  = 3.5;
  const ARROW_L = 16, ARROW_W = 6;   // arrowhead px
  const LABEL_OFF = 10;              // label beyond arrow tip px
  const SQ    = 18;                  // planar square half-size px
  const SQ_OF = 28;                  // planar square offset from origin px
  const HIT_A = 22, HIT_P = 18, HIT_C = 14;

  const C_X = '#ff4444', C_Y = '#22dd66', C_Z = '#3399ff', C_FREE = '#e8e8e8';

  // ── Helper: screen direction from world axis ─────────────────────
  function screenDir(wx, wy, wz) {
    const far = w2s(cx + wx, cy0 + wy, cz + wz);
    if (!far) return null;
    const dx = far.x - origin.x, dy = far.y - origin.y;
    const len = Math.hypot(dx, dy) || 1;
    return { x: dx / len, y: dy / len };
  }
  const dirX = screenDir(1, 0, 0);
  const dirY = screenDir(0, 1, 0);
  const dirZ = screenDir(0, 0, 1);
  if (!dirX || !dirY || !dirZ) { window.__gizmoHandles = null; return; }

  // ── Expose to mouse.rs ───────────────────────────────────────────
  (function() {
    function rawVec(wx, wy, wz) {
      const f = w2s(cx + wx, cy0 + wy, cz + wz);
      if (!f) return null;
      const ddx = f.x - origin.x, ddy = f.y - origin.y;
      return { dx: ddx, dy: ddy, pxPerUnit: Math.hypot(ddx, ddy) || 1 };
    }
    window.__gizmoAxisScreenDirs = {
      X: rawVec(1, 0, 0), Y: rawVec(0, 1, 0), Z: rawVec(0, 0, 1),
    };
  })();

  const handles = [];
  ctx.save();

  // ════════════════════════════════════════════════════════════════
  // 1. DASHED INFINITE GUIDE LINE along locked axis
  // ════════════════════════════════════════════════════════════════
  const lockColor = lock === 'X' ? C_X : lock === 'Y' ? C_Y : lock === 'Z' ? C_Z : C_FREE;

  if (lock && ['X','Y','Z'].includes(lock)) {
    const startW = grab.startMouseWorld || { x: cx, y: cy0, z: cz };
    const ox = startW.x, oy = startW.y, oz = startW.z;
    const d  = 2000;
    let p1, p2;
    if (lock === 'X') { p1 = w2s(ox-d, oy, oz);   p2 = w2s(ox+d, oy, oz); }
    if (lock === 'Y') { p1 = w2s(ox, oy-d, oz);   p2 = w2s(ox, oy+d, oz); }
    if (lock === 'Z') { p1 = w2s(ox, oy, oz-d);   p2 = w2s(ox, oy, oz+d); }
    if (p1 && p2) {
      ctx.save();
      ctx.setLineDash([6, 5]);
      ctx.strokeStyle = lockColor;
      ctx.globalAlpha = 0.55;
      ctx.lineWidth   = 1.2;
      ctx.beginPath(); ctx.moveTo(p1.x, p1.y); ctx.lineTo(p2.x, p2.y); ctx.stroke();
      ctx.restore();
    }
  }

  // ════════════════════════════════════════════════════════════════
  // 2. LIVE MOVEMENT ARROW: origin → current position (yellow)
  // ════════════════════════════════════════════════════════════════
  if (grab.pointIds && grab.pointIds.length > 0) {
    const sId   = grab.pointIds[0];
    const sOrig = grab.originalPoints && grab.originalPoints.get(sId);
    const sNow  = byId.get(sId);
    if (sOrig && sNow) {
      const origScreen = w2s(sOrig.x, sOrig.y, sOrig.z);
      const nowScreen  = w2s(sNow.x,  sNow.y,  sNow.z);
      if (origScreen && nowScreen) {
        const mvdx = nowScreen.x - origScreen.x;
        const mvdy = nowScreen.y - origScreen.y;
        const mvLen = Math.hypot(mvdx, mvdy);
        if (mvLen > 3) {
          const ux = mvdx / mvLen, uy = mvdy / mvLen;
          ctx.save();
          ctx.globalAlpha = 0.90;
          // dashed line from original to current
          ctx.setLineDash([4, 3]);
          ctx.strokeStyle = '#ffe066';
          ctx.lineWidth   = 1.5;
          ctx.beginPath();
          ctx.moveTo(origScreen.x, origScreen.y);
          ctx.lineTo(nowScreen.x - ux * 10, nowScreen.y - uy * 10);
          ctx.stroke();
          ctx.setLineDash([]);
          // arrowhead at current position
          const aA = Math.atan2(uy, ux);
          ctx.fillStyle = '#ffe066';
          ctx.beginPath();
          ctx.moveTo(nowScreen.x + ux * 3, nowScreen.y + uy * 3);
          ctx.lineTo(nowScreen.x - 10 * Math.cos(aA) + 5 * Math.sin(aA),
                     nowScreen.y - 10 * Math.sin(aA) - 5 * Math.cos(aA));
          ctx.lineTo(nowScreen.x - 10 * Math.cos(aA) - 5 * Math.sin(aA),
                     nowScreen.y - 10 * Math.sin(aA) + 5 * Math.cos(aA));
          ctx.closePath(); ctx.fill();
          // small circle at original position
          ctx.beginPath();
          ctx.arc(origScreen.x, origScreen.y, 3.5, 0, Math.PI * 2);
          ctx.fillStyle   = 'rgba(255,224,102,0.6)';
          ctx.fill();
          ctx.strokeStyle = '#ffe066';
          ctx.lineWidth   = 1;
          ctx.stroke();
          ctx.restore();
        }
      }
    }
  }

  // ════════════════════════════════════════════════════════════════
  // 3. PLANAR SQUARES (XY / YZ / XZ) — small squares between axes
  // ════════════════════════════════════════════════════════════════
  const planesDef = [
    { axis: 'XY', col: C_X,    dA: dirX, dB: dirY },
    { axis: 'YZ', col: C_Y,    dA: dirY, dB: dirZ },
    { axis: 'XZ', col: C_Z,    dA: dirX, dB: dirZ },
  ];
  for (const pl of planesDef) {
    const active = lock === pl.axis || hov === pl.axis;
    const ox = origin.x + pl.dA.x * SQ_OF + pl.dB.x * SQ_OF;
    const oy = origin.y + pl.dA.y * SQ_OF + pl.dB.y * SQ_OF;
    const c0x = ox, c0y = oy;
    const c1x = ox + pl.dA.x * SQ,              c1y = oy + pl.dA.y * SQ;
    const c2x = c1x + pl.dB.x * SQ,             c2y = c1y + pl.dB.y * SQ;
    const c3x = ox  + pl.dB.x * SQ,             c3y = oy  + pl.dB.y * SQ;
    ctx.beginPath();
    ctx.moveTo(c0x,c0y); ctx.lineTo(c1x,c1y);
    ctx.lineTo(c2x,c2y); ctx.lineTo(c3x,c3y);
    ctx.closePath();
    if (active) {
      ctx.globalAlpha = 0.55; ctx.fillStyle = pl.col; ctx.fill();
      ctx.globalAlpha = 1.0;  ctx.strokeStyle = '#fff'; ctx.lineWidth = 1.8; ctx.stroke();
    } else {
      ctx.globalAlpha = lock ? 0.05 : 0.18; ctx.fillStyle = pl.col; ctx.fill();
      ctx.globalAlpha = lock ? 0.08 : 0.55; ctx.strokeStyle = pl.col; ctx.lineWidth = 1.2; ctx.stroke();
    }
    ctx.globalAlpha = 1.0;
    handles.push({ axis: pl.axis, x: (c0x+c2x)/2, y: (c0y+c2y)/2, r: HIT_P });
  }

  // ════════════════════════════════════════════════════════════════
  // 4. AXIS ARROWS: shaft + solid arrowhead + axis label
  // ════════════════════════════════════════════════════════════════
  const axesDef = [
    { axis: 'X', col: C_X, dir: dirX },
    { axis: 'Y', col: C_Y, dir: dirY },
    { axis: 'Z', col: C_Z, dir: dirZ },
  ];
  for (const a of axesDef) {
    const active    = lock === a.axis || hov === a.axis;
    const dimmed    = lock && lock !== a.axis && !['XY','YZ','XZ'].includes(lock);
    const alpha     = dimmed ? 0.20 : (active ? 1.0 : 0.90);
    const tx = origin.x + a.dir.x * ARM;
    const ty = origin.y + a.dir.y * ARM;
    const angle = Math.atan2(a.dir.y, a.dir.x);

    ctx.globalAlpha = alpha;

    // Shadow / glow for active axis
    if (active) {
      ctx.shadowColor = a.col;
      ctx.shadowBlur  = 8;
    }

    // Shaft (stop before arrowhead)
    ctx.strokeStyle = a.col;
    ctx.lineWidth   = active ? SHAFT_W_ACT : SHAFT_W_NORM;
    ctx.lineCap     = 'round';
    ctx.beginPath();
    ctx.moveTo(origin.x, origin.y);
    ctx.lineTo(tx - a.dir.x * ARROW_L, ty - a.dir.y * ARROW_L);
    ctx.stroke();

    // Arrowhead (solid filled triangle)
    ctx.fillStyle = a.col;
    ctx.shadowBlur = 0;
    ctx.beginPath();
    ctx.moveTo(tx + a.dir.x * 3, ty + a.dir.y * 3);
    ctx.lineTo(tx - ARROW_L * Math.cos(angle) + ARROW_W * Math.sin(angle),
               ty - ARROW_L * Math.sin(angle) - ARROW_W * Math.cos(angle));
    ctx.lineTo(tx - ARROW_L * Math.cos(angle) - ARROW_W * Math.sin(angle),
               ty - ARROW_L * Math.sin(angle) + ARROW_W * Math.cos(angle));
    ctx.closePath(); ctx.fill();

    // Axis label (X / Y / Z)
    const lx = tx + a.dir.x * LABEL_OFF;
    const ly = ty + a.dir.y * LABEL_OFF;
    ctx.font         = active ? 'bold 13px system-ui, sans-serif'
                              : '12px system-ui, sans-serif';
    ctx.fillStyle    = active ? '#ffffff' : a.col;
    ctx.textAlign    = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(a.axis, lx, ly);

    ctx.globalAlpha = 1.0;
    ctx.shadowBlur  = 0;
    handles.push({ axis: a.axis, x: tx, y: ty, r: HIT_A });
  }

  // ════════════════════════════════════════════════════════════════
  // 5. FREE-MOVE SQUARE at centre (replaces old circle)
  // ════════════════════════════════════════════════════════════════
  const freeActive = !lock || lock === 'FREE' || hov === 'FREE';
  const FS = 9; // half-size of the square px
  ctx.globalAlpha = (lock && lock !== 'FREE') ? 0.25 : (freeActive ? 1.0 : 0.80);
  ctx.shadowColor  = freeActive ? 'rgba(255,255,255,0.6)' : 'transparent';
  ctx.shadowBlur   = freeActive ? 6 : 0;

  // Filled square
  ctx.fillStyle = freeActive ? 'rgba(255,255,255,0.25)' : 'rgba(255,255,255,0.08)';
  ctx.beginPath();
  ctx.rect(origin.x - FS, origin.y - FS, FS * 2, FS * 2);
  ctx.fill();
  // Border
  ctx.strokeStyle = freeActive ? '#ffffff' : 'rgba(255,255,255,0.5)';
  ctx.lineWidth   = freeActive ? 2.0 : 1.2;
  ctx.stroke();
  // Inner dot
  ctx.beginPath();
  ctx.arc(origin.x, origin.y, freeActive ? 3.0 : 2.0, 0, Math.PI * 2);
  ctx.fillStyle = freeActive ? '#ffffff' : 'rgba(255,255,255,0.5)';
  ctx.fill();

  ctx.globalAlpha = 1.0;
  ctx.shadowBlur  = 0;
  handles.push({ axis: 'FREE', x: origin.x, y: origin.y, r: HIT_C });

  ctx.restore();

  window.__gizmoHandles      = handles;
  window.__gizmoCenterScreen = origin;

  // ════════════════════════════════════════════════════════════════
  // 6. DELTA READOUT near cursor (floating tooltip)
  // ════════════════════════════════════════════════════════════════
  if (grab.pointIds && grab.pointIds.length > 0) {
    const sId   = grab.pointIds[0];
    const sOrig = grab.originalPoints && grab.originalPoints.get(sId);
    const sNow  = byId.get(sId);
    if (sOrig && sNow) {
      const ddx  = ((sNow.x - sOrig.x) * 1000).toFixed(1);
      const ddy  = ((sNow.y - sOrig.y) * 1000).toFixed(1);
      const ddz  = ((sNow.z - sOrig.z) * 1000).toFixed(1);
      const dist = (Math.hypot(sNow.x-sOrig.x, sNow.y-sOrig.y, sNow.z-sOrig.z) * 1000).toFixed(1);

      // Build label line
      let label;
      if (lock === 'X')       label = 'X  ' + ddx + ' мм';
      else if (lock === 'Y')  label = 'Y  ' + ddy + ' мм';
      else if (lock === 'Z')  label = 'Z  ' + ddz + ' мм';
      else                    label = 'X ' + ddx + '  Y ' + ddy + '  Z ' + ddz + ' мм';

      // If numeric input is being typed, show it prominently
      const numLabel = numIn ? '▶ ' + (lock || '?') + ' ' + numIn + '_' : null;

      const sx = (sketchState.hoverWorld && sketchState.hoverWorld.screenX) != null
                 ? sketchState.hoverWorld.screenX + 18 : origin.x + 20;
      const sy = (sketchState.hoverWorld && sketchState.hoverWorld.screenY) != null
                 ? sketchState.hoverWorld.screenY - 8  : origin.y - 8;

      ctx.save();
      // Main delta pill
      ctx.font = '11px "JetBrains Mono", monospace';
      const tw1 = ctx.measureText(label).width + 16;
      const pillH = numLabel ? 40 : 22;
      const tw = numLabel ? Math.max(tw1, ctx.measureText(numLabel).width + 16) : tw1;
      ctx.fillStyle = 'rgba(8,12,28,0.92)';
      ctx.beginPath(); ctx.roundRect(sx, sy - pillH + 22, tw, pillH, 5); ctx.fill();
      ctx.strokeStyle = lockColor + '88'; ctx.lineWidth = 1;
      ctx.stroke();

      ctx.textAlign = 'left'; ctx.textBaseline = 'middle';
      if (numLabel) {
        ctx.font      = 'bold 13px "JetBrains Mono", monospace';
        ctx.fillStyle = '#ffe066';
        ctx.fillText(numLabel, sx + 8, sy - pillH + 22 + 11);
        ctx.font      = '10px "JetBrains Mono", monospace';
        ctx.fillStyle = lockColor;
        ctx.fillText(label, sx + 8, sy - pillH + 22 + 30);
      } else {
        ctx.fillStyle = lockColor;
        ctx.fillText(label, sx + 8, sy - pillH + 22 + 11);
      }
      ctx.restore();
    }
  }

  // ════════════════════════════════════════════════════════════════
  // 7. TOP BANNER
  // ════════════════════════════════════════════════════════════════
  {
    const ptCount = grab.pointIds ? grab.pointIds.length : n;
    const axisLabel = lock ? lock : 'свободно';
    const hint = numIn
      ? ('⤢ Перемещение · ' + (lock||'?') + ' · ' + numIn + '_  Enter ✓  Esc ✗')
      : ('⤢ Перемещение ' + ptCount + ' точек · ' + axisLabel + '  |  X/Y/Z · Enter ✓ · Esc ✗');

    ctx.save();
    ctx.font = 'bold 12px system-ui, sans-serif';
    const bw  = ctx.measureText(hint).width + 28;
    const bx  = sk.width / 2 - bw / 2;
    const by  = 10;
    // Background
    ctx.fillStyle = 'rgba(8,12,28,0.92)';
    ctx.beginPath(); ctx.roundRect(bx, by, bw, 30, 7); ctx.fill();
    // Coloured left stripe
    ctx.fillStyle = lockColor;
    ctx.beginPath(); ctx.roundRect(bx, by, 4, 30, [7,0,0,7]); ctx.fill();
    // Text
    ctx.fillStyle    = numIn ? '#ffe066' : (lock ? lockColor : '#e8e8e8');
    ctx.textAlign    = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(hint, sk.width / 2, by + 15);
    ctx.restore();
  }
}

window.__drawGrabGizmo = drawGrabGizmo;
"#;
