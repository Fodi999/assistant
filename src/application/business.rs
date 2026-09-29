//! Businesses (the tenant): creation, membership lookup, profile changes.

use crate::application::access::{BusinessAccess, Role};
use crate::application::auth::{clean_optional, on_unique};
use crate::infrastructure::{begin_scoped, DbScope};
use crate::shared::{AppError, AppResult, BusinessId, UserId};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

const LOCALES: [&str; 4] = ["pl", "en", "ru", "uk"];

#[derive(Debug, Deserialize)]
pub struct CreateBusinessInput {
    pub name: String,
    /// URL-friendly unique name; generated when omitted.
    pub slug: Option<String>,
    pub description: Option<String>,
    pub country: Option<String>,
    pub timezone: Option<String>,
    pub currency: Option<String>,
    pub default_locale: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBusinessInput {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct BusinessRow {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub country: String,
    pub timezone: String,
    pub currency: String,
    pub default_locale: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct StaffView {
    pub id: Uuid,
    pub display_name: String,
    pub photo_url: Option<String>,
    pub bio: Option<String>,
    pub color: Option<String>,
    pub is_bookable: bool,
    pub sort_order: i32,
    /// True for the caller's own staff card.
    pub is_mine: bool,
}

#[derive(Debug, Serialize)]
pub struct BusinessView {
    #[serde(flatten)]
    pub business: BusinessRow,
    /// The caller's role in this business.
    pub role: Role,
}

const SELECT_BUSINESS: &str = "SELECT id, name, slug, description, country::text AS country,
        timezone, currency::text AS currency, default_locale
     FROM business WHERE id = $1 AND deleted_at IS NULL";

#[derive(Clone)]
pub struct BusinessService {
    pool: PgPool,
}

impl BusinessService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Resolves the caller's role in a business. Non-members get 404, so the
    /// existence of other people's businesses is not revealed.
    pub async fn access(
        &self,
        user_id: UserId,
        business_id: BusinessId,
    ) -> AppResult<BusinessAccess> {
        let mut tx = begin_scoped(&self.pool, DbScope::user(user_id)).await?;
        let row: Option<(String, String)> = sqlx::query_as(
            "SELECT m.role, m.status
             FROM membership m
             JOIN business b ON b.id = m.business_id
             WHERE m.business_id = $1 AND m.user_id = $2 AND b.deleted_at IS NULL",
        )
        .bind(business_id.as_uuid())
        .bind(user_id.as_uuid())
        .fetch_optional(&mut *tx)
        .await?;

        match row {
            Some((role, status)) if status == "active" => Ok(BusinessAccess {
                user_id,
                business_id,
                role: Role::parse(&role)?,
            }),
            _ => Err(AppError::not_found("Business not found")),
        }
    }

    /// Creates a business with the caller as owner, plus a bookable staff card
    /// for them (a solo master can take bookings right away).
    pub async fn create(
        &self,
        user_id: UserId,
        input: CreateBusinessInput,
    ) -> AppResult<BusinessView> {
        let name = input.name.trim().to_string();
        if name.is_empty() || name.chars().count() > 255 {
            return Err(AppError::validation("name must be 1-255 characters"));
        }
        let slug = match input.slug.as_deref().map(str::trim) {
            Some(slug) if !slug.is_empty() => validate_slug(slug)?,
            _ => generated_slug(),
        };
        let description = clean_optional(input.description, 2000, "description")?;
        let country = upper_code(input.country.as_deref(), "PL", 2, "country")?;
        let currency = upper_code(input.currency.as_deref(), "PLN", 3, "currency")?;
        let default_locale = match input.default_locale.as_deref().map(str::trim) {
            None | Some("") => "pl".to_string(),
            Some(value) if LOCALES.contains(&value) => value.to_string(),
            Some(_) => {
                return Err(AppError::validation(
                    "default_locale must be one of pl, en, ru, uk",
                ))
            }
        };
        let timezone = input
            .timezone
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("Europe/Warsaw")
            .to_string();

        let business_id = BusinessId::new();
        // The insert policy only accepts a business whose id equals the scope's
        // business id, so the scope is set to the new id up front.
        let mut tx = begin_scoped(&self.pool, DbScope::business(user_id, business_id)).await?;

        let timezone_known: bool =
            sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_timezone_names WHERE name = $1)")
                .bind(&timezone)
                .fetch_one(&mut *tx)
                .await?;
        if !timezone_known {
            return Err(AppError::validation("timezone is not a known IANA zone"));
        }

        sqlx::query(
            "INSERT INTO business (id, name, slug, description, country, timezone, currency, default_locale)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(business_id.as_uuid())
        .bind(&name)
        .bind(&slug)
        .bind(&description)
        .bind(&country)
        .bind(&timezone)
        .bind(&currency)
        .bind(&default_locale)
        .execute(&mut *tx)
        .await
        .map_err(on_unique("This slug is already taken"))?;

        let membership_id: Uuid = sqlx::query_scalar(
            "INSERT INTO membership (business_id, user_id, role, status)
             VALUES ($1, $2, 'owner', 'active')
             RETURNING id",
        )
        .bind(business_id.as_uuid())
        .bind(user_id.as_uuid())
        .fetch_one(&mut *tx)
        .await?;

        let owner_name: Option<String> =
            sqlx::query_scalar("SELECT display_name FROM users WHERE id = $1")
                .bind(user_id.as_uuid())
                .fetch_one(&mut *tx)
                .await?;
        let card_name = owner_name
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| name.chars().take(120).collect());
        sqlx::query(
            "INSERT INTO staff_member (business_id, membership_id, display_name) VALUES ($1, $2, $3)",
        )
        .bind(business_id.as_uuid())
        .bind(membership_id)
        .bind(&card_name)
        .execute(&mut *tx)
        .await?;

        write_audit(&mut tx, business_id, user_id, "business.create").await?;
        let business = load_business(&mut tx, business_id).await?;
        tx.commit().await?;

        Ok(BusinessView {
            business,
            role: Role::Owner,
        })
    }

    pub async fn get(&self, access: BusinessAccess) -> AppResult<BusinessView> {
        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        let business = load_business(&mut tx, access.business_id).await?;
        Ok(BusinessView {
            business,
            role: access.role,
        })
    }

    /// People who can be booked in this business. Visible to every member.
    pub async fn list_staff(&self, access: BusinessAccess) -> AppResult<Vec<StaffView>> {
        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        let staff = sqlx::query_as::<_, StaffView>(
            "SELECT s.id, s.display_name, s.photo_url, s.bio, s.color, s.is_bookable, s.sort_order,
                    COALESCE(m.user_id = $2, false) AS is_mine
             FROM staff_member s
             LEFT JOIN membership m ON m.id = s.membership_id
             WHERE s.business_id = $1
             ORDER BY s.sort_order, s.display_name",
        )
        .bind(access.business_id.as_uuid())
        .bind(access.user_id.as_uuid())
        .fetch_all(&mut *tx)
        .await?;
        Ok(staff)
    }

    pub async fn update(
        &self,
        access: BusinessAccess,
        input: UpdateBusinessInput,
    ) -> AppResult<BusinessView> {
        access.require(&[Role::Owner, Role::Manager])?;

        let name = match input.name {
            Some(value) => {
                let value = value.trim().to_string();
                if value.is_empty() || value.chars().count() > 255 {
                    return Err(AppError::validation("name must be 1-255 characters"));
                }
                Some(value)
            }
            None => None,
        };
        let description = clean_optional(input.description, 2000, "description")?;

        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        sqlx::query(
            "UPDATE business SET name = COALESCE($2, name), description = COALESCE($3, description)
             WHERE id = $1",
        )
        .bind(access.business_id.as_uuid())
        .bind(&name)
        .bind(&description)
        .execute(&mut *tx)
        .await?;
        write_audit(
            &mut tx,
            access.business_id,
            access.user_id,
            "business.update",
        )
        .await?;
        let business = load_business(&mut tx, access.business_id).await?;
        tx.commit().await?;

        Ok(BusinessView {
            business,
            role: access.role,
        })
    }
}

async fn load_business(conn: &mut PgConnection, id: BusinessId) -> AppResult<BusinessRow> {
    sqlx::query_as::<_, BusinessRow>(SELECT_BUSINESS)
        .bind(id.as_uuid())
        .fetch_optional(&mut *conn)
        .await?
        .ok_or_else(|| AppError::not_found("Business not found"))
}

async fn write_audit(
    conn: &mut PgConnection,
    business_id: BusinessId,
    actor: UserId,
    action: &str,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO audit_log (business_id, actor_user_id, action, entity, entity_id)
         VALUES ($1, $2, $3, 'business', $1)",
    )
    .bind(business_id.as_uuid())
    .bind(actor.as_uuid())
    .bind(action)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// 3-63 characters of a-z, 0-9 and single hyphens, not starting or ending with one.
fn validate_slug(slug: &str) -> AppResult<String> {
    let valid = (3..=63).contains(&slug.len())
        && slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !slug.starts_with('-')
        && !slug.ends_with('-')
        && !slug.contains("--");
    if valid {
        Ok(slug.to_string())
    } else {
        Err(AppError::validation(
            "slug must be 3-63 characters: lowercase letters, digits and single hyphens",
        ))
    }
}

fn generated_slug() -> String {
    let random = Uuid::new_v4().simple().to_string();
    format!("b-{}", &random[..10])
}

fn upper_code(raw: Option<&str>, default: &str, len: usize, field: &str) -> AppResult<String> {
    match raw.map(str::trim).filter(|value| !value.is_empty()) {
        None => Ok(default.to_string()),
        Some(value) if value.len() == len && value.chars().all(|c| c.is_ascii_alphabetic()) => {
            Ok(value.to_ascii_uppercase())
        }
        Some(_) => Err(AppError::validation(format!(
            "{field} must be {len} letters"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_are_validated() {
        assert!(validate_slug("anna-lashes").is_ok());
        assert!(validate_slug("ab").is_err());
        for bad in [
            "-anna",
            "anna-",
            "an--na",
            "Anna",
            "anna lashes",
            "anna_lashes",
        ] {
            assert!(validate_slug(bad).is_err(), "{bad:?} must be rejected");
        }
    }

    #[test]
    fn generated_slugs_pass_validation() {
        for _ in 0..20 {
            assert!(validate_slug(&generated_slug()).is_ok());
        }
    }

    #[test]
    fn codes_are_uppercased_and_checked() {
        assert_eq!(upper_code(None, "PL", 2, "country").unwrap(), "PL");
        assert_eq!(upper_code(Some("de"), "PL", 2, "country").unwrap(), "DE");
        assert!(upper_code(Some("DEU"), "PL", 2, "country").is_err());
        assert!(upper_code(Some("1A"), "PL", 2, "country").is_err());
    }
}
