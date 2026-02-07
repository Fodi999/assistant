use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::application::DishService;
use crate::domain::{DishName, Money, RecipeId};
use crate::interfaces::http::middleware::AuthUser;
use crate::shared::AppError;

/// POST /api/dishes
/// Create a new dish
#[derive(Debug, Deserialize)]
pub struct CreateDishRequest {
    pub recipe_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub selling_price_cents: i32,
}

pub async fn create_dish(
    State(service): State<DishService>,
    AuthUser { tenant_id, .. }: AuthUser,
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
        )
        .await?;
    
    Ok((StatusCode::CREATED, Json(serde_json::json!({
        "id": dish.id().as_uuid(),
        "name": dish.name().as_str(),
        "recipe_id": dish.recipe_id().as_uuid(),
        "selling_price_cents": dish.selling_price().as_cents(),
        "active": dish.is_active(),
    }))))
}
