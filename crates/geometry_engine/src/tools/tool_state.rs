// ── tools/tool_state.rs — Tool FSM (native Rust) ──────────────────────────────
//
// Tracks the active tool phase server-side in WASM memory.
// JS calls wasm_tool_click(tool, gx, gy, gz, plane) and gets back a ToolClickResult
// that contains either a SketchDelta (if the tool completed) or a phase update.

use super::types::SketchDelta;
use serde::{Deserialize, Serialize};

/// Phase within a multi-click tool.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ToolPhase {
    Idle,
    /// Waiting for second click (rect corner B, circle rim, etc.)
    WaitingSecond { gx: i64, gy: i64, gz: i64 },
}

/// Which tool is currently active.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ActiveTool {
    None,
    Rect,
    Circle { segments: usize },
    Grab,
    EdgeExtrude,
    SolidExtrude,
    CopyConnect,
}

impl Default for ActiveTool {
    fn default() -> Self { ActiveTool::None }
}

/// Persistent state of the tool FSM (lives in WASM memory).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolState {
    pub active: ActiveTool,
    pub phase: ToolPhase,
    pub working_plane: String,
    pub grid_size: f64,
    /// Monotonically increasing counter for unique ID generation.
    pub id_counter: u64,
}

impl Default for ToolState {
    fn default() -> Self {
        ToolState {
            active: ActiveTool::None,
            phase: ToolPhase::Idle,
            working_plane: "XZ".into(),
            grid_size: 0.01,
            id_counter: 1,
        }
    }
}

impl ToolState {
    pub fn new() -> Self { Self::default() }

    pub fn next_id(&mut self) -> u64 {
        let id = self.id_counter;
        self.id_counter += 1000;
        id
    }

    pub fn activate(&mut self, tool: ActiveTool, plane: String, grid_size: f64) {
        self.active = tool;
        self.phase = ToolPhase::Idle;
        self.working_plane = plane;
        self.grid_size = grid_size;
    }

    pub fn cancel(&mut self) {
        self.active = ActiveTool::None;
        self.phase = ToolPhase::Idle;
    }
}

/// Result of wasm_tool_click.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolClickResult {
    pub ok: bool,
    pub error: Option<String>,
    /// Current tool phase after this click.
    pub phase: ToolPhase,
    /// Status message to display.
    pub status: String,
    /// If Some, the tool completed and here is the delta to apply.
    pub delta: Option<SketchDelta>,
    /// True if the tool is still active (waiting for more input).
    pub still_active: bool,
}

impl ToolClickResult {
    pub fn waiting(phase: ToolPhase, status: impl Into<String>) -> Self {
        ToolClickResult {
            ok: true,
            error: None,
            phase,
            status: status.into(),
            delta: None,
            still_active: true,
        }
    }

    pub fn done(delta: SketchDelta, status: impl Into<String>) -> Self {
        ToolClickResult {
            ok: true,
            error: None,
            phase: ToolPhase::Idle,
            status: status.into(),
            delta: Some(delta),
            still_active: false,
        }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        let s: String = msg.into();
        ToolClickResult {
            ok: false,
            error: Some(s.clone()),
            phase: ToolPhase::Idle,
            status: s,
            delta: None,
            still_active: false,
        }
    }
}

/// Process a click at grid position (gx, gy, gz) for the currently active tool.
pub fn tool_click(state: &mut ToolState, gx: i64, gy: i64, gz: i64) -> ToolClickResult {
    let plane = state.working_plane.clone();
    match &state.active.clone() {
        ActiveTool::Rect => {
            match &state.phase.clone() {
                ToolPhase::Idle => {
                    state.phase = ToolPhase::WaitingSecond { gx, gy, gz };
                    ToolClickResult::waiting(
                        state.phase.clone(),
                        format!("⬡ Rect corner A set · click opposite corner · Esc cancel"),
                    )
                }
                ToolPhase::WaitingSecond { gx: gx1, gy: gy1, gz: gz1 } => {
                    let (gx1, gy1, gz1) = (*gx1, *gy1, *gz1);
                    let id_offset = state.next_id();
                    let delta = super::rect::create_rect(super::rect::RectInput {
                        gx1, gy1, gz1, gx2: gx, gy2: gy, gz2: gz,
                        plane: plane.clone(), id_offset,
                    });
                    if delta.ok {
                        state.phase = ToolPhase::Idle;
                        let np = delta.new_points.len();
                        let ne = delta.new_edges.len();
                        ToolClickResult::done(delta,
                            format!("✓ Rectangle ({} pts, {} edges)", np, ne))
                    } else {
                        ToolClickResult::err(delta.error.unwrap_or_default())
                    }
                }
            }
        }
        ActiveTool::Circle { segments } => {
            let segments = *segments;
            match &state.phase.clone() {
                ToolPhase::Idle => {
                    state.phase = ToolPhase::WaitingSecond { gx, gy, gz };
                    ToolClickResult::waiting(
                        state.phase.clone(),
                        String::from("⬤ Circle centre set · click rim point · Esc cancel"),
                    )
                }
                ToolPhase::WaitingSecond { gx: cx, gy: cy, gz: cz } => {
                    let (cx, cy, cz) = (*cx, *cy, *cz);
                    let radius = match plane.as_str() {
                        "XY" => {
                            let dx = (gx - cx) as f64;
                            let dy = (gy - cy) as f64;
                            (dx * dx + dy * dy).sqrt()
                        }
                        "YZ" => {
                            let dy = (gy - cy) as f64;
                            let dz = (gz - cz) as f64;
                            (dy * dy + dz * dz).sqrt()
                        }
                        _ => {
                            let dx = (gx - cx) as f64;
                            let dz = (gz - cz) as f64;
                            (dx * dx + dz * dz).sqrt()
                        }
                    };
                    let id_offset = state.next_id();
                    let delta = super::circle::create_circle(super::circle::CircleInput {
                        center_gx: cx, center_gy: cy, center_gz: cz,
                        radius, plane: plane.clone(), segments, id_offset,
                    });
                    if delta.ok {
                        state.phase = ToolPhase::Idle;
                        let np = delta.new_points.len();
                        ToolClickResult::done(delta,
                            format!("✓ Circle ({} pts, r={:.1} grid units)", np, radius))
                    } else {
                        ToolClickResult::err(delta.error.unwrap_or_default())
                    }
                }
            }
        }
        ActiveTool::None => ToolClickResult::err("no tool active"),
        _ => ToolClickResult::err("tool_click: tool does not use click FSM"),
    }
}
