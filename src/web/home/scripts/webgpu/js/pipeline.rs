// ── JS: render pipeline and depth texture setup ───────────────────────────────────
// Domain: Infrastructure — WGSL module compilation, bg+particle pipelines, depth.

pub const JS: &str = r##"
      const module = device.createShaderModule({ code: shaderSrc });
      // Surface any WGSL compile errors to the on-page log AND console
      module.getCompilationInfo().then((info) => {
        if (info.messages.length === 0) {
          log('✓ WGSL скомпилирован', '#34d399');
        } else {
          for (const m of info.messages) {
            const tag = m.type === 'error' ? '✗ WGSL' : '⚠ WGSL';
            const colour = m.type === 'error' ? '#f87171' : '#fbbf24';
            log(`${tag} L${m.lineNum}:${m.linePos} — ${m.message}`, colour);
            console[m.type === 'error' ? 'error' : 'warn'](
              `[WGSL ${m.type}] line ${m.lineNum} col ${m.linePos}:`, m.message
            );
          }
        }
      });

      const DEPTH_FMT = 'depth24plus';
      const bgPipeline = device.createRenderPipeline({
        layout: pipelineLayout,
        vertex:   { module, entryPoint: 'vs_full' },
        fragment: { module, entryPoint: 'fs_bg', targets: [{ format: fmt }] },
        primitive: { topology: 'triangle-list' },
        depthStencil: { format: DEPTH_FMT, depthWriteEnabled: true, depthCompare: 'less' },
      });
      const spherePipeline = device.createRenderPipeline({
        layout: pipelineLayout,
        vertex:   { module, entryPoint: 'vs_particles' },
        // fully opaque — no blend, write alpha=1 from fragment
        fragment: { module, entryPoint: 'fs_particles', targets: [{ format: fmt }] },
        primitive: { topology: 'triangle-list' },
        depthStencil: { format: DEPTH_FMT, depthWriteEnabled: true, depthCompare: 'less' },
      });

      // ── Depth texture, recreated on resize ─────────────────────
      let depthTex = device.createTexture({
        size: [canvas.width, canvas.height, 1],
        format: DEPTH_FMT,
        usage: GPUTextureUsage.RENDER_ATTACHMENT,
      });
      let depthW = canvas.width, depthH = canvas.height;
      function ensureDepth() {
        if (canvas.width === depthW && canvas.height === depthH) return;
        depthTex.destroy();
        depthTex = device.createTexture({
          size: [canvas.width, canvas.height, 1],
          format: DEPTH_FMT,
          usage: GPUTextureUsage.RENDER_ATTACHMENT,
        });
        depthW = canvas.width; depthH = canvas.height;
      }
      log(`✓ pipelines готовы (bg + ${NUM_SPHERES.toLocaleString()} частиц)`, '#34d399');
"##;
