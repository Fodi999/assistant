pub mod assistant_state_repository;
pub mod catalog_category_repository;
pub mod catalog_ingredient_repository;
pub mod dictionary_service;
pub mod dish_repository;
pub mod inventory_product_repository;
pub mod recipe_repository;
pub mod recipe_v2_repository;  // V2 with translation support
pub mod recipe_translation_repository;
pub mod recipe_ai_insights_repository;  // AI insights repository
pub mod refresh_token_repository;
pub mod tenant_repository;
pub mod tenant_ingredient_repository;
pub mod user_repository;

pub use assistant_state_repository::*;
pub use catalog_category_repository::*;
pub use catalog_ingredient_repository::*;
pub use dictionary_service::*;
pub use dish_repository::*;
pub use inventory_product_repository::*;
pub use recipe_repository::*;
pub use recipe_v2_repository::*;
pub use recipe_translation_repository::*;
pub use recipe_ai_insights_repository::*;
pub use refresh_token_repository::*;
pub use tenant_repository::*;
pub use tenant_ingredient_repository::*;
pub use user_repository::*;

use sqlx::PgPool;

#[derive(Clone)]
pub struct Repositories {
    pub pool: PgPool,
    pub tenant: TenantRepository,
    pub user: UserRepository,
    pub refresh_token: RefreshTokenRepository,
    pub assistant_state: AssistantStateRepository,
    pub catalog_category: CatalogCategoryRepository,
    pub catalog_ingredient: CatalogIngredientRepository,
    pub dictionary: DictionaryService,
    pub inventory_product: InventoryProductRepository,
    pub recipe: RecipeRepository,
    pub dish: DishRepository,
    pub recipe_v2: RecipeRepositoryV2,
    pub recipe_ingredient: RecipeIngredientRepository,
    pub recipe_translation: RecipeTranslationRepository,
    pub recipe_ai_insights: RecipeAIInsightsRepository,
    pub tenant_ingredient: TenantIngredientRepository,
}

impl Repositories {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool: pool.clone(),
            tenant: TenantRepository::new(pool.clone()),
            user: UserRepository::new(pool.clone()),
            refresh_token: RefreshTokenRepository::new(pool.clone()),
            assistant_state: AssistantStateRepository::new(pool.clone()),
            catalog_category: CatalogCategoryRepository::new(pool.clone()),
            catalog_ingredient: CatalogIngredientRepository::new(pool.clone()),
            dictionary: DictionaryService::new(pool.clone()),
            inventory_product: InventoryProductRepository::new(pool.clone()),
            recipe: RecipeRepository::new(pool.clone()),
            dish: DishRepository::new(pool.clone()),
            recipe_v2: RecipeRepositoryV2::new(pool.clone()),
            recipe_ingredient: RecipeIngredientRepository::new(pool.clone()),
            recipe_translation: RecipeTranslationRepository::new(pool.clone()),
            recipe_ai_insights: RecipeAIInsightsRepository::new(pool.clone()),
            tenant_ingredient: TenantIngredientRepository::new(pool),
        }
    }
}
