use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::application::DishService;
use crate::domain::{DishName, Money, RecipeId};
use crate::interfaces::http::middleware::AuthUser;
use crate::shared::{AppError, PaginatedResponse, PaginationParams};

/// POST /api/dishes
#[derive(Debug, Deserialize)]
pub struct CreateDishRequest {
    pub recipe_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub selling_price_cents: i32,
    pub image_url: Option<String>,
}

/// Helper: build dish JSON with cost fields
fn dish_to_json(dish: &crate::domain::Dish) -> serde_json::Value {
    serde_json::json!({
        "id": dish.id().as_uuid(),
        "name": dish.name().as_str(),
        "recipe_id": dish.recipe_id().as_uuid(),
        "selling_price_cents": dish.selling_price().as_cents(),
        "active": dish.is_active(),
        "image_url": dish.image_url(),
        "recipe_cost_cents": dish.recipe_cost_cents(),
        "food_cost_percent": dish.food_cost_percent(),
        "profit_margin_percent": dish.profit_margin_percent(),
        "cost_calculated_at": dish.cost_calculated_at().map(|t| t.format(&time::format_description::well_known::Rfc3339).unwrap_or_default()),
    })
}

pub async fn create_dish(
    State(service): State<DishService>,
    AuthUser {
        tenant_id,
        language: _,
        user_id: _,
    }: AuthUser,
    Json(payload): Json<CreateDishRequest>,
) -> Result<impl IntoResponse, AppError> {
    let dish_name = DishName::new(payload.name)?;
    let selling_price = Money::from_cents(payload.selling_price_cents as i64)?;
    let recipe_id = RecipeId::from_uuid(payload.recipe_id);

    let dish = service
        .create_dish(
            tenant_id,
            recipe_id,
            dish_name,
            payload.description,
            selling_price,
            payload.image_url,
        )
        .await?;

    Ok((StatusCode::CREATED, Json(dish_to_json(&dish))))
}

/// GET /api/dishes?page=1&per_page=50&active_only=true
#[derive(Debug, Deserialize)]
pub struct ListDishesQuery {
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub per_page: Option<u32>,
    #[serde(default = "default_active_only")]
    pub active_only: bool,
}

fn default_active_only() -> bool {
    false
}

pub async fn list_dishes(
    State(service): State<DishService>,
    auth: AuthUser,
    Query(query): Query<ListDishesQuery>,
) -> Result<impl IntoResponse, AppError> {
    let pagination = PaginationParams {
        page: query.page,
        per_page: query.per_page,
    };

    let (dishes, total) = service
        .list_dishes(auth.tenant_id, query.active_only, &pagination)
        .await?;

    let items: Vec<serde_json::Value> = dishes.iter().map(dish_to_json).collect();
    let response = PaginatedResponse::new(items, total, &pagination);

    Ok(Json(serde_json::json!({
        "items": response.items,
        "total": response.total,
        "page": response.page,
        "per_page": response.per_page,
        "total_pages": response.total_pages,
    })))
}

/// POST /api/dishes/recalculate-all
/// Recalculate materialized costs for ALL tenant dishes.
/// Called when ingredient prices change or manually by owner.
pub async fn recalculate_all_costs(
    State(service): State<DishService>,
    auth: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    let result = service.recalculate_all_costs(auth.tenant_id).await?;

    Ok(Json(serde_json::json!({
        "updated": result.updated,
        "errors": result.errors,
        "message": format!("{} dishes recalculated, {} errors", result.updated, result.errors),
    })))
}
