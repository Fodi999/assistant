// ── Part 6: Status banner, gizmos (grab/extrude/solid), copy preview, RAF loop ─

pub const JS: &str = r##"
          // ── Status message banner ──
          if (sketchState.statusMessage) {
            const txt = sketchState.statusMessage;
            ctx.font = '12px "JetBrains Mono", system-ui, monospace';
            const tw = ctx.measureText(txt).width + 20;
            const y0 = sk.height - 70;
            ctx.fillStyle = 'rgba(15,23,42,0.92)';
            ctx.fillRect(sk.width/2 - tw/2, y0, tw, 26);
            ctx.strokeStyle = 'rgba(251,191,36,0.55)';
            ctx.lineWidth = 1;
            ctx.strokeRect(sk.width/2 - tw/2, y0, tw, 26);
            ctx.fillStyle = '#fbbf24';
            ctx.textAlign = 'center'; ctx.textBaseline = 'middle';
            ctx.fillText(txt, sk.width/2, y0 + 13);
          }

          // ── Gizmo delegates ──
          if (typeof window.__drawGrabGizmo         === 'function') window.__drawGrabGizmo(ctx, sketchState, w2s, sk);
          if (typeof window.__drawExtrudeGizmo      === 'function') window.__drawExtrudeGizmo(ctx, sketchState, w2s, sk);
          if (typeof window.__drawSolidExtrudeGizmo === 'function') window.__drawSolidExtrudeGizmo(ctx, w2s);

          // ── Copy Connect preview ──
          if (sketchState.copy.active) {
            const cp = sketchState.copy;
            const { dx, dy, dz } = cp.delta;
            const lock = cp.axisLock;
            const lockColor = lock === 'X' ? '#ef4444' : lock === 'Y' ? '#22c55e' : lock === 'Z' ? '#3b82f6' : '#22d3ee';
            const cyan = '#22d3ee';
            const byId = new Map(sketchState.points.map(p => [p.id, p]));
            const origScr = new Map();
            const copyScr = new Map();
            for (const id of cp.pointIds) {
              const o = cp.originals.get(id);
              if (!o) continue;
              const so = w2s(o.x, o.y, o.z);
              const sc = w2s(o.x + dx, o.y + dy, o.z + dz);
              if (so) origScr.set(id, so);
              if (sc) copyScr.set(id, sc);
            }
            ctx.save();
            ctx.setLineDash([5, 4]);
            ctx.strokeStyle = cyan;
            ctx.lineWidth = 1.6;
            for (const [a, b] of cp.edges) {
              const pa = copyScr.get(a), pb = copyScr.get(b);
              if (!pa || !pb) continue;
              ctx.beginPath(); ctx.moveTo(pa.x, pa.y); ctx.lineTo(pb.x, pb.y); ctx.stroke();
            }
            ctx.setLineDash([3, 4]);
            ctx.strokeStyle = 'rgba(34,211,238,0.75)';
            ctx.lineWidth = 1.0;
            for (const id of cp.pointIds) {
              const a = origScr.get(id), b = copyScr.get(id);
              if (!a || !b) continue;
              ctx.beginPath(); ctx.moveTo(a.x, a.y); ctx.lineTo(b.x, b.y); ctx.stroke();
            }
            ctx.setLineDash([]);
            ctx.fillStyle = cyan;
            for (const p of copyScr.values()) {
              ctx.beginPath(); ctx.arc(p.x, p.y, 3.2, 0, Math.PI * 2); ctx.fill();
            }
            if (lock && cp.startMouseWorld) {
              const o = cp.startMouseWorld;
              const dst = 50;
              let p1, p2;
              if (lock === 'X') { p1 = w2s(o.x-dst,o.y,o.z);   p2 = w2s(o.x+dst,o.y,o.z); }
              if (lock === 'Y') { p1 = w2s(o.x,o.y-dst,o.z);   p2 = w2s(o.x,o.y+dst,o.z); }
              if (lock === 'Z') { p1 = w2s(o.x,o.y,o.z-dst);   p2 = w2s(o.x,o.y,o.z+dst); }
              if (p1 && p2) {
                ctx.setLineDash([4, 4]);
                ctx.strokeStyle = lockColor;
                ctx.lineWidth = 1.5;
                ctx.beginPath(); ctx.moveTo(p1.x, p1.y); ctx.lineTo(p2.x, p2.y); ctx.stroke();
                ctx.setLineDash([]);
              }
            }
            ctx.restore();
            const dist = Math.hypot(dx, dy, dz);
            const head = 'COPY ' + cp.pointIds.length + 'pt'
              + (lock ? (' · ' + lock + '-axis') : '')
              + '  ΔX ' + dx.toFixed(2) + '  ΔY ' + dy.toFixed(2) + '  ΔZ ' + dz.toFixed(2)
              + '  · |Δ| ' + dist.toFixed(2);
            ctx.save();
            ctx.font = '12px "JetBrains Mono", system-ui, monospace';
            const tw = ctx.measureText(head).width + 16;
            ctx.fillStyle = 'rgba(15,23,42,0.92)';
            ctx.fillRect(sk.width/2 - tw/2, 46, tw, 24);
            ctx.fillStyle = lockColor;
            ctx.textAlign = 'center'; ctx.textBaseline = 'middle';
            ctx.fillText(head, sk.width/2, 58);
            ctx.restore();
          }
        }

        if (window.__perfSample)    window.__perfSample('overlay', performance.now() - __pfOverlay);
        if (window.__updatePerfHud) window.__updatePerfHud();
        if (window.__cadPanelTick)  window.__cadPanelTick();

        gpuRafId = requestAnimationFrame(frame);
      }
      gpuRafId = requestAnimationFrame(frame);

      window.__stopWebGpuScene = function() {
        if (gpuRafId) cancelAnimationFrame(gpuRafId);
        gpuRafId = null;
      };
    }
    window.startWebGpuScene = startWebGpuScene;
"##;
