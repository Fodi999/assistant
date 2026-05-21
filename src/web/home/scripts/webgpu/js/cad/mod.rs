// ── CAD subsystem ─────────────────────────────────────────────────────────
//
//  Professional CAD architecture separating concerns:
//
//    geometry_engine  (Rust/WASM)   — pure geometry math
//    scene/model      (future)       — object/face/edge/vertex storage
//    interaction/                    — picking, selection, highlight, overlays
//      ├─ picking/                   — raycast + pickers (face/edge/vertex)
//      ├─ selection/                 — state store + selection modes
//      ├─ highlight/                 — visual feedback bridge (→ UBO)
//      └─ overlays/                  — debug panels & viewport overlays
//
//  All interaction state is owned by `window.CadInteraction.*`, not
//  scattered globals. Legacy globals (`window.__solidSelected` etc.) remain
//  as read-only shims for the render loop until full migration is complete.

pub mod interaction;
