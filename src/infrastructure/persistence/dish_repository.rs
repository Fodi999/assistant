use crate::domain::{Dish, DishId, DishName, Money, RecipeId};
use crate::shared::{AppError, AppResult, PaginationParams, TenantId};
use async_trait::async_trait;
use sqlx::PgPool;
use uuid;

#[async_trait]
pub trait DishRepositoryTrait: Send + Sync {
    async fn create(&self, dish: &Dish) -> AppResult<()>;
    async fn find_by_id(&self, id: DishId, tenant_id: TenantId) -> AppResult<Option<Dish>>;
    async fn list_by_tenant(
        &self,
        tenant_id: TenantId,
        active_only: bool,
        pagination: &PaginationParams,
    ) -> AppResult<(Vec<Dish>, i64)>;
    async fn update(&self, dish: &Dish) -> AppResult<()>;
    async fn delete(&self, id: DishId, tenant_id: TenantId) -> AppResult<bool>;
}

#[derive(Clone)]
pub struct DishRepository {
    pool: PgPool,
}

impl DishRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

// Column set shared across all SELECTs (13 columns)
const DISH_COLUMNS: &str = r#"
    id, tenant_id, recipe_id, name, description, selling_price_cents, active,
    recipe_cost_cents, food_cost_percent, profit_margin_percent, cost_calculated_at,
    image_url, created_at, updated_at
"#;

// Row type: 13 columns
type DishRow = (
    uuid::Uuid,                         // id
    uuid::Uuid,                         // tenant_id
    uuid::Uuid,                         // recipe_id
    String,                             // name
    Option<String>,                     // description
    i32,                                // selling_price_cents
    bool,                               // active
    Option<i64>,                        // recipe_cost_cents
    Option<f64>,                        // food_cost_percent
    Option<f64>,                        // profit_margin_percent
    Option<time::OffsetDateTime>,       // cost_calculated_at
    Option<String>,                     // image_url
    time::OffsetDateTime,               // created_at
    time::OffsetDateTime,               // updated_at
);

fn row_to_dish(row: DishRow) -> AppResult<Dish> {
    Ok(Dish::from_parts(
        DishId::from_uuid(row.0),
        TenantId::from_uuid(row.1),
        RecipeId::from_uuid(row.2),
        DishName::new(row.3)?,
        row.4,
        Money::from_cents(row.5 as i64)?,
        row.6,
        row.7,
        row.8,
        row.9,
        row.10,
        row.11,
        row.12,
        row.13,
    ))
}

#[async_trait]
impl DishRepositoryTrait for DishRepository {
    async fn create(&self, dish: &Dish) -> AppResult<()> {
        sqlx::query(&format!(
            r#"
            INSERT INTO dishes ({})
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            "#,
            DISH_COLUMNS.trim()
        ))
        .bind(dish.id().as_uuid())
        .bind(dish.tenant_id().as_uuid())
        .bind(dish.recipe_id().as_uuid())
        .bind(dish.name().as_str())
        .bind(dish.description())
        .bind(dish.selling_price().as_cents() as i32)
        .bind(dish.is_active())
        .bind(dish.recipe_cost_cents())
        .bind(dish.food_cost_percent())
        .bind(dish.profit_margin_percent())
        .bind(dish.cost_calculated_at())
        .bind(dish.image_url())
        .bind(dish.created_at())
        .bind(dish.updated_at())
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }

    async fn find_by_id(&self, id: DishId, tenant_id: TenantId) -> AppResult<Option<Dish>> {
        let row = sqlx::query_as::<_, DishRow>(&format!(
            "SELECT {} FROM dishes WHERE id = $1 AND tenant_id = $2",
            DISH_COLUMNS.trim()
        ))
        .bind(id.as_uuid())
        .bind(tenant_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        match row {
            Some(r) => Ok(Some(row_to_dish(r)?)),
            None => Ok(None),
        }
    }

    async fn list_by_tenant(
        &self,
        tenant_id: TenantId,
        active_only: bool,
        pagination: &PaginationParams,
    ) -> AppResult<(Vec<Dish>, i64)> {
        // Count
        let total: i64 = if active_only {
            sqlx::query_scalar("SELECT COUNT(*) FROM dishes WHERE tenant_id = $1 AND active = true")
                .bind(tenant_id.as_uuid())
                .fetch_one(&self.pool)
                .await
                .map_err(AppError::Database)?
        } else {
            sqlx::query_scalar("SELECT COUNT(*) FROM dishes WHERE tenant_id = $1")
                .bind(tenant_id.as_uuid())
                .fetch_one(&self.pool)
                .await
                .map_err(AppError::Database)?
        };

        let where_clause = if active_only {
            "WHERE tenant_id = $1 AND active = true"
        } else {
            "WHERE tenant_id = $1"
        };

        let query = format!(
            "SELECT {} FROM dishes {} ORDER BY name ASC LIMIT $2 OFFSET $3",
            DISH_COLUMNS.trim(),
            where_clause
        );

        let rows = sqlx::query_as::<_, DishRow>(&query)
            .bind(tenant_id.as_uuid())
            .bind(pagination.limit())
            .bind(pagination.offset())
            .fetch_all(&self.pool)
            .await
            .map_err(AppError::Database)?;

        let dishes: Vec<Dish> = rows
            .into_iter()
            .map(row_to_dish)
            .collect::<AppResult<Vec<_>>>()?;

        Ok((dishes, total))
    }

    async fn update(&self, dish: &Dish) -> AppResult<()> {
        sqlx::query(
            r#"
            UPDATE dishes
            SET name = $1, description = $2, selling_price_cents = $3, active = $4,
                recipe_cost_cents = $5, food_cost_percent = $6,
                profit_margin_percent = $7, cost_calculated_at = $8,
                image_url = $9, updated_at = $10
            WHERE id = $11 AND tenant_id = $12
            "#,
        )
        .bind(dish.name().as_str())
        .bind(dish.description())
        .bind(dish.selling_price().as_cents() as i32)
        .bind(dish.is_active())
        .bind(dish.recipe_cost_cents())
        .bind(dish.food_cost_percent())
        .bind(dish.profit_margin_percent())
        .bind(dish.cost_calculated_at())
        .bind(dish.image_url())
        .bind(dish.updated_at())
        .bind(dish.id().as_uuid())
        .bind(dish.tenant_id().as_uuid())
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }

    async fn delete(&self, id: DishId, tenant_id: TenantId) -> AppResult<bool> {
        let result = sqlx::query(
            r#"
            DELETE FROM dishes
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id.as_uuid())
        .bind(tenant_id.as_uuid())
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(result.rows_affected() > 0)
    }
}
