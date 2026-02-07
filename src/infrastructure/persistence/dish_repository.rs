use crate::domain::{Dish, DishId, DishName, RecipeId, Money};
use crate::shared::{AppError, AppResult, TenantId};
use async_trait::async_trait;
use sqlx::PgPool;
use uuid;

#[async_trait]
pub trait DishRepositoryTrait: Send + Sync {
    async fn create(&self, dish: &Dish) -> AppResult<()>;
    async fn find_by_id(&self, id: DishId, tenant_id: TenantId) -> AppResult<Option<Dish>>;
    async fn list_by_tenant(&self, tenant_id: TenantId, active_only: bool) -> AppResult<Vec<Dish>>;
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

#[async_trait]
impl DishRepositoryTrait for DishRepository {
    async fn create(&self, dish: &Dish) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO dishes (id, tenant_id, recipe_id, name, description, selling_price_cents, active)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#
        )
        .bind(dish.id().as_uuid())
        .bind(dish.tenant_id().as_uuid())
        .bind(dish.recipe_id().as_uuid())
        .bind(dish.name().as_str())
        .bind(dish.description())
        .bind(dish.selling_price().as_cents() as i32)
        .bind(dish.is_active())
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }

    async fn find_by_id(&self, id: DishId, tenant_id: TenantId) -> AppResult<Option<Dish>> {
        let row = sqlx::query_as::<_, (uuid::Uuid, uuid::Uuid, uuid::Uuid, String, Option<String>, i32, bool, time::OffsetDateTime, time::OffsetDateTime)>(
            r#"
            SELECT id, tenant_id, recipe_id, name, description, selling_price_cents, active, created_at, updated_at
            FROM dishes
            WHERE id = $1 AND tenant_id = $2
            "#
        )
        .bind(id.as_uuid())
        .bind(tenant_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let Some((row_id, row_tenant_id, row_recipe_id, row_name, row_description, row_price, row_active, row_created_at, row_updated_at)) = row else {
            return Ok(None);
        };

        let dish = Dish::from_parts(
            DishId::from_uuid(row_id),
            TenantId::from_uuid(row_tenant_id),
            RecipeId::from_uuid(row_recipe_id),
            DishName::new(row_name)?,
            row_description,
            Money::from_cents(row_price as i64)?,
            row_active,
            row_created_at,
            row_updated_at,
        );

        Ok(Some(dish))
    }

    async fn list_by_tenant(&self, tenant_id: TenantId, active_only: bool) -> AppResult<Vec<Dish>> {
        let query = if active_only {
            r#"
            SELECT id, tenant_id, recipe_id, name, description, selling_price_cents, active, created_at, updated_at
            FROM dishes
            WHERE tenant_id = $1 AND active = true
            ORDER BY name ASC
            "#
        } else {
            r#"
            SELECT id, tenant_id, recipe_id, name, description, selling_price_cents, active, created_at, updated_at
            FROM dishes
            WHERE tenant_id = $1
            ORDER BY name ASC
            "#
        };

        let rows = sqlx::query_as::<_, (uuid::Uuid, uuid::Uuid, uuid::Uuid, String, Option<String>, i32, bool, time::OffsetDateTime, time::OffsetDateTime)>(query)
            .bind(tenant_id.as_uuid())
            .fetch_all(&self.pool)
            .await
            .map_err(AppError::Database)?;

        let dishes: Vec<Dish> = rows
            .into_iter()
            .map(|(row_id, row_tenant_id, row_recipe_id, row_name, row_description, row_price, row_active, row_created_at, row_updated_at)| {
                Ok(Dish::from_parts(
                    DishId::from_uuid(row_id),
                    TenantId::from_uuid(row_tenant_id),
                    RecipeId::from_uuid(row_recipe_id),
                    DishName::new(row_name)?,
                    row_description,
                    Money::from_cents(row_price as i64)?,
                    row_active,
                    row_created_at,
                    row_updated_at,
                ))
            })
            .collect::<AppResult<Vec<_>>>()?;

        Ok(dishes)
    }

    async fn update(&self, dish: &Dish) -> AppResult<()> {
        sqlx::query(
            r#"
            UPDATE dishes
            SET name = $1, description = $2, selling_price_cents = $3, active = $4, updated_at = $5
            WHERE id = $6 AND tenant_id = $7
            "#
        )
        .bind(dish.name().as_str())
        .bind(dish.description())
        .bind(dish.selling_price().as_cents() as i32)
        .bind(dish.is_active())
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
            "#
        )
        .bind(id.as_uuid())
        .bind(tenant_id.as_uuid())
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(result.rows_affected() > 0)
    }
}
