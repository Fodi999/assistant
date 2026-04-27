//! HTTP handlers for the Laboratory module.
//!
//! Mounted under `/api/laboratory/...` inside the protected (JWT) router, so
//! every handler receives an `AuthUser` extractor and uses
//! `auth_user.user_id` as the source of truth for `owner_id`.
//! `owner_id` is **never** taken from the request body.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::application::laboratory::{
    AddLabIngredientRequest, AddLabStepRequest, CopilotSuggestRequest, CopilotSuggestResponse,
    CreateLabProjectRequest, LabProcessStepDto,
    LabProjectFull, LabProjectIngredientDto, LabProjectSummary, LaboratoryService,
    LaboratoryVisualStory,
};
use crate::interfaces::http::middleware::AuthUser;
use crate::shared::{AppError, Language};

// ─────────────────────────────────────────────────────────────────────────────
// Projects
// ─────────────────────────────────────────────────────────────────────────────

/// `POST /api/laboratory/projects`
pub async fn create_project(
    auth: AuthUser,
    State(svc): State<LaboratoryService>,
    Json(req): Json<CreateLabProjectRequest>,
) -> Result<(StatusCode, Json<LabProjectFull>), AppError> {
    let project = svc.create_project(*auth.user_id.as_uuid(), req).await?;
    Ok((StatusCode::CREATED, Json(project)))
}

/// `GET /api/laboratory/projects`
pub async fn list_projects(
    auth: AuthUser,
    State(svc): State<LaboratoryService>,
) -> Result<Json<Vec<LabProjectSummary>>, AppError> {
    let list = svc.list_projects(*auth.user_id.as_uuid()).await?;
    Ok(Json(list))
}

/// `GET /api/laboratory/projects/:id`
pub async fn get_project(
    auth: AuthUser,
    State(svc): State<LaboratoryService>,
    Path(project_id): Path<Uuid>,
) -> Result<Json<LabProjectFull>, AppError> {
    let project = svc.get_project(*auth.user_id.as_uuid(), project_id).await?;
    Ok(Json(project))
}

/// `DELETE /api/laboratory/projects/:id`
pub async fn delete_project(
    auth: AuthUser,
    State(svc): State<LaboratoryService>,
    Path(project_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    svc.delete_project(*auth.user_id.as_uuid(), project_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ─────────────────────────────────────────────────────────────────────────────
// Ingredients
// ─────────────────────────────────────────────────────────────────────────────

/// `POST /api/laboratory/projects/:id/ingredients`
pub async fn add_ingredient(
    auth: AuthUser,
    State(svc): State<LaboratoryService>,
    Path(project_id): Path<Uuid>,
    Json(req): Json<AddLabIngredientRequest>,
) -> Result<(StatusCode, Json<LabProjectIngredientDto>), AppError> {
    let ing = svc
        .add_ingredient(*auth.user_id.as_uuid(), project_id, req)
        .await?;
    Ok((StatusCode::CREATED, Json(ing)))
}

/// `DELETE /api/laboratory/projects/:id/ingredients/:ingredient_id`
pub async fn delete_ingredient(
    auth: AuthUser,
    State(svc): State<LaboratoryService>,
    Path((project_id, ingredient_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    svc.delete_ingredient(*auth.user_id.as_uuid(), project_id, ingredient_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

// ─────────────────────────────────────────────────────────────────────────────
// Process steps
// ─────────────────────────────────────────────────────────────────────────────

/// `POST /api/laboratory/projects/:id/steps`
pub async fn add_step(
    auth: AuthUser,
    State(svc): State<LaboratoryService>,
    Path(project_id): Path<Uuid>,
    Json(req): Json<AddLabStepRequest>,
) -> Result<(StatusCode, Json<LabProcessStepDto>), AppError> {
    let step = svc
        .add_step(*auth.user_id.as_uuid(), project_id, req)
        .await?;
    Ok((StatusCode::CREATED, Json(step)))
}

/// `DELETE /api/laboratory/projects/:id/steps/:step_id`
pub async fn delete_step(
    auth: AuthUser,
    State(svc): State<LaboratoryService>,
    Path((project_id, step_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    svc.delete_step(*auth.user_id.as_uuid(), project_id, step_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

// ─────────────────────────────────────────────────────────────────────────────
// Analyze
// ─────────────────────────────────────────────────────────────────────────────

/// Optional override for the localization language of the analysis (used to
/// pick localized ingredient names from the catalog). If absent we fall back
/// to the language carried by the JWT/middleware.
#[derive(Debug, Deserialize, Default)]
pub struct AnalyzeProjectQuery {
    #[serde(default)]
    pub lang: Option<String>,
}

/// `POST /api/laboratory/projects/:id/analyze`
///
/// Runs the deterministic analysis pipeline and persists a snapshot in
/// `lab_project_analysis`. Returns the full project document with the new
/// snapshot already attached as `latest_analysis`.
pub async fn analyze_project(
    auth: AuthUser,
    State(svc): State<LaboratoryService>,
    Path(project_id): Path<Uuid>,
    Query(query): Query<AnalyzeProjectQuery>,
) -> Result<Json<LabProjectFull>, AppError> {
    let language = query
        .lang
        .as_deref()
        .and_then(Language::from_code)
        .unwrap_or(auth.language);

    let project = svc
        .analyze_project(*auth.user_id.as_uuid(), project_id, language)
        .await?;
    Ok(Json(project))
}

// ─────────────────────────────────────────────────────────────────────────────
// Copilot
// ─────────────────────────────────────────────────────────────────────────────

/// `POST /api/laboratory/copilot/suggest?lang=ru`
///
/// Pure (deterministic) keyword-to-draft translator. Returns a `CopilotDraft`
/// containing `ingredients[]`, `steps[]`, `product_type`, `confidence`.
/// Does NOT create a project — the frontend uses the draft to populate the
/// constructor zone, then the user clicks "Create" / "Add" as usual.
pub async fn copilot_suggest(
    auth: AuthUser,
    State(svc): State<LaboratoryService>,
    Query(query): Query<AnalyzeProjectQuery>,
    Json(body): Json<CopilotSuggestRequest>,
) -> Result<Json<CopilotSuggestResponse>, AppError> {
    let language = query
        .lang
        .as_deref()
        .and_then(Language::from_code)
        .unwrap_or(auth.language);

    if body.prompt.trim().is_empty() {
        return Err(AppError::validation("prompt must not be empty"));
    }
    if body.prompt.chars().count() > 500 {
        return Err(AppError::validation("prompt too long (max 500 chars)"));
    }

    let response = svc.suggest_draft(&body.prompt, language).await?;
    Ok(Json(response))
}

// ─────────────────────────────────────────────────────────────────────────────
// Visual story (Step 9)
// ─────────────────────────────────────────────────────────────────────────────

/// `POST /api/laboratory/projects/:id/generate-scenes`
///
/// Returns the deterministic visual story (`raw → … → ready`) built from
/// the project's latest analysis. Image generation (Gemini / Imagen) is
/// intentionally **not** wired here yet — `image_url` is `null` for every
/// frame so the frontend can already build its story-player.
///
/// Status codes:
///  * `200 OK`        — story returned
///  * `404 NOT_FOUND` — project does not exist or is not owned by the caller
///  * `409 CONFLICT`  — project has no analysis yet (run `/analyze` first)
pub async fn generate_scenes(
    auth: AuthUser,
    State(svc): State<LaboratoryService>,
    Path(project_id): Path<Uuid>,
) -> Result<Json<LaboratoryVisualStory>, AppError> {
    let story = svc
        .generate_visual_story(*auth.user_id.as_uuid(), project_id)
        .await?
        .ok_or_else(|| AppError::not_found("Laboratory project not found"))?;
    Ok(Json(story))
}

