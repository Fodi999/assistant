// ── JS: render pipeline and depth texture setup ───────────────────────────────────
// Domain: Infrastructure — WGSL module compilation, bg+particle pipelines, depth.

pub const JS: &str = r##"
      // ── Particle / Morph pipeline (SDF billboard raymarching) ──
      const module = device.createShaderModule({ code: shaderSrc });
      module.getCompilationInfo().then((info) => {
        if (info.messages.length === 0) {
          log('✓ WGSL [PARTICLE] скомпилирован', '#34d399');
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

      // ── CAD / Solid pipeline (rasterized mesh, clean shading) ──
      const cadModule = device.createShaderModule({ code: cadShaderSrc });
      cadModule.getCompilationInfo().then((info) => {
        if (info.messages.length === 0) {
          log('✓ WGSL [CAD] скомпилирован', '#34d399');
        } else {
          for (const m of info.messages) {
            const tag = m.type === 'error' ? '✗ WGSL [CAD]' : '⚠ WGSL [CAD]';
            const colour = m.type === 'error' ? '#f87171' : '#fbbf24';
            log(`${tag} L${m.lineNum}:${m.linePos} — ${m.message}`, colour);
            console[m.type === 'error' ? 'error' : 'warn'](
              `[WGSL CAD ${m.type}] line ${m.lineNum} col ${m.linePos}:`, m.message
            );
          }
        }
      });

      const DEPTH_FMT = 'depth24plus';

      // Background fullscreen pass (shared by both modes)
      const bgPipeline = device.createRenderPipeline({
        layout: pipelineLayout,
        vertex:   { module, entryPoint: 'vs_full' },
        fragment: { module, entryPoint: 'fs_bg', targets: [{ format: fmt }] },
        primitive: { topology: 'triangle-list' },
        depthStencil: { format: DEPTH_FMT, depthWriteEnabled: true, depthCompare: 'less' },
      });

      // Mode 1: Particle / Morph — instanced billboards + SDF raymarching
      const spherePipeline = device.createRenderPipeline({
        layout: pipelineLayout,
        vertex:   { module, entryPoint: 'vs_particles' },
        fragment: { module, entryPoint: 'fs_particles', targets: [{ format: fmt }] },
        primitive: { topology: 'triangle-list', cullMode: 'back' },
        depthStencil: { format: DEPTH_FMT, depthWriteEnabled: true, depthCompare: 'less' },
      });

      // Mode 2: CAD / Solid — rasterized cube mesh, clean Blender-style lighting
      const cadPipeline = device.createRenderPipeline({
        layout: pipelineLayout,
        vertex: { 
          module: cadModule, 
          entryPoint: 'vs_cad',
          buffers: [
            { arrayStride: 12, attributes: [{ shaderLocation: 0, offset: 0, format: 'float32x3' }] }, // positions
            { arrayStride: 12, attributes: [{ shaderLocation: 1, offset: 0, format: 'float32x3' }] }, // normals
            { arrayStride: 4,  attributes: [{ shaderLocation: 2, offset: 0, format: 'uint32' }] }     // face_ids
          ]
        },
        fragment: { module: cadModule, entryPoint: 'fs_cad', targets: [{ format: fmt }] },
        primitive: { topology: 'triangle-list', cullMode: 'back' },
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
      log(`✓ pipelines готовы — PARTICLE + CAD`, '#34d399');
"##;
