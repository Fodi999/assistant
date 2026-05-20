// ── Solid Extrude Gizmo (Plasticity-style) ─────────────────────────────────
//
//  Workflow:
//    1. Нарисовал замкнутый профиль (R → прямоугольник)
//    2. Нажал E или кнопку Solid — активируется этот гизмо
//    3. Синяя стрелка перпендикулярно плоскости, тяни вверх
//    4. Drag → депт в мм → debounce 280ms → POST /api/matter/sketch/extrude
//    5. Enter = подтвердить, Esc = отмена
//
//  API: __startSolidExtrude / __cancelSolidExtrude / __commitSolidExtrude
//       __drawSolidExtrudeGizmo(ctx, w2s) — вызывается из render_loop

pub const JS: &str = r##"
(function registerSolidExtrudeGizmo() {

  var _state = {
    active: false, profileId: null, profile: null,
    depthMm: 100, plane: 'XZ', dir: { x:0, y:1, z:0 },
    centroid: { x:0, y:0, z:0 }, dragging: false,
    startPx: 0, startDepthMm: 0, hover: false,
    handle: null, previewMesh: null, _debounce: null,
  };
  window.__solidExtrudeState = _state;

  function _getProfile(profileId) {
    var ss = window.sketchState;
    if (!ss) return null;
    if (window.__recomputeProfiles) window.__recomputeProfiles();
    if (profileId) return (ss.profiles || []).find(function(p){ return p.id === profileId; }) || null;
    if (ss.selectedProfileId) {
      var s = (ss.profiles || []).find(function(p){ return p.id === ss.selectedProfileId; });
      if (s) return s;
    }
    var selEdges = [].concat(ss.selectedEdgeIds || []);
    if (selEdges.length) {
      var f = (ss.profiles || []).find(function(p){
        return selEdges.some(function(eid){ return (p.edgeIds||[]).includes(eid); });
      });
      if (f) return f;
    }
    return (ss.profiles && ss.profiles.length) ? ss.profiles[0] : null;
  }

  function _centroid(profile) {
    var ss = window.sketchState;
    if (!ss || !profile) return { x:0, y:0, z:0 };
    var byId = new Map((ss.points||[]).map(function(p){ return [p.id, p]; }));
    var pts = (profile.pointIds||[]).map(function(id){ return byId.get(id); }).filter(Boolean);
    if (!pts.length) return { x:0, y:0, z:0 };
    var s = pts.reduce(function(a,p){ return { x:a.x+p.x, y:a.y+p.y, z:a.z+p.z }; }, {x:0,y:0,z:0});
    return { x:s.x/pts.length, y:s.y/pts.length, z:s.z/pts.length };
  }

  function _planeDir(plane) {
    if (plane === 'XY') return { x:0, y:0, z:1 };
    if (plane === 'YZ') return { x:1, y:0, z:0 };
    return { x:0, y:1, z:0 }; // XZ
  }

  function _profilePoints(profile) {
    var ss = window.sketchState;
    if (!ss || !profile) return null;
    var byId = new Map((ss.points||[]).map(function(p){ return [p.id, p]; }));
    var pts = (profile.pointIds||[]).map(function(id){ return byId.get(id); }).filter(Boolean);
    if (pts.length < 3) return null;
    return pts.map(function(p){ return { x:p.x, y:p.y, z:p.z }; });
  }

  // ── Start / Cancel / Commit ───────────────────────────────────────────────

  window.__startSolidExtrude = function(profileId) {
    var prof = _getProfile(profileId);
    if (!prof) {
      if (window.__setStatusMessage)
        window.__setStatusMessage('Solid: no profile — draw a closed shape (R)');
      return;
    }
    var ss    = window.sketchState;
    var plane = (prof.plane && prof.plane !== 'unknown') ? prof.plane
              : (ss ? (ss.workingPlane || 'XZ') : 'XZ');
    _state.active = true;  _state.profileId = prof.id; _state.profile = prof;
    _state.depthMm = 100;  _state.plane = plane;
    _state.dir = _planeDir(plane);  _state.centroid = _centroid(prof);
    _state.dragging = false; _state.hover = false;
    _state.handle = null;   _state.previewMesh = null;
    if (_state._debounce) { clearTimeout(_state._debounce); _state._debounce = null; }
    var dimEl = document.getElementById('__dim-editor');
    if (dimEl) dimEl.style.display = 'none';
    _showDepthHud(100);
    if (window.__setStatusMessage)
      window.__setStatusMessage('Solid extrude: drag arrow up / type mm / Enter=OK / Esc=cancel');
    console.log('[SolidExtrude] start', prof.id, plane, _state.centroid);
    _schedulePreview(100);
  };

  window.__cancelSolidExtrude = function() {
    if (!_state.active) return;
    _state.active = false;
    if (_state._debounce) { clearTimeout(_state._debounce); _state._debounce = null; }
    _hideDepthHud();
    if (window.__closeSolidPreview) window.__closeSolidPreview();
    if (window.__setStatusMessage) window.__setStatusMessage('Solid: cancelled');
  };

  window.__commitSolidExtrude = async function() {
    if (!_state.active) return;
    var d = _state.depthMm;
    _state.active = false;
    if (_state._debounce) { clearTimeout(_state._debounce); _state._debounce = null; }
    _hideDepthHud();
    await _sendToKernel(d, true);
  };

  // ── Depth HUD ─────────────────────────────────────────────────────────────

  function _getHud() {
    var el = document.getElementById('__solid_extrude_hud');
    if (el) return el;
    var T = window.__modalTheme;
    var C = T ? T.COLORS : { panel:'#1e293b', border:'rgba(56,189,248,.3)', mute:'#64748b' };
    var L = T ? T.LAYOUT : { font:"'JetBrains Mono',monospace", borderRadius:'8px' };
    el = document.createElement('div');
    el.id = '__solid_extrude_hud';
    Object.assign(el.style, {
      display:'none', position:'fixed', zIndex:'10020',
      background:C.panel, border:'1px solid ' + C.border,
      borderRadius:L.borderRadius, padding:'6px 10px 8px',
      fontFamily:L.font, fontSize:'11px', color:C.mute,
      userSelect:'none', pointerEvents:'auto',
      boxShadow:'0 4px 16px rgba(0,0,0,.5)', minWidth:'180px',
    });
    el.innerHTML =
      '<div style="margin-bottom:4px;font-size:10px;letter-spacing:.6px;text-transform:uppercase;color:rgba(56,189,248,.7);">Solid Extrude</div>' +
      '<div style="display:flex;align-items:center;gap:6px;">' +
        '<input id="__se_depth_hud_inp" type="number" min="1" step="1" value="100" ' +
          'style="width:80px;text-align:right;font-size:18px;font-weight:700;' +
          'background:rgba(255,255,255,.07);border:1px solid rgba(56,189,248,.3);' +
          'border-radius:5px;padding:3px 6px;color:#38bdf8;outline:none;font-family:inherit;" />' +
        '<span style="color:#94a3b8;">mm</span>' +
      '</div>' +
      '<div style="margin-top:5px;font-size:10px;color:#475569;">Enter = OK &nbsp; Esc = cancel</div>';
    document.body.appendChild(el);
    var inp = el.querySelector('#__se_depth_hud_inp');
    inp.addEventListener('input', function() {
      var v = parseFloat(inp.value);
      if (isFinite(v) && v > 0) { _state.depthMm = v; _schedulePreview(v); }
    });
    inp.addEventListener('keydown', function(e) {
      e.stopPropagation();
      if (e.key === 'Enter')  { e.preventDefault(); window.__commitSolidExtrude(); }
      if (e.key === 'Escape') { e.preventDefault(); window.__cancelSolidExtrude(); }
    });
    return el;
  }

  function _showDepthHud(d) {
    var hud = _getHud();
    hud.style.display = 'block'; hud.style.left = '50%';
    hud.style.bottom = '72px'; hud.style.top = 'auto';
    hud.style.transform = 'translateX(-50%)';
    var inp = document.getElementById('__se_depth_hud_inp');
    if (inp) { inp.value = d; setTimeout(function(){ inp.focus(); inp.select(); }, 60); }
  }
  function _hideDepthHud() {
    var hud = document.getElementById('__solid_extrude_hud');
    if (hud) hud.style.display = 'none';
  }
  function _setHudDepth(d) {
    var inp = document.getElementById('__se_depth_hud_inp');
    if (inp && document.activeElement !== inp) inp.value = Math.round(d);
  }

  // ── Kernel POST ───────────────────────────────────────────────────────────

  function _schedulePreview(depthMm) {
    if (_state._debounce) clearTimeout(_state._debounce);
    _state._debounce = setTimeout(function() {
      _state._debounce = null;
      _sendToKernel(depthMm, false);
    }, 280);
  }

  async function _sendToKernel(depthMm, isFinal) {
    if (_state.active && !_state.profile) _state.profile = _getProfile(_state.profileId);
    var prof = _state.profile || _getProfile(null);
    if (!prof) { console.warn('[SolidExtrude] no profile'); return; }
    var pts = _profilePoints(prof);
    if (!pts || pts.length < 3) {
      if (window.__setStatusMessage) window.__setStatusMessage('Solid: no profile points');
      return;
    }
    var plane  = _state.plane || 'XZ';
    var depthM = depthMm / 1000.0;
    if (window.__setStatusMessage)
      window.__setStatusMessage('⏳ building ' + depthMm + ' mm...');
    console.log('[SolidExtrude] POST /api/matter/sketch/extrude plane=' + plane +
      ' depth=' + depthM + 'm pts=' + pts.length + ' first=' + JSON.stringify(pts[0]));
    var body = { plane: plane, depth: depthM, profile: pts, tolerance: isFinal ? 0.003 : 0.008 };
    try {
      var t0 = performance.now();
      var resp = await fetch('/api/matter/sketch/extrude', {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
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
      if (window.__uploadSolidToScene) window.__uploadSolidToScene(result);
      if (window.__setStatusMessage)
        window.__setStatusMessage(
          (isFinal ? '✓ Solid' : 'preview') + ': ' +
          result.vertex_count + ' vert / ' + result.triangle_count + ' tri / ' + dt + 'ms'
        );
      if (window.__showSolidPreviewPanel) {
        window.__showSolidPreviewPanel(result, depthMm, plane);
      }
    } catch(e) {
      console.error('[SolidExtrude] kernel error:', e);
      if (window.__setStatusMessage) window.__setStatusMessage('✗ kernel error: ' + e.message);
    }
  }

  // ── Draw gizmo ────────────────────────────────────────────────────────────

  window.__drawSolidExtrudeGizmo = function(ctx, w2s) {
    if (!_state.active) { _state.handle = null; return; }
    var cen = _state.centroid, dir = _state.dir;
    var dep = _state.depthMm / 1000;
    var origin = w2s(cen.x, cen.y, cen.z);
    if (!origin) return;

    var tipWorld = { x:cen.x+dir.x*dep, y:cen.y+dir.y*dep, z:cen.z+dir.z*dep };
    var tipScr   = w2s(tipWorld.x, tipWorld.y, tipWorld.z);
    var tipX, tipY, previewMode;
    if (!tipScr || dep < 0.001) {
      var unit = w2s(cen.x+dir.x*0.3, cen.y+dir.y*0.3, cen.z+dir.z*0.3);
      if (!unit) return;
      var ddx=unit.x-origin.x, ddy=unit.y-origin.y, l=Math.hypot(ddx,ddy)||1;
      tipX=origin.x+ddx/l*90; tipY=origin.y+ddy/l*90; previewMode=true;
    } else { tipX=tipScr.x; tipY=tipScr.y; previewMode=false; }

    var adx=tipX-origin.x, ady=tipY-origin.y;
    var armLen=Math.hypot(adx,ady)||1, ux=adx/armLen, uy=ady/armLen;
    var hover=_state.hover, drag=_state.dragging;
    var shaft=drag?'#93c5fd':hover?'#60a5fa':previewMode?'rgba(96,165,250,0.55)':'#3b82f6';
    var AH=20, AW=9;

    ctx.save(); ctx.lineCap='round'; ctx.lineJoin='round';

    // Жёлтый контур профиля
    if (_state.profile) {
      var ss=window.sketchState;
      var byPt=new Map((ss.points||[]).map(function(p){ return [p.id,p]; }));
      var pids=_state.profile.pointIds||[];
      if (pids.length>=3) {
        ctx.beginPath();
        for (var i=0;i<pids.length;i++) {
          var pt=byPt.get(pids[i]); if(!pt) continue;
          var sc2=w2s(pt.x,pt.y,pt.z); if(!sc2) continue;
          if(i===0) ctx.moveTo(sc2.x,sc2.y); else ctx.lineTo(sc2.x,sc2.y);
        }
        ctx.closePath();
        ctx.strokeStyle='rgba(250,204,21,0.75)'; ctx.lineWidth=2;
        ctx.setLineDash([]); ctx.stroke();
      }
    }

    // Шток
    ctx.beginPath(); ctx.moveTo(origin.x,origin.y); ctx.lineTo(tipX-ux*AH,tipY-uy*AH);
    ctx.strokeStyle=shaft; ctx.lineWidth=drag?3:2;
    ctx.setLineDash(previewMode?[6,4]:[]); ctx.stroke(); ctx.setLineDash([]);

    // Наконечник
    var px=-uy, py=ux;
    ctx.beginPath(); ctx.moveTo(tipX,tipY);
    ctx.lineTo(tipX-ux*AH+px*AW/2,tipY-uy*AH+py*AW/2);
    ctx.lineTo(tipX-ux*AH-px*AW/2,tipY-uy*AH-py*AW/2);
    ctx.closePath(); ctx.fillStyle=shaft; ctx.fill();

    // Кружок
    ctx.beginPath(); ctx.arc(origin.x,origin.y,hover?7:5,0,Math.PI*2);
    ctx.fillStyle=shaft; ctx.fill();

    // Метка
    if (!previewMode && _state.depthMm>0) {
      ctx.font='bold 12px monospace'; ctx.fillStyle='#93c5fd';
      ctx.textAlign='left'; ctx.textBaseline='middle';
      ctx.fillText(_state.depthMm.toFixed(0)+' mm', tipX+ux*10+6, tipY+uy*10);
    } else if (previewMode) {
      ctx.font='10px system-ui,sans-serif'; ctx.fillStyle='rgba(96,165,250,0.8)';
      ctx.textAlign='center'; ctx.textBaseline='top';
      ctx.fillText('drag', origin.x, origin.y+8);
    }
    ctx.restore();
    _state.handle={ox:origin.x,oy:origin.y,tx:tipX,ty:tipY,ux:ux,uy:uy,armLen:armLen};
  };

  // ── Pointer events ────────────────────────────────────────────────────────

  function _getCanvas() { return document.getElementById('webgpu-canvas'); }
  function _hitTest(cx,cy) {
    var h=_state.handle; if(!h) return false;
    var dtx=cx-h.tx,dty=cy-h.ty;
    if(dtx*dtx+dty*dty<28*28) return true;
    var dox=cx-h.ox,doy=cy-h.oy,proj=dox*h.ux+doy*h.uy;
    if(proj<0||proj>h.armLen) return false;
    return Math.abs(-doy*h.ux+dox*h.uy)<14;
  }

  window.__solidExtrudePointerDown = function(e) {
    if (!_state.active) return false;
    var canvas=_getCanvas(); if(!canvas) return false;
    var rect=canvas.getBoundingClientRect(), dpr=window.devicePixelRatio||1;
    var cx=(e.clientX-rect.left)*dpr, cy=(e.clientY-rect.top)*dpr;
    if (!_hitTest(cx,cy)) return false;
    e.preventDefault(); e.stopPropagation();
    _state.dragging=true;
    _state.startPx=cx*_state.handle.ux+cy*_state.handle.uy;
    _state.startDepthMm=_state.depthMm;
    if (canvas.setPointerCapture) canvas.setPointerCapture(e.pointerId);
    return true;
  };

  var PX_PER_MM=0.4;
  document.addEventListener('pointermove', function(e) {
    if (!_state.active) return;
    var canvas=_getCanvas(); if(!canvas) return;
    var rect=canvas.getBoundingClientRect(), dpr=window.devicePixelRatio||1;
    var cx=(e.clientX-rect.left)*dpr, cy=(e.clientY-rect.top)*dpr;
    if (_state.dragging) {
      var h=_state.handle; if(!h) return;
      var newMm=Math.max(1,Math.round(_state.startDepthMm+(cx*h.ux+cy*h.uy-_state.startPx)/PX_PER_MM));
      _state.depthMm=newMm; _setHudDepth(newMm); _schedulePreview(newMm);
      if (window.__setStatusMessage) window.__setStatusMessage('Solid: '+newMm+' mm · Enter=OK · Esc=cancel');
      return;
    }
    var hit=_hitTest(cx,cy);
    if (hit!==_state.hover) { _state.hover=hit; canvas.style.cursor=hit?'ns-resize':''; }
  },false);

  document.addEventListener('pointerup',function() {
    if (!_state.dragging) return;
    _state.dragging=false;
    var canvas=_getCanvas(); if(canvas) canvas.style.cursor='';
  },false);

  document.addEventListener('keydown',function(e) {
    if (!_state.active) return;
    if (e.target&&e.target.id==='__se_depth_hud_inp') return;
    if (e.key==='Enter')  { e.stopImmediatePropagation(); window.__commitSolidExtrude(); }
    if (e.key==='Escape') { e.stopImmediatePropagation(); window.__cancelSolidExtrude(); }
    if (/^[0-9]$/.test(e.key)) {
      var inp=document.getElementById('__se_depth_hud_inp'); if(inp) inp.focus();
    }
  },true);

  console.log('[solid-extrude-gizmo] ready: __startSolidExtrude / __commitSolidExtrude / __cancelSolidExtrude / __drawSolidExtrudeGizmo');

})();
"##;
