use sqlx::{PgPool, Row};
use crate::domain::catalog::{CatalogCategory, CatalogCategoryId};
use crate::shared::{result::AppResult, Language};

#[async_trait::async_trait]
pub trait CatalogCategoryRepositoryTrait: Send + Sync {
    async fn list(&self, language: Language) -> AppResult<Vec<CatalogCategory>>;
    async fn find_by_id(&self, id: CatalogCategoryId) -> AppResult<Option<CatalogCategory>>;
}

#[derive(Clone)]
pub struct CatalogCategoryRepository {
    pool: PgPool,
}

impl CatalogCategoryRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn row_to_category(row: &sqlx::postgres::PgRow) -> AppResult<CatalogCategory> {
        let id = CatalogCategoryId::from_uuid(row.try_get("id")?);
        let name_pl: String = row.try_get("name_pl")?;
        let name_en: String = row.try_get("name_en")?;
        let name_uk: String = row.try_get("name_uk")?;
        let name_ru: String = row.try_get("name_ru")?;
        let sort_order: i32 = row.try_get("sort_order")?;

        Ok(CatalogCategory::from_parts(
            id,
            name_pl,
            name_en,
            name_uk,
            name_ru,
            sort_order,
        ))
    }
}

#[async_trait::async_trait]
impl CatalogCategoryRepositoryTrait for CatalogCategoryRepository {
    async fn list(&self, _language: Language) -> AppResult<Vec<CatalogCategory>> {
        let sql = r#"
            SELECT id, name_pl, name_en, name_uk, name_ru, sort_order
            FROM catalog_categories
            ORDER BY sort_order ASC
        "#;

        let rows = sqlx::query(sql)
            .fetch_all(&self.pool)
            .await?;

        rows.iter()
            .map(Self::row_to_category)
            .collect()
    }

    async fn find_by_id(&self, id: CatalogCategoryId) -> AppResult<Option<CatalogCategory>> {
        let sql = r#"
            SELECT id, name_pl, name_en, name_uk, name_ru, sort_order
            FROM catalog_categories
            WHERE id = $1
        "#;

        let row = sqlx::query(sql)
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await?;

        row.as_ref()
            .map(Self::row_to_category)
            .transpose()
    }
}
