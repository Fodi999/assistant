//! Clients of a business: the people who book. A client belongs to exactly one
//! business (row-level security on `business_id`). A signed-in customer
//! (registered or guest) gets one client row per business they book with;
//! their identity is the user session, never the phone number or e-mail typed
//! into the form.

use crate::application::access::{BusinessAccess, Role};
use crate::application::auth::{clean_optional, normalize_email};
use crate::application::booking::{normalize_phone, own_staff_id};
use crate::application::schedule::format_utc;
use crate::infrastructure::begin_scoped;
use crate::shared::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{PgConnection, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

const DEFAULT_LIMIT: i64 = 50;
const MAX_LIMIT: i64 = 100;
const NOTE_MAX: usize = 1000;

#[derive(Debug, Deserialize)]
pub struct ClientQuery {
    /// Part of the name, phone or e-mail.
    pub q: Option<String>,
    /// `name` (default) or `recent` (latest completed visit first).
    pub sort: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// A client card. Numbers come from the visits the caller may see: an employee
/// gets only the ones with their own appointments and never the note.
#[derive(Debug, Serialize)]
pub struct ClientView {
    pub id: Uuid,
    pub full_name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    /// The business's private note. Absent for an employee.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// `staff` (typed in by the business), `guest` or `account`.
    pub source: String,
    pub has_account: bool,
    /// Confirmed visits (upcoming, or not yet closed).
    pub confirmed_appointments: i64,
    pub completed_count: i64,
    pub no_show_count: i64,
    /// Start of the latest completed visit.
    pub last_visit_at: Option<String>,
    /// Sum of the booked prices (snapshots) of completed visits only.
    pub total_spent_minor: i64,
    /// The business currency.
    pub total_spent_currency: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateClientInput {
    pub full_name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub note: Option<String>,
}

/// Absent fields stay as they are; an empty `phone`, `email` or `note` clears it.
/// Name, phone and e-mail belong to the customer once the client has used the
/// app (`guest`/`account`), so for those only the note can be changed.
#[derive(Debug, Deserialize)]
pub struct UpdateClientInput {
    pub full_name: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub note: Option<String>,
}

type ClientRow = (
    Uuid,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    bool,
    i64,
    i64,
    i64,
    Option<OffsetDateTime>,
    i64,
    String,
    OffsetDateTime,
);

fn view(row: ClientRow) -> ClientView {
    ClientView {
        id: row.0,
        full_name: row.1,
        phone: row.2,
        email: row.3,
        note: row.4,
        source: row.5,
        has_account: row.6,
        confirmed_appointments: row.7,
        completed_count: row.8,
        no_show_count: row.9,
        last_visit_at: row.10.map(format_utc),
        total_spent_minor: row.11,
        total_spent_currency: row.12,
        created_at: format_utc(row.13),
    }
}

/// $1 business, $2 restrict to one master, $3 that master, $4 note visible.
/// The visit numbers are computed inside the same restriction as the list.
const SELECT: &str = "SELECT c.id, c.full_name, c.phone_e164, c.email,
        CASE WHEN $4::bool THEN c.note END, c.source, c.source = 'account',
        s.confirmed, s.completed, s.no_show, s.last_visit, s.spent,
        b.currency::text, c.created_at
     FROM client c
     JOIN business b ON b.id = c.business_id
     CROSS JOIN LATERAL (
        SELECT count(*) FILTER (WHERE a.status = 'confirmed') AS confirmed,
               count(*) FILTER (WHERE a.status = 'completed') AS completed,
               count(*) FILTER (WHERE a.status = 'no_show') AS no_show,
               max(a.start_at) FILTER (WHERE a.status = 'completed') AS last_visit,
               COALESCE(sum(p.price) FILTER (WHERE a.status = 'completed'), 0)::bigint AS spent
        FROM appointment a
        LEFT JOIN LATERAL (
            SELECT sum(i.price_minor) AS price FROM appointment_item i
            WHERE i.appointment_id = a.id AND i.business_id = a.business_id
        ) p ON true
        WHERE a.client_id = c.id AND a.business_id = c.business_id
          AND (NOT $2::bool OR a.staff_id = $3::uuid)
     ) s";

/// An employee sees only the clients of their own appointments.
const VISIBLE: &str = "c.business_id = $1
     AND (NOT $2::bool OR EXISTS (
            SELECT 1 FROM appointment v
            WHERE v.client_id = c.id AND v.business_id = c.business_id
              AND v.staff_id = $3::uuid))";

/// What the caller may see of the clients.
pub(crate) struct Visibility {
    /// Employee: only clients (and visits) of `staff`.
    pub restrict: bool,
    pub staff: Option<Uuid>,
    pub note_visible: bool,
}

pub(crate) async fn visibility(
    conn: &mut PgConnection,
    access: &BusinessAccess,
) -> AppResult<Visibility> {
    Ok(match access.role {
        Role::Owner | Role::Manager | Role::Reception => Visibility {
            restrict: false,
            staff: None,
            note_visible: true,
        },
        Role::Employee => Visibility {
            restrict: true,
            staff: own_staff_id(conn, access).await?,
            note_visible: false,
        },
    })
}

fn client_not_found() -> AppError {
    AppError::not_found("Client not found")
}

/// Business-side editing: everyone but an employee.
fn require_editor(access: &BusinessAccess) -> AppResult<()> {
    access.require(&[Role::Owner, Role::Manager, Role::Reception])
}

async fn audit(
    conn: &mut PgConnection,
    access: &BusinessAccess,
    action: &str,
    client_id: Uuid,
    meta: serde_json::Value,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO audit_log (business_id, actor_user_id, action, entity, entity_id, meta)
         VALUES ($1, $2, $3, 'client', $4, $5::text::jsonb)",
    )
    .bind(access.business_id.as_uuid())
    .bind(access.user_id.as_uuid())
    .bind(action)
    .bind(client_id)
    .bind(meta.to_string())
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// The staff card of `phone` (E.164) in this business, if any.
async fn staff_card_by_phone(
    conn: &mut PgConnection,
    business_id: Uuid,
    phone: &str,
) -> AppResult<Option<Uuid>> {
    Ok(sqlx::query_scalar(
        "SELECT id FROM client
         WHERE business_id = $1 AND source = 'staff' AND phone_e164 = $2",
    )
    .bind(business_id)
    .bind(phone)
    .fetch_optional(&mut *conn)
    .await?)
}

/// A manual booking with a phone: the business's card for that number, made on
/// the spot when there is none. Only staff cards are matched: the phone of an
/// app customer is unverified, so a visit typed in by hand never lands in a
/// customer's own history. The card keeps its name; the visit keeps the name
/// that was typed. Returns the card id.
pub(crate) async fn find_or_create_staff_client(
    conn: &mut PgConnection,
    access: &BusinessAccess,
    name: &str,
    phone: &str,
) -> AppResult<Uuid> {
    let business_id = *access.business_id.as_uuid();
    if let Some(id) = staff_card_by_phone(conn, business_id, phone).await? {
        return Ok(id);
    }
    let inserted: Option<Uuid> = sqlx::query_scalar(
        "INSERT INTO client (business_id, source, full_name, phone_e164)
         VALUES ($1, 'staff', $2, $3)
         ON CONFLICT (business_id, phone_e164)
             WHERE source = 'staff' AND phone_e164 IS NOT NULL
         DO NOTHING
         RETURNING id",
    )
    .bind(business_id)
    .bind(name)
    .bind(phone)
    .fetch_optional(&mut *conn)
    .await?;
    if let Some(id) = inserted {
        audit(
            conn,
            access,
            "client.create",
            id,
            json!({ "via": "booking" }),
        )
        .await?;
        return Ok(id);
    }
    // Lost a race with a booking that committed first: its card is there now.
    staff_card_by_phone(conn, business_id, phone)
        .await?
        .ok_or_else(|| AppError::internal("client card vanished after a conflict"))
}

/// A client the caller may see (and so book for): name and phone from the card. Not visible
/// (or not in this business) is a 404.
pub(crate) async fn visible_client(
    conn: &mut PgConnection,
    access: &BusinessAccess,
    client_id: Uuid,
) -> AppResult<(String, Option<String>)> {
    let vis = visibility(conn, access).await?;
    sqlx::query_as::<_, (String, Option<String>)>(&format!(
        "SELECT c.full_name, c.phone_e164 FROM client c WHERE {VISIBLE} AND c.id = $4"
    ))
    .bind(access.business_id.as_uuid())
    .bind(vis.restrict)
    .bind(vis.staff)
    .bind(client_id)
    .fetch_optional(&mut *conn)
    .await?
    .ok_or_else(client_not_found)
}

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

    /// Owner, manager and reception see all the business's clients; an employee
    /// only those with their own appointments.
    pub async fn list(
        &self,
        access: BusinessAccess,
        query: ClientQuery,
    ) -> AppResult<Vec<ClientView>> {
        let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
        let offset = query.offset.unwrap_or(0).max(0);
        let needle = clean_optional(query.q, 100, "q")?;
        let order = match query.sort.as_deref() {
            None | Some("name") => "lower(c.full_name), c.id",
            Some("recent") => "s.last_visit DESC NULLS LAST, lower(c.full_name), c.id",
            Some(_) => return Err(AppError::validation("sort must be name or recent")),
        };
        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        let vis = visibility(&mut tx, &access).await?;
        let rows = sqlx::query_as::<_, ClientRow>(&format!(
            "{SELECT}
             WHERE {VISIBLE}
               AND ($5::text IS NULL
                    OR position(lower($5) IN lower(c.full_name)) > 0
                    OR position($5 IN COALESCE(c.phone_e164, '')) > 0
                    OR position(lower($5) IN COALESCE(c.email, '')) > 0)
             ORDER BY {order}
             LIMIT $6 OFFSET $7"
        ))
        .bind(access.business_id.as_uuid())
        .bind(vis.restrict)
        .bind(vis.staff)
        .bind(vis.note_visible)
        .bind(needle)
        .bind(limit)
        .bind(offset)
        .fetch_all(&mut *tx)
        .await?;
        Ok(rows.into_iter().map(view).collect())
    }

    pub async fn get(&self, access: BusinessAccess, id: Uuid) -> AppResult<ClientView> {
        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        let vis = visibility(&mut tx, &access).await?;
        fetch_view(&mut tx, &access, &vis, id).await
    }

    /// A card without a visit: owner, manager or reception. The phone is optional;
    /// when given it must be new among the business's staff cards.
    pub async fn create(
        &self,
        access: BusinessAccess,
        input: CreateClientInput,
    ) -> AppResult<ClientView> {
        require_editor(&access)?;
        let contact = parse_contact(&input.full_name, input.phone, input.email)?;
        let note = clean_optional(input.note, NOTE_MAX, "note")?;
        let business_id = *access.business_id.as_uuid();
        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        if let Some(phone) = &contact.phone {
            if let Some(existing) = staff_card_by_phone(&mut tx, business_id, phone).await? {
                return Err(AppError::DuplicateClient(existing));
            }
        }
        let inserted = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO client (business_id, source, full_name, phone_e164, email, note)
             VALUES ($1, 'staff', $2, $3, $4, $5)
             RETURNING id",
        )
        .bind(business_id)
        .bind(&contact.full_name)
        .bind(&contact.phone)
        .bind(&contact.email)
        .bind(&note)
        .fetch_one(&mut *tx)
        .await;
        let id = match inserted {
            Ok(id) => id,
            Err(error) if is_unique_violation(&error) => {
                // A parallel request took the number between our check and the insert.
                drop(tx);
                return self.duplicate_after_race(&access, &contact).await;
            }
            Err(error) => return Err(error.into()),
        };
        audit(
            &mut tx,
            &access,
            "client.create",
            id,
            json!({ "via": "manual" }),
        )
        .await?;
        let vis = visibility(&mut tx, &access).await?;
        let created = fetch_view(&mut tx, &access, &vis, id).await?;
        tx.commit().await?;
        Ok(created)
    }

    async fn duplicate_after_race(
        &self,
        access: &BusinessAccess,
        contact: &ClientContact,
    ) -> AppResult<ClientView> {
        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        let phone = contact.phone.as_deref().unwrap_or_default();
        match staff_card_by_phone(&mut tx, *access.business_id.as_uuid(), phone).await? {
            Some(existing) => Err(AppError::DuplicateClient(existing)),
            None => Err(AppError::conflict("Please try again")),
        }
    }

    pub async fn update(
        &self,
        access: BusinessAccess,
        id: Uuid,
        input: UpdateClientInput,
    ) -> AppResult<ClientView> {
        require_editor(&access)?;
        if input.full_name.is_none()
            && input.phone.is_none()
            && input.email.is_none()
            && input.note.is_none()
        {
            return Err(AppError::validation("Nothing to change"));
        }
        let business_id = *access.business_id.as_uuid();
        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        let (source, current_name): (String, String) = sqlx::query_as(
            "SELECT source, full_name FROM client
             WHERE id = $1 AND business_id = $2 FOR UPDATE",
        )
        .bind(id)
        .bind(business_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(client_not_found)?;

        let touches_contact =
            input.full_name.is_some() || input.phone.is_some() || input.email.is_some();
        if touches_contact && source != "staff" {
            return Err(AppError::conflict(
                "This client keeps their own contact details; only the note can be changed",
            ));
        }

        let mut changed: Vec<&str> = Vec::new();
        let name = match input.full_name {
            Some(value) => {
                let value = value.trim().to_string();
                if value.is_empty() || value.chars().count() > 120 {
                    return Err(AppError::validation("full_name must be 1-120 characters"));
                }
                if value != current_name {
                    changed.push("full_name");
                }
                Some(value)
            }
            None => None,
        };
        let phone_set = input.phone.is_some();
        let phone = match clean_optional(input.phone, 40, "phone")? {
            Some(value) => Some(normalize_phone(&value)?),
            None => None,
        };
        if phone_set {
            changed.push("phone");
            if let Some(phone) = &phone {
                if let Some(other) = staff_card_by_phone(&mut tx, business_id, phone).await? {
                    if other != id {
                        return Err(AppError::DuplicateClient(other));
                    }
                }
            }
        }
        let email_set = input.email.is_some();
        let email = match clean_optional(input.email, 254, "email")? {
            Some(value) => Some(normalize_email(&value)?),
            None => None,
        };
        if email_set {
            changed.push("email");
        }
        let note_set = input.note.is_some();
        let note = clean_optional(input.note, NOTE_MAX, "note")?;
        if note_set {
            changed.push("note");
        }

        let updated = sqlx::query(
            "UPDATE client
             SET full_name  = COALESCE($3, full_name),
                 phone_e164 = CASE WHEN $4 THEN $5 ELSE phone_e164 END,
                 email      = CASE WHEN $6 THEN $7 ELSE email END,
                 note       = CASE WHEN $8 THEN $9 ELSE note END
             WHERE id = $1 AND business_id = $2",
        )
        .bind(id)
        .bind(business_id)
        .bind(&name)
        .bind(phone_set)
        .bind(&phone)
        .bind(email_set)
        .bind(&email)
        .bind(note_set)
        .bind(&note)
        .execute(&mut *tx)
        .await;
        match updated {
            Ok(_) => {}
            Err(error) if is_unique_violation(&error) => {
                drop(tx);
                let contact = ClientContact {
                    full_name: String::new(),
                    phone,
                    email: None,
                };
                return self.duplicate_after_race(&access, &contact).await;
            }
            Err(error) => return Err(error.into()),
        }
        // Field names only: phones and notes never go into the log.
        audit(
            &mut tx,
            &access,
            "client.update",
            id,
            json!({ "fields": changed }),
        )
        .await?;
        let vis = visibility(&mut tx, &access).await?;
        let view = fetch_view(&mut tx, &access, &vis, id).await?;
        tx.commit().await?;
        Ok(view)
    }
}

async fn fetch_view(
    conn: &mut PgConnection,
    access: &BusinessAccess,
    vis: &Visibility,
    id: Uuid,
) -> AppResult<ClientView> {
    sqlx::query_as::<_, ClientRow>(&format!("{SELECT} WHERE {VISIBLE} AND c.id = $5"))
        .bind(access.business_id.as_uuid())
        .bind(vis.restrict)
        .bind(vis.staff)
        .bind(vis.note_visible)
        .bind(id)
        .fetch_optional(&mut *conn)
        .await?
        .map(view)
        .ok_or_else(client_not_found)
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .and_then(|db| db.code())
        .map(|code| code == "23505")
        .unwrap_or(false)
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
