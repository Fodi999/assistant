//! PurchaseDraftService — создание/чтение/отмена заготовок закупок.
//!
//! Используется Copilot-ом для tool `prepare_purchase_draft`.
//! Жизненный цикл: draft → (sent | cancelled).

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::shared::{AppError, AppResult, TenantId, UserId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseDraftItemInput {
    pub catalog_ingredient_id: Option<Uuid>,
    pub ingredient_name: String,
    pub quantity: f64,
    pub unit: String,
    pub price_per_unit_cents: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePurchaseDraftInput {
    pub supplier_name: Option<String>,
    pub delivery_date: Option<time::Date>,
    pub note: Option<String>,
    pub items: Vec<PurchaseDraftItemInput>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PurchaseDraft {
    pub id: Uuid,
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub supplier_name: Option<String>,
    pub delivery_date: Option<time::Date>,
    pub note: Option<String>,
    pub status: String,
    pub total_cost_cents: i64,
    pub items: Vec<PurchaseDraftItem>,
    pub created_at: time::OffsetDateTime,
}

#[derive(Debug, Clone, Serialize)]
pub struct PurchaseDraftItem {
    pub id: Uuid,
    pub catalog_ingredient_id: Option<Uuid>,
    pub ingredient_name: String,
    pub quantity: f64,
    pub unit: String,
    pub price_per_unit_cents: Option<i64>,
}

pub struct PurchaseDraftService {
    pool: PgPool,
}

impl PurchaseDraftService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Создать новый draft с позициями.
    pub async fn create(
        &self,
        user_id: UserId,
        tenant_id: TenantId,
        input: CreatePurchaseDraftInput,
    ) -> AppResult<Uuid> {
        if input.items.is_empty() {
            return Err(AppError::validation("Purchase draft must contain at least one item"));
        }

        let draft_id = Uuid::new_v4();
        let uid = *user_id.as_uuid();
        let tid = *tenant_id.as_uuid();

        let total: i64 = input.items.iter()
            .filter_map(|i| i.price_per_unit_cents.map(|p| (p as f64 * i.quantity) as i64))
            .sum();

        let mut tx = self.pool.begin().await?;

        sqlx::query(
            "INSERT INTO purchase_drafts \
             (id, user_id, tenant_id, supplier_name, delivery_date, note, status, total_cost_cents) \
             VALUES ($1, $2, $3, $4, $5, $6, 'draft', $7)"
        )
        .bind(draft_id)
        .bind(uid)
        .bind(tid)
        .bind(&input.supplier_name)
        .bind(input.delivery_date)
        .bind(&input.note)
        .bind(total)
        .execute(&mut *tx)
        .await?;

        for item in &input.items {
            sqlx::query(
                "INSERT INTO purchase_draft_items \
                 (id, draft_id, catalog_ingredient_id, ingredient_name, quantity, unit, price_per_unit_cents) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7)"
            )
            .bind(Uuid::new_v4())
            .bind(draft_id)
            .bind(item.catalog_ingredient_id)
            .bind(&item.ingredient_name)
            .bind(item.quantity)
            .bind(&item.unit)
            .bind(item.price_per_unit_cents)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        tracing::info!(
            "✅ Purchase draft created: id={} items={} supplier={:?}",
            draft_id,
            input.items.len(),
            input.supplier_name,
        );

        Ok(draft_id)
    }

    /// Получить draft по id (для пользователя).
    pub async fn get(&self, draft_id: Uuid, user_id: UserId) -> AppResult<Option<PurchaseDraft>> {
        let uid = *user_id.as_uuid();

        let row: Option<DraftRow> = sqlx::query_as::<_, DraftRow>(
            "SELECT id, user_id, tenant_id, supplier_name, delivery_date, note, status, total_cost_cents, created_at \
             FROM purchase_drafts WHERE id = $1 AND user_id = $2"
        )
        .bind(draft_id)
        .bind(uid)
        .fetch_optional(&self.pool)
        .await?;

        let Some(r) = row else { return Ok(None); };

        let items: Vec<ItemRow> = sqlx::query_as::<_, ItemRow>(
            "SELECT id, catalog_ingredient_id, ingredient_name, quantity, unit, price_per_unit_cents \
             FROM purchase_draft_items WHERE draft_id = $1 ORDER BY created_at"
        )
        .bind(draft_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(Some(PurchaseDraft {
            id: r.id,
            user_id: r.user_id,
            tenant_id: r.tenant_id,
            supplier_name: r.supplier_name,
            delivery_date: r.delivery_date,
            note: r.note,
            status: r.status,
            total_cost_cents: r.total_cost_cents,
            created_at: r.created_at,
            items: items.into_iter().map(|i| PurchaseDraftItem {
                id: i.id,
                catalog_ingredient_id: i.catalog_ingredient_id,
                ingredient_name: i.ingredient_name,
                quantity: i.quantity,
                unit: i.unit,
                price_per_unit_cents: i.price_per_unit_cents,
            }).collect(),
        }))
    }

    /// Список drafts пользователя.
    pub async fn list(&self, tenant_id: TenantId, limit: i64) -> AppResult<Vec<PurchaseDraft>> {
        let tid = *tenant_id.as_uuid();
        let drafts: Vec<DraftRow> = sqlx::query_as::<_, DraftRow>(
            "SELECT id, user_id, tenant_id, supplier_name, delivery_date, note, status, total_cost_cents, created_at \
             FROM purchase_drafts WHERE tenant_id = $1 ORDER BY created_at DESC LIMIT $2"
        )
        .bind(tid)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::with_capacity(drafts.len());
        for r in drafts {
            let items: Vec<ItemRow> = sqlx::query_as::<_, ItemRow>(
                "SELECT id, catalog_ingredient_id, ingredient_name, quantity, unit, price_per_unit_cents \
                 FROM purchase_draft_items WHERE draft_id = $1 ORDER BY created_at"
            )
            .bind(r.id)
            .fetch_all(&self.pool)
            .await?;

            out.push(PurchaseDraft {
                id: r.id,
                user_id: r.user_id,
                tenant_id: r.tenant_id,
                supplier_name: r.supplier_name,
                delivery_date: r.delivery_date,
                note: r.note,
                status: r.status,
                total_cost_cents: r.total_cost_cents,
                created_at: r.created_at,
                items: items.into_iter().map(|i| PurchaseDraftItem {
                    id: i.id,
                    catalog_ingredient_id: i.catalog_ingredient_id,
                    ingredient_name: i.ingredient_name,
                    quantity: i.quantity,
                    unit: i.unit,
                    price_per_unit_cents: i.price_per_unit_cents,
                }).collect(),
            });
        }
        Ok(out)
    }
}

#[derive(sqlx::FromRow)]
struct DraftRow {
    id: Uuid,
    user_id: Uuid,
    tenant_id: Uuid,
    supplier_name: Option<String>,
    delivery_date: Option<time::Date>,
    note: Option<String>,
    status: String,
    total_cost_cents: i64,
    created_at: time::OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct ItemRow {
    id: Uuid,
    catalog_ingredient_id: Option<Uuid>,
    ingredient_name: String,
    quantity: f64,
    unit: String,
    price_per_unit_cents: Option<i64>,
}
