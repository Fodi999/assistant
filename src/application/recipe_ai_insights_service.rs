use crate::application::recipe_validator::RecipeValidator;
use crate::domain::recipe_ai_insights::*;
use crate::domain::recipe_v2::{Recipe, RecipeId};
use crate::infrastructure::persistence::{RecipeAIInsightsRepository, RecipeV2RepositoryTrait};
use crate::infrastructure::LlmAdapter;
use crate::shared::AppError;
use std::sync::Arc;
use std::time::Instant;

pub struct RecipeAIInsightsService {
    llm_adapter: Arc<LlmAdapter>,
    repository: Arc<RecipeAIInsightsRepository>,
    recipe_repo: Arc<dyn RecipeV2RepositoryTrait>,
    validator: RecipeValidator,
}

impl RecipeAIInsightsService {
    pub fn new(
        llm_adapter: Arc<LlmAdapter>,
        repository: Arc<RecipeAIInsightsRepository>,
        recipe_repo: Arc<dyn RecipeV2RepositoryTrait>,
    ) -> Self {
        Self {
            llm_adapter,
            repository,
            recipe_repo,
            validator: RecipeValidator::new(),
        }
    }

    /// Generate AI insights for a recipe in specific language (by ID)
    /// This method fetches the recipe and delegates to generate_insights_for_recipe
    pub async fn generate_insights_by_id(
        &self,
        recipe_id: RecipeId,
        tenant_id: crate::shared::TenantId,
        target_language: &str,
    ) -> Result<RecipeAIInsightsResponse, AppError> {
        let recipe = self
            .recipe_repo
            .find_by_id(recipe_id, tenant_id)
            .await?
            .ok_or_else(|| AppError::not_found("Recipe"))?;

        self.generate_insights_for_recipe(&recipe, target_language)
            .await
    }

    /// Generate AI insights for a recipe in specific language
    /// This is the main method that calls Groq AI and saves results
    pub async fn generate_insights_for_recipe(
        &self,
        recipe: &Recipe,
        target_language: &str,
    ) -> Result<RecipeAIInsightsResponse, AppError> {
        let start_time = Instant::now();

        tracing::info!(
            "🤖 Generating AI insights for recipe {:?} (language: {})",
            recipe.id,
            target_language
        );

        // 🔍 STEP 1: Run rule-based validation BEFORE AI
        let validation_result = self.validator.validate(recipe);

        tracing::debug!(
            "📋 Validation: is_valid={}, errors={}, warnings={}",
            validation_result.is_valid,
            validation_result.errors.len(),
            validation_result.warnings.len()
        );

        // Build prompt for AI (with validation context)
        let prompt = self.build_analysis_prompt(recipe, target_language, &validation_result);

        // Call AI via Adapter (handles cache)
        let ai_response = self.llm_adapter.analyze_recipe(&prompt, &recipe.id.0.to_string()).await?;

        // Parse AI response
        let (steps, ai_validation, suggestions, _ai_feasibility_score) =
            self.parse_ai_response(&ai_response)?;

        // 🧠 RUST LOGIC: Calculate feasibility and final validation status in Rust (not AI)
        // AI can help with textual validation, but final status depends on our rule engine
        let final_is_valid = validation_result.is_valid && ai_validation.errors.is_empty();
        
        // Feasibility score: 100 - (errors * 30) - (warnings * 10)
        let feasibility_score = (100 - (validation_result.errors.len() as i32 * 30) - (validation_result.warnings.len() as i32 * 10)).max(0);

        // Save to database (extract UUID from RecipeId)
        let insights = self
            .repository
            .upsert(
                recipe.id.0, // Extract UUID
                target_language,
                steps,
                ai_validation,
                suggestions,
                feasibility_score,
                "gemini-2.5-flash",
            )
            .await?;

        let elapsed = start_time.elapsed().as_millis() as u64;
        tracing::info!("✅ Generated AI insights in {}ms", elapsed);

        Ok(RecipeAIInsightsResponse {
            insights,
            generated_in_ms: elapsed,
        })
    }

    /// Get or generate AI insights for a recipe in specific language (by ID)
    pub async fn get_or_generate_insights_by_id(
        &self,
        recipe_id: RecipeId,
        tenant_id: crate::shared::TenantId,
        target_language: &str,
    ) -> Result<RecipeAIInsightsResponse, AppError> {
        // Check if exists first
        if let Some(existing) = self
            .repository
            .get_by_recipe_and_language(recipe_id.as_uuid(), target_language)
            .await?
        {
            return Ok(existing.into());
        }

        // Generate if not exists
        self.generate_insights_by_id(recipe_id, tenant_id, target_language)
            .await
    }

    /// Refresh (force regenerate) insights
    pub async fn refresh_insights_by_id(
        &self,
        recipe_id: RecipeId,
        tenant_id: crate::shared::TenantId,
        target_language: &str,
    ) -> Result<RecipeAIInsightsResponse, AppError> {
        let recipe = self
            .recipe_repo
            .find_by_id(recipe_id, tenant_id)
            .await?
            .ok_or_else(|| AppError::not_found("Recipe"))?;

        self.generate_insights_for_recipe(&recipe, target_language)
            .await
    }

    /// Get insights for all languages
    pub async fn get_all_insights(
        &self,
        recipe_id: uuid::Uuid,
    ) -> Result<Vec<RecipeAIInsights>, AppError> {
        self.repository.get_all_by_recipe(recipe_id).await
    }

    /// Build prompt for AI analysis
    fn build_analysis_prompt(
        &self,
        recipe: &Recipe,
        _language: &str,
        validation: &crate::application::recipe_validator::ValidationResult,
    ) -> String {
        // Build validation context for AI
        let validation_context = if !validation.errors.is_empty() || !validation.warnings.is_empty()
        {
            let mut context = String::from("\n\n🔍 ПРЕДВАРИТЕЛЬНАЯ ВАЛИДАЦИЯ:\n");

            if !validation.errors.is_empty() {
                context.push_str("⚠️ КРИТИЧЕСКИЕ ОШИБКИ:\n");
                for error in &validation.errors {
                    context.push_str(&format!("  - [{}] {}\n", error.code, error.message));
                }
            }

            if !validation.warnings.is_empty() {
                context.push_str("⚡ ПРЕДУПРЕЖДЕНИЯ:\n");
                for warning in &validation.warnings {
                    context.push_str(&format!("  - [{}] {}\n", warning.code, warning.message));
                }
            }

            if !validation.missing_critical_ingredients.is_empty() {
                context.push_str("❌ ОТСУТСТВУЮТ КРИТИЧЕСКИЕ ИНГРЕДИЕНТЫ:\n");
                for missing in &validation.missing_critical_ingredients {
                    context.push_str(&format!("  - {}\n", missing));
                }
            }

            if let Some(ref dish_type) = validation.dish_type {
                context.push_str(&format!("📋 ТИП БЛЮДА: {:?}\n", dish_type));
            }

            context
        } else {
            String::new()
        };

        format!(
            r#"Ты — профессиональный технолог общественного питания с сертификацией HACCP.
Твоя задача — проанализировать рецепт и предоставить структурированные рекомендации.

ВАЖНЫЕ ПРАВИЛА:
1. НЕ выдумывай ингредиенты, которых нет в описании
2. Проверь логичность рецепта (можно ли приготовить указанное блюдо из данных ингредиентов)
3. Обрати внимание на безопасность пищевых продуктов
4. Проверь реалистичность времени приготовления
5. Укажи критические точки контроля (CCP) по стандарту HACCP
{}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

РЕЦЕПТ:
Название: {}
Инструкции: {}
Порций: {}

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

ТВОЯ ЗАДАЧА:
Предоставь анализ в формате JSON (ТОЛЬКО JSON, без других текстов):

{{
  "steps": [
    {{
      "step_number": 1,
      "action": "глагол действия",
      "description": "подробное описание",
      "duration_minutes": 10,
      "temperature": "180°C или null",
      "technique": "название техники или null",
      "ingredients_used": ["ингредиент1", "ингредиент2"]
    }}
  ],
  "validation": {{
    "is_valid": true,
    "warnings": [
      {{"severity": "warning", "code": "CODE", "message": "сообщение", "field": "field_name"}}
    ],
    "errors": [],
    "missing_ingredients": ["что добавить для полноценного блюда"],
    "safety_checks": ["Проверка X", "Контроль Y"]
  }},
  "suggestions": [
    {{
      "suggestion_type": "improvement или substitution",
      "title": "заголовок предложения",
      "description": "подробное описание",
      "impact": "вкус/текстура/аромат/безопасность",
      "confidence": 0.85
    }}
  ]
}}

(Note: Do NOT calculate feasibility score or numerical food cost. Just JSON text.)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
"#,
            validation_context, recipe.name_default, recipe.instructions_default, recipe.servings
        )
    }

    /// Parse AI response and extract structured data
    fn parse_ai_response(
        &self,
        response: &str,
    ) -> Result<
        (
            Vec<CookingStep>,
            RecipeValidation,
            Vec<RecipeSuggestion>,
            i32,
        ),
        AppError,
    > {
        #[derive(serde::Deserialize)]
        struct AIResponse {
            steps: Vec<CookingStep>,
            validation: RecipeValidation,
            suggestions: Vec<RecipeSuggestion>,
            #[serde(default)] // Allow missing feasibility score from AI
            feasibility_score: i32,
        }

        // Try to parse JSON directly
        let parsed: AIResponse = serde_json::from_str(response)
            .or_else(|_| {
                // Fallback: try to extract JSON from text
                if let Some(start) = response.find('{') {
                    if let Some(end) = response.rfind('}') {
                        let json_str = &response[start..=end];
                        return serde_json::from_str(json_str);
                    }
                }
                Err(serde_json::Error::io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "No valid JSON found in AI response",
                )))
            })
            .map_err(|e| {
                tracing::error!("❌ Failed to parse AI response: {:?}", e);
                tracing::debug!("Raw AI response: {}", response);
                AppError::internal("Failed to parse AI analysis")
            })?;

        // Validate feasibility score
        if parsed.feasibility_score < 0 || parsed.feasibility_score > 100 {
            return Err(AppError::validation(
                "Feasibility score must be between 0 and 100",
            ));
        }

        Ok((
            parsed.steps,
            parsed.validation,
            parsed.suggestions,
            parsed.feasibility_score,
        ))
    }
}
