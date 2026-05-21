// ── Part 3: Sketch geometry — profiles, walls, extrude preview, edges, points ──

pub const JS: &str = r##"
          const pById = new Map(sketchState.points.map(p => [p.id, p]));
          const eById = new Map(sketchState.edges.map(e  => [e.id, e]));

          // ── Closed profile fills ──
          // Задача 5: не рисовать sketch-заливку поверх committed solid.
          // Если solid зафиксирован (lastSolidResult) и гизмо не активен → скрываем fill.
          // Управляется через window.__debugSolidRender.drawSketchFill (Задача 4).
          const _dsr      = window.__debugSolidRender;
          const _hasSolid = window.__lastSolidResult != null
                         && !(window.__solidExtrudeState && window.__solidExtrudeState.active);
          const _drawSketchFill = _dsr
            ? (_dsr.drawSketchFill)                       // явное управление через debug object
            : (!_hasSolid);                               // авто: скрыть если solid committed
          if (_drawSketchFill && sketchState.profiles && sketchState.profiles.length) {
            for (const prof of sketchState.profiles) {
              const ringPts = prof.pointIds.map(id => pById.get(id)).filter(Boolean);
              if (ringPts.length < 3) continue;
              const screenPts = ringPts.map(p => w2s(p.x, p.y, p.z));
              if (screenPts.some(s => !s)) continue;
              const isSelected = sketchState.selectedProfileId === prof.id;
              const isHover    = !isSelected && sketchState.hoverProfileId === prof.id;
              const isFullySelected = prof.edgeIds.every(eid => sketchState.selectedEdgeIds.has(eid));
              ctx.save();
              ctx.beginPath();
              ctx.moveTo(screenPts[0].x, screenPts[0].y);
              for (let i = 1; i < screenPts.length; i++) ctx.lineTo(screenPts[i].x, screenPts[i].y);
              ctx.closePath();
              if (isSelected) {
                ctx.fillStyle   = 'rgba(251,146,60,0.22)';
                ctx.strokeStyle = 'rgba(251,146,60,0.90)';
                ctx.lineWidth   = 2.0;
              } else if (isHover) {
                ctx.fillStyle   = 'rgba(56,189,248,0.20)';
                ctx.strokeStyle = 'rgba(56,189,248,0.75)';
                ctx.lineWidth   = 1.6;
              } else if (isFullySelected) {
                ctx.fillStyle   = 'rgba(251,146,60,0.18)';
                ctx.strokeStyle = 'rgba(251,146,60,0.55)';
                ctx.lineWidth   = 1.2;
              } else {
                ctx.fillStyle   = 'rgba(56,189,248,0.06)';
                ctx.strokeStyle = 'rgba(56,189,248,0.25)';
                ctx.lineWidth   = 1.0;
              }
              ctx.fill();
              ctx.stroke();
              ctx.restore();
            }
          }  // end _drawSketchFill

          // ── Wall Surfaces (Edge Extrude) ──
          if (sketchState.wallSurfaces && sketchState.wallSurfaces.length) {
            for (const wall of sketchState.wallSurfaces) {
              const sBA = w2s(wall.bottomA.x, wall.bottomA.y, wall.bottomA.z);
              const sBB = w2s(wall.bottomB.x, wall.bottomB.y, wall.bottomB.z);
              const sTA = w2s(wall.topA.x,    wall.topA.y,    wall.topA.z);
              const sTB = w2s(wall.topB.x,    wall.topB.y,    wall.topB.z);
              if (!sBA || !sBB || !sTA || !sTB) continue;
              ctx.save();
              ctx.beginPath();
              ctx.moveTo(sBA.x, sBA.y);
              ctx.lineTo(sBB.x, sBB.y);
              ctx.lineTo(sTB.x, sTB.y);
              ctx.lineTo(sTA.x, sTA.y);
              ctx.closePath();
              ctx.fillStyle = 'rgba(255,170,40,0.10)';
              ctx.fill();
              ctx.strokeStyle = 'rgba(255,180,40,0.75)';
              ctx.lineWidth   = 1.8;
              ctx.setLineDash([]);
              ctx.beginPath(); ctx.moveTo(sTA.x, sTA.y); ctx.lineTo(sTB.x, sTB.y); ctx.stroke();
              ctx.beginPath(); ctx.moveTo(sBA.x, sBA.y); ctx.lineTo(sTA.x, sTA.y); ctx.stroke();
              ctx.beginPath(); ctx.moveTo(sBB.x, sBB.y); ctx.lineTo(sTB.x, sTB.y); ctx.stroke();
              ctx.restore();
            }
          }

          // ── Extrude Live Preview ──
          if (sketchState.extrude && sketchState.extrude.active
              && sketchState.extrude.edgeIds && sketchState.extrude.edgeIds.length) {
            const ex      = sketchState.extrude;
            const inp     = document.getElementById('__extrude-modal-input');
            const heightMm = parseFloat(inp ? inp.value : (ex.heightInput || '0')) || 0;
            const heightM  = heightMm / 1000;
            const plane    = sketchState.workingPlane || 'XZ';
            const dir      = window.__getExtrudeDir ? window.__getExtrudeDir(plane) : { x:0, y:1, z:0 };
            if (heightM > 0.0001) {
              for (const edgeId of ex.edgeIds) {
                const edge = sketchState.edges.find(e => e.id === edgeId);
                if (!edge) continue;
                const pA = pById.get(edge.a), pB = pById.get(edge.b);
                if (!pA || !pB) continue;
                const sBA = w2s(pA.x, pA.y, pA.z);
                const sBB = w2s(pB.x, pB.y, pB.z);
                const sTA = w2s(pA.x + dir.x * heightM, pA.y + dir.y * heightM, pA.z + dir.z * heightM);
                const sTB = w2s(pB.x + dir.x * heightM, pB.y + dir.y * heightM, pB.z + dir.z * heightM);
                if (!sBA || !sBB || !sTA || !sTB) continue;
                ctx.save();
                const dragging = window.__extrudeGizmoDrag;
                ctx.beginPath();
                ctx.moveTo(sBA.x, sBA.y);
                ctx.lineTo(sBB.x, sBB.y);
                ctx.lineTo(sTB.x, sTB.y);
                ctx.lineTo(sTA.x, sTA.y);
                ctx.closePath();
                ctx.fillStyle = dragging ? 'rgba(255,200,60,0.22)' : 'rgba(255,170,40,0.14)';
                ctx.fill();
                ctx.strokeStyle = dragging ? 'rgba(255,220,80,0.95)' : 'rgba(255,180,40,0.85)';
                ctx.lineWidth   = 2;
                ctx.setLineDash([6, 3]);
                ctx.beginPath(); ctx.moveTo(sTA.x, sTA.y); ctx.lineTo(sTB.x, sTB.y); ctx.stroke();
                ctx.beginPath(); ctx.moveTo(sBA.x, sBA.y); ctx.lineTo(sTA.x, sTA.y); ctx.stroke();
                ctx.beginPath(); ctx.moveTo(sBB.x, sBB.y); ctx.lineTo(sTB.x, sTB.y); ctx.stroke();
                ctx.setLineDash([]);
                ctx.restore();
              }
            }
          }

          // ── Edges ──
          // Если solid зафиксирован и гизмо не активен — не рисуем рёбра профиля
          // (они создают white-line артефакт поверх 3D solid).
          // Управляется через window.__debugSolidRender.drawSketchFill (тот же флаг).
          const _showEdges = _drawSketchFill;
          const grabPointSet = sketchState.grab.active
            ? new Set(sketchState.grab.pointIds)
            : null;
          if (_showEdges) {
          for (const e of sketchState.edges) {
            const a = pById.get(e.a), b = pById.get(e.b);
            if (!a || !b) continue;
            const sa = w2s(a.x, a.y, a.z), sb = w2s(b.x, b.y, b.z);
            if (!sa || !sb) continue;
            const isHover = sketchState.hoverEdgeId === e.id;
            const isSel   = sketchState.selectedEdgeIds.has(e.id);
            const isGrab  = grabPointSet && grabPointSet.has(e.a) && grabPointSet.has(e.b);
            const edgeCol = isGrab ? '#facc15' : isSel ? '#fb923c' : isHover ? '#facc15' : null;
            const kind    = e.kind || 'normal';
            ctx.save();
            if (kind === 'construction') {
              ctx.strokeStyle = edgeCol || '#67e8f9';
              ctx.lineWidth   = (isSel || isGrab) ? 1.6 : 1.2;
              ctx.setLineDash([4, 3]);
            } else if (kind === 'hidden') {
              ctx.strokeStyle = edgeCol || '#94a3b8';
              ctx.lineWidth   = (isSel || isGrab) ? 1.8 : 1.4;
              ctx.setLineDash([6, 4]);
            } else {
              ctx.strokeStyle = edgeCol || '#cbd5e1';
              ctx.lineWidth   = (isSel || isGrab) ? 3.0 : isHover ? 2.5 : 2.0;
            }
            ctx.beginPath();
            ctx.moveTo(sa.x, sa.y);
            ctx.lineTo(sb.x, sb.y);
            ctx.stroke();
            ctx.restore();

            // ── Constraint badges ──
            {
              const eCons = sketchState.constraints.filter(c => c.targetId === e.id);
              const iconMap = {
                HORIZONTAL:   { glyph: 'H', color: '#a78bfa' },
                VERTICAL:     { glyph: 'V', color: '#a78bfa' },
                FIXED_LENGTH: { glyph: 'L', color: '#34d399' },
                EQUAL:        { glyph: '=', color: '#fbbf24' },
                EQUAL_LENGTH: { glyph: '=', color: '#fbbf24' },
                PERPENDICULAR:{ glyph: '⊥', color: '#f472b6' },
                PARALLEL:     { glyph: '∥', color: '#f472b6' },
                MIDPOINT:     { glyph: '◇', color: '#38bdf8' },
                COINCIDENT:   { glyph: '●', color: '#fb923c' },
                EDGE_LENGTH:  { glyph: 'L', color: '#34d399' },
              };
              if (eCons.length > 0) {
                const mx = (sa.x + sb.x) * 0.5;
                const my = (sa.y + sb.y) * 0.5;
                const dx = sb.x - sa.x, dy = sb.y - sa.y;
                const len = Math.sqrt(dx*dx + dy*dy) || 1;
                let nx = -dy / len, ny = dx / len;
                if (ny > 0) { nx = -nx; ny = -ny; }
                const BADGE_W = 16, BADGE_H = 16, GAP = 4;
                const totalW  = eCons.length * BADGE_W + (eCons.length - 1) * GAP;
                const PERP_DIST = 18;
                const bx0 = mx + nx * PERP_DIST - totalW * 0.5;
                const by0 = my + ny * PERP_DIST - BADGE_H * 0.5;
                ctx.save();
                ctx.font = 'bold 10px "JetBrains Mono", system-ui, monospace';
                ctx.textAlign = 'center';
                ctx.textBaseline = 'middle';
                eCons.forEach((c, i) => {
                  const info = iconMap[(c.type || '').toUpperCase()] || { glyph: '?', color: '#64748b' };
                  const bx = bx0 + i * (BADGE_W + GAP);
                  const by = by0;
                  const radius = 4;
                  ctx.beginPath();
                  ctx.roundRect
                    ? ctx.roundRect(bx, by, BADGE_W, BADGE_H, radius)
                    : (ctx.rect(bx, by, BADGE_W, BADGE_H));
                  ctx.fillStyle = 'rgba(15,23,42,0.88)';
                  ctx.fill();
                  ctx.strokeStyle = info.color;
                  ctx.lineWidth = 1.2;
                  ctx.stroke();
                  ctx.fillStyle = info.color;
                  ctx.fillText(info.glyph, bx + BADGE_W * 0.5, by + BADGE_H * 0.5);
                });
                ctx.restore();
              }

              // ── CAD Dimension line ──
              const dimC = window.__getEdgeLengthConstraint && window.__getEdgeLengthConstraint(e.id);
              if (dimC && dimC.value != null) {
                const edx = sb.x - sa.x, edy = sb.y - sa.y;
                const edgeScr = Math.sqrt(edx*edx + edy*edy) || 1;
                const ux = edx / edgeScr, uy = edy / edgeScr;
                let nx = -uy, ny = ux;
                if (ny > 0) { nx = -nx; ny = -ny; }
                const OFFSET = 32, EXT_OVR = 5, ARROW = 7, ARROW_W = 3;
                const d1x = sa.x + nx * OFFSET, d1y = sa.y + ny * OFFSET;
                const d2x = sb.x + nx * OFFSET, d2y = sb.y + ny * OFFSET;
                ctx.save();
                ctx.strokeStyle = '#34d399';
                ctx.fillStyle   = '#34d399';
                ctx.lineWidth   = 1.3;
                ctx.setLineDash([]);
                ctx.beginPath();
                ctx.moveTo(sa.x + nx * 5, sa.y + ny * 5);
                ctx.lineTo(d1x + nx * EXT_OVR, d1y + ny * EXT_OVR);
                ctx.stroke();
                ctx.beginPath();
                ctx.moveTo(sb.x + nx * 5, sb.y + ny * 5);
                ctx.lineTo(d2x + nx * EXT_OVR, d2y + ny * EXT_OVR);
                ctx.stroke();
                if (edgeScr >= ARROW * 3) {
                  ctx.beginPath();
                  ctx.moveTo(d1x, d1y); ctx.lineTo(d2x, d2y); ctx.stroke();
                  ctx.beginPath(); ctx.moveTo(d1x, d1y);
                  ctx.lineTo(d1x + ux*ARROW + ny*ARROW_W, d1y + uy*ARROW - nx*ARROW_W);
                  ctx.lineTo(d1x + ux*ARROW - ny*ARROW_W, d1y + uy*ARROW + nx*ARROW_W);
                  ctx.closePath(); ctx.fill();
                  ctx.beginPath(); ctx.moveTo(d2x, d2y);
                  ctx.lineTo(d2x - ux*ARROW + ny*ARROW_W, d2y - uy*ARROW - nx*ARROW_W);
                  ctx.lineTo(d2x - ux*ARROW - ny*ARROW_W, d2y - uy*ARROW + nx*ARROW_W);
                  ctx.closePath(); ctx.fill();
                } else {
                  ctx.beginPath(); ctx.moveTo(d1x - ux*20, d1y - uy*20); ctx.lineTo(d1x, d1y); ctx.stroke();
                  ctx.beginPath(); ctx.moveTo(d2x + ux*20, d2y + uy*20); ctx.lineTo(d2x, d2y); ctx.stroke();
                  ctx.beginPath(); ctx.moveTo(d1x, d1y);
                  ctx.lineTo(d1x - ux*ARROW + ny*ARROW_W, d1y - uy*ARROW - nx*ARROW_W);
                  ctx.lineTo(d1x - ux*ARROW - ny*ARROW_W, d1y - uy*ARROW + nx*ARROW_W);
                  ctx.closePath(); ctx.fill();
                  ctx.beginPath(); ctx.moveTo(d2x, d2y);
                  ctx.lineTo(d2x + ux*ARROW + ny*ARROW_W, d2y + uy*ARROW - nx*ARROW_W);
                  ctx.lineTo(d2x + ux*ARROW - ny*ARROW_W, d2y + uy*ARROW + nx*ARROW_W);
                  ctx.closePath(); ctx.fill();
                }
                const dmx = (d1x + d2x) * 0.5, dmy = (d1y + d2y) * 0.5;
                const txt = Number(dimC.value).toFixed(1) + ' мм';
                ctx.font = 'bold 11px "JetBrains Mono", system-ui, monospace';
                const tw = ctx.measureText(txt).width + 8;
                ctx.fillStyle = 'rgba(10,20,35,0.93)';
                const ph = 16, pr = 4;
                ctx.beginPath();
                ctx.roundRect
                  ? ctx.roundRect(dmx - tw/2, dmy - ph/2, tw, ph, pr)
                  : ctx.rect(dmx - tw/2, dmy - ph/2, tw, ph);
                ctx.fill();
                ctx.strokeStyle = '#34d399'; ctx.lineWidth = 1; ctx.stroke();
                ctx.fillStyle = '#34d399';
                ctx.textAlign = 'center'; ctx.textBaseline = 'middle';
                ctx.fillText(txt, dmx, dmy);
                ctx.restore();
              }
            }
          }
          }  // end _showEdges

          // ── Line tool preview ──
          if (sketchState.activeTool === 'line' && sketchState.line.startPointId) {
            const anchor = pById.get(sketchState.line.startPointId);
            const target = sketchState.line.previewPoint;
            if (anchor && target) {
              const sa = w2s(anchor.x, anchor.y, anchor.z);
              const sb = w2s(target.x, target.y, target.z);
              if (sa && sb) {
                const valid = sketchState.line.previewValid;
                ctx.save();
                ctx.setLineDash([6, 4]);
                ctx.strokeStyle = valid ? 'rgba(56,189,248,0.85)' : 'rgba(239,68,68,0.85)';
                ctx.lineWidth = 2;
                ctx.beginPath();
                ctx.moveTo(sa.x, sa.y); ctx.lineTo(sb.x, sb.y);
                ctx.stroke();
                ctx.restore();
                const len = sketchState.line.previewLength || 0;
                const txt = window.__fmtLength ? window.__fmtLength(len) : (len * 1000).toFixed(1) + ' mm';
                const mx = (sa.x + sb.x) * 0.5;
                const my = (sa.y + sb.y) * 0.5;
                ctx.font = '12px "JetBrains Mono", system-ui, monospace';
                const w = ctx.measureText(txt).width + 10;
                ctx.fillStyle = 'rgba(15,23,42,0.9)';
                ctx.fillRect(mx - w/2, my - 22, w, 18);
                ctx.fillStyle = valid ? '#38bdf8' : '#ef4444';
                ctx.textAlign = 'center'; ctx.textBaseline = 'middle';
                ctx.fillText(txt, mx, my - 13);
              }
            }
          }

          // ── Rect tool preview ──
          if (sketchState.activeTool === 'rect' && sketchState.rect && sketchState.rect.active && sketchState.rect.startSnap && sketchState.hoverWorld) {
            const g1   = sketchState.rect.startSnap;
            const hw   = sketchState.hoverWorld;
            const gs   = sketchState.gridSize || 1.0;
            const plane = sketchState.workingPlane || 'XZ';
            let c0, c1, c2, c3;
            const g2gx = Math.round(hw.gx !== undefined ? hw.gx : hw.x / gs);
            const g2gy = Math.round(hw.gy !== undefined ? hw.gy : hw.y / gs);
            const g2gz = Math.round(hw.gz !== undefined ? hw.gz : hw.z / gs);
            if (plane === 'XZ') {
              c0 = { x: g1.gx*gs, y: 0, z: g1.gz*gs };
              c1 = { x: g2gx*gs, y: 0, z: g1.gz*gs };
              c2 = { x: g2gx*gs, y: 0, z: g2gz*gs };
              c3 = { x: g1.gx*gs, y: 0, z: g2gz*gs };
            } else if (plane === 'XY') {
              c0 = { x: g1.gx*gs, y: g1.gy*gs, z: 0 };
              c1 = { x: g2gx*gs, y: g1.gy*gs, z: 0 };
              c2 = { x: g2gx*gs, y: g2gy*gs, z: 0 };
              c3 = { x: g1.gx*gs, y: g2gy*gs, z: 0 };
            } else {
              c0 = { x: 0, y: g1.gy*gs, z: g1.gz*gs };
              c1 = { x: 0, y: g1.gy*gs, z: g2gz*gs };
              c2 = { x: 0, y: g2gy*gs, z: g2gz*gs };
              c3 = { x: 0, y: g2gy*gs, z: g1.gz*gs };
            }
            const sc0 = w2s(c0.x,c0.y,c0.z), sc1 = w2s(c1.x,c1.y,c1.z);
            const sc2 = w2s(c2.x,c2.y,c2.z), sc3 = w2s(c3.x,c3.y,c3.z);
            if (sc0 && sc1 && sc2 && sc3) {
              ctx.save();
              ctx.setLineDash([6, 4]);
              ctx.strokeStyle = 'rgba(56,189,248,0.85)';
              ctx.lineWidth = 2;
              ctx.beginPath();
              ctx.moveTo(sc0.x,sc0.y); ctx.lineTo(sc1.x,sc1.y);
              ctx.lineTo(sc2.x,sc2.y); ctx.lineTo(sc3.x,sc3.y);
              ctx.closePath(); ctx.stroke();
              ctx.fillStyle = 'rgba(56,189,248,0.07)';
              ctx.fill();
              ctx.restore();
              ctx.beginPath();
              ctx.arc(sc0.x, sc0.y, 5, 0, Math.PI * 2);
              ctx.fillStyle = '#10b981'; ctx.fill();
            }
          }

          // ── Circle tool preview ──
          if (sketchState.activeTool === 'circle' && sketchState.circle && sketchState.circle.active &&
              sketchState.circle.centerSnap && sketchState.hoverWorld) {
            const gc = sketchState.circle.centerSnap;
            const hw = sketchState.hoverWorld;
            const gs = sketchState.gridSize || 1.0;
            const plane = sketchState.workingPlane || 'XZ';
            const g2gx = Math.round(hw.gx !== undefined ? hw.gx : hw.x / gs);
            const g2gy = Math.round(hw.gy !== undefined ? hw.gy : hw.y / gs);
            const g2gz = Math.round(hw.gz !== undefined ? hw.gz : hw.z / gs);
            let radiusSq;
            if (plane === 'XZ')      radiusSq = (g2gx - gc.gx) ** 2 + (g2gz - gc.gz) ** 2;
            else if (plane === 'XY') radiusSq = (g2gx - gc.gx) ** 2 + (g2gy - gc.gy) ** 2;
            else                     radiusSq = (g2gy - gc.gy) ** 2 + (g2gz - gc.gz) ** 2;
            if (radiusSq >= 0.25) {
              const radius = Math.sqrt(radiusSq) * gs;
              let cx, cy, cz;
              if (plane === 'XZ')      { cx = gc.gx*gs; cy = 0;         cz = gc.gz*gs; }
              else if (plane === 'XY') { cx = gc.gx*gs; cy = gc.gy*gs;  cz = 0; }
              else                     { cx = 0;         cy = gc.gy*gs;  cz = gc.gz*gs; }
              const sc = w2s(cx, cy, cz);
              let rx, ry, rz;
              if (plane === 'XZ')      { rx = cx + radius; ry = 0;         rz = cz; }
              else if (plane === 'XY') { rx = cx + radius; ry = cy;        rz = 0; }
              else                     { rx = 0;            ry = cy+radius; rz = cz; }
              const sr = w2s(rx, ry, rz);
              if (sc && sr) {
                const scrRadius = Math.hypot(sr.x - sc.x, sr.y - sc.y);
                if (scrRadius > 1) {
                  ctx.save();
                  ctx.setLineDash([6, 4]);
                  ctx.strokeStyle = 'rgba(56,189,248,0.85)';
                  ctx.lineWidth = 2;
                  ctx.beginPath();
                  ctx.arc(sc.x, sc.y, scrRadius, 0, Math.PI * 2);
                  ctx.stroke();
                  ctx.fillStyle = 'rgba(56,189,248,0.06)';
                  ctx.fill();
                  ctx.restore();
                  ctx.beginPath();
                  ctx.arc(sc.x, sc.y, 5, 0, Math.PI * 2);
                  ctx.fillStyle = '#10b981'; ctx.fill();
                  ctx.save();
                  ctx.font = '11px monospace';
                  ctx.fillStyle = 'rgba(56,189,248,0.9)';
                  ctx.fillText('r≈' + (radius).toFixed(2), sc.x + 8, sc.y - 8);
                  ctx.restore();
                }
              }
            }
          }

          // ── Validation overlay ──
          if (sketchState.showValidation) {
            const isoSet = new Set(sketchState.validation.isolatedIds);
            const oeSet  = new Set(sketchState.validation.openEndIds);
            for (const p of sketchState.points) {
              const s = w2s(p.x, p.y, p.z);
              if (!s) continue;
              if (isoSet.has(p.id)) {
                ctx.save();
                ctx.beginPath();
                ctx.arc(s.x, s.y, 12, 0, Math.PI * 2);
                ctx.strokeStyle = 'rgba(244,114,182,0.85)';
                ctx.setLineDash([3, 2]);
                ctx.lineWidth = 1.5;
                ctx.stroke();
                ctx.restore();
              } else if (oeSet.has(p.id)) {
                ctx.save();
                ctx.beginPath();
                ctx.arc(s.x, s.y, 10, 0, Math.PI * 2);
                ctx.strokeStyle = 'rgba(239,68,68,0.75)';
                ctx.lineWidth = 1.2;
                ctx.stroke();
                ctx.restore();
              }
            }
          }

          // ── Points ── (скрываем когда solid committed, как и рёбра)
          if (_showEdges) {
          for (const p of sketchState.points) {
            const s = w2s(p.x, p.y, p.z);
            if (!s) continue;
            const isHover  = sketchState.hoverPointId === p.id;
            const isSel    = sketchState.selectedPointIds.has(p.id);
            const isGrabPt = grabPointSet && grabPointSet.has(p.id);
            const isAnchor = sketchState.line.startPointId === p.id;
            const isFixed  = window.__isPointFixed && window.__isPointFixed(p.id);
            const r = (isSel || isGrabPt) ? 8 : isAnchor ? 7.5 : isHover ? 7 : 6;
            ctx.beginPath();
            ctx.arc(s.x, s.y, r, 0, Math.PI * 2);
            ctx.fillStyle = isGrabPt ? '#facc15' : isSel ? '#fb923c' : isHover ? '#facc15' : isAnchor ? '#10b981' : '#38bdf8';
            ctx.fill();
            ctx.strokeStyle = '#0f172a';
            ctx.lineWidth = 1.8;
            ctx.stroke();
            if (isFixed) {
              const d = r + 4;
              ctx.save();
              ctx.strokeStyle = '#fbbf24';
              ctx.lineWidth = 1.8;
              ctx.strokeRect(s.x - d, s.y - d, d * 2, d * 2);
              ctx.beginPath();
              ctx.arc(s.x, s.y - d - 4, 2.2, 0, Math.PI * 2);
              ctx.fillStyle = '#fbbf24';
              ctx.fill();
              ctx.restore();
            }
          }
          }  // end _showEdges (points)
"##;
