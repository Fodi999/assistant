// ── CAD Interaction subsystem ────────────────────────────────────────────
//
//  Load order (matters — later modules read from earlier ones):
//    1. selection/  — defines the store + modes
//    2. picking/    — raycast, face metadata, pickers
//    3. highlight/  — bridge selection → UBO globals (read by render_loop)
//    4. overlays/   — debug panels (read from selection + picking)
//
//  Public API (single namespace, no scattered globals):
//
//    window.CadInteraction = {
//      selection: { get, set, clear, mode, subscribe, snapshot },
//      picking:   { buildFaceMetadata, pickFace, makeCameraRay },
//      highlight: { syncToUbo, setHover, clear },
//      overlays:  { debug: { show, hide, update } },
//    }

pub mod selection;
pub mod picking;
pub mod highlight;
pub mod overlays;

/// Bootstrap script — must run before any other module touches CadInteraction.
pub const BOOTSTRAP_JS: &str = r##"
(function bootstrapCadInteraction() {
  if (window.CadInteraction) return;
  window.CadInteraction = {
    selection: null,   // filled by selection/selection_store.rs
    picking:   null,   // filled by picking/*
    highlight: null,   // filled by highlight/highlight_state.rs
    overlays:  {},     // filled by overlays/*
  };
  console.log('[CadInteraction] namespace ready');
})();
"##;
