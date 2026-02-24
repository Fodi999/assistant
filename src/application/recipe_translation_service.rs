// Recipe Translation Service - AI-powered translations via Groq
use crate::domain::recipe_v2::{RecipeId, RecipeTranslation, TranslationSource};
use crate::infrastructure::LlmAdapter;
use crate::infrastructure::persistence::{
    RecipeTranslationRepositoryTrait, RecipeV2RepositoryTrait,
};
use crate::shared::{AppError, AppResult, Language};
use std::sync::Arc;

#[derive(Clone)]
pub struct RecipeTranslationService {
    pub translation_repo: Arc<dyn RecipeTranslationRepositoryTrait>,
    recipe_repo: Arc<dyn RecipeV2RepositoryTrait>,
    llm_adapter: Arc<LlmAdapter>,
}

impl RecipeTranslationService {
    pub fn new(
        translation_repo: Arc<dyn RecipeTranslationRepositoryTrait>,
        recipe_repo: Arc<dyn RecipeV2RepositoryTrait>,
        llm_adapter: Arc<LlmAdapter>,
    ) -> Self {
        Self {
            translation_repo,
            recipe_repo,
            llm_adapter,
        }
    }

    /// Translate recipe to target language using Groq AI
    /// Returns the created translation
    pub async fn translate_recipe(
        &self,
        recipe_id: RecipeId,
        tenant_id: crate::shared::TenantId,
        target_language: Language,
        force: bool,
    ) -> AppResult<RecipeTranslation> {
        // Load recipe with tenant isolation
        let recipe = self
            .recipe_repo
            .find_by_id(recipe_id, tenant_id)
            .await?
            .ok_or_else(|| AppError::not_found("Recipe"))?;

        // Don't translate to same language as default
        if recipe.language_default == target_language {
            return Err(AppError::validation(&format!(
                "Recipe is already in {}",
                target_language.code()
            )));
        }

        // Check if translation already exists and not forcing
        if !force {
            if let Some(existing) = self
                .translation_repo
                .find_by_recipe_and_language(recipe_id, target_language)
                .await?
            {
                return Ok(existing);
            }
        }

        // Translate name using Adapter (handles cache)
        tracing::debug!(
            "🔍 Translating recipe name: '{}' to {}",
            recipe.name_default,
            target_language.code()
        );
        let translated_name = self
            .llm_adapter
            .translate_to_language(&recipe.name_default, target_language.code())
            .await
            .map_err(|e| {
                tracing::error!(
                    "❌ AI translation failed for name '{}': {:?}",
                    recipe.name_default,
                    e
                );
                AppError::internal(&format!("Failed to translate name: {}", e))
            })?;
        tracing::debug!("✅ Name translated: '{}'", translated_name);

        // Translate instructions using Adapter (handles cache)
        tracing::debug!(
            "🔍 Translating recipe instructions (len={})",
            recipe.instructions_default.len()
        );
        let translated_instructions = self
            .llm_adapter
            .translate_to_language(&recipe.instructions_default, target_language.code())
            .await
            .map_err(|e| {
                tracing::error!("❌ AI translation failed for instructions: {:?}", e);
                AppError::internal(&format!("Failed to translate instructions: {}", e))
            })?;
        tracing::debug!(
            "✅ Instructions translated (len={})",
            translated_instructions.len()
        );

        // Create translation record
        let translation = RecipeTranslation::new(
            recipe_id,
            target_language,
            translated_name,
            translated_instructions,
            TranslationSource::AI,
        );

        // Save translation
        self.translation_repo.save(&translation).await?;

        Ok(translation)
    }

    /// Translate recipe to all other supported languages (async, non-blocking)
    /// Spawns background tasks for each language
    /// Returns immediately without waiting for translations to complete
    pub async fn translate_to_all_languages(
        &self,
        recipe_id: RecipeId,
        tenant_id: crate::shared::TenantId,
        source_language: Language,
        force: bool,
    ) -> AppResult<()> {
        let targets = Language::all();

        for target in targets {
            if target == source_language {
                continue;
            }

            // Note: In production we might want to use a job queue
            if let Err(e) = self.translate_recipe(recipe_id, tenant_id, target, force).await {
                tracing::error!("Translation failed for {}: {}", target.code(), e);
            }
        }

        Ok(())
    }

    /// Get localized recipe content (default or translated)
    /// Returns (name, instructions) in requested language
    pub async fn get_localized_content(
        &self,
        recipe_id: RecipeId,
        tenant_id: crate::shared::TenantId,
        language: Language,
    ) -> AppResult<(String, String)> {
        // Load recipe to get default language with tenant isolation
        let recipe = self
            .recipe_repo
            .find_by_id(recipe_id, tenant_id)
            .await?
            .ok_or_else(|| AppError::not_found("Recipe"))?;

        // If requesting default language, return default content
        if recipe.language_default == language {
            return Ok((recipe.name_default, recipe.instructions_default));
        }

        // Try to find translation
        match self
            .translation_repo
            .find_by_recipe_and_language(recipe_id, language)
            .await?
        {
            Some(translation) => Ok((translation.name, translation.instructions)),
            None => {
                // Translation not found, return default content as fallback
                Ok((recipe.name_default, recipe.instructions_default))
            }
        }
    }

    /// Get all translations for a recipe
    pub async fn get_all_translations(
        &self,
        recipe_id: RecipeId,
    ) -> AppResult<Vec<RecipeTranslation>> {
        self.translation_repo.find_by_recipe_id(recipe_id).await
    }
}
