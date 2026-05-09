//! Laboratory use cases — orchestrates the repository and (later) engines.
//!
//! Every method receives the authenticated `owner_id` from the HTTP layer.
//! Tenancy is enforced *here* (not in handlers) so that future callers
//! (e.g. background jobs) can't accidentally bypass it.

use rust_decimal::Decimal;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::infrastructure::persistence::laboratory_repository::{
    LaboratoryRepository, NewLabProcessStep, NewLabProject, NewLabProjectAnalysis,
    NewLabProjectIngredient,
};
use crate::shared::{AppError, AppResult, Language};

use super::catalog_profile_adapter::CatalogProfileAdapter;
use super::copilot_engine;
use super::flavor_engine;
use super::process_engine::{self, LaboratoryProcessAnalysis};
use super::shelf_life_engine::{self, max_risk_level};
use super::types::{
    AddLabIngredientRequest, AddLabStepRequest, CopilotSuggestIngredient, CopilotSuggestResponse,
    CopilotSuggestStep, CreateLabProjectRequest, LabProcessStepDto, LabProjectAnalysisDto,
    LabProjectFull, LabProjectIngredientDto, LabProjectSummary,
};
use super::visual_story_engine::{self, LaboratoryVisualStory};

#[derive(Clone)]
pub struct LaboratoryService {
    repo: LaboratoryRepository,
    catalog: CatalogProfileAdapter,
}

impl LaboratoryService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            repo: LaboratoryRepository::new(pool.clone()),
            catalog: CatalogProfileAdapter::new(pool),
        }
    }

    // ── Projects ────────────────────────────────────────────────────────────

    pub async fn create_project(
        &self,
        owner_id: Uuid,
        req: CreateLabProjectRequest,
    ) -> AppResult<LabProjectFull> {
        let name = req.name.trim();
        if name.is_empty() {
            return Err(AppError::validation("Project name cannot be empty"));
        }
        if name.chars().count() > 200 {
            return Err(AppError::validation("Project name is too long (max 200)"));
        }

        let row = self
            .repo
            .insert_project(NewLabProject {
                owner_id,
                name: name.to_string(),
                description: req
                    .description
                    .map(|d| d.trim().to_string())
                    .filter(|s| !s.is_empty()),
                target_product_type: req
                    .target_product_type
                    .map(|t| t.trim().to_lowercase())
                    .filter(|s| !s.is_empty()),
            })
            .await?;

        // Brand-new project — no children yet.
        Ok(self.hydrate_project(row.into(), owner_id).await?)
    }

    pub async fn list_projects(&self, owner_id: Uuid) -> AppResult<Vec<LabProjectSummary>> {
        let rows = self.repo.list_projects_by_owner(owner_id).await?;
        Ok(rows.into_iter().map(LabProjectSummary::from).collect())
    }

    pub async fn get_project(&self, owner_id: Uuid, project_id: Uuid) -> AppResult<LabProjectFull> {
        let row = self
            .repo
            .get_project_for_owner(project_id, owner_id)
            .await?
            .ok_or_else(|| AppError::not_found("Laboratory project not found"))?;
        let summary: LabProjectSummary = row.into();
        self.hydrate_project(summary, owner_id).await
    }

    pub async fn delete_project(&self, owner_id: Uuid, project_id: Uuid) -> AppResult<()> {
        let removed = self
            .repo
            .delete_project_for_owner(project_id, owner_id)
            .await?;
        if !removed {
            return Err(AppError::not_found("Laboratory project not found"));
        }
        Ok(())
    }

    // ── Ingredients ─────────────────────────────────────────────────────────

    pub async fn add_ingredient(
        &self,
        owner_id: Uuid,
        project_id: Uuid,
        req: AddLabIngredientRequest,
    ) -> AppResult<LabProjectIngredientDto> {
        self.repo.assert_owner(project_id, owner_id).await?;

        let slug = req.ingredient_slug.trim().to_lowercase();
        if slug.is_empty() {
            return Err(AppError::validation("ingredient_slug cannot be empty"));
        }
        if req.quantity <= Decimal::ZERO {
            return Err(AppError::validation("quantity must be > 0"));
        }
        let unit = req.unit.trim().to_lowercase();
        if unit.is_empty() {
            return Err(AppError::validation("unit cannot be empty"));
        }

        let (row, merged) = self
            .repo
            .upsert_ingredient(NewLabProjectIngredient {
                project_id,
                ingredient_slug: slug,
                quantity: req.quantity,
                unit,
                role: req
                    .role
                    .map(|r| r.trim().to_lowercase())
                    .filter(|s| !s.is_empty()),
                sort_order: req.sort_order,
                notes: req
                    .notes
                    .map(|n| n.trim().to_string())
                    .filter(|s| !s.is_empty()),
            })
            .await?;
        let mut dto: LabProjectIngredientDto = row.into();
        if merged {
            dto.merged = Some(true);
        }
        Ok(dto)
    }

    pub async fn delete_ingredient(
        &self,
        owner_id: Uuid,
        project_id: Uuid,
        ingredient_id: Uuid,
    ) -> AppResult<()> {
        let removed = self
            .repo
            .delete_ingredient_for_owner(ingredient_id, project_id, owner_id)
            .await?;
        if !removed {
            return Err(AppError::not_found("Ingredient not found"));
        }
        Ok(())
    }

    // ── Process steps ───────────────────────────────────────────────────────

    pub async fn add_step(
        &self,
        owner_id: Uuid,
        project_id: Uuid,
        req: AddLabStepRequest,
    ) -> AppResult<LabProcessStepDto> {
        self.repo.assert_owner(project_id, owner_id).await?;

        let technique = req.technique.trim().to_lowercase();
        if technique.is_empty() {
            return Err(AppError::validation("technique cannot be empty"));
        }
        if let Some(d) = req.duration_min {
            if d < 0 {
                return Err(AppError::validation("duration_min must be >= 0"));
            }
        }

        let target_slugs: Option<Vec<String>> = req.target_slugs.map(|v| {
            v.into_iter()
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty())
                .collect()
        });

        // ── Duplicate-step guard (Laboratory v2) ─────────────────────────
        // Block trivially-identical back-to-back steps so the visual story
        // doesn't degenerate into "softening → softening → softening".
        // We compare against the *last* step of the project on:
        //   * technique
        //   * temperature_c
        //   * duration_min
        //   * target_slugs (order-insensitive)
        if let Some(prev) = self.repo.latest_step(project_id).await? {
            let prev_targets: Vec<String> = prev.target_slugs.clone().unwrap_or_default();
            let new_targets: Vec<String> = target_slugs.clone().unwrap_or_default();
            let same_targets = {
                let mut a = prev_targets.clone();
                let mut b = new_targets.clone();
                a.sort();
                b.sort();
                a == b
            };
            if prev.technique == technique
                && prev.temperature_c == req.temperature_c
                && prev.duration_min == req.duration_min
                && same_targets
            {
                return Err(AppError::conflict(
                    "DUPLICATE_STEP: похожий шаг уже идёт последним. \
                     Повторяйте его только если это технологически необходимо.",
                ));
            }
        }

        let row = self
            .repo
            .insert_step(NewLabProcessStep {
                project_id,
                order_index: req.order_index,
                technique,
                temperature_c: req.temperature_c,
                duration_min: req.duration_min,
                target_slugs,
                notes: req
                    .notes
                    .map(|n| n.trim().to_string())
                    .filter(|s| !s.is_empty()),
            })
            .await?;
        Ok(row.into())
    }

    pub async fn delete_step(
        &self,
        owner_id: Uuid,
        project_id: Uuid,
        step_id: Uuid,
    ) -> AppResult<()> {
        let removed = self
            .repo
            .delete_step_for_owner(step_id, project_id, owner_id)
            .await?;
        if !removed {
            return Err(AppError::not_found("Process step not found"));
        }
        Ok(())
    }

    // ── Analysis ────────────────────────────────────────────────────────────

    /// Run the deterministic analysis pipeline for a project:
    ///
    ///   load → catalog profiles → process engine → persist snapshot → return
    ///
    /// MVP scope: only the process engine runs. Shelf-life / flavor /
    /// nutrition engines will plug into the same flow in later steps and
    /// fill `flavor_result`, `nutrition_result`, `shelf_life_days`, etc.
    pub async fn analyze_project(
        &self,
        owner_id: Uuid,
        project_id: Uuid,
        language: Language,
    ) -> AppResult<LabProjectFull> {
        // 1) Ownership + project existence.
        let project = self
            .repo
            .get_project_for_owner(project_id, owner_id)
            .await?
            .ok_or_else(|| AppError::not_found("Laboratory project not found"))?;

        // 2) Load children.
        let (ingredient_rows, step_rows) = tokio::try_join!(
            self.repo.list_ingredients(project_id),
            self.repo.list_steps(project_id),
        )?;
        let ingredients: Vec<LabProjectIngredientDto> =
            ingredient_rows.into_iter().map(Into::into).collect();
        let steps: Vec<LabProcessStepDto> = step_rows.into_iter().map(Into::into).collect();

        // 3) Catalog profiles for every distinct slug.
        let slugs: Vec<String> = {
            let mut seen = std::collections::HashSet::new();
            ingredients
                .iter()
                .map(|i| i.ingredient_slug.clone())
                .filter(|s| seen.insert(s.clone()))
                .collect()
        };
        let profiles = self
            .catalog
            .get_profiles_by_slugs(&slugs, language)
            .await
            .unwrap_or_else(|e| {
                // Catalog failure is non-fatal — engine just gets fewer facts.
                tracing::warn!(error = %e, "catalog profile fetch failed; running engine without profiles");
                Vec::new()
            });

        // 4) Run engine (pure, deterministic).
        let analysis = process_engine::analyze_process(&ingredients, &steps, &profiles);

        // 4b) Shelf-life engine on the same inputs.
        let shelf =
            shelf_life_engine::analyze_shelf_life(&ingredients, &steps, &profiles, &analysis);

        // 4c) Flavor engine — sensory profile + pairing suggestions.
        let flavor = flavor_engine::analyze_flavor(&ingredients, &profiles);

        // 5) Persist snapshot.
        let process_effects_json = json!({
            "step_effects": analysis.step_effects,
            "global_effects": analysis.global_effects,
        });
        // Merge warnings from all engines.
        let mut all_warnings = analysis.warnings.clone();
        all_warnings.extend(shelf.warnings.clone());
        all_warnings.extend(flavor.warnings.clone());
        let warnings_json = serde_json::to_value(&all_warnings).unwrap_or_else(|_| json!([]));

        let storage_recommendations_json =
            serde_json::to_value(&shelf.storage_recommendations).unwrap_or_else(|_| json!([]));
        let flavor_result_json =
            serde_json::to_value(&flavor.flavor_result).unwrap_or_else(|_| json!({}));
        let pairing_suggestions_json =
            serde_json::to_value(&flavor.pairing_suggestions).unwrap_or_else(|_| json!([]));

        let input_snapshot = json!({
            "ingredients": ingredients,
            "process_steps": steps,
            "profile_slugs": slugs,
            "language": language.code(),
        });
        // Risk: max(process_risk, shelf_life_risk).
        let process_risk = risk_level_from_analysis(&analysis);
        let combined_risk = max_risk_level(&process_risk, &shelf.risk_level);

        let _ = self
            .repo
            .insert_analysis(NewLabProjectAnalysis {
                project_id,
                shelf_life_days: shelf.shelf_life_days,
                estimated_cost: None,
                complexity_score: None,
                risk_level: Some(combined_risk),
                texture_result: None,
                flavor_result: flavor_result_json,
                nutrition_result: json!({}),
                process_effects: process_effects_json,
                storage_recommendations: storage_recommendations_json,
                pairing_suggestions: pairing_suggestions_json,
                warnings: warnings_json,
                input_snapshot: Some(input_snapshot),
            })
            .await?;

        // 6) Return the project with the freshly persisted analysis.
        self.hydrate_project(project.into(), owner_id).await
    }

    // ── Visual story (Step 9) ───────────────────────────────────────────────

    /// `POST /api/laboratory/projects/:id/generate-scenes`
    ///
    /// Builds a deterministic visual story (frame plan) from the project's
    /// ingredients, steps and **most recent** persisted analysis. Pure: no
    /// AI call here, no DB write. Frontend uses the returned `scenes[]` to
    /// render the story-player; a follow-up step will fill `image_url`s
    /// with Gemini Image / Imagen output.
    ///
    /// Errors:
    ///  * `Ok(None)` — project does not exist (or is not owned by `owner_id`)
    ///  * `Err(Conflict)` — project has no analysis yet → run `/analyze` first
    pub async fn generate_visual_story(
        &self,
        owner_id: Uuid,
        project_id: Uuid,
    ) -> AppResult<Option<LaboratoryVisualStory>> {
        // 1) Ownership.
        let project = match self
            .repo
            .get_project_for_owner(project_id, owner_id)
            .await?
        {
            Some(p) => p,
            None => return Ok(None),
        };

        // 2) Children + latest analysis in parallel.
        let (ingredient_rows, step_rows, latest_analysis) = tokio::try_join!(
            self.repo.list_ingredients(project_id),
            self.repo.list_steps(project_id),
            self.repo.latest_analysis(project_id),
        )?;

        let ingredients: Vec<LabProjectIngredientDto> =
            ingredient_rows.into_iter().map(Into::into).collect();
        let steps: Vec<LabProcessStepDto> = step_rows.into_iter().map(Into::into).collect();

        // 3) Require an analysis snapshot — visual story is downstream of /analyze.
        let analysis_row = latest_analysis.ok_or_else(|| {
            AppError::conflict("Run /analyze on this project before generating visual scenes.")
        })?;

        // 4) Decode persisted process effects → engine struct.
        //    Stored shape: `{ "step_effects": [...], "global_effects": [...] }`
        //    `warnings` is missing from the stored blob (it lives in its own
        //    column) — `#[serde(default)]` on the engine type makes that fine.
        let process_analysis: LaboratoryProcessAnalysis =
            serde_json::from_value(analysis_row.process_effects.clone()).unwrap_or_default();

        // 5) Build the story (pure, deterministic).
        let story = visual_story_engine::build_visual_story(
            &ingredients,
            &steps,
            &process_analysis,
            project.target_product_type.as_deref(),
        );

        Ok(Some(story))
    }

    /// `POST /api/laboratory/copilot/suggest`
    ///
    /// Deterministic keyword-to-draft translator. No AI call, no DB write.
    /// Validates matched slugs against the `products` catalog so the client
    /// knows which suggestions are immediately usable.
    pub async fn suggest_draft(
        &self,
        prompt: &str,
        language: crate::shared::Language,
    ) -> AppResult<CopilotSuggestResponse> {
        // 1) Run the pure engine (sync, no I/O).
        let draft = copilot_engine::suggest(prompt);

        // 2) Validate matched slugs against the catalog.
        let slugs: Vec<String> = draft.ingredients.iter().map(|i| i.slug.clone()).collect();
        let profiles = if slugs.is_empty() {
            vec![]
        } else {
            self.catalog
                .get_profiles_by_slugs(&slugs, language)
                .await
                .unwrap_or_default()
        };
        let known_slugs: std::collections::HashSet<String> =
            profiles.iter().map(|p| p.slug.clone()).collect();

        // 3) Map draft → response (add `in_catalog` flag, localise name if known).
        let ingredients: Vec<CopilotSuggestIngredient> = draft
            .ingredients
            .into_iter()
            .map(|i| {
                let in_catalog = known_slugs.contains(&i.slug);
                CopilotSuggestIngredient {
                    slug: i.slug,
                    quantity: i.quantity,
                    unit: i.unit,
                    role: i.role,
                    in_catalog,
                }
            })
            .collect();

        let steps: Vec<CopilotSuggestStep> = draft
            .steps
            .into_iter()
            .map(|s| CopilotSuggestStep {
                technique: s.technique,
                temperature_c: s.temperature_c,
                duration_min: s.duration_min,
                note: s.note,
            })
            .collect();

        Ok(CopilotSuggestResponse {
            product_type: draft.product_type,
            suggested_name: draft.suggested_name,
            ingredients,
            steps,
            rationale: draft.rationale,
            confidence: draft.confidence,
            unmatched_tokens: draft.unmatched_tokens,
        })
    }

    // ── Internal: full hydration ────────────────────────────────────────────

    /// Loads ingredients + steps + latest analysis in parallel and assembles
    /// the response document. Caller must have already verified ownership of
    /// `summary.id` (either by inserting it, or via `get_project_for_owner`).
    async fn hydrate_project(
        &self,
        summary: LabProjectSummary,
        _owner_id: Uuid,
    ) -> AppResult<LabProjectFull> {
        let project_id = summary.id;
        let (ingredients, steps, analysis) = tokio::try_join!(
            self.repo.list_ingredients(project_id),
            self.repo.list_steps(project_id),
            self.repo.latest_analysis(project_id),
        )?;

        Ok(LabProjectFull {
            id: summary.id,
            name: summary.name,
            description: summary.description,
            target_product_type: summary.target_product_type,
            status: summary.status,
            ingredients: ingredients.into_iter().map(Into::into).collect(),
            process_steps: steps.into_iter().map(Into::into).collect(),
            latest_analysis: analysis.map(LabProjectAnalysisDto::from),
            created_at: summary.created_at,
            updated_at: summary.updated_at,
        })
    }
}

/// Derive a single bucketed risk level from the engine warnings.
///
///   any "critical" → "critical"
///   any "high"     → "high"
///   any "medium"   → "medium"   (also: warnings without explicit severity)
///   otherwise      → "low"
fn risk_level_from_analysis(analysis: &LaboratoryProcessAnalysis) -> String {
    let mut max_rank: u8 = 0; // 0 low, 1 medium, 2 high, 3 critical
    for w in &analysis.warnings {
        let rank = match w.severity.as_str() {
            "critical" => 3,
            "high" => 2,
            "warning" | "medium" => 1,
            "info" | "low" => 0,
            _ => 1, // unknown severity — be conservative
        };
        if rank > max_rank {
            max_rank = rank;
        }
    }
    match max_rank {
        3 => "critical",
        2 => "high",
        1 => "medium",
        _ => "low",
    }
    .to_string()
}
