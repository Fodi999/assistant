// ── View Cube ─────────────────────────────────────────────────────────────────
// Рисует мини-куб ориентации в правом верхнем углу canvas (как в SketchUp/Blender).
// Показывает текущие оси и подписи X/Y/Z / Front/Top/Right.
// Клик по грани — анимирует камеру к соответствующему пресету.

pub const JS: &str = r##"
  (function() {
    if (window.__viewCubeInited) return;
    window.__viewCubeInited = true;

    // Размер куба (CSS px)
    const SIZE = 80;
    const MARGIN = 12;

    // Создаём canvas поверх основного
    var vc = document.createElement('canvas');
    vc.id = '__view-cube';
    vc.width  = SIZE;
    vc.height = SIZE;
    vc.style.cssText = [
      'position:absolute',
      'top:'    + MARGIN + 'px',
      'right:'  + MARGIN + 'px',
      'width:'  + SIZE + 'px',
      'height:' + SIZE + 'px',
      'cursor:pointer',
      'border-radius:6px',
      'z-index:50',
      'pointer-events:auto',
    ].join(';');

    // Ждём родительский контейнер
    function _mount() {
      var wrap = document.getElementById('webgpu-wrap') ||
                 document.querySelector('.webgpu-container') ||
                 document.body;
      wrap.appendChild(vc);
    }
    if (document.readyState === 'loading') {
      document.addEventListener('DOMContentLoaded', _mount);
    } else {
      _mount();
    }

    var ctx2 = vc.getContext('2d');

    // ── Матрица вращения из yaw/pitch ──────────────────────────────
    function _rotMat(yaw, pitch) {
      var cy = Math.cos(yaw),  sy = Math.sin(yaw);
      var cp = Math.cos(pitch), sp = Math.sin(pitch);
      // Row vectors of camera rotation matrix (column-major world→view)
      // right = [cy, 0, sy]
      // up    = [sy*sp, cp, -cy*sp]
      // fwd   = [-sy*cp, sp, cy*cp]
      return {
        rx: [cy,      0,   sy],
        ux: [sy*sp,   cp, -cy*sp],
        fx: [-sy*cp,  sp,  cy*cp],
      };
    }

    // Project world point → cube canvas 2D (center = SIZE/2)
    function _proj(wx, wy, wz, R) {
      var vx =  wx*R.rx[0] + wy*R.rx[1] + wz*R.rx[2];
      var vy = -(wx*R.ux[0] + wy*R.ux[1] + wz*R.ux[2]);
      var s  = SIZE * 0.36; // scale
      return { x: SIZE/2 + vx*s, y: SIZE/2 + vy*s };
    }

    // Cube faces: [normal_xyz, label, colorFill, colorStroke]
    var FACES = [
      { n:[1,0,0],  label:'RIGHT', fill:'rgba(220,60,60,0.75)',   stroke:'#ef4444' },
      { n:[-1,0,0], label:'LEFT',  fill:'rgba(180,40,40,0.45)',   stroke:'#f87171' },
      { n:[0,1,0],  label:'TOP',   fill:'rgba(60,200,60,0.75)',   stroke:'#22c55e' },
      { n:[0,-1,0], label:'BOT',   fill:'rgba(40,140,40,0.45)',   stroke:'#4ade80' },
      { n:[0,0,1],  label:'FRONT', fill:'rgba(60,120,220,0.75)',  stroke:'#3b82f6' },
      { n:[0,0,-1], label:'BACK',  fill:'rgba(40,80,160,0.45)',   stroke:'#60a5fa' },
    ];

    // Unit cube corner coords (x,y,z each ±0.5)
    var CORNERS = [
      [-1,-1,-1],[-1,-1,1],[-1,1,-1],[-1,1,1],
      [1,-1,-1], [1,-1,1], [1,1,-1], [1,1,1],
    ];

    // Face → 4 corner indices
    var FACE_CORNERS = [
      [4,5,7,6], // +X right
      [0,2,3,1], // -X left
      [2,6,7,3], // +Y top
      [0,1,5,4], // -Y bottom
      [1,3,7,5], // +Z front
      [0,4,6,2], // -Z back
    ];

    // Axis labels on the three positive axes
    var AXES = [
      { p:[1.15,0,0], label:'X', color:'#ef4444' },
      { p:[0,1.15,0], label:'Y', color:'#22c55e' },
      { p:[0,0,1.15], label:'Z', color:'#3b82f6' },
    ];

    // ── Hit test: which face under px (cx,cy)? ──────────────────
    function _hitFace(cx, cy) {
      if (!window.cam) return null;
      var R   = _rotMat(cam.yaw, cam.pitch);
      var fwd = [cam.yaw, cam.pitch]; // just for sorting

      // Sort faces back-to-front by dot with view direction
      var sorted = FACES.map(function(f,i) {
        var dot = f.n[0]*R.fx[0] + f.n[1]*R.fx[1] + f.n[2]*R.fx[2];
        return { i:i, f:f, dot:dot };
      }).sort(function(a,b) { return a.dot - b.dot; }); // back first

      // Test front faces (dot > 0 = facing camera)
      for (var k = sorted.length-1; k >= 0; k--) {
        var fi = sorted[k];
        if (fi.dot <= 0) continue;
        var ci = FACE_CORNERS[fi.i];
        var pts = ci.map(function(idx) {
          var c = CORNERS[idx];
          return _proj(c[0]*0.72, c[1]*0.72, c[2]*0.72, R);
        });
        // Point-in-polygon test
        var inside = false;
        for (var a=0,b=pts.length-1; a<pts.length; b=a++) {
          var xi=pts[a].x,yi=pts[a].y,xj=pts[b].x,yj=pts[b].y;
          if (((yi>cy)!==(yj>cy)) && cx < (xj-xi)*(cy-yi)/(yj-yi)+xi) inside=!inside;
        }
        if (inside) return fi.f;
      }
      return null;
    }

    // ── Click → animate camera ──────────────────────────────────
    vc.addEventListener('click', function(e) {
      if (!window.cam) return;
      var rect = vc.getBoundingClientRect();
      var cx = (e.clientX - rect.left) * (SIZE / rect.width);
      var cy = (e.clientY - rect.top)  * (SIZE / rect.height);
      var face = _hitFace(cx, cy);
      if (!face) return;

      var n = face.n;
      var targetYaw, targetPitch;

      if      (n[1] >=  0.9) { targetYaw = cam.yaw; targetPitch = -Math.PI*0.5+0.001; } // Top
      else if (n[1] <= -0.9) { targetYaw = cam.yaw; targetPitch =  Math.PI*0.5-0.001; } // Bottom
      else if (n[0] >=  0.9) { targetYaw = Math.PI*0.5; targetPitch = 0; }              // Right
      else if (n[0] <= -0.9) { targetYaw = -Math.PI*0.5; targetPitch = 0; }             // Left
      else if (n[2] >=  0.9) { targetYaw = 0;            targetPitch = 0; }             // Front
      else                   { targetYaw = Math.PI;      targetPitch = 0; }             // Back

      // Animate
      var FRAMES = 20;
      var y0 = cam.yaw, p0 = cam.pitch;
      var dy = ((targetYaw - y0) % (Math.PI*2) + Math.PI*3) % (Math.PI*2) - Math.PI;
      var dp = targetPitch - p0;
      var f = 0;
      function _step() {
        f++;
        var ease = 1 - Math.pow(1 - f/FRAMES, 3);
        cam.yaw   = y0 + dy * ease;
        cam.pitch = p0 + dp * ease;
        if (f < FRAMES) requestAnimationFrame(_step);
      }
      requestAnimationFrame(_step);
    });

    // ── Hover state ─────────────────────────────────────────────
    var _hoverFace = null;
    vc.addEventListener('mousemove', function(e) {
      var rect = vc.getBoundingClientRect();
      var cx = (e.clientX - rect.left) * (SIZE / rect.width);
      var cy = (e.clientY - rect.top)  * (SIZE / rect.height);
      var f = _hitFace(cx, cy);
      if (f !== _hoverFace) { _hoverFace = f; }
    });
    vc.addEventListener('mouseleave', function() { _hoverFace = null; });

    // ── Draw loop ────────────────────────────────────────────────
    function _draw() {
      requestAnimationFrame(_draw);
      if (!window.cam) return;

      ctx2.clearRect(0, 0, SIZE, SIZE);

      var R = _rotMat(cam.yaw, cam.pitch);

      // Sort faces back-to-front
      var sorted = FACES.map(function(f,i) {
        var dot = f.n[0]*R.fx[0] + f.n[1]*R.fx[1] + f.n[2]*R.fx[2];
        return { i:i, f:f, dot:dot };
      }).sort(function(a,b) { return a.dot - b.dot; });

      // Draw each face
      for (var k = 0; k < sorted.length; k++) {
        var fi = sorted[k];
        var facing = fi.dot > 0;
        var ci = FACE_CORNERS[fi.i];
        var pts = ci.map(function(idx) {
          var c = CORNERS[idx];
          return _proj(c[0]*0.72, c[1]*0.72, c[2]*0.72, R);
        });

        ctx2.save();
        ctx2.beginPath();
        ctx2.moveTo(pts[0].x, pts[0].y);
        for (var j=1;j<pts.length;j++) ctx2.lineTo(pts[j].x, pts[j].y);
        ctx2.closePath();

        var isHover = facing && _hoverFace === fi.f;
        if (facing) {
          ctx2.fillStyle = isHover ? fi.f.fill.replace(/[\d.]+\)$/, '0.95)') : fi.f.fill;
          ctx2.fill();
          ctx2.strokeStyle = fi.f.stroke;
          ctx2.lineWidth = isHover ? 2 : 1;
          ctx2.stroke();
          // Label on front-facing
          if (fi.dot > 0.3) {
            var cx2 = pts.reduce(function(s,p){return s+p.x;},0)/pts.length;
            var cy2 = pts.reduce(function(s,p){return s+p.y;},0)/pts.length;
            ctx2.fillStyle = '#fff';
            ctx2.font = 'bold 8px system-ui,sans-serif';
            ctx2.textAlign = 'center';
            ctx2.textBaseline = 'middle';
            ctx2.fillText(fi.f.label, cx2, cy2);
          }
        } else {
          // Back face — subtle outline only
          ctx2.strokeStyle = 'rgba(255,255,255,0.08)';
          ctx2.lineWidth = 0.5;
          ctx2.stroke();
        }
        ctx2.restore();
      }

      // Draw axis lines and labels
      var origin = _proj(0,0,0,R);
      var axLen  = 0.85;
      var AXIS_DIRS = [
        { v:[1,0,0], label:'X', color:'#ef4444' },
        { v:[0,1,0], label:'Y', color:'#22c55e' },
        { v:[0,0,1], label:'Z', color:'#3b82f6' },
      ];
      for (var a = 0; a < AXIS_DIRS.length; a++) {
        var ax  = AXIS_DIRS[a];
        var tip = _proj(ax.v[0]*axLen, ax.v[1]*axLen, ax.v[2]*axLen, R);
        ctx2.save();
        ctx2.strokeStyle = ax.color;
        ctx2.lineWidth = 1.5;
        ctx2.beginPath();
        ctx2.moveTo(origin.x, origin.y);
        ctx2.lineTo(tip.x, tip.y);
        ctx2.stroke();
        ctx2.fillStyle = ax.color;
        ctx2.font = 'bold 9px system-ui,monospace';
        ctx2.textAlign = 'center';
        ctx2.textBaseline = 'middle';
        ctx2.fillText(ax.label, tip.x + (tip.x-origin.x)*0.25, tip.y + (tip.y-origin.y)*0.25);
        ctx2.restore();
      }

      // Ortho badge
      if (window.cam && cam.ortho) {
        ctx2.save();
        ctx2.fillStyle = 'rgba(250,204,21,0.85)';
        ctx2.font = 'bold 7px system-ui,sans-serif';
        ctx2.textAlign = 'center';
        ctx2.textBaseline = 'bottom';
        ctx2.fillText('ORTHO', SIZE/2, SIZE - 3);
        ctx2.restore();
      }
    }
    requestAnimationFrame(_draw);

  })();
"##;
