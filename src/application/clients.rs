//! Clients of a business: the people who book. A client belongs to exactly one
//! business (row-level security on `business_id`). A signed-in customer
//! (registered or guest) gets one client row per business they book with;
//! their identity is the user session, never the phone number or e-mail typed
//! into the form.

use crate::application::access::{BusinessAccess, Role};
use crate::application::auth::{clean_optional, normalize_email};
use crate::application::booking::normalize_phone;
use crate::application::schedule::format_utc;
use crate::infrastructure::begin_scoped;
use crate::shared::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

const DEFAULT_LIMIT: i64 = 50;
const MAX_LIMIT: i64 = 100;

#[derive(Debug, Deserialize)]
pub struct ClientQuery {
    /// Part of the name, phone or e-mail.
    pub q: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ClientView {
    pub id: Uuid,
    pub full_name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    /// `staff` (typed in by the business), `guest` or `account`.
    pub source: String,
    pub has_account: bool,
    pub confirmed_appointments: i64,
    pub created_at: String,
}

type ClientRow = (
    Uuid,
    String,
    Option<String>,
    Option<String>,
    String,
    bool,
    i64,
    OffsetDateTime,
);

fn view(row: ClientRow) -> ClientView {
    ClientView {
        id: row.0,
        full_name: row.1,
        phone: row.2,
        email: row.3,
        source: row.4,
        has_account: row.5,
        confirmed_appointments: row.6,
        created_at: format_utc(row.7),
    }
}

const SELECT: &str = "SELECT c.id, c.full_name, c.phone_e164, c.email, c.source,
        c.source = 'account',
        (SELECT count(*) FROM appointment a
          WHERE a.client_id = c.id AND a.business_id = c.business_id AND a.status = 'confirmed'),
        c.created_at
     FROM client c";

/// Contact details a customer typed in, validated.
#[derive(Debug, Clone)]
pub(crate) struct ClientContact {
    pub full_name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
}

pub(crate) fn parse_contact(
    name: &str,
    phone: Option<String>,
    email: Option<String>,
) -> AppResult<ClientContact> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 120 {
        return Err(AppError::validation("client_name must be 1-120 characters"));
    }
    let phone = match clean_optional(phone, 40, "client_phone")? {
        Some(value) => Some(normalize_phone(&value)?),
        None => None,
    };
    let email = match clean_optional(email, 254, "client_email")? {
        Some(value) => Some(normalize_email(&value)?),
        None => None,
    };
    Ok(ClientContact {
        full_name: name.to_string(),
        phone,
        email,
    })
}

/// The customer's client row in one business, created on first use and
/// refreshed with the latest contact details afterwards.
pub(crate) async fn upsert_customer_client(
    conn: &mut PgConnection,
    business_id: Uuid,
    user_id: Uuid,
    contact: &ClientContact,
) -> AppResult<Uuid> {
    Ok(sqlx::query_scalar(
        "INSERT INTO client (business_id, user_id, source, full_name, phone_e164, email)
         SELECT $1::uuid, $2::uuid,
                CASE WHEN u.email IS NULL THEN 'guest' ELSE 'account' END,
                $3::text, $4::text, COALESCE($5::text, u.email)
         FROM users u WHERE u.id = $2::uuid
         ON CONFLICT (business_id, user_id) WHERE user_id IS NOT NULL
         DO UPDATE SET full_name = EXCLUDED.full_name,
                       phone_e164 = COALESCE(EXCLUDED.phone_e164, client.phone_e164),
                       email = COALESCE(EXCLUDED.email, client.email)
         RETURNING id",
    )
    .bind(business_id)
    .bind(user_id)
    .bind(&contact.full_name)
    .bind(&contact.phone)
    .bind(&contact.email)
    .fetch_one(&mut *conn)
    .await?)
}

/// The customer's client row, if they have booked here before.
pub(crate) async fn customer_client_id(
    conn: &mut PgConnection,
    business_id: Uuid,
    user_id: Uuid,
) -> AppResult<Option<Uuid>> {
    Ok(
        sqlx::query_scalar("SELECT id FROM client WHERE business_id = $1 AND user_id = $2")
            .bind(business_id)
            .bind(user_id)
            .fetch_optional(&mut *conn)
            .await?,
    )
}

#[derive(Clone)]
pub struct ClientService {
    pool: PgPool,
}

impl ClientService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Owner, manager and reception see the business's clients.
    pub async fn list(
        &self,
        access: BusinessAccess,
        query: ClientQuery,
    ) -> AppResult<Vec<ClientView>> {
        access.require(&[Role::Owner, Role::Manager, Role::Reception])?;
        let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
        let offset = query.offset.unwrap_or(0).max(0);
        let needle = clean_optional(query.q, 100, "q")?;
        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        let rows = sqlx::query_as::<_, ClientRow>(&format!(
            "{SELECT}
             WHERE c.business_id = $1
               AND ($2::text IS NULL
                    OR position(lower($2) IN lower(c.full_name)) > 0
                    OR position($2 IN COALESCE(c.phone_e164, '')) > 0
                    OR position(lower($2) IN COALESCE(c.email, '')) > 0)
             ORDER BY lower(c.full_name), c.id
             LIMIT $3 OFFSET $4"
        ))
        .bind(access.business_id.as_uuid())
        .bind(needle)
        .bind(limit)
        .bind(offset)
        .fetch_all(&mut *tx)
        .await?;
        Ok(rows.into_iter().map(view).collect())
    }

    pub async fn get(&self, access: BusinessAccess, id: Uuid) -> AppResult<ClientView> {
        access.require(&[Role::Owner, Role::Manager, Role::Reception])?;
        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        sqlx::query_as::<_, ClientRow>(&format!("{SELECT} WHERE c.business_id = $1 AND c.id = $2"))
            .bind(access.business_id.as_uuid())
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .map(view)
            .ok_or_else(|| AppError::not_found("Client not found"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contacts_are_trimmed_and_normalised() {
        let contact = parse_contact(
            "  Ewa Nowak ",
            Some("+48 600-100-200".into()),
            Some(" Ewa@Example.PL ".into()),
        )
        .unwrap();
        assert_eq!(contact.full_name, "Ewa Nowak");
        assert_eq!(contact.phone.as_deref(), Some("+48600100200"));
        assert_eq!(contact.email.as_deref(), Some("ewa@example.pl"));

        let bare = parse_contact("Ewa", Some("  ".into()), None).unwrap();
        assert!(bare.phone.is_none() && bare.email.is_none());
    }

    #[test]
    fn bad_contacts_are_rejected() {
        assert!(parse_contact("   ", None, None).is_err());
        assert!(parse_contact(&"x".repeat(121), None, None).is_err());
        assert!(parse_contact("Ewa", Some("600100200".into()), None).is_err());
        assert!(parse_contact("Ewa", None, Some("not-an-email".into())).is_err());
    }
}
