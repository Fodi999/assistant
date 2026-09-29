//! Booking: slot holds (this stage), later confirmed appointments,
//! rescheduling and cancellation.
//!
//! A hold is an appointment row in status `held` with `hold_expires_at`. It
//! blocks the master's calendar exactly like a confirmed appointment (the
//! database refuses overlaps, see migration `appointments`), until it expires,
//! is released or is confirmed. The requested start must be one of the times
//! the server itself offers (`availability`), re-checked inside the same
//! transaction that inserts the row.
//!
//! Who books (PRODUCT_SPEC §17.1): owner, manager and reception for any master;
//! an employee only for their own calendar.

use crate::application::access::{BusinessAccess, Role};
use crate::application::availability::{eligible_staff, load_service_rules, offered_slots};
use crate::application::schedule::{format_utc, parse_instant};
use crate::infrastructure::begin_scoped;
use crate::shared::{AppError, AppResult, Clock};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use time::{Date, Duration, OffsetDateTime};
use uuid::Uuid;

/// How long a slot is held for a client who is still choosing.
pub const HOLD_TTL_MINUTES: i64 = 10;
/// Most live holds one user may keep at a time (guards against slot hoarding).
const MAX_ACTIVE_HOLDS: i64 = 10;

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

#[derive(Debug, Serialize)]
pub struct HoldView {
    pub id: Uuid,
    /// `held`, or `expired` once the time is up or the hold was released.
    pub status: String,
    pub staff_id: Uuid,
    pub service_id: Uuid,
    pub variant_id: Uuid,
    pub start_at: String,
    pub end_at: String,
    pub hold_expires_at: String,
    pub source: String,
}

#[derive(sqlx::FromRow)]
struct HoldRow {
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
}

const HOLD_COLUMNS: &str = "a.id, a.status, a.staff_id, i.service_id, i.variant_id, a.start_at,
     a.end_at, a.hold_expires_at, a.source, a.request_fingerprint";

impl HoldRow {
    fn into_view(self, now: OffsetDateTime) -> HoldView {
        let expires = self.hold_expires_at.unwrap_or(self.end_at);
        let live = self.status == "held" && expires > now;
        HoldView {
            id: self.id,
            status: if self.status == "held" && !live {
                "expired".to_string()
            } else {
                self.status
            },
            staff_id: self.staff_id,
            service_id: self.service_id,
            variant_id: self.variant_id,
            start_at: format_utc(self.start_at),
            end_at: format_utc(self.end_at),
            hold_expires_at: format_utc(expires),
            source: self.source,
        }
    }
}

#[derive(Clone)]
pub struct BookingService {
    pool: PgPool,
    clock: Clock,
}

/// Result of a hold request: `created` is false when the same idempotency key
/// was already used for the same request and the existing hold is returned.
pub struct HoldOutcome {
    pub hold: HoldView,
    pub created: bool,
}

impl BookingService {
    pub fn new(pool: PgPool, clock: Clock) -> Self {
        Self { pool, clock }
    }

    pub async fn create_hold(
        &self,
        access: BusinessAccess,
        idempotency_key: &str,
        input: CreateHoldInput,
    ) -> AppResult<HoldOutcome> {
        let key = idempotency_key.trim();
        if !(8..=100).contains(&key.chars().count()) || !key.chars().all(|c| c.is_ascii_graphic()) {
            return Err(AppError::validation(
                "Idempotency-Key header is required (8-100 visible characters)",
            ));
        }
        let start = parse_instant(&input.start_at, "start_at")?;
        let source = input.source.as_deref().unwrap_or("manual");
        if !["manual", "app", "web"].contains(&source) {
            return Err(AppError::validation("source must be manual, app or web"));
        }
        let online = source != "manual";
        let fingerprint = format!(
            "{}|{}|{}|{}|{}",
            input.service_id,
            input.variant_id,
            input.staff_id,
            start.unix_timestamp(),
            source
        );
        let now = self.clock.now();

        let result = self
            .create_in_tx(
                access,
                key,
                &input,
                start,
                source,
                online,
                &fingerprint,
                now,
            )
            .await;
        // Two identical requests racing: the loser hits the calendar constraint
        // (or the key index) although the winner is the very same request.
        // Answer it with the winner's hold instead of a conflict.
        if matches!(&result, Err(error) if is_race_loss(error)) {
            let mut tx = begin_scoped(&self.pool, access.scope()).await?;
            if let Some(existing) = find_by_key(&mut tx, access, key).await? {
                return replay(existing, &fingerprint, now);
            }
        }
        result
    }

    #[allow(clippy::too_many_arguments)]
    async fn create_in_tx(
        &self,
        access: BusinessAccess,
        key: &str,
        input: &CreateHoldInput,
        start: OffsetDateTime,
        source: &str,
        online: bool,
        fingerprint: &str,
        now: OffsetDateTime,
    ) -> AppResult<HoldOutcome> {
        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        authorize_staff(&mut tx, &access, input.staff_id).await?;

        // A retry with the same key returns the original hold, whatever
        // happened to the calendar since.
        if let Some(existing) = find_by_key(&mut tx, access, key).await? {
            return replay(existing, fingerprint, now);
        }

        let rules =
            load_service_rules(&mut tx, access, input.service_id, input.variant_id, online).await?;
        let staff = eligible_staff(&mut tx, access, input.service_id).await?;
        if !staff.contains(&input.staff_id) {
            return Err(AppError::not_found(
                "This staff member does not perform the service",
            ));
        }

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

        let end = start + Duration::minutes(rules.duration_min.into());
        let blocked_end = end + Duration::minutes(rules.buffer_after_min.into());

        // Holds whose time is up no longer block the calendar; the database
        // constraint cannot know the time, so they are marked expired here.
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
        .bind(input.staff_id)
        .bind(now)
        .bind(start)
        .bind(blocked_end)
        .execute(&mut *tx)
        .await?;

        // The requested start must be a time the server offers right now.
        let (day,): (Date,) = sqlx::query_as(
            "SELECT ($2::timestamptz AT TIME ZONE timezone)::date FROM business WHERE id = $1",
        )
        .bind(access.business_id.as_uuid())
        .bind(start)
        .fetch_one(&mut *tx)
        .await?;
        let offered = offered_slots(
            &mut tx,
            access,
            &rules,
            &[input.staff_id],
            day,
            day,
            now,
            None,
        )
        .await?;
        if !offered.iter().any(|slot| slot.start == start) {
            return Err(AppError::SlotUnavailable(
                "This start time is not available".to_string(),
            ));
        }

        let inserted: Result<Uuid, sqlx::Error> = sqlx::query_scalar(
            "INSERT INTO appointment
                 (business_id, staff_id, status, start_at, end_at, blocked_end, hold_expires_at,
                  source, booked_by_user_id, idempotency_key, request_fingerprint)
             VALUES ($1, $2, 'held', $3, $4, $5, $6, $7, $8, $9, $10)
             RETURNING id",
        )
        .bind(access.business_id.as_uuid())
        .bind(input.staff_id)
        .bind(start)
        .bind(end)
        .bind(blocked_end)
        .bind(now + Duration::minutes(HOLD_TTL_MINUTES))
        .bind(source)
        .bind(access.user_id.as_uuid())
        .bind(key)
        .bind(fingerprint)
        .fetch_one(&mut *tx)
        .await;
        let appointment_id = match inserted {
            Ok(id) => id,
            // Someone else took the slot between our check and the insert.
            Err(error) if is_code(&error, "23P01") => {
                return Err(AppError::SlotUnavailable(
                    "This start time is not available".to_string(),
                ));
            }
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
        .bind(input.service_id)
        .bind(input.variant_id)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO appointment_event (business_id, appointment_id, type, actor_user_id)
             VALUES ($1, $2, 'hold_created', $3)",
        )
        .bind(access.business_id.as_uuid())
        .bind(appointment_id)
        .bind(access.user_id.as_uuid())
        .execute(&mut *tx)
        .await?;

        let row = fetch_hold(&mut tx, access, appointment_id).await?;
        tx.commit().await?;
        Ok(HoldOutcome {
            hold: row.into_view(now),
            created: true,
        })
    }

    pub async fn get_hold(&self, access: BusinessAccess, id: Uuid) -> AppResult<HoldView> {
        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        let row = fetch_hold(&mut tx, access, id).await?;
        authorize_staff(&mut tx, &access, row.staff_id).await?;
        Ok(row.into_view(self.clock.now()))
    }

    /// Frees a held slot. Releasing an already expired hold is fine.
    pub async fn release_hold(&self, access: BusinessAccess, id: Uuid) -> AppResult<()> {
        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        let row = fetch_hold(&mut tx, access, id).await?;
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
                sqlx::query(
                    "INSERT INTO appointment_event (business_id, appointment_id, type, actor_user_id)
                     VALUES ($1, $2, 'hold_released', $3)",
                )
                .bind(access.business_id.as_uuid())
                .bind(id)
                .bind(access.user_id.as_uuid())
                .execute(&mut *tx)
                .await?;
            }
            "expired" => {}
            _ => return Err(AppError::conflict("This appointment is not a hold")),
        }
        tx.commit().await?;
        Ok(())
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

fn replay(existing: HoldRow, fingerprint: &str, now: OffsetDateTime) -> AppResult<HoldOutcome> {
    if existing.request_fingerprint.as_deref() != Some(fingerprint) {
        return Err(AppError::conflict(
            "This Idempotency-Key was already used for a different request",
        ));
    }
    Ok(HoldOutcome {
        hold: existing.into_view(now),
        created: false,
    })
}

async fn find_by_key(
    conn: &mut PgConnection,
    access: BusinessAccess,
    key: &str,
) -> AppResult<Option<HoldRow>> {
    Ok(sqlx::query_as::<_, HoldRow>(&format!(
        "SELECT {HOLD_COLUMNS}
         FROM appointment a JOIN appointment_item i ON i.appointment_id = a.id
         WHERE a.business_id = $1 AND a.booked_by_user_id = $2 AND a.idempotency_key = $3"
    ))
    .bind(access.business_id.as_uuid())
    .bind(access.user_id.as_uuid())
    .bind(key)
    .fetch_optional(&mut *conn)
    .await?)
}

async fn fetch_hold(
    conn: &mut PgConnection,
    access: BusinessAccess,
    id: Uuid,
) -> AppResult<HoldRow> {
    sqlx::query_as::<_, HoldRow>(&format!(
        "SELECT {HOLD_COLUMNS}
         FROM appointment a JOIN appointment_item i ON i.appointment_id = a.id
         WHERE a.id = $1 AND a.business_id = $2"
    ))
    .bind(id)
    .bind(access.business_id.as_uuid())
    .fetch_optional(&mut *conn)
    .await?
    .ok_or_else(|| AppError::not_found("Hold not found"))
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
