// ── JS: benchmark engine ──────────────────────────────────────────────────────────
// Domain: Performance testing — stepped particle-count sweep, stats collection,
//         results overlay (avg/p99/1%-low FPS).

pub const JS: &str = r##"
      // ── Benchmark engine ─────────────────────────────────────────
      // Steps: 1K · 10K · 100K · 500K · 1M · 2M · 5M (filtered by MAX_PARTICLES)
      const ALL_STEPS      = [1_000, 10_000, 100_000, 500_000, 1_000_000, 2_000_000, 5_000_000];
      const BENCH_STEPS    = ALL_STEPS.filter(n => n <= MAX_PARTICLES);
      const BENCH_WARMUP   = 8;    // frames to skip before measuring
      const BENCH_SAMPLES  = 120;  // frames to measure per step

      const bench = {
        running:   false,
        step:      0,
        warmup:    0,
        samples:   [],            // frame-time samples (ms) for current step
        cpuSamples: [],           // CPU submit time (ms)
        results:   [],            // [{count, avgFps, p1Fps, avgMs, maxMs, p99Ms, cpuMs}]
        frameStart: 0,
      };

      // called from frame() when bench.running
      function benchTick(cpuFrameMs) {
        if (bench.warmup < BENCH_WARMUP) { bench.warmup++; return; }
        bench.samples.push(cpuFrameMs);
        bench.cpuSamples.push(cpuFrameMs);
        if (bench.samples.length < BENCH_SAMPLES) return;

        // compute stats
        const times = bench.samples.slice().sort((a, b) => a - b);
        const n     = times.length;
        const avgMs = times.reduce((s, v) => s + v, 0) / n;
        const maxMs = times[n - 1];
        const p99Ms = times[Math.floor(n * 0.99)];
        const p1idx = Math.max(0, Math.floor(n * 0.01));
        // 1% low FPS = avg of bottom 1% worst frame times
        const worst1pct = times.slice(Math.floor(n * 0.99));
        const p1AvgMs   = worst1pct.reduce((s, v) => s + v, 0) / worst1pct.length;
        const avgFps    = 1000 / avgMs;
        const p1Fps     = 1000 / p1AvgMs;

        bench.results.push({
          count: NUM_SPHERES,
          avgFps: avgFps.toFixed(1),
          p1Fps:  p1Fps.toFixed(1),
          avgMs:  avgMs.toFixed(2),
          maxMs:  maxMs.toFixed(2),
          p99Ms:  p99Ms.toFixed(2),
        });

        bench.step++;
        bench.samples = [];
        bench.cpuSamples = [];
        bench.warmup = 0;

        if (bench.step >= BENCH_STEPS.length) {
          bench.running = false;
          showBenchResults();
          setParticleCount(1_000_000);   // restore 1M
          return;
        }
        // next step
        const nextCount = BENCH_STEPS[bench.step];
        if (nextCount > MAX_PARTICLES) {
          // can't go higher — record N/A and finish
          for (let i = bench.step; i < BENCH_STEPS.length; i++) {
            bench.results.push({ count: BENCH_STEPS[i], avgFps: 'OOM', p1Fps: '—',
              avgMs: '—', maxMs: '—', p99Ms: '—' });
          }
          bench.running = false;
          showBenchResults();
          setParticleCount(1_000_000);
          return;
        }
        setParticleCount(nextCount);
      }

      async function runBenchmark() {
        bench.running  = true;
        bench.step     = 0;
        bench.warmup   = 0;
        bench.samples  = [];
        bench.results  = [];
        // remove old result panel
        document.getElementById('bench-overlay')?.remove();
        setParticleCount(BENCH_STEPS[0]);
        log('🔬 Benchmark запущен… нажми B чтобы увидеть результат', '#fbbf24');
        // show progress in HUD
        const origUpdate = updateHud;
        // override HUD to show bench progress
        const benchHudInterval = setInterval(() => {
          if (!bench.running) { clearInterval(benchHudInterval); return; }
          const stepCount = BENCH_STEPS[bench.step] || 0;
          hud.innerHTML =
            `<div style="color:#fbbf24;font-weight:600">🔬 BENCHMARK RUNNING</div>` +
            `<div style="margin-top:4px"><span style="color:#94a3b8">step</span> ` +
            `<b style="color:#f0abfc">${bench.step + 1} / ${BENCH_STEPS.length}</b></div>` +
            `<div><span style="color:#94a3b8">current</span> <b style="color:#a78bfa">${fmtN(stepCount)}</b></div>` +
            `<div><span style="color:#94a3b8">samples</span> <b>${bench.samples.length} / ${BENCH_SAMPLES}</b></div>` +
            `<div style="margin-top:4px;color:#64748b;font-size:11px">auto-collecting…</div>`;
        }, 200);
      }

      function showBenchResults() {
        // remove progress interval is already stopped
        document.getElementById('bench-overlay')?.remove();
        const overlay = document.createElement('div');
        overlay.id = 'bench-overlay';
        overlay.style.cssText = [
          'position:fixed','inset:0','z-index:99999',
          'background:rgba(2,6,23,.88)','backdrop-filter:blur(18px)',
          '-webkit-backdrop-filter:blur(18px)',
          'display:flex','flex-direction:column',
          'align-items:center','justify-content:center',
          'font-family:-apple-system,SF Pro Display,system-ui,monospace',
          'color:#e2e8f0',
        ].join(';');

        const VRAM_MB = (MAX_PARTICLES * 32 / 1048576).toFixed(0);

        let rows = bench.results.map(r => {
          const fpsColor = r.avgFps === 'OOM' ? '#f87171'
            : +r.avgFps >= 60 ? '#34d399'
            : +r.avgFps >= 30 ? '#fbbf24' : '#f87171';
          const p1Color  = r.p1Fps === '—' ? '#475569'
            : +r.p1Fps >= 60 ? '#34d399'
            : +r.p1Fps >= 30 ? '#fbbf24' : '#f87171';
          const bottleneck = r.avgFps === 'OOM'
            ? '<span style="color:#f87171">OUT OF MEM</span>'
            : +r.avgFps < 15
              ? '<span style="color:#f87171">GPU bound ⚠</span>'
              : +r.avgFps < 40
                ? '<span style="color:#fbbf24">GPU bound</span>'
                : '<span style="color:#34d399">CPU/rAF OK</span>';
          return `<tr>
            <td style="padding:7px 14px;color:#a78bfa;font-weight:600">${fmtN(r.count)}</td>
            <td style="padding:7px 14px;color:${fpsColor};font-weight:700">${r.avgFps}</td>
            <td style="padding:7px 14px;color:${p1Color}">${r.p1Fps}</td>
            <td style="padding:7px 14px;color:#94a3b8">${r.avgMs}</td>
            <td style="padding:7px 14px;color:#f87171">${r.maxMs}</td>
            <td style="padding:7px 14px;color:#fb923c">${r.p99Ms}</td>
            <td style="padding:7px 14px">${bottleneck}</td>
          </tr>`;
        }).join('');

        // find cliff — where fps drops below 30
        const cliffIdx = bench.results.findIndex(r => +r.avgFps < 30 && r.avgFps !== 'OOM');
        const cliffNote = cliffIdx > 0
          ? `<p style="margin-top:16px;color:#fbbf24">⚠ Bottleneck начинается при <b style="color:#f0abfc">${fmtN(bench.results[cliffIdx].count)}</b> частиц (fps &lt; 30)</p>`
          : cliffIdx === 0
            ? `<p style="margin-top:16px;color:#f87171">⚠ GPU недостаточен уже с <b>${fmtN(bench.results[0].count)}</b></p>`
            : `<p style="margin-top:16px;color:#34d399">✓ GPU держит &gt;30 fps на всех протестированных ступенях</p>`;

        overlay.innerHTML = `
          <div style="max-width:820px;width:95%;padding:32px;border-radius:16px;
            background:rgba(8,14,36,.95);border:1px solid rgba(103,232,249,.2);
            box-shadow:0 32px 80px rgba(0,0,0,.7)">
            <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:20px">
              <div>
                <h2 style="margin:0;font-size:20px;color:#67e8f9;letter-spacing:.06em">
                  🔬 PARTICLE BENCHMARK RESULTS
                </h2>
                <p style="margin:4px 0 0;font-size:12px;color:#475569">
                  ${BENCH_SAMPLES} frames / step · timestamp-query: ${hasTimestamp ? '<span style="color:#34d399">enabled</span>' : '<span style="color:#64748b">off (CPU timing)</span>'}
                  · buffer: <b style="color:#a78bfa">${VRAM_MB} MB</b> (5M max)
                </p>
              </div>
              <button id="bench-close" style="background:#1e293b;border:1px solid #334155;
                color:#94a3b8;padding:6px 16px;border-radius:8px;cursor:pointer;font-size:13px">
                ✕ закрыть
              </button>
            </div>
            <table style="width:100%;border-collapse:collapse;font-size:13px">
              <thead>
                <tr style="color:#64748b;font-size:11px;letter-spacing:.06em;text-transform:uppercase;
                  border-bottom:1px solid #1e293b">
                  <th style="padding:6px 14px;text-align:left">particles</th>
                  <th style="padding:6px 14px;text-align:left">avg fps</th>
                  <th style="padding:6px 14px;text-align:left">1% low fps</th>
                  <th style="padding:6px 14px;text-align:left">avg ms</th>
                  <th style="padding:6px 14px;text-align:left">max ms</th>
                  <th style="padding:6px 14px;text-align:left">p99 ms</th>
                  <th style="padding:6px 14px;text-align:left">bottleneck</th>
                </tr>
              </thead>
              <tbody style="border-top:1px solid #0f172a">${rows}</tbody>
            </table>
            ${cliffNote}
            <p style="margin-top:8px;font-size:11px;color:#334155">
              ms = CPU rAF→submit · 1% low = средний худший 1% кадров · p99 = 99-й перцентиль задержки
            </p>
          </div>`;
        document.body.appendChild(overlay);
        document.getElementById('bench-close').onclick = () => overlay.remove();
      }
"##;
