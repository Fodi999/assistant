//! The public side of the platform: the catalog of approved masters, their
//! profiles, availability, and the booking flow for customers.
//!
//! Who sees what
//! * Anyone (no sign-in): the catalog, a business profile, availability.
//!   Only businesses a platform admin approved AND whose owner published the
//!   profile are visible; the queries repeat what row-level security enforces.
//!   Profiles expose names, prices and durations, never buffers, notice rules,
//!   intake questions, contact data of the team or any client.
//! * A signed-in customer (registered user or guest session): holds and books
//!   in a public business, and reads / cancels / moves ONLY their own
//!   appointments. Ownership is checked before every operation
//!   (`BookingService::ensure_owned`): a foreign appointment is a 404.
//!
//! Customers reuse the master's booking engine, so the same availability rules
//! and the same PostgreSQL double-booking guard apply to them.

use crate::application::access::BusinessAccess;
use crate::application::availability::{AvailabilityQuery, AvailabilityService, AvailabilityView};
use crate::application::booking::{
    AppointmentView, BookingOutcome, BookingService, CancelInput, CreateAppointmentInput,
    CreateHoldInput, RescheduleInput,
};
use crate::application::clients::{
    customer_client_id, parse_contact, upsert_customer_client, ClientContact,
};
use crate::infrastructure::{begin_scoped, DbScope};
use crate::shared::{AppError, AppResult, BusinessId, UserId};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgConnection, PgPool};
use std::collections::HashMap;
use uuid::Uuid;

const DEFAULT_LIMIT: i64 = 20;
const MAX_LIMIT: i64 = 50;

/// SQL condition shared by every public lookup (row-level security enforces
/// the same rule; it is repeated so a policy mistake cannot expose a business).
const PUBLIC_BUSINESS: &str = "moderation_status = 'approved' AND is_published
     AND status = 'active' AND deleted_at IS NULL";

// ---------------------------------------------------------------------------
// Input / output
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct CatalogQuery {
    /// Exact city, case-insensitive.
    pub city: Option<String>,
    /// Part of the name or headline.
    pub q: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct CatalogItem {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub headline: Option<String>,
    pub city: Option<String>,
    pub country: String,
    pub timezone: String,
    pub currency: String,
}

#[derive(Debug, Serialize)]
pub struct PublicStaff {
    pub id: Uuid,
    pub display_name: String,
    pub photo_url: Option<String>,
    pub bio: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PublicVariant {
    pub id: Uuid,
    pub name: Option<Value>,
    pub duration_min: i32,
    pub price_minor: i64,
    /// `fixed` or `from`.
    pub price_type: String,
    pub currency: String,
}

#[derive(Debug, Serialize)]
pub struct PublicServiceItem {
    pub id: Uuid,
    pub category_id: Option<Uuid>,
    pub name: Value,
    pub description: Option<Value>,
    /// Masters who perform it (ids of `staff`).
    pub staff_ids: Vec<Uuid>,
    pub variants: Vec<PublicVariant>,
}

#[derive(Debug, Serialize)]
pub struct PublicCategory {
    pub id: Uuid,
    pub name: Value,
}

#[derive(Debug, Serialize)]
pub struct PortfolioItemView {
    pub id: Uuid,
    pub staff_id: Option<Uuid>,
    pub image_url: String,
    pub caption: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PublicProfile {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub headline: Option<String>,
    pub about: Option<String>,
    pub city: Option<String>,
    pub country: String,
    pub timezone: String,
    pub currency: String,
    pub default_locale: String,
    pub instagram: Option<String>,
    pub staff: Vec<PublicStaff>,
    pub categories: Vec<PublicCategory>,
    pub services: Vec<PublicServiceItem>,
    /// Placeholder until photo upload exists: only items with a stored URL.
    pub portfolio: Vec<PortfolioItemView>,
}

/// A customer's booking request: confirm a hold (`hold_id`) or book directly.
#[derive(Debug, Deserialize)]
pub struct CustomerBookInput {
    pub hold_id: Option<Uuid>,
    pub service_id: Option<Uuid>,
    pub variant_id: Option<Uuid>,
    pub staff_id: Option<Uuid>,
    pub start_at: Option<String>,
    /// `app` (default) or `web`; customers cannot book as `manual`.
    pub source: Option<String>,
    pub client_name: String,
    pub client_phone: Option<String>,
    pub client_email: Option<String>,
    /// Note for the master (up to 500 characters). Not for health information.
    pub note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MyAppointmentsQuery {
    pub status: Option<String>,
}

fn parse_json(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or(Value::Null)
}

fn customer_source(source: Option<&str>) -> AppResult<String> {
    match source.unwrap_or("app") {
        value @ ("app" | "web") => Ok(value.to_string()),
        _ => Err(AppError::validation("source must be app or web")),
    }
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct PublicApi {
    pool: PgPool,
    booking: BookingService,
    availability: AvailabilityService,
}

impl PublicApi {
    pub fn new(pool: PgPool, booking: BookingService, availability: AvailabilityService) -> Self {
        Self {
            pool,
            booking,
            availability,
        }
    }

    // -- anonymous ----------------------------------------------------------

    pub async fn catalog(&self, query: CatalogQuery) -> AppResult<Vec<CatalogItem>> {
        let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
        let offset = query.offset.unwrap_or(0).max(0);
        let city = query
            .city
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let needle = query
            .q
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        if needle.as_ref().map(|v| v.chars().count()).unwrap_or(0) > 100 {
            return Err(AppError::validation("q must be at most 100 characters"));
        }

        let mut tx = begin_scoped(&self.pool, DbScope::anonymous()).await?;
        let rows: Vec<(
            Uuid,
            String,
            String,
            Option<String>,
            Option<String>,
            String,
            String,
            String,
        )> = sqlx::query_as(&format!(
            "SELECT id, slug, name, headline, city, country::text, timezone, currency::text
                 FROM business
                 WHERE {PUBLIC_BUSINESS}
                   AND ($1::text IS NULL OR lower(city) = lower($1))
                   AND ($2::text IS NULL
                        OR position(lower($2) IN lower(name)) > 0
                        OR position(lower($2) IN lower(COALESCE(headline, ''))) > 0)
                 ORDER BY name, id
                 LIMIT $3 OFFSET $4"
        ))
        .bind(city)
        .bind(needle)
        .bind(limit)
        .bind(offset)
        .fetch_all(&mut *tx)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| CatalogItem {
                id: row.0,
                slug: row.1,
                name: row.2,
                headline: row.3,
                city: row.4,
                country: row.5,
                timezone: row.6,
                currency: row.7,
            })
            .collect())
    }

    /// Public profile by business id or slug.
    pub async fn profile(&self, key: &str) -> AppResult<PublicProfile> {
        let id = self.public_business_id(key).await?;
        let scope = DbScope {
            user_id: None,
            business_id: Some(id),
        };
        let mut tx = begin_scoped(&self.pool, scope).await?;
        let business = id.as_uuid();

        let head: (
            Uuid,
            String,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            String,
            String,
            String,
            String,
            Option<String>,
        ) = sqlx::query_as(&format!(
            "SELECT id, slug, name, headline, about, city, country::text, timezone,
                    currency::text, default_locale, instagram
             FROM business WHERE id = $1 AND {PUBLIC_BUSINESS}"
        ))
        .bind(business)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::not_found("Business not found"))?;

        let staff = sqlx::query_as::<_, (Uuid, String, Option<String>, Option<String>)>(
            "SELECT s.id, s.display_name, s.photo_url, s.bio
             FROM staff_member s
             WHERE s.business_id = $1 AND s.is_bookable
               AND EXISTS (SELECT 1 FROM staff_service ss WHERE ss.staff_id = s.id)
             ORDER BY s.sort_order, s.display_name, s.id",
        )
        .bind(business)
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(|row| PublicStaff {
            id: row.0,
            display_name: row.1,
            photo_url: row.2,
            bio: row.3,
        })
        .collect();

        let categories = sqlx::query_as::<_, (Uuid, String)>(
            "SELECT id, name::text FROM service_category
             WHERE business_id = $1 AND deleted_at IS NULL
             ORDER BY sort_order, created_at, id",
        )
        .bind(business)
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(|row| PublicCategory {
            id: row.0,
            name: parse_json(&row.1),
        })
        .collect();

        let services = load_services(&mut tx, *business).await?;

        let portfolio = sqlx::query_as::<_, (Uuid, Option<Uuid>, String, Option<String>)>(
            "SELECT id, staff_id, image_url, caption FROM portfolio_item
             WHERE business_id = $1 AND is_published
             ORDER BY sort_order, created_at, id LIMIT 50",
        )
        .bind(business)
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(|row| PortfolioItemView {
            id: row.0,
            staff_id: row.1,
            image_url: row.2,
            caption: row.3,
        })
        .collect();

        Ok(PublicProfile {
            id: head.0,
            slug: head.1,
            name: head.2,
            headline: head.3,
            about: head.4,
            city: head.5,
            country: head.6,
            timezone: head.7,
            currency: head.8,
            default_locale: head.9,
            instagram: head.10,
            staff,
            categories,
            services,
            portfolio,
        })
    }

    /// Slots a customer may book (online rules; the channel cannot be chosen).
    pub async fn availability(
        &self,
        key: &str,
        mut query: AvailabilityQuery,
    ) -> AppResult<AvailabilityView> {
        let id = self.public_business_id(key).await?;
        query.channel = Some("online".to_string());
        self.availability
            .availability(BusinessAccess::for_public(id), query)
            .await
    }

    // -- signed-in customer -------------------------------------------------

    pub async fn hold(
        &self,
        user: UserId,
        key: &str,
        idempotency_key: &str,
        mut input: CreateHoldInput,
    ) -> AppResult<BookingOutcome> {
        let id = self.public_business_id(key).await?;
        input.source = Some(customer_source(input.source.as_deref())?);
        self.booking
            .create_hold(
                BusinessAccess::for_customer(user, id),
                idempotency_key,
                input,
            )
            .await
    }

    pub async fn release_hold(&self, user: UserId, key: &str, hold_id: Uuid) -> AppResult<()> {
        let (access, client_id) = self.own_context(user, key).await?;
        self.booking
            .ensure_owned(access, hold_id, client_id)
            .await?;
        self.booking.release_hold(access, hold_id).await
    }

    pub async fn book(
        &self,
        user: UserId,
        key: &str,
        idempotency_key: Option<&str>,
        input: CustomerBookInput,
    ) -> AppResult<BookingOutcome> {
        let contact: ClientContact = parse_contact(
            &input.client_name,
            input.client_phone.clone(),
            input.client_email.clone(),
        )?;
        let source = if input.hold_id.is_some() {
            if input.source.is_some() {
                return Err(AppError::validation(
                    "hold_id cannot be combined with service, staff, start or source",
                ));
            }
            None
        } else {
            Some(customer_source(input.source.as_deref())?)
        };

        let id = self.public_business_id(key).await?;
        let access = BusinessAccess::for_customer(user, id);
        if let Some(hold_id) = input.hold_id {
            let client_id = self.client_id_of(access).await?;
            self.booking
                .ensure_owned(access, hold_id, client_id)
                .await?;
        }

        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        let client_id =
            upsert_customer_client(&mut tx, *id.as_uuid(), *user.as_uuid(), &contact).await?;
        tx.commit().await?;

        self.booking
            .create_appointment_for(
                access,
                Some(client_id),
                idempotency_key,
                CreateAppointmentInput {
                    hold_id: input.hold_id,
                    service_id: input.service_id,
                    variant_id: input.variant_id,
                    staff_id: input.staff_id,
                    start_at: input.start_at,
                    source,
                    client_id: None,
                    client_name: contact.full_name,
                    client_phone: contact.phone,
                    note: input.note,
                },
            )
            .await
    }

    /// The customer's own appointments in one business.
    pub async fn my_appointments(
        &self,
        user: UserId,
        key: &str,
        query: MyAppointmentsQuery,
    ) -> AppResult<Vec<AppointmentView>> {
        let (access, client_id) = self.own_context(user, key).await?;
        self.booking
            .list_for_client(access, client_id, query.status.as_deref())
            .await
    }

    pub async fn my_appointment(
        &self,
        user: UserId,
        key: &str,
        id: Uuid,
    ) -> AppResult<AppointmentView> {
        let (access, client_id) = self.own_context(user, key).await?;
        self.booking.ensure_owned(access, id, client_id).await?;
        self.booking.get_appointment(access, id).await
    }

    pub async fn cancel(
        &self,
        user: UserId,
        key: &str,
        id: Uuid,
        input: CancelInput,
    ) -> AppResult<AppointmentView> {
        let (access, client_id) = self.own_context(user, key).await?;
        self.booking.ensure_owned(access, id, client_id).await?;
        self.booking.cancel(access, id, input).await
    }

    pub async fn reschedule(
        &self,
        user: UserId,
        key: &str,
        id: Uuid,
        input: RescheduleInput,
    ) -> AppResult<AppointmentView> {
        let (access, client_id) = self.own_context(user, key).await?;
        self.booking.ensure_owned(access, id, client_id).await?;
        self.booking.reschedule_online(access, id, input).await
    }

    // -- internals ----------------------------------------------------------

    /// The id of a business that is public right now (by id or slug). Anything
    /// else, including a pending or suspended business, is a 404.
    async fn public_business_id(&self, key: &str) -> AppResult<BusinessId> {
        let key = key.trim();
        let mut tx = begin_scoped(&self.pool, DbScope::anonymous()).await?;
        let id: Option<Uuid> = sqlx::query_scalar(&format!(
            "SELECT id FROM business WHERE (id = $1 OR slug = $2) AND {PUBLIC_BUSINESS}"
        ))
        .bind(Uuid::parse_str(key).ok())
        .bind(key.to_lowercase())
        .fetch_optional(&mut *tx)
        .await?;
        id.map(BusinessId::from_uuid)
            .ok_or_else(|| AppError::not_found("Business not found"))
    }

    /// Access and client row for operations on the customer's own
    /// appointments. These stay possible after a business was suspended, so a
    /// customer can still cancel; hence the business id (not only the slug)
    /// is accepted here.
    async fn own_context(
        &self,
        user: UserId,
        key: &str,
    ) -> AppResult<(BusinessAccess, Option<Uuid>)> {
        let key = key.trim();
        let business_id = match Uuid::parse_str(key) {
            Ok(id) => BusinessId::from_uuid(id),
            Err(_) => self.public_business_id(key).await?,
        };
        let access = BusinessAccess::for_customer(user, business_id);
        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        let exists: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM business WHERE id = $1 AND deleted_at IS NULL")
                .bind(business_id.as_uuid())
                .fetch_optional(&mut *tx)
                .await?;
        if exists.is_none() {
            return Err(AppError::not_found("Business not found"));
        }
        let client_id =
            customer_client_id(&mut tx, *business_id.as_uuid(), *user.as_uuid()).await?;
        Ok((access, client_id))
    }

    async fn client_id_of(&self, access: BusinessAccess) -> AppResult<Option<Uuid>> {
        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        customer_client_id(
            &mut tx,
            *access.business_id.as_uuid(),
            *access.user_id.as_uuid(),
        )
        .await
    }
}

/// Bookable services with their variants and the masters who perform them.
async fn load_services(
    conn: &mut PgConnection,
    business: Uuid,
) -> AppResult<Vec<PublicServiceItem>> {
    let services = sqlx::query_as::<_, (Uuid, Option<Uuid>, String, Option<String>)>(
        "SELECT id, category_id, name::text, description::text FROM service
         WHERE business_id = $1 AND is_active AND is_online_bookable AND deleted_at IS NULL
         ORDER BY sort_order, created_at, id",
    )
    .bind(business)
    .fetch_all(&mut *conn)
    .await?;

    let variants = sqlx::query_as::<_, (Uuid, Uuid, Option<String>, i32, i64, String, String)>(
        "SELECT id, service_id, name::text, duration_min, price_minor, price_type, currency::text
         FROM service_variant
         WHERE business_id = $1 AND is_active AND deleted_at IS NULL
         ORDER BY service_id, sort_order, created_at, id",
    )
    .bind(business)
    .fetch_all(&mut *conn)
    .await?;
    let mut variants_by_service: HashMap<Uuid, Vec<PublicVariant>> = HashMap::new();
    for row in variants {
        variants_by_service
            .entry(row.1)
            .or_default()
            .push(PublicVariant {
                id: row.0,
                name: row.2.as_deref().map(parse_json),
                duration_min: row.3,
                price_minor: row.4,
                price_type: row.5,
                currency: row.6,
            });
    }

    let staff = sqlx::query_as::<_, (Uuid, Uuid)>(
        "SELECT ss.service_id, ss.staff_id
         FROM staff_service ss
         JOIN staff_member s ON s.id = ss.staff_id AND s.business_id = ss.business_id
         WHERE ss.business_id = $1 AND s.is_bookable
         ORDER BY s.sort_order, s.display_name, s.id",
    )
    .bind(business)
    .fetch_all(&mut *conn)
    .await?;
    let mut staff_by_service: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for (service_id, staff_id) in staff {
        staff_by_service
            .entry(service_id)
            .or_default()
            .push(staff_id);
    }

    Ok(services
        .into_iter()
        .filter_map(|row| {
            let variants = variants_by_service.remove(&row.0)?;
            Some(PublicServiceItem {
                staff_ids: staff_by_service.remove(&row.0).unwrap_or_default(),
                id: row.0,
                category_id: row.1,
                name: parse_json(&row.2),
                description: row.3.as_deref().map(parse_json),
                variants,
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn customers_book_only_as_app_or_web() {
        assert_eq!(customer_source(None).unwrap(), "app");
        assert_eq!(customer_source(Some("web")).unwrap(), "web");
        assert!(customer_source(Some("manual")).is_err());
        assert!(customer_source(Some("x")).is_err());
    }

    #[test]
    fn stored_json_text_is_parsed() {
        assert_eq!(parse_json(r#"{"pl":"Klasyczne"}"#)["pl"], "Klasyczne");
        assert_eq!(parse_json("not json"), Value::Null);
    }
}
