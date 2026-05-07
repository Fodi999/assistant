// ── JS: GPU buffer and bind-group allocation ──────────────────────────────────────
// Domain: Infrastructure — uniform buffer, storage buffer, bind group layout.

pub const JS: &str = r##"
      // ── 5. GPU buffers ──────────────────────────────────────────
      // Uniform layout (9 × vec4 = 144 bytes):
      //   u0: time, w, h, pushStrength
      //   u1: roX, roY, roZ, _
      //   u2: rightX, rightY, rightZ, _
      //   u3: upX, upY, upZ, _
      //   u4: fwdX, fwdY, fwdZ, _
      //   u5: mouseX, mouseY, mouseActive, shapeExponent
      //   u6: formMix(0..1), formMode(0=cloud,1=cube,2=wall), formA, formScale
      //   u7: cellSdfOn, cellRadius, colorMode(0/1/2), hideLow(0/1)
      //   u8: objectX, objectY, objectZ, objectScale  (scene placement)
      //   u9: floorGridScale, _, _, _
      const uniformBuf = device.createBuffer({
        size: 160,
        usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
      });

      let sphereBuf;
      try {
        sphereBuf = device.createBuffer({
          size: MAX_PARTICLES * PARTICLE_STRIDE,
          usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
        });
      } catch (e) {
        setBadge('✗ buffer alloc failed: ' + e.message, '#f87171');
        log('✗ не удалось выделить storage buffer — снизьте MAX_PARTICLES', '#f87171');
        return;
      }
      device.queue.writeBuffer(sphereBuf, 0, sphereData);

      // ── Timestamp query (GPU timing) ────────────────────────────
      let tsQuerySet = null, tsResolveBuf = null, tsReadBuf = null;
      if (hasTimestamp) {
        tsQuerySet  = device.createQuerySet({ type: 'timestamp', count: 2 });
        tsResolveBuf = device.createBuffer({ size: 16, usage: GPUBufferUsage.QUERY_RESOLVE | GPUBufferUsage.COPY_SRC });
        tsReadBuf    = device.createBuffer({ size: 16, usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ });
      }

      const bgl = device.createBindGroupLayout({ entries: [
        { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } },
        { binding: 1, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'read-only-storage' } },
      ]});
      const bindGroup = device.createBindGroup({
        layout: bgl,
        entries: [
          { binding: 0, resource: { buffer: uniformBuf } },
          { binding: 1, resource: { buffer: sphereBuf  } },
        ],
      });
      const pipelineLayout = device.createPipelineLayout({ bindGroupLayouts: [bgl] });
"##;
