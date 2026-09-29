//! Applies database migrations. Run as a deploy step, never at process start.
//!
//! Uses MIGRATION_DATABASE_URL (the schema-owner role) if set, otherwise
//! DATABASE_URL. The application itself must connect as a different, limited
//! role that is a member of `beauty_app` (see the identity_and_tenancy migration).

use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let url = std::env::var("MIGRATION_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .map_err(|_| {
            anyhow::anyhow!("set MIGRATION_DATABASE_URL (schema owner) or DATABASE_URL")
        })?;

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    println!("migrations applied");
    Ok(())
}
