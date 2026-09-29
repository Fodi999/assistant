//! Service catalog: categories, services, variants (price + duration) and the
//! staff who perform each service. Reading is open to every active member;
//! changes are for owners and managers (PRODUCT_SPEC §17.1).
//!
//! Money is an integer in minor units (grosze). Names are i18n objects such as
//! `{"pl": "Klasyczne", "en": "Classic"}`.

use crate::application::access::{BusinessAccess, Role};
use crate::infrastructure::begin_scoped;
use crate::shared::{AppError, AppResult, BusinessId, Clock};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};
use sqlx::{PgConnection, PgPool};
use std::collections::HashMap;
use time::OffsetDateTime;
use uuid::Uuid;

const LOCALES: [&str; 4] = ["pl", "en", "ru", "uk"];
const BOOKING_STEPS: [i32; 6] = [5, 10, 15, 20, 30, 60];
const WRITERS: [Role; 2] = [Role::Owner, Role::Manager];

const SERVICE_COLS: &str = "id, category_id, name::text AS name, description::text AS description,
    is_active, is_online_bookable, booking_step_minutes, buffer_after_min, min_notice_min,
    max_advance_days, intake_questions::text AS intake_questions, sort_order, version";

const VARIANT_COLS: &str = "id, service_id, name::text AS name, duration_min, price_minor,
    price_type, currency::text AS currency, is_active, sort_order, version";

// ---------------------------------------------------------------------------
// Input
// ---------------------------------------------------------------------------

/// Distinguishes "field absent" (`None`) from "field set to null" (`Some(None)`).
fn double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

#[derive(Debug, Deserialize)]
pub struct CreateCategoryInput {
    pub name: Value,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCategoryInput {
    pub name: Option<Value>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct CreateServiceInput {
    pub category_id: Option<Uuid>,
    pub name: Value,
    pub description: Option<Value>,
    pub is_active: Option<bool>,
    pub is_online_bookable: Option<bool>,
    pub booking_step_minutes: Option<i32>,
    pub buffer_after_min: Option<i32>,
    pub min_notice_min: Option<i32>,
    pub max_advance_days: Option<i32>,
    pub intake_questions: Option<Value>,
    pub sort_order: Option<i32>,
    /// Variants created together with the service.
    pub variants: Option<Vec<CreateVariantInput>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateServiceInput {
    /// `null` moves the service out of its category.
    #[serde(default, deserialize_with = "double_option")]
    pub category_id: Option<Option<Uuid>>,
    pub name: Option<Value>,
    /// `null` clears the description.
    #[serde(default, deserialize_with = "double_option")]
    pub description: Option<Option<Value>>,
    pub is_active: Option<bool>,
    pub is_online_bookable: Option<bool>,
    pub booking_step_minutes: Option<i32>,
    pub buffer_after_min: Option<i32>,
    pub min_notice_min: Option<i32>,
    pub max_advance_days: Option<i32>,
    pub intake_questions: Option<Value>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct CreateVariantInput {
    pub name: Option<Value>,
    pub duration_min: i32,
    /// Price in minor units (grosze): 25000 = 250.00 PLN.
    pub price_minor: i64,
    /// `fixed` (default) or `from` ("od 250 zł").
    pub price_type: Option<String>,
    /// Must equal the business currency when given.
    pub currency: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateVariantInput {
    #[serde(default, deserialize_with = "double_option")]
    pub name: Option<Option<Value>>,
    pub duration_min: Option<i32>,
    pub price_minor: Option<i64>,
    pub price_type: Option<String>,
    pub is_active: Option<bool>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct SetServiceStaffInput {
    pub staff_ids: Vec<Uuid>,
}

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct CategoryView {
    pub id: Uuid,
    pub name: Value,
    pub sort_order: i32,
    pub version: i32,
}

#[derive(Debug, Serialize)]
pub struct VariantView {
    pub id: Uuid,
    pub service_id: Uuid,
    pub name: Option<Value>,
    pub duration_min: i32,
    pub price_minor: i64,
    pub price_type: String,
    pub currency: String,
    pub is_active: bool,
    pub sort_order: i32,
    pub version: i32,
}

#[derive(Debug, Serialize)]
pub struct ServiceView {
    pub id: Uuid,
    pub category_id: Option<Uuid>,
    pub name: Value,
    pub description: Option<Value>,
    pub is_active: bool,
    pub is_online_bookable: bool,
    pub booking_step_minutes: i32,
    pub buffer_after_min: i32,
    pub min_notice_min: i32,
    pub max_advance_days: i32,
    pub intake_questions: Value,
    pub sort_order: i32,
    pub version: i32,
    pub variants: Vec<VariantView>,
    /// Staff members who perform this service.
    pub staff_ids: Vec<Uuid>,
}

#[derive(sqlx::FromRow)]
struct CategoryRow {
    id: Uuid,
    name: String,
    sort_order: i32,
    version: i32,
}

#[derive(sqlx::FromRow)]
struct ServiceRow {
    id: Uuid,
    category_id: Option<Uuid>,
    name: String,
    description: Option<String>,
    is_active: bool,
    is_online_bookable: bool,
    booking_step_minutes: i32,
    buffer_after_min: i32,
    min_notice_min: i32,
    max_advance_days: i32,
    intake_questions: String,
    sort_order: i32,
    version: i32,
}

#[derive(sqlx::FromRow)]
struct VariantRow {
    id: Uuid,
    service_id: Uuid,
    name: Option<String>,
    duration_min: i32,
    price_minor: i64,
    price_type: String,
    currency: String,
    is_active: bool,
    sort_order: i32,
    version: i32,
}

fn parse_json(text: &str) -> AppResult<Value> {
    serde_json::from_str(text)
        .map_err(|e| AppError::internal(format!("Invalid JSON in database: {e}")))
}

fn parse_json_opt(text: Option<&str>) -> AppResult<Option<Value>> {
    text.map(parse_json).transpose()
}

impl CategoryRow {
    fn into_view(self) -> AppResult<CategoryView> {
        Ok(CategoryView {
            id: self.id,
            name: parse_json(&self.name)?,
            sort_order: self.sort_order,
            version: self.version,
        })
    }
}

impl VariantRow {
    fn into_view(self) -> AppResult<VariantView> {
        Ok(VariantView {
            id: self.id,
            service_id: self.service_id,
            name: parse_json_opt(self.name.as_deref())?,
            duration_min: self.duration_min,
            price_minor: self.price_minor,
            price_type: self.price_type,
            currency: self.currency,
            is_active: self.is_active,
            sort_order: self.sort_order,
            version: self.version,
        })
    }
}

impl ServiceRow {
    fn into_view(self, variants: Vec<VariantView>, staff_ids: Vec<Uuid>) -> AppResult<ServiceView> {
        Ok(ServiceView {
            id: self.id,
            category_id: self.category_id,
            name: parse_json(&self.name)?,
            description: parse_json_opt(self.description.as_deref())?,
            is_active: self.is_active,
            is_online_bookable: self.is_online_bookable,
            booking_step_minutes: self.booking_step_minutes,
            buffer_after_min: self.buffer_after_min,
            min_notice_min: self.min_notice_min,
            max_advance_days: self.max_advance_days,
            intake_questions: parse_json(&self.intake_questions)?,
            sort_order: self.sort_order,
            version: self.version,
            variants,
            staff_ids,
        })
    }
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct CatalogService {
    pool: PgPool,
    clock: Clock,
}

impl CatalogService {
    pub fn new(pool: PgPool, clock: Clock) -> Self {
        Self { pool, clock }
    }

    // -- categories ---------------------------------------------------------

    pub async fn list_categories(&self, access: BusinessAccess) -> AppResult<Vec<CategoryView>> {
        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        let rows = sqlx::query_as::<_, CategoryRow>(
            "SELECT id, name::text AS name, sort_order, version
             FROM service_category
             WHERE business_id = $1 AND deleted_at IS NULL
             ORDER BY sort_order, created_at",
        )
        .bind(access.business_id.as_uuid())
        .fetch_all(&mut *tx)
        .await?;
        rows.into_iter().map(CategoryRow::into_view).collect()
    }

    pub async fn create_category(
        &self,
        access: BusinessAccess,
        input: CreateCategoryInput,
    ) -> AppResult<CategoryView> {
        access.require(&WRITERS)?;
        let name = validate_i18n(&input.name, 120, "name")?;

        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        let row = sqlx::query_as::<_, CategoryRow>(
            "INSERT INTO service_category (business_id, name, sort_order)
             VALUES ($1, $2::text::jsonb, $3)
             RETURNING id, name::text AS name, sort_order, version",
        )
        .bind(access.business_id.as_uuid())
        .bind(name.to_string())
        .bind(input.sort_order.unwrap_or(0))
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        row.into_view()
    }

    pub async fn update_category(
        &self,
        access: BusinessAccess,
        category_id: Uuid,
        input: UpdateCategoryInput,
    ) -> AppResult<CategoryView> {
        access.require(&WRITERS)?;
        let name = match &input.name {
            Some(value) => Some(validate_i18n(value, 120, "name")?.to_string()),
            None => None,
        };

        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        let row = sqlx::query_as::<_, CategoryRow>(
            "UPDATE service_category
             SET name = COALESCE($3::text::jsonb, name), sort_order = COALESCE($4, sort_order)
             WHERE id = $1 AND business_id = $2 AND deleted_at IS NULL
             RETURNING id, name::text AS name, sort_order, version",
        )
        .bind(category_id)
        .bind(access.business_id.as_uuid())
        .bind(name)
        .bind(input.sort_order)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::not_found("Category not found"))?;
        tx.commit().await?;
        row.into_view()
    }

    /// Soft-deletes the category; its services stay, without a category.
    pub async fn delete_category(
        &self,
        access: BusinessAccess,
        category_id: Uuid,
    ) -> AppResult<()> {
        access.require(&WRITERS)?;
        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        let deleted: Option<Uuid> = sqlx::query_scalar(
            "UPDATE service_category SET deleted_at = now()
             WHERE id = $1 AND business_id = $2 AND deleted_at IS NULL
             RETURNING id",
        )
        .bind(category_id)
        .bind(access.business_id.as_uuid())
        .fetch_optional(&mut *tx)
        .await?;
        if deleted.is_none() {
            return Err(AppError::not_found("Category not found"));
        }
        sqlx::query(
            "UPDATE service SET category_id = NULL WHERE category_id = $1 AND business_id = $2",
        )
        .bind(category_id)
        .bind(access.business_id.as_uuid())
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    // -- services -----------------------------------------------------------

    pub async fn list_services(&self, access: BusinessAccess) -> AppResult<Vec<ServiceView>> {
        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        load_services(&mut tx, access.business_id, None).await
    }

    pub async fn get_service(
        &self,
        access: BusinessAccess,
        service_id: Uuid,
    ) -> AppResult<ServiceView> {
        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        load_one_service(&mut tx, access.business_id, service_id).await
    }

    pub async fn create_service(
        &self,
        access: BusinessAccess,
        input: CreateServiceInput,
    ) -> AppResult<ServiceView> {
        access.require(&WRITERS)?;
        let name = validate_i18n(&input.name, 255, "name")?.to_string();
        let description = match &input.description {
            Some(value) => Some(validate_i18n(value, 2000, "description")?.to_string()),
            None => None,
        };
        let step = input.booking_step_minutes.unwrap_or(15);
        check_step(step)?;
        let buffer = input.buffer_after_min.unwrap_or(0);
        check_range(buffer.into(), 0, 240, "buffer_after_min")?;
        let notice = input.min_notice_min.unwrap_or(120);
        check_range(notice.into(), 0, 43_200, "min_notice_min")?;
        let advance = input.max_advance_days.unwrap_or(90);
        check_range(advance.into(), 1, 365, "max_advance_days")?;
        let intake = match &input.intake_questions {
            Some(value) => validate_intake(value)?,
            None => "[]".to_string(),
        };

        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        if let Some(category_id) = input.category_id {
            ensure_category(&mut tx, access.business_id, category_id).await?;
        }
        let service_id: Uuid = sqlx::query_scalar(
            "INSERT INTO service (business_id, category_id, name, description, is_active,
                                  is_online_bookable, booking_step_minutes, buffer_after_min,
                                  min_notice_min, max_advance_days, intake_questions, sort_order)
             VALUES ($1, $2, $3::text::jsonb, $4::text::jsonb, $5, $6, $7, $8, $9, $10,
                     $11::text::jsonb, $12)
             RETURNING id",
        )
        .bind(access.business_id.as_uuid())
        .bind(input.category_id)
        .bind(&name)
        .bind(&description)
        .bind(input.is_active.unwrap_or(true))
        .bind(input.is_online_bookable.unwrap_or(true))
        .bind(step)
        .bind(buffer)
        .bind(notice)
        .bind(advance)
        .bind(&intake)
        .bind(input.sort_order.unwrap_or(0))
        .fetch_one(&mut *tx)
        .await?;

        if let Some(variants) = input.variants {
            if variants.len() > 30 {
                return Err(AppError::validation(
                    "A service can have at most 30 variants",
                ));
            }
            let currency = business_currency(&mut tx, access.business_id).await?;
            for variant in variants {
                insert_variant(&mut tx, access.business_id, service_id, &currency, variant).await?;
            }
        }

        let view = load_one_service(&mut tx, access.business_id, service_id).await?;
        tx.commit().await?;
        Ok(view)
    }

    pub async fn update_service(
        &self,
        access: BusinessAccess,
        service_id: Uuid,
        input: UpdateServiceInput,
    ) -> AppResult<ServiceView> {
        access.require(&WRITERS)?;
        let name = match &input.name {
            Some(value) => Some(validate_i18n(value, 255, "name")?.to_string()),
            None => None,
        };
        let set_description = input.description.is_some();
        let description = match &input.description {
            Some(Some(value)) => Some(validate_i18n(value, 2000, "description")?.to_string()),
            _ => None,
        };
        if let Some(step) = input.booking_step_minutes {
            check_step(step)?;
        }
        if let Some(value) = input.buffer_after_min {
            check_range(value.into(), 0, 240, "buffer_after_min")?;
        }
        if let Some(value) = input.min_notice_min {
            check_range(value.into(), 0, 43_200, "min_notice_min")?;
        }
        if let Some(value) = input.max_advance_days {
            check_range(value.into(), 1, 365, "max_advance_days")?;
        }
        let intake = match &input.intake_questions {
            Some(value) => Some(validate_intake(value)?),
            None => None,
        };
        let set_category = input.category_id.is_some();
        let category_id: Option<Uuid> = input.category_id.flatten();

        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        if let Some(category_id) = category_id {
            ensure_category(&mut tx, access.business_id, category_id).await?;
        }
        let updated: Option<Uuid> = sqlx::query_scalar(
            "UPDATE service SET
                 category_id          = CASE WHEN $3 THEN $4::uuid ELSE category_id END,
                 name                 = COALESCE($5::text::jsonb, name),
                 description          = CASE WHEN $6 THEN $7::text::jsonb ELSE description END,
                 is_active            = COALESCE($8, is_active),
                 is_online_bookable   = COALESCE($9, is_online_bookable),
                 booking_step_minutes = COALESCE($10, booking_step_minutes),
                 buffer_after_min     = COALESCE($11, buffer_after_min),
                 min_notice_min       = COALESCE($12, min_notice_min),
                 max_advance_days     = COALESCE($13, max_advance_days),
                 intake_questions     = COALESCE($14::text::jsonb, intake_questions),
                 sort_order           = COALESCE($15, sort_order)
             WHERE id = $1 AND business_id = $2 AND deleted_at IS NULL
             RETURNING id",
        )
        .bind(service_id)
        .bind(access.business_id.as_uuid())
        .bind(set_category)
        .bind(category_id)
        .bind(name)
        .bind(set_description)
        .bind(description)
        .bind(input.is_active)
        .bind(input.is_online_bookable)
        .bind(input.booking_step_minutes)
        .bind(input.buffer_after_min)
        .bind(input.min_notice_min)
        .bind(input.max_advance_days)
        .bind(intake)
        .bind(input.sort_order)
        .fetch_optional(&mut *tx)
        .await?;
        if updated.is_none() {
            return Err(AppError::not_found("Service not found"));
        }
        let view = load_one_service(&mut tx, access.business_id, service_id).await?;
        tx.commit().await?;
        Ok(view)
    }

    /// Soft-deletes the service and its variants and detaches its staff.
    pub async fn delete_service(&self, access: BusinessAccess, service_id: Uuid) -> AppResult<()> {
        access.require(&WRITERS)?;
        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        ensure_no_upcoming(
            &mut tx,
            access.business_id.as_uuid(),
            "service_id",
            service_id,
            self.clock.now(),
            "service",
        )
        .await?;
        let deleted: Option<Uuid> = sqlx::query_scalar(
            "UPDATE service SET deleted_at = now(), is_active = false
             WHERE id = $1 AND business_id = $2 AND deleted_at IS NULL
             RETURNING id",
        )
        .bind(service_id)
        .bind(access.business_id.as_uuid())
        .fetch_optional(&mut *tx)
        .await?;
        if deleted.is_none() {
            return Err(AppError::not_found("Service not found"));
        }
        sqlx::query(
            "UPDATE service_variant SET deleted_at = now(), is_active = false
             WHERE service_id = $1 AND business_id = $2 AND deleted_at IS NULL",
        )
        .bind(service_id)
        .bind(access.business_id.as_uuid())
        .execute(&mut *tx)
        .await?;
        sqlx::query("DELETE FROM staff_service WHERE service_id = $1 AND business_id = $2")
            .bind(service_id)
            .bind(access.business_id.as_uuid())
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    // -- variants -----------------------------------------------------------

    pub async fn create_variant(
        &self,
        access: BusinessAccess,
        service_id: Uuid,
        input: CreateVariantInput,
    ) -> AppResult<VariantView> {
        access.require(&WRITERS)?;
        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        ensure_service(&mut tx, access.business_id, service_id).await?;
        let currency = business_currency(&mut tx, access.business_id).await?;
        let id = insert_variant(&mut tx, access.business_id, service_id, &currency, input).await?;
        let row = load_variant(&mut tx, access.business_id, id).await?;
        tx.commit().await?;
        row.into_view()
    }

    pub async fn update_variant(
        &self,
        access: BusinessAccess,
        variant_id: Uuid,
        input: UpdateVariantInput,
    ) -> AppResult<VariantView> {
        access.require(&WRITERS)?;
        let set_name = input.name.is_some();
        let name = match &input.name {
            Some(Some(value)) => Some(validate_i18n(value, 255, "name")?.to_string()),
            _ => None,
        };
        if let Some(value) = input.duration_min {
            check_range(value.into(), 5, 720, "duration_min")?;
        }
        if let Some(value) = input.price_minor {
            check_range(value, 0, 100_000_000, "price_minor")?;
        }
        let price_type = match input.price_type.as_deref() {
            Some(value) => Some(check_price_type(value)?),
            None => None,
        };

        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        let updated: Option<Uuid> = sqlx::query_scalar(
            "UPDATE service_variant SET
                 name         = CASE WHEN $3 THEN $4::text::jsonb ELSE name END,
                 duration_min = COALESCE($5, duration_min),
                 price_minor  = COALESCE($6, price_minor),
                 price_type   = COALESCE($7, price_type),
                 is_active    = COALESCE($8, is_active),
                 sort_order   = COALESCE($9, sort_order)
             WHERE id = $1 AND business_id = $2 AND deleted_at IS NULL
             RETURNING id",
        )
        .bind(variant_id)
        .bind(access.business_id.as_uuid())
        .bind(set_name)
        .bind(name)
        .bind(input.duration_min)
        .bind(input.price_minor)
        .bind(price_type)
        .bind(input.is_active)
        .bind(input.sort_order)
        .fetch_optional(&mut *tx)
        .await?;
        if updated.is_none() {
            return Err(AppError::not_found("Variant not found"));
        }
        let row = load_variant(&mut tx, access.business_id, variant_id).await?;
        tx.commit().await?;
        row.into_view()
    }

    /// Soft-deletes a variant. Refused while a live booking still uses it, and
    /// for the last offered variant of an offered service (hide the service
    /// instead). Past visits keep their own snapshot and are not affected.
    pub async fn delete_variant(&self, access: BusinessAccess, variant_id: Uuid) -> AppResult<()> {
        access.require(&WRITERS)?;
        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        let found: Option<(Uuid, bool, bool)> = sqlx::query_as(
            "SELECT v.service_id, s.is_active, v.is_active
             FROM service_variant v
             JOIN service s ON s.id = v.service_id AND s.business_id = v.business_id
             WHERE v.id = $1 AND v.business_id = $2 AND v.deleted_at IS NULL",
        )
        .bind(variant_id)
        .bind(access.business_id.as_uuid())
        .fetch_optional(&mut *tx)
        .await?;
        let Some((service_id, service_active, variant_active)) = found else {
            return Err(AppError::not_found("Variant not found"));
        };
        ensure_no_upcoming(
            &mut tx,
            access.business_id.as_uuid(),
            "variant_id",
            variant_id,
            self.clock.now(),
            "variant",
        )
        .await?;
        if service_active && variant_active {
            let others: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM service_variant
                 WHERE service_id = $1 AND business_id = $2 AND id <> $3
                   AND is_active AND deleted_at IS NULL",
            )
            .bind(service_id)
            .bind(access.business_id.as_uuid())
            .bind(variant_id)
            .fetch_one(&mut *tx)
            .await?;
            if others == 0 {
                return Err(AppError::conflict(
                    "This is the last offered variant of the service. Hide the service instead",
                ));
            }
        }
        sqlx::query(
            "UPDATE service_variant SET deleted_at = now(), is_active = false
             WHERE id = $1 AND business_id = $2 AND deleted_at IS NULL",
        )
        .bind(variant_id)
        .bind(access.business_id.as_uuid())
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    // -- staff assignment ---------------------------------------------------

    /// Replaces the set of staff members who perform the service.
    pub async fn set_service_staff(
        &self,
        access: BusinessAccess,
        service_id: Uuid,
        input: SetServiceStaffInput,
    ) -> AppResult<ServiceView> {
        access.require(&WRITERS)?;
        let mut staff_ids = input.staff_ids;
        staff_ids.sort();
        staff_ids.dedup();

        let mut tx = begin_scoped(&self.pool, access.scope()).await?;
        ensure_service(&mut tx, access.business_id, service_id).await?;

        let known: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM staff_member WHERE business_id = $1 AND id = ANY($2)",
        )
        .bind(access.business_id.as_uuid())
        .bind(&staff_ids)
        .fetch_one(&mut *tx)
        .await?;
        if known != staff_ids.len() as i64 {
            return Err(AppError::validation(
                "staff_ids contains a member that does not belong to this business",
            ));
        }

        sqlx::query(
            "DELETE FROM staff_service
             WHERE service_id = $1 AND business_id = $2 AND NOT (staff_id = ANY($3))",
        )
        .bind(service_id)
        .bind(access.business_id.as_uuid())
        .bind(&staff_ids)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO staff_service (staff_id, service_id, business_id)
             SELECT unnest($3::uuid[]), $1::uuid, $2::uuid
             ON CONFLICT DO NOTHING",
        )
        .bind(service_id)
        .bind(access.business_id.as_uuid())
        .bind(&staff_ids)
        .execute(&mut *tx)
        .await?;

        let view = load_one_service(&mut tx, access.business_id, service_id).await?;
        tx.commit().await?;
        Ok(view)
    }
}

// ---------------------------------------------------------------------------
// Queries shared by several operations
// ---------------------------------------------------------------------------

async fn load_services(
    conn: &mut PgConnection,
    business_id: BusinessId,
    only: Option<Uuid>,
) -> AppResult<Vec<ServiceView>> {
    let services = sqlx::query_as::<_, ServiceRow>(&format!(
        "SELECT {SERVICE_COLS} FROM service
         WHERE business_id = $1 AND deleted_at IS NULL AND ($2::uuid IS NULL OR id = $2)
         ORDER BY sort_order, created_at"
    ))
    .bind(business_id.as_uuid())
    .bind(only)
    .fetch_all(&mut *conn)
    .await?;
    if services.is_empty() {
        return Ok(Vec::new());
    }
    let ids: Vec<Uuid> = services.iter().map(|service| service.id).collect();

    let variant_rows = sqlx::query_as::<_, VariantRow>(&format!(
        "SELECT {VARIANT_COLS} FROM service_variant
         WHERE business_id = $1 AND deleted_at IS NULL AND service_id = ANY($2)
         ORDER BY sort_order, created_at"
    ))
    .bind(business_id.as_uuid())
    .bind(&ids)
    .fetch_all(&mut *conn)
    .await?;
    let mut variants: HashMap<Uuid, Vec<VariantView>> = HashMap::new();
    for row in variant_rows {
        let view = row.into_view()?;
        variants.entry(view.service_id).or_default().push(view);
    }

    let staff_rows = sqlx::query_as::<_, (Uuid, Uuid)>(
        "SELECT service_id, staff_id FROM staff_service
         WHERE business_id = $1 AND service_id = ANY($2)
         ORDER BY created_at, staff_id",
    )
    .bind(business_id.as_uuid())
    .bind(&ids)
    .fetch_all(&mut *conn)
    .await?;
    let mut staff: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for (service_id, staff_id) in staff_rows {
        staff.entry(service_id).or_default().push(staff_id);
    }

    services
        .into_iter()
        .map(|service| {
            let id = service.id;
            service.into_view(
                variants.remove(&id).unwrap_or_default(),
                staff.remove(&id).unwrap_or_default(),
            )
        })
        .collect()
}

async fn load_one_service(
    conn: &mut PgConnection,
    business_id: BusinessId,
    service_id: Uuid,
) -> AppResult<ServiceView> {
    load_services(conn, business_id, Some(service_id))
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| AppError::not_found("Service not found"))
}

async fn load_variant(
    conn: &mut PgConnection,
    business_id: BusinessId,
    variant_id: Uuid,
) -> AppResult<VariantRow> {
    sqlx::query_as::<_, VariantRow>(&format!(
        "SELECT {VARIANT_COLS} FROM service_variant
         WHERE id = $1 AND business_id = $2 AND deleted_at IS NULL"
    ))
    .bind(variant_id)
    .bind(business_id.as_uuid())
    .fetch_optional(&mut *conn)
    .await?
    .ok_or_else(|| AppError::not_found("Variant not found"))
}

async fn ensure_category(
    conn: &mut PgConnection,
    business_id: BusinessId,
    category_id: Uuid,
) -> AppResult<()> {
    let found: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM service_category WHERE id = $1 AND business_id = $2 AND deleted_at IS NULL",
    )
    .bind(category_id)
    .bind(business_id.as_uuid())
    .fetch_optional(&mut *conn)
    .await?;
    match found {
        Some(_) => Ok(()),
        None => Err(AppError::validation(
            "category_id does not exist in this business",
        )),
    }
}

async fn ensure_service(
    conn: &mut PgConnection,
    business_id: BusinessId,
    service_id: Uuid,
) -> AppResult<()> {
    let found: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM service WHERE id = $1 AND business_id = $2 AND deleted_at IS NULL",
    )
    .bind(service_id)
    .bind(business_id.as_uuid())
    .fetch_optional(&mut *conn)
    .await?;
    match found {
        Some(_) => Ok(()),
        None => Err(AppError::not_found("Service not found")),
    }
}

async fn business_currency(conn: &mut PgConnection, business_id: BusinessId) -> AppResult<String> {
    let currency: Option<String> =
        sqlx::query_scalar("SELECT currency::text FROM business WHERE id = $1")
            .bind(business_id.as_uuid())
            .fetch_optional(&mut *conn)
            .await?;
    currency.ok_or_else(|| AppError::not_found("Business not found"))
}

async fn insert_variant(
    conn: &mut PgConnection,
    business_id: BusinessId,
    service_id: Uuid,
    business_currency: &str,
    input: CreateVariantInput,
) -> AppResult<Uuid> {
    let name = match &input.name {
        Some(value) => Some(validate_i18n(value, 255, "name")?.to_string()),
        None => None,
    };
    check_range(input.duration_min.into(), 5, 720, "duration_min")?;
    check_range(input.price_minor, 0, 100_000_000, "price_minor")?;
    let price_type = check_price_type(input.price_type.as_deref().unwrap_or("fixed"))?;
    let currency = match input
        .currency
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        None => business_currency.to_string(),
        Some(value) if value.eq_ignore_ascii_case(business_currency) => {
            business_currency.to_string()
        }
        Some(_) => {
            return Err(AppError::validation(format!(
                "currency must be {business_currency}, the currency of this business"
            )))
        }
    };

    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO service_variant (business_id, service_id, name, duration_min, price_minor,
                                      price_type, currency, sort_order)
         VALUES ($1, $2, $3::text::jsonb, $4, $5, $6, $7, $8)
         RETURNING id",
    )
    .bind(business_id.as_uuid())
    .bind(service_id)
    .bind(name)
    .bind(input.duration_min)
    .bind(input.price_minor)
    .bind(price_type)
    .bind(currency)
    .bind(input.sort_order.unwrap_or(0))
    .fetch_one(&mut *conn)
    .await?;
    Ok(id)
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// Checks an i18n text object and returns it trimmed.
pub(crate) fn validate_i18n(value: &Value, max_len: usize, field: &str) -> AppResult<Value> {
    let object = value.as_object().ok_or_else(|| {
        AppError::validation(format!(
            "{field} must be an object like {{\"pl\": \"...\"}}"
        ))
    })?;
    let mut clean = Map::new();
    for (locale, text) in object {
        if !LOCALES.contains(&locale.as_str()) {
            return Err(AppError::validation(format!(
                "{field}: unsupported language '{locale}' (use pl, en, ru, uk)"
            )));
        }
        let text = text
            .as_str()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .ok_or_else(|| {
                AppError::validation(format!("{field}.{locale} must be a non-empty string"))
            })?;
        if text.chars().count() > max_len {
            return Err(AppError::validation(format!(
                "{field}.{locale} must be at most {max_len} characters"
            )));
        }
        clean.insert(locale.clone(), Value::String(text.to_string()));
    }
    if clean.is_empty() {
        return Err(AppError::validation(format!(
            "{field} needs at least one language"
        )));
    }
    Ok(Value::Object(clean))
}

fn validate_intake(value: &Value) -> AppResult<String> {
    let questions = value
        .as_array()
        .ok_or_else(|| AppError::validation("intake_questions must be an array"))?;
    let text = value.to_string();
    if questions.len() > 20 || text.len() > 20_000 {
        return Err(AppError::validation(
            "intake_questions: at most 20 questions and 20 KB",
        ));
    }
    Ok(text)
}

fn check_range(value: i64, min: i64, max: i64, field: &str) -> AppResult<()> {
    if (min..=max).contains(&value) {
        Ok(())
    } else {
        Err(AppError::validation(format!(
            "{field} must be between {min} and {max}"
        )))
    }
}

fn check_step(step: i32) -> AppResult<()> {
    if BOOKING_STEPS.contains(&step) {
        Ok(())
    } else {
        Err(AppError::validation(
            "booking_step_minutes must be one of 5, 10, 15, 20, 30, 60",
        ))
    }
}

fn check_price_type(value: &str) -> AppResult<String> {
    match value {
        "fixed" | "from" => Ok(value.to_string()),
        _ => Err(AppError::validation("price_type must be 'fixed' or 'from'")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn i18n_names_are_trimmed_and_checked() {
        let ok =
            validate_i18n(&json!({"pl": "  Klasyczne ", "en": "Classic"}), 50, "name").unwrap();
        assert_eq!(ok, json!({"pl": "Klasyczne", "en": "Classic"}));

        for bad in [
            json!({}),
            json!("Classic"),
            json!({"de": "Klassisch"}),
            json!({"pl": "   "}),
            json!({"pl": 5}),
            json!({"pl": "x".repeat(51)}),
        ] {
            assert!(
                validate_i18n(&bad, 50, "name").is_err(),
                "{bad} must be rejected"
            );
        }
    }

    #[test]
    fn numeric_rules() {
        assert!(check_range(0, 0, 10, "x").is_ok());
        assert!(check_range(11, 0, 10, "x").is_err());
        assert!(check_step(15).is_ok());
        assert!(check_step(7).is_err());
        assert!(check_price_type("from").is_ok());
        assert!(check_price_type("free").is_err());
    }

    #[test]
    fn intake_questions_must_be_a_small_array() {
        assert!(validate_intake(&json!([{"key": "allergies", "type": "text"}])).is_ok());
        assert!(validate_intake(&json!({"a": 1})).is_err());
        assert!(validate_intake(&Value::Array(vec![json!("q"); 21])).is_err());
    }

    #[test]
    fn update_input_tells_absent_from_null() {
        let absent: UpdateServiceInput =
            serde_json::from_value(json!({"is_active": false})).unwrap();
        assert!(absent.category_id.is_none() && absent.description.is_none());

        let cleared: UpdateServiceInput =
            serde_json::from_value(json!({"category_id": null, "description": null})).unwrap();
        assert_eq!(cleared.category_id, Some(None));
        assert!(matches!(cleared.description, Some(None)));

        let id = Uuid::now_v7();
        let set: UpdateServiceInput = serde_json::from_value(json!({"category_id": id})).unwrap();
        assert_eq!(set.category_id, Some(Some(id)));
    }
}

/// Refuses to delete a service or variant that a live booking still uses: a
/// confirmed visit, or a hold that is still running, ending after `now`.
/// Finished and cancelled visits do not block; they keep their own snapshot
/// in `appointment_item`. `column` is `service_id` or `variant_id`.
async fn ensure_no_upcoming(
    conn: &mut PgConnection,
    business_id: &Uuid,
    column: &str,
    id: Uuid,
    now: OffsetDateTime,
    what: &str,
) -> AppResult<()> {
    let sql = format!(
        "SELECT EXISTS (
             SELECT 1
             FROM appointment_item i
             JOIN appointment a ON a.id = i.appointment_id AND a.business_id = i.business_id
             WHERE i.business_id = $1 AND i.{column} = $2 AND a.end_at > $3
               AND (a.status = 'confirmed'
                    OR (a.status = 'held' AND a.hold_expires_at > $3)))"
    );
    let busy: bool = sqlx::query_scalar(&sql)
        .bind(business_id)
        .bind(id)
        .bind(now)
        .fetch_one(conn)
        .await?;
    if busy {
        return Err(AppError::conflict(format!(
            "This {what} has upcoming appointments. Hide it instead of deleting"
        )));
    }
    Ok(())
}
