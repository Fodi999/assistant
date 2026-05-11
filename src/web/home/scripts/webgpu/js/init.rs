// ── JS: application bootstrap and GPU device initialisation ──────────────────────
// Domain: Infrastructure — WebGPU adapter/device request, canvas configuration.

pub const JS: &str = r##"
    // ══════════════════════════════════════════════════════════════
    // WebGPU Scene v2.1 — Cell-SDF megashape (kernel::particle_shape port)
    //   • drag / shift+drag  — orbit / pan camera
    //   • wheel              — zoom 0.5…80
    //   • 1-5                — 1K / 10K / 100K / 500K / 1M
    //   • + / -              — ×1.5 fine
    //   • C / V / W          — cloud / cube / wall formation
    //   • S                  — toggle Cell-SDF rendering (kernel parity)
    //   • [ / ]              — cell radius (when Cell-SDF on) / shape exp (otherwise)
    //   • N                  — debug: show surface normals as RGB
    //   • M                  — debug: colour by SlotKind (face/edge/corner)
    //   • I                  — debug: hide interior + face cells (edges & corners only)
    //   • F                  — debug preset: M + I together
    //   • R                  — auto-rotate
    //   • B                  — run full benchmark (1K→5M)
    // ══════════════════════════════════════════════════════════════
    let gpuRafId = null;

    async function startWebGpuScene() {
      const canvas = document.getElementById('webgpu-canvas');
      const badge  = document.getElementById('webgpu-status');

      function log(msg, color = '#e2e8f0') {
        console.log('[WebGPU]', msg);
      }
      function setBadge(text, color) {
        if (badge) { badge.textContent = text; badge.style.color = color; }
      }

      canvas.width  = window.innerWidth;
      canvas.height = window.innerHeight;
      log(`canvas: ${canvas.width}×${canvas.height} | dpr: ${window.devicePixelRatio}`);

      // ── 1. Probe ────────────────────────────────────────────────
      if (!navigator.gpu) { setBadge('✗ WebGPU недоступен', '#f87171'); return; }
      let adapter, device;
      try {
        adapter = await navigator.gpu.requestAdapter({ powerPreference: 'high-performance' });
        if (!adapter) throw new Error('requestAdapter() = null');
        const hasTimestamp = adapter.features.has('timestamp-query');

        // ask for the maximum storage buffer the adapter allows (so we can fit 5M)
        const maxStorageBind = adapter.limits.maxStorageBufferBindingSize;
        const maxBuffer      = adapter.limits.maxBufferSize;
        device = await adapter.requestDevice({
          requiredFeatures: hasTimestamp ? ['timestamp-query'] : [],
          requiredLimits: {
            maxStorageBufferBindingSize: maxStorageBind,
            maxBufferSize:               maxBuffer,
          },
        });
        log(`✓ adapter · maxStorageBuf=${(maxStorageBind/1048576)|0}MB · ts=${hasTimestamp}`, '#34d399');
      } catch (e) { setBadge('✗ ' + e.message, '#fb923c'); return; }
      const hasTimestamp = device.features.has('timestamp-query');

      // ── 2. Configure canvas ─────────────────────────────────────
      const grad = document.querySelector('.render-gradient');
      const grid = document.querySelector('.render-grid');
      if (grad) grad.style.visibility = 'hidden';
      if (grid) grid.style.visibility = 'hidden';
      document.body.classList.add('gpu-active');

      const dpr = window.devicePixelRatio || 1;
      const sketchCanvas = document.getElementById('sketch-canvas');
      function resizeCanvas() {
        canvas.width  = Math.floor(window.innerWidth  * dpr);
        canvas.height = Math.floor(window.innerHeight * dpr);
        
        if (sketchCanvas) {
          sketchCanvas.width = Math.floor(window.innerWidth * dpr);
          sketchCanvas.height = Math.floor(window.innerHeight * dpr);
          sketchCanvas.style.width = window.innerWidth + 'px';
          sketchCanvas.style.height = window.innerHeight + 'px';
        }
      }
      resizeCanvas();
      window.addEventListener('resize', resizeCanvas);

      const fmt    = navigator.gpu.getPreferredCanvasFormat();
      const gpuCtx = canvas.getContext('webgpu');
      gpuCtx.configure({ device, format: fmt, alphaMode: 'opaque' });
"##;
