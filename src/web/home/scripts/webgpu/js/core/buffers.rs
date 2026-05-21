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
      //   u8: objectX, objectY, objectZ, _
      //   u9: floorGridScale, orthoFlag, isSelected, _
      //   u10: objectRotX, objectRotY, objectRotZ, _
      //   u11: objectScaleX, objectScaleY, objectScaleZ, _
      const uniformBuf = device.createBuffer({
        size: 256,
        usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
      });

      // ── CAD Mesh Buffers (Полигональная сетка от geometry-kernel) ──
      // Выделяем пустые буферы с запасом на 100k вершин и треугольников.
      // Позже мы их обновим через device.queue.writeBuffer
      let cadPosBuf = device.createBuffer({ size: 100000 * 3 * 4, usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST });
      let cadNormalBuf = device.createBuffer({ size: 100000 * 3 * 4, usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST });
      let cadFaceIdBuf = device.createBuffer({ size: 100000 * 4, usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST });
      let cadIndexBuf = device.createBuffer({ size: 100000 * 3 * 4, usage: GPUBufferUsage.INDEX | GPUBufferUsage.COPY_DST });
      let cadIndexCount = 0;

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

      // ── Multi-body scene store ─────────────────────────────────
      // CAD.renderBodies   — committed bodies (id, visible, positions, ...)
      // CAD._previewBody   — single transient preview slot (gizmo drag)
      // CAD.renderer.*     — public API: setPreviewBody, commitPreviewAsBody,
      //                      commitBody, removeBody, setBodyVisible,
      //                      rebuildSolidScene, clearAll
      // Face metadata: each body gets a face_id base of (bodyIdx+1)*FACE_ID_STRIDE.
      // Preview body uses base 998*FACE_ID_STRIDE (reserved).
      window.CAD = window.CAD || {};
      window.CAD.renderBodies = window.CAD.renderBodies || [];
      window.CAD._previewBody = null;

      const MAX_VERTS = 100000;
      const MAX_TRIS  = 100000;
      const FACE_ID_STRIDE = 1000;

      window.CAD.renderer = window.CAD.renderer || {};
      window.CAD.renderer._faceMeta = {};

      function _writeMergedBuffers(posArr, nrmArr, idxArr, fidArr) {
        const pos = new Float32Array(posArr);
        const nrm = new Float32Array(nrmArr);
        const idx = new Uint32Array(idxArr);
        const fid = new Uint32Array(fidArr);
        if (pos.length / 3 > MAX_VERTS) {
          console.warn('[CAD.renderer] vertex overflow: ' + (pos.length/3) + ' > ' + MAX_VERTS);
        }
        if (idx.length / 3 > MAX_TRIS) {
          console.warn('[CAD.renderer] triangle overflow: ' + (idx.length/3) + ' > ' + MAX_TRIS);
        }
        device.queue.writeBuffer(cadPosBuf,    0, pos);
        device.queue.writeBuffer(cadNormalBuf, 0, nrm);
        device.queue.writeBuffer(cadFaceIdBuf, 0, fid);
        device.queue.writeBuffer(cadIndexBuf,  0, idx);
        cadIndexCount = idx.length;
      }

      function _mergeBodyInto(body, bodyIdx, vertOffsetIn, outPos, outNrm, outIdx, outFid) {
        const pos = body.positions, nrm = body.normals;
        const idx = body.indices,   fid = body.faceIds;
        const vcount = (pos.length / 3) | 0;
        const faceBase = (bodyIdx + 1) * FACE_ID_STRIDE;
        body.vertexOffset = vertOffsetIn;
        body.indexOffset  = outIdx.length;
        for (let i = 0; i < pos.length; i++) outPos.push(pos[i]);
        for (let i = 0; i < nrm.length; i++) outNrm.push(nrm[i] || 0);
        for (let i = 0; i < fid.length; i++) {
          const lf = fid[i] | 0;
          const gf = faceBase + lf;
          outFid.push(gf);
          window.CAD.renderer._faceMeta[gf] = { bodyId: body.id, localFaceId: lf };
        }
        for (let i = 0; i < idx.length; i++) outIdx.push(idx[i] + vertOffsetIn);
        return vcount;
      }

      window.CAD.renderer.rebuildSolidScene = function() {
        try {
          window.CAD.renderer._faceMeta = {};
          const bodies = (window.CAD.renderBodies || []).filter(b => b.visible !== false && b.positions && b.indices);
          const outPos = [], outNrm = [], outIdx = [], outFid = [];
          let vo = 0;
          bodies.forEach(function(b, i) {
            vo += _mergeBodyInto(b, i, vo, outPos, outNrm, outIdx, outFid);
          });
          const pv = window.CAD._previewBody;
          if (pv && pv.positions && pv.indices) {
            _mergeBodyInto(pv, 998, vo, outPos, outNrm, outIdx, outFid);
          }
          if (outIdx.length === 0) {
            cadIndexCount = 0;
            return;
          }
          _writeMergedBuffers(outPos, outNrm, outIdx, outFid);
        } catch (e) {
          console.warn('[CAD.renderer] rebuild failed:', e);
        }
      };

      function _resultToBody(result, id, featureId) {
        const posLen = result.positions.length;
        return {
          id:            id || null,
          featureId:     featureId || null,
          visible:       true,
          positions:     (result.positions instanceof Float32Array) ? result.positions : new Float32Array(result.positions),
          normals:       result.normals ? ((result.normals instanceof Float32Array) ? result.normals : new Float32Array(result.normals)) : new Float32Array(posLen),
          indices:       (result.indices instanceof Uint32Array) ? result.indices : new Uint32Array(result.indices),
          faceIds:       result.face_ids ? ((result.face_ids instanceof Uint32Array) ? result.face_ids : new Uint32Array(result.face_ids)) : new Uint32Array((posLen / 3) | 0).fill(1),
          faces:         result.faces || [],
          vertexCount:   result.vertex_count   || ((posLen / 3) | 0),
          triangleCount: result.triangle_count || ((result.indices.length / 3) | 0),
          objData:       result.obj_data || null,
          color:         null,
          vertexOffset:  0,
          indexOffset:   0,
        };
      }

      window.CAD.renderer.setPreviewBody = function(result) {
        if (!result || !result.positions || !result.indices) {
          window.CAD._previewBody = null;
        } else {
          window.CAD._previewBody = _resultToBody(result, '__preview__', null);
        }
        window.CAD.renderer.rebuildSolidScene();
      };

      window.CAD.renderer.commitPreviewAsBody = function(bodyId, featureId) {
        const pv = window.CAD._previewBody;
        if (!pv) { console.warn('[CAD.renderer] commitPreviewAsBody: no preview'); return null; }
        pv.id        = bodyId    || ('body_' + (window.CAD.renderBodies.length + 1));
        pv.featureId = featureId || null;
        window.CAD.renderBodies.push(pv);
        window.CAD._previewBody = null;
        window.CAD.renderer.rebuildSolidScene();
        console.log('[CAD.renderer] +body ' + pv.id + ' (total=' + window.CAD.renderBodies.length + ')');
        return pv;
      };

      window.CAD.renderer.commitBody = function(result, bodyId, featureId) {
        if (!result || !result.positions || !result.indices) {
          console.warn('[CAD.renderer] commitBody: invalid result'); return null;
        }
        const b = _resultToBody(result, bodyId, featureId);
        window.CAD.renderBodies.push(b);
        window.CAD._previewBody = null;
        window.CAD.renderer.rebuildSolidScene();
        return b;
      };

      window.CAD.renderer.removeBody = function(bodyId) {
        const arr = window.CAD.renderBodies;
        const i = arr.findIndex(b => b.id === bodyId);
        if (i >= 0) { arr.splice(i, 1); window.CAD.renderer.rebuildSolidScene(); return true; }
        return false;
      };

      window.CAD.renderer.setBodyVisible = function(bodyId, visible) {
        const b = (window.CAD.renderBodies || []).find(x => x.id === bodyId);
        if (b) {
          b.visible = !!visible;
          window.CAD.renderer.rebuildSolidScene();
          return true;
        }
        return false;
      };

      window.CAD.renderer.clearAll = function() {
        window.CAD.renderBodies = [];
        window.CAD._previewBody = null;
        cadIndexCount = 0;
      };

      // Debug helper: dump current scene state to console.
      window.CAD.debug = window.CAD.debug || {};
      window.CAD.debug.dumpRenderBodies = function() {
        const arr = window.CAD.renderBodies || [];
        const vis = arr.filter(b => b.visible !== false);
        const totalV = arr.reduce((s,b) => s + (b.vertexCount    || 0), 0);
        const totalT = arr.reduce((s,b) => s + (b.triangleCount  || 0), 0);
        console.log('[CAD.debug] bodies=' + arr.length + ' visible=' + vis.length +
          ' totalV=' + totalV + ' totalT=' + totalT +
          ' cadIndexCount=' + cadIndexCount +
          (window.CAD._previewBody ? ' (+preview)' : ''));
        try {
          console.table(arr.map(b => ({
            id: b.id, featureId: b.featureId, visible: b.visible,
            verts: b.vertexCount, tris: b.triangleCount,
            vOff: b.vertexOffset, iOff: b.indexOffset,
          })));
        } catch(_) {}
        return { count: arr.length, visible: vis.length, totalV: totalV, totalT: totalT, cadIndexCount: cadIndexCount };
      };

      // ── Resolve globalFaceId → body / localFaceId / face metadata ──
      //   Strategy A: direct _faceMeta lookup (built during rebuild).
      //   Strategy B: arithmetic fallback — bodySlot = floor(gf / 1000),
      //               localFaceId = gf % 1000. bodySlot is 1-based, 998 = preview.
      window.CAD.renderer.resolveFaceId = function(globalFaceId) {
        const gf = globalFaceId | 0;
        if (!gf) return null;
        let bodyId = null, localFaceId = 0;
        const meta = window.CAD.renderer._faceMeta && window.CAD.renderer._faceMeta[gf];
        if (meta) {
          bodyId      = meta.bodyId;
          localFaceId = meta.localFaceId;
        } else {
          const slot = Math.floor(gf / FACE_ID_STRIDE);
          localFaceId = gf % FACE_ID_STRIDE;
          if (slot === 998) {
            // preview
            const pv = window.CAD._previewBody;
            if (pv) bodyId = pv.id;
          } else if (slot >= 1) {
            const idx = slot - 1;
            const arr = window.CAD.renderBodies || [];
            if (idx < arr.length) bodyId = arr[idx].id;
          }
        }
        if (!bodyId) return null;
        const arr = window.CAD.renderBodies || [];
        let body = arr.find(b => b.id === bodyId);
        let isPreview = false;
        if (!body && window.CAD._previewBody && window.CAD._previewBody.id === bodyId) {
          body = window.CAD._previewBody;
          isPreview = true;
        }
        if (!body) return null;
        const face = (body.faces || []).find(f => f && f.face_id === localFaceId) || null;
        return {
          globalFaceId: gf,
          bodyId:       bodyId,
          localFaceId:  localFaceId,
          body:         body,
          featureId:    body.featureId || null,
          face:         face,
          isPreview:    isPreview,
        };
      };

      window.CAD.debug.dumpPickedFace = function(globalFaceId) {
        const r = window.CAD.renderer.resolveFaceId(globalFaceId);
        if (!r) {
          console.warn('[CAD.debug] dumpPickedFace: globalFaceId=' + globalFaceId + ' → no body resolved');
          return null;
        }
        console.log('[CAD.debug] picked face:', {
          globalFaceId: r.globalFaceId,
          bodyId:       r.bodyId,
          localFaceId:  r.localFaceId,
          featureId:    r.featureId,
          isPreview:    r.isPreview,
          bodyVerts:    r.body.vertexCount,
          bodyTris:     r.body.triangleCount,
          face:         r.face ? {
            id:       r.face.face_id,
            source:   r.face.source_face_id,
            normal:   r.face.normal,
            center:   r.face.center,
            verts:    r.face.vertex_count,
          } : null,
        });
        return r;
      };

      // ── Upload a solid mesh from the geometry kernel ──────────────
      // Both preview drags and final commits enter through here. We route
      // every upload into the *preview* slot — then `CAD.document` promotes
      // the preview into a permanent body via commitPreviewAsBody() in the
      // wrapper around __commitSolidExtrude (see ui/document.rs).
      // If multi-body path fails for any reason, fall back to legacy
      // single-body buffer write so the app keeps working.
      window.__uploadSolidToScene = function(result) {
        if (!result || !result.positions || !result.indices) return;
        try {
          window.CAD.renderer.setPreviewBody(result);
          console.log('[CAD upload] preview: verts=' + ((result.positions.length/3)|0) +
            ' tris=' + ((result.indices.length/3)|0) +
            ' committed=' + window.CAD.renderBodies.length);
        } catch (e) {
          console.warn('[CAD upload] multi-body path failed, falling back to single-body:', e);
          const pos = new Float32Array(result.positions);
          const nrm = new Float32Array(result.normals || new Array(result.positions.length).fill(0));
          const fid = new Uint32Array(result.face_ids || new Array(result.positions.length / 3).fill(1));
          const idx = new Uint32Array(result.indices);
          device.queue.writeBuffer(cadPosBuf,    0, pos);
          device.queue.writeBuffer(cadNormalBuf, 0, nrm);
          device.queue.writeBuffer(cadFaceIdBuf, 0, fid);
          device.queue.writeBuffer(cadIndexBuf,  0, idx);
          cadIndexCount = idx.length;
        }
      };
"##;
