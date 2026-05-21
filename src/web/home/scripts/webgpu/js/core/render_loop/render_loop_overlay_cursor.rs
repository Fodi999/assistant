// ── Part 4: Precision CAD cursor — crosshair, snap marker, badge tooltip ──────

pub const JS: &str = r##"
          if (sketchState.hoverWorld &&
              (sketchState.activeTool === 'point' || sketchState.activeTool === 'line' || sketchState.activeTool === 'rect' || sketchState.activeTool === 'circle')) {
            const hw     = sketchState.hoverWorld;
            const prec   = !!sketchState.precisionMode;
            const cs     = sketchState.cursorSettings || {};
            const kind   = sketchState.snap.kind;
            const snapPt = kind === 'point';
            const snapFr = kind === 'free';
            let cx, cy;
            if (hw.screenX !== undefined) { cx = hw.screenX; cy = hw.screenY; }
            else {
              const _c = w2s(hw.x, hw.y, hw.z);
              if (!_c) { cx = null; } else { cx = _c.x; cy = _c.y; }
            }
            if (cx === null || cx === undefined) { /* skip */ } else {
            const _dpr       = window.devicePixelRatio || 1;
            const CROSS_ARM   = prec ? 14 : 10;
            const CROSS_GAP   = prec ?  4 :  3;
            const MARKER_SZ   = prec ?  4 :  3;
            const LBL_OX      = prec ? 48 : 34;
            const LBL_OY      = prec ? 32 : 26;
            const snapGrid = kind === 'grid';
            const SNAP_HIT_PX = 10;
            let onGridIntersection = false;
            if (snapGrid) {
              const _gs = w2s(hw.x, hw.y, hw.z);
              if (_gs) {
                const _sd = Math.hypot(cx - _gs.x, cy - _gs.y);
                onGridIntersection = _sd < SNAP_HIT_PX;
              }
            }
            const CROSS_COLOR = snapPt            ? 'rgba(16,185,129,0.95)'
                              : onGridIntersection ? 'rgba(250,255,80,1.00)'
                              : snapFr            ? 'rgba(203,213,225,0.50)'
                              :                     'rgba(103,232,249,0.70)';
            ctx.save();
            ctx.strokeStyle = CROSS_COLOR;
            ctx.lineWidth   = onGridIntersection ? 2 : 1;
            if (onGridIntersection) {
              ctx.shadowColor = 'rgba(250,255,80,0.75)';
              ctx.shadowBlur  = 6 * _dpr;
            }
            ctx.beginPath();
            ctx.moveTo(cx - CROSS_ARM, cy); ctx.lineTo(cx - CROSS_GAP, cy);
            ctx.moveTo(cx + CROSS_GAP, cy); ctx.lineTo(cx + CROSS_ARM, cy);
            ctx.moveTo(cx, cy - CROSS_ARM); ctx.lineTo(cx, cy - CROSS_GAP);
            ctx.moveTo(cx, cy + CROSS_GAP); ctx.lineTo(cx, cy + CROSS_ARM);
            ctx.stroke();
            ctx.shadowColor = 'transparent';
            ctx.shadowBlur  = 0;
            if (cs.showSnapMarker !== false) {
              const sm = MARKER_SZ;
              ctx.strokeStyle = CROSS_COLOR;
              ctx.lineWidth   = 1;
              if (snapPt) {
                ctx.strokeRect(cx - sm, cy - sm, sm * 2, sm * 2);
              } else if (onGridIntersection) {
                ctx.fillStyle = CROSS_COLOR;
                ctx.fillRect(cx - 2, cy - 2, 4, 4);
              }
            }
            if (prec) {
              ctx.beginPath();
              ctx.arc(cx, cy, CROSS_ARM + 4, 0, Math.PI * 2);
              ctx.strokeStyle = onGridIntersection ? 'rgba(250,255,80,0.30)' : 'rgba(103,232,249,0.25)';
              ctx.lineWidth = 1;
              ctx.stroke();
            }
            if (cs.showCoords !== false) {
              let badge;
              const previewPt = sketchState.line && sketchState.line.previewPoint;
              if (sketchState.orthoLock && previewPt && previewPt._orthoAxis) {
                badge = previewPt._orthoAxis;
              } else if (kind === 'point') { badge = 'POINT'; }
              else if (kind === 'grid')    { badge = 'GRID';  }
              else if (kind === 'free')    { badge = 'FREE';  }
              else                         { badge = null; }
              if (badge) {
                ctx.font = (prec ? '10px' : '9.5px') + ' "JetBrains Mono", system-ui, monospace';
                const tw   = ctx.measureText(badge).width;
                const boxW = tw + 10;
                const boxH = 16;
                const cw = ctx.canvas.width, ch = ctx.canvas.height;
                let lx = cx + LBL_OX;
                let ly = cy + LBL_OY;
                if (lx + boxW > cw - 12) lx = cx - LBL_OX - boxW;
                if (ly + boxH > ch - 12) ly = cy - LBL_OY - boxH;
                ctx.fillStyle   = 'rgba(10,14,26,0.88)';
                ctx.strokeStyle = snapPt          ? 'rgba(16,185,129,0.50)'
                                : sketchState.orthoLock ? 'rgba(251,191,36,0.55)'
                                :                  'rgba(56,189,248,0.35)';
                ctx.lineWidth = 1;
                const rr = 3;
                ctx.beginPath();
                ctx.moveTo(lx + rr, ly);
                ctx.lineTo(lx + boxW - rr, ly);        ctx.arcTo(lx + boxW, ly,          lx + boxW, ly + rr,          rr);
                ctx.lineTo(lx + boxW, ly + boxH - rr); ctx.arcTo(lx + boxW, ly + boxH,  lx + boxW - rr, ly + boxH,  rr);
                ctx.lineTo(lx + rr, ly + boxH);        ctx.arcTo(lx,        ly + boxH,  lx,          ly + boxH - rr, rr);
                ctx.lineTo(lx, ly + rr);               ctx.arcTo(lx,        ly,          lx + rr,    ly,             rr);
                ctx.closePath();
                ctx.fill(); ctx.stroke();
                ctx.fillStyle    = sketchState.orthoLock ? '#fbbf24'
                                 : snapPt               ? '#6ee7b7'
                                 :                        '#67e8f9';
                ctx.textAlign    = 'left';
                ctx.textBaseline = 'middle';
                ctx.fillText(badge, lx + 5, ly + boxH * 0.5);
              }
            }
            ctx.restore();
            } // end cx !== null
          }
"##;
