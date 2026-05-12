// ── JS: Touchpad / wheel zoom ─────────────────────────────────────────────────
// Domain: User input — wheel event for zoom with zoom-to-cursor.
//
// Features:
//   1. { passive: false } so e.preventDefault() works.
//   2. deltaMode normalisation: pixel | line (*16) | page (*innerHeight).
//   3. Pinch detection: e.ctrlKey === true (browser synthesises on macOS pinch).
//   4. deltaY clamped to [-120, 120] before applying.
//   5. Factors: pinch = 0.005, wheel = 0.002.
//   6. Zoom-to-cursor: shifts cam.target so the world point under the cursor
//      stays fixed after the distance changes.
//   7. Guards against accidental point/edge creation during wheel.
//   CSS (applied once in init): canvas { touch-action:none; overscroll-behavior:none }

pub const JS: &str = r##"
      // ── Zoom-to-cursor helper ─────────────────────────────────
      // Returns the world position on the active working plane for a given NDC
      // coordinate, or null if no plane is set.
      function __worldUnderCursor(ndcX, ndcY) {
        if (!window.__raycastSketchPlane) return null;
        return window.__raycastSketchPlane(ndcX, ndcY);
      }

      // ── Wheel / pinch listener ────────────────────────────────
      canvas.addEventListener('wheel', (e) => {
        e.preventDefault();

        // 1. Normalise deltaY to pixel units.
        let dy = e.deltaY;
        if      (e.deltaMode === 1) dy *= 16;                  // DOM_DELTA_LINE
        else if (e.deltaMode === 2) dy *= window.innerHeight;  // DOM_DELTA_PAGE

        // 2. Clamp so a single trackpad gesture can't jump too far.
        dy = Math.max(-120, Math.min(120, dy));

        // 3. Choose factor: pinch (ctrlKey synthesised by macOS) vs. wheel.
        const isPinch = e.ctrlKey;
        const factor  = isPinch ? 0.005 : 0.002;

        // 4. Zoom-to-cursor: sample world point BEFORE distance change.
        const rect   = canvas.getBoundingClientRect();
        const ndcX   = ((e.clientX - rect.left) / rect.width)  * 2 - 1;
        const ndcY   = 1 - ((e.clientY - rect.top)  / rect.height) * 2;
        const before = __worldUnderCursor(ndcX, ndcY);

        // 5. Apply zoom.
        const oldDist = cam.dist;
        cam.dist = Math.max(0.5, Math.min(200, cam.dist * Math.exp(dy * factor)));

        // 6. Zoom-to-cursor: sample world point AFTER and shift cam.target.
        if (before) {
          const after = __worldUnderCursor(ndcX, ndcY);
          if (after) {
            cam.target[0] += before.freeX - after.freeX;
            cam.target[1] += before.freeY - after.freeY;
            cam.target[2] += before.freeZ - after.freeZ;
          }
        }

        // 7. Guard: do NOT fire sketch click/point during wheel.
        //    We flag a wheel in progress so mouse.rs pointerup ignores it.
        //    (dragMoved is already true during pointer-drag, wheel adds its own guard)
        if (window.__wheelZoomActive !== undefined) {
          clearTimeout(window.__wheelZoomTimer);
          window.__wheelZoomActive = true;
          window.__wheelZoomTimer  = setTimeout(() => { window.__wheelZoomActive = false; }, 150);
        }
      }, { passive: false });

      // Expose flag so __handleSketchClick can check it.
      window.__wheelZoomActive = false;
"##;
