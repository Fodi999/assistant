// ── Circle sketch state initialisation ───────────────────────────────────────
// sketchState.circle is initialised here so all tools can read it safely
// before circle_tool.rs loads.

pub const JS: &str = r##"
      // Circle tool state — initialised alongside the rest of sketchState
      sketchState.circle = sketchState.circle || { active: false, centerSnap: null };
"##;
