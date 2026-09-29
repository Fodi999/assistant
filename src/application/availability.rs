//! Availability: the bookable start times of a service, computed on the server
//! only (clients never derive slots themselves).
//!
//! A start time is offered when, for a master who performs the service:
//! - it lies on the service's slot grid (minutes from local midnight),
//! - the whole service (variant duration) fits inside a working interval, where
//!   the working interval is the weekly pattern of that local date, or the
//!   custom hours of a `schedule_exception` (a day off gives nothing),
//! - the service plus its `buffer_after_min` does not touch a weekly break
//!   (breaks apply to the weekly pattern only, not to custom hours), time off
//!   or - from the booking stage on - another appointment or hold,
//! - it is not earlier than now + `min_notice_min` nor later than
//!   now + `max_advance_days`.
//!
//! Local wall-clock times are converted to UTC by PostgreSQL with the business
//! IANA zone, so daylight saving changes are handled by the tz database.

use crate::application::access::BusinessAccess;
use crate::application::schedule::{format_date, format_utc, parse_date};
use crate::infrastructure::begin_scoped;
use crate::shared::{AppError, AppResult, Clock};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use std::collections::BTreeMap;
use time::{Date, Duration, OffsetDateTime};
use uuid::Uuid;

/// Longest date range one request may ask for.
const MAX_RANGE_DAYS: i64 = 14;

// ---------------------------------------------------------------------------
// Input / output
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct AvailabilityQuery {
    pub service_id: Uuid,
    pub variant_id: Uuid,
    /// One master; all masters who perform the service when absent.
    pub staff_id: Option<Uuid>,
    /// First local date, `YYYY-MM-DD` (business time zone).
    pub from: String,
    /// Last local date (inclusive); defaults to `from`. At most 14 days.
    pub to: Option<String>,
    /// `online` (default; the service must be bookable online) or `manual`
    /// (a member booking on behalf of a client).
    pub channel: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SlotView {
    /// Local date of the slot in the business time zone.
    pub date: String,
    pub staff_id: Uuid,
    /// UTC, RFC 3339.
    pub start_at: String,
    /// End of the service itself (the buffer after it is not included).
    pub end_at: String,
}

#[derive(Debug, Serialize)]
pub struct AvailabilityView {
    pub timezone: String,
    pub service_id: Uuid,
    pub variant_id: Uuid,
    pub duration_min: i32,
    pub buffer_after_min: i32,
    pub slots: Vec<SlotView>,
}

// ---------------------------------------------------------------------------
// Pure slot arithmetic (unit-tested without a database)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Interval {
    pub start: OffsetDateTime,
    pub end: OffsetDateTime,
}

impl Interval {
    fn overlaps(&self, other: &Interval) -> bool {
        self.start < other.end && self.end > other.start
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SlotRules {
    pub duration: Duration,
    pub buffer: Duration,
    pub step: Duration,
    pub earliest: OffsetDateTime,
    pub latest: OffsetDateTime,
}

/// Start times for one master on one local day.
///
/// `day_start` is local midnight as an instant: the slot grid counts from it.
/// `work` are the working intervals of the day, `busy` everything that blocks
/// (breaks, time off, appointments).
pub fn slots_for_day(
    day_start: OffsetDateTime,
    work: &[Interval],
    busy: &[Interval],
    rules: &SlotRules,
) -> Vec<OffsetDateTime> {
    let step_ns = rules.step.whole_nanoseconds().max(1);
    let mut starts = Vec::new();
    for interval in work {
        let lower = interval.start.max(rules.earliest);
        // First grid instant that is not before `lower`.
        let offset_ns = (lower - day_start).whole_nanoseconds();
        let steps = if offset_ns <= 0 {
            0
        } else {
            (offset_ns + step_ns - 1) / step_ns
        };
        let mut start = day_start + Duration::nanoseconds_i128(steps * step_ns);
        while start + rules.duration <= interval.end && start <= rules.latest {
            let blocked = Interval {
                start,
                end: start + rules.duration + rules.buffer,
            };
            if !busy.iter().any(|item| item.overlaps(&blocked)) {
                starts.push(start);
            }
            start += rules.step;
        }
    }
    starts
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct AvailabilityService {
    pool: PgPool,
    clock: Clock,
}

#[derive(sqlx::FromRow)]
struct ServiceRules {
    is_online_bookable: bool,
    booking_step_minutes: i32,
    buffer_after_min: i32,
    min_notice_min: i32,
    max_advance_days: i32,
    duration_min: i32,
}

#[derive(sqlx::FromRow)]
struct WindowRow {
    staff_id: Uuid,
    day: Date,
    day_start: OffsetDateTime,
    kind: String,
    start_at: OffsetDateTime,
    end_at: OffsetDateTime,
}

#[derive(Default)]
struct DayPlan {
    day_start: Option<OffsetDateTime>,
    work: Vec<Interval>,
    breaks: Vec<Interval>,
}

impl AvailabilityService {
    pub fn new(pool: PgPool, clock: Clock) -> Self {
        Self { pool, clock }
    }

    pub async fn availability(
        &self,
        access: BusinessAccess,
        query: AvailabilityQuery,
    ) -> AppResult<AvailabilityView> {
        let from = parse_date(&query.from, "from")?;
        let to = match query.to.as_deref() {
            Some(value) => parse_date(value, "to")?,
            None => from,
        };
        if to < from {
            return Err(AppError::validation("to must not be before from"));
        }
        if (to - from).whole_days() + 1 > MAX_RANGE_DAYS {
            return Err(AppError::validation(format!(
                "At most {MAX_RANGE_DAYS} days per request"
            )));
        }
        let online = match query.channel.as_deref() {
            None | Some("online") => true,
            Some("manual") => false,
            Some(_) => {
                return Err(AppError::validation("channel must be 'online' or 'manual'"));
            }
        };

        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        let rules = sqlx::query_as::<_, ServiceRules>(
            "SELECT s.is_online_bookable, s.booking_step_minutes, s.buffer_after_min,
                    s.min_notice_min, s.max_advance_days, v.duration_min
             FROM service s
             JOIN service_variant v ON v.service_id = s.id AND v.business_id = s.business_id
             WHERE s.id = $1 AND v.id = $2 AND s.business_id = $3
               AND s.is_active AND s.deleted_at IS NULL
               AND v.is_active AND v.deleted_at IS NULL",
        )
        .bind(query.service_id)
        .bind(query.variant_id)
        .bind(access.business_id.as_uuid())
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::not_found("Service or variant not found"))?;
        if online && !rules.is_online_bookable {
            return Err(AppError::validation("This service cannot be booked online"));
        }

        let mut staff_ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT st.id
             FROM staff_service ss
             JOIN staff_member st ON st.id = ss.staff_id AND st.business_id = ss.business_id
             WHERE ss.service_id = $1 AND ss.business_id = $2 AND st.is_bookable
             ORDER BY st.sort_order, st.display_name, st.id",
        )
        .bind(query.service_id)
        .bind(access.business_id.as_uuid())
        .fetch_all(&mut *tx)
        .await?;
        if let Some(wanted) = query.staff_id {
            if !staff_ids.contains(&wanted) {
                return Err(AppError::not_found(
                    "This staff member does not perform the service",
                ));
            }
            staff_ids = vec![wanted];
        }

        let (timezone, range_start, range_end): (String, OffsetDateTime, OffsetDateTime) =
            sqlx::query_as(
                "SELECT timezone,
                        ($2::date)::timestamp AT TIME ZONE timezone,
                        (($3::date) + 1)::timestamp AT TIME ZONE timezone
                 FROM business WHERE id = $1",
            )
            .bind(access.business_id.as_uuid())
            .bind(from)
            .bind(to)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| AppError::not_found("Business not found"))?;

        let mut view = AvailabilityView {
            timezone,
            service_id: query.service_id,
            variant_id: query.variant_id,
            duration_min: rules.duration_min,
            buffer_after_min: rules.buffer_after_min,
            slots: Vec::new(),
        };
        if staff_ids.is_empty() {
            return Ok(view);
        }

        let windows = load_windows(&mut tx, access, &staff_ids, from, to).await?;
        let busy = load_busy(
            &mut tx,
            access,
            &staff_ids,
            range_start - Duration::days(1),
            range_end + Duration::days(1),
        )
        .await?;

        let now = self.clock.now();
        let slot_rules = SlotRules {
            duration: Duration::minutes(rules.duration_min.into()),
            buffer: Duration::minutes(rules.buffer_after_min.into()),
            step: Duration::minutes(rules.booking_step_minutes.into()),
            earliest: now + Duration::minutes(rules.min_notice_min.into()),
            latest: now + Duration::days(rules.max_advance_days.into()),
        };

        let mut plans: BTreeMap<(Uuid, Date), DayPlan> = BTreeMap::new();
        for row in windows {
            let plan = plans.entry((row.staff_id, row.day)).or_default();
            plan.day_start = Some(row.day_start);
            let interval = Interval {
                start: row.start_at,
                end: row.end_at,
            };
            match row.kind.as_str() {
                "work" => plan.work.push(interval),
                _ => plan.breaks.push(interval),
            }
        }
        for ((staff_id, day), mut plan) in plans {
            let Some(day_start) = plan.day_start else {
                continue;
            };
            plan.work.sort_by_key(|interval| interval.start);
            let mut blockers: Vec<Interval> = plan.breaks;
            blockers.extend(
                busy.iter()
                    .filter(|(id, _)| *id == staff_id)
                    .map(|(_, interval)| *interval),
            );
            for start in slots_for_day(day_start, &plan.work, &blockers, &slot_rules) {
                view.slots.push(SlotView {
                    date: format_date(day),
                    staff_id,
                    start_at: format_utc(start),
                    end_at: format_utc(start + slot_rules.duration),
                });
            }
        }
        view.slots.sort_by(|a, b| {
            a.start_at
                .cmp(&b.start_at)
                .then_with(|| a.staff_id.cmp(&b.staff_id))
        });
        Ok(view)
    }
}

/// Working and break intervals per staff and local day, already in UTC.
async fn load_windows(
    conn: &mut PgConnection,
    access: BusinessAccess,
    staff_ids: &[Uuid],
    from: Date,
    to: Date,
) -> AppResult<Vec<WindowRow>> {
    let rows = sqlx::query_as::<_, WindowRow>(
        "WITH biz AS (SELECT timezone FROM business WHERE id = $1),
         days AS (SELECT gs::date AS day FROM generate_series($3::date, $4::date, interval '1 day') gs),
         grid AS (
             SELECT s.id AS staff_id, d.day,
                    d.day::timestamp AT TIME ZONE biz.timezone AS day_start,
                    biz.timezone AS tz,
                    (extract(isodow FROM d.day)::int - 1) AS weekday
             FROM staff_member s CROSS JOIN days d CROSS JOIN biz
             WHERE s.business_id = $1 AND s.id = ANY($2)
         ),
         ex AS (
             SELECT staff_id, date_local, kind, start_local, end_local
             FROM schedule_exception
             WHERE business_id = $1 AND staff_id = ANY($2) AND date_local BETWEEN $3 AND $4
         )
         SELECT g.staff_id, g.day, g.day_start, 'work'::text AS kind,
                (g.day + w.start_local) AT TIME ZONE g.tz AS start_at,
                (g.day + w.end_local) AT TIME ZONE g.tz AS end_at
         FROM grid g
         JOIN working_schedule w
           ON w.business_id = $1 AND w.staff_id = g.staff_id AND w.weekday = g.weekday
          AND (w.valid_from IS NULL OR w.valid_from <= g.day)
          AND (w.valid_to IS NULL OR w.valid_to >= g.day)
         WHERE NOT EXISTS (SELECT 1 FROM ex WHERE ex.staff_id = g.staff_id AND ex.date_local = g.day)
         UNION ALL
         SELECT g.staff_id, g.day, g.day_start, 'work'::text,
                (g.day + e.start_local) AT TIME ZONE g.tz,
                (g.day + e.end_local) AT TIME ZONE g.tz
         FROM grid g
         JOIN ex e ON e.staff_id = g.staff_id AND e.date_local = g.day AND e.kind = 'custom_hours'
         UNION ALL
         SELECT g.staff_id, g.day, g.day_start, 'break'::text,
                (g.day + b.start_local) AT TIME ZONE g.tz,
                (g.day + b.end_local) AT TIME ZONE g.tz
         FROM grid g
         JOIN schedule_break b
           ON b.business_id = $1 AND b.staff_id = g.staff_id AND b.weekday = g.weekday
         WHERE NOT EXISTS (SELECT 1 FROM ex WHERE ex.staff_id = g.staff_id AND ex.date_local = g.day)",
    )
    .bind(access.business_id.as_uuid())
    .bind(staff_ids)
    .bind(from)
    .bind(to)
    .fetch_all(&mut *conn)
    .await?;
    Ok(rows)
}

/// Everything that blocks a master's calendar in `[from, to)`, as UTC intervals.
/// Time off today; appointments and holds are added by the booking stage.
async fn load_busy(
    conn: &mut PgConnection,
    access: BusinessAccess,
    staff_ids: &[Uuid],
    from: OffsetDateTime,
    to: OffsetDateTime,
) -> AppResult<Vec<(Uuid, Interval)>> {
    let rows: Vec<(Uuid, OffsetDateTime, OffsetDateTime)> = sqlx::query_as(
        "SELECT staff_id, start_at, end_at
         FROM time_off
         WHERE business_id = $1 AND staff_id = ANY($2) AND end_at > $3 AND start_at < $4",
    )
    .bind(access.business_id.as_uuid())
    .bind(staff_ids)
    .bind(from)
    .bind(to)
    .fetch_all(&mut *conn)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(staff_id, start, end)| (staff_id, Interval { start, end }))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    fn at(hour: i64, minute: i64) -> OffsetDateTime {
        datetime!(2027-03-22 00:00 UTC) + Duration::minutes(hour * 60 + minute)
    }

    fn interval(a: (i64, i64), b: (i64, i64)) -> Interval {
        Interval {
            start: at(a.0, a.1),
            end: at(b.0, b.1),
        }
    }

    fn rules(duration: i64, buffer: i64, step: i64) -> SlotRules {
        SlotRules {
            duration: Duration::minutes(duration),
            buffer: Duration::minutes(buffer),
            step: Duration::minutes(step),
            earliest: datetime!(2027-03-01 00:00 UTC),
            latest: datetime!(2027-12-31 00:00 UTC),
        }
    }

    fn hours(starts: Vec<OffsetDateTime>) -> Vec<(i64, i64)> {
        starts
            .into_iter()
            .map(|start| {
                let minutes = (start - at(0, 0)).whole_minutes();
                (minutes / 60, minutes % 60)
            })
            .collect()
    }

    #[test]
    fn service_must_fit_the_working_interval() {
        let work = [interval((9, 0), (11, 0))];
        let got = slots_for_day(at(0, 0), &work, &[], &rules(60, 0, 30));
        assert_eq!(hours(got), [(9, 0), (9, 30), (10, 0)]);
    }

    #[test]
    fn grid_counts_from_midnight_not_from_the_interval_start() {
        let work = [interval((9, 10), (11, 0))];
        let got = slots_for_day(at(0, 0), &work, &[], &rules(30, 0, 30));
        assert_eq!(hours(got), [(9, 30), (10, 0), (10, 30)]);
    }

    #[test]
    fn busy_time_and_buffer_block_slots() {
        let work = [interval((9, 0), (13, 0))];
        let busy = [interval((12, 0), (12, 30))];
        // 60 min, no buffer: 11:00 is the last start before the break.
        let got = slots_for_day(at(0, 0), &work, &busy, &rules(60, 0, 30));
        assert_eq!(hours(got), [(9, 0), (9, 30), (10, 0), (10, 30), (11, 0)]);
        // 60 min + 30 min buffer: the buffer would run into the break at 10:30.
        let got = slots_for_day(at(0, 0), &work, &busy, &rules(60, 30, 30));
        assert_eq!(hours(got), [(9, 0), (9, 30), (10, 0)]);
    }

    #[test]
    fn buffer_may_run_past_the_end_of_the_shift() {
        let work = [interval((9, 0), (10, 0))];
        let got = slots_for_day(at(0, 0), &work, &[], &rules(60, 30, 30));
        assert_eq!(hours(got), [(9, 0)]);
    }

    #[test]
    fn split_shift_gives_slots_in_both_parts() {
        let work = [interval((9, 0), (10, 0)), interval((14, 0), (15, 0))];
        let got = slots_for_day(at(0, 0), &work, &[], &rules(60, 0, 60));
        assert_eq!(hours(got), [(9, 0), (14, 0)]);
    }

    #[test]
    fn notice_and_horizon_cut_the_ends() {
        let work = [interval((9, 0), (13, 0))];
        let mut limits = rules(60, 0, 30);
        limits.earliest = at(10, 10);
        limits.latest = at(11, 30);
        let got = slots_for_day(at(0, 0), &work, &[], &limits);
        assert_eq!(hours(got), [(10, 30), (11, 0), (11, 30)]);
    }

    #[test]
    fn touching_intervals_do_not_block() {
        let work = [interval((9, 0), (12, 0))];
        let busy = [interval((10, 0), (11, 0))];
        let got = slots_for_day(at(0, 0), &work, &busy, &rules(60, 0, 60));
        assert_eq!(hours(got), [(9, 0), (11, 0)]);
    }
}
