// ── Grab Tool + Grab Gizmo ────────────────────────────────────────────────────
// Handles:
//   __startGrab              — G key / toolbar grab start
//   __updateGrab             — raycast fallback (legacy, skipped when useScreenProjection=true)
//   __confirmGrab            — Enter / click confirm
//   __cancelGrab             — Esc cancel + restore points
//   __collectSelectedPointIdsForGizmo — collect moveable point ids
//   __startGrabFromGizmo     — start grab when clicking a gizmo handle
//   drawGrabGizmo / __drawGrabGizmo   — draw Plasticity-style axis gizmo on canvas

pub const JS: &str = r##"
      // ─────────────────────────────────────────────────────────
      // __startGrab() — G key / toolbar path
      // Collects selected points (+ edge endpoints + profile points).
      // Uses screen-projection path (same as gizmo drag).
      // ─────────────────────────────────────────────────────────
      // ─────────────────────────────────────────────────────────
      // window.__raycastSketchPlaneAt(ndcX, ndcY, center)
      // Like pick.rs __raycastSketchPlane but anchors the plane at `center`
      // (not at origin). Exposed on window so outer-scope code can call it.
      //   XZ  → plane y = center.y  (normal 0,1,0)
      //   XY  → plane z = center.z  (normal 0,0,1)
      //   YZ  → plane x = center.x  (normal 1,0,0)
      // Falls back to view-aligned plane when ray is nearly parallel.
      // Returns raw (unsnapped) {x,y,z} world point, or null.
      // ─────────────────────────────────────────────────────────
      window.__raycastSketchPlaneAt = function(ndcX, ndcY, center) {
        const r = __pickRay(ndcX, ndcY);
        const plane = sketchState.workingPlane || 'XZ';
        let nx = 0, ny = 0, nz = 0, d;
        if      (plane === 'XY') { nz = 1; d = center.z; }
        else if (plane === 'YZ') { nx = 1; d = center.x; }
        else /* XZ */            { ny = 1; d = center.y; }
        const denom = r.dx * nx + r.dy * ny + r.dz * nz;
        if (Math.abs(denom) < 1e-6) return window.__raycastDragPlaneAt(ndcX, ndcY, center);
        const t = (d - (r.ox * nx + r.oy * ny + r.oz * nz)) / denom;
        if (t < 0) return window.__raycastDragPlaneAt(ndcX, ndcY, center);
        return { x: r.ox + r.dx * t, y: r.oy + r.dy * t, z: r.oz + r.dz * t };
      };

      // view-aligned fallback — exposed on window for grab_gizmo.rs
      window.__raycastDragPlaneAt = function(ndcX, ndcY, center) {
        const r = __pickRay(ndcX, ndcY);
        const cp = Math.cos(cam.pitch), sp = Math.sin(cam.pitch);
        const cy = Math.cos(cam.yaw),   sy = Math.sin(cam.yaw);
        const nx = -sy * cp, ny = -sp, nz = cy * cp;
        const denom = r.dx*nx + r.dy*ny + r.dz*nz;
        if (Math.abs(denom) < 1e-6) return null;
        const t = ((center.x-r.ox)*nx + (center.y-r.oy)*ny + (center.z-r.oz)*nz) / denom;
        if (t < 0) return null;
        return { x: r.ox + r.dx*t, y: r.oy + r.dy*t, z: r.oz + r.dz*t };
      };

      window.__startGrab = function() {
        // Auto-collect from edges if no points selected
        if (!sketchState.selectedPointIds.size && sketchState.selectedEdgeIds.size > 0) {
          const eById = new Map(sketchState.edges.map(e => [e.id, e]));
          for (const eid of sketchState.selectedEdgeIds) {
            const edge = eById.get(eid);
            if (edge) {
              sketchState.selectedPointIds.add(edge.a);
              sketchState.selectedPointIds.add(edge.b);
            }
          }
        }
        // Auto-collect from profile if no points selected
        if (!sketchState.selectedPointIds.size && sketchState.selectedProfileId) {
          const prof = window.__getProfileById
            ? window.__getProfileById(sketchState.selectedProfileId)
            : null;
          if (prof && prof.pointIds) {
            for (const id of prof.pointIds) sketchState.selectedPointIds.add(id);
          }
        }

        const allIds  = [...sketchState.selectedPointIds];
        const moveIds = allIds.filter(id => !window.__isPointFixed(id));
        if (!moveIds.length) {
          window.__setStatusMessage('Нельзя двигать: все точки зафиксированы');
          return;
        }
        window.__pushHistory();
        const byId = new Map(sketchState.points.map(p => [p.id, p]));
        const snapshot = new Map();
        const dragBase = new Map();
        for (const id of moveIds) {
          const p = byId.get(id);
          if (p) {
            snapshot.set(id, { x: p.x, y: p.y, z: p.z });
            dragBase.set(id, { x: p.x, y: p.y, z: p.z });
          }
        }
        const startWorld = sketchState.hoverWorld
          ? { x: sketchState.hoverWorld.x, y: sketchState.hoverWorld.y, z: sketchState.hoverWorld.z }
          : { x: 0, y: 0, z: 0 };
        // Compute selection center (for drag plane anchor)
        let _cx=0, _cy=0, _cz=0, _cn=0;
        for (const id of moveIds) {
          const p = byId.get(id); if(!p) continue;
          _cx+=p.x; _cy+=p.y; _cz+=p.z; _cn++;
        }
        const startCenter = _cn ? { x:_cx/_cn, y:_cy/_cn, z:_cz/_cn } : { x:0, y:0, z:0 };
        // startScreen in canvas device-px, anchored to gizmo center (object center).
        // Falls back to lastMouseScreen (also canvas device-px).
        const _ctrG = window.__gizmoCenterScreen;
        const _lms  = sketchState.precision?.lastMouseScreen;
        const startScreen = _ctrG
          ? { x: _ctrG.x, y: _ctrG.y }
          : _lms
            ? { x: _lms.x, y: _lms.y }
            : { x: 0, y: 0 };
        sketchState.grab = {
          active: true,
          mode: 'command',           // G-key opens command mode — no auto-drag
          dragging: false,           // becomes true only when user clicks gizmo handle
          source: 'keyboard',
          pointIds: moveIds,
          startMouseWorld: startWorld,
          startScreen,
          startCenter,
          startDragPoint: null,
          originalPoints: snapshot,
          dragBase,
          screenAcc: { x: 0, y: 0, z: 0 },
          axisLock: null,
          dragAxis: null,
          useDragPlane: false,
          numericInput: '',
          useScreenProjection: false,
          plane: sketchState.workingPlane || 'XZ',
        };
        // Ensure gizmoDrag is OFF — G-key shows gizmo but does not start drag
        if (window.__resetGizmoDrag) window.__resetGizmoDrag();
        window.__grabIsScreenProjection = false;  // G-key does NOT auto-move on mousemove
        if (window.__resetGrabTracking) window.__resetGrabTracking();
        window.__grabLogCount = 0;  // сброс счётчика диагностики
        const skipped = allIds.length - moveIds.length;
        const planeName = sketchState.workingPlane || 'XZ';
        const msg = '⤢ Захват ' + moveIds.length + ' т'
          + (skipped ? ' (' + skipped + ' зафикс.)' : '')
          + ' · пл. ' + planeName + ' · X/Y/Z · Enter ✓ · Esc ✗';
        window.__setStatusMessage(msg);
        console.group('%c[GRAB START]', 'color:#3b82f6;font-weight:bold');
        console.log('pointIds    :', moveIds);
        console.log('plane       :', planeName);
        console.log('startCenter :', { x:+(startCenter.x*1000).toFixed(2)+'mm', y:+(startCenter.y*1000).toFixed(2)+'mm', z:+(startCenter.z*1000).toFixed(2)+'mm' });
        console.log('cam state   :', { yaw: +(cam.yaw*180/Math.PI).toFixed(1)+'°', pitch: +(cam.pitch*180/Math.PI).toFixed(1)+'°', dist: +cam.dist.toFixed(3)+'m' });
        console.log('► Нажми Y чтобы зафиксировать ось, затем двигай тачпадом');
        console.groupEnd();
        if (window.__updateSketchInspector) window.__updateSketchInspector();
      };

      // ─────────────────────────────────────────────────────────
      // __updateGrab(hoverWorld) — legacy raycast path
      // Called from mouse.rs only when useScreenProjection=false.
      // ─────────────────────────────────────────────────────────
      window.__updateGrab = function(hoverWorld) {
        const grab = sketchState.grab;
        if (!grab.active || !hoverWorld || !grab.startMouseWorld) return;
        if (!window.__isPointerDragging || !window.__isPointerDragging()) return;
        if (!grab.dragBase) return;

        let dx = hoverWorld.x - grab.startMouseWorld.x;
        let dy = hoverWorld.y - grab.startMouseWorld.y;
        let dz = hoverWorld.z - grab.startMouseWorld.z;

        if      (grab.axisLock === 'X')  { dy = 0; dz = 0; }
        else if (grab.axisLock === 'Y')  { dx = 0; dz = 0; }
        else if (grab.axisLock === 'Z')  { dx = 0; dy = 0; }
        else if (grab.axisLock === 'XY') { dz = 0; }
        else if (grab.axisLock === 'YZ') { dx = 0; }
        else if (grab.axisLock === 'XZ') { dy = 0; }

        const g = (sketchState.precision && sketchState.precision.internalStepM)
                  || sketchState.gridSize || 0.00001;
        const sdx = Math.round(dx / g) * g;
        const sdy = Math.round(dy / g) * g;
        const sdz = Math.round(dz / g) * g;
        const byId = new Map(sketchState.points.map(p => [p.id, p]));
        for (const id of grab.pointIds) {
          const base = grab.dragBase.get(id);
          const p = byId.get(id);
          if (!base || !p) continue;
          p.x  = base.x + sdx; p.y  = base.y + sdy; p.z  = base.z + sdz;
          p.gx = Math.round(p.x / g);
          p.gy = Math.round(p.y / g);
          p.gz = Math.round(p.z / g);
        }
      };

      // ─────────────────────────────────────────────────────────
      // __confirmGrab() — Enter / click confirm
      // Applies numeric input, then persists ALL moved points through
      // __movePointViaEngine (WASM-first + backend-sync).
      // ─────────────────────────────────────────────────────────
      window.__confirmGrab = async function() {
        const grab = sketchState.grab;
        const g = (sketchState.precision && sketchState.precision.internalStepM)
                  || sketchState.gridSize || 0.00001;

        // ── Apply numeric input if user typed a value (e.g. G X 120 Enter = 120 mm) ──
        const numStr = (grab.numericInput || '').trim();
        if (numStr !== '' && grab.axisLock && ['X','Y','Z'].includes(grab.axisLock)) {
          const val = parseFloat(numStr) / 1000;  // user types mm → metres
          if (isFinite(val)) {
            const byId = new Map(sketchState.points.map(p => [p.id, p]));
            for (const [id, base] of grab.dragBase.entries()) {
              const p = byId.get(id);
              if (!p) continue;
              p.x = base.x + (grab.axisLock === 'X' ? val : 0);
              p.y = base.y + (grab.axisLock === 'Y' ? val : 0);
              p.z = base.z + (grab.axisLock === 'Z' ? val : 0);
              p.gx = Math.round(p.x / g);
              p.gy = Math.round(p.y / g);
              p.gz = Math.round(p.z / g);
            }
          }
        }

        // ── Persist each moved point through WASM + backend ──
        const byId = new Map(sketchState.points.map(p => [p.id, p]));
        const moved = [];
        for (const id of grab.pointIds) {
          const p    = byId.get(id);
          const orig = grab.originalPoints.get(id);
          if (!p || !orig) continue;
          // Only sync if position actually changed
          if (p.gx === Math.round(orig.x / g) &&
              p.gy === Math.round(orig.y / g) &&
              p.gz === Math.round(orig.z / g)) continue;
          moved.push({ id, gx: p.gx, gy: p.gy, gz: p.gz });
        }

        const n = grab.pointIds.length;

        // Clear grab state BEFORE async calls so UI feels instant
        sketchState.grab = {
          active: false, pointIds: [], startMouseWorld: null,
          originalPoints: new Map(), axisLock: null, screenAcc: null, numericInput: '',
          mode: null, dragging: false, source: null,
          dragAxis: null, useDragPlane: false,
        };
        if (window.__resetGizmoDrag) window.__resetGizmoDrag();
        window.__grabIsScreenProjection = false;
        if (window.__resetGrabTracking) window.__resetGrabTracking();
        window.__notifySketchChanged();
        window.__setStatusMessage('Захват подтверждён (' + n + ' т.)');
        if (window.__updateSketchInspector) window.__updateSketchInspector();

        // Fire WASM + backend sync for each moved point (fire-and-forget after state is clear)
        if (moved.length > 0 && window.__movePointViaEngine) {
          for (const { id, gx, gy, gz } of moved) {
            try {
              await window.__movePointViaEngine(id, gx, gy, gz);
            } catch (e) {
              console.warn('[confirmGrab] movePointViaEngine failed for', id, e);
            }
          }
        }
      };

      // ─────────────────────────────────────────────────────────
      // __cancelGrab() — Esc cancel + restore original positions
      // ─────────────────────────────────────────────────────────
      window.__cancelGrab = function() {
        const grab = sketchState.grab;
        const byId = new Map(sketchState.points.map(p => [p.id, p]));
        const g = (sketchState.precision && sketchState.precision.internalStepM)
                  || sketchState.gridSize || 0.00001;
        for (const id of grab.pointIds) {
          const orig = grab.originalPoints.get(id);
          const p = byId.get(id);
          if (!orig || !p) continue;
          p.x = orig.x; p.y = orig.y; p.z = orig.z;
          p.gx = Math.round(p.x / g);
          p.gy = Math.round(p.y / g);
          p.gz = Math.round(p.z / g);
        }
        sketchState.grab = {
          active: false, pointIds: [], startMouseWorld: null,
          originalPoints: new Map(), axisLock: null, screenAcc: null, numericInput: '',
          mode: null, dragging: false, source: null,
          dragAxis: null, useDragPlane: false,
        };
        if (window.__resetGizmoDrag) window.__resetGizmoDrag();
        window.__grabIsScreenProjection = false;
        if (window.__resetGrabTracking) window.__resetGrabTracking();
        if (sketchState._history.undo.length) sketchState._history.undo.pop();
        window.__setStatusMessage('Захват отменён');
        if (window.__updateSketchInspector) window.__updateSketchInspector();
      };

      // ─────────────────────────────────────────────────────────
      // Grab Gizmo — collect ids, start from gizmo, draw
      // ─────────────────────────────────────────────────────────

  window.__collectSelectedPointIdsForGizmo = function() {
    const pts = new Set();
    const sk = sketchState;
    if (sk.grab.active) {
      for (const id of sk.grab.pointIds) pts.add(id);
      return pts;
    }
    for (const id of sk.selectedPointIds) pts.add(id);
    for (const eId of sk.selectedEdgeIds) {
      const e = sk.edges.find(e => e.id === eId);
      if (e) { pts.add(e.a); pts.add(e.b); }
    }
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

  // axis      — 'X' | 'Y' | 'Z' | 'FREE'
  // ndcX/ndcY — NDC coords of the click (range -1..1), passed by gizmo_controller.rs
  window.__startGrabFromGizmo = function(axis, ndcX, ndcY) {
    const ids = window.__collectSelectedPointIdsForGizmo();
    if (!ids.size) return;
    const moveIds = [...ids].filter(id => !window.__isPointFixed || !window.__isPointFixed(id));
    if (!moveIds.length) { window.__setStatusMessage('Нельзя двигать зафиксированные точки'); return; }
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
    const canvas = document.getElementById('webgpu-canvas');
    const dpr2 = window.devicePixelRatio || 1;

    // Compute selection center from grabbed points (drag plane anchor)
    let _scx=0, _scy=0, _scz=0, _scn=0;
    for (const id of moveIds) {
      const p = byId.get(id); if(!p) continue;
      _scx+=p.x; _scy+=p.y; _scz+=p.z; _scn++;
    }
    const startCenter = _scn
      ? { x:_scx/_scn, y:_scy/_scn, z:_scz/_scn }
      : { x:0, y:0, z:0 };

    // Compute initial drag point from NDC click position
    const startDragPoint = __raycastDragPlane(ndcX, ndcY, startCenter);

    // startScreen anchored to gizmo center (canvas device-px).
    const _ctr = window.__gizmoCenterScreen;
    const startScreen = _ctr
      ? { x: _ctr.x, y: _ctr.y }
      : { x: (ndcX + 1) * 0.5 * (canvas ? canvas.width : 1),
          y: (1 - ndcY) * 0.5 * (canvas ? canvas.height : 1) };
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
      startDragPoint,
      originalPoints: snapshot,
      axisLock: (axis === 'FREE') ? null : axis,
      dragBase,
      screenAcc: { x: 0, y: 0, z: 0 },
      numericInput: '',
    };
    window.__setStatusMessage('⤢ Захват ' + moveIds.length + ' т. — ' + (axis === 'FREE' ? 'свободно' : axis + '-ось'));
    window.__grabIsScreenProjection = false;  // gizmo drag uses world-space raycasting, not screen projection
    if (window.__resetGrabTracking) window.__resetGrabTracking();
    if (window.__updateSketchInspector) window.__updateSketchInspector();
  };

  // ─────────────────────────────────────────────────────────
  // __raycastDragPlane — alias kept for any legacy callers
  // ─────────────────────────────────────────────────────────
  function __raycastDragPlane(ndcX, ndcY, center) {
    return window.__raycastDragPlaneAt(ndcX, ndcY, center);
  }

  // ─────────────────────────────────────────────────────────
  // window.__updateGizmoDrag(ndcX, ndcY)
  // Called every pointermove while grab.active.
  //
  // Implements canonical CAD gizmo formula:
  //
  //   currentPoint = raycastDragPlane(ndcX, ndcY, center)
  //   delta = dot(currentPoint - startPoint, axisVector)
  //   offset = axisVector * delta
  //   newPosition = basePosition + offset
  //
  // All math in world-space — no screen-pixel heuristics.
  // ─────────────────────────────────────────────────────────
  window.__updateGizmoDrag = function(ndcX, ndcY) {
    // ── Strict guards: movement only allowed during a real gizmo drag ──
    if (!sketchState.gizmoDrag?.active) return;
    if (!sketchState.grab?.active) return;
    if (!sketchState.grab?.dragging) return;

    const grab = sketchState.grab;

    const center = grab.startCenter;
    if (!center) return;

    // ── Numeric input mode: apply typed value (user types mm), freeze mouse drag ──
    const numStr = (grab.numericInput || '').trim();
    if (numStr !== '' && grab.axisLock && ['X','Y','Z'].includes(grab.axisLock)) {
      const val = parseFloat(numStr) / 1000;  // user types mm → convert to metres
      if (isFinite(val)) {
        const g = (sketchState.precision && sketchState.precision.internalStepM)
                  || sketchState.gridSize || 0.00001;
        const byId = new Map(sketchState.points.map(p => [p.id, p]));
        for (const [id, base] of grab.dragBase.entries()) {
          const p = byId.get(id);
          if (!p) continue;
          p.x = base.x + (grab.axisLock === 'X' ? val : 0);
          p.y = base.y + (grab.axisLock === 'Y' ? val : 0);
          p.z = base.z + (grab.axisLock === 'Z' ? val : 0);
          p.gx = Math.round(p.x / g);
          p.gy = Math.round(p.y / g);
          p.gz = Math.round(p.z / g);
        }
      }
      return;
    }

    // ── Mouse drag: всегда используем вид-выровненную плоскость ──
    // __raycastSketchPlaneAt при pitch≈0 и center.y=0 возвращает t=0 (позицию
    // камеры) потому что луч почти параллелен плоскости XZ.
    // Решение: всегда dragPlaneAt — вид-выровненная плоскость всегда имеет
    // ненулевой denom. Маскировка осей применяется после.
    const lock  = grab.axisLock;
    const cur = window.__raycastDragPlaneAt(ndcX, ndcY, center)
             || window.__raycastSketchPlaneAt(ndcX, ndcY, center);

    // ── ДИАГНОСТИКА захвата (каждые ~30 вызовов) ──────────────────
    if (!window.__grabLogCount) window.__grabLogCount = 0;
    window.__grabLogCount++;
    if (window.__grabLogCount % 30 === 1) {
      console.group('%c[GRAB DIAG]', 'color:#22c55e;font-weight:bold');
      console.log('axisLock   :', lock);
      console.log('ndc input  :', { ndcX: +ndcX.toFixed(4), ndcY: +ndcY.toFixed(4) });
      console.log('center     :', center ? { x:+(center.x*1000).toFixed(2)+'mm', y:+(center.y*1000).toFixed(2)+'mm', z:+(center.z*1000).toFixed(2)+'mm' } : null);
      console.log('cur point  :', cur  ? { x:+(cur.x*1000).toFixed(2)+'mm', y:+(cur.y*1000).toFixed(2)+'mm', z:+(cur.z*1000).toFixed(2)+'mm' } : 'NULL!');
      if (grab.startDragPoint) {
        const sp = grab.startDragPoint;
        console.log('startDrag  :', { x:+(sp.x*1000).toFixed(2)+'mm', y:+(sp.y*1000).toFixed(2)+'mm', z:+(sp.z*1000).toFixed(2)+'mm' });
      } else {
        console.log('startDrag  : не задан (первый вызов)');
      }
      console.log('cam        :', { yaw: +(cam.yaw*180/Math.PI).toFixed(1)+'°', pitch: +(cam.pitch*180/Math.PI).toFixed(1)+'°', dist: +cam.dist.toFixed(3)+'m' });
      console.log('workingPlane:', sketchState.workingPlane || 'XZ (default)');
      console.groupEnd();
    }

    if (!cur) {
      if (window.__grabLogCount % 30 === 1) console.warn('[GRAB] __raycastDragPlaneAt вернул null — камера параллельна плоскости?');
      return;
    }

    if (!grab.startDragPoint) {
      grab.startDragPoint = { x: cur.x, y: cur.y, z: cur.z };
    }

    const start = grab.startDragPoint;

    let dx = cur.x - start.x;
    let dy = cur.y - start.y;
    let dz = cur.z - start.z;

    // ── Axis lock ──────────────────────────────────────────────────
    // Явная ось (X/Y/Z/XY/XZ/YZ): обнуляем лишние компоненты.
    // Свободный режим (null): НЕ обнуляем dy — двигаемся в плоскости
    // экрана (как Blender). Плоскость __raycastDragPlaneAt уже выровнена
    // по камере, поэтому все три компоненты могут быть ненулевыми.
    if      (lock === 'X')  { dy = 0; dz = 0; }
    else if (lock === 'Y')  { dx = 0; dz = 0; }
    else if (lock === 'Z')  { dx = 0; dy = 0; }
    else if (lock === 'XY') { dz = 0; }
    else if (lock === 'XZ') { dy = 0; }
    else if (lock === 'YZ') { dx = 0; }
    // else: свободно — всё движение сохраняем как есть

    const g  = (sketchState.precision && sketchState.precision.internalStepM)
               || sketchState.gridSize || 0.00001;
    const fx = Math.round(dx / g) * g;
    const fy = Math.round(dy / g) * g;
    const fz = Math.round(dz / g) * g;

    if (window.__grabLogCount % 30 === 1) {
      console.log('%c[GRAB DELTA]', 'color:#ffe066', {
        raw_mm: { dx: +(dx*1000).toFixed(3), dy: +(dy*1000).toFixed(3), dz: +(dz*1000).toFixed(3) },
        snapped_mm: { fx: +(fx*1000).toFixed(3), fy: +(fy*1000).toFixed(3), fz: +(fz*1000).toFixed(3) },
        gridStep_mm: +(g*1000).toFixed(4),
      });
    }

    const byId = new Map(sketchState.points.map(p => [p.id, p]));
    for (const [id, base] of grab.dragBase.entries()) {
      const p = byId.get(id);
      if (!p) continue;
      p.x = base.x + fx;
      p.y = base.y + fy;
      p.z = base.z + fz;
      p.gx = Math.round(p.x / g);
      p.gy = Math.round(p.y / g);
      p.gz = Math.round(p.z / g);
    }

    // Rebuild profiles + dimensions so overlay stays in sync during drag
    if (window.__recomputeProfiles) window.__recomputeProfiles();
  };

  // drawGrabGizmo is defined and exported in tools/grab_gizmo.rs
  // DO NOT define it here — grab_gizmo.rs is the canonical source.
  function __legacyDrawGrabGizmoUnused(ctx, sketchState, w2s, sk) {
    const isGrabbing = sketchState.grab.active;
    if (!isGrabbing) {
      window.__gizmoHandles   = null;
      window.__gizmoHoverAxis = null;
      return;
    }
    const grab  = sketchState.grab;
    const lock  = grab.axisLock || null;
    const hov   = window.__gizmoHoverAxis || null;
    const byId  = new Map(sketchState.points.map(p => [p.id, p]));
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

    const ARM = 80, SQ = 22, SQ_OF = 26;
    const HIT_A = 20, HIT_P = 20, HIT_C = 16;

    function screenDir(wx, wy, wz) {
      const far = w2s(cx + wx, cy0 + wy, cz + wz);
      if (!far) return null;
      const dx = far.x - origin.x, dy = far.y - origin.y;
      const len = Math.hypot(dx, dy) || 1;
      return { x: dx / len, y: dy / len };
    }
    const dirX = screenDir(1,0,0), dirY = screenDir(0,1,0), dirZ = screenDir(0,0,1);
    if (!dirX || !dirY || !dirZ) { window.__gizmoHandles = null; return; }

    const axes = [
      { axis: 'X', color: '#f04040', dir: dirX },
      { axis: 'Y', color: '#20d060', dir: dirY },
      { axis: 'Z', color: '#3d8fff', dir: dirZ },
    ];
    const planes = [
      { axis: 'XY', color: '#f04040', dA: dirX, dB: dirY },
      { axis: 'YZ', color: '#20d060', dA: dirY, dB: dirZ },
      { axis: 'XZ', color: '#3d8fff', dA: dirX, dB: dirZ },
    ];
    const handles = [];
    ctx.save();

    // 1. Planar squares
    const axisColors = { XY: '#f04040', YZ: '#20d060', XZ: '#3d8fff' };
    for (const pl of planes) {
      const active = lock === pl.axis || hov === pl.axis;
      const ox = origin.x + pl.dA.x * SQ_OF + pl.dB.x * SQ_OF;
      const oy = origin.y + pl.dA.y * SQ_OF + pl.dB.y * SQ_OF;
      const c0x = ox, c0y = oy;
      const c1x = ox + pl.dA.x * SQ,                c1y = oy + pl.dA.y * SQ;
      const c2x = ox + pl.dA.x * SQ + pl.dB.x * SQ, c2y = oy + pl.dA.y * SQ + pl.dB.y * SQ;
      const c3x = ox + pl.dB.x * SQ,                c3y = oy + pl.dB.y * SQ;
      ctx.beginPath();
      ctx.moveTo(c0x, c0y); ctx.lineTo(c1x, c1y);
      ctx.lineTo(c2x, c2y); ctx.lineTo(c3x, c3y);
      ctx.closePath();
      const col = axisColors[pl.axis];
      if (active) {
        ctx.globalAlpha = 0.55; ctx.fillStyle = col; ctx.fill();
        ctx.globalAlpha = 1.0; ctx.strokeStyle = '#ffffff'; ctx.lineWidth = 1.5; ctx.stroke();
      } else {
        ctx.globalAlpha = lock ? 0.06 : 0.18; ctx.fillStyle = col; ctx.fill();
        ctx.globalAlpha = lock ? 0.08 : 0.45; ctx.strokeStyle = col; ctx.lineWidth = 1; ctx.stroke();
      }
      ctx.globalAlpha = 1.0;
      handles.push({ axis: pl.axis, x: (c0x + c2x) / 2, y: (c0y + c2y) / 2, r: HIT_P });
    }

    // 2. Axis shafts + arrowheads
    for (const a of axes) {
      const active = lock === a.axis || hov === a.axis;
      const tx = origin.x + a.dir.x * ARM, ty = origin.y + a.dir.y * ARM;
      ctx.globalAlpha = lock && lock !== a.axis ? 0.18 : (active ? 1.0 : 0.88);
      ctx.strokeStyle = a.color; ctx.lineWidth = active ? 2.5 : 1.8; ctx.lineCap = 'round';
      ctx.beginPath();
      ctx.moveTo(origin.x, origin.y);
      ctx.lineTo(tx - a.dir.x * 14, ty - a.dir.y * 14);
      ctx.stroke();
      const hw = 4.5, hl = 14;
      const angle = Math.atan2(a.dir.y, a.dir.x);
      ctx.fillStyle = a.color;
      ctx.beginPath();
      ctx.moveTo(tx + a.dir.x*2, ty + a.dir.y*2);
      ctx.lineTo(tx - hl*Math.cos(angle) + hw*Math.sin(angle), ty - hl*Math.sin(angle) - hw*Math.cos(angle));
      ctx.lineTo(tx - hl*Math.cos(angle) - hw*Math.sin(angle), ty - hl*Math.sin(angle) + hw*Math.cos(angle));
      ctx.closePath(); ctx.fill();
      handles.push({ axis: a.axis, x: tx, y: ty, r: HIT_A });
    }

    // 3. Centre FREE circle
    const cActive = lock === 'FREE' || hov === 'FREE';
    ctx.globalAlpha = lock && lock !== 'FREE' ? 0.2 : (cActive ? 1.0 : 0.75);
    ctx.beginPath(); ctx.arc(origin.x, origin.y, 7, 0, Math.PI*2);
    ctx.strokeStyle = '#ffffff'; ctx.lineWidth = cActive ? 2.0 : 1.2; ctx.stroke();
    ctx.beginPath(); ctx.arc(origin.x, origin.y, 3.5, 0, Math.PI*2);
    ctx.fillStyle = cActive ? '#ffffff' : 'rgba(255,255,255,0.5)'; ctx.fill();
    if (cActive) {
      ctx.beginPath(); ctx.arc(origin.x, origin.y, 11, 0, Math.PI*2);
      ctx.strokeStyle = 'rgba(255,255,255,0.35)'; ctx.lineWidth = 1; ctx.stroke();
    }
    handles.push({ axis: 'FREE', x: origin.x, y: origin.y, r: HIT_C });
    ctx.globalAlpha = 1.0;
    ctx.restore();

    window.__gizmoHandles      = handles;
    window.__gizmoCenterScreen = origin;

    // Expose screen-space axis dirs for projection drag
    (function() {
      function rawVec(wx, wy, wz) {
        const f = w2s(cx + wx, cy0 + wy, cz + wz);
        if (!f) return null;
        const ddx = f.x - origin.x, ddy = f.y - origin.y;
        return { dx: ddx, dy: ddy, pxPerUnit: Math.hypot(ddx, ddy) || 1 };
      }
      window.__gizmoAxisScreenDirs = { X: rawVec(1,0,0), Y: rawVec(0,1,0), Z: rawVec(0,0,1) };
    })();

    // Dashed guide line along locked axis
    const lockColor = lock === 'X' ? '#f04040' : lock === 'Y' ? '#20d060' : lock === 'Z' ? '#3d8fff' : '#a78bfa';
    if (lock && ['X','Y','Z'].includes(lock) && grab.startMouseWorld) {
      const o = grab.startMouseWorld, d = 1000;
      let p1, p2;
      if (lock === 'X') { p1 = w2s(o.x-d,o.y,o.z);   p2 = w2s(o.x+d,o.y,o.z);   }
      if (lock === 'Y') { p1 = w2s(o.x,o.y-d,o.z);   p2 = w2s(o.x,o.y+d,o.z);   }
      if (lock === 'Z') { p1 = w2s(o.x,o.y,o.z-d);   p2 = w2s(o.x,o.y,o.z+d);   }
      if (p1 && p2) {
        ctx.save();
        ctx.setLineDash([5,5]); ctx.strokeStyle = lockColor;
        ctx.globalAlpha = 0.45; ctx.lineWidth = 1.0;
        ctx.beginPath(); ctx.moveTo(p1.x,p1.y); ctx.lineTo(p2.x,p2.y); ctx.stroke();
        ctx.restore();
      }
    }

    // Delta readout near cursor
    if (grab.pointIds && grab.pointIds.length > 0) {
      const sId = grab.pointIds[0];
      const sOrig = grab.originalPoints && grab.originalPoints.get(sId);
      const sNow  = byId.get(sId);
      if (sOrig && sNow) {
        const ddx  = (sNow.x - sOrig.x).toFixed(2);
        const ddy  = (sNow.y - sOrig.y).toFixed(2);
        const ddz  = (sNow.z - sOrig.z).toFixed(2);
        const dist = Math.hypot(sNow.x-sOrig.x, sNow.y-sOrig.y, sNow.z-sOrig.z).toFixed(2);
        const numStr = (grab.numericInput || '').trim();
        const numPart = numStr ? ' ⌨ ' + (lock || '?') + ' ' + numStr + '▌' : '';
        const label = (lock ? lock+' ' : '') + '|Δ|'+dist + '  Δx'+ddx+' Δy'+ddy+' Δz'+ddz + numPart;
        const sx = (sketchState.hoverWorld && sketchState.hoverWorld.screenX) || origin.x;
        const sy = (sketchState.hoverWorld && sketchState.hoverWorld.screenY)
                 ? sketchState.hoverWorld.screenY - 24 : origin.y - 24;
        ctx.save();
        ctx.font = '11px "JetBrains Mono", monospace';
        const tw = ctx.measureText(label).width + 14;
        ctx.fillStyle = 'rgba(10,15,30,0.88)';
        ctx.beginPath(); ctx.roundRect(sx - tw/2, sy - 11, tw, 20, 4); ctx.fill();
        ctx.fillStyle = lockColor; ctx.textAlign = 'center'; ctx.textBaseline = 'middle';
        ctx.fillText(label, sx, sy);
        ctx.restore();
      }
    }

    // Top banner
    const planeName = sketchState.workingPlane || 'XZ';
    const numStr2 = (grab.numericInput || '').trim();
    const numDisp = numStr2 ? '  ⌨ ' + (lock || '?') + ' ' + numStr2 + '▌' : '';
    const gStep = ((sketchState.gridSize || 0.001) * 1000).toFixed(1);
    const txt = '⤢ ЗАХВАТ ' + (grab.pointIds ? grab.pointIds.length : n)
      + ' · пл.' + planeName
      + (lock ? ' · ось ' + lock : ' · свободно')
      + numDisp
      + '   X/Y/Z · Enter ✓ · Esc ✗  |  шаг ' + gStep + ' мм';
    ctx.save();
    ctx.font = '11.5px "JetBrains Mono", monospace';
    const bw = ctx.measureText(txt).width + 20;
    ctx.fillStyle = 'rgba(10,15,30,0.9)';
    ctx.beginPath(); ctx.roundRect(sk.width/2 - bw/2, 12, bw, 26, 6); ctx.fill();
    ctx.fillStyle = lockColor; ctx.textAlign = 'center'; ctx.textBaseline = 'middle';
    ctx.fillText(txt, sk.width/2, 25);
    ctx.restore();
  }

  // window.__drawGrabGizmo is assigned in tools/grab_gizmo.rs (loaded after this file)
"##;
