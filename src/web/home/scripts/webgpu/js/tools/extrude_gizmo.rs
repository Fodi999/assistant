// ── Extrude Gizmo ────────────────────────────────────────────────────────────

pub const JS: &str = r##"
  // ── __drawExtrudeGizmo ────────────────────────────────────────────────────
  // Called every frame from render_loop when extrude is active.
  // Draws an orange arrow from the edge midpoint in the extrude direction.
  // Stores window.__extrudeGizmoHandle for hit-testing.
  window.__drawExtrudeGizmo = function(ctx, sketchState, w2s, sk) {
    const ex = sketchState.extrude;
    if (!ex || !ex.active || !ex.edgeIds || !ex.edgeIds.length) {
      window.__extrudeGizmoHandle = null;
      return;
    }

    const pById = new Map(sketchState.points.map(function(p) { return [p.id, p]; }));
    const edge  = sketchState.edges.find(function(e) { return e.id === ex.edgeIds[0]; });
    if (!edge) return;
    const pA = pById.get(edge.a), pB = pById.get(edge.b);
    if (!pA || !pB) return;

    // Edge midpoint (world)
    const mx = (pA.x + pB.x) / 2;
    const my = (pA.y + pB.y) / 2;
    const mz = (pA.z + pB.z) / 2;

    // Extrude direction
    const plane = sketchState.workingPlane || 'XZ';
    const dir   = window.__getExtrudeDir ? window.__getExtrudeDir(plane) : { x:0, y:1, z:0 };

    // Current height in metres
    const inp      = document.getElementById('__extrude-modal-input');
    const heightMm = parseFloat(inp ? inp.value : (ex.heightInput || '0')) || 0;
    const heightM  = heightMm / 1000;

    // Project origin
    const origin = w2s(mx, my, mz);
    if (!origin) return;

    // Tip screen position
    var tipX, tipY, previewMode;
    var tipW = w2s(mx + dir.x * heightM, my + dir.y * heightM, mz + dir.z * heightM);

    if (!tipW || heightM < 0.0001) {
      // No height yet — show a fixed 80px preview arrow
      var tipUnit = w2s(mx + dir.x * 0.5, my + dir.y * 0.5, mz + dir.z * 0.5);
      if (!tipUnit) return;
      var ddx = tipUnit.x - origin.x, ddy = tipUnit.y - origin.y;
      var len = Math.sqrt(ddx*ddx + ddy*ddy) || 1;
      tipX = origin.x + ddx / len * 80;
      tipY = origin.y + ddy / len * 80;
      previewMode = true;
    } else {
      tipX = tipW.x; tipY = tipW.y;
      previewMode = false;
    }

    var hover = window.__extrudeGizmoHover || false;
    var drag  = window.__extrudeGizmoDrag  || false;
    var adx   = tipX - origin.x, ady = tipY - origin.y;
    var armLen = Math.sqrt(adx*adx + ady*ady) || 1;
    var ux = adx / armLen, uy = ady / armLen;

    var AH = 18, AW = 8;
    var color = drag ? '#ffe066' : hover ? '#ffcc44' : previewMode ? 'rgba(255,170,40,0.6)' : '#ffaa28';

    ctx.save();
    ctx.lineCap = 'round'; ctx.lineJoin = 'round';

    // Shaft
    ctx.beginPath();
    ctx.moveTo(origin.x, origin.y);
    ctx.lineTo(tipX - ux * AH, tipY - uy * AH);
    ctx.strokeStyle = color;
    ctx.lineWidth   = (hover || drag) ? 3 : 2;
    ctx.setLineDash(previewMode ? [5,4] : []);
    ctx.stroke();

    // Arrowhead
    var px = -uy, py = ux;
    ctx.beginPath();
    ctx.moveTo(tipX, tipY);
    ctx.lineTo(tipX - ux*AH + px*AW/2, tipY - uy*AH + py*AW/2);
    ctx.lineTo(tipX - ux*AH - px*AW/2, tipY - uy*AH - py*AW/2);
    ctx.closePath();
    ctx.fillStyle = color;
    ctx.setLineDash([]);
    ctx.fill();

    // Label
    if (!previewMode && heightMm > 0) {
      ctx.font = 'bold 12px monospace';
      ctx.fillStyle = '#ffe066';
      ctx.textAlign = 'left';
      ctx.textBaseline = 'middle';
      ctx.fillText(heightMm.toFixed(0) + ' мм', tipX + ux*10 + 6, tipY + uy*10);
    }
    if (previewMode) {
      ctx.font = '10px system-ui, sans-serif';
      ctx.fillStyle = 'rgba(255,170,40,0.75)';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'top';
      ctx.fillText('↕ drag', origin.x, origin.y + 8);
    }
    ctx.restore();

    // Store handle for hit-testing
    window.__extrudeGizmoHandle = { ox: origin.x, oy: origin.y, tx: tipX, ty: tipY, ux: ux, uy: uy, armLen: armLen };
  };

  // ── Pointer listeners ─────────────────────────────────────────────────────
  // Registered on document (not canvas) so we never miss events.
  // Guards: window.__extrudeGizmoInited prevents double-registration.
  (function() {
    if (window.__extrudeGizmoInited) return;
    window.__extrudeGizmoInited = true;

    var _dragging = false;
    var _startPx  = 0;
    var _startMm  = 0;
    var PX_PER_MM = 0.5; // 2px drag = 1mm

    function _getCanvas() {
      return document.getElementById('webgpu-canvas');
    }

    function _hitTest(clientX, clientY) {
      var h = window.__extrudeGizmoHandle;
      if (!h) return false;
      var canvas = _getCanvas();
      if (!canvas) return false;
      var rect = canvas.getBoundingClientRect();
      var dpr  = window.devicePixelRatio || 1;
      var cx   = (clientX - rect.left) * dpr;
      var cy   = (clientY - rect.top)  * dpr;
      // Hit near tip
      var dtx = cx - h.tx, dty = cy - h.ty;
      if (dtx*dtx + dty*dty < 28*28) return true;
      // Hit along shaft
      var dox = cx - h.ox, doy = cy - h.oy;
      var proj = dox * h.ux + doy * h.uy;
      if (proj < 0 || proj > h.armLen) return false;
      return Math.abs(-doy * h.ux + dox * h.uy) < 12;
    }

    // Public — called by mouse.rs before the extrude early-return block
    window.__extrudeGizmoPointerDown = function(e) {
      if (!window.sketchState || !window.sketchState.extrude || !window.sketchState.extrude.active) return false;
      if (!_hitTest(e.clientX, e.clientY)) return false;
      e.preventDefault();
      e.stopPropagation();
      _dragging = true;
      window.__extrudeGizmoDrag = true;
      var h = window.__extrudeGizmoHandle;
      var canvas = _getCanvas();
      var rect = canvas.getBoundingClientRect();
      var dpr  = window.devicePixelRatio || 1;
      var cx   = (e.clientX - rect.left) * dpr;
      var cy   = (e.clientY - rect.top)  * dpr;
      _startPx = cx * h.ux + cy * h.uy;
      var inp  = document.getElementById('__extrude-modal-input');
      _startMm = parseFloat(inp ? inp.value : '0') || 0;
      if (canvas.setPointerCapture) canvas.setPointerCapture(e.pointerId);
      return true;
    };

    document.addEventListener('pointermove', function(e) {
      var sk = window.sketchState;
      if (!sk || !sk.extrude || !sk.extrude.active) {
        window.__extrudeGizmoHover = false;
        return;
      }
      if (_dragging) {
        var h = window.__extrudeGizmoHandle;
        if (!h) return;
        var canvas = _getCanvas();
        if (!canvas) return;
        var rect = canvas.getBoundingClientRect();
        var dpr  = window.devicePixelRatio || 1;
        var cx   = (e.clientX - rect.left) * dpr;
        var cy   = (e.clientY - rect.top)  * dpr;
        var curPx   = cx * h.ux + cy * h.uy;
        var deltaPx = curPx - _startPx;
        var newMm   = Math.max(1, Math.round(_startMm + deltaPx / PX_PER_MM));
        var inp = document.getElementById('__extrude-modal-input');
        if (inp) { inp.value = newMm; }
        sk.extrude.heightInput = String(newMm);
        if (window.__extrudeModalShow) window.__extrudeModalShow(String(newMm));
        if (window.__setStatusMessage) {
          window.__setStatusMessage('Extrude · ' + sk.extrude.edgeIds.length + ' рёбер · ' + newMm + ' мм · Enter ✓ · Esc ✗');
        }
        return;
      }
      var hit = _hitTest(e.clientX, e.clientY);
      if (hit !== window.__extrudeGizmoHover) {
        window.__extrudeGizmoHover = hit;
        var canvas = _getCanvas();
        if (canvas) canvas.style.cursor = hit ? 'ns-resize' : '';
      }
    }, false);

    document.addEventListener('pointerup', function() {
      if (!_dragging) return;
      _dragging = false;
      window.__extrudeGizmoDrag  = false;
      window.__extrudeGizmoHover = false;
      var canvas = _getCanvas();
      if (canvas) canvas.style.cursor = '';
    }, false);

  })();
"##;
