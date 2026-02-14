use axum::{
    extract::State,
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
    pub is_active: bool,
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
            u.name,
            t.restaurant_name,
            u.language,
            u.created_at::text,
            u.is_active
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
    pub active_users: i64,
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
            COUNT(DISTINCT CASE WHEN u.is_active THEN u.id END) as active_users,
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
