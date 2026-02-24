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

/// AI Services Configuration (Groq for translations)
#[derive(Debug, Clone)]
pub struct AiConfig {
    pub groq_api_key: String,
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
                    .unwrap_or_else(|_| "http://localhost:3000".to_string())
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
            },
        })
    }

    pub fn server_address(&self) -> String {
        format!("0.0.0.0:{}", self.server.port)
    }
}
