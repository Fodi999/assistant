use crate::domain::catalog::*;
use crate::shared::{AppResult, Language};
use async_trait::async_trait;
use sqlx::{PgPool, Row};

#[async_trait]
pub trait CatalogIngredientRepositoryTrait: Send + Sync {
    /// Search ingredients by name in the user's language
    async fn search(&self, query: &str, language: Language, limit: i64) -> AppResult<Vec<CatalogIngredient>>;
    
    /// Search ingredients by category and optional name filter
    async fn search_by_category(&self, category_id: CatalogCategoryId, query: Option<&str>, language: Language, limit: i64) -> AppResult<Vec<CatalogIngredient>>;
    
    /// Get ingredient by ID
    async fn find_by_id(&self, id: CatalogIngredientId) -> AppResult<Option<CatalogIngredient>>;
    
    /// Get all ingredients (paginated)
    async fn list(&self, language: Language, offset: i64, limit: i64) -> AppResult<Vec<CatalogIngredient>>;
}

#[derive(Clone)]
pub struct CatalogIngredientRepository {
    pool: PgPool,
}

impl CatalogIngredientRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn row_to_ingredient(row: &sqlx::postgres::PgRow) -> AppResult<CatalogIngredient> {
        let id: uuid::Uuid = row.try_get("id")?;
        let category_id: uuid::Uuid = row.try_get("category_id")?;
        let name_pl: String = row.try_get("name_pl")?;
        let name_en: String = row.try_get("name_en")?;
        let name_uk: String = row.try_get("name_uk")?;
        let name_ru: String = row.try_get("name_ru")?;
        
        // CAST ENUM to TEXT in SQL query instead of trying to parse here
        let unit_str: String = row.try_get("default_unit")?;
        let default_unit = Unit::from_str(&unit_str)?;
        let default_shelf_life_days: Option<i32> = row.try_get("default_shelf_life_days")?;
        
        let allergens_str: Vec<String> = row.try_get("allergens")?;
        let allergens: Vec<Allergen> = allergens_str
            .iter()
            .filter_map(|s| Allergen::from_str(s).ok())
            .collect();
        
        let calories_per_100g: Option<i32> = row.try_get("calories_per_100g")?;
        
        let seasons_str: Vec<String> = row.try_get("seasons")?;
        let seasons: Vec<Season> = seasons_str
            .iter()
            .filter_map(|s| Season::from_str(s).ok())
            .collect();
        
        let image_url: Option<String> = row.try_get("image_url")?;

        Ok(CatalogIngredient::from_parts(
            CatalogIngredientId::from_uuid(id),
            CatalogCategoryId::from_uuid(category_id),
            name_pl,
            name_en,
            name_uk,
            name_ru,
            default_unit,
            default_shelf_life_days,
            allergens,
            calories_per_100g,
            seasons,
            image_url,
        ))
    }
}

#[async_trait]
impl CatalogIngredientRepositoryTrait for CatalogIngredientRepository {
    async fn search(&self, query: &str, language: Language, limit: i64) -> AppResult<Vec<CatalogIngredient>> {
        // Search in the user's language column only
        let name_column = match language {
            Language::Pl => "name_pl",
            Language::En => "name_en",
            Language::Uk => "name_uk",
            Language::Ru => "name_ru",
        };

        let sql = format!(
            r#"
            SELECT id, category_id, name_pl, name_en, name_uk, name_ru, 
                   default_unit::text as default_unit, default_shelf_life_days,
                   ARRAY(SELECT unnest(allergens)::text) as allergens, 
                   calories_per_100g, 
                   ARRAY(SELECT unnest(seasons)::text) as seasons, 
                   image_url
            FROM catalog_ingredients
            WHERE {} ILIKE $1
            ORDER BY {} ASC
            LIMIT $2
            "#,
            name_column, name_column
        );

        let search_pattern = format!("{}%", query);
        let rows = sqlx::query(&sql)
            .bind(&search_pattern)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;

        rows.iter()
            .map(Self::row_to_ingredient)
            .collect()
    }

    async fn search_by_category(&self, category_id: CatalogCategoryId, query: Option<&str>, language: Language, limit: i64) -> AppResult<Vec<CatalogIngredient>> {
        // Search in the user's language column only
        let name_column = match language {
            Language::Pl => "name_pl",
            Language::En => "name_en",
            Language::Uk => "name_uk",
            Language::Ru => "name_ru",
        };

        let sql = if let Some(_q) = query {
            format!(
                r#"
                SELECT id, category_id, name_pl, name_en, name_uk, name_ru, 
                       default_unit::text as default_unit, default_shelf_life_days,
                       ARRAY(SELECT unnest(allergens)::text) as allergens, 
                       calories_per_100g, 
                       ARRAY(SELECT unnest(seasons)::text) as seasons, 
                       image_url
                FROM catalog_ingredients
                WHERE category_id = $1 AND {} ILIKE $2
                ORDER BY {} ASC
                LIMIT $3
                "#,
                name_column, name_column
            )
        } else {
            format!(
                r#"
                SELECT id, category_id, name_pl, name_en, name_uk, name_ru, 
                       default_unit::text as default_unit, default_shelf_life_days,
                       ARRAY(SELECT unnest(allergens)::text) as allergens, 
                       calories_per_100g, 
                       ARRAY(SELECT unnest(seasons)::text) as seasons, 
                       image_url
                FROM catalog_ingredients
                WHERE category_id = $1
                ORDER BY {} ASC
                LIMIT $2
                "#,
                name_column
            )
        };

        let rows = if let Some(q) = query {
            let search_pattern = format!("{}%", q);
            sqlx::query(&sql)
                .bind(category_id.as_uuid())
                .bind(&search_pattern)
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query(&sql)
                .bind(category_id.as_uuid())
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
        };

        rows.iter()
            .map(Self::row_to_ingredient)
            .collect()
    }

    async fn find_by_id(&self, id: CatalogIngredientId) -> AppResult<Option<CatalogIngredient>> {
        let row = sqlx::query(
            r#"
            SELECT id, category_id, name_pl, name_en, name_uk, name_ru, 
                   default_unit::text as default_unit, default_shelf_life_days,
                   ARRAY(SELECT unnest(allergens)::text) as allergens, 
                   calories_per_100g, 
                   ARRAY(SELECT unnest(seasons)::text) as seasons, 
                   image_url
            FROM catalog_ingredients
            WHERE id = $1
            "#
        )
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(ref r) => Ok(Some(Self::row_to_ingredient(r)?)),
            None => Ok(None),
        }
    }

    async fn list(&self, language: Language, offset: i64, limit: i64) -> AppResult<Vec<CatalogIngredient>> {
        // Order by the user's language column
        let name_column = match language {
            Language::Pl => "name_pl",
            Language::En => "name_en",
            Language::Uk => "name_uk",
            Language::Ru => "name_ru",
        };

        let sql = format!(
            r#"
            SELECT id, category_id, name_pl, name_en, name_uk, name_ru, 
                   default_unit::text as default_unit, default_shelf_life_days,
                   ARRAY(SELECT unnest(allergens)::text) as allergens, 
                   calories_per_100g, 
                   ARRAY(SELECT unnest(seasons)::text) as seasons, 
                   image_url
            FROM catalog_ingredients
            ORDER BY {} ASC
            OFFSET $1
            LIMIT $2
            "#,
            name_column
        );

        let rows = sqlx::query(&sql)
            .bind(offset)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;

        rows.iter()
            .map(Self::row_to_ingredient)
            .collect()
    }
}
