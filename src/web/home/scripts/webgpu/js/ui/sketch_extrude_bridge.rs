// ── Sketch → Solid Bridge ──────────────────────────────────────────────────
//
//  Соединяет две библиотеки:
//    sketch_engine (WASM, браузер) — 2D constraint solver
//    truck-modeling (Rust, сервер) — B-Rep solid / tessellation
//
//  Пайплайн:
//    1. sketchState.profiles  → закрытые 2D профили (из sketch_engine)
//    2. POST /api/matter/sketch/extrude  → truck: Wire → Face → Solid → меш
//    3. Результат: WebGL мини-превью + OBJ-скачать + статус-значок
//
//  Публичные функции:
//    window.__extrudeToSolid(profileId?, depthMm?)
//    window.__closeSolidPreview()

pub const JS: &str = r##"
(function registerSketchExtrudeBridge() {

  // ─────────────────────────────────────────────────────────────────────────
  // 1. Вспомогательные функции
  // ─────────────────────────────────────────────────────────────────────────

  /** Выбрать профиль: явный id или первый замкнутый из sketchState */
  function _pickProfile(profileId) {
    const ss = window.sketchState;
    if (!ss || !ss.profiles || !ss.profiles.length) return null;
    if (profileId) return ss.profiles.find(p => p.id === profileId) || null;
    // Предпочитаем выбранный; иначе — первый
    if (ss.selectedProfileId) {
      const sel = ss.profiles.find(p => p.id === ss.selectedProfileId);
      if (sel) return sel;
    }
    return ss.profiles[0];
  }

  /** Преобразуем профиль в массив [{x,y,z}] (world metres, CCW) */
  function _profilePoints(profile) {
    const ss = window.sketchState;
    if (!ss || !profile || !profile.pointIds) return null;
    const byId = new Map((ss.points || []).map(p => [p.id, p]));
    const pts = profile.pointIds.map(id => byId.get(id)).filter(Boolean);
    if (pts.length < 3) return null;
    return pts.map(p => ({ x: p.x, y: p.y, z: p.z }));
  }

  /** Определить плоскость профиля */
  function _profilePlane(profile) {
    const ss = window.sketchState;
    if (!ss || !profile || !profile.pointIds) return ss ? (ss.workingPlane || 'XZ') : 'XZ';
    const byId = new Map((ss.points || []).map(p => [p.id, p]));
    const pts = profile.pointIds.map(id => byId.get(id)).filter(Boolean);
    const eps = 1e-6;
    if (pts.length && pts.every(p => Math.abs(p.y - pts[0].y) < eps)) return 'XZ';
    if (pts.length && pts.every(p => Math.abs(p.z - pts[0].z) < eps)) return 'XY';
    if (pts.length && pts.every(p => Math.abs(p.x - pts[0].x) < eps)) return 'YZ';
    return (ss.workingPlane || 'XZ');
  }

  // ─────────────────────────────────────────────────────────────────────────
  // 2. Модальное окно ввода глубины
  // ─────────────────────────────────────────────────────────────────────────

  function _getModal() {
    let el = document.getElementById('__se_bridge_modal');
    if (el) return el;

    const T = window.__modalTheme;
    const C = T ? T.COLORS : { panel:'#1e293b', border:'rgba(56,189,248,.25)', input:'#f1f5f9', mute:'#64748b', dim:'#94a3b8' };
    const L = T ? T.LAYOUT : { font:"'JetBrains Mono',monospace", borderRadius:'10px' };

    el = document.createElement('div');
    el.id = '__se_bridge_modal';
    Object.assign(el.style, {
      display:        'none',
      position:       'fixed',
      left:           '50%',
      top:            '50%',
      transform:      'translate(-50%,-50%)',
      zIndex:         '10010',
      background:     C.panel,
      border:         '1px solid ' + C.border,
      borderRadius:   L.borderRadius,
      padding:        '18px 20px 16px',
      minWidth:       '300px',
      maxWidth:       '380px',
      boxShadow:      '0 8px 32px rgba(0,0,0,.55)',
      fontFamily:     L.font,
      color:          C.input,
      userSelect:     'none',
    });

    el.innerHTML = `
      <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:12px;">
        <span style="font-size:11px;font-weight:700;letter-spacing:.8px;text-transform:uppercase;color:${C.mute};">
          🧱 Sketch → Solid (truck B-Rep)
        </span>
        <button id="__se_bridge_close" style="background:none;border:none;color:${C.dim};font-size:16px;cursor:pointer;padding:0 2px;">✕</button>
      </div>

      <div style="font-size:12px;color:${C.mute};margin-bottom:8px;">Глубина экструзии (мм)</div>
      <input id="__se_bridge_depth" type="number" min="1" max="100000" step="1" value="100"
        style="display:block;width:100%;box-sizing:border-box;text-align:center;
               font-size:28px;font-weight:700;font-family:${L.font};
               color:${C.input};background:rgba(255,255,255,.06);
               border:1px solid ${C.border};border-radius:8px;
               padding:6px 8px;outline:none;-moz-appearance:textfield;margin-bottom:14px;" />

      <div id="__se_bridge_info" style="font-size:11px;color:${C.mute};margin-bottom:12px;min-height:14px;"></div>

      <div style="display:flex;gap:8px;">
        <button id="__se_bridge_cancel" style="flex:1;padding:8px 0;font-family:${L.font};
          font-size:12px;font-weight:600;background:rgba(255,255,255,.07);
          border:1px solid ${C.border};border-radius:7px;color:${C.dim};cursor:pointer;">
          Esc · Отмена
        </button>
        <button id="__se_bridge_ok" style="flex:2;padding:8px 0;font-family:${L.font};
          font-size:12px;font-weight:700;background:rgba(56,189,248,.18);
          border:1px solid rgba(56,189,248,.4);border-radius:7px;color:#38bdf8;cursor:pointer;">
          ↵ Выдавить
        </button>
      </div>
    `;

    document.body.appendChild(el);

    el.querySelector('#__se_bridge_close').onclick  = () => _hideModal();
    el.querySelector('#__se_bridge_cancel').onclick = () => _hideModal();
    el.querySelector('#__se_bridge_ok').onclick     = () => _submitExtrude();

    // Enter = submit, Escape = close
    el.querySelector('#__se_bridge_depth').addEventListener('keydown', function(e) {
      if (e.key === 'Enter')  { e.preventDefault(); _submitExtrude(); }
      if (e.key === 'Escape') { e.preventDefault(); _hideModal(); }
      e.stopPropagation();
    });

    return el;
  }

  let _activeProfile = null;

  function _showModal(profileId) {
    _activeProfile = _pickProfile(profileId);
    const modal = _getModal();
    const info  = document.getElementById('__se_bridge_info');
    if (info) {
      if (_activeProfile) {
        const n = (_activeProfile.pointIds || []).length;
        const plane = _profilePlane(_activeProfile);
        info.textContent = 'Профиль: ' + (_activeProfile.id || '—') + ' · ' + n + ' точек · плоскость ' + plane;
      } else {
        info.textContent = '⚠ Нет замкнутого профиля — нарисуйте замкнутый контур';
      }
    }
    modal.style.display = 'block';
    setTimeout(() => {
      const inp = document.getElementById('__se_bridge_depth');
      if (inp) { inp.focus(); inp.select(); }
    }, 40);
  }

  function _hideModal() {
    const modal = document.getElementById('__se_bridge_modal');
    if (modal) modal.style.display = 'none';
    _activeProfile = null;
  }

  // ─────────────────────────────────────────────────────────────────────────
  // 3. Отправка на truck-modeling backend
  // ─────────────────────────────────────────────────────────────────────────

  async function _submitExtrude() {
    if (!_activeProfile) {
      if (window.__setStatusMessage) window.__setStatusMessage('⚠ Нет профиля для экструзии');
      _hideModal();
      return;
    }
    const depthInput = document.getElementById('__se_bridge_depth');
    const depthMm    = parseFloat(depthInput ? depthInput.value : '100');
    if (!isFinite(depthMm) || depthMm <= 0) {
      if (window.__setStatusMessage) window.__setStatusMessage('⚠ Введите положительную глубину');
      return;
    }
    const depthM = depthMm / 1000.0; // мм → метры

    const pts = _profilePoints(_activeProfile);
    if (!pts || pts.length < 3) {
      if (window.__setStatusMessage) window.__setStatusMessage('⚠ Профиль должен содержать ≥ 3 точек');
      _hideModal();
      return;
    }

    const plane = _profilePlane(_activeProfile);

    _hideModal();
    if (window.__setStatusMessage) window.__setStatusMessage('⏳ truck: строю B-Rep solid…');

    const body = { plane, depth: depthM, profile: pts, tolerance: 0.005 };
    console.log('[sketch→solid] POST /api/matter/sketch/extrude', body);

    let result;
    try {
      const t0   = performance.now();
      const resp = await fetch('/api/matter/sketch/extrude', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });
      const dt = (performance.now() - t0).toFixed(0);
      if (!resp.ok) {
        const err = await resp.json().catch(() => ({ error: resp.statusText }));
        throw new Error(err.error || resp.statusText);
      }
      result = await resp.json();
      result.__dt = dt;
    } catch (e) {
      console.error('[sketch→solid] error:', e);
      if (window.__setStatusMessage)
        window.__setStatusMessage('✗ truck: ' + e.message);
      return;
    }

    console.log('[sketch→solid] result:', {
      vertices:  result.vertex_count,
      triangles: result.triangle_count,
      faces:     result.face_count,
      kernel:    result.kernel,
      dt:        result.__dt + ' мс',
    });

    if (window.__setStatusMessage)
      window.__setStatusMessage(
        '✅ B-Rep solid: ' + result.vertex_count + ' вершин · ' +
        result.triangle_count + ' треугольников · ' +
        result.face_count + ' граней · ' + result.__dt + ' мс'
      );

    // Сохраняем результат глобально
    window.__lastSolidResult = result;

    // Показать превью + предложить скачать
    _showSolidPreview(result, depthMm, plane);
  }

  // ─────────────────────────────────────────────────────────────────────────
  // 4. WebGL мини-превью
  // ─────────────────────────────────────────────────────────────────────────

  function _getPreviewPanel() {
    let el = document.getElementById('__se_preview_panel');
    if (el) return el;

    const T = window.__modalTheme;
    const C = T ? T.COLORS : { panel:'#1e293b', border:'rgba(56,189,248,.25)', input:'#f1f5f9', mute:'#64748b', dim:'#94a3b8' };
    const L = T ? T.LAYOUT : { font:"'JetBrains Mono',monospace", borderRadius:'10px' };

    el = document.createElement('div');
    el.id = '__se_preview_panel';
    Object.assign(el.style, {
      position:     'fixed',
      right:        '24px',
      bottom:       '72px',
      zIndex:       '10005',
      background:   C.panel,
      border:       '1px solid ' + C.border,
      borderRadius: L.borderRadius,
      padding:      '12px 14px 10px',
      minWidth:     '260px',
      boxShadow:    '0 8px 32px rgba(0,0,0,.55)',
      fontFamily:   L.font,
      color:        C.input,
      display:      'none',
      userSelect:   'none',
    });

    el.innerHTML = `
      <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:8px;">
        <span style="font-size:11px;font-weight:700;letter-spacing:.7px;text-transform:uppercase;color:${C.mute};">
          🧱 Solid Preview
        </span>
        <button id="__se_preview_close"
          style="background:none;border:none;color:${C.dim};font-size:14px;cursor:pointer;padding:0 2px;">✕</button>
      </div>
      <canvas id="__se_preview_canvas" width="232" height="180"
        style="display:block;border-radius:6px;background:#0f172a;border:1px solid ${C.border};"></canvas>
      <div id="__se_preview_stats" style="font-size:10px;color:${C.mute};margin-top:6px;line-height:1.6;"></div>
      <div style="display:flex;gap:6px;margin-top:8px;">
        <button id="__se_preview_download" style="flex:1;padding:5px 0;font-family:${L.font};
          font-size:11px;font-weight:700;background:rgba(56,189,248,.16);
          border:1px solid rgba(56,189,248,.35);border-radius:6px;color:#38bdf8;cursor:pointer;">
          ⬇ Скачать OBJ
        </button>
        <button id="__se_preview_rerun" style="flex:1;padding:5px 0;font-family:${L.font};
          font-size:11px;font-weight:600;background:rgba(255,255,255,.06);
          border:1px solid ${C.border};border-radius:6px;color:${C.dim};cursor:pointer;">
          ↺ Снова
        </button>
      </div>
    `;

    document.body.appendChild(el);

    el.querySelector('#__se_preview_close').onclick    = () => window.__closeSolidPreview();
    el.querySelector('#__se_preview_download').onclick = () => _downloadObj();
    el.querySelector('#__se_preview_rerun').onclick    = () => window.__extrudeToSolid();

    return el;
  }

  function _showSolidPreview(result, depthMm, plane) {
    const panel = _getPreviewPanel();
    panel.style.display = 'block';

    // Stats
    const stats = document.getElementById('__se_preview_stats');
    if (stats) {
      stats.innerHTML =
        'Вершин: <b>' + result.vertex_count + '</b> · ' +
        'Треугольников: <b>' + result.triangle_count + '</b><br>' +
        'Граней: <b>' + result.face_count + '</b> · ' +
        'Глубина: <b>' + depthMm.toFixed(1) + ' мм</b> · ' +
        'Плоскость: <b>' + plane + '</b><br>' +
        'Движок: <b>' + (result.kernel || 'truck-modeling') + '</b>';
    }

    // WebGL render
    _renderWebGL(result);
  }

  // ─────────────────────────────────────────────────────────────────────────
  // 5. WebGL рендер (простой фонг с вращением)
  // ─────────────────────────────────────────────────────────────────────────

  let _glCtx = null, _rafId = null;
  let _rotY = 0, _rotX = 0.3;

  function _renderWebGL(result) {
    const canvas = document.getElementById('__se_preview_canvas');
    if (!canvas) return;

    // Остановить предыдущую анимацию
    if (_rafId) { cancelAnimationFrame(_rafId); _rafId = null; }

    const gl = canvas.getContext('webgl2') || canvas.getContext('webgl');
    if (!gl) {
      canvas.getContext('2d')?.clearRect(0, 0, canvas.width, canvas.height);
      const ctx2 = canvas.getContext('2d');
      if (ctx2) {
        ctx2.fillStyle = '#0f172a';
        ctx2.fillRect(0, 0, canvas.width, canvas.height);
        ctx2.fillStyle = '#64748b';
        ctx2.font = '11px monospace';
        ctx2.fillText('WebGL недоступен', 10, 90);
      }
      return;
    }
    _glCtx = gl;

    const vs = `
      attribute vec3 aPos;
      attribute vec3 aNorm;
      uniform mat4 uMVP;
      uniform mat3 uNM;
      varying vec3 vNorm;
      void main() {
        vNorm = normalize(uNM * aNorm);
        gl_Position = uMVP * vec4(aPos, 1.0);
      }
    `;
    const fs = `
      precision mediump float;
      varying vec3 vNorm;
      uniform vec3 uLightDir;
      uniform vec3 uBaseColor;
      void main() {
        float diff = max(dot(normalize(vNorm), normalize(uLightDir)), 0.0);
        float amb  = 0.25;
        vec3 color = uBaseColor * (amb + diff * 0.75);
        gl_FragColor = vec4(color, 1.0);
      }
    `;

    function _compileShader(src, type) {
      const s = gl.createShader(type);
      gl.shaderSource(s, src);
      gl.compileShader(s);
      return s;
    }
    const prog = gl.createProgram();
    gl.attachShader(prog, _compileShader(vs, gl.VERTEX_SHADER));
    gl.attachShader(prog, _compileShader(fs, gl.FRAGMENT_SHADER));
    gl.linkProgram(prog);
    gl.useProgram(prog);

    // Буферы
    const posBuf  = gl.createBuffer();
    const normBuf = gl.createBuffer();
    const idxBuf  = gl.createBuffer();

    gl.bindBuffer(gl.ARRAY_BUFFER, posBuf);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(result.positions), gl.STATIC_DRAW);

    gl.bindBuffer(gl.ARRAY_BUFFER, normBuf);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(result.normals), gl.STATIC_DRAW);

    gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, idxBuf);
    gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, new Uint32Array(result.indices), gl.STATIC_DRAW);

    const aPos   = gl.getAttribLocation(prog, 'aPos');
    const aNorm  = gl.getAttribLocation(prog, 'aNorm');
    const uMVP   = gl.getUniformLocation(prog, 'uMVP');
    const uNM    = gl.getUniformLocation(prog, 'uNM');
    const uLight = gl.getUniformLocation(prog, 'uLightDir');
    const uColor = gl.getUniformLocation(prog, 'uBaseColor');

    // AABB → нормализация
    const pos = result.positions;
    let mnX = Infinity, mnY = Infinity, mnZ = Infinity;
    let mxX = -Infinity, mxY = -Infinity, mxZ = -Infinity;
    for (let i = 0; i < pos.length; i += 3) {
      mnX = Math.min(mnX, pos[i]);   mxX = Math.max(mxX, pos[i]);
      mnY = Math.min(mnY, pos[i+1]); mxY = Math.max(mxY, pos[i+1]);
      mnZ = Math.min(mnZ, pos[i+2]); mxZ = Math.max(mxZ, pos[i+2]);
    }
    const cx = (mnX + mxX) / 2, cy = (mnY + mxY) / 2, cz = (mnZ + mxZ) / 2;
    const sz = Math.max(mxX - mnX, mxY - mnY, mxZ - mnZ) || 1;
    const sc = 1.6 / sz;

    // mat4 helpers (column-major)
    function _mat4mul(a, b) {
      const out = new Float32Array(16);
      for (let col = 0; col < 4; col++)
        for (let row = 0; row < 4; row++) {
          let v = 0;
          for (let k = 0; k < 4; k++) v += a[k*4+row] * b[col*4+k];
          out[col*4+row] = v;
        }
      return out;
    }
    function _rotY_(t) {
      const c = Math.cos(t), s = Math.sin(t);
      return new Float32Array([c,0,-s,0, 0,1,0,0, s,0,c,0, 0,0,0,1]);
    }
    function _rotX_(t) {
      const c = Math.cos(t), s = Math.sin(t);
      return new Float32Array([1,0,0,0, 0,c,s,0, 0,-s,c,0, 0,0,0,1]);
    }
    function _persp(fov, asp, near, far) {
      const f = 1.0 / Math.tan(fov / 2);
      return new Float32Array([
        f/asp, 0, 0, 0,
        0, f, 0, 0,
        0, 0, (far+near)/(near-far), -1,
        0, 0, (2*far*near)/(near-far), 0,
      ]);
    }
    function _translate(tx, ty, tz) {
      return new Float32Array([1,0,0,0, 0,1,0,0, 0,0,1,0, tx,ty,tz,1]);
    }
    function _scale(s) {
      return new Float32Array([s,0,0,0, 0,s,0,0, 0,0,s,0, 0,0,0,1]);
    }

    const W = canvas.width, H = canvas.height;

    function _draw() {
      gl.viewport(0, 0, W, H);
      gl.clearColor(0.059, 0.090, 0.165, 1.0);
      gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
      gl.enable(gl.DEPTH_TEST);
      gl.enable(gl.CULL_FACE);

      // MVP
      const model = _mat4mul(
        _mat4mul(_translate(-cx, -cy, -cz), _scale(sc)),
        _mat4mul(_rotX_(_rotX), _rotY_(_rotY))
      );
      const view  = _translate(0, 0, -2.8);
      const proj  = _persp(0.9, W / H, 0.01, 100.0);
      const mvp   = _mat4mul(_mat4mul(model, view), proj);
      gl.uniformMatrix4fv(uMVP, false, mvp);

      // Normal matrix (upper 3×3 of model, since no non-uniform scale)
      const nm = new Float32Array([
        model[0], model[1], model[2],
        model[4], model[5], model[6],
        model[8], model[9], model[10],
      ]);
      gl.uniformMatrix3fv(uNM, false, nm);
      gl.uniform3f(uLight, 0.6, 1.0, 0.8);
      gl.uniform3f(uColor, 0.22, 0.74, 0.98); // sky blue

      // Positions
      gl.bindBuffer(gl.ARRAY_BUFFER, posBuf);
      gl.enableVertexAttribArray(aPos);
      gl.vertexAttribPointer(aPos, 3, gl.FLOAT, false, 0, 0);

      // Normals
      gl.bindBuffer(gl.ARRAY_BUFFER, normBuf);
      gl.enableVertexAttribArray(aNorm);
      gl.vertexAttribPointer(aNorm, 3, gl.FLOAT, false, 0, 0);

      gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, idxBuf);
      gl.drawElements(gl.TRIANGLES, result.indices.length, gl.UNSIGNED_INT, 0);

      _rotY += 0.012;
      _rafId = requestAnimationFrame(_draw);
    }
    _draw();
  }

  // ─────────────────────────────────────────────────────────────────────────
  // 6. OBJ скачивание
  // ─────────────────────────────────────────────────────────────────────────

  function _downloadObj() {
    const r = window.__lastSolidResult;
    if (!r || !r.obj_data) {
      if (window.__setStatusMessage) window.__setStatusMessage('⚠ Нет данных OBJ');
      return;
    }
    const blob = new Blob([r.obj_data], { type: 'text/plain' });
    const url  = URL.createObjectURL(blob);
    const a    = document.createElement('a');
    a.href     = url;
    a.download = 'sketch_solid_' + Date.now() + '.obj';
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    if (window.__setStatusMessage) window.__setStatusMessage('⬇ OBJ скачан');
  }

  // ─────────────────────────────────────────────────────────────────────────
  // 7. Публичный API
  // ─────────────────────────────────────────────────────────────────────────

  /**
   * Главная точка входа.
   * Открывает модальное окно ввода глубины, затем:
   *   sketch_engine (WASM, 2D профиль) → truck (Rust, 3D solid) → WebGL превью
   *
   * @param {string} [profileId]  — id профиля (необязательно; default: первый/выбранный)
   * @param {number} [depthMm]    — глубина мм (необязательно; если передана, пропустить модал)
   */
  window.__extrudeToSolid = function(profileId, depthMm) {
    if (depthMm != null && isFinite(depthMm) && depthMm > 0) {
      // Прямой вызов без диалога (например, из теста или другого инструмента)
      _activeProfile = _pickProfile(profileId);
      const mock = document.getElementById('__se_bridge_depth');
      if (mock) mock.value = String(depthMm);
      _submitExtrude();
    } else {
      _showModal(profileId);
    }
  };

  /** Скрыть превью и остановить WebGL-анимацию */
  window.__closeSolidPreview = function() {
    if (_rafId) { cancelAnimationFrame(_rafId); _rafId = null; }
    const p = document.getElementById('__se_preview_panel');
    if (p) p.style.display = 'none';
  };

  console.log('[sketch→solid bridge] зарегистрировано: __extrudeToSolid, __closeSolidPreview');

})();
"##;
