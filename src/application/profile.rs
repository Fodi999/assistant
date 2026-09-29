//! The public profile of a business, edited by its owner or manager.
//!
//! A business appears in the public catalog only when a platform admin has
//! approved it (`moderation_status`) AND the owner has published the profile
//! (`is_published`). The owner cannot change the moderation status.

use crate::application::access::{BusinessAccess, Role};
use crate::infrastructure::begin_scoped;
use crate::shared::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};

/// Absent fields stay as they are; an empty string clears a text field.
#[derive(Debug, Deserialize)]
pub struct UpdateProfileInput {
    pub city: Option<String>,
    pub headline: Option<String>,
    pub about: Option<String>,
    /// Instagram handle, with or without `@`.
    pub instagram: Option<String>,
    pub is_published: Option<bool>,
    /// One of `BUSINESS_TYPES`; an empty string clears it.
    pub business_type: Option<String>,
    /// Street address as a single line (no geocoding); empty clears it.
    pub address_line: Option<String>,
}

/// Fixed set of business types offered at onboarding.
pub const BUSINESS_TYPES: [&str; 6] =
    ["lashes", "brows", "nails", "hair", "beauty_studio", "other"];

#[derive(Debug, Serialize)]
pub struct ProfileView {
    pub city: Option<String>,
    pub headline: Option<String>,
    pub about: Option<String>,
    pub instagram: Option<String>,
    pub business_type: Option<String>,
    pub address_line: Option<String>,
    pub is_published: bool,
    /// `pending`, `approved`, `rejected` or `suspended` (set by the platform).
    pub moderation_status: String,
    /// The platform's latest note (why a request was rejected or suspended).
    pub moderation_note: Option<String>,
    /// True when customers can find this business in the public catalog.
    pub publicly_visible: bool,
}

type ProfileRow = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    bool,
    String,
    String,
    Option<String>,
    Option<String>,
);

/// `(changed, new value)` of an optional text field.
fn text_change(
    value: Option<String>,
    max_len: usize,
    field: &str,
) -> AppResult<(bool, Option<String>)> {
    let Some(value) = value else {
        return Ok((false, None));
    };
    let value = value.trim().to_string();
    if value.is_empty() {
        return Ok((true, None));
    }
    if value.chars().count() > max_len {
        return Err(AppError::validation(format!(
            "{field} must be at most {max_len} characters"
        )));
    }
    Ok((true, Some(value)))
}

fn business_type_change(value: Option<String>) -> AppResult<(bool, Option<String>)> {
    let (changed, value) = text_change(value, 30, "business_type")?;
    match value {
        None => Ok((changed, None)),
        Some(v) if BUSINESS_TYPES.contains(&v.as_str()) => Ok((true, Some(v))),
        Some(_) => Err(AppError::validation(format!(
            "business_type must be one of: {}",
            BUSINESS_TYPES.join(", ")
        ))),
    }
}

fn instagram_change(value: Option<String>) -> AppResult<(bool, Option<String>)> {
    let (changed, value) = text_change(value, 31, "instagram")?;
    let Some(value) = value else {
        return Ok((changed, None));
    };
    let handle = value.strip_prefix('@').unwrap_or(&value);
    let ok = (1..=30).contains(&handle.len())
        && handle
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_');
    if !ok {
        return Err(AppError::validation(
            "instagram must be a handle of letters, digits, dots and underscores",
        ));
    }
    Ok((true, Some(handle.to_string())))
}

#[derive(Clone)]
pub struct ProfileService {
    pool: PgPool,
}

impl ProfileService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Visible to every member of the business.
    pub async fn get(&self, access: BusinessAccess) -> AppResult<ProfileView> {
        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        load(&mut tx, access).await
    }

    pub async fn update(
        &self,
        access: BusinessAccess,
        input: UpdateProfileInput,
    ) -> AppResult<ProfileView> {
        access.require(&[Role::Owner, Role::Manager])?;
        let (city_set, city) = text_change(input.city, 80, "city")?;
        let (headline_set, headline) = text_change(input.headline, 140, "headline")?;
        let (about_set, about) = text_change(input.about, 2000, "about")?;
        let (instagram_set, instagram) = instagram_change(input.instagram)?;
        let (type_set, business_type) = business_type_change(input.business_type)?;
        let (address_set, address_line) = text_change(input.address_line, 200, "address_line")?;

        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        sqlx::query(
            "UPDATE business SET
                 city = CASE WHEN $2 THEN $3 ELSE city END,
                 headline = CASE WHEN $4 THEN $5 ELSE headline END,
                 about = CASE WHEN $6 THEN $7 ELSE about END,
                 instagram = CASE WHEN $8 THEN $9 ELSE instagram END,
                 is_published = COALESCE($10, is_published),
                 business_type = CASE WHEN $11 THEN $12 ELSE business_type END,
                 address_line = CASE WHEN $13 THEN $14 ELSE address_line END
             WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(access.business_id.as_uuid())
        .bind(city_set)
        .bind(city)
        .bind(headline_set)
        .bind(headline)
        .bind(about_set)
        .bind(about)
        .bind(instagram_set)
        .bind(instagram)
        .bind(input.is_published)
        .bind(type_set)
        .bind(business_type)
        .bind(address_set)
        .bind(address_line)
        .execute(&mut *tx)
        .await?;
        let view = load(&mut tx, access).await?;
        tx.commit().await?;
        Ok(view)
    }
}

async fn load(conn: &mut PgConnection, access: BusinessAccess) -> AppResult<ProfileView> {
    let row: ProfileRow = sqlx::query_as(
        "SELECT city, headline, about, instagram, is_published, moderation_status, status,
                business_type, address_line
         FROM business WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(access.business_id.as_uuid())
    .fetch_optional(&mut *conn)
    .await?
    .ok_or_else(|| AppError::not_found("Business not found"))?;
    let note: Option<String> = sqlx::query_scalar::<_, Option<String>>(
        "SELECT note FROM business_moderation
         WHERE business_id = $1 ORDER BY created_at DESC, id LIMIT 1",
    )
    .bind(access.business_id.as_uuid())
    .fetch_optional(&mut *conn)
    .await?
    .flatten();
    let visible = row.4 && row.5 == "approved" && row.6 == "active";
    Ok(ProfileView {
        city: row.0,
        headline: row.1,
        about: row.2,
        instagram: row.3,
        business_type: row.7,
        address_line: row.8,
        is_published: row.4,
        moderation_status: row.5,
        moderation_note: note,
        publicly_visible: visible,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_fields_keep_clear_or_set() {
        assert_eq!(text_change(None, 10, "x").unwrap(), (false, None));
        assert_eq!(
            text_change(Some("  ".into()), 10, "x").unwrap(),
            (true, None)
        );
        assert_eq!(
            text_change(Some(" Wawa ".into()), 10, "x").unwrap(),
            (true, Some("Wawa".into()))
        );
        assert!(text_change(Some("12345678901".into()), 10, "x").is_err());
    }

    #[test]
    fn business_types_are_a_fixed_set() {
        assert_eq!(
            business_type_change(Some("lashes".into())).unwrap(),
            (true, Some("lashes".into()))
        );
        assert_eq!(business_type_change(Some("".into())).unwrap(), (true, None));
        assert_eq!(business_type_change(None).unwrap(), (false, None));
        assert!(business_type_change(Some("barber".into())).is_err());
    }

    #[test]
    fn instagram_handles_are_checked() {
        assert_eq!(
            instagram_change(Some("@anna.lashes_".into())).unwrap(),
            (true, Some("anna.lashes_".into()))
        );
        assert!(instagram_change(Some("bad handle".into())).is_err());
        assert!(instagram_change(Some("@".into())).is_err());
    }
}
