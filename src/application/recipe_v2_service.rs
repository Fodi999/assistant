// Recipe V2 Service - Recipe management with automatic translations
use crate::application::recipe_translation_service::RecipeTranslationService;
use crate::domain::recipe_v2::{Recipe, RecipeId, RecipeIngredient, RecipeStatus};
use crate::domain::CatalogIngredientId;
use crate::infrastructure::persistence::{
    CatalogIngredientRepositoryTrait, RecipeIngredientRepositoryTrait, RecipeV2RepositoryTrait,
};
use crate::infrastructure::R2Client;
use crate::shared::{AppError, AppResult, Language, TenantId, UserId};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
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
    pub image_url: Option<String>,
    pub ingredients: Vec<CreateRecipeIngredientDto>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRecipeIngredientDto {
    pub catalog_ingredient_id: Uuid,
    pub quantity: Decimal,
    pub unit: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRecipeDto {
    pub name: String,
    pub instructions: String,
    pub language: Language,
    pub servings: i32,
    pub image_url: Option<String>,
    pub ingredients: Vec<CreateRecipeIngredientDto>,
}

#[derive(Debug, Serialize, Clone)]
pub struct RecipeResponseDto {
    pub id: Uuid,
    pub name: String,
    pub instructions: String,
    pub language: Language,
    pub servings: i32,
    pub image_url: Option<String>,
    pub total_cost_cents: Option<i32>,
    pub cost_per_serving_cents: Option<i32>,
    pub status: String,
    pub is_public: bool,
    pub published_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub ingredients: Vec<RecipeIngredientResponseDto>,
}

#[derive(Debug, Serialize, Clone)]
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
    r2_client: R2Client,
    pool: PgPool,
}

impl RecipeV2Service {
    pub fn new(
        recipe_repo: Arc<dyn RecipeV2RepositoryTrait>,
        ingredient_repo: Arc<dyn RecipeIngredientRepositoryTrait>,
        catalog_repo: Arc<dyn CatalogIngredientRepositoryTrait>,
        translation_service: Arc<RecipeTranslationService>,
        r2_client: R2Client,
        pool: PgPool,
    ) -> Self {
        Self {
            recipe_repo,
            ingredient_repo,
            catalog_repo,
            translation_service,
            r2_client,
            pool,
        }
    }

    /// Upload image to R2 for a specific recipe
    pub async fn upload_image(
        &self,
        recipe_id: RecipeId,
        tenant_id: TenantId,
        file_data: Vec<u8>,
        content_type: &str,
    ) -> AppResult<String> {
        // Verify recipe belongs to tenant
        let exists = self.recipe_repo.find_by_id(recipe_id, tenant_id).await?.is_some();
        if !exists {
            return Err(AppError::not_found("Recipe"));
        }

        let key = format!("recipes/{}/{}.webp", tenant_id.as_uuid(), recipe_id.as_uuid());
        
        // Use Bytes to avoid unnecessary copies (R2Client uses bytes::Bytes)
        let bytes = bytes::Bytes::from(file_data);
        
        let image_url = self
            .r2_client
            .upload_image(&key, bytes, content_type)
            .await?;

        // Update database
        sqlx::query("UPDATE recipes SET image_url = $1 WHERE id = $2 AND tenant_id = $3")
            .bind(&image_url)
            .bind(recipe_id.as_uuid())
            .bind(tenant_id.as_uuid())
            .execute(&self.pool)
            .await?;

        Ok(image_url)
    }

    /// Update recipe image URL (for direct uploads)
    pub async fn save_image_url(
        &self,
        recipe_id: RecipeId,
        tenant_id: TenantId,
        image_url: String,
    ) -> AppResult<()> {
        sqlx::query("UPDATE recipes SET image_url = $1 WHERE id = $2 AND tenant_id = $3")
            .bind(image_url)
            .bind(recipe_id.as_uuid())
            .bind(tenant_id.as_uuid())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Get presigned URL for direct recipe image upload
    pub async fn get_image_upload_url(
        &self,
        recipe_id: RecipeId,
        tenant_id: TenantId,
        content_type: &str,
    ) -> AppResult<crate::application::user::AvatarUploadResponse> {
        let key = format!("recipes/{}/{}.webp", tenant_id.as_uuid(), recipe_id.as_uuid());
        
        let upload_url = self.r2_client.generate_presigned_upload_url(&key, content_type).await?;
        let public_url = self.r2_client.get_public_url(&key);

        Ok(crate::application::user::AvatarUploadResponse {
            upload_url,
            public_url,
        })
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
            let ingredient_id =
                CatalogIngredientId::from_uuid(ingredient_dto.catalog_ingredient_id);
            let exists = self.catalog_repo.find_by_id(ingredient_id).await?.is_some();

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
            image_url: dto.image_url,
            servings: dto.servings,
            total_cost_cents: None, // Will be calculated from inventory later
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
            let ingredient_id =
                CatalogIngredientId::from_uuid(ingredient_dto.catalog_ingredient_id);
            let catalog_ingredient = self
                .catalog_repo
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
                cost_at_use_cents: None, // Will be calculated from inventory later
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
        let t_id = tenant_id;
        tokio::spawn(async move {
            if let Err(e) = translation_service
                .translate_to_all_languages(recipe_id, t_id, default_language, false)
                .await
            {
                tracing::error!(
                    "Failed to trigger translations for recipe {}: {}",
                    recipe_id.as_uuid(),
                    e
                );
            }
        });

        // Return response
        Ok(RecipeResponseDto {
            id: recipe.id.as_uuid(),
            name: recipe.name_default,
            instructions: recipe.instructions_default,
            language: recipe.language_default,
            servings: recipe.servings,
            image_url: recipe.image_url,
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
        id: RecipeId,
        tenant_id: TenantId,
        language: Language,
    ) -> AppResult<RecipeResponseDto> {
        let recipe = self
            .recipe_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| AppError::not_found("Recipe"))?;

        let (name, instructions) = if language == recipe.language_default {
            (recipe.name_default.clone(), recipe.instructions_default.clone())
        } else {
            // Find translation
            let translation = sqlx::query(
                "SELECT name, instructions FROM recipe_translations WHERE recipe_id = $1 AND language = $2",
            )
            .bind(recipe.id.as_uuid())
            .bind(language.code())
            .fetch_optional(&self.pool)
            .await?;

            match translation {
                Some(row) => (
                    row.try_get("name")?,
                    row.try_get("instructions")?,
                ),
                None => (recipe.name_default.clone(), recipe.instructions_default.clone()),
            }
        };

        let response_ingredients = self.get_response_ingredients(recipe.id, language).await?;

        Ok(RecipeResponseDto {
            id: recipe.id.as_uuid(),
            name,
            instructions,
            language,
            servings: recipe.servings,
            image_url: recipe.image_url,
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

    /// List all user's recipes
    pub async fn list_user_recipes(
        &self,
        user_id: UserId,
        tenant_id: TenantId,
        language: Language,
    ) -> AppResult<Vec<RecipeResponseDto>> {
        let recipes = self.recipe_repo.find_by_user_id(user_id, tenant_id).await?;

        let mut results = Vec::new();
        for recipe in recipes {
            let (name, instructions) = if language == recipe.language_default {
                (recipe.name_default.clone(), recipe.instructions_default.clone())
            } else {
                // Find translation (cached/pre-calculated if possible, but keep it simple for now)
                let translation = sqlx::query(
                    "SELECT name, instructions FROM recipe_translations WHERE recipe_id = $1 AND language = $2",
                )
                .bind(recipe.id.as_uuid())
                .bind(language.code())
                .fetch_optional(&self.pool)
                .await?;

                match translation {
                    Some(row) => (
                        row.try_get("name")?,
                        row.try_get("instructions")?,
                    ),
                    None => (recipe.name_default.clone(), recipe.instructions_default.clone()),
                }
            };

            let response_ingredients = self.get_response_ingredients(recipe.id, language).await?;

            results.push(RecipeResponseDto {
                id: recipe.id.as_uuid(),
                name,
                instructions,
                language,
                servings: recipe.servings,
                image_url: recipe.image_url,
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

        Ok(results)
    }

    /// Publish recipe (make it public or active)
    pub async fn publish_recipe(&self, recipe_id: RecipeId, tenant_id: TenantId) -> AppResult<()> {
        let mut recipe = self
            .recipe_repo
            .find_by_id(recipe_id, tenant_id)
            .await?
            .ok_or_else(|| AppError::not_found("Recipe"))?;

        recipe.status = RecipeStatus::Published;
        recipe.is_public = true;
        recipe.published_at = Some(OffsetDateTime::now_utc());
        recipe.updated_at = OffsetDateTime::now_utc();

        self.recipe_repo.update(&recipe).await?;
        Ok(())
    }

    /// Delete recipe
    pub async fn delete_recipe(&self, recipe_id: RecipeId, tenant_id: TenantId) -> AppResult<()> {
        // Enforce tenant isolation by checking if it exists for this tenant
        let exists = self
            .recipe_repo
            .find_by_id(recipe_id, tenant_id)
            .await?
            .is_some();

        if !exists {
            return Err(AppError::not_found("Recipe"));
        }

        self.recipe_repo.delete(recipe_id, tenant_id).await
    }

    /// Update an existing recipe
    pub async fn update_recipe(
        &self,
        id: RecipeId, // Use id here
        dto: UpdateRecipeDto,
        tenant_id: TenantId,
    ) -> AppResult<RecipeResponseDto> {
        let mut recipe = self
            .recipe_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| AppError::not_found("Recipe"))?;

        // Update basic fields
        recipe.name_default = dto.name;
        recipe.instructions_default = dto.instructions;
        recipe.language_default = dto.language;
        recipe.servings = dto.servings;
        recipe.image_url = dto.image_url;
        recipe.updated_at = OffsetDateTime::now_utc();

        // Save updated recipe
        self.recipe_repo.update(&recipe).await?;

        // Update ingredients (easiest way: delete all and re-insert)
        self.ingredient_repo.delete_by_recipe_id(id).await?;

        let mut response_ingredients = Vec::new();
        let now = OffsetDateTime::now_utc();

        for ingredient_dto in &dto.ingredients {
            let ingredient_id =
                CatalogIngredientId::from_uuid(ingredient_dto.catalog_ingredient_id);
            let catalog_ingredient = self
                .catalog_repo
                .find_by_id(ingredient_id)
                .await?
                .ok_or_else(|| AppError::not_found("Catalog ingredient"))?;

            let ingredient_name = catalog_ingredient.name(recipe.language_default).to_string();

            let ingredient = RecipeIngredient {
                id: crate::domain::recipe_v2::RecipeIngredientId::new(),
                recipe_id: id,
                catalog_ingredient_id: ingredient_dto.catalog_ingredient_id,
                quantity: ingredient_dto.quantity,
                unit: ingredient_dto.unit.clone(),
                cost_at_use_cents: None,
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

        // Trigger automatic re-translation
        let translation_service = self.translation_service.clone();
        let default_language = dto.language;
        let t_id = tenant_id;
        let r_id = id;
        tokio::spawn(async move {
            if let Err(e) = translation_service
                .translate_to_all_languages(r_id, t_id, default_language, true)
                .await
            {
                tracing::error!(
                    "Failed to trigger re-translations for recipe {}: {}",
                    r_id.as_uuid(),
                    e
                );
            }
        });

        Ok(RecipeResponseDto {
            id: recipe.id.as_uuid(),
            name: recipe.name_default,
            instructions: recipe.instructions_default,
            language: recipe.language_default,
            servings: recipe.servings,
            image_url: recipe.image_url,
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

    /// Internal helper to get ingredients for response
    async fn get_response_ingredients(
        &self,
        recipe_id: RecipeId,
        _language: Language,
    ) -> AppResult<Vec<RecipeIngredientResponseDto>> {
        let ingredients = self.ingredient_repo.find_by_recipe_id(recipe_id).await?;
        
        Ok(ingredients
            .into_iter()
            .map(|i| RecipeIngredientResponseDto {
                id: i.id.as_uuid(),
                catalog_ingredient_id: i.catalog_ingredient_id,
                catalog_ingredient_name: i.catalog_ingredient_name_snapshot,
                quantity: i.quantity,
                unit: i.unit,
                cost_at_use_cents: i.cost_at_use_cents,
            })
            .collect())
    }
}
