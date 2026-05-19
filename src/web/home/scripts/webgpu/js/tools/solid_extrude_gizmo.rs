// ── Solid Extrude Gizmo (Plasticity-style) ─────────────────────────────────
//
//  Workflow (как в Plasticity):
//    1. Нарисовал замкнутый профиль (R → прямоугольник)
//    2. Нажал E или кнопку ↑Solid — активируется этот гизмо
//    3. Синяя стрелка появляется на профиле, перпендикулярно плоскости
//    4. Тянешь стрелку мышью → solid растёт, превью обновляется (~300мс debounce)
//    5. Enter / двойной клик → финальный POST на truck → WebGL превью
//    6. Esc → отмена
//
//  Публичный API:
//    window.__startSolidExtrude(profileId?)  — запустить гизмо
//    window.__cancelSolidExtrude()           — отменить
//    window.__commitSolidExtrude()           — подтвердить (Enter)
//    window.__drawSolidExtrudeGizmo(ctx, w2s) — вызывается из render_loop

pub const JS: &str = r##"
(function registerSolidExtrudeGizmo() {

  // ─────────────────────────────────────────────────────────────────────────
  // Состояние
  // ─────────────────────────────────────────────────────────────────────────
  var _state = {
    active:      false,
    profileId:   null,
    profile:     null,       // профиль-объект
    depthMm:     0,          // текущая глубина в мм
    plane:       'XZ',
    dir:         { x:0, y:1, z:0 },
    centroid:    { x:0, y:0, z:0 }, // центр профиля в world space
    dragging:    false,
    startPx:     0,
    startDepthMm:0,
    hover:       false,
    handle:      null,       // { ox,oy,tx,ty,ux,uy,armLen } — screen
    previewMesh: null,       // последний результат truck
    _debounce:   null,
  };
  window.__solidExtrudeState = _state;

  // ─────────────────────────────────────────────────────────────────────────
  // Helpers
  // ─────────────────────────────────────────────────────────────────────────

  function _getProfile(profileId) {
    const ss = window.sketchState;
    if (!ss) return null;
    if (window.__recomputeProfiles) window.__recomputeProfiles();
    if (profileId) return (ss.profiles || []).find(p => p.id === profileId) || null;
    if (ss.selectedProfileId) {
      const s = (ss.profiles || []).find(p => p.id === ss.selectedProfileId);
      if (s) return s;
    }
    // Профиль содержащий выделенные рёбра
    const selEdges = [...(ss.selectedEdgeIds || [])];
    if (selEdges.length) {
      const f = (ss.profiles || []).find(p => selEdges.some(eid => p.edgeIds.includes(eid)));
      if (f) return f;
    }
    return (ss.profiles && ss.profiles.length) ? ss.profiles[0] : null;
  }

  function _centroid(profile) {
    const ss = window.sketchState;
    if (!ss || !profile) return { x:0, y:0, z:0 };
    const byId = new Map((ss.points||[]).map(p=>[p.id,p]));
    const pts = (profile.pointIds||[]).map(id=>byId.get(id)).filter(Boolean);
    if (!pts.length) return { x:0, y:0, z:0 };
    const s = pts.reduce((a,p)=>({ x:a.x+p.x, y:a.y+p.y, z:a.z+p.z }),{x:0,y:0,z:0});
    return { x:s.x/pts.length, y:s.y/pts.length, z:s.z/pts.length };
  }

  function _planeDir(plane) {
    if (plane === 'XY') return { x:0, y:0, z:1 };
    if (plane === 'YZ') return { x:1, y:0, z:0 };
    return { x:0, y:1, z:0 }; // XZ
  }

  function _profilePoints(profile) {
    const ss = window.sketchState;
    if (!ss || !profile) return null;
    const byId = new Map((ss.points||[]).map(p=>[p.id,p]));
    const pts = (profile.pointIds||[]).map(id=>byId.get(id)).filter(Boolean);
    if (pts.length < 3) return null;
    return pts.map(p=>({ x:p.x, y:p.y, z:p.z }));
  }

  // ─────────────────────────────────────────────────────────────────────────
  // Публичный API — старт / стоп / коммит
  // ─────────────────────────────────────────────────────────────────────────

  window.__startSolidExtrude = function(profileId) {
    const prof = _getProfile(profileId);
    if (!prof) {
      if (window.__setStatusMessage)
        window.__setStatusMessage('↑Solid: нет профиля — нарисуй замкнутый контур (R)');
      return;
    }
    const ss    = window.sketchState;
    const plane = (prof.plane && prof.plane !== 'unknown') ? prof.plane : (ss ? (ss.workingPlane||'XZ') : 'XZ');
    const dir   = _planeDir(plane);
    const cen   = _centroid(prof);

    _state.active       = true;
    _state.profileId    = prof.id;
    _state.profile      = prof;
    _state.depthMm      = 100;
    _state.plane        = plane;
    _state.dir          = dir;
    _state.centroid     = cen;
    _state.dragging     = false;
    _state.hover        = false;
    _state.handle       = null;
    _state.previewMesh  = null;
    if (_state._debounce) { clearTimeout(_state._debounce); _state._debounce = null; }

    // Закрываем другие модалы
    var dimEl = document.getElementById('__dim-editor');
    if (dimEl) dimEl.style.display = 'none';
    var bridgeEl = document.getElementById('__se_bridge_modal');
    if (bridgeEl) bridgeEl.style.display = 'none';

    _showDepthHud(100);
    if (window.__setStatusMessage)
      window.__setStatusMessage('↑Solid: тяни стрелку · Enter ✓ · Esc ✗ · или вводи мм');

    console.log('[SolidExtrude] started, profile=', prof.id, 'plane=', plane, 'centroid=', cen);
  };

  window.__cancelSolidExtrude = function() {
    if (!_state.active) return;
    _state.active = false;
    if (_state._debounce) { clearTimeout(_state._debounce); _state._debounce = null; }
    _hideDepthHud();
    if (window.__closeSolidPreview) window.__closeSolidPreview();
    if (window.__setStatusMessage) window.__setStatusMessage('↑Solid: отменено');
  };

  window.__commitSolidExtrude = async function() {
    if (!_state.active) return;
    const depthMm = _state.depthMm;
    _state.active = false;
    if (_state._debounce) { clearTimeout(_state._debounce); _state._debounce = null; }
    _hideDepthHud();
    await _sendToTruck(depthMm, true /* final */);
  };

  // ─────────────────────────────────────────────────────────────────────────
  // Depth HUD — маленький инпут над стрелкой
  // ─────────────────────────────────────────────────────────────────────────

  function _getHud() {
    var el = document.getElementById('__solid_extrude_hud');
    if (el) return el;

    var T = window.__modalTheme;
    var C = T ? T.COLORS : { panel:'#1e293b', border:'rgba(56,189,248,.3)', input:'#f1f5f9', mute:'#64748b' };
    var L = T ? T.LAYOUT : { font:"'JetBrains Mono',monospace", borderRadius:'8px' };

    el = document.createElement('div');
    el.id = '__solid_extrude_hud';
    Object.assign(el.style, {
      display:      'none',
      position:     'fixed',
      zIndex:       '10020',
      background:   C.panel,
      border:       '1px solid ' + C.border,
      borderRadius: L.borderRadius,
      padding:      '6px 10px 8px',
      fontFamily:   L.font,
      fontSize:     '11px',
      color:        C.mute,
      userSelect:   'none',
      pointerEvents:'auto',
      boxShadow:    '0 4px 16px rgba(0,0,0,.5)',
      minWidth:     '180px',
    });

    el.innerHTML =
      '<div style="margin-bottom:4px;font-size:10px;letter-spacing:.6px;text-transform:uppercase;color:rgba(56,189,248,.7);">' +
        '↑ Solid Extrude' +
      '</div>' +
      '<div style="display:flex;align-items:center;gap:6px;">' +
        '<input id="__se_depth_hud_inp" type="number" min="1" step="1" value="100" ' +
          'style="width:80px;text-align:right;font-size:18px;font-weight:700;' +
          'background:rgba(255,255,255,.07);border:1px solid rgba(56,189,248,.3);' +
          'border-radius:5px;padding:3px 6px;color:#38bdf8;outline:none;' +
          'font-family:inherit;-moz-appearance:textfield;" />' +
        '<span style="color:#94a3b8;">мм</span>' +
      '</div>' +
      '<div style="margin-top:5px;font-size:10px;color:#475569;">' +
        'Enter ✓ &nbsp; Esc ✗ &nbsp; тяни стрелку ↕' +
      '</div>';

    document.body.appendChild(el);

    var inp = el.querySelector('#__se_depth_hud_inp');
    inp.addEventListener('input', function() {
      var v = parseFloat(inp.value);
      if (isFinite(v) && v > 0) {
        _state.depthMm = v;
        _schedulePreview(v);
      }
    });
    inp.addEventListener('keydown', function(e) {
      e.stopPropagation();
      if (e.key === 'Enter') { e.preventDefault(); window.__commitSolidExtrude(); }
      if (e.key === 'Escape') { e.preventDefault(); window.__cancelSolidExtrude(); }
    });

    return el;
  }

  function _showDepthHud(depthMm) {
    var hud = _getHud();
    hud.style.display = 'block';
    // Позиция: снизу по центру над тулбаром
    hud.style.left   = '50%';
    hud.style.bottom = '72px';
    hud.style.top    = 'auto';
    hud.style.transform = 'translateX(-50%)';
    var inp = document.getElementById('__se_depth_hud_inp');
    if (inp) { inp.value = depthMm; setTimeout(function(){ inp.focus(); inp.select(); }, 60); }
  }

  function _hideDepthHud() {
    var hud = document.getElementById('__solid_extrude_hud');
    if (hud) hud.style.display = 'none';
  }

  function _setHudDepth(depthMm) {
    var inp = document.getElementById('__se_depth_hud_inp');
    if (inp && document.activeElement !== inp) inp.value = Math.round(depthMm);
  }

  // ─────────────────────────────────────────────────────────────────────────
  // truck запрос (с debounce для превью)
  // ─────────────────────────────────────────────────────────────────────────

  function _schedulePreview(depthMm) {
    if (_state._debounce) clearTimeout(_state._debounce);
    _state._debounce = setTimeout(function() {
      _state._debounce = null;
      _sendToTruck(depthMm, false);
    }, 280);
  }

  async function _sendToTruck(depthMm, isFinal) {
    if (!_state.profile && _state.active) _state.profile = _getProfile(_state.profileId);
    const prof = _state.profile || _getProfile(null);
    if (!prof) return;

    const pts = _profilePoints(prof);
    if (!pts || pts.length < 3) {
      if (window.__setStatusMessage) window.__setStatusMessage('↑Solid: нет точек профиля');
      return;
    }

    const depthM = depthMm / 1000.0;
    const plane  = _state.plane || 'XZ';

    if (window.__setStatusMessage)
      window.__setStatusMessage('⏳ truck: строю solid ' + depthMm.toFixed(0) + ' мм…');

    var body = { plane: plane, depth: depthM, profile: pts, tolerance: isFinal ? 0.003 : 0.008 };
    console.log('[SolidExtrude] →truck', { depthMm, plane, pts: pts.length, isFinal });

    try {
      var t0   = performance.now();
      var resp = await fetch('/api/matter/sketch/extrude', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });
      var dt = (performance.now() - t0).toFixed(0);
      if (!resp.ok) {
        var err = await resp.json().catch(function(){ return {}; });
        throw new Error(err.error || resp.statusText);
      }
      var result = await resp.json();
      result.__dt = dt;
      _state.previewMesh = result;
      window.__lastSolidResult = result;

      if (window.__setStatusMessage)
        window.__setStatusMessage(
          (isFinal ? '✅' : '👁') + ' Solid: ' +
          result.vertex_count + ' вершин · ' + result.triangle_count + ' треуг. · ' + dt + ' мс'
        );

      // Показываем / обновляем WebGL превью
      _showInlinePreview(result, depthMm, plane, isFinal);

    } catch(e) {
      console.error('[SolidExtrude] truck error:', e);
      if (window.__setStatusMessage) window.__setStatusMessage('✗ truck: ' + e.message);
    }
  }

  // ─────────────────────────────────────────────────────────────────────────
  // Inline WebGL превью на канвасе
  // ─────────────────────────────────────────────────────────────────────────

  var _rafId = null, _glCtx = null, _rotY = 0;

  function _showInlinePreview(result, depthMm, plane, isFinal) {
    // Открываем стандартный solid preview из bridge
    if (window.__closeSolidPreview) window.__closeSolidPreview();

    // Создаём/обновляем превью-панель из bridge
    var panelFn = window.__showSolidPreviewPanel;
    if (panelFn) {
      panelFn(result, depthMm, plane);
      return;
    }

    // Fallback — мини-канвас
    _renderMiniPreview(result);
  }

  function _renderMiniPreview(result) {
    var mini = document.getElementById('__se_mini_canvas');
    if (!mini) {
      mini = document.createElement('canvas');
      mini.id = '__se_mini_canvas';
      mini.width = 200; mini.height = 160;
      Object.assign(mini.style, {
        position: 'fixed', right: '24px', top: '80px',
        zIndex: '10015', borderRadius: '8px',
        border: '1px solid rgba(56,189,248,.25)',
        background: '#0f172a', display: 'none',
        boxShadow: '0 4px 16px rgba(0,0,0,.5)',
      });
      document.body.appendChild(mini);
    }
    mini.style.display = 'block';

    if (_rafId) { cancelAnimationFrame(_rafId); _rafId = null; }

    var gl = mini.getContext('webgl2') || mini.getContext('webgl');
    if (!gl) return;
    _glCtx = gl;

    var vs = 'attribute vec3 aP;attribute vec3 aN;uniform mat4 uM;uniform mat3 uN;varying vec3 vN;' +
             'void main(){vN=normalize(uN*aN);gl_Position=uM*vec4(aP,1.);}';
    var fs = 'precision mediump float;varying vec3 vN;' +
             'void main(){float d=max(dot(normalize(vN),normalize(vec3(.6,1.,.8))),.0);' +
             'gl_FragColor=vec4(vec3(.1,.5,.9)*(0.25+d*.75),1.);}';

    function sh(src, t) {
      var s = gl.createShader(t); gl.shaderSource(s, src); gl.compileShader(s); return s;
    }
    var prog = gl.createProgram();
    gl.attachShader(prog, sh(vs, gl.VERTEX_SHADER));
    gl.attachShader(prog, sh(fs, gl.FRAGMENT_SHADER));
    gl.linkProgram(prog); gl.useProgram(prog);

    var pb = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, pb);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(result.positions), gl.STATIC_DRAW);
    var nb = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, nb);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(result.normals), gl.STATIC_DRAW);
    var ib = gl.createBuffer(); gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, ib);
    gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, new Uint32Array(result.indices), gl.STATIC_DRAW);

    var aP = gl.getAttribLocation(prog,'aP'), aN = gl.getAttribLocation(prog,'aN');
    var uM = gl.getUniformLocation(prog,'uM'), uN = gl.getUniformLocation(prog,'uN');

    var pos = result.positions, n = pos.length;
    var mnX=1e9,mnY=1e9,mnZ=1e9,mxX=-1e9,mxY=-1e9,mxZ=-1e9;
    for (var i=0;i<n;i+=3){mnX=Math.min(mnX,pos[i]);mxX=Math.max(mxX,pos[i]);mnY=Math.min(mnY,pos[i+1]);mxY=Math.max(mxY,pos[i+1]);mnZ=Math.min(mnZ,pos[i+2]);mxZ=Math.max(mxZ,pos[i+2]);}
    var cx=(mnX+mxX)/2,cy=(mnY+mxY)/2,cz=(mnZ+mxZ)/2,sz=Math.max(mxX-mnX,mxY-mnY,mxZ-mnZ)||1,sc=1.6/sz;

    function mm(a,b){var o=new Float32Array(16);for(var c=0;c<4;c++)for(var r=0;r<4;r++){var v=0;for(var k=0;k<4;k++)v+=a[k*4+r]*b[c*4+k];o[c*4+r]=v;}return o;}
    function ry(t){var c=Math.cos(t),s=Math.sin(t);return new Float32Array([c,0,-s,0,0,1,0,0,s,0,c,0,0,0,0,1]);}
    function rx(t){var c=Math.cos(t),s=Math.sin(t);return new Float32Array([1,0,0,0,0,c,s,0,0,-s,c,0,0,0,0,1]);}
    function pr(f,a,near,far){var v=1/Math.tan(f/2);return new Float32Array([v/a,0,0,0,0,v,0,0,0,0,(far+near)/(near-far),-1,0,0,2*far*near/(near-far),0]);}
    function tr(x,y,z){return new Float32Array([1,0,0,0,0,1,0,0,0,0,1,0,x,y,z,1]);}
    function sc4(s){return new Float32Array([s,0,0,0,0,s,0,0,0,0,s,0,0,0,0,1]);}

    var W=mini.width, H=mini.height;
    function draw() {
      gl.viewport(0,0,W,H);
      gl.clearColor(0.059,0.090,0.165,1); gl.clear(gl.COLOR_BUFFER_BIT|gl.DEPTH_BUFFER_BIT);
      gl.enable(gl.DEPTH_TEST); gl.enable(gl.CULL_FACE);

      var model=mm(mm(tr(-cx,-cy,-cz),sc4(sc)),mm(rx(0.3),ry(_rotY)));
      var view=tr(0,0,-2.8);
      var proj=pr(0.9,W/H,0.01,100);
      var mvp=mm(mm(model,view),proj);
      gl.uniformMatrix4fv(uM,false,mvp);
      var nm=new Float32Array([model[0],model[1],model[2],model[4],model[5],model[6],model[8],model[9],model[10]]);
      gl.uniformMatrix3fv(uN,false,nm);

      gl.bindBuffer(gl.ARRAY_BUFFER,pb); gl.enableVertexAttribArray(aP); gl.vertexAttribPointer(aP,3,gl.FLOAT,false,0,0);
      gl.bindBuffer(gl.ARRAY_BUFFER,nb); gl.enableVertexAttribArray(aN); gl.vertexAttribPointer(aN,3,gl.FLOAT,false,0,0);
      gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER,ib);
      gl.drawElements(gl.TRIANGLES,result.indices.length,gl.UNSIGNED_INT,0);
      _rotY+=0.014; _rafId=requestAnimationFrame(draw);
    }
    draw();
  }

  // ─────────────────────────────────────────────────────────────────────────
  // Рисование стрелки гизмо на sketch-canvas
  // ─────────────────────────────────────────────────────────────────────────

  window.__drawSolidExtrudeGizmo = function(ctx, w2s) {
    if (!_state.active) { _state.handle = null; return; }

    var cen = _state.centroid;
    var dir = _state.dir;
    var dep = _state.depthMm / 1000;

    var origin = w2s(cen.x, cen.y, cen.z);
    if (!origin) return;

    var tipWorld = { x: cen.x + dir.x * dep, y: cen.y + dir.y * dep, z: cen.z + dir.z * dep };
    var tipScr   = w2s(tipWorld.x, tipWorld.y, tipWorld.z);

    var tipX, tipY, previewMode;
    if (!tipScr || dep < 0.001) {
      // Фиксированная длина 90px
      var unit = w2s(cen.x + dir.x * 0.3, cen.y + dir.y * 0.3, cen.z + dir.z * 0.3);
      if (!unit) return;
      var ddx = unit.x - origin.x, ddy = unit.y - origin.y, l = Math.hypot(ddx, ddy) || 1;
      tipX = origin.x + ddx/l * 90; tipY = origin.y + ddy/l * 90;
      previewMode = true;
    } else {
      tipX = tipScr.x; tipY = tipScr.y; previewMode = false;
    }

    var adx = tipX - origin.x, ady = tipY - origin.y;
    var armLen = Math.hypot(adx, ady) || 1;
    var ux = adx/armLen, uy = ady/armLen;

    var hover = _state.hover, drag = _state.dragging;
    // Синяя цветовая схема (как в Plasticity для face extrude)
    var shaft  = drag ? '#93c5fd' : hover ? '#60a5fa' : previewMode ? 'rgba(96,165,250,0.55)' : '#3b82f6';
    var AH = 20, AW = 9;

    ctx.save();
    ctx.lineCap = 'round'; ctx.lineJoin = 'round';

    // Жёлтая подсветка контура профиля
    if (_state.profile) {
      var ss = window.sketchState;
      var byPt = new Map((ss.points||[]).map(function(p){ return [p.id,p]; }));
      var pids = _state.profile.pointIds || [];
      if (pids.length >= 3) {
        ctx.beginPath();
        for (var i=0; i<pids.length; i++) {
          var pt = byPt.get(pids[i]);
          if (!pt) continue;
          var s = w2s(pt.x, pt.y, pt.z);
          if (!s) continue;
          if (i===0) ctx.moveTo(s.x, s.y); else ctx.lineTo(s.x, s.y);
        }
        ctx.closePath();
        ctx.strokeStyle = 'rgba(250,204,21,0.75)';
        ctx.lineWidth = 2;
        ctx.setLineDash([]);
        ctx.stroke();
      }
    }

    // Штриховая линия от центра до начала стрелки (если не в превью режиме)
    if (!previewMode && dep > 0.001) {
      ctx.beginPath();
      ctx.moveTo(origin.x, origin.y);
      ctx.lineTo(tipX - ux*AH, tipY - uy*AH);
      ctx.strokeStyle = shaft;
      ctx.lineWidth = drag ? 3 : 2;
      ctx.setLineDash([]);
      ctx.stroke();
    } else {
      ctx.beginPath();
      ctx.moveTo(origin.x, origin.y);
      ctx.lineTo(tipX - ux*AH, tipY - uy*AH);
      ctx.strokeStyle = shaft;
      ctx.lineWidth = 2;
      ctx.setLineDash([6,4]);
      ctx.stroke();
      ctx.setLineDash([]);
    }

    // Стрелка
    var px = -uy, py = ux;
    ctx.beginPath();
    ctx.moveTo(tipX, tipY);
    ctx.lineTo(tipX - ux*AH + px*AW/2, tipY - uy*AH + py*AW/2);
    ctx.lineTo(tipX - ux*AH - px*AW/2, tipY - uy*AH - py*AW/2);
    ctx.closePath();
    ctx.fillStyle = shaft;
    ctx.fill();

    // Кружок у основания
    ctx.beginPath();
    ctx.arc(origin.x, origin.y, hover ? 7 : 5, 0, Math.PI*2);
    ctx.fillStyle = shaft;
    ctx.fill();

    // Метка глубины
    if (!previewMode && _state.depthMm > 0) {
      ctx.font = 'bold 12px monospace';
      ctx.fillStyle = '#93c5fd';
      ctx.textAlign = 'left';
      ctx.textBaseline = 'middle';
      ctx.fillText(_state.depthMm.toFixed(0) + ' мм', tipX + ux*10 + 6, tipY + uy*10);
    } else if (previewMode) {
      ctx.font = '10px system-ui, sans-serif';
      ctx.fillStyle = 'rgba(96,165,250,0.8)';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'top';
      ctx.fillText('↕ тяни', origin.x, origin.y + 8);
    }
    ctx.restore();

    // Сохраняем handle для hit-тестирования
    _state.handle = { ox: origin.x, oy: origin.y, tx: tipX, ty: tipY, ux: ux, uy: uy, armLen: armLen };
  };

  // ─────────────────────────────────────────────────────────────────────────
  // Pointer events
  // ─────────────────────────────────────────────────────────────────────────

  function _getCanvas() { return document.getElementById('webgpu-canvas'); }

  function _hitTest(cx, cy) {
    var h = _state.handle;
    if (!h) return false;
    // Попадание по наконечнику
    var dtx = cx - h.tx, dty = cy - h.ty;
    if (dtx*dtx + dty*dty < 28*28) return true;
    // Попадание по штоку
    var dox = cx - h.ox, doy = cy - h.oy;
    var proj = dox*h.ux + doy*h.uy;
    if (proj < 0 || proj > h.armLen) return false;
    return Math.abs(-doy*h.ux + dox*h.uy) < 14;
  }

  // Вызывается из mouse.rs до обычной обработки
  window.__solidExtrudePointerDown = function(e) {
    if (!_state.active) return false;
    var canvas = _getCanvas();
    if (!canvas) return false;
    var rect = canvas.getBoundingClientRect();
    var dpr  = window.devicePixelRatio || 1;
    var cx   = (e.clientX - rect.left) * dpr;
    var cy   = (e.clientY - rect.top)  * dpr;
    if (!_hitTest(cx, cy)) return false;

    e.preventDefault(); e.stopPropagation();
    _state.dragging    = true;
    _state.startPx     = cx * _state.handle.ux + cy * _state.handle.uy;
    _state.startDepthMm = _state.depthMm;
    if (canvas.setPointerCapture) canvas.setPointerCapture(e.pointerId);
    return true;
  };

  var PX_PER_MM = 0.4; // 1px drag ≈ 2.5 мм (удобно для реальных размеров)

  document.addEventListener('pointermove', function(e) {
    if (!_state.active) return;
    var canvas = _getCanvas();
    if (!canvas) return;
    var rect = canvas.getBoundingClientRect();
    var dpr  = window.devicePixelRatio || 1;
    var cx   = (e.clientX - rect.left) * dpr;
    var cy   = (e.clientY - rect.top)  * dpr;

    if (_state.dragging) {
      var h       = _state.handle;
      if (!h) return;
      var curPx   = cx * h.ux + cy * h.uy;
      var deltaPx = curPx - _state.startPx;
      var newMm   = Math.max(1, Math.round(_state.startDepthMm + deltaPx / PX_PER_MM));
      _state.depthMm = newMm;
      _setHudDepth(newMm);
      _schedulePreview(newMm);
      if (window.__setStatusMessage)
        window.__setStatusMessage('↑Solid: ' + newMm + ' мм · Enter ✓ · Esc ✗');
      return;
    }

    var hit = _hitTest(cx, cy);
    if (hit !== _state.hover) {
      _state.hover = hit;
      canvas.style.cursor = hit ? 'ns-resize' : '';
    }
  }, false);

  document.addEventListener('pointerup', function() {
    if (!_state.dragging) return;
    _state.dragging = false;
    var canvas = _getCanvas();
    if (canvas) canvas.style.cursor = '';
  }, false);

  // Keyboard: Enter / Escape / цифры
  document.addEventListener('keydown', function(e) {
    if (!_state.active) return;
    // Пропускаем если фокус в нашем HUD-инпуте
    if (e.target && e.target.id === '__se_depth_hud_inp') return;
    if (e.key === 'Enter') { e.stopImmediatePropagation(); window.__commitSolidExtrude(); }
    if (e.key === 'Escape') { e.stopImmediatePropagation(); window.__cancelSolidExtrude(); }
    // Цифры → фокус в HUD
    if (/^[0-9]$/.test(e.key)) {
      var inp = document.getElementById('__se_depth_hud_inp');
      if (inp) inp.focus();
    }
  }, true);

  console.log('[solid-extrude-gizmo] зарегистрировано: __startSolidExtrude, __commitSolidExtrude, __cancelSolidExtrude, __drawSolidExtrudeGizmo');

})();
"##;
