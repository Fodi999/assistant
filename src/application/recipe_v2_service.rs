// Recipe V2 Service - Recipe management with automatic translations
use crate::application::recipe_translation_service::RecipeTranslationService;
use crate::domain::recipe_v2::{Recipe, RecipeId, RecipeIngredient, RecipeStatus};
use crate::domain::CatalogIngredientId;
use crate::infrastructure::persistence::{
    RecipeV2RepositoryTrait, RecipeIngredientRepositoryTrait, CatalogIngredientRepositoryTrait,
};
use crate::shared::{AppError, AppResult, Language, TenantId, UserId};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

// ========== DTOs ==========

#[derive(Debug, Deserialize)]
pub struct CreateRecipeDto {
    pub name: String,
    pub instructions: String,
    pub language: Language,
    pub servings: i32,
    pub ingredients: Vec<CreateRecipeIngredientDto>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRecipeIngredientDto {
    pub catalog_ingredient_id: Uuid,
    pub quantity: Decimal,
    pub unit: String,
}

#[derive(Debug, Serialize)]
pub struct RecipeResponseDto {
    pub id: Uuid,
    pub name: String,
    pub instructions: String,
    pub language: Language,
    pub servings: i32,
    pub total_cost_cents: Option<i32>,
    pub cost_per_serving_cents: Option<i32>,
    pub status: String,
    pub is_public: bool,
    pub published_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub ingredients: Vec<RecipeIngredientResponseDto>,
}

#[derive(Debug, Serialize)]
pub struct RecipeIngredientResponseDto {
    pub id: Uuid,
    pub catalog_ingredient_id: Uuid,
    pub catalog_ingredient_name: Option<String>,
    pub quantity: Decimal,
    pub unit: String,
    pub cost_at_use_cents: Option<i32>,
}

// ========== Service ==========

#[derive(Clone)]
pub struct RecipeV2Service {
    recipe_repo: Arc<dyn RecipeV2RepositoryTrait>,
    ingredient_repo: Arc<dyn RecipeIngredientRepositoryTrait>,
    catalog_repo: Arc<dyn CatalogIngredientRepositoryTrait>,
    translation_service: Arc<RecipeTranslationService>,
}

impl RecipeV2Service {
    pub fn new(
        recipe_repo: Arc<dyn RecipeV2RepositoryTrait>,
        ingredient_repo: Arc<dyn RecipeIngredientRepositoryTrait>,
        catalog_repo: Arc<dyn CatalogIngredientRepositoryTrait>,
        translation_service: Arc<RecipeTranslationService>,
    ) -> Self {
        Self {
            recipe_repo,
            ingredient_repo,
            catalog_repo,
            translation_service,
        }
    }

    /// Create a new recipe with automatic translation to all languages
    /// Translations happen asynchronously and don't block the response
    pub async fn create_recipe(
        &self,
        dto: CreateRecipeDto,
        user_id: UserId,
        tenant_id: TenantId,
    ) -> AppResult<RecipeResponseDto> {
        // Validate servings
        if dto.servings <= 0 {
            return Err(AppError::validation("Servings must be positive"));
        }

        // Validate ingredients exist in catalog
        for ingredient_dto in &dto.ingredients {
            let ingredient_id = CatalogIngredientId::from_uuid(ingredient_dto.catalog_ingredient_id);
            let exists = self.catalog_repo
                .find_by_id(ingredient_id)
                .await?
                .is_some();
            
            if !exists {
                return Err(AppError::not_found("Catalog ingredient"));
            }
        }

        // Create recipe (cost calculation will be done separately via inventory)
        let recipe_id = RecipeId::new();
        let now = OffsetDateTime::now_utc();
        
        let recipe = Recipe {
            id: recipe_id,
            user_id,
            tenant_id,
            name_default: dto.name,
            instructions_default: dto.instructions,
            language_default: dto.language,
            servings: dto.servings,
            total_cost_cents: None,  // Will be calculated from inventory later
            cost_per_serving_cents: None,
            status: RecipeStatus::Draft,
            is_public: false,
            published_at: None,
            created_at: now,
            updated_at: now,
        };

        // Save recipe
        self.recipe_repo.save(&recipe).await?;

        // Save ingredients
        let mut response_ingredients = Vec::new();
        
        for ingredient_dto in &dto.ingredients {
            let ingredient_id = CatalogIngredientId::from_uuid(ingredient_dto.catalog_ingredient_id);
            let catalog_ingredient = self.catalog_repo
                .find_by_id(ingredient_id)
                .await?
                .ok_or_else(|| AppError::not_found("Catalog ingredient"))?;
            
            let ingredient_name = catalog_ingredient.name(recipe.language_default).to_string();
            
            let ingredient = RecipeIngredient {
                id: crate::domain::recipe_v2::RecipeIngredientId::new(),
                recipe_id,
                catalog_ingredient_id: ingredient_dto.catalog_ingredient_id,
                quantity: ingredient_dto.quantity,
                unit: ingredient_dto.unit.clone(),
                cost_at_use_cents: None,  // Will be calculated from inventory later
                catalog_ingredient_name_snapshot: Some(ingredient_name.clone()),
                created_at: now,
            };

            self.ingredient_repo.save(&ingredient).await?;

            response_ingredients.push(RecipeIngredientResponseDto {
                id: ingredient.id.as_uuid(),
                catalog_ingredient_id: ingredient.catalog_ingredient_id,
                catalog_ingredient_name: Some(ingredient_name),
                quantity: ingredient.quantity,
                unit: ingredient.unit.clone(),
                cost_at_use_cents: None,
            });
        }

        // Trigger automatic translation to all other languages (async, non-blocking)
        let translation_service = self.translation_service.clone();
        let default_language = dto.language;
        tokio::spawn(async move {
            if let Err(e) = translation_service.translate_to_all_languages(recipe_id, default_language).await {
                tracing::error!("Failed to trigger translations for recipe {}: {}", recipe_id.as_uuid(), e);
            }
        });

        // Return response
        Ok(RecipeResponseDto {
            id: recipe.id.as_uuid(),
            name: recipe.name_default,
            instructions: recipe.instructions_default,
            language: recipe.language_default,
            servings: recipe.servings,
            total_cost_cents: recipe.total_cost_cents,
            cost_per_serving_cents: recipe.cost_per_serving_cents,
            status: recipe.status.as_str().to_string(),
            is_public: recipe.is_public,
            published_at: recipe.published_at,
            created_at: recipe.created_at,
            updated_at: recipe.updated_at,
            ingredients: response_ingredients,
        })
    }

    /// Get recipe by ID with localized content
    pub async fn get_recipe(
        &self,
        recipe_id: RecipeId,
        language: Language,
    ) -> AppResult<RecipeResponseDto> {
        // Load recipe
        let recipe = self.recipe_repo
            .find_by_id(recipe_id)
            .await?
            .ok_or_else(|| AppError::not_found("Recipe"))?;

        // Get localized content
        let (name, instructions) = self.translation_service
            .get_localized_content(recipe_id, language)
            .await?;

        // Load ingredients
        let ingredients = self.ingredient_repo
            .find_by_recipe_id(recipe_id)
            .await?;

        let response_ingredients = ingredients
            .into_iter()
            .map(|i| RecipeIngredientResponseDto {
                id: i.id.as_uuid(),
                catalog_ingredient_id: i.catalog_ingredient_id,
                catalog_ingredient_name: i.catalog_ingredient_name_snapshot,
                quantity: i.quantity,
                unit: i.unit,
                cost_at_use_cents: i.cost_at_use_cents,
            })
            .collect();

        Ok(RecipeResponseDto {
            id: recipe.id.as_uuid(),
            name,
            instructions,
            language,
            servings: recipe.servings,
            total_cost_cents: recipe.total_cost_cents,
            cost_per_serving_cents: recipe.cost_per_serving_cents,
            status: recipe.status.as_str().to_string(),
            is_public: recipe.is_public,
            published_at: recipe.published_at,
            created_at: recipe.created_at,
            updated_at: recipe.updated_at,
            ingredients: response_ingredients,
        })
    }

    /// List all recipes for user
    pub async fn list_user_recipes(
        &self,
        user_id: UserId,
        language: Language,
    ) -> AppResult<Vec<RecipeResponseDto>> {
        let recipes = self.recipe_repo.find_by_user_id(user_id).await?;

        let mut response = Vec::new();
        for recipe in recipes {
            let (name, instructions) = self.translation_service
                .get_localized_content(recipe.id, language)
                .await
                .unwrap_or((recipe.name_default.clone(), recipe.instructions_default.clone()));

            let ingredients = self.ingredient_repo
                .find_by_recipe_id(recipe.id)
                .await?;

            let response_ingredients = ingredients
                .into_iter()
                .map(|i| RecipeIngredientResponseDto {
                    id: i.id.as_uuid(),
                    catalog_ingredient_id: i.catalog_ingredient_id,
                    catalog_ingredient_name: i.catalog_ingredient_name_snapshot,
                    quantity: i.quantity,
                    unit: i.unit,
                    cost_at_use_cents: i.cost_at_use_cents,
                })
                .collect();

            response.push(RecipeResponseDto {
                id: recipe.id.as_uuid(),
                name,
                instructions,
                language,
                servings: recipe.servings,
                total_cost_cents: recipe.total_cost_cents,
                cost_per_serving_cents: recipe.cost_per_serving_cents,
                status: recipe.status.as_str().to_string(),
                is_public: recipe.is_public,
                published_at: recipe.published_at,
                created_at: recipe.created_at,
                updated_at: recipe.updated_at,
                ingredients: response_ingredients,
            });
        }

        Ok(response)
    }

    /// Publish recipe (make it public)
    pub async fn publish_recipe(&self, recipe_id: RecipeId) -> AppResult<()> {
        let mut recipe = self.recipe_repo
            .find_by_id(recipe_id)
            .await?
            .ok_or_else(|| AppError::not_found("Recipe"))?;

        recipe.status = RecipeStatus::Published;
        recipe.is_public = true;
        recipe.published_at = Some(OffsetDateTime::now_utc());
        recipe.updated_at = OffsetDateTime::now_utc();

        self.recipe_repo.update(&recipe).await?;
        Ok(())
    }

    /// Delete recipe and all related data
    pub async fn delete_recipe(&self, recipe_id: RecipeId) -> AppResult<()> {
        // Delete ingredients first (foreign key constraint)
        self.ingredient_repo.delete_by_recipe_id(recipe_id).await?;
        
        // Delete translations
        self.translation_service
            .translation_repo
            .delete_by_recipe_id(recipe_id)
            .await?;
        
        // Delete recipe
        self.recipe_repo.delete(recipe_id).await?;
        
        Ok(())
    }
}
