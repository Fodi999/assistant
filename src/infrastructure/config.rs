use std::env;
use tracing;

/// Insecure secrets that must never be used in production
const INSECURE_SECRETS: &[&str] = &[
    "change_me",
    "secret",
    "password",
    "jwt_secret",
    "your-super-secret-jwt-key",
    "test_secret_for_local_development_only_12345",
];

#[derive(Debug, Clone)]
pub struct Config {
    pub database: DatabaseConfig,
    pub server: ServerConfig,
    pub jwt: JwtConfig,
    pub cors: CorsConfig,
    pub admin: AdminConfig,
    pub r2: R2Config,
    pub ai: AiConfig,
    pub telegram: Option<TelegramConfig>,
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub port: u16,
    pub rate_limit_per_second: u32,
}

#[derive(Debug, Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub issuer: String,
    pub access_token_ttl_minutes: i64,
    pub refresh_token_ttl_days: i64,
}

#[derive(Debug, Clone)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AdminConfig {
    pub email: String,
    pub password_hash: String,
    pub jwt_secret: String,
    pub token_ttl_hours: usize,
}

#[derive(Debug, Clone)]
pub struct R2Config {
    pub account_id: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub bucket_name: String,
    pub public_url_base: String,
}

/// AI Services Configuration
#[derive(Debug, Clone)]
pub struct AiConfig {
    pub groq_api_key: String,
    pub gemini_api_key: String,
}

/// Telegram Bot configuration for "Світло Ікони".
///
/// Deliberately does NOT derive/implement `Default` — the whole point is that
/// the bot is either fully configured (token present) or `None` (disabled).
/// The bot token must never be printed; `Debug` is implemented by hand below
/// to guarantee that even `{:?}` logging can't leak it.
#[derive(Clone)]
pub struct TelegramConfig {
    pub bot_token: String,
    /// `None` when TELEGRAM_WEBHOOK_SECRET is unset — the webhook endpoint
    /// treats that as "reject everything" (fail closed), never as "skip the check".
    pub webhook_secret: Option<String>,
    pub channel: String,
}

impl std::fmt::Debug for TelegramConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TelegramConfig")
            .field("bot_token", &"***redacted***")
            .field(
                "webhook_secret",
                &self.webhook_secret.as_ref().map(|_| "***redacted***"),
            )
            .field("channel", &self.channel)
            .finish()
    }
}

/// Resolves Telegram config from env in isolation from the rest of [`Config`]
/// so it can be unit-tested without the many other required env vars
/// (`DATABASE_URL`, `ADMIN_EMAIL`, R2 credentials, ...) that `Config::from_env`
/// needs. Returns `None` whenever `TELEGRAM_BOT_TOKEN` is absent or blank —
/// the rest of the backend must keep working in that case.
pub(crate) fn resolve_telegram_config() -> Option<TelegramConfig> {
    let bot_token = env::var("TELEGRAM_BOT_TOKEN")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())?;

    let channel = env::var("TELEGRAM_CHANNEL")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "@svit_ikony".to_string());

    let webhook_secret = env::var("TELEGRAM_WEBHOOK_SECRET")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());

    if webhook_secret.is_none() {
        tracing::warn!(
            "⚠️ TELEGRAM_WEBHOOK_SECRET not set — /telegram/webhook will reject every \
             request (401) until it is configured. The rest of the backend is unaffected."
        );
    }

    tracing::info!("✅ Telegram bot configured (channel: {})", channel);

    Some(TelegramConfig {
        bot_token,
        webhook_secret,
        channel,
    })
}

/// Check if a secret value is insecure
fn is_insecure_secret(secret: &str) -> bool {
    let lower = secret.to_lowercase();
    INSECURE_SECRETS.iter().any(|s| lower == *s) || secret.len() < 16
}

impl Config {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        dotenvy::dotenv().ok();

        let jwt_secret = env::var("JWT_SECRET")?;

        // Resolve admin JWT secret with safe fallback
        let admin_jwt_secret = env::var("ADMIN_JWT_SECRET").unwrap_or_else(|_| jwt_secret.clone());

        // Security validation
        let is_test = env::var("RUST_TEST").is_ok() || cfg!(test);
        if !is_test {
            if is_insecure_secret(&jwt_secret) {
                tracing::warn!(
                    "⚠️ JWT_SECRET is extremely weak ('{}'). Please use a 32+ char random string in production.",
                    &jwt_secret[..jwt_secret.len().min(10)]
                );
            }
            if is_insecure_secret(&admin_jwt_secret) {
                tracing::warn!(
                    "⚠️ ADMIN_JWT_SECRET is extremely weak. Please set a separate strong ADMIN_JWT_SECRET."
                );
            }
        }

        Ok(Self {
            database: DatabaseConfig {
                url: env::var("DATABASE_URL")?,
                max_connections: env::var("MAX_DB_CONNECTIONS")
                    .unwrap_or_else(|_| "25".to_string())
                    .parse()?,
            },
            server: ServerConfig {
                port: env::var("PORT")
                    .unwrap_or_else(|_| "8000".to_string())
                    .parse()?,
                rate_limit_per_second: env::var("RATE_LIMIT_PER_SECOND")
                    .unwrap_or_else(|_| "50".to_string())
                    .parse()?,
            },
            jwt: JwtConfig {
                secret: jwt_secret,
                issuer: env::var("JWT_ISSUER").unwrap_or_else(|_| "restaurant-backend".to_string()),
                access_token_ttl_minutes: env::var("ACCESS_TOKEN_TTL_MINUTES")
                    .unwrap_or_else(|_| "15".to_string())
                    .parse()?,
                refresh_token_ttl_days: env::var("REFRESH_TOKEN_TTL_DAYS")
                    .unwrap_or_else(|_| "30".to_string())
                    .parse()?,
            },
            cors: CorsConfig {
                allowed_origins: env::var("CORS_ALLOWED_ORIGINS")
                    .unwrap_or_else(|_| {
                        "http://localhost:3000,http://localhost:3001,http://localhost:5173,http://127.0.0.1:3001,http://127.0.0.1:5173,https://czystetrojmiasto.pl,https://www.czystetrojmiasto.pl,https://kazaxbud.pages.dev,https://svet-ikony.fodi85999.workers.dev,https://svetikony.com,https://www.svetikony.com,https://b2b-saas-tau.vercel.app,https://dima-fomin.pl,https://www.dima-fomin.pl".to_string()
                    })
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect(),
            },
            admin: AdminConfig {
                email: env::var("ADMIN_EMAIL")?,
                password_hash: env::var("ADMIN_PASSWORD_HASH")?,
                jwt_secret: admin_jwt_secret,
                token_ttl_hours: env::var("ADMIN_TOKEN_TTL_HOURS")
                    .unwrap_or_else(|_| "24".to_string())
                    .parse()?,
            },
            r2: R2Config {
                account_id: env::var("CLOUDFLARE_ACCOUNT_ID")?,
                access_key_id: env::var("CLOUDFLARE_R2_ACCESS_KEY_ID")?,
                secret_access_key: env::var("CLOUDFLARE_R2_SECRET_ACCESS_KEY")?,
                bucket_name: env::var("CLOUDFLARE_R2_BUCKET_NAME")?,
                public_url_base: env::var("CLOUDFLARE_R2_PUBLIC_URL")?,
            },
            ai: AiConfig {
                groq_api_key: env::var("GROQ_API_KEY").unwrap_or_else(|_| "".to_string()),
                gemini_api_key: env::var("GEMINI_API_KEY").unwrap_or_else(|_| "".to_string()),
            },
            telegram: resolve_telegram_config(),
        })
    }

    pub fn server_address(&self) -> String {
        format!("0.0.0.0:{}", self.server.port)
    }
}

#[cfg(test)]
mod telegram_config_tests {
    use super::resolve_telegram_config;

    // `std::env` is process-global state and `cargo test` runs tests on
    // multiple threads by default, so any test touching TELEGRAM_* env vars
    // must hold this lock for its whole duration to avoid racing the others.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn clear_telegram_env() {
        std::env::remove_var("TELEGRAM_BOT_TOKEN");
        std::env::remove_var("TELEGRAM_CHANNEL");
        std::env::remove_var("TELEGRAM_WEBHOOK_SECRET");
    }

    #[test]
    fn disabled_when_token_missing() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_telegram_env();

        assert!(resolve_telegram_config().is_none());

        clear_telegram_env();
    }

    #[test]
    fn disabled_when_token_blank() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_telegram_env();
        std::env::set_var("TELEGRAM_BOT_TOKEN", "   ");

        assert!(resolve_telegram_config().is_none());

        clear_telegram_env();
    }

    #[test]
    fn enabled_with_token_uses_production_channel_default() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_telegram_env();
        std::env::set_var("TELEGRAM_BOT_TOKEN", "123:ABC-fake-token-for-test");
        std::env::set_var("TELEGRAM_WEBHOOK_SECRET", "some-secret");

        let cfg = resolve_telegram_config().expect("should be configured");
        assert_eq!(cfg.bot_token, "123:ABC-fake-token-for-test");
        assert_eq!(cfg.channel, "@svit_ikony");
        assert_eq!(cfg.webhook_secret.as_deref(), Some("some-secret"));

        // The Debug impl must never leak the token or the secret.
        let debug_output = format!("{cfg:?}");
        assert!(!debug_output.contains("123:ABC-fake-token-for-test"));
        assert!(!debug_output.contains("some-secret"));

        clear_telegram_env();
    }

    #[test]
    fn custom_channel_overrides_default() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_telegram_env();
        std::env::set_var("TELEGRAM_BOT_TOKEN", "123:ABC-fake-token-for-test");
        std::env::set_var("TELEGRAM_CHANNEL", "@custom_channel");

        let cfg = resolve_telegram_config().expect("should be configured");
        assert_eq!(cfg.channel, "@custom_channel");
        assert!(cfg.webhook_secret.is_none());

        clear_telegram_env();
    }
}
