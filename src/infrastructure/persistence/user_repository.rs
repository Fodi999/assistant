use crate::domain::{DisplayName, Email, User, UserRole};
use crate::shared::{AppResult, Language, TenantId, UserId};
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

#[async_trait]
pub trait UserRepositoryTrait: Send + Sync {
    async fn create(&self, user: &User) -> AppResult<()>;
    async fn find_by_id(&self, id: UserId) -> AppResult<Option<User>>;
    async fn find_by_email(&self, email: &Email) -> AppResult<Option<User>>;
    async fn exists_by_email(&self, email: &Email) -> AppResult<bool>;
    async fn update_login_stats(&self, user_id: UserId) -> AppResult<()>;
}

#[derive(Clone)]
pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepositoryTrait for UserRepository {
    async fn create(&self, user: &User) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO users (id, tenant_id, email, password_hash, display_name, role, language, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#
        )
        .bind(user.id.as_uuid())
        .bind(user.tenant_id.as_uuid())
        .bind(user.email.as_str())
        .bind(&user.password_hash)
        .bind(user.display_name.as_ref().map(|n| n.as_str()))
        .bind(user.role.as_str())
        .bind(user.language.code())
        .bind(user.created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn find_by_id(&self, id: UserId) -> AppResult<Option<User>> {
        let result = sqlx::query(
            r#"
            SELECT id, tenant_id, email, password_hash, display_name, role, language, created_at
            FROM users
            WHERE id = $1
            "#
        )
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.and_then(|row| {
            let id: Uuid = row.get("id");
            let tenant_id: Uuid = row.get("tenant_id");
            let email: String = row.get("email");
            let password_hash: String = row.get("password_hash");
            let display_name: Option<String> = row.get("display_name");
            let role: String = row.get("role");
            let language: String = row.get("language");
            let created_at: OffsetDateTime = row.get("created_at");

            Some(User::from_parts(
                UserId::from_uuid(id),
                TenantId::from_uuid(tenant_id),
                Email::new(email).ok()?,
                password_hash,
                display_name.and_then(|n| DisplayName::new(n).ok()),
                UserRole::from_str(&role).ok()?,
                Language::from_str(&language).ok()?,
                created_at,
            ))
        }))
    }

    async fn find_by_email(&self, email: &Email) -> AppResult<Option<User>> {
        let result = sqlx::query(
            r#"
            SELECT id, tenant_id, email, password_hash, display_name, role, language, created_at
            FROM users
            WHERE email = $1
            "#
        )
        .bind(email.as_str())
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.and_then(|row| {
            let id: Uuid = row.get("id");
            let tenant_id: Uuid = row.get("tenant_id");
            let email: String = row.get("email");
            let password_hash: String = row.get("password_hash");
            let display_name: Option<String> = row.get("display_name");
            let role: String = row.get("role");
            let language: String = row.get("language");
            let created_at: OffsetDateTime = row.get("created_at");

            Some(User::from_parts(
                UserId::from_uuid(id),
                TenantId::from_uuid(tenant_id),
                Email::new(email).ok()?,
                password_hash,
                display_name.and_then(|n| DisplayName::new(n).ok()),
                UserRole::from_str(&role).ok()?,
                Language::from_str(&language).ok()?,
                created_at,
            ))
        }))
    }

    async fn exists_by_email(&self, email: &Email) -> AppResult<bool> {
        let result = sqlx::query(
            r#"
            SELECT EXISTS(SELECT 1 FROM users WHERE email = $1) as exists
            "#
        )
        .bind(email.as_str())
        .fetch_one(&self.pool)
        .await?;

        let exists: bool = result.get("exists");
        Ok(exists)
    }

    async fn update_login_stats(&self, user_id: UserId) -> AppResult<()> {
        sqlx::query(
            "UPDATE users SET login_count = login_count + 1, last_login_at = NOW() WHERE id = $1"
        )
        .bind(user_id.as_uuid())
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
