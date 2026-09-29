//! Booking: slot holds, confirmed appointments, cancellation and rescheduling.
//!
//! An appointment is one row that blocks its master's calendar over
//! `[start_at, blocked_end)` (the service plus its buffer) while it is `held`
//! or `confirmed`; PostgreSQL refuses overlaps (migration `appointments`). A
//! hold is a short reservation (`hold_expires_at`); confirming it, or booking
//! directly, makes it `confirmed`. Every requested start must be a time the
//! server itself offers (`availability`), re-checked in the transaction that
//! writes the row. Rescheduling moves the same row and keeps the history in
//! `appointment_event`.
//!
//! Who acts (PRODUCT_SPEC §17.1): owner, manager and reception on any master's
//! calendar; an employee only on their own.
//!
//! Cancellation policy (no money yet): cancelling less than
//! [`FREE_CANCELLATION_HOURS`] before the start is recorded as a late
//! cancellation. Deposits and fees come with the payments stage.

use crate::application::access::{BusinessAccess, Role};
use crate::application::auth::clean_optional;
use crate::application::availability::{eligible_staff, load_service_rules, offered_slots};
use crate::application::schedule::{format_utc, parse_date, parse_instant};
use crate::infrastructure::begin_scoped;
use crate::shared::{AppError, AppResult, Clock};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{PgConnection, PgPool};
use time::{Date, Duration, OffsetDateTime};
use uuid::Uuid;

/// How long a slot is held for a client who is still choosing.
pub const HOLD_TTL_MINUTES: i64 = 10;
/// Most live holds one user may keep at a time (guards against slot hoarding).
const MAX_ACTIVE_HOLDS: i64 = 10;
/// Cancelling later than this before the start is a "late" cancellation.
pub const FREE_CANCELLATION_HOURS: i64 = 24;
/// Longest date range of the appointment list.
const MAX_LIST_DAYS: i64 = 31;

// ---------------------------------------------------------------------------
// Input
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct CreateHoldInput {
    pub service_id: Uuid,
    pub variant_id: Uuid,
    pub staff_id: Uuid,
    /// RFC 3339 instant of a start time returned by `availability`.
    pub start_at: String,
    /// `manual` (default: a member books for a client), `app` or `web`
    /// (online booking rules apply).
    pub source: Option<String>,
}

/// Books an appointment. Either confirm an existing hold (`hold_id`) or book
/// directly (`service_id`, `variant_id`, `staff_id`, `start_at`); not both.
#[derive(Debug, Deserialize)]
pub struct CreateAppointmentInput {
    pub hold_id: Option<Uuid>,
    pub service_id: Option<Uuid>,
    pub variant_id: Option<Uuid>,
    pub staff_id: Option<Uuid>,
    pub start_at: Option<String>,
    pub source: Option<String>,
    pub client_name: String,
    /// International format, e.g. `+48 600 100 200`; stored as `+48600100200`.
    pub client_phone: Option<String>,
    /// Internal note, up to 500 characters. Not for health information.
    pub note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CancelInput {
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RescheduleInput {
    /// New start, RFC 3339; must be offered by `availability`.
    pub start_at: String,
    /// Move to another master who performs the service.
    pub staff_id: Option<Uuid>,
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AppointmentQuery {
    /// First local date (business time zone), `YYYY-MM-DD`.
    pub from: String,
    /// Last local date, inclusive; defaults to `from`. At most 31 days.
    pub to: Option<String>,
    pub staff_id: Option<Uuid>,
    /// `confirmed` (default), `held`, `cancelled`, `completed` or `no_show`.
    pub status: Option<String>,
}

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct AppointmentView {
    pub id: Uuid,
    /// `held` (or `expired` once time is up / released), `confirmed`,
    /// `cancelled`, `completed`, `no_show`.
    pub status: String,
    pub staff_id: Uuid,
    pub service_id: Uuid,
    pub variant_id: Uuid,
    pub start_at: String,
    pub end_at: String,
    pub hold_expires_at: Option<String>,
    pub source: String,
    pub client_name: Option<String>,
    pub client_phone: Option<String>,
    pub note: Option<String>,
    /// Price at booking time, minor units.
    pub price_minor: i64,
    pub currency: String,
    pub duration_min: i32,
    pub confirmed_at: Option<String>,
    pub cancelled_at: Option<String>,
    pub cancel_reason: Option<String>,
    pub late_cancellation: Option<bool>,
    pub version: i32,
}

#[derive(Debug, Serialize)]
pub struct EventView {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub kind: String,
    pub actor_user_id: Option<Uuid>,
    pub data: Value,
    pub created_at: String,
}

/// Result of a booking request: `created` is false when the same idempotency
/// key was already used for the same request and the existing row is returned.
pub struct BookingOutcome {
    pub appointment: AppointmentView,
    pub created: bool,
}

#[derive(sqlx::FromRow)]
struct AppointmentRow {
    id: Uuid,
    status: String,
    staff_id: Uuid,
    service_id: Uuid,
    variant_id: Uuid,
    start_at: OffsetDateTime,
    end_at: OffsetDateTime,
    hold_expires_at: Option<OffsetDateTime>,
    source: String,
    request_fingerprint: Option<String>,
    client_name: Option<String>,
    client_phone: Option<String>,
    note: Option<String>,
    price_minor: i64,
    currency: String,
    duration_min: i32,
    confirmed_at: Option<OffsetDateTime>,
    cancelled_at: Option<OffsetDateTime>,
    cancel_reason: Option<String>,
    cancel_late: Option<bool>,
    version: i32,
}

const COLUMNS: &str =
    "a.id, a.status, a.staff_id, i.service_id, i.variant_id, a.start_at, a.end_at,
     a.hold_expires_at, a.source, a.request_fingerprint, a.client_name, a.client_phone, a.note,
     i.price_minor, i.currency::text AS currency, i.duration_min, a.confirmed_at, a.cancelled_at,
     a.cancel_reason, a.cancel_late, a.version";

const FROM: &str = "FROM appointment a JOIN appointment_item i ON i.appointment_id = a.id";

impl AppointmentRow {
    /// A hold whose time is up reads as `expired` even before a booking attempt
    /// has marked the row.
    fn effective_status(&self, now: OffsetDateTime) -> String {
        let over = self.status == "held"
            && self
                .hold_expires_at
                .map(|expires| expires <= now)
                .unwrap_or(true);
        if over {
            "expired".to_string()
        } else {
            self.status.clone()
        }
    }

    fn into_view(self, now: OffsetDateTime) -> AppointmentView {
        AppointmentView {
            status: self.effective_status(now),
            id: self.id,
            staff_id: self.staff_id,
            service_id: self.service_id,
            variant_id: self.variant_id,
            start_at: format_utc(self.start_at),
            end_at: format_utc(self.end_at),
            hold_expires_at: self.hold_expires_at.map(format_utc),
            source: self.source,
            client_name: self.client_name,
            client_phone: self.client_phone,
            note: self.note,
            price_minor: self.price_minor,
            currency: self.currency,
            duration_min: self.duration_min,
            confirmed_at: self.confirmed_at.map(format_utc),
            cancelled_at: self.cancelled_at.map(format_utc),
            cancel_reason: self.cancel_reason,
            late_cancellation: self.cancel_late,
            version: self.version,
        }
    }
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct BookingService {
    pool: PgPool,
    clock: Clock,
}

#[derive(Debug, Clone)]
struct Client {
    name: String,
    phone: Option<String>,
    note: Option<String>,
}

/// One request to occupy a slot, as a hold (`client` is None) or directly as a
/// confirmed appointment.
struct Placement {
    key: String,
    service_id: Uuid,
    variant_id: Uuid,
    staff_id: Uuid,
    start: OffsetDateTime,
    source: String,
    client: Option<Client>,
    /// The customer this booking belongs to (None: staff booked it by hand).
    client_id: Option<Uuid>,
    fingerprint: String,
}

impl BookingService {
    pub fn new(pool: PgPool, clock: Clock) -> Self {
        Self { pool, clock }
    }

    // -- holds --------------------------------------------------------------

    pub async fn create_hold(
        &self,
        access: BusinessAccess,
        idempotency_key: &str,
        input: CreateHoldInput,
    ) -> AppResult<BookingOutcome> {
        let key = valid_key(idempotency_key)?;
        let start = parse_instant(&input.start_at, "start_at")?;
        let source = valid_source(input.source.as_deref())?;
        let fingerprint = format!(
            "hold|{}|{}|{}|{}|{}",
            input.service_id,
            input.variant_id,
            input.staff_id,
            start.unix_timestamp(),
            source
        );
        self.place(
            access,
            Placement {
                key,
                service_id: input.service_id,
                variant_id: input.variant_id,
                staff_id: input.staff_id,
                start,
                source,
                client: None,
                client_id: None,
                fingerprint,
            },
        )
        .await
    }

    /// Frees a held slot. Releasing an already expired hold is fine.
    pub async fn release_hold(&self, access: BusinessAccess, id: Uuid) -> AppResult<()> {
        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        let row = fetch(&mut tx, access, id, true).await?;
        authorize_staff(&mut tx, &access, row.staff_id).await?;
        match row.status.as_str() {
            "held" => {
                sqlx::query(
                    "UPDATE appointment SET status = 'expired' WHERE id = $1 AND business_id = $2",
                )
                .bind(id)
                .bind(access.business_id.as_uuid())
                .execute(&mut *tx)
                .await?;
                add_event(&mut tx, access, id, "hold_released", json!({})).await?;
            }
            "expired" => {}
            _ => return Err(AppError::conflict("This appointment is not a hold")),
        }
        tx.commit().await?;
        Ok(())
    }

    // -- appointments -------------------------------------------------------

    pub async fn create_appointment(
        &self,
        access: BusinessAccess,
        idempotency_key: Option<&str>,
        input: CreateAppointmentInput,
    ) -> AppResult<BookingOutcome> {
        self.create_appointment_for(access, None, idempotency_key, input)
            .await
    }

    /// Same, for a known client (a customer booking through the public API).
    pub async fn create_appointment_for(
        &self,
        access: BusinessAccess,
        client_id: Option<Uuid>,
        idempotency_key: Option<&str>,
        input: CreateAppointmentInput,
    ) -> AppResult<BookingOutcome> {
        let client = parse_client(&input.client_name, input.client_phone, input.note)?;
        if let Some(hold_id) = input.hold_id {
            if input.service_id.is_some()
                || input.variant_id.is_some()
                || input.staff_id.is_some()
                || input.start_at.is_some()
                || input.source.is_some()
            {
                return Err(AppError::validation(
                    "hold_id cannot be combined with service, staff, start or source",
                ));
            }
            return self.confirm_hold(access, hold_id, client, client_id).await;
        }

        let (Some(service_id), Some(variant_id), Some(staff_id), Some(start_at)) = (
            input.service_id,
            input.variant_id,
            input.staff_id,
            input.start_at.as_deref(),
        ) else {
            return Err(AppError::validation(
                "Send hold_id, or service_id, variant_id, staff_id and start_at",
            ));
        };
        let key = valid_key(idempotency_key.unwrap_or(""))?;
        let start = parse_instant(start_at, "start_at")?;
        let source = valid_source(input.source.as_deref())?;
        let fingerprint = format!(
            "book|{service_id}|{variant_id}|{staff_id}|{}|{source}|{}|{}|{}",
            start.unix_timestamp(),
            client.name,
            client.phone.as_deref().unwrap_or(""),
            client.note.as_deref().unwrap_or("")
        );
        self.place(
            access,
            Placement {
                key,
                service_id,
                variant_id,
                staff_id,
                start,
                source,
                client: Some(client),
                client_id,
                fingerprint,
            },
        )
        .await
    }

    pub async fn get_appointment(
        &self,
        access: BusinessAccess,
        id: Uuid,
    ) -> AppResult<AppointmentView> {
        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        let row = fetch(&mut tx, access, id, false).await?;
        authorize_staff(&mut tx, &access, row.staff_id).await?;
        Ok(row.into_view(self.clock.now()))
    }

    /// Appointments starting on the local dates `from..=to`, oldest first. An
    /// employee sees only their own calendar.
    pub async fn list_appointments(
        &self,
        access: BusinessAccess,
        query: AppointmentQuery,
    ) -> AppResult<Vec<AppointmentView>> {
        let from = parse_date(&query.from, "from")?;
        let to = match query.to.as_deref() {
            Some(value) => parse_date(value, "to")?,
            None => from,
        };
        if to < from {
            return Err(AppError::validation("to must not be before from"));
        }
        if (to - from).whole_days() + 1 > MAX_LIST_DAYS {
            return Err(AppError::validation(format!(
                "At most {MAX_LIST_DAYS} days per request"
            )));
        }
        let status = query.status.as_deref().unwrap_or("confirmed");
        if !["confirmed", "held", "cancelled", "completed", "no_show"].contains(&status) {
            return Err(AppError::validation(
                "status must be confirmed, held, cancelled, completed or no_show",
            ));
        }

        let now = self.clock.now();
        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        let mut staff_filter = query.staff_id;
        if access.role == Role::Employee {
            let own = own_staff_id(&mut tx, &access).await?;
            match (query.staff_id, own) {
                (Some(wanted), Some(own)) if wanted == own => {}
                (None, Some(own)) => staff_filter = Some(own),
                _ => {
                    return Err(AppError::authorization(
                        "Employees can list only their own calendar",
                    ))
                }
            }
        }
        let (range_start, range_end): (OffsetDateTime, OffsetDateTime) = sqlx::query_as(
            "SELECT ($2::date)::timestamp AT TIME ZONE timezone,
                    (($3::date) + 1)::timestamp AT TIME ZONE timezone
             FROM business WHERE id = $1",
        )
        .bind(access.business_id.as_uuid())
        .bind(from)
        .bind(to)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::not_found("Business not found"))?;

        let rows = sqlx::query_as::<_, AppointmentRow>(&format!(
            "SELECT {COLUMNS} {FROM}
             WHERE a.business_id = $1 AND a.start_at >= $2 AND a.start_at < $3
               AND ($4::uuid IS NULL OR a.staff_id = $4)
               AND a.status = $5
               AND (a.status <> 'held' OR a.hold_expires_at > $6)
             ORDER BY a.start_at, a.id"
        ))
        .bind(access.business_id.as_uuid())
        .bind(range_start)
        .bind(range_end)
        .bind(staff_filter)
        .bind(status)
        .bind(now)
        .fetch_all(&mut *tx)
        .await?;
        Ok(rows.into_iter().map(|row| row.into_view(now)).collect())
    }

    pub async fn history(&self, access: BusinessAccess, id: Uuid) -> AppResult<Vec<EventView>> {
        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        let row = fetch(&mut tx, access, id, false).await?;
        authorize_staff(&mut tx, &access, row.staff_id).await?;
        let rows: Vec<(Uuid, String, Option<Uuid>, String, OffsetDateTime)> = sqlx::query_as(
            "SELECT id, type, actor_user_id, data::text, created_at
             FROM appointment_event
             WHERE appointment_id = $1 AND business_id = $2
             ORDER BY created_at, id",
        )
        .bind(id)
        .bind(access.business_id.as_uuid())
        .fetch_all(&mut *tx)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(id, kind, actor_user_id, data, created_at)| EventView {
                id,
                kind,
                actor_user_id,
                data: serde_json::from_str(&data).unwrap_or(Value::Null),
                created_at: format_utc(created_at),
            })
            .collect())
    }

    /// Cancels a confirmed appointment and frees its slot. Cancelling again
    /// returns the cancelled appointment.
    pub async fn cancel(
        &self,
        access: BusinessAccess,
        id: Uuid,
        input: CancelInput,
    ) -> AppResult<AppointmentView> {
        let reason = clean_optional(input.reason, 500, "reason")?;
        let now = self.clock.now();
        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        let row = fetch(&mut tx, access, id, true).await?;
        authorize_staff(&mut tx, &access, row.staff_id).await?;
        match row.status.as_str() {
            "cancelled" => return Ok(row.into_view(now)),
            "confirmed" => {}
            "held" | "expired" => {
                return Err(AppError::conflict("A hold is released, not cancelled"));
            }
            _ => {
                return Err(AppError::conflict(
                    "This appointment can no longer be cancelled",
                ));
            }
        }
        if row.start_at <= now {
            return Err(AppError::conflict("The appointment has already started"));
        }
        let late = row.start_at - now < Duration::hours(FREE_CANCELLATION_HOURS);
        sqlx::query(
            "UPDATE appointment
             SET status = 'cancelled', cancelled_at = $3, cancelled_by_user_id = $4,
                 cancel_reason = $5, cancel_late = $6
             WHERE id = $1 AND business_id = $2",
        )
        .bind(id)
        .bind(access.business_id.as_uuid())
        .bind(now)
        .bind(access.user_id.as_uuid())
        .bind(&reason)
        .bind(late)
        .execute(&mut *tx)
        .await?;
        add_event(
            &mut tx,
            access,
            id,
            "cancelled",
            json!({ "late": late, "reason": reason }),
        )
        .await?;
        let row = fetch(&mut tx, access, id, false).await?;
        tx.commit().await?;
        Ok(row.into_view(now))
    }

    /// Moves the same appointment to another start (and optionally another
    /// master). The old time is free at once; the move is one row update, so
    /// two moves to one slot cannot both succeed.
    pub async fn reschedule(
        &self,
        access: BusinessAccess,
        id: Uuid,
        input: RescheduleInput,
    ) -> AppResult<AppointmentView> {
        self.reschedule_with(access, id, input, false).await
    }

    /// Same, under the online booking rules (a customer moving their own visit).
    pub async fn reschedule_online(
        &self,
        access: BusinessAccess,
        id: Uuid,
        input: RescheduleInput,
    ) -> AppResult<AppointmentView> {
        self.reschedule_with(access, id, input, true).await
    }

    async fn reschedule_with(
        &self,
        access: BusinessAccess,
        id: Uuid,
        input: RescheduleInput,
        online: bool,
    ) -> AppResult<AppointmentView> {
        let start = parse_instant(&input.start_at, "start_at")?;
        let reason = clean_optional(input.reason, 500, "reason")?;
        let now = self.clock.now();
        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        let row = fetch(&mut tx, access, id, true).await?;
        authorize_staff(&mut tx, &access, row.staff_id).await?;
        if row.status != "confirmed" {
            return Err(AppError::conflict(
                "Only a confirmed appointment can be moved",
            ));
        }
        if row.start_at <= now {
            return Err(AppError::conflict("The appointment has already started"));
        }
        let staff_id = input.staff_id.unwrap_or(row.staff_id);
        if staff_id == row.staff_id && start == row.start_at {
            return Err(AppError::validation("Nothing to change"));
        }
        if staff_id != row.staff_id {
            authorize_staff(&mut tx, &access, staff_id).await?;
        }

        // Same service and length as booked; the buffer follows the service now.
        let mut rules =
            load_service_rules(&mut tx, access, row.service_id, row.variant_id, online).await?;
        rules.duration_min = row.duration_min;
        let staff = eligible_staff(&mut tx, access, row.service_id).await?;
        if !staff.contains(&staff_id) {
            return Err(AppError::not_found(
                "This staff member does not perform the service",
            ));
        }
        let end = start + Duration::minutes(rules.duration_min.into());
        let blocked_end = end + Duration::minutes(rules.buffer_after_min.into());
        expire_stale_holds(&mut tx, access, staff_id, start, blocked_end, now).await?;
        let day = local_date(&mut tx, access, start).await?;
        // The appointment's own current time does not count as busy.
        let offered = offered_slots(
            &mut tx,
            access,
            &rules,
            &[staff_id],
            day,
            day,
            now,
            Some(id),
        )
        .await?;
        if !offered.iter().any(|slot| slot.start == start) {
            return Err(unavailable());
        }

        let moved = sqlx::query(
            "UPDATE appointment
             SET staff_id = $3, start_at = $4, end_at = $5, blocked_end = $6
             WHERE id = $1 AND business_id = $2 AND status = 'confirmed'",
        )
        .bind(id)
        .bind(access.business_id.as_uuid())
        .bind(staff_id)
        .bind(start)
        .bind(end)
        .bind(blocked_end)
        .execute(&mut *tx)
        .await;
        match moved {
            Ok(result) if result.rows_affected() == 1 => {}
            Ok(_) => return Err(AppError::conflict("The appointment changed; reload it")),
            Err(error) if is_code(&error, "23P01") => return Err(unavailable()),
            Err(error) => return Err(error.into()),
        }
        add_event(
            &mut tx,
            access,
            id,
            "rescheduled",
            json!({
                "from": {
                    "staff_id": row.staff_id,
                    "start_at": format_utc(row.start_at),
                    "end_at": format_utc(row.end_at),
                },
                "to": {
                    "staff_id": staff_id,
                    "start_at": format_utc(start),
                    "end_at": format_utc(end),
                },
                "reason": reason,
            }),
        )
        .await?;
        let row = fetch(&mut tx, access, id, false).await?;
        tx.commit().await?;
        Ok(row.into_view(now))
    }

    // -- a customer's own appointments ---------------------------------------

    /// 404 unless the appointment was booked by this user or belongs to this
    /// client. Foreign appointments are indistinguishable from missing ones.
    pub async fn ensure_owned(
        &self,
        access: BusinessAccess,
        id: Uuid,
        client_id: Option<Uuid>,
    ) -> AppResult<()> {
        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        let owned: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                 SELECT 1 FROM appointment
                 WHERE id = $1 AND business_id = $2
                   AND (booked_by_user_id = $3 OR ($4::uuid IS NOT NULL AND client_id = $4)))",
        )
        .bind(id)
        .bind(access.business_id.as_uuid())
        .bind(access.user_id.as_uuid())
        .bind(client_id)
        .fetch_one(&mut *tx)
        .await?;
        if owned {
            Ok(())
        } else {
            Err(AppError::not_found("Appointment not found"))
        }
    }

    /// The customer's appointments in this business, newest first. Without a
    /// `status`: live holds, confirmed, cancelled, completed and no-show.
    pub async fn list_for_client(
        &self,
        access: BusinessAccess,
        client_id: Option<Uuid>,
        status: Option<&str>,
    ) -> AppResult<Vec<AppointmentView>> {
        if let Some(status) = status {
            if !["confirmed", "held", "cancelled", "completed", "no_show"].contains(&status) {
                return Err(AppError::validation(
                    "status must be confirmed, held, cancelled, completed or no_show",
                ));
            }
        }
        let now = self.clock.now();
        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        let rows = sqlx::query_as::<_, AppointmentRow>(&format!(
            "SELECT {COLUMNS} {FROM}
             WHERE a.business_id = $1
               AND (a.booked_by_user_id = $2 OR ($3::uuid IS NOT NULL AND a.client_id = $3))
               AND (
                   ($4::text IS NULL AND (a.status IN ('confirmed', 'cancelled', 'completed', 'no_show')
                                          OR (a.status = 'held' AND a.hold_expires_at > $5)))
                OR ($4::text IS NOT NULL AND a.status = $4
                    AND (a.status <> 'held' OR a.hold_expires_at > $5))
               )
             ORDER BY a.start_at DESC, a.id
             LIMIT 100"
        ))
        .bind(access.business_id.as_uuid())
        .bind(access.user_id.as_uuid())
        .bind(client_id)
        .bind(status)
        .bind(now)
        .fetch_all(&mut *tx)
        .await?;
        Ok(rows.into_iter().map(|row| row.into_view(now)).collect())
    }

    // -- internals ----------------------------------------------------------

    /// Places a hold or a direct booking. Two identical requests racing return
    /// the winner's row instead of a conflict for the loser.
    async fn place(
        &self,
        access: BusinessAccess,
        placement: Placement,
    ) -> AppResult<BookingOutcome> {
        let now = self.clock.now();
        let result = self.place_in_tx(access, &placement, now).await;
        if matches!(&result, Err(error) if is_race_loss(error)) {
            let mut tx = begin_scoped(&self.pool, access.scope()).await?;
            if let Some(existing) = find_by_key(&mut tx, access, &placement.key).await? {
                return replay(existing, &placement.fingerprint, now);
            }
        }
        result
    }

    async fn place_in_tx(
        &self,
        access: BusinessAccess,
        placement: &Placement,
        now: OffsetDateTime,
    ) -> AppResult<BookingOutcome> {
        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        authorize_staff(&mut tx, &access, placement.staff_id).await?;

        // A retry with the same key returns the original result, whatever
        // happened to the calendar since.
        if let Some(existing) = find_by_key(&mut tx, access, &placement.key).await? {
            return replay(existing, &placement.fingerprint, now);
        }

        let online = placement.source != "manual";
        let rules = load_service_rules(
            &mut tx,
            access,
            placement.service_id,
            placement.variant_id,
            online,
        )
        .await?;
        let staff = eligible_staff(&mut tx, access, placement.service_id).await?;
        if !staff.contains(&placement.staff_id) {
            return Err(AppError::not_found(
                "This staff member does not perform the service",
            ));
        }

        if placement.client.is_none() {
            let live_holds: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM appointment
                 WHERE business_id = $1 AND booked_by_user_id = $2
                   AND status = 'held' AND hold_expires_at > $3",
            )
            .bind(access.business_id.as_uuid())
            .bind(access.user_id.as_uuid())
            .bind(now)
            .fetch_one(&mut *tx)
            .await?;
            if live_holds >= MAX_ACTIVE_HOLDS {
                return Err(AppError::RateLimited(
                    "Too many held slots; confirm or release some first".to_string(),
                ));
            }
        }

        let start = placement.start;
        let end = start + Duration::minutes(rules.duration_min.into());
        let blocked_end = end + Duration::minutes(rules.buffer_after_min.into());
        expire_stale_holds(&mut tx, access, placement.staff_id, start, blocked_end, now).await?;

        // The requested start must be a time the server offers right now.
        let day = local_date(&mut tx, access, start).await?;
        let offered = offered_slots(
            &mut tx,
            access,
            &rules,
            &[placement.staff_id],
            day,
            day,
            now,
            None,
        )
        .await?;
        if !offered.iter().any(|slot| slot.start == start) {
            return Err(unavailable());
        }

        let (status, expires, confirmed_at, event) = match placement.client {
            None => (
                "held",
                Some(now + Duration::minutes(HOLD_TTL_MINUTES)),
                None,
                "hold_created",
            ),
            Some(_) => ("confirmed", None, Some(now), "booked"),
        };
        let client = placement.client.as_ref();
        let inserted = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO appointment
                 (business_id, staff_id, status, start_at, end_at, blocked_end, hold_expires_at,
                  source, booked_by_user_id, idempotency_key, request_fingerprint,
                  client_name, client_phone, note, confirmed_at, client_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
             RETURNING id",
        )
        .bind(access.business_id.as_uuid())
        .bind(placement.staff_id)
        .bind(status)
        .bind(start)
        .bind(end)
        .bind(blocked_end)
        .bind(expires)
        .bind(&placement.source)
        .bind(access.user_id.as_uuid())
        .bind(&placement.key)
        .bind(&placement.fingerprint)
        .bind(client.map(|c| c.name.clone()))
        .bind(client.and_then(|c| c.phone.clone()))
        .bind(client.and_then(|c| c.note.clone()))
        .bind(confirmed_at)
        .bind(placement.client_id)
        .fetch_one(&mut *tx)
        .await;
        let appointment_id = match inserted {
            Ok(id) => id,
            // Someone else took the slot between our check and the insert.
            Err(error) if is_code(&error, "23P01") => return Err(unavailable()),
            Err(error) => return Err(error.into()),
        };

        // Names and price as they are now: what the client was shown.
        sqlx::query(
            "INSERT INTO appointment_item
                 (business_id, appointment_id, service_id, variant_id, service_name, variant_name,
                  duration_min, price_minor, currency)
             SELECT $1, $2, s.id, v.id, s.name, v.name, v.duration_min, v.price_minor, v.currency
             FROM service s
             JOIN service_variant v ON v.service_id = s.id AND v.business_id = s.business_id
             WHERE s.id = $3 AND v.id = $4 AND s.business_id = $1",
        )
        .bind(access.business_id.as_uuid())
        .bind(appointment_id)
        .bind(placement.service_id)
        .bind(placement.variant_id)
        .execute(&mut *tx)
        .await?;
        add_event(&mut tx, access, appointment_id, event, json!({})).await?;

        let row = fetch(&mut tx, access, appointment_id, false).await?;
        tx.commit().await?;
        Ok(BookingOutcome {
            appointment: row.into_view(now),
            created: true,
        })
    }

    /// Turns a live hold into a confirmed appointment.
    async fn confirm_hold(
        &self,
        access: BusinessAccess,
        hold_id: Uuid,
        client: Client,
        client_id: Option<Uuid>,
    ) -> AppResult<BookingOutcome> {
        let now = self.clock.now();
        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        let row = fetch(&mut tx, access, hold_id, true).await?;
        authorize_staff(&mut tx, &access, row.staff_id).await?;
        match row.status.as_str() {
            "confirmed" => {
                let same = row.client_name.as_deref() == Some(client.name.as_str())
                    && row.client_phone == client.phone
                    && row.note == client.note;
                if !same {
                    return Err(AppError::conflict(
                        "This hold was already confirmed with other details",
                    ));
                }
                return Ok(BookingOutcome {
                    appointment: row.into_view(now),
                    created: false,
                });
            }
            "held" => {}
            _ => return Err(hold_over()),
        }
        let confirmed = sqlx::query(
            "UPDATE appointment
             SET status = 'confirmed', hold_expires_at = NULL, confirmed_at = $3,
                 client_name = $4, client_phone = $5, note = $6,
                 client_id = COALESCE($7, client_id)
             WHERE id = $1 AND business_id = $2 AND status = 'held' AND hold_expires_at > $3",
        )
        .bind(hold_id)
        .bind(access.business_id.as_uuid())
        .bind(now)
        .bind(&client.name)
        .bind(&client.phone)
        .bind(&client.note)
        .bind(client_id)
        .execute(&mut *tx)
        .await?;
        if confirmed.rows_affected() != 1 {
            return Err(hold_over());
        }
        add_event(&mut tx, access, hold_id, "confirmed", json!({})).await?;
        let row = fetch(&mut tx, access, hold_id, false).await?;
        tx.commit().await?;
        Ok(BookingOutcome {
            appointment: row.into_view(now),
            created: true,
        })
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn unavailable() -> AppError {
    AppError::SlotUnavailable("This start time is not available".to_string())
}

fn hold_over() -> AppError {
    AppError::SlotUnavailable("The hold has expired; choose a time again".to_string())
}

fn valid_key(key: &str) -> AppResult<String> {
    let key = key.trim();
    if !(8..=100).contains(&key.chars().count()) || !key.chars().all(|c| c.is_ascii_graphic()) {
        return Err(AppError::validation(
            "Idempotency-Key header is required (8-100 visible characters)",
        ));
    }
    Ok(key.to_string())
}

fn valid_source(source: Option<&str>) -> AppResult<String> {
    match source.unwrap_or("manual") {
        value @ ("manual" | "app" | "web") => Ok(value.to_string()),
        _ => Err(AppError::validation("source must be manual, app or web")),
    }
}

fn parse_client(name: &str, phone: Option<String>, note: Option<String>) -> AppResult<Client> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 120 {
        return Err(AppError::validation("client_name must be 1-120 characters"));
    }
    let phone = match phone.map(|value| value.trim().to_string()) {
        Some(value) if !value.is_empty() => Some(normalize_phone(&value)?),
        _ => None,
    };
    Ok(Client {
        name: name.to_string(),
        phone,
        note: clean_optional(note, 500, "note")?,
    })
}

/// `+48 600-100-200` -> `+48600100200`; only international numbers.
pub(crate) fn normalize_phone(value: &str) -> AppResult<String> {
    let compact: String = value
        .chars()
        .filter(|c| !matches!(c, ' ' | '-' | '(' | ')'))
        .collect();
    let digits = compact.strip_prefix('+').unwrap_or("");
    let ok = (7..=15).contains(&digits.len())
        && digits.chars().all(|c| c.is_ascii_digit())
        && !digits.starts_with('0');
    if ok {
        Ok(compact)
    } else {
        Err(AppError::validation(
            "client_phone must be an international number like +48 600 100 200",
        ))
    }
}

/// A booking attempt lost a race: the calendar constraint or the idempotency
/// key index refused the insert.
fn is_race_loss(error: &AppError) -> bool {
    match error {
        AppError::SlotUnavailable(_) => true,
        AppError::Database(error) => is_code(error, "23505"),
        _ => false,
    }
}

fn is_code(error: &sqlx::Error, code: &str) -> bool {
    error
        .as_database_error()
        .and_then(|db| db.code())
        .map(|value| value == code)
        .unwrap_or(false)
}

fn replay(
    existing: AppointmentRow,
    fingerprint: &str,
    now: OffsetDateTime,
) -> AppResult<BookingOutcome> {
    if existing.request_fingerprint.as_deref() != Some(fingerprint) {
        return Err(AppError::conflict(
            "This Idempotency-Key was already used for a different request",
        ));
    }
    Ok(BookingOutcome {
        appointment: existing.into_view(now),
        created: false,
    })
}

async fn find_by_key(
    conn: &mut PgConnection,
    access: BusinessAccess,
    key: &str,
) -> AppResult<Option<AppointmentRow>> {
    Ok(sqlx::query_as::<_, AppointmentRow>(&format!(
        "SELECT {COLUMNS} {FROM}
         WHERE a.business_id = $1 AND a.booked_by_user_id = $2 AND a.idempotency_key = $3"
    ))
    .bind(access.business_id.as_uuid())
    .bind(access.user_id.as_uuid())
    .bind(key)
    .fetch_optional(&mut *conn)
    .await?)
}

/// Loads one appointment; `lock` takes a row lock so cancel/reschedule/confirm
/// of the same appointment run one after another.
async fn fetch(
    conn: &mut PgConnection,
    access: BusinessAccess,
    id: Uuid,
    lock: bool,
) -> AppResult<AppointmentRow> {
    sqlx::query_as::<_, AppointmentRow>(&format!(
        "SELECT {COLUMNS} {FROM}
         WHERE a.id = $1 AND a.business_id = $2{}",
        if lock { " FOR UPDATE OF a" } else { "" }
    ))
    .bind(id)
    .bind(access.business_id.as_uuid())
    .fetch_optional(&mut *conn)
    .await?
    .ok_or_else(|| AppError::not_found("Appointment not found"))
}

async fn add_event(
    conn: &mut PgConnection,
    access: BusinessAccess,
    appointment_id: Uuid,
    kind: &str,
    data: Value,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO appointment_event (business_id, appointment_id, type, actor_user_id, data)
         VALUES ($1, $2, $3, $4, $5::text::jsonb)",
    )
    .bind(access.business_id.as_uuid())
    .bind(appointment_id)
    .bind(kind)
    .bind(access.user_id.as_uuid())
    .bind(data.to_string())
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Holds whose time is up no longer block the calendar; the database
/// constraint cannot know the time, so they are marked expired here before a
/// new booking on that master's calendar.
async fn expire_stale_holds(
    conn: &mut PgConnection,
    access: BusinessAccess,
    staff_id: Uuid,
    start: OffsetDateTime,
    blocked_end: OffsetDateTime,
    now: OffsetDateTime,
) -> AppResult<()> {
    sqlx::query(
        "WITH flipped AS (
             UPDATE appointment SET status = 'expired'
             WHERE business_id = $1 AND staff_id = $2 AND status = 'held'
               AND hold_expires_at <= $3
               AND tstzrange(start_at, blocked_end) && tstzrange($4, $5)
             RETURNING id
         )
         INSERT INTO appointment_event (business_id, appointment_id, type)
         SELECT $1, id, 'hold_expired' FROM flipped",
    )
    .bind(access.business_id.as_uuid())
    .bind(staff_id)
    .bind(now)
    .bind(start)
    .bind(blocked_end)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// The local date (business time zone) of an instant.
async fn local_date(
    conn: &mut PgConnection,
    access: BusinessAccess,
    instant: OffsetDateTime,
) -> AppResult<Date> {
    let (day,): (Date,) = sqlx::query_as(
        "SELECT ($2::timestamptz AT TIME ZONE timezone)::date FROM business WHERE id = $1",
    )
    .bind(access.business_id.as_uuid())
    .bind(instant)
    .fetch_one(&mut *conn)
    .await?;
    Ok(day)
}

async fn own_staff_id(conn: &mut PgConnection, access: &BusinessAccess) -> AppResult<Option<Uuid>> {
    Ok(sqlx::query_scalar(
        "SELECT s.id FROM staff_member s
         JOIN membership m ON m.id = s.membership_id
         WHERE s.business_id = $1 AND m.user_id = $2
         LIMIT 1",
    )
    .bind(access.business_id.as_uuid())
    .bind(access.user_id.as_uuid())
    .fetch_optional(&mut *conn)
    .await?)
}

/// Owner, manager and reception act on any master's calendar; an employee only
/// on their own. An unknown master is a 404.
async fn authorize_staff(
    conn: &mut PgConnection,
    access: &BusinessAccess,
    staff_id: Uuid,
) -> AppResult<()> {
    let own: Option<bool> = sqlx::query_scalar(
        "SELECT COALESCE(m.user_id = $3, false)
         FROM staff_member s
         LEFT JOIN membership m ON m.id = s.membership_id
         WHERE s.id = $1 AND s.business_id = $2",
    )
    .bind(staff_id)
    .bind(access.business_id.as_uuid())
    .bind(access.user_id.as_uuid())
    .fetch_optional(&mut *conn)
    .await?;
    let own = own.ok_or_else(|| AppError::not_found("Staff member not found"))?;
    match access.role {
        Role::Owner | Role::Manager | Role::Reception => Ok(()),
        Role::Employee if own => Ok(()),
        Role::Employee => Err(AppError::authorization(
            "Employees can book only their own calendar",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phones_are_normalised_to_international_form() {
        assert_eq!(normalize_phone("+48 600-100-200").unwrap(), "+48600100200");
        assert_eq!(
            normalize_phone("+1 (415) 555 0100").unwrap(),
            "+14155550100"
        );
        for bad in [
            "600100200",
            "+0123456789",
            "+48 12",
            "+48abc600100",
            "++48600100200",
        ] {
            assert!(normalize_phone(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn clients_need_a_name_and_a_sane_phone() {
        assert!(parse_client("  ", None, None).is_err());
        assert!(parse_client(&"x".repeat(121), None, None).is_err());
        let client = parse_client(" Ewa ", Some("  ".into()), Some(" ".into())).unwrap();
        assert_eq!(client.name, "Ewa");
        assert_eq!(client.phone, None);
        assert_eq!(client.note, None);
        assert!(parse_client("Ewa", Some("123".into()), None).is_err());
    }

    #[test]
    fn idempotency_keys_and_sources_are_checked() {
        assert!(valid_key("short").is_err());
        assert!(valid_key("has space in it").is_err());
        assert!(valid_key(&"k".repeat(101)).is_err());
        assert_eq!(valid_key("  good-key-1234 ").unwrap(), "good-key-1234");
        assert_eq!(valid_source(None).unwrap(), "manual");
        assert!(valid_source(Some("phone")).is_err());
    }
}
