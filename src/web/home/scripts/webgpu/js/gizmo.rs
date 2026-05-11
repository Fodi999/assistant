// ── JS: axis orientation gizmo (top-right corner) ────────────────────────────────
// Domain: Scene navigation — 2D projected XYZ axes, click to snap camera view.
// Mirrors Blender's navigation gizmo: X=red, Y=green, Z=blue.

pub const JS: &str = r##"
      // ── Axis Gizmo ──────────────────────────────────────────────
      (function initGizmo() {
        const gc   = document.getElementById('axis-gizmo');
        if (!gc) return;
        const ctx  = gc.getContext('2d');
        const SIZE = 96;
        const CX   = SIZE / 2, CY = SIZE / 2;
        const R    = 34;   // length of each axis arm in px

        // Snap views: [yaw, pitch] in radians
        const SNAPS = {
          X:  [Math.PI / 2,  0],        // right (+X face)
          X_: [-Math.PI / 2, 0],        // left  (-X face)
          Y:  [0, -Math.PI / 2 + 0.01], // top   (+Y)
          Y_: [0,  Math.PI / 2 - 0.01], // bottom(-Y)
          Z:  [0,  0],                   // front (+Z)
          Z_: [Math.PI, 0],              // back  (-Z)
        };

        const AXES = [
          { id: 'X',  label: 'X', neg: 'X_',  color: '#e05555', neg_color: '#7a2222' },
          { id: 'Y',  label: 'Y', neg: 'Y_',  color: '#5dba5d', neg_color: '#2a5a2a' },
          { id: 'Z',  label: 'Z', neg: 'Z_',  color: '#4d87d6', neg_color: '#223366' },
        ];

        // Local axis directions in 3D (model space)
        const DIRS = {
          X:  [ 1, 0, 0], X_: [-1, 0, 0],
          Y:  [ 0, 1, 0], Y_: [ 0,-1, 0],
          Z:  [ 0, 0, 1], Z_: [ 0, 0,-1],
        };

        function project3d(v3) {
          // Project 3D axis dir using current cam yaw/pitch → 2D screen pos
          const cy = Math.cos(cam.yaw),   sy = Math.sin(cam.yaw);
          const cp = Math.cos(cam.pitch), sp = Math.sin(cam.pitch);
          // camera right, up, fwd
          const rx =  cy, ry = 0,   rz = sy;
          const ux = -sy * sp, uy = cp, uz = cy * sp;
          const [dx, dy, dz] = v3;
          const sx =  dx * rx + dy * 0 + dz * rz;   // dot(d, right)
          const sy2 = dx * ux + dy * uy + dz * uz;  // dot(d, up)
          return [CX + sx * R, CY - sy2 * R];
        }

        function drawGizmo() {
          ctx.clearRect(0, 0, SIZE, SIZE);

          // Collect all 6 tips with depth for painter-sort (back→front)
          const tips = [];
          for (const ax of AXES) {
            for (const key of [ax.neg, ax.id]) {
              const d = DIRS[key];
              const [px, py] = project3d(d);
              // depth = dot(d, fwd) to sort back-to-front
              const cy = Math.cos(cam.yaw), sy = Math.sin(cam.yaw);
              const cp = Math.cos(cam.pitch);
              const fwdX = -sy * cp, fwdY = -Math.sin(cam.pitch), fwdZ = cy * cp;
              const depth = d[0]*fwdX + d[1]*fwdY + d[2]*fwdZ;
              const isPos = key === ax.id;
              tips.push({ key, px, py, depth, isPos,
                color:   isPos ? ax.color     : ax.neg_color,
                label:   isPos ? ax.label      : '',
                opacity: isPos ? 1.0           : 0.45,
              });
            }
          }
          tips.sort((a, b) => a.depth - b.depth);  // back first

          // Draw lines then dots
          for (const t of tips) {
            ctx.globalAlpha = t.opacity;
            ctx.strokeStyle = t.color;
            ctx.lineWidth   = t.isPos ? 2.5 : 1.5;
            ctx.beginPath();
            ctx.moveTo(CX, CY);
            ctx.lineTo(t.px, t.py);
            ctx.stroke();
          }
          for (const t of tips) {
            const dotR = t.isPos ? 7 : 4;
            ctx.globalAlpha = t.opacity;
            ctx.fillStyle   = t.color;
            ctx.beginPath();
            ctx.arc(t.px, t.py, dotR, 0, Math.PI * 2);
            ctx.fill();

            if (t.label) {
              ctx.globalAlpha = 1.0;
              ctx.fillStyle   = '#fff';
              ctx.font        = 'bold 10px Inter, system-ui, sans-serif';
              ctx.textAlign   = 'center';
              ctx.textBaseline = 'middle';
              ctx.fillText(t.label, t.px, t.py);
            }
          }
          ctx.globalAlpha = 1.0;
        }

        // Hit-test: find which axis dot was clicked
        function hitTest(ex, ey) {
          const rect = gc.getBoundingClientRect();
          const mx = (ex - rect.left) * (SIZE / rect.width);
          const my = (ey - rect.top)  * (SIZE / rect.height);
          for (const ax of AXES) {
            for (const key of [ax.id, ax.neg]) {
              const [px, py] = project3d(DIRS[key]);
              const r = key === ax.id ? 10 : 7;
              if (Math.hypot(mx - px, my - py) < r) return key;
            }
          }
          return null;
        }

        gc.addEventListener('click', (e) => {
          // Ignore if drag happened (moved > 4px)
          if (gc._dragMoved) return;
          const hit = hitTest(e.clientX, e.clientY);
          if (!hit) return;
          const [ty, tp] = SNAPS[hit];

          // In Sketch Mode: clicking X/Y/Z switches sketch plane (not just camera).
          const inSketch = window.editorState && window.editorState.mode === 'sketch';
          if (inSketch && window.__setSketchPlane) {
            // Map axis = normal of plane → plane name
            const planeFor = { Y:'XZ', 'Y_':'XZ', Z:'XY', 'Z_':'XY', X:'YZ', 'X_':'YZ' };
            const pl = planeFor[hit];
            if (pl) {
              window.__setSketchPlane(pl);
              return; // setSketchPlane already handles camera
            }
          }

          // Smooth snap via micro-animation
          const startYaw   = cam.yaw;
          const startPitch = cam.pitch;
          let dyaw   = ty - startYaw;
          // take shorter arc
          while (dyaw >  Math.PI) dyaw -= Math.PI * 2;
          while (dyaw < -Math.PI) dyaw += Math.PI * 2;
          const dpitch = tp - startPitch;
          const dur    = 320; // ms
          const t0     = performance.now();
          function tick(now) {
            const frac = Math.min((now - t0) / dur, 1);
            const ease = 1 - Math.pow(1 - frac, 3);  // cubic ease-out
            cam.yaw   = startYaw   + dyaw   * ease;
            cam.pitch = startPitch + dpitch * ease;
            if (frac < 1) requestAnimationFrame(tick);
          }
          requestAnimationFrame(tick);
        });

        // ── Drag-to-orbit on the gizmo ────────────────────────────
        // Same feel as main canvas orbit but scoped to the gizmo widget.
        // Sensitivity scaled so a full 96px drag ≈ 180° rotation.
        let gizmoDragging = false;
        let gizmoLastX = 0, gizmoLastY = 0;
        gc._dragMoved = false;

        gc.addEventListener('pointerdown', (e) => {
          gizmoDragging = true;
          gc._dragMoved = false;
          gizmoLastX = e.clientX;
          gizmoLastY = e.clientY;
          gc.classList.add('dragging');
          gc.setPointerCapture(e.pointerId);
          e.stopPropagation();
        });

        gc.addEventListener('pointermove', (e) => {
          if (!gizmoDragging) return;
          const dx = e.clientX - gizmoLastX;
          const dy = e.clientY - gizmoLastY;
          if (Math.hypot(dx, dy) > 4) gc._dragMoved = true;
          gizmoLastX = e.clientX;
          gizmoLastY = e.clientY;
          // Sensitivity: 360° per ~600px drag (same as main canvas orbit 0.005 * px)
          cam.yaw   += dx * 0.012;
          cam.pitch += dy * 0.012;
          // Clamp pitch to avoid gimbal flip
          cam.pitch = Math.max(-Math.PI / 2 + 0.05, Math.min(Math.PI / 2 - 0.05, cam.pitch));
          e.stopPropagation();
        });

        gc.addEventListener('pointerup', (e) => {
          gizmoDragging = false;
          gc.classList.remove('dragging');
          try { gc.releasePointerCapture(e.pointerId); } catch {}
          e.stopPropagation();
        });

        gc.addEventListener('pointerleave', () => {
          if (!gizmoDragging) gc._dragMoved = false;
        });

        // Redraw every frame (cheap 2D canvas, ~0.1 ms)
        function gizmoLoop() {
          drawGizmo();
          requestAnimationFrame(gizmoLoop);
        }
        requestAnimationFrame(gizmoLoop);
      })();
"##;
