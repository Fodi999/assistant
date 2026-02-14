use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Serialize;
use sqlx::PgPool;

/// Response with list of users
#[derive(Debug, Serialize)]
pub struct UsersListResponse {
    pub users: Vec<UserInfo>,
    pub total: i64,
}

/// User info for admin panel
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub restaurant_name: String,
    pub language: String,
    pub created_at: String,
}

/// GET /api/admin/users - List all users with their restaurants
pub async fn list_users(
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    // Query to get users with their tenants
    let users = sqlx::query_as::<_, UserInfo>(
        r#"
        SELECT 
            u.id::text,
            u.email,
            u.display_name as name,
            t.name as restaurant_name,
            COALESCE(u.language, 'ru') as language,
            u.created_at::text
        FROM users u
        JOIN tenants t ON u.tenant_id = t.id
        ORDER BY u.created_at DESC
        "#
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error fetching users: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let total = users.len() as i64;

    Ok(Json(UsersListResponse { users, total }))
}

/// User statistics for dashboard
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct UserStats {
    pub total_users: i64,
    pub total_restaurants: i64,
}

/// GET /api/admin/stats - Get user statistics
pub async fn get_stats(
    State(pool): State<PgPool>,
) -> Result<impl IntoResponse, StatusCode> {
    let stats = sqlx::query_as::<_, UserStats>(
        r#"
        SELECT 
            COUNT(DISTINCT u.id) as total_users,
            COUNT(DISTINCT t.id) as total_restaurants
        FROM users u
        JOIN tenants t ON u.tenant_id = t.id
        "#
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error fetching stats: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(stats))
}

/// DELETE /api/admin/users/:id - Delete user and their tenant
/// ⚠️ This is a CASCADE delete - removes user, tenant, and all related data
pub async fn delete_user(
    State(pool): State<PgPool>,
    Path(user_id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    // First, get the tenant_id for this user
    let tenant_id: Option<String> = sqlx::query_scalar(
        "SELECT tenant_id::text FROM users WHERE id = $1::uuid"
    )
    .bind(&user_id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error fetching tenant_id: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let Some(tenant_id) = tenant_id else {
        tracing::warn!("User {} not found", user_id);
        return Err(StatusCode::NOT_FOUND);
    };

    // Delete the tenant (CASCADE will delete user and all related data)
    let result = sqlx::query(
        "DELETE FROM tenants WHERE id = $1::uuid"
    )
    .bind(&tenant_id)
    .execute(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error deleting tenant: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if result.rows_affected() == 0 {
        tracing::warn!("Tenant {} not found", tenant_id);
        return Err(StatusCode::NOT_FOUND);
    }

    tracing::info!("Deleted user {} and tenant {}", user_id, tenant_id);

    Ok(Json(serde_json::json!({
        "message": "User and tenant deleted successfully",
        "user_id": user_id,
        "tenant_id": tenant_id
    })))
}
