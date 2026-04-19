use serde::{Deserialize, Serialize};
use time::Date;
use uuid::Uuid;

// ============================================================================
// Server-driven limits (loaded from DB, changeable without deploy)
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct ServerLimits {
    pub plans: i32,
    pub recipes: i32,
    pub scans: i32,
    pub optimize: i32,
    pub chats: i32,
    pub cost_plan: i32,
    pub cost_recipe: i32,
    pub cost_scan: i32,
    pub cost_optimize: i32,
    pub cost_chat: i32,
}

impl Default for ServerLimits {
    fn default() -> Self {
        Self {
            plans: 2, recipes: 2, scans: 1, optimize: 1, chats: 10,
            cost_plan: 5, cost_recipe: 3, cost_scan: 2, cost_optimize: 4, cost_chat: 1,
        }
    }
}

impl ServerLimits {
    pub fn daily_limit(&self, action: ActionType) -> i32 {
        match action {
            ActionType::GeneratePlan => self.plans,
            ActionType::CreateRecipe => self.recipes,
            ActionType::ScanReceipt => self.scans,
            ActionType::OptimizeDay => self.optimize,
            ActionType::AiChat => self.chats,
        }
    }

    pub fn action_cost(&self, action: ActionType) -> i32 {
        match action {
            ActionType::GeneratePlan => self.cost_plan,
            ActionType::CreateRecipe => self.cost_recipe,
            ActionType::ScanReceipt => self.cost_scan,
            ActionType::OptimizeDay => self.cost_optimize,
            ActionType::AiChat => self.cost_chat,
        }
    }
}

// Keep DailyLimits as fallback constants (used only if DB read fails)
pub struct DailyLimits;
impl DailyLimits {
    pub const PLANS: i32 = 2;
    pub const RECIPES: i32 = 2;
    pub const SCANS: i32 = 1;
    pub const OPTIMIZE: i32 = 1;
    pub const CHATS: i32 = 10;
}

// ============================================================================
// Action Types
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    GeneratePlan,
    CreateRecipe,
    ScanReceipt,
    OptimizeDay,
    AiChat,
}

impl ActionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ActionType::GeneratePlan => "generate_plan",
            ActionType::CreateRecipe => "create_recipe",
            ActionType::ScanReceipt => "scan_receipt",
            ActionType::OptimizeDay => "optimize_day",
            ActionType::AiChat => "ai_chat",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "generate_plan" => Some(ActionType::GeneratePlan),
            "create_recipe" => Some(ActionType::CreateRecipe),
            "scan_receipt" => Some(ActionType::ScanReceipt),
            "optimize_day" => Some(ActionType::OptimizeDay),
            "ai_chat" => Some(ActionType::AiChat),
            _ => None,
        }
    }

    /// Column name in user_usage table
    pub fn column(&self) -> &'static str {
        match self {
            ActionType::GeneratePlan => "plans_used",
            ActionType::CreateRecipe => "recipes_used",
            ActionType::ScanReceipt => "scans_used",
            ActionType::OptimizeDay => "optimize_used",
            ActionType::AiChat => "chats_used",
        }
    }
}

// ============================================================================
// Domain Models
// ============================================================================

/// Daily usage row for a user
#[derive(Debug, Clone, Serialize)]
pub struct DailyUsage {
    pub user_id: Uuid,
    pub date: Date,
    pub plans_used: i32,
    pub recipes_used: i32,
    pub scans_used: i32,
    pub optimize_used: i32,
    pub chats_used: i32,
}

impl DailyUsage {
    pub fn used_count(&self, action: ActionType) -> i32 {
        match action {
            ActionType::GeneratePlan => self.plans_used,
            ActionType::CreateRecipe => self.recipes_used,
            ActionType::ScanReceipt => self.scans_used,
            ActionType::OptimizeDay => self.optimize_used,
            ActionType::AiChat => self.chats_used,
        }
    }

    pub fn free_remaining(&self, action: ActionType, limits: &ServerLimits) -> i32 {
        (limits.daily_limit(action) - self.used_count(action)).max(0)
    }

    pub fn free_remaining_with(&self, action: ActionType, limits: &ServerLimits) -> i32 {
        (limits.daily_limit(action) - self.used_count(action)).max(0)
    }
}

/// Purchased action balance
#[derive(Debug, Clone, Serialize)]
pub struct ActionBalance {
    pub user_id: Uuid,
    pub purchased_actions: i32,
    pub total_purchased: i32,
    pub total_spent: i32,
    pub welcome_bonus: bool,
    pub last_weekly_bonus: Option<Date>,
}

/// Result of attempting an action
#[derive(Debug, Clone, Serialize)]
pub struct ActionResult {
    pub allowed: bool,
    pub source: ActionSource,
    pub reason: Option<DenyReason>,
    pub remaining_free: i32,
    pub purchased_actions_left: i32,
    pub warning: bool,
    pub message: Option<String>,
    pub usage: UsageSnapshot,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionSource {
    FreeTier,
    Purchased,
    Denied,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DenyReason {
    DailyLimitReached,
    InsufficientActions,
}

/// Compact usage snapshot included in every action response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageSnapshot {
    pub plans_left: i32,
    pub recipes_left: i32,
    pub scans_left: i32,
    pub optimize_left: i32,
    pub chats_left: i32,
    pub purchased_actions: i32,
}

/// Result for batch actions
#[derive(Debug, Clone, Serialize)]
pub struct BatchActionResult {
    pub results: Vec<BatchActionItem>,
    pub usage: UsageSnapshot,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchActionItem {
    pub action: String,
    pub allowed: bool,
    pub source: ActionSource,
    pub reason: Option<DenyReason>,
    pub message: Option<String>,
}
