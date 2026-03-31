// ─── Metrics — Observability for Recipe Generation Pipeline ─────────────────
//
// Structured logging + in-memory counters for production monitoring.
// Every generation event is logged with:
//   - dish_type, locale, model
//   - validation pass/fail, fix attempts
//   - generation time (ms)
//   - quality score + confidence
//
// Counters are exposed via /api/admin/lab-combos/metrics for dashboarding.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

// ── Global Counters ─────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct PipelineMetrics {
    pub generations_total: AtomicU64,
    pub generations_success: AtomicU64,
    pub generations_failed: AtomicU64,
    pub validations_passed: AtomicU64,
    pub validations_failed: AtomicU64,
    pub fix_attempts: AtomicU64,
    pub fix_successes: AtomicU64,
    pub ai_calls_total: AtomicU64,
    pub ai_errors: AtomicU64,
    pub total_generation_ms: AtomicU64,
}

static METRICS: OnceLock<PipelineMetrics> = OnceLock::new();

pub fn global_metrics() -> &'static PipelineMetrics {
    METRICS.get_or_init(|| PipelineMetrics {
        generations_total: AtomicU64::new(0),
        generations_success: AtomicU64::new(0),
        generations_failed: AtomicU64::new(0),
        validations_passed: AtomicU64::new(0),
        validations_failed: AtomicU64::new(0),
        fix_attempts: AtomicU64::new(0),
        fix_successes: AtomicU64::new(0),
        ai_calls_total: AtomicU64::new(0),
        ai_errors: AtomicU64::new(0),
        total_generation_ms: AtomicU64::new(0),
    })
}

impl PipelineMetrics {
    pub fn snapshot(&self) -> MetricsSnapshot {
        let total = self.generations_total.load(Ordering::Relaxed);
        let success = self.generations_success.load(Ordering::Relaxed);
        let failed = self.generations_failed.load(Ordering::Relaxed);
        let val_pass = self.validations_passed.load(Ordering::Relaxed);
        let val_fail = self.validations_failed.load(Ordering::Relaxed);
        let fixes = self.fix_attempts.load(Ordering::Relaxed);
        let fix_ok = self.fix_successes.load(Ordering::Relaxed);
        let ai_calls = self.ai_calls_total.load(Ordering::Relaxed);
        let ai_err = self.ai_errors.load(Ordering::Relaxed);
        let total_ms = self.total_generation_ms.load(Ordering::Relaxed);

        MetricsSnapshot {
            generations_total: total,
            generations_success: success,
            generations_failed: failed,
            success_rate: if total > 0 { success as f64 / total as f64 } else { 0.0 },
            validations_passed: val_pass,
            validations_failed: val_fail,
            validation_pass_rate: if val_pass + val_fail > 0 {
                val_pass as f64 / (val_pass + val_fail) as f64
            } else { 0.0 },
            fix_attempts: fixes,
            fix_successes: fix_ok,
            fix_success_rate: if fixes > 0 { fix_ok as f64 / fixes as f64 } else { 0.0 },
            ai_calls_total: ai_calls,
            ai_errors: ai_err,
            avg_generation_ms: if total > 0 { total_ms / total } else { 0 },
        }
    }
}

// ── Snapshot for JSON serialization ─────────────────────────────────────────

#[derive(Debug, serde::Serialize)]
pub struct MetricsSnapshot {
    pub generations_total: u64,
    pub generations_success: u64,
    pub generations_failed: u64,
    pub success_rate: f64,
    pub validations_passed: u64,
    pub validations_failed: u64,
    pub validation_pass_rate: f64,
    pub fix_attempts: u64,
    pub fix_successes: u64,
    pub fix_success_rate: f64,
    pub ai_calls_total: u64,
    pub ai_errors: u64,
    pub avg_generation_ms: u64,
}

// ── Generation Event Logger ─────────────────────────────────────────────────

pub struct GenerationTimer {
    start: Instant,
    dish_type: String,
    locale: String,
    model: String,
}

impl GenerationTimer {
    /// Start timing a generation event.
    pub fn start(dish_type: &str, locale: &str, model: &str) -> Self {
        let m = global_metrics();
        m.generations_total.fetch_add(1, Ordering::Relaxed);

        tracing::info!(
            dish_type = dish_type,
            locale = locale,
            model = model,
            "📊 recipe_generation.started"
        );

        Self {
            start: Instant::now(),
            dish_type: dish_type.to_string(),
            locale: locale.to_string(),
            model: model.to_string(),
        }
    }

    /// Record a successful generation.
    pub fn success(self, quality_score: u8, confidence: f32, fix_attempts: u32) {
        let elapsed = self.start.elapsed().as_millis() as u64;
        let m = global_metrics();
        m.generations_success.fetch_add(1, Ordering::Relaxed);
        m.total_generation_ms.fetch_add(elapsed, Ordering::Relaxed);

        tracing::info!(
            dish_type = %self.dish_type,
            locale = %self.locale,
            model = %self.model,
            duration_ms = elapsed,
            quality_score = quality_score,
            confidence = confidence,
            fix_attempts = fix_attempts,
            "📊 recipe_generation.success"
        );
    }

    /// Record a failed generation.
    pub fn failure(self, reason: &str) {
        let elapsed = self.start.elapsed().as_millis() as u64;
        let m = global_metrics();
        m.generations_failed.fetch_add(1, Ordering::Relaxed);
        m.total_generation_ms.fetch_add(elapsed, Ordering::Relaxed);

        tracing::warn!(
            dish_type = %self.dish_type,
            locale = %self.locale,
            model = %self.model,
            duration_ms = elapsed,
            reason = reason,
            "📊 recipe_generation.failed"
        );
    }
}

/// Log a validation result.
pub fn record_validation(passed: bool, dish_type: &str, problems: &[String]) {
    let m = global_metrics();
    if passed {
        m.validations_passed.fetch_add(1, Ordering::Relaxed);
        tracing::info!(
            dish_type = dish_type,
            "📊 recipe_validation.passed"
        );
    } else {
        m.validations_failed.fetch_add(1, Ordering::Relaxed);
        tracing::warn!(
            dish_type = dish_type,
            problem_count = problems.len(),
            problems = ?problems,
            "📊 recipe_validation.failed"
        );
    }
}

/// Log a fix attempt.
pub fn record_fix_attempt(success: bool, dish_type: &str) {
    let m = global_metrics();
    m.fix_attempts.fetch_add(1, Ordering::Relaxed);
    if success {
        m.fix_successes.fetch_add(1, Ordering::Relaxed);
        tracing::info!(
            dish_type = dish_type,
            "📊 recipe_fix.success"
        );
    } else {
        tracing::warn!(
            dish_type = dish_type,
            "📊 recipe_fix.failed"
        );
    }
}

/// Log an AI call.
pub fn record_ai_call(success: bool, model: &str) {
    let m = global_metrics();
    m.ai_calls_total.fetch_add(1, Ordering::Relaxed);
    if !success {
        m.ai_errors.fetch_add(1, Ordering::Relaxed);
        tracing::warn!(model = model, "📊 ai_call.error");
    }
}
