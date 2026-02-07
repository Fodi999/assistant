use crate::domain::catalog::{CatalogCategory, CatalogCategoryId, CatalogIngredient, CatalogIngredientId};
use crate::infrastructure::persistence::{CatalogCategoryRepository, CatalogCategoryRepositoryTrait, CatalogIngredientRepository, CatalogIngredientRepositoryTrait};
use crate::shared::{result::AppResult, Language};
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct CatalogService {
    category_repo: Arc<CatalogCategoryRepository>,
    ingredient_repo: Arc<CatalogIngredientRepository>,
}

impl CatalogService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            category_repo: Arc::new(CatalogCategoryRepository::new(pool.clone())),
            ingredient_repo: Arc::new(CatalogIngredientRepository::new(pool)),
        }
    }

    /// Get all categories ordered by sort_order
    pub async fn get_categories(&self, language: Language) -> AppResult<Vec<CatalogCategory>> {
        self.category_repo.list(language).await
    }

    /// Get category by ID
    pub async fn get_category_by_id(&self, id: CatalogCategoryId) -> AppResult<Option<CatalogCategory>> {
        self.category_repo.find_by_id(id).await
    }

    /// Search ingredients by name in user's language
    pub async fn search_ingredients(
        &self,
        query: &str,
        language: Language,
        limit: i64,
    ) -> AppResult<Vec<CatalogIngredient>> {
        self.ingredient_repo.search(query, language, limit).await
    }

    /// Search ingredients by category (with optional name filter)
    pub async fn search_ingredients_by_category(
        &self,
        category_id: CatalogCategoryId,
        query: Option<&str>,
        language: Language,
        limit: i64,
    ) -> AppResult<Vec<CatalogIngredient>> {
        self.ingredient_repo.search_by_category(category_id, query, language, limit).await
    }

    /// Get ingredient by ID
    pub async fn get_ingredient_by_id(&self, id: CatalogIngredientId) -> AppResult<Option<CatalogIngredient>> {
        self.ingredient_repo.find_by_id(id).await
    }

    /// List all ingredients (paginated)
    pub async fn list_ingredients(&self, language: Language, offset: i64, limit: i64) -> AppResult<Vec<CatalogIngredient>> {
        self.ingredient_repo.list(language, offset, limit).await
    }
}
