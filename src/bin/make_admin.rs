//! Makes an existing account a platform admin (can approve businesses).
//!
//! Usage: MIGRATION_DATABASE_URL='<owner url>' cargo run --release --bin make_admin -- admin@example.com
//!
//! The account must already exist (register it through the API first). Uses
//! the schema-owner connection, like the `migrate` binary; the application
//! role can never grant this right to anyone.

use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let email = std::env::args()
        .nth(1)
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("usage: make_admin <email of an existing account>"))?;
    let url = std::env::var("MIGRATION_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .map_err(|_| {
            anyhow::anyhow!("set MIGRATION_DATABASE_URL (schema owner) or DATABASE_URL")
        })?;

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await?;
    sqlx::query(
        "INSERT INTO platform_admin (user_id)
         SELECT id FROM users WHERE email = $1 AND deleted_at IS NULL
         ON CONFLICT DO NOTHING",
    )
    .bind(&email)
    .execute(&pool)
    .await?;
    let is_admin: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM platform_admin a JOIN users u ON u.id = a.user_id
                        WHERE u.email = $1)",
    )
    .bind(&email)
    .fetch_one(&pool)
    .await?;
    if !is_admin {
        anyhow::bail!("no account with this e-mail; register it first");
    }
    println!("{email} is a platform admin");
    Ok(())
}
