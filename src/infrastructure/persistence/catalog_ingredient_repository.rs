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
        // 🎯 ЭТАЛОН B2B SaaS: Use translations table with COALESCE fallback
        let lang_code = language.code();
        
        let sql = r#"
            SELECT 
                ci.id, 
                ci.category_id, 
                ci.name_pl, 
                ci.name_en, 
                ci.name_uk, 
                ci.name_ru,
                ci.default_unit::text as default_unit, 
                ci.default_shelf_life_days,
                ARRAY(SELECT unnest(ci.allergens)::text) as allergens, 
                ci.calories_per_100g, 
                ARRAY(SELECT unnest(ci.seasons)::text) as seasons, 
                ci.image_url,
                COALESCE(cit_user.name, cit_en.name) as search_name
            FROM catalog_ingredients ci
            LEFT JOIN catalog_ingredient_translations cit_user 
                ON cit_user.ingredient_id = ci.id AND cit_user.language = $2
            LEFT JOIN catalog_ingredient_translations cit_en 
                ON cit_en.ingredient_id = ci.id AND cit_en.language = 'en'
            WHERE COALESCE(cit_user.name, cit_en.name) ILIKE '%' || $1 || '%'
            ORDER BY COALESCE(cit_user.name, cit_en.name) ASC
            LIMIT $3
        "#;

        let rows = sqlx::query(sql)
            .bind(query)
            .bind(lang_code)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;

        rows.iter()
            .map(Self::row_to_ingredient)
            .collect()
    }

    async fn search_by_category(&self, category_id: CatalogCategoryId, query: Option<&str>, language: Language, limit: i64) -> AppResult<Vec<CatalogIngredient>> {
        // 🎯 ЭТАЛОН B2B SaaS: Use translations table with COALESCE fallback
        let lang_code = language.code();

        let sql = if query.is_some() {
            r#"
                SELECT 
                    ci.id, 
                    ci.category_id, 
                    ci.name_pl, 
                    ci.name_en, 
                    ci.name_uk, 
                    ci.name_ru,
                    ci.default_unit::text as default_unit, 
                    ci.default_shelf_life_days,
                    ARRAY(SELECT unnest(ci.allergens)::text) as allergens, 
                    ci.calories_per_100g, 
                    ARRAY(SELECT unnest(ci.seasons)::text) as seasons, 
                    ci.image_url,
                    COALESCE(cit_user.name, cit_en.name) as search_name
                FROM catalog_ingredients ci
                LEFT JOIN catalog_ingredient_translations cit_user 
                    ON cit_user.ingredient_id = ci.id AND cit_user.language = $3
                LEFT JOIN catalog_ingredient_translations cit_en 
                    ON cit_en.ingredient_id = ci.id AND cit_en.language = 'en'
                WHERE ci.category_id = $1 
                  AND COALESCE(cit_user.name, cit_en.name) ILIKE '%' || $2 || '%'
                ORDER BY COALESCE(cit_user.name, cit_en.name) ASC
                LIMIT $4
            "#
        } else {
            r#"
                SELECT 
                    ci.id, 
                    ci.category_id, 
                    ci.name_pl, 
                    ci.name_en, 
                    ci.name_uk, 
                    ci.name_ru,
                    ci.default_unit::text as default_unit, 
                    ci.default_shelf_life_days,
                    ARRAY(SELECT unnest(ci.allergens)::text) as allergens, 
                    ci.calories_per_100g, 
                    ARRAY(SELECT unnest(ci.seasons)::text) as seasons, 
                    ci.image_url,
                    COALESCE(cit_user.name, cit_en.name) as search_name
                FROM catalog_ingredients ci
                LEFT JOIN catalog_ingredient_translations cit_user 
                    ON cit_user.ingredient_id = ci.id AND cit_user.language = $2
                LEFT JOIN catalog_ingredient_translations cit_en 
                    ON cit_en.ingredient_id = ci.id AND cit_en.language = 'en'
                WHERE ci.category_id = $1
                ORDER BY COALESCE(cit_user.name, cit_en.name) ASC
                LIMIT $3
            "#
        };

        let rows = if let Some(q) = query {
            sqlx::query(sql)
                .bind(category_id.as_uuid())
                .bind(q)
                .bind(lang_code)
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query(sql)
                .bind(category_id.as_uuid())
                .bind(lang_code)
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
        // 🎯 ЭТАЛОН B2B SaaS: Use translations table with COALESCE fallback
        let lang_code = language.code();

        let sql = r#"
            SELECT 
                ci.id, 
                ci.category_id, 
                ci.name_pl, 
                ci.name_en, 
                ci.name_uk, 
                ci.name_ru,
                ci.default_unit::text as default_unit, 
                ci.default_shelf_life_days,
                ARRAY(SELECT unnest(ci.allergens)::text) as allergens, 
                ci.calories_per_100g, 
                ARRAY(SELECT unnest(ci.seasons)::text) as seasons, 
                ci.image_url,
                COALESCE(cit_user.name, cit_en.name) as search_name
            FROM catalog_ingredients ci
            LEFT JOIN catalog_ingredient_translations cit_user 
                ON cit_user.ingredient_id = ci.id AND cit_user.language = $3
            LEFT JOIN catalog_ingredient_translations cit_en 
                ON cit_en.ingredient_id = ci.id AND cit_en.language = 'en'
            ORDER BY COALESCE(cit_user.name, cit_en.name) ASC
            OFFSET $1
            LIMIT $2
        "#;

        let rows = sqlx::query(sql)
            .bind(offset)
            .bind(limit)
            .bind(lang_code)
            .fetch_all(&self.pool)
            .await?;

        rows.iter()
            .map(Self::row_to_ingredient)
            .collect()
    }
}
