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
    const rect = canvas ? canvas.getBoundingClientRect() : { left:0, top:0 };
    const startScreen = { x: clientX - rect.left, y: clientY - rect.top };

    sketchState.grab = {
      active: true,
      pointIds: moveIds,
      startMouseWorld: startWorld,
      startScreen,
      originalPoints: snapshot,
      axisLock: (axis === 'FREE') ? null : axis,
      dragBase: snapshot,  // will be reset on first pointermove
      screenAcc: { x: 0, y: 0, z: 0 },  // accumulates screen-space world delta
    };

    // Also set dragBase properly
    sketchState.grab.dragBase = new Map();
    for (const id of moveIds) {
      const p = byId.get(id);
      if (p) sketchState.grab.dragBase.set(id, { x: p.x, y: p.y, z: p.z });
    }

    window.__setStatusMessage('⤢ Grab ' + moveIds.length + ' pt — ' + (axis === 'FREE' ? 'free' : axis + '-lock'));
    console.log('[Gizmo] start axis ' + axis + ', points: ' + moveIds.length);
    window.__grabIsScreenProjection = true;
    if (window.__resetGrabTracking) window.__resetGrabTracking();
    if (window.__updateSketchInspector) window.__updateSketchInspector();
  };

function drawGrabGizmo(ctx, sketchState, w2s, sk) {
  // Show gizmo ONLY when grab is active (G key pressed)
  const isGrabbing = sketchState.grab.active;

  if (!isGrabbing) {
    window.__gizmoHandles   = null;
    window.__gizmoHoverAxis = null;
    return;
  }

  const grab  = sketchState.grab;
  const lock  = grab.axisLock || null;
  const hov   = window.__gizmoHoverAxis || null;

  // ── Centroid ──
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

  // ── Screen-space axis directions (fixed 80px length) ──
  const ARM   = 80;   // px
  const SQ    = 22;   // planar square size px
  const SQ_OF = 26;   // planar square offset from origin px
  const HIT_A = 20;   // hit radius for axis tip
  const HIT_P = 20;   // hit radius for plane square centre
  const HIT_C = 16;   // hit radius for centre FREE

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

  const axes = [
    { axis: 'X', color: '#f04040', dir: dirX },
    { axis: 'Y', color: '#20d060', dir: dirY },
    { axis: 'Z', color: '#3d8fff', dir: dirZ },
  ];

  // Planar squares: positioned between two axes at SQ_OF distance
  const planes = [
    { axis: 'XY', color: '#f04040', dA: dirX, dB: dirY },
    { axis: 'YZ', color: '#20d060', dA: dirY, dB: dirZ },
    { axis: 'XZ', color: '#3d8fff', dA: dirX, dB: dirZ },
  ];

  const handles = [];
  ctx.save();

  // ── 1. Planar squares ──
  for (const pl of planes) {
    const isLocked  = lock  === pl.axis;
    const isHovered = hov   === pl.axis;
    const active    = isLocked || isHovered;

    // Square corners in screen space
    const ox = origin.x + pl.dA.x * SQ_OF + pl.dB.x * SQ_OF;
    const oy = origin.y + pl.dA.y * SQ_OF + pl.dB.y * SQ_OF;
    // 4 corners going around the square
    const c0x = ox,                             c0y = oy;
    const c1x = ox + pl.dA.x * SQ,             c1y = oy + pl.dA.y * SQ;
    const c2x = ox + pl.dA.x * SQ + pl.dB.x * SQ, c2y = oy + pl.dA.y * SQ + pl.dB.y * SQ;
    const c3x = ox + pl.dB.x * SQ,             c3y = oy + pl.dB.y * SQ;

    ctx.beginPath();
    ctx.moveTo(c0x, c0y); ctx.lineTo(c1x, c1y);
    ctx.lineTo(c2x, c2y); ctx.lineTo(c3x, c3y);
    ctx.closePath();

    const axisColors = { XY: '#f04040', YZ: '#20d060', XZ: '#3d8fff' };
    const col = axisColors[pl.axis];

    if (active) {
      ctx.globalAlpha = 0.55;
      ctx.fillStyle   = col;
      ctx.fill();
      ctx.globalAlpha = 1.0;
      ctx.strokeStyle = '#ffffff';
      ctx.lineWidth   = 1.5;
      ctx.stroke();
    } else {
      ctx.globalAlpha = lock ? 0.06 : 0.18;
      ctx.fillStyle   = col;
      ctx.fill();
      ctx.globalAlpha = lock ? 0.08 : 0.45;
      ctx.strokeStyle = col;
      ctx.lineWidth   = 1;
      ctx.stroke();
    }
    ctx.globalAlpha = 1.0;

    const hcx = (c0x + c2x) / 2;
    const hcy = (c0y + c2y) / 2;
    handles.push({ axis: pl.axis, x: hcx, y: hcy, r: HIT_P });
  }

  // ── 2. Axis shafts + arrowheads ──
  for (const a of axes) {
    const isLocked  = lock === a.axis;
    const isHovered = hov  === a.axis;
    const active    = isLocked || isHovered;

    const tx = origin.x + a.dir.x * ARM;
    const ty = origin.y + a.dir.y * ARM;

    // Dim non-selected axes when one is locked
    ctx.globalAlpha = lock && !isLocked ? 0.18 : (active ? 1.0 : 0.88);

    // Shaft
    ctx.strokeStyle = a.color;
    ctx.lineWidth   = active ? 2.5 : 1.8;
    ctx.lineCap     = 'round';
    ctx.beginPath();
    ctx.moveTo(origin.x, origin.y);
    ctx.lineTo(tx - a.dir.x * 14, ty - a.dir.y * 14); // stop before arrowhead
    ctx.stroke();

    // Arrowhead (cone-style, 3 lines)
    const hw = 4.5, hl = 14;
    const angle = Math.atan2(a.dir.y, a.dir.x);
    ctx.fillStyle = a.color;
    ctx.beginPath();
    ctx.moveTo(tx + a.dir.x * 2,   ty + a.dir.y * 2);
    ctx.lineTo(tx - hl * Math.cos(angle) + hw * Math.sin(angle),
               ty - hl * Math.sin(angle) - hw * Math.cos(angle));
    ctx.lineTo(tx - hl * Math.cos(angle) - hw * Math.sin(angle),
               ty - hl * Math.sin(angle) + hw * Math.cos(angle));
    ctx.closePath();
    ctx.fill();

    handles.push({ axis: a.axis, x: tx, y: ty, r: HIT_A });
  }

  // ── 3. Centre FREE circle ──
  const cActive  = lock === 'FREE' || hov === 'FREE';
  ctx.globalAlpha = lock && lock !== 'FREE' ? 0.2 : (cActive ? 1.0 : 0.75);

  // Outer ring
  ctx.beginPath();
  ctx.arc(origin.x, origin.y, 7, 0, Math.PI * 2);
  ctx.strokeStyle = '#ffffff';
  ctx.lineWidth   = cActive ? 2.0 : 1.2;
  ctx.stroke();

  // Inner fill
  ctx.beginPath();
  ctx.arc(origin.x, origin.y, 3.5, 0, Math.PI * 2);
  ctx.fillStyle   = cActive ? '#ffffff' : 'rgba(255,255,255,0.5)';
  ctx.fill();

  if (cActive) {
    // Extra outer glow ring
    ctx.beginPath();
    ctx.arc(origin.x, origin.y, 11, 0, Math.PI * 2);
    ctx.strokeStyle = 'rgba(255,255,255,0.35)';
    ctx.lineWidth   = 1;
    ctx.stroke();
  }

  handles.push({ axis: 'FREE', x: origin.x, y: origin.y, r: HIT_C });

  ctx.globalAlpha = 1.0;
  ctx.restore();

  window.__gizmoHandles      = handles;
  window.__gizmoCenterScreen = origin;

  // ── Expose axis screen-space directions for projection-based drag ──
  // Used by mouse.rs to compute correct world delta per pixel at any camera angle.
  (function() {
    function rawVec(wx, wy, wz) {
      const f = w2s(cx + wx, cy0 + wy, cz + wz);
      if (!f) return null;
      const ddx = f.x - origin.x, ddy = f.y - origin.y;
      const px = Math.hypot(ddx, ddy) || 1;
      return { dx: ddx, dy: ddy, pxPerUnit: px };
    }
    window.__gizmoAxisScreenDirs = {
      X: rawVec(1, 0, 0),
      Y: rawVec(0, 1, 0),
      Z: rawVec(0, 0, 1),
    };
  })();

  // ── Dashed guide line along locked single axis ──
  const lockColor = lock === 'X' ? '#f04040'
                  : lock === 'Y' ? '#20d060'
                  : lock === 'Z' ? '#3d8fff'
                  : '#a78bfa';

  if (isGrabbing && lock && ['X','Y','Z'].includes(lock) && grab.startMouseWorld) {
    const o = grab.startMouseWorld;
    const d = 1000;
    let p1, p2;
    if (lock === 'X') { p1 = w2s(o.x-d, o.y,   o.z  ); p2 = w2s(o.x+d, o.y,   o.z  ); }
    if (lock === 'Y') { p1 = w2s(o.x,   o.y-d, o.z  ); p2 = w2s(o.x,   o.y+d, o.z  ); }
    if (lock === 'Z') { p1 = w2s(o.x,   o.y,   o.z-d); p2 = w2s(o.x,   o.y,   o.z+d); }
    if (p1 && p2) {
      ctx.save();
      ctx.setLineDash([5, 5]);
      ctx.strokeStyle  = lockColor;
      ctx.globalAlpha  = 0.45;
      ctx.lineWidth    = 1.0;
      ctx.beginPath(); ctx.moveTo(p1.x, p1.y); ctx.lineTo(p2.x, p2.y); ctx.stroke();
      ctx.restore();
    }
  }

  // ── Delta readout near cursor (only while grabbing) ──
  if (isGrabbing && grab.pointIds && grab.pointIds.length > 0) {
    const sId   = grab.pointIds[0];
    const sOrig = grab.originalPoints && grab.originalPoints.get(sId);
    const sNow  = byId.get(sId);
    if (sOrig && sNow) {
      const ddx  = (sNow.x - sOrig.x).toFixed(2);
      const ddy  = (sNow.y - sOrig.y).toFixed(2);
      const ddz  = (sNow.z - sOrig.z).toFixed(2);
      const dist = Math.hypot(sNow.x-sOrig.x, sNow.y-sOrig.y, sNow.z-sOrig.z).toFixed(2);
      const label = (lock ? lock+' ' : '') + '|Δ|'+dist + '  Δx'+ddx+' Δy'+ddy+' Δz'+ddz;
      const sx = (sketchState.hoverWorld && sketchState.hoverWorld.screenX) || origin.x;
      const sy = (sketchState.hoverWorld && sketchState.hoverWorld.screenY)
               ? sketchState.hoverWorld.screenY - 24 : origin.y - 24;
      ctx.save();
      ctx.font = '11px "JetBrains Mono", monospace';
      const tw = ctx.measureText(label).width + 14;
      ctx.fillStyle = 'rgba(10,15,30,0.88)';
      ctx.beginPath();
      ctx.roundRect(sx - tw/2, sy - 11, tw, 20, 4);
      ctx.fill();
      ctx.fillStyle    = lockColor;
      ctx.textAlign    = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText(label, sx, sy);
      ctx.restore();
    }
  }

  // ── Top banner (only while grabbing) ──
  if (isGrabbing) {
  const txt = '⤢ GRAB ' + (grab.pointIds ? grab.pointIds.length : n)
    + (lock ? ' · ' + lock : ' · free')
    + '   X/Y/Z · XY/YZ/XZ · Enter ✓ · Esc ✗';
  ctx.save();
  ctx.font = '11.5px "JetBrains Mono", monospace';
  const bw = ctx.measureText(txt).width + 20;
  ctx.fillStyle = 'rgba(10,15,30,0.9)';
  ctx.beginPath();
  ctx.roundRect(sk.width/2 - bw/2, 12, bw, 26, 6);
  ctx.fill();
  ctx.fillStyle    = lockColor;
  ctx.textAlign    = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(txt, sk.width/2, 25);
  ctx.restore();
  } // end if isGrabbing banner
}

window.__drawGrabGizmo = drawGrabGizmo;
"#;
