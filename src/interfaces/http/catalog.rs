//! /v1/businesses/:business_id/{categories,services,variants}

use crate::application::catalog::{
    CategoryView, CreateCategoryInput, CreateServiceInput, CreateVariantInput, ServiceView,
    SetServiceStaffInput, UpdateCategoryInput, UpdateServiceInput, UpdateVariantInput, VariantView,
};
use crate::interfaces::http::{
    extract::{member_access, AuthUser},
    state::AppState,
};
use crate::shared::AppResult;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

// -- categories -------------------------------------------------------------

pub async fn list_categories(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(business_id): Path<Uuid>,
) -> AppResult<Json<Vec<CategoryView>>> {
    let access = member_access(&state, user_id, business_id).await?;
    Ok(Json(state.catalog.list_categories(access).await?))
}

pub async fn create_category(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(business_id): Path<Uuid>,
    Json(input): Json<CreateCategoryInput>,
) -> AppResult<(StatusCode, Json<CategoryView>)> {
    let access = member_access(&state, user_id, business_id).await?;
    let view = state.catalog.create_category(access, input).await?;
    Ok((StatusCode::CREATED, Json(view)))
}

pub async fn update_category(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((business_id, category_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<UpdateCategoryInput>,
) -> AppResult<Json<CategoryView>> {
    let access = member_access(&state, user_id, business_id).await?;
    Ok(Json(
        state
            .catalog
            .update_category(access, category_id, input)
            .await?,
    ))
}

pub async fn delete_category(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((business_id, category_id)): Path<(Uuid, Uuid)>,
) -> AppResult<StatusCode> {
    let access = member_access(&state, user_id, business_id).await?;
    state.catalog.delete_category(access, category_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// -- services ---------------------------------------------------------------

pub async fn list_services(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(business_id): Path<Uuid>,
) -> AppResult<Json<Vec<ServiceView>>> {
    let access = member_access(&state, user_id, business_id).await?;
    Ok(Json(state.catalog.list_services(access).await?))
}

pub async fn create_service(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(business_id): Path<Uuid>,
    Json(input): Json<CreateServiceInput>,
) -> AppResult<(StatusCode, Json<ServiceView>)> {
    let access = member_access(&state, user_id, business_id).await?;
    let view = state.catalog.create_service(access, input).await?;
    Ok((StatusCode::CREATED, Json(view)))
}

pub async fn get_service(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((business_id, service_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<ServiceView>> {
    let access = member_access(&state, user_id, business_id).await?;
    Ok(Json(state.catalog.get_service(access, service_id).await?))
}

pub async fn update_service(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((business_id, service_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<UpdateServiceInput>,
) -> AppResult<Json<ServiceView>> {
    let access = member_access(&state, user_id, business_id).await?;
    Ok(Json(
        state
            .catalog
            .update_service(access, service_id, input)
            .await?,
    ))
}

pub async fn delete_service(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((business_id, service_id)): Path<(Uuid, Uuid)>,
) -> AppResult<StatusCode> {
    let access = member_access(&state, user_id, business_id).await?;
    state.catalog.delete_service(access, service_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn set_service_staff(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((business_id, service_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<SetServiceStaffInput>,
) -> AppResult<Json<ServiceView>> {
    let access = member_access(&state, user_id, business_id).await?;
    Ok(Json(
        state
            .catalog
            .set_service_staff(access, service_id, input)
            .await?,
    ))
}

// -- variants ---------------------------------------------------------------

pub async fn create_variant(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((business_id, service_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<CreateVariantInput>,
) -> AppResult<(StatusCode, Json<VariantView>)> {
    let access = member_access(&state, user_id, business_id).await?;
    let view = state
        .catalog
        .create_variant(access, service_id, input)
        .await?;
    Ok((StatusCode::CREATED, Json(view)))
}

pub async fn update_variant(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((business_id, variant_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<UpdateVariantInput>,
) -> AppResult<Json<VariantView>> {
    let access = member_access(&state, user_id, business_id).await?;
    Ok(Json(
        state
            .catalog
            .update_variant(access, variant_id, input)
            .await?,
    ))
}

pub async fn delete_variant(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((business_id, variant_id)): Path<(Uuid, Uuid)>,
) -> AppResult<StatusCode> {
    let access = member_access(&state, user_id, business_id).await?;
    state.catalog.delete_variant(access, variant_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
