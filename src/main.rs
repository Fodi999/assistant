use beauty_backend::infrastructure::Config;
use beauty_backend::interfaces::http::{create_router, AppState};
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // `.env` is a development convenience; real environments inject variables.
    dotenvy::dotenv().ok();
    init_tracing();

    let config = Config::from_env()?;
    tracing::info!("Starting beauty-backend (env: {:?})", config.env);

    // Lazy pool: the process starts (and /health answers) even if the database
    // is briefly down; /ready reports the real state.
    let pool = PgPoolOptions::new()
        .max_connections(config.database.max_connections)
        .acquire_timeout(Duration::from_secs(10))
        .connect_lazy(&config.database.url)?;

    let address = config.server_address();
    let app = create_router(AppState::new(config, pool));

    let listener = tokio::net::TcpListener::bind(&address).await?;
    tracing::info!("Listening on {}", address);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("Server stopped");
    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    if std::env::var("LOG_FORMAT").as_deref() == Ok("json") {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .json()
            .init();
    } else {
        tracing_subscriber::fmt().with_env_filter(filter).init();
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if tokio::signal::ctrl_c().await.is_err() {
            tracing::error!("Failed to listen for Ctrl+C");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(e) => {
                tracing::error!("Failed to install SIGTERM handler: {}", e);
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Shutdown signal received");
}
