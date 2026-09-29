//! Application configuration, read from environment variables.
//!
//! Secrets are never given usable defaults. In `staging` and `production` a
//! weak `JWT_SECRET` is a startup error, not a warning.

use anyhow::{anyhow, bail, Context, Result};
use std::env;
use std::str::FromStr;

/// Values that must never be used as secrets outside development.
const INSECURE_SECRETS: &[&str] = &[
    "change_me",
    "changeme",
    "secret",
    "password",
    "jwt_secret",
    "your-super-secret-jwt-key",
];

const MIN_SECRET_LEN: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppEnv {
    Development,
    Staging,
    Production,
}

impl AppEnv {
    pub fn is_production_like(self) -> bool {
        !matches!(self, AppEnv::Development)
    }
}

impl FromStr for AppEnv {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "development" | "dev" | "local" => Ok(AppEnv::Development),
            "staging" => Ok(AppEnv::Staging),
            "production" | "prod" => Ok(AppEnv::Production),
            other => Err(anyhow!(
                "APP_ENV must be development, staging or production (got '{}')",
                other
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub env: AppEnv,
    pub database: DatabaseConfig,
    pub server: ServerConfig,
    pub jwt: JwtConfig,
    pub cors: CorsConfig,
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

#[derive(Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub issuer: String,
    pub audience: String,
    pub access_token_ttl_minutes: i64,
    pub refresh_token_ttl_days: i64,
}

// The secret must never appear in logs, even through `{:?}`.
impl std::fmt::Debug for JwtConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwtConfig")
            .field("secret", &"***redacted***")
            .field("issuer", &self.issuer)
            .field("audience", &self.audience)
            .field("access_token_ttl_minutes", &self.access_token_ttl_minutes)
            .field("refresh_token_ttl_days", &self.refresh_token_ttl_days)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
}

fn required(name: &str) -> Result<String> {
    env::var(name).map_err(|_| anyhow!("missing required environment variable {}", name))
}

fn optional<T>(name: &str, default: T) -> Result<T>
where
    T: FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    match env::var(name) {
        Ok(value) => value
            .trim()
            .parse::<T>()
            .with_context(|| format!("invalid value for {}", name)),
        Err(_) => Ok(default),
    }
}

fn is_insecure_secret(secret: &str) -> bool {
    let lower = secret.to_lowercase();
    INSECURE_SECRETS.iter().any(|s| lower == *s) || secret.len() < MIN_SECRET_LEN
}

fn parse_origins(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|s| s.trim().trim_end_matches('/').to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

impl Config {
    /// Reads configuration from the process environment (the caller loads
    /// `.env` first, if any).
    pub fn from_env() -> Result<Self> {
        let app_env = match env::var("APP_ENV") {
            Ok(v) => v.parse::<AppEnv>()?,
            Err(_) => AppEnv::Development,
        };

        let jwt_secret = required("JWT_SECRET")?;
        if is_insecure_secret(&jwt_secret) {
            if app_env.is_production_like() {
                bail!(
                    "JWT_SECRET is weak: use at least {} random characters (openssl rand -base64 64)",
                    MIN_SECRET_LEN
                );
            }
            tracing::warn!("JWT_SECRET is weak; acceptable only in development");
        }

        Ok(Self {
            env: app_env,
            database: DatabaseConfig {
                url: required("DATABASE_URL")?,
                max_connections: optional("MAX_DB_CONNECTIONS", 10)?,
            },
            server: ServerConfig {
                port: optional("PORT", 8000)?,
                rate_limit_per_second: optional("RATE_LIMIT_PER_SECOND", 50)?,
            },
            jwt: JwtConfig {
                secret: jwt_secret,
                issuer: env::var("JWT_ISSUER").unwrap_or_else(|_| "beauty-backend".to_string()),
                audience: env::var("JWT_AUDIENCE").unwrap_or_else(|_| "beauty-app".to_string()),
                access_token_ttl_minutes: optional("ACCESS_TOKEN_TTL_MINUTES", 15)?,
                refresh_token_ttl_days: optional("REFRESH_TOKEN_TTL_DAYS", 30)?,
            },
            cors: CorsConfig {
                allowed_origins: parse_origins(
                    &env::var("CORS_ALLOWED_ORIGINS").unwrap_or_default(),
                ),
            },
        })
    }

    pub fn server_address(&self) -> String {
        format!("0.0.0.0:{}", self.server.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weak_secrets_are_detected() {
        assert!(is_insecure_secret("secret"));
        assert!(is_insecure_secret("CHANGE_ME"));
        assert!(is_insecure_secret("short"));
        assert!(!is_insecure_secret(
            "R/x7ccoRyHGedn5KuPeOCMVl94V8mlTv6vXYVpxQ7fVTFG8A"
        ));
    }

    #[test]
    fn app_env_parses_aliases_and_rejects_unknown() {
        assert_eq!("prod".parse::<AppEnv>().unwrap(), AppEnv::Production);
        assert_eq!("Staging".parse::<AppEnv>().unwrap(), AppEnv::Staging);
        assert_eq!("dev".parse::<AppEnv>().unwrap(), AppEnv::Development);
        assert!("nonsense".parse::<AppEnv>().is_err());
        assert!(AppEnv::Production.is_production_like());
        assert!(!AppEnv::Development.is_production_like());
    }

    #[test]
    fn origins_are_trimmed_and_empty_entries_dropped() {
        assert_eq!(
            parse_origins(" https://a.example/ , ,http://localhost:3000 "),
            vec![
                "https://a.example".to_string(),
                "http://localhost:3000".to_string()
            ]
        );
        assert!(parse_origins("").is_empty());
    }

    #[test]
    fn jwt_config_debug_never_leaks_secret() {
        let cfg = JwtConfig {
            secret: "super-secret-value".to_string(),
            issuer: "i".to_string(),
            audience: "a".to_string(),
            access_token_ttl_minutes: 15,
            refresh_token_ttl_days: 30,
        };
        assert!(!format!("{:?}", cfg).contains("super-secret-value"));
    }
}
