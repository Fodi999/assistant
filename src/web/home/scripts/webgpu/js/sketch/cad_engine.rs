// ── JS: CAD Engine Adapter ───────────────────────────────────────────────────
// Domain: Sketch — single entry point for ALL geometry creation.
//
// Architecture:
//
//     Frontend Tools (Line, Rect, Circle, Copy …)
//                    │
//                    ▼
//     __createPointViaEngine / __createEdgeViaEngine      (facade — unchanged API)
//                    │
//                    ▼
//     __cadAddPoint / __cadAddEdge                        (this file)
//                    │
//          ┌─────────┴─────────┐
//          ▼                   ▼
//        WASM                Backend
//      (instant,            (async sync,
//      apply at once)       source of truth)
//                    │
//                    ▼
//                Database
//
// Loaded AFTER:
//   - sketch_state.rs        (defines legacy __createPointViaEngine / __createEdgeViaEngine)
//   - sketch_backend.rs      (defines __backendAddPoint / __backendAddEdge)
//   - sketch_wasm.rs         (defines __ensureSketchWasm, __wasmAddPoint, __wasmAddEdge)
//
// Replaces those legacy facade functions so every tool transparently uses
// "WASM-first + backend-sync" without needing to know the engine mode.

pub const JS: &str = r##"
      // ── CAD engine runtime state (debug / inspector) ─────────────────────
      window.cadState = {
        wasmStatus:   'unknown',   // 'unknown' | 'ready' | 'loading' | 'error'
        backendStatus:'unknown',   // 'unknown' | 'ok' | 'pending' | 'offline' | 'diff' | 'error'
        lastWasmMs:    0,
        lastBackendMs: 0,
        lastSyncMs:    0,
        pending:       0,          // backend ops in flight
        diffs:         0,
        nextCommandId: 1,
      };
      sketchState.cadSyncStatus = '—';

      function __cadNextId() {
        const id = cadState.nextCommandId++;
        return 'cad_' + Date.now().toString(36) + '_' + id;
      }

      function __cadSetSyncStatus(s) {
        cadState.backendStatus = s;
        sketchState.cadSyncStatus = s;
        if (window.__updateSketchInspector) window.__updateSketchInspector();
      }

      function __cadSetWasmStatus() {
        if (typeof wasmState !== 'undefined' && wasmState && wasmState.status) {
          cadState.wasmStatus = wasmState.status;
        }
      }

      // ── Backend sync helpers ────────────────────────────────────────────
      // Fire-and-forget POST. The pre-sketch snapshot is what was active
      // *before* the WASM op so the server applies the same input.
      async function __cadBackendPoint(preSketch, gx, gy, gz, wasmResult, commandId) {
        cadState.pending++;
        __cadSetSyncStatus('pending');
        try {
          const t0 = performance.now();
          const res = await fetch('/api/matter/sketch/add-point', {
            method:  'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
              commandId,
              sketch:       preSketch,
              workingPlane: sketchState.workingPlane,
              gridSize:     sketchState.gridSize,
              gx: gx | 0, gy: gy | 0, gz: gz | 0,
              ignorePlaneConstraint: true,
            }),
          });
          const dt = performance.now() - t0;
          cadState.lastBackendMs = dt;
          cadState.lastSyncMs    = dt;
          sketchState.lastBackendMs = dt;
          if (window.__perfSample) window.__perfSample('backend', dt);
          if (!res.ok) {
            __cadSetSyncStatus('error');
            if (window.__perfMarkBackendError) window.__perfMarkBackendError();
            return null;
          }
          const json = await res.json();
          __cadReconcile(json, wasmResult);
          return json;
        } catch (e) {
          __cadSetSyncStatus('offline');
          if (window.__perfMarkBackendError) window.__perfMarkBackendError();
          return null;
        } finally {
          cadState.pending = Math.max(0, cadState.pending - 1);
          if (window.__updateSketchInspector) window.__updateSketchInspector();
        }
      }

      async function __cadBackendEdge(preSketch, startRef, endRef, kind, wasmResult, commandId) {
        cadState.pending++;
        __cadSetSyncStatus('pending');
        try {
          const t0 = performance.now();
          const res = await fetch('/api/matter/sketch/add-edge', {
            method:  'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
              commandId,
              sketch:       preSketch,
              workingPlane: sketchState.workingPlane,
              gridSize:     sketchState.gridSize,
              start: startRef,
              end:   endRef,
              kind:  kind || 'normal',
              ignorePlaneConstraint: true,
            }),
          });
          const dt = performance.now() - t0;
          cadState.lastBackendMs = dt;
          cadState.lastSyncMs    = dt;
          sketchState.lastBackendMs = dt;
          if (window.__perfSample) window.__perfSample('backend', dt);
          if (!res.ok) {
            __cadSetSyncStatus('error');
            if (window.__perfMarkBackendError) window.__perfMarkBackendError();
            return null;
          }
          const json = await res.json();
          __cadReconcile(json, wasmResult);
          return json;
        } catch (e) {
          __cadSetSyncStatus('offline');
          if (window.__perfMarkBackendError) window.__perfMarkBackendError();
          return null;
        } finally {
          cadState.pending = Math.max(0, cadState.pending - 1);
          if (window.__updateSketchInspector) window.__updateSketchInspector();
        }
      }

      // ── Reconciliation — backend is source of truth ─────────────────────
      // If signatures match → OK. If they differ → adopt backend state and
      // warn (the user already saw the WASM-applied geometry; we reconcile
      // silently to the authoritative version).
      function __cadReconcile(backendResult, wasmResult) {
        if (!backendResult || !backendResult.sketch) {
          __cadSetSyncStatus('error');
          return;
        }
        const sigW = window.__sketchSignature
          ? window.__sketchSignature(wasmResult    && wasmResult.sketch)
          : '';
        const sigB = window.__sketchSignature
          ? window.__sketchSignature(backendResult && backendResult.sketch)
          : '';
        if (sigW && sigB && sigW === sigB) {
          __cadSetSyncStatus('ok');
        } else {
          cadState.diffs++;
          console.warn('[CAD sync diff]', { wasmSignature: sigW, backendSignature: sigB });
          // Backend wins.
          if (window.__applyBackendSketchResult) {
            window.__applyBackendSketchResult(backendResult);
            if (window.__updateSketchInspector) window.__updateSketchInspector();
          }
          __cadSetSyncStatus('diff');
        }
      }

      // ── Public API: __cadAddPoint(gx, gy, gz, options) ──────────────────
      // options: { ignorePlaneConstraint?: bool, skipBackend?: bool }
      // Returns: { ok, pointId, created, message }
      window.__cadAddPoint = async function(gx, gy, gz, options) {
        const opts = options || {};
        const commandId = __cadNextId();
        const ignorePlane = opts.ignorePlaneConstraint !== false; // default true
        const skipBackend = !!opts.skipBackend;

        // 1) WASM first — instant local apply.
        let wasmResult = null;
        let wasmReady  = false;
        if (window.__ensureSketchWasm) {
          wasmReady = await window.__ensureSketchWasm();
        }
        __cadSetWasmStatus();

        const preSketch = window.__sketchToJSON();

        if (wasmReady && window.__wasmAddPoint) {
          const t0 = performance.now();
          wasmResult = window.__wasmAddPoint({
            commandId,
            sketch:       preSketch,
            workingPlane: sketchState.workingPlane || 'XZ',
            gridSize:     sketchState.gridSize,
            gx: gx | 0, gy: gy | 0, gz: gz | 0,
            ignorePlaneConstraint: ignorePlane,
          });
          const dt = performance.now() - t0;
          cadState.lastWasmMs = dt;
          sketchState.lastWasmMs = dt;
          if (window.__perfSample) window.__perfSample('wasm', dt);
          if (wasmResult && wasmResult.ok && wasmResult.sketch) {
            window.__applyBackendSketchResult(wasmResult);
            if (window.__updateSketchInspector) window.__updateSketchInspector();
          }
        }

        // 2) Backend sync (async, fire-and-forget but awaitable).
        let backendResult = null;
        if (!skipBackend) {
          backendResult = await __cadBackendPoint(preSketch, gx, gy, gz, wasmResult, commandId);
        }

        // 3) Pick best pointId: backend > wasm.
        const source = (backendResult && backendResult.ok) ? backendResult : wasmResult;
        if (!source || !source.ok) {
          const msg = (source && source.message) || 'CAD: add-point failed';
          return { ok: false, error: msg };
        }
        const pid = source.createdPointId || source.reusedPointId;
        return {
          ok: true,
          pointId: pid,
          created: !!source.createdPointId,
          message: source.message,
        };
      };

      // ── Public API: __cadAddEdge(aId, bId, kind, options) ───────────────
      // options: { ignorePlaneConstraint?: bool, skipBackend?: bool }
      // Returns: { ok, edgeId, message }
      window.__cadAddEdge = async function(aId, bId, kind, options) {
        const opts = options || {};
        const commandId = __cadNextId();
        const ignorePlane = opts.ignorePlaneConstraint !== false;
        const skipBackend = !!opts.skipBackend;

        if (!aId || !bId) return { ok: false, error: 'CAD: missing endpoint id' };
        if (aId === bId)  return { ok: false, error: 'CAD: self-loop' };

        const startRef = { pointId: aId };
        const endRef   = { pointId: bId };
        const edgeKind = kind || 'normal';

        let wasmResult = null;
        let wasmReady  = false;
        if (window.__ensureSketchWasm) {
          wasmReady = await window.__ensureSketchWasm();
        }
        __cadSetWasmStatus();

        const preSketch = window.__sketchToJSON();

        if (wasmReady && window.__wasmAddEdge) {
          const t0 = performance.now();
          wasmResult = window.__wasmAddEdge({
            commandId,
            sketch:       preSketch,
            workingPlane: sketchState.workingPlane || 'XZ',
            gridSize:     sketchState.gridSize,
            start: startRef,
            end:   endRef,
            kind:  edgeKind,
            ignorePlaneConstraint: ignorePlane,
          });
          const dt = performance.now() - t0;
          cadState.lastWasmMs = dt;
          sketchState.lastWasmMs = dt;
          if (window.__perfSample) window.__perfSample('wasm', dt);
          if (wasmResult && wasmResult.sketch) {
            window.__applyBackendSketchResult(wasmResult);
            // Stamp edge kind locally for the WASM result (engine returns 'normal'
            // edges; tools-specific kind is a frontend label).
            if (wasmResult.createdEdgeId && edgeKind !== 'normal') {
              const e = sketchState.edges.find(x => x.id === wasmResult.createdEdgeId);
              if (e) e.kind = edgeKind;
            }
            if (window.__updateSketchInspector) window.__updateSketchInspector();
          }
        }

        let backendResult = null;
        if (!skipBackend) {
          backendResult = await __cadBackendEdge(preSketch, startRef, endRef, edgeKind, wasmResult, commandId);
        }

        const source = (backendResult && backendResult.ok) ? backendResult : wasmResult;
        if (!source || !source.ok) {
          // Edge may already exist — engine returns ok:false but sketch is unchanged.
          const msg = (source && source.message) || 'CAD: add-edge failed';
          return { ok: false, error: msg };
        }
        return {
          ok: true,
          edgeId: source.createdEdgeId,
          message: source.message,
        };
      };

      // ── Validation passthrough ──────────────────────────────────────────
      window.__cadValidate = async function() {
        if (window.__wasmValidateSketch) {
          return await window.__wasmValidateSketch();
        }
        return null;
      };

      // ── Move point (WASM-first + backend-sync) ──────────────────────────
      // Mirrors __cadAddPoint pattern, with a synchronous backend fallback
      // for the case when WASM is unavailable (e.g. stale cached module).
      window.__cadMovePoint = async function(pointId, gx, gy, gz, options) {
        const opts = options || {};
        const commandId = __cadNextId();
        const skipBackend = !!opts.skipBackend;

        console.log('[MovePoint] request', { pointId, gx, gy, gz, commandId });
        console.log('[MovePoint] sketchState', {
          gridSize: sketchState.gridSize,
          workingPlane: sketchState.workingPlane,
          points: (sketchState.points || []).length,
          edges: (sketchState.edges || []).length,
        });

        // 1) WASM first — instant local apply.
        let wasmResult = null;
        let wasmReady  = false;
        if (window.__ensureSketchWasm) {
          wasmReady = await window.__ensureSketchWasm();
        }
        __cadSetWasmStatus();

        const preSketch = window.__sketchToJSON();
        const payload = {
          commandId,
          sketch:                preSketch,
          workingPlane:          sketchState.workingPlane || 'XZ',
          gridSize:              sketchState.gridSize,
          pointId,
          gx: gx | 0, gy: gy | 0, gz: gz | 0,
          ignorePlaneConstraint: true,
        };

        if (wasmReady && window.__wasmMovePoint) {
          const t0 = performance.now();
          wasmResult = window.__wasmMovePoint(payload);
          const dt = performance.now() - t0;
          cadState.lastWasmMs    = dt;
          sketchState.lastWasmMs = dt;
          if (window.__perfSample) window.__perfSample('wasm', dt);
          console.log('[MovePoint] wasmResult', wasmResult);
          if (wasmResult && wasmResult.ok && wasmResult.sketch) {
            window.__applyBackendSketchResult(wasmResult);
            if (window.__updateSketchInspector) window.__updateSketchInspector();
          }
        } else {
          console.warn('[MovePoint] WASM not ready', { wasmReady, hasFn: !!window.__wasmMovePoint });
        }

        const wasmOk = !!(wasmResult && wasmResult.ok);

        // 2a) WASM SUCCESS → fire-and-forget backend sync, return immediately.
        if (wasmOk) {
          if (!skipBackend) {
            (async () => {
              cadState.pending++;
              __cadSetSyncStatus('pending');
              try {
                const t0 = performance.now();
                const res = await fetch('/api/matter/sketch/move-point', {
                  method:  'POST',
                  headers: { 'Content-Type': 'application/json' },
                  body:    JSON.stringify(payload),
                });
                const dt = performance.now() - t0;
                cadState.lastBackendMs   = dt;
                cadState.lastSyncMs      = dt;
                sketchState.lastBackendMs = dt;
                if (window.__perfSample) window.__perfSample('backend', dt);
                if (res.ok) {
                  const json = await res.json();
                  console.log('[MovePoint] backendResult (bg)', json);
                  __cadReconcile(json, wasmResult);
                } else {
                  console.warn('[MovePoint] backend HTTP', res.status);
                  __cadSetSyncStatus('error');
                  if (window.__perfMarkBackendError) window.__perfMarkBackendError();
                }
              } catch (err) {
                console.warn('[MovePoint] backend offline', err);
                __cadSetSyncStatus('offline');
                if (window.__perfMarkBackendError) window.__perfMarkBackendError();
              } finally {
                cadState.pending = Math.max(0, cadState.pending - 1);
                if (window.__updateSketchInspector) window.__updateSketchInspector();
              }
            })();
          }
          return { ok: true, pointId, source: 'wasm' };
        }

        // 2b) WASM FAILED / UNAVAILABLE → await backend synchronously so
        //     the caller learns the real outcome rather than seeing a false
        //     "failed" while the backend is still in flight.
        if (skipBackend) {
          return { ok: false, error: (wasmResult && wasmResult.message) || 'CAD: WASM failed, backend skipped' };
        }
        try {
          cadState.pending++;
          __cadSetSyncStatus('pending');
          const t0 = performance.now();
          const res = await fetch('/api/matter/sketch/move-point', {
            method:  'POST',
            headers: { 'Content-Type': 'application/json' },
            body:    JSON.stringify(payload),
          });
          const dt = performance.now() - t0;
          cadState.lastBackendMs   = dt;
          cadState.lastSyncMs      = dt;
          sketchState.lastBackendMs = dt;
          if (window.__perfSample) window.__perfSample('backend', dt);
          if (!res.ok) {
            __cadSetSyncStatus('error');
            if (window.__perfMarkBackendError) window.__perfMarkBackendError();
            return { ok: false, error: 'CAD: backend HTTP ' + res.status };
          }
          const json = await res.json();
          console.log('[MovePoint] backendResult (await)', json);
          if (json && json.ok && json.sketch) {
            // Apply the backend authoritative state directly.
            window.__applyBackendSketchResult(json);
            __cadReconcile(json, null);
            if (window.__updateSketchInspector) window.__updateSketchInspector();
            return { ok: true, pointId, source: 'backend', result: json };
          }
          return { ok: false, error: (json && json.message) || 'CAD: backend rejected move-point' };
        } catch (err) {
          console.warn('[MovePoint] backend offline (await)', err);
          __cadSetSyncStatus('offline');
          if (window.__perfMarkBackendError) window.__perfMarkBackendError();
          return { ok: false, error: 'CAD: backend offline' };
        } finally {
          cadState.pending = Math.max(0, cadState.pending - 1);
          if (window.__updateSketchInspector) window.__updateSketchInspector();
        }
      };

      // ── FACADE — override legacy engine helpers ─────────────────────────
      // All tools (Line, Rect, Circle, Copy Connect, presets in sketch_state)
      // call these two functions. By routing them through the CAD adapter we
      // get WASM-first + backend-sync everywhere with zero tool changes.
      window.__createPointViaEngine = async function(gx, gy, gz) {
        // Dedup pre-check — engine itself also dedupes, but this saves a roundtrip.
        const existing = window.__findPointAtGrid && window.__findPointAtGrid(gx, gy, gz);
        if (existing) return existing.id;
        const r = await window.__cadAddPoint(gx, gy, gz, { ignorePlaneConstraint: true });
        return r && r.ok ? r.pointId : null;
      };

      window.__createEdgeViaEngine = async function(aId, bId, kind) {
        if (!aId || !bId || aId === bId) return null;
        const existing = window.__findEdgeBetween && window.__findEdgeBetween(aId, bId);
        if (existing) return existing.id;
        const r = await window.__cadAddEdge(aId, bId, kind || 'normal', { ignorePlaneConstraint: true });
        return r && r.ok ? r.edgeId : null;
      };

      window.__movePointViaEngine = async function(pointId, gx, gy, gz) {
        const r = await window.__cadMovePoint(pointId, gx, gy, gz, { ignorePlaneConstraint: true });
        return r || { ok: false, error: 'move-point: no result' };
      };

      // ── Auto-bootstrap WASM on first use ────────────────────────────────
      // CAD Engine = "WASM-first + backend-sync" is the only mode users see.
      // We do NOT touch sketchState.engineMode — that flag is now debug-only.
      if (window.__ensureSketchWasm) {
        window.__ensureSketchWasm().then(() => {
          __cadSetWasmStatus();
          if (window.__updateSketchInspector) window.__updateSketchInspector();
        });
      }
"##;
