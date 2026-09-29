//! Master schedules: weekly working hours, breaks, one-day exceptions and time
//! off (PRODUCT_SPEC §10.3, §12).
//!
//! Time model: working hours, breaks and exceptions are wall-clock times in the
//! business time zone (`business.timezone`, IANA; Europe/Warsaw to start), so a
//! daylight-saving change never shifts a master's day. Time off is an absolute
//! UTC interval. Weekdays are numbered 0 = Monday ... 6 = Sunday. Intervals do
//! not cross midnight.
//!
//! Who may do what (PRODUCT_SPEC §17.1): owners and managers manage every
//! staff member's schedule; an employee manages only their own and cannot add
//! time in the past; reception has no access.

use crate::application::access::{BusinessAccess, Role};
use crate::infrastructure::begin_scoped;
use crate::shared::{AppError, AppResult, BusinessId};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use time::format_description::well_known::Rfc3339;
use time::format_description::BorrowedFormatItem;
use time::macros::format_description;
use time::{Date, Duration, OffsetDateTime, Time, UtcOffset};
use uuid::Uuid;

const TIME_FORMAT: &[BorrowedFormatItem<'static>] = format_description!("[hour]:[minute]");
const DATE_FORMAT: &[BorrowedFormatItem<'static>] = format_description!("[year]-[month]-[day]");

const MAX_INTERVALS: usize = 60;
const MAX_TIME_OFF_DAYS: i64 = 366;
const TIME_OFF_KINDS: [&str; 4] = ["vacation", "sick", "blocked", "break"];

// ---------------------------------------------------------------------------
// Input
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct WeeklyIntervalInput {
    /// 0 = Monday ... 6 = Sunday.
    pub weekday: i16,
    /// Local time "HH:MM".
    pub start: String,
    pub end: String,
    /// First and last local date (inclusive) the interval applies; open when absent.
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SetWeeklyInput {
    pub intervals: Vec<WeeklyIntervalInput>,
}

#[derive(Debug, Deserialize)]
pub struct BreakInput {
    pub weekday: i16,
    pub start: String,
    pub end: String,
}

#[derive(Debug, Deserialize)]
pub struct SetBreaksInput {
    pub breaks: Vec<BreakInput>,
}

#[derive(Debug, Deserialize)]
pub struct ExceptionInput {
    /// `day_off` or `custom_hours` (then `start` and `end` are required).
    pub kind: String,
    pub start: Option<String>,
    pub end: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TimeOffInput {
    /// RFC 3339, e.g. `2026-07-01T00:00:00Z`; stored and returned in UTC.
    pub start_at: String,
    pub end_at: String,
    /// `vacation`, `sick`, `blocked` or `break`.
    pub kind: String,
    pub note: Option<String>,
    /// Reserved for recurring blocks; not supported yet.
    pub rrule: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TimeOffQuery {
    pub from: Option<String>,
    pub to: Option<String>,
}

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct WeeklyView {
    pub id: Uuid,
    pub weekday: i16,
    pub start: String,
    pub end: String,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BreakView {
    pub id: Uuid,
    pub weekday: i16,
    pub start: String,
    pub end: String,
}

#[derive(Debug, Serialize)]
pub struct ExceptionView {
    pub id: Uuid,
    pub date: String,
    pub kind: String,
    pub start: Option<String>,
    pub end: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TimeOffView {
    pub id: Uuid,
    pub staff_id: Uuid,
    pub start_at: String,
    pub end_at: String,
    pub kind: String,
    pub note: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ScheduleView {
    /// IANA zone in which all local times below are meant.
    pub timezone: String,
    pub weekly: Vec<WeeklyView>,
    pub breaks: Vec<BreakView>,
    /// Exceptions from today (business local date) onwards.
    pub exceptions: Vec<ExceptionView>,
}

#[derive(sqlx::FromRow)]
struct WeeklyRow {
    id: Uuid,
    weekday: i16,
    start_local: Time,
    end_local: Time,
    valid_from: Option<Date>,
    valid_to: Option<Date>,
}

#[derive(sqlx::FromRow)]
struct BreakRow {
    id: Uuid,
    weekday: i16,
    start_local: Time,
    end_local: Time,
}

#[derive(sqlx::FromRow)]
struct ExceptionRow {
    id: Uuid,
    date_local: Date,
    kind: String,
    start_local: Option<Time>,
    end_local: Option<Time>,
}

#[derive(sqlx::FromRow)]
struct TimeOffRow {
    id: Uuid,
    staff_id: Uuid,
    start_at: OffsetDateTime,
    end_at: OffsetDateTime,
    kind: String,
    note: Option<String>,
}

impl From<WeeklyRow> for WeeklyView {
    fn from(row: WeeklyRow) -> Self {
        Self {
            id: row.id,
            weekday: row.weekday,
            start: format_time(row.start_local),
            end: format_time(row.end_local),
            valid_from: row.valid_from.map(format_date),
            valid_to: row.valid_to.map(format_date),
        }
    }
}

impl From<BreakRow> for BreakView {
    fn from(row: BreakRow) -> Self {
        Self {
            id: row.id,
            weekday: row.weekday,
            start: format_time(row.start_local),
            end: format_time(row.end_local),
        }
    }
}

impl From<ExceptionRow> for ExceptionView {
    fn from(row: ExceptionRow) -> Self {
        Self {
            id: row.id,
            date: format_date(row.date_local),
            kind: row.kind,
            start: row.start_local.map(format_time),
            end: row.end_local.map(format_time),
        }
    }
}

impl From<TimeOffRow> for TimeOffView {
    fn from(row: TimeOffRow) -> Self {
        Self {
            id: row.id,
            staff_id: row.staff_id,
            start_at: format_utc(row.start_at),
            end_at: format_utc(row.end_at),
            kind: row.kind,
            note: row.note,
        }
    }
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct ScheduleService {
    pool: PgPool,
}

impl ScheduleService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_schedule(
        &self,
        access: BusinessAccess,
        staff_id: Uuid,
    ) -> AppResult<ScheduleView> {
        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        authorize_staff(&mut tx, &access, staff_id).await?;
        let (timezone, today) = business_clock(&mut tx, access.business_id).await?;

        let weekly = load_weekly(&mut tx, access.business_id, staff_id).await?;
        let breaks = load_breaks(&mut tx, access.business_id, staff_id).await?;
        let exceptions = sqlx::query_as::<_, ExceptionRow>(
            "SELECT id, date_local, kind, start_local, end_local
             FROM schedule_exception
             WHERE business_id = $1 AND staff_id = $2 AND date_local >= $3
             ORDER BY date_local",
        )
        .bind(access.business_id.as_uuid())
        .bind(staff_id)
        .bind(today)
        .fetch_all(&mut *tx)
        .await?;

        Ok(ScheduleView {
            timezone,
            weekly,
            breaks,
            exceptions: exceptions.into_iter().map(ExceptionView::from).collect(),
        })
    }

    /// Replaces the whole weekly pattern of a staff member (all or nothing).
    pub async fn set_weekly(
        &self,
        access: BusinessAccess,
        staff_id: Uuid,
        input: SetWeeklyInput,
    ) -> AppResult<Vec<WeeklyView>> {
        if input.intervals.len() > MAX_INTERVALS {
            return Err(AppError::validation(format!(
                "At most {MAX_INTERVALS} intervals"
            )));
        }
        let mut parsed = Vec::with_capacity(input.intervals.len());
        for interval in &input.intervals {
            check_weekday(interval.weekday)?;
            let (start, end) = parse_span(&interval.start, &interval.end)?;
            let valid_from = parse_optional_date(interval.valid_from.as_deref(), "valid_from")?;
            let valid_to = parse_optional_date(interval.valid_to.as_deref(), "valid_to")?;
            if let (Some(from), Some(to)) = (valid_from, valid_to) {
                if from > to {
                    return Err(AppError::validation(
                        "valid_from must not be after valid_to",
                    ));
                }
            }
            parsed.push((interval.weekday, start, end, valid_from, valid_to));
        }

        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        authorize_staff(&mut tx, &access, staff_id).await?;
        sqlx::query("DELETE FROM working_schedule WHERE business_id = $1 AND staff_id = $2")
            .bind(access.business_id.as_uuid())
            .bind(staff_id)
            .execute(&mut *tx)
            .await?;
        for (weekday, start, end, valid_from, valid_to) in parsed {
            sqlx::query(
                "INSERT INTO working_schedule
                     (business_id, staff_id, weekday, start_local, end_local, valid_from, valid_to)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(access.business_id.as_uuid())
            .bind(staff_id)
            .bind(weekday)
            .bind(start)
            .bind(end)
            .bind(valid_from)
            .bind(valid_to)
            .execute(&mut *tx)
            .await
            .map_err(on_overlap(
                "Working intervals of the same weekday must not overlap",
            ))?;
        }
        let weekly = load_weekly(&mut tx, access.business_id, staff_id).await?;
        tx.commit().await?;
        Ok(weekly)
    }

    /// Replaces all weekly breaks of a staff member (all or nothing).
    pub async fn set_breaks(
        &self,
        access: BusinessAccess,
        staff_id: Uuid,
        input: SetBreaksInput,
    ) -> AppResult<Vec<BreakView>> {
        if input.breaks.len() > MAX_INTERVALS {
            return Err(AppError::validation(format!(
                "At most {MAX_INTERVALS} breaks"
            )));
        }
        let mut parsed = Vec::with_capacity(input.breaks.len());
        for item in &input.breaks {
            check_weekday(item.weekday)?;
            let (start, end) = parse_span(&item.start, &item.end)?;
            parsed.push((item.weekday, start, end));
        }

        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        authorize_staff(&mut tx, &access, staff_id).await?;
        sqlx::query("DELETE FROM schedule_break WHERE business_id = $1 AND staff_id = $2")
            .bind(access.business_id.as_uuid())
            .bind(staff_id)
            .execute(&mut *tx)
            .await?;
        for (weekday, start, end) in parsed {
            sqlx::query(
                "INSERT INTO schedule_break (business_id, staff_id, weekday, start_local, end_local)
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(access.business_id.as_uuid())
            .bind(staff_id)
            .bind(weekday)
            .bind(start)
            .bind(end)
            .execute(&mut *tx)
            .await
            .map_err(on_overlap("Breaks of the same weekday must not overlap"))?;
        }
        let breaks = load_breaks(&mut tx, access.business_id, staff_id).await?;
        tx.commit().await?;
        Ok(breaks)
    }

    /// Creates or replaces the exception for one local date.
    pub async fn put_exception(
        &self,
        access: BusinessAccess,
        staff_id: Uuid,
        date: &str,
        input: ExceptionInput,
    ) -> AppResult<ExceptionView> {
        let date = parse_date(date, "date")?;
        let (start, end) = match input.kind.as_str() {
            "day_off" => {
                if input.start.is_some() || input.end.is_some() {
                    return Err(AppError::validation("A day off has no start or end"));
                }
                (None, None)
            }
            "custom_hours" => {
                let (Some(start), Some(end)) = (input.start.as_deref(), input.end.as_deref())
                else {
                    return Err(AppError::validation("custom_hours needs start and end"));
                };
                let (start, end) = parse_span(start, end)?;
                (Some(start), Some(end))
            }
            _ => {
                return Err(AppError::validation(
                    "kind must be 'day_off' or 'custom_hours'",
                ))
            }
        };

        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        authorize_staff(&mut tx, &access, staff_id).await?;
        let (_, today) = business_clock(&mut tx, access.business_id).await?;
        if date < today && !access.role.can_manage_business() {
            return Err(AppError::validation("Dates in the past cannot be changed"));
        }
        let row = sqlx::query_as::<_, ExceptionRow>(
            "INSERT INTO schedule_exception (business_id, staff_id, date_local, kind, start_local, end_local)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (staff_id, date_local) DO UPDATE
                 SET kind = EXCLUDED.kind,
                     start_local = EXCLUDED.start_local,
                     end_local = EXCLUDED.end_local
             RETURNING id, date_local, kind, start_local, end_local",
        )
        .bind(access.business_id.as_uuid())
        .bind(staff_id)
        .bind(date)
        .bind(&input.kind)
        .bind(start)
        .bind(end)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(row.into())
    }

    pub async fn delete_exception(
        &self,
        access: BusinessAccess,
        staff_id: Uuid,
        date: &str,
    ) -> AppResult<()> {
        let date = parse_date(date, "date")?;
        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        authorize_staff(&mut tx, &access, staff_id).await?;
        let deleted: Option<Uuid> = sqlx::query_scalar(
            "DELETE FROM schedule_exception
             WHERE business_id = $1 AND staff_id = $2 AND date_local = $3
             RETURNING id",
        )
        .bind(access.business_id.as_uuid())
        .bind(staff_id)
        .bind(date)
        .fetch_optional(&mut *tx)
        .await?;
        if deleted.is_none() {
            return Err(AppError::not_found("No exception on this date"));
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn create_time_off(
        &self,
        access: BusinessAccess,
        staff_id: Uuid,
        input: TimeOffInput,
    ) -> AppResult<TimeOffView> {
        let start_at = parse_instant(&input.start_at, "start_at")?;
        let end_at = parse_instant(&input.end_at, "end_at")?;
        if end_at <= start_at {
            return Err(AppError::validation("end_at must be after start_at"));
        }
        if end_at - start_at > Duration::days(MAX_TIME_OFF_DAYS) {
            return Err(AppError::validation(format!(
                "Time off can last at most {MAX_TIME_OFF_DAYS} days"
            )));
        }
        if !TIME_OFF_KINDS.contains(&input.kind.as_str()) {
            return Err(AppError::validation(
                "kind must be one of vacation, sick, blocked, break",
            ));
        }
        if input
            .rrule
            .as_deref()
            .is_some_and(|rule| !rule.trim().is_empty())
        {
            return Err(AppError::validation(
                "Recurring time off (rrule) is not supported yet",
            ));
        }
        let note = crate::application::auth::clean_optional(input.note, 500, "note")?;

        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        authorize_staff(&mut tx, &access, staff_id).await?;
        if end_at <= OffsetDateTime::now_utc() && !access.role.can_manage_business() {
            return Err(AppError::validation("Time in the past cannot be blocked"));
        }
        let row = sqlx::query_as::<_, TimeOffRow>(
            "INSERT INTO time_off (business_id, staff_id, start_at, end_at, kind, note, created_by)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING id, staff_id, start_at, end_at, kind, note",
        )
        .bind(access.business_id.as_uuid())
        .bind(staff_id)
        .bind(start_at)
        .bind(end_at)
        .bind(&input.kind)
        .bind(note)
        .bind(access.user_id.as_uuid())
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(row.into())
    }

    /// Time off overlapping `[from, to)`; both bounds are optional.
    pub async fn list_time_off(
        &self,
        access: BusinessAccess,
        staff_id: Uuid,
        query: TimeOffQuery,
    ) -> AppResult<Vec<TimeOffView>> {
        let from = match query.from.as_deref() {
            Some(value) => Some(parse_instant(value, "from")?),
            None => None,
        };
        let to = match query.to.as_deref() {
            Some(value) => Some(parse_instant(value, "to")?),
            None => None,
        };

        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        authorize_staff(&mut tx, &access, staff_id).await?;
        let rows = sqlx::query_as::<_, TimeOffRow>(
            "SELECT id, staff_id, start_at, end_at, kind, note
             FROM time_off
             WHERE business_id = $1 AND staff_id = $2
               AND ($3::timestamptz IS NULL OR end_at > $3)
               AND ($4::timestamptz IS NULL OR start_at < $4)
             ORDER BY start_at",
        )
        .bind(access.business_id.as_uuid())
        .bind(staff_id)
        .bind(from)
        .bind(to)
        .fetch_all(&mut *tx)
        .await?;
        Ok(rows.into_iter().map(TimeOffView::from).collect())
    }

    pub async fn delete_time_off(
        &self,
        access: BusinessAccess,
        time_off_id: Uuid,
    ) -> AppResult<()> {
        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        let staff_id: Option<Uuid> =
            sqlx::query_scalar("SELECT staff_id FROM time_off WHERE id = $1 AND business_id = $2")
                .bind(time_off_id)
                .bind(access.business_id.as_uuid())
                .fetch_optional(&mut *tx)
                .await?;
        let staff_id = staff_id.ok_or_else(|| AppError::not_found("Time off not found"))?;
        authorize_staff(&mut tx, &access, staff_id).await?;
        sqlx::query("DELETE FROM time_off WHERE id = $1 AND business_id = $2")
            .bind(time_off_id)
            .bind(access.business_id.as_uuid())
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Queries and rules shared by the operations
// ---------------------------------------------------------------------------

/// The staff member must exist in the business; owners and managers may act on
/// anyone, an employee only on their own card, reception on nobody.
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
        Role::Owner | Role::Manager => Ok(()),
        Role::Employee if own => Ok(()),
        Role::Employee => Err(AppError::authorization(
            "Employees can manage only their own schedule",
        )),
        Role::Reception => Err(AppError::authorization("This role cannot access schedules")),
    }
}

/// The business time zone and today's date in it.
async fn business_clock(
    conn: &mut PgConnection,
    business_id: BusinessId,
) -> AppResult<(String, Date)> {
    let clock: Option<(String, Date)> = sqlx::query_as(
        "SELECT timezone, (now() AT TIME ZONE timezone)::date FROM business WHERE id = $1",
    )
    .bind(business_id.as_uuid())
    .fetch_optional(&mut *conn)
    .await?;
    clock.ok_or_else(|| AppError::not_found("Business not found"))
}

async fn load_weekly(
    conn: &mut PgConnection,
    business_id: BusinessId,
    staff_id: Uuid,
) -> AppResult<Vec<WeeklyView>> {
    let rows = sqlx::query_as::<_, WeeklyRow>(
        "SELECT id, weekday, start_local, end_local, valid_from, valid_to
         FROM working_schedule
         WHERE business_id = $1 AND staff_id = $2
         ORDER BY weekday, start_local, valid_from NULLS FIRST",
    )
    .bind(business_id.as_uuid())
    .bind(staff_id)
    .fetch_all(&mut *conn)
    .await?;
    Ok(rows.into_iter().map(WeeklyView::from).collect())
}

async fn load_breaks(
    conn: &mut PgConnection,
    business_id: BusinessId,
    staff_id: Uuid,
) -> AppResult<Vec<BreakView>> {
    let rows = sqlx::query_as::<_, BreakRow>(
        "SELECT id, weekday, start_local, end_local
         FROM schedule_break
         WHERE business_id = $1 AND staff_id = $2
         ORDER BY weekday, start_local",
    )
    .bind(business_id.as_uuid())
    .bind(staff_id)
    .fetch_all(&mut *conn)
    .await?;
    Ok(rows.into_iter().map(BreakView::from).collect())
}

/// Maps an exclusion-constraint violation (overlap) to a 400 with a clear message.
fn on_overlap(message: &'static str) -> impl FnOnce(sqlx::Error) -> AppError {
    move |error| {
        let overlap = error
            .as_database_error()
            .and_then(|db| db.code())
            .map(|code| code == "23P01")
            .unwrap_or(false);
        if overlap {
            AppError::validation(message)
        } else {
            AppError::from(error)
        }
    }
}

// ---------------------------------------------------------------------------
// Parsing and formatting
// ---------------------------------------------------------------------------

fn check_weekday(weekday: i16) -> AppResult<()> {
    if (0..=6).contains(&weekday) {
        Ok(())
    } else {
        Err(AppError::validation(
            "weekday must be 0 (Monday) to 6 (Sunday)",
        ))
    }
}

fn parse_time(value: &str, field: &str) -> AppResult<Time> {
    Time::parse(value.trim(), TIME_FORMAT)
        .map_err(|_| AppError::validation(format!("{field} must be a local time like 09:30")))
}

/// A local time span within one day: start strictly before end.
fn parse_span(start: &str, end: &str) -> AppResult<(Time, Time)> {
    let start = parse_time(start, "start")?;
    let end = parse_time(end, "end")?;
    if start >= end {
        return Err(AppError::validation("start must be before end"));
    }
    Ok((start, end))
}

pub(crate) fn parse_date(value: &str, field: &str) -> AppResult<Date> {
    Date::parse(value.trim(), DATE_FORMAT)
        .map_err(|_| AppError::validation(format!("{field} must be a date like 2026-10-05")))
}

fn parse_optional_date(value: Option<&str>, field: &str) -> AppResult<Option<Date>> {
    value.map(|text| parse_date(text, field)).transpose()
}

pub(crate) fn parse_instant(value: &str, field: &str) -> AppResult<OffsetDateTime> {
    OffsetDateTime::parse(value.trim(), &Rfc3339)
        .map(|instant| instant.to_offset(UtcOffset::UTC))
        .map_err(|_| {
            AppError::validation(format!(
                "{field} must be an RFC 3339 timestamp like 2026-07-01T09:00:00Z"
            ))
        })
}

fn format_time(value: Time) -> String {
    value.format(TIME_FORMAT).unwrap_or_default()
}

pub(crate) fn format_date(value: Date) -> String {
    value.format(DATE_FORMAT).unwrap_or_default()
}

pub(crate) fn format_utc(value: OffsetDateTime) -> String {
    value
        .to_offset(UtcOffset::UTC)
        .format(&Rfc3339)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_times_need_hh_mm() {
        assert!(parse_time("09:30", "start").is_ok());
        assert!(parse_time(" 23:59 ", "start").is_ok());
        for bad in ["9:30", "24:00", "09:60", "0930", "", "09:30:00", "noon"] {
            assert!(
                parse_time(bad, "start").is_err(),
                "{bad:?} must be rejected"
            );
        }
    }

    #[test]
    fn spans_must_run_forward() {
        assert!(parse_span("09:00", "17:00").is_ok());
        assert!(parse_span("17:00", "09:00").is_err());
        assert!(parse_span("09:00", "09:00").is_err());
    }

    #[test]
    fn dates_and_instants() {
        assert!(parse_date("2026-10-05", "date").is_ok());
        for bad in ["2026-13-01", "2026-10-5", "05.10.2026", ""] {
            assert!(parse_date(bad, "date").is_err(), "{bad:?} must be rejected");
        }

        // An offset is converted to UTC.
        let instant = parse_instant("2099-07-01T02:00:00+02:00", "start_at").unwrap();
        assert_eq!(format_utc(instant), "2099-07-01T00:00:00Z");
        assert!(parse_instant("2099-07-01", "start_at").is_err());
    }

    #[test]
    fn formatting_roundtrips() {
        let time = parse_time("07:05", "t").unwrap();
        assert_eq!(format_time(time), "07:05");
        let date = parse_date("2026-01-09", "d").unwrap();
        assert_eq!(format_date(date), "2026-01-09");
    }

    #[test]
    fn weekdays_are_zero_to_six() {
        assert!(check_weekday(0).is_ok());
        assert!(check_weekday(6).is_ok());
        assert!(check_weekday(7).is_err());
        assert!(check_weekday(-1).is_err());
    }
}
