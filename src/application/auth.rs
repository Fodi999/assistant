//! Accounts and sessions: email + password sign-up and sign-in, rotating
//! refresh tokens with reuse detection, logout, and the "who am I" view.
//!
//! Sign in with Apple / Google and one-time codes plug in later as more ways to
//! obtain the same token pair; everything after that point is shared.

use crate::infrastructure::{begin_scoped, AppCache, DbScope, JwtService, PasswordHasher};
use crate::shared::{AppError, AppResult, UserId};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{PgConnection, PgPool};
use std::time::Duration;
use time::OffsetDateTime;
use uuid::Uuid;

/// After this many wrong passwords for one email, sign-in is blocked for
/// [`LOCKOUT`] since the last failure.
const MAX_FAILED_LOGINS: u64 = 5;
const LOCKOUT: Duration = Duration::from_secs(600);

const MIN_PASSWORD_LEN: usize = 10;
const MAX_PASSWORD_LEN: usize = 128;
const DEFAULT_TERMS_VERSION: &str = "v1";
const LOCALES: [&str; 4] = ["pl", "en", "ru", "uk"];
const PLATFORMS: [&str; 3] = ["ios", "android", "web"];

#[derive(Debug, Deserialize)]
pub struct RegisterInput {
    pub email: String,
    pub password: String,
    pub display_name: Option<String>,
    pub locale: Option<String>,
    /// Must be true: terms of service and privacy policy (stored as consents).
    pub accepted_terms: bool,
    pub terms_version: Option<String>,
    pub device: Option<DeviceInput>,
}

/// An anonymous customer session ("book without an account"). It is an
/// ordinary user without e-mail or password, so tokens, refresh and the
/// booking rules work unchanged; the person can register later.
#[derive(Debug, Deserialize)]
pub struct GuestInput {
    /// Must be true: terms of service and privacy policy (stored as consents).
    pub accepted_terms: bool,
    pub terms_version: Option<String>,
    pub display_name: Option<String>,
    pub locale: Option<String>,
    pub device: Option<DeviceInput>,
}

#[derive(Debug, Deserialize)]
pub struct LoginInput {
    pub email: String,
    pub password: String,
    pub device: Option<DeviceInput>,
}

#[derive(Debug, Deserialize)]
pub struct DeviceInput {
    pub platform: String,
    pub push_token: Option<String>,
    pub app_version: Option<String>,
    pub locale: Option<String>,
    pub timezone: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: &'static str,
    /// Access-token lifetime in seconds.
    pub expires_in: i64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct UserView {
    pub id: Uuid,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub locale: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub user: UserView,
    pub tokens: TokenPair,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct MembershipView {
    pub business_id: Uuid,
    pub business_name: String,
    pub business_slug: String,
    pub role: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub user: UserView,
    pub memberships: Vec<MembershipView>,
}

#[derive(sqlx::FromRow)]
struct LoginRow {
    user_id: Uuid,
    password_hash: String,
    status: String,
}

#[derive(sqlx::FromRow)]
struct RefreshRow {
    token_id: Uuid,
    user_id: Uuid,
    device_id: Option<Uuid>,
    family_id: Uuid,
    expires_at: OffsetDateTime,
    revoked_at: Option<OffsetDateTime>,
    user_status: String,
}

#[derive(Clone)]
pub struct AuthService {
    pool: PgPool,
    jwt: JwtService,
    passwords: PasswordHasher,
    /// Failed-login counters per email (in memory, per instance).
    login_failures: AppCache,
}

impl AuthService {
    pub fn new(pool: PgPool, jwt: JwtService) -> Self {
        Self {
            pool,
            jwt,
            passwords: PasswordHasher::new(),
            login_failures: AppCache::new(50_000, LOCKOUT),
        }
    }

    pub async fn register(&self, input: RegisterInput) -> AppResult<AuthResponse> {
        let email = normalize_email(&input.email)?;
        validate_password(&input.password)?;
        if !input.accepted_terms {
            return Err(AppError::validation(
                "The terms of service and privacy policy must be accepted",
            ));
        }
        let display_name = clean_optional(input.display_name, 120, "display_name")?;
        let locale = normalize_locale(input.locale.as_deref())?;
        let terms_version = clean_optional(input.terms_version, 40, "terms_version")?
            .unwrap_or_else(|| DEFAULT_TERMS_VERSION.to_string());
        let password_hash = self.hash(input.password).await?;

        let user_id = UserId::new();
        let mut tx = begin_scoped(&self.pool, DbScope::user(user_id)).await?;

        sqlx::query("INSERT INTO users (id, email, display_name, locale) VALUES ($1, $2, $3, $4)")
            .bind(user_id.as_uuid())
            .bind(&email)
            .bind(&display_name)
            .bind(&locale)
            .execute(&mut *tx)
            .await
            .map_err(on_unique("An account with this email already exists"))?;

        sqlx::query(
            "INSERT INTO auth_identity (user_id, provider, provider_subject) VALUES ($1, 'email', $2)",
        )
        .bind(user_id.as_uuid())
        .bind(&email)
        .execute(&mut *tx)
        .await?;

        sqlx::query("INSERT INTO user_password (user_id, password_hash) VALUES ($1, $2)")
            .bind(user_id.as_uuid())
            .bind(&password_hash)
            .execute(&mut *tx)
            .await?;

        for consent in ["terms", "privacy"] {
            sqlx::query(
                "INSERT INTO consent (user_id, type, granted, text_version) VALUES ($1, $2, true, $3)",
            )
            .bind(user_id.as_uuid())
            .bind(consent)
            .bind(&terms_version)
            .execute(&mut *tx)
            .await?;
        }

        let device_id = match &input.device {
            Some(device) => Some(insert_device(&mut tx, user_id, device).await?),
            None => None,
        };
        let (tokens, _) = self
            .issue_tokens(&mut tx, user_id, device_id, Uuid::now_v7())
            .await?;
        let user = load_user(&mut tx, user_id).await?;
        tx.commit().await?;

        Ok(AuthResponse { user, tokens })
    }

    /// Starts an anonymous customer session (no e-mail, no password).
    pub async fn guest(&self, input: GuestInput) -> AppResult<AuthResponse> {
        if !input.accepted_terms {
            return Err(AppError::validation(
                "The terms of service and privacy policy must be accepted",
            ));
        }
        let display_name = clean_optional(input.display_name, 120, "display_name")?;
        let locale = normalize_locale(input.locale.as_deref())?;
        let terms_version = clean_optional(input.terms_version, 40, "terms_version")?
            .unwrap_or_else(|| DEFAULT_TERMS_VERSION.to_string());

        let user_id = UserId::new();
        let mut tx = begin_scoped(&self.pool, DbScope::user(user_id)).await?;
        sqlx::query("INSERT INTO users (id, display_name, locale) VALUES ($1, $2, $3)")
            .bind(user_id.as_uuid())
            .bind(&display_name)
            .bind(&locale)
            .execute(&mut *tx)
            .await?;
        for consent in ["terms", "privacy"] {
            sqlx::query(
                "INSERT INTO consent (user_id, type, granted, text_version, source)
                 VALUES ($1, $2, true, $3, 'guest')",
            )
            .bind(user_id.as_uuid())
            .bind(consent)
            .bind(&terms_version)
            .execute(&mut *tx)
            .await?;
        }
        let device_id = match &input.device {
            Some(device) => Some(insert_device(&mut tx, user_id, device).await?),
            None => None,
        };
        let (tokens, _) = self
            .issue_tokens(&mut tx, user_id, device_id, Uuid::now_v7())
            .await?;
        let user = load_user(&mut tx, user_id).await?;
        tx.commit().await?;
        Ok(AuthResponse { user, tokens })
    }

    pub async fn login(&self, input: LoginInput) -> AppResult<AuthResponse> {
        let email = normalize_email(&input.email)?;
        if self.failed_logins(&email) >= MAX_FAILED_LOGINS {
            return Err(AppError::RateLimited(
                "Too many failed attempts. Try again later.".to_string(),
            ));
        }

        let row = sqlx::query_as::<_, LoginRow>(
            "SELECT user_id, password_hash, status FROM auth_login_lookup($1)",
        )
        .bind(&email)
        .fetch_optional(&self.pool)
        .await?;

        // Unknown emails still pay for one hash, so response time does not
        // reveal whether an account exists.
        let verified = match &row {
            Some(found) => {
                self.verify(input.password.clone(), found.password_hash.clone())
                    .await?
            }
            None => {
                self.hash(input.password.clone()).await?;
                false
            }
        };
        let found = match row {
            Some(found) if verified => found,
            _ => {
                self.record_failed_login(&email);
                return Err(AppError::authentication("Invalid email or password"));
            }
        };
        if found.status != "active" {
            return Err(AppError::authentication("This account is not active"));
        }
        self.clear_failed_logins(&email);

        let user_id = UserId::from_uuid(found.user_id);
        let mut tx = begin_scoped(&self.pool, DbScope::user(user_id)).await?;
        let device_id = match &input.device {
            Some(device) => Some(insert_device(&mut tx, user_id, device).await?),
            None => None,
        };
        let (tokens, _) = self
            .issue_tokens(&mut tx, user_id, device_id, Uuid::now_v7())
            .await?;
        let user = load_user(&mut tx, user_id).await?;
        tx.commit().await?;

        Ok(AuthResponse { user, tokens })
    }

    /// Exchanges a refresh token for a new pair. The presented token is used up;
    /// presenting a used-up token again revokes the whole login (family), on the
    /// assumption that it was stolen.
    pub async fn refresh(&self, refresh_token: &str) -> AppResult<TokenPair> {
        let row = self
            .find_refresh_token(refresh_token)
            .await?
            .ok_or_else(|| AppError::authentication("Invalid refresh token"))?;
        if row.user_status != "active" {
            return Err(AppError::authentication("This account is not active"));
        }

        let user_id = UserId::from_uuid(row.user_id);
        let mut tx = begin_scoped(&self.pool, DbScope::user(user_id)).await?;

        if row.revoked_at.is_some() {
            return reject_reuse(tx, row.family_id).await;
        }
        if row.expires_at <= OffsetDateTime::now_utc() {
            return Err(AppError::authentication("Refresh token expired"));
        }

        // The guard `revoked_at IS NULL` makes two concurrent refreshes of the
        // same token race safely: exactly one wins, the other is treated as reuse.
        let used: Option<Uuid> = sqlx::query_scalar(
            "UPDATE refresh_token SET revoked_at = now() WHERE id = $1 AND revoked_at IS NULL RETURNING id",
        )
        .bind(row.token_id)
        .fetch_optional(&mut *tx)
        .await?;
        if used.is_none() {
            return reject_reuse(tx, row.family_id).await;
        }

        let (tokens, new_id) = self
            .issue_tokens(&mut tx, user_id, row.device_id, row.family_id)
            .await?;
        sqlx::query("UPDATE refresh_token SET replaced_by = $2 WHERE id = $1")
            .bind(row.token_id)
            .bind(new_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;

        Ok(tokens)
    }

    /// Ends the login this refresh token belongs to. Unknown tokens are ignored,
    /// so the endpoint never reveals whether a token was valid.
    pub async fn logout(&self, refresh_token: &str) -> AppResult<()> {
        if let Some(row) = self.find_refresh_token(refresh_token).await? {
            let mut tx =
                begin_scoped(&self.pool, DbScope::user(UserId::from_uuid(row.user_id))).await?;
            revoke_family(&mut tx, row.family_id).await?;
            tx.commit().await?;
        }
        Ok(())
    }

    pub async fn me(&self, user_id: UserId) -> AppResult<MeResponse> {
        let mut tx = begin_scoped(&self.pool, DbScope::user(user_id)).await?;
        let user = load_user(&mut tx, user_id).await?;
        let memberships = sqlx::query_as::<_, MembershipView>(
            "SELECT m.business_id, b.name AS business_name, b.slug AS business_slug, m.role, m.status
             FROM membership m
             JOIN business b ON b.id = m.business_id
             WHERE m.user_id = $1 AND m.status = 'active' AND b.deleted_at IS NULL
             ORDER BY b.name",
        )
        .bind(user_id.as_uuid())
        .fetch_all(&mut *tx)
        .await?;

        Ok(MeResponse { user, memberships })
    }

    async fn find_refresh_token(&self, token: &str) -> AppResult<Option<RefreshRow>> {
        if token.is_empty() || token.len() > 256 {
            return Ok(None);
        }
        let row = sqlx::query_as::<_, RefreshRow>(
            "SELECT token_id, user_id, device_id, family_id, expires_at, revoked_at, user_status
             FROM auth_refresh_lookup($1)",
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn issue_tokens(
        &self,
        conn: &mut PgConnection,
        user_id: UserId,
        device_id: Option<Uuid>,
        family_id: Uuid,
    ) -> AppResult<(TokenPair, Uuid)> {
        let access_token = self.jwt.generate_access_token(user_id)?;
        let refresh_token = self.jwt.generate_refresh_token();
        let expires_at = OffsetDateTime::now_utc() + self.jwt.refresh_token_ttl();

        let row_id: Uuid = sqlx::query_scalar(
            "INSERT INTO refresh_token (user_id, device_id, family_id, token_hash, expires_at)
             VALUES ($1, $2, $3, hash_token($4), $5)
             RETURNING id",
        )
        .bind(user_id.as_uuid())
        .bind(device_id)
        .bind(family_id)
        .bind(&refresh_token)
        .bind(expires_at)
        .fetch_one(&mut *conn)
        .await?;

        let tokens = TokenPair {
            access_token,
            refresh_token,
            token_type: "Bearer",
            expires_in: self.jwt.access_token_ttl_seconds(),
        };
        Ok((tokens, row_id))
    }

    // Password hashing is CPU-heavy: keep it off the async worker threads.
    async fn hash(&self, password: String) -> AppResult<String> {
        let hasher = self.passwords.clone();
        tokio::task::spawn_blocking(move || hasher.hash_password(&password))
            .await
            .map_err(|e| AppError::internal(format!("Password task failed: {e}")))?
    }

    async fn verify(&self, password: String, hash: String) -> AppResult<bool> {
        let hasher = self.passwords.clone();
        tokio::task::spawn_blocking(move || hasher.verify_password(&password, &hash))
            .await
            .map_err(|e| AppError::internal(format!("Password task failed: {e}")))?
    }

    fn failed_logins(&self, email: &str) -> u64 {
        self.login_failures
            .get(&login_key(email))
            .and_then(|value| value.as_u64())
            .unwrap_or(0)
    }

    fn record_failed_login(&self, email: &str) {
        let count = self.failed_logins(email) + 1;
        self.login_failures.set(login_key(email), json!(count));
    }

    fn clear_failed_logins(&self, email: &str) {
        self.login_failures.bust(&login_key(email));
    }
}

fn login_key(email: &str) -> String {
    format!("login_fail:{email}")
}

/// Revokes the whole login and reports the reuse. The revocation is committed
/// before the error is returned, otherwise the rollback would undo it.
async fn reject_reuse(
    mut tx: sqlx::Transaction<'static, sqlx::Postgres>,
    family_id: Uuid,
) -> AppResult<TokenPair> {
    revoke_family(&mut tx, family_id).await?;
    tx.commit().await?;
    Err(AppError::authentication("Refresh token was already used"))
}

async fn revoke_family(conn: &mut PgConnection, family_id: Uuid) -> AppResult<()> {
    sqlx::query(
        "UPDATE refresh_token SET revoked_at = now() WHERE family_id = $1 AND revoked_at IS NULL",
    )
    .bind(family_id)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

async fn load_user(conn: &mut PgConnection, user_id: UserId) -> AppResult<UserView> {
    sqlx::query_as::<_, UserView>(
        "SELECT id, email, display_name, locale, avatar_url FROM users WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(user_id.as_uuid())
    .fetch_optional(&mut *conn)
    .await?
    .ok_or_else(|| AppError::authentication("Account not found"))
}

async fn insert_device(
    conn: &mut PgConnection,
    user_id: UserId,
    device: &DeviceInput,
) -> AppResult<Uuid> {
    if !PLATFORMS.contains(&device.platform.as_str()) {
        return Err(AppError::validation(
            "device.platform must be ios, android or web",
        ));
    }
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO device (user_id, platform, push_token, app_version, locale, timezone, last_seen_at)
         VALUES ($1, $2, $3, $4, $5, $6, now())
         RETURNING id",
    )
    .bind(user_id.as_uuid())
    .bind(&device.platform)
    .bind(&device.push_token)
    .bind(&device.app_version)
    .bind(&device.locale)
    .bind(&device.timezone)
    .fetch_one(&mut *conn)
    .await?;
    Ok(id)
}

/// Maps a unique-constraint violation to a 409 with a friendly message.
pub(crate) fn on_unique(message: &'static str) -> impl FnOnce(sqlx::Error) -> AppError {
    move |error| {
        let unique = error
            .as_database_error()
            .and_then(|db| db.code())
            .map(|code| code == "23505")
            .unwrap_or(false);
        if unique {
            AppError::conflict(message)
        } else {
            AppError::from(error)
        }
    }
}

pub(crate) fn clean_optional(
    value: Option<String>,
    max_len: usize,
    field: &str,
) -> AppResult<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim().to_string();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > max_len {
        return Err(AppError::validation(format!(
            "{field} must be at most {max_len} characters"
        )));
    }
    Ok(Some(value))
}

pub(crate) fn normalize_email(raw: &str) -> AppResult<String> {
    let email = raw.trim().to_lowercase();
    let valid = email.len() <= 254
        && !email.chars().any(|c| c.is_whitespace() || c.is_control())
        && match email.split_once('@') {
            Some((local, domain)) => {
                !local.is_empty()
                    && !domain.contains('@')
                    && domain.contains('.')
                    && !domain.starts_with('.')
                    && !domain.ends_with('.')
            }
            None => false,
        };
    if valid {
        Ok(email)
    } else {
        Err(AppError::validation("Email address is not valid"))
    }
}

fn validate_password(password: &str) -> AppResult<()> {
    let length = password.chars().count();
    if !(MIN_PASSWORD_LEN..=MAX_PASSWORD_LEN).contains(&length) {
        return Err(AppError::validation(format!(
            "Password must be {MIN_PASSWORD_LEN}-{MAX_PASSWORD_LEN} characters"
        )));
    }
    Ok(())
}

fn normalize_locale(raw: Option<&str>) -> AppResult<String> {
    match raw.map(str::trim).filter(|value| !value.is_empty()) {
        None => Ok("pl".to_string()),
        Some(value) if LOCALES.contains(&value) => Ok(value.to_string()),
        Some(_) => Err(AppError::validation("locale must be one of pl, en, ru, uk")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emails_are_trimmed_lowercased_and_checked() {
        assert_eq!(
            normalize_email("  Ann@Example.PL ").unwrap(),
            "ann@example.pl"
        );
        for bad in [
            "",
            "ann",
            "ann@",
            "@example.pl",
            "a b@example.pl",
            "ann@example",
            "a@@b.pl",
            "ann@.pl",
        ] {
            assert!(normalize_email(bad).is_err(), "{bad:?} must be rejected");
        }
    }

    #[test]
    fn password_length_is_enforced() {
        assert!(validate_password("short").is_err());
        assert!(validate_password("long enough password").is_ok());
        assert!(validate_password(&"x".repeat(129)).is_err());
    }

    #[test]
    fn locale_defaults_to_polish_and_rejects_unknown() {
        assert_eq!(normalize_locale(None).unwrap(), "pl");
        assert_eq!(normalize_locale(Some("uk")).unwrap(), "uk");
        assert!(normalize_locale(Some("de")).is_err());
    }

    #[test]
    fn optional_text_is_trimmed_and_bounded() {
        assert_eq!(clean_optional(None, 5, "x").unwrap(), None);
        assert_eq!(clean_optional(Some("  ".into()), 5, "x").unwrap(), None);
        assert_eq!(
            clean_optional(Some(" ab ".into()), 5, "x").unwrap(),
            Some("ab".to_string())
        );
        assert!(clean_optional(Some("abcdef".into()), 5, "x").is_err());
    }
}
