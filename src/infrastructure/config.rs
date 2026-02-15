use std::env;

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
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub port: u16,
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

impl Config {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        dotenvy::dotenv().ok();

        Ok(Self {
            database: DatabaseConfig {
                url: env::var("DATABASE_URL")?,
            },
            server: ServerConfig {
                port: env::var("PORT")
                    .unwrap_or_else(|_| "8000".to_string())
                    .parse()?,
            },
            jwt: JwtConfig {
                secret: env::var("JWT_SECRET")?,
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
                jwt_secret: env::var("ADMIN_JWT_SECRET")
                    .unwrap_or_else(|_| env::var("JWT_SECRET").unwrap_or_else(|_| "change_me".to_string())),
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
                groq_api_key: env::var("GROQ_API_KEY")
                    .unwrap_or_else(|_| "".to_string()),
            },
        })
    }

    pub fn server_address(&self) -> String {
        format!("0.0.0.0:{}", self.server.port)
    }
}
