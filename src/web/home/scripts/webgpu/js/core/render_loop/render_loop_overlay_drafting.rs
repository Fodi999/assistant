// ── Part 5: Drafting overlay — dimensions, labels, centerlines, grid ruler ────

pub const JS: &str = r##"
          // ── Projection plane badge + guide lines ──
          if (sketchState.draftMode === 'projection' && window.__showProjectionGuide !== false) {
            const lbl = window.__planeDescriptor
              ? window.__planeDescriptor(sketchState.workingPlane)
              : (sketchState.workingPlane || 'XZ');
            ctx.font = 'bold 12px "JetBrains Mono", system-ui, monospace';
            const bw = ctx.measureText(lbl).width + 16;
            ctx.fillStyle = 'rgba(15,23,42,0.92)';
            ctx.fillRect(16, 60, bw, 24);
            ctx.strokeStyle = 'rgba(56,189,248,0.55)';
            ctx.lineWidth = 1;
            ctx.strokeRect(16, 60, bw, 24);
            ctx.fillStyle = '#67e8f9';
            ctx.textAlign = 'left'; ctx.textBaseline = 'middle';
            ctx.fillText(lbl, 24, 72);
            const pl = sketchState.workingPlane || 'XZ';
            const guideTargets = [];
            if (sketchState.hoverPointId) {
              const p = pById.get(sketchState.hoverPointId);
              if (p) guideTargets.push({ p, color: 'rgba(250,204,21,0.55)' });
            }
            for (const id of sketchState.selectedPointIds) {
              const p = pById.get(id);
              if (p) guideTargets.push({ p, color: 'rgba(251,146,60,0.65)' });
            }
            if (guideTargets.length && sketchState.projection.showGuides) {
              const D = 50;
              const guideAlpha = window.__fadeBackgroundHelpers ? 0.28 : 0.65;
              ctx.save();
              ctx.setLineDash([3, 3]);
              ctx.lineWidth = 1;
              ctx.globalAlpha = guideAlpha;
              for (const g of guideTargets) {
                const p = g.p;
                let line1A, line1B, line2A, line2B;
                if (pl === 'XZ') {
                  line1A = w2s(-D, p.y, p.z); line1B = w2s(D, p.y, p.z);
                  line2A = w2s(p.x, p.y, -D); line2B = w2s(p.x, p.y,  D);
                } else if (pl === 'XY') {
                  line1A = w2s(-D, p.y, p.z); line1B = w2s(D, p.y, p.z);
                  line2A = w2s(p.x, -D, p.z); line2B = w2s(p.x,  D, p.z);
                } else {
                  line1A = w2s(p.x, p.y, -D); line1B = w2s(p.x, p.y, D);
                  line2A = w2s(p.x, -D, p.z); line2B = w2s(p.x,  D, p.z);
                }
                ctx.strokeStyle = g.color;
                ctx.beginPath();
                if (line1A && line1B) { ctx.moveTo(line1A.x, line1A.y); ctx.lineTo(line1B.x, line1B.y); }
                if (line2A && line2B) { ctx.moveTo(line2A.x, line2A.y); ctx.lineTo(line2B.x, line2B.y); }
                ctx.stroke();
                const map = window.__projectionCoordsForPlane && window.__projectionCoordsForPlane(p, pl);
                if (map) {
                  const ps = w2s(p.x, p.y, p.z);
                  if (ps) {
                    const fmt = window.__fmtCoord || (v => Number(v).toFixed(2));
                    const t = pl + ' · ' + map.hAxis + '=' + fmt(map.h) + ' ' + map.vAxis + '=' + fmt(map.v)
                            + '  (' + map.hiddenAxis + '=' + fmt(map.hidden) + ')';
                    ctx.font = '10px "JetBrains Mono", system-ui, monospace';
                    const tw2 = ctx.measureText(t).width + 8;
                    ctx.setLineDash([]);
                    ctx.fillStyle = 'rgba(15,23,42,0.85)';
                    ctx.fillRect(ps.x + 10, ps.y + 10, tw2, 16);
                    ctx.fillStyle = '#67e8f9';
                    ctx.textAlign = 'left'; ctx.textBaseline = 'middle';
                    ctx.fillText(t, ps.x + 14, ps.y + 18);
                    ctx.setLineDash([3, 3]);
                  }
                }
              }
              ctx.restore();
            }
          }

          // ── Drafting overlay IIFE ──
          (function drawDraftingOverlay() {
            const df = sketchState.drafting;
            if (!df) return;
            sketchState.draftingHitLabels = [];
            const internalMm = ((sketchState.precision && sketchState.precision.internalStepM) || 0.00001) * 1000;
            const DIM_COL    = 'rgba(226,232,240,0.85)';
            const DIM_FILL   = 'rgba(15,23,42,0.85)';
            const EXT_COL    = 'rgba(148,163,184,0.75)';
            const CENTER_COL = 'rgba(167,139,250,0.65)';
            const fontMain   = '11px "JetBrains Mono", system-ui, monospace';
            const fontTiny   = '10px "JetBrains Mono", system-ui, monospace';

            function arrow(x, y, dx, dy, size) {
              const L  = Math.hypot(dx, dy) || 1;
              const ux = dx / L, uy = dy / L;
              const px = -uy, py = ux;
              const bx = x - ux * size, by = y - uy * size;
              const lx = bx + px * size * 0.4, ly = by + py * size * 0.4;
              const rx = bx - px * size * 0.4, ry = by - py * size * 0.4;
              ctx.beginPath();
              ctx.moveTo(x, y); ctx.lineTo(lx, ly); ctx.lineTo(rx, ry); ctx.closePath();
              ctx.fill();
            }

            function drawDimension(saX, saY, sbX, sbY, label, opts, hitMeta) {
              opts = opts || {};
              const off  = (opts.offsetPx  != null) ? opts.offsetPx  : (df.dimensionOffsetPx || 20);
              const arrS = (opts.arrowPx   != null) ? opts.arrowPx   : (df.arrowSizePx       || 7);
              const gap  = (opts.gapPx     != null) ? opts.gapPx     : (df.textGapPx         || 6);
              const flip = !!opts.flip;
              let dx = sbX - saX, dy = sbY - saY;
              const L = Math.hypot(dx, dy);
              if (L < 1) return;
              const ux = dx / L, uy = dy / L;
              let nx = -uy, ny = ux;
              if (flip) { nx = -nx; ny = -ny; }
              const off1 = off;
              const dax = saX + nx * off1, day = saY + ny * off1;
              const dbx = sbX + nx * off1, dby = sbY + ny * off1;
              ctx.save();
              ctx.lineWidth = 1;
              ctx.strokeStyle = EXT_COL;
              ctx.beginPath();
              ctx.moveTo(saX + nx * gap, saY + ny * gap);
              ctx.lineTo(saX + nx * (off1 + 4), saY + ny * (off1 + 4));
              ctx.moveTo(sbX + nx * gap, sbY + ny * gap);
              ctx.lineTo(sbX + nx * (off1 + 4), sbY + ny * (off1 + 4));
              ctx.stroke();
              ctx.strokeStyle = DIM_COL;
              ctx.lineWidth = 1.1;
              ctx.beginPath();
              ctx.moveTo(dax, day); ctx.lineTo(dbx, dby);
              ctx.stroke();
              ctx.fillStyle = DIM_COL;
              arrow(dax, day, -ux, -uy, arrS);
              arrow(dbx, dby,  ux,  uy, arrS);
              const mx = (dax + dbx) * 0.5;
              const my = (day + dby) * 0.5;
              ctx.font = fontMain;
              const tw = ctx.measureText(label).width + 10;
              const th = 16;
              ctx.fillStyle = DIM_FILL;
              ctx.fillRect(mx - tw / 2, my - th / 2, tw, th);
              ctx.fillStyle = '#e2e8f0';
              ctx.textAlign = 'center'; ctx.textBaseline = 'middle';
              ctx.fillText(label, mx, my);
              ctx.restore();
              if (hitMeta) {
                const pad = 6;
                sketchState.draftingHitLabels.push(Object.assign({}, hitMeta, {
                  rect: { x: mx - tw / 2 - pad, y: my - th / 2 - pad, w: tw + pad * 2, h: th + pad * 2 },
                }));
              }
            }

            // ── Dimensions for selected edges ──
            if (df.showDimensions) {
              for (const eid of sketchState.selectedEdgeIds) {
                const e = eById.get(eid); if (!e) continue;
                const a = pById.get(e.a), b = pById.get(e.b); if (!a || !b) continue;
                const sa = w2s(a.x, a.y, a.z), sb = w2s(b.x, b.y, b.z);
                if (!sa || !sb) continue;
                const lenMm = window.__edgeLengthMm(a, b);
                drawDimension(sa.x, sa.y, sb.x, sb.y, window.__formatDim(lenMm), undefined,
                  { kind: 'edge_length_dimension', edgeId: eid, aPointId: a.id, bPointId: b.id, valueMm: lenMm });
              }
              const profId = sketchState.selectedProfileId;
              if (profId) {
                const prof = (sketchState.profiles || []).find(p => p.id === profId);
                if (prof && prof.pointIds && prof.pointIds.length >= 3) {
                  let minSx = Infinity, minSy = Infinity, maxSx = -Infinity, maxSy = -Infinity;
                  let minGx = Infinity, maxGx = -Infinity;
                  let minGy = Infinity, maxGy = -Infinity;
                  let minGz = Infinity, maxGz = -Infinity;
                  let okScreen = true;
                  for (const pid of prof.pointIds) {
                    const p = pById.get(pid); if (!p) { okScreen = false; break; }
                    const s = w2s(p.x, p.y, p.z); if (!s) { okScreen = false; break; }
                    if (s.x < minSx) minSx = s.x; if (s.y < minSy) minSy = s.y;
                    if (s.x > maxSx) maxSx = s.x; if (s.y > maxSy) maxSy = s.y;
                    if (p.gx < minGx) minGx = p.gx; if (p.gx > maxGx) maxGx = p.gx;
                    if (p.gy < minGy) minGy = p.gy; if (p.gy > maxGy) maxGy = p.gy;
                    if (p.gz < minGz) minGz = p.gz; if (p.gz > maxGz) maxGz = p.gz;
                  }
                  if (okScreen) {
                    const pl = prof.plane || sketchState.workingPlane || 'XZ';
                    let widthMm, heightMm;
                    if      (pl === 'XZ') { widthMm = (maxGx-minGx)*internalMm; heightMm = (maxGz-minGz)*internalMm; }
                    else if (pl === 'XY') { widthMm = (maxGx-minGx)*internalMm; heightMm = (maxGy-minGy)*internalMm; }
                    else                  { widthMm = (maxGz-minGz)*internalMm; heightMm = (maxGy-minGy)*internalMm; }
                    if (widthMm > 0)  drawDimension(minSx, minSy, maxSx, minSy, window.__formatDim(widthMm),
                      { flip: true }, { kind: 'profile_width_dimension',  profileId: profId, valueMm: widthMm });
                    if (heightMm > 0) drawDimension(maxSx, minSy, maxSx, maxSy, window.__formatDim(heightMm),
                      {},             { kind: 'profile_height_dimension', profileId: profId, valueMm: heightMm });
                  }
                }
              }
            }

            // ── Edge lengths ──
            if (df.showEdgeLengths) {
              const total = sketchState.edges.length;
              const showAll = total < 20;
              for (const e of sketchState.edges) {
                const isHover = sketchState.hoverEdgeId === e.id;
                const isSel   = sketchState.selectedEdgeIds.has(e.id);
                if (!showAll && !isHover && !isSel) continue;
                if (df.showDimensions && isSel) continue;
                const a = pById.get(e.a), b = pById.get(e.b); if (!a || !b) continue;
                const sa = w2s(a.x, a.y, a.z), sb = w2s(b.x, b.y, b.z);
                if (!sa || !sb) continue;
                const lenMm = window.__edgeLengthMm(a, b);
                const txt = window.__formatDim(lenMm);
                const mx = (sa.x + sb.x) * 0.5, my = (sa.y + sb.y) * 0.5;
                ctx.font = fontTiny;
                const tw = ctx.measureText(txt).width + 8;
                ctx.fillStyle = DIM_FILL;
                ctx.fillRect(mx - tw / 2, my - 8, tw, 14);
                ctx.fillStyle = '#cbd5e1';
                ctx.textAlign = 'center'; ctx.textBaseline = 'middle';
                ctx.fillText(txt, mx, my - 1);
                const pad = 6;
                sketchState.draftingHitLabels.push({
                  kind: 'edge_length_dimension', edgeId: e.id, aPointId: a.id, bPointId: b.id, valueMm: lenMm,
                  rect: { x: mx - tw / 2 - pad, y: my - 8 - pad, w: tw + pad * 2, h: 14 + pad * 2 },
                });
              }
            }

            // ── Point coordinate labels ──
            if (df.showPointLabels) {
              const ids = new Set(sketchState.selectedPointIds);
              if (sketchState.hoverPointId) ids.add(sketchState.hoverPointId);
              for (const id of ids) {
                const p = pById.get(id); if (!p) continue;
                const s = w2s(p.x, p.y, p.z); if (!s) continue;
                const c = window.__pointCoordsMm(p);
                const t = 'X ' + c.x + '  Y ' + c.y + '  Z ' + c.z;
                ctx.font = fontTiny;
                const tw = ctx.measureText(t).width + 8;
                ctx.fillStyle = DIM_FILL;
                ctx.fillRect(s.x + 10, s.y - 22, tw, 14);
                ctx.fillStyle = '#cbd5e1';
                ctx.textAlign = 'left'; ctx.textBaseline = 'middle';
                ctx.fillText(t, s.x + 14, s.y - 15);
              }
            }

            // ── Centerlines ──
            if (df.showCenterlines && sketchState.profiles && sketchState.profiles.length) {
              ctx.save();
              ctx.setLineDash([8, 3, 2, 3]);
              ctx.lineWidth = 0.8;
              ctx.strokeStyle = CENTER_COL;
              for (const prof of sketchState.profiles) {
                let cx = 0, cy = 0, cz = 0, n = 0;
                for (const pid of prof.pointIds) {
                  const p = pById.get(pid); if (!p) { n = 0; break; }
                  cx += p.x; cy += p.y; cz += p.z; n++;
                }
                if (!n) continue;
                cx /= n; cy /= n; cz /= n;
                const sc = w2s(cx, cy, cz); if (!sc) continue;
                ctx.beginPath();
                ctx.moveTo(sc.x - 18, sc.y); ctx.lineTo(sc.x + 18, sc.y);
                ctx.moveTo(sc.x, sc.y - 18); ctx.lineTo(sc.x, sc.y + 18);
                ctx.stroke();
              }
              ctx.restore();
            }

            // ── Grid ruler numbers ──
            if (df.showGridNumbers) {
              const pr      = sketchState.precision || {};
              const dispM   = (pr.displayGridStepM > 0) ? pr.displayGridStepM : 0.001;
              const pl      = sketchState.workingPlane || 'XZ';
              const minStep = 60;
              function rulerAxis(getWorld, dimEdge, axisName) {
                const N = 200;
                let lastPx = -Infinity;
                ctx.font = fontTiny;
                ctx.fillStyle = '#94a3b8';
                ctx.textAlign    = (dimEdge === 'bottom') ? 'center' : 'right';
                ctx.textBaseline = (dimEdge === 'bottom') ? 'bottom'  : 'middle';
                for (let i = -N; i <= N; i++) {
                  const w = getWorld(i * dispM);
                  const s = w2s(w[0], w[1], w[2]);
                  if (!s) continue;
                  const key = (dimEdge === 'bottom') ? s.x : s.y;
                  if (Math.abs(key - lastPx) < minStep) continue;
                  if (dimEdge === 'bottom') {
                    if (s.x < 30 || s.x > sk.width - 30) continue;
                  } else {
                    if (s.y < 30 || s.y > sk.height - 30) continue;
                  }
                  lastPx = key;
                  const valMm = (i * dispM) * 1000;
                  const txt = window.__formatDim(valMm);
                  if (dimEdge === 'bottom') ctx.fillText(txt, s.x, sk.height - 6);
                  else                      ctx.fillText(txt, sk.width - 6, s.y);
                }
              }
              if (pl === 'XZ') {
                rulerAxis(v => [v, 0, 0], 'bottom', 'X');
                rulerAxis(v => [0, 0, v], 'left',   'Z');
              } else if (pl === 'XY') {
                rulerAxis(v => [v, 0, 0], 'bottom', 'X');
                rulerAxis(v => [0, v, 0], 'left',   'Y');
              } else {
                rulerAxis(v => [0, 0, v], 'bottom', 'Z');
                rulerAxis(v => [0, v, 0], 'left',   'Y');
              }
            }
          })();
"##;
