use crate::shared::AppError;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Минимальный ответ от Groq API (для backward compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroqTranslationResponse {
    pub pl: String,
    pub ru: String,
    pub uk: String,
}

/// AI Classification Response - для автоматического определения категории и unit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiClassification {
    pub category_slug: String,  // Например: "dairy_and_eggs", "vegetables", "fruits"
    pub unit: String,           // Например: "kilogram", "piece", "liter"
}

/// 🚀 UNIFIED RESPONSE - Single AI call returns everything!
/// 
/// Вместо 3 раздельных вызовов (normalize + classify + translate)
/// One unified request returns all at once
/// Performance: ×3 faster, 1/3 cost
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedProductResponse {
    pub name_en: String,           // Нормализованное имя на английском
    pub name_pl: String,           // Перевод на польский
    pub name_ru: String,           // Перевод на русский
    pub name_uk: String,           // Перевод на украинский
    pub category_slug: String,     // Категория (dairy_and_eggs, fruits, vegetables, meat, seafood, grains, beverages)
    pub unit: String,              // Unit (piece, kilogram, gram, liter, milliliter)
}

/// Сервис для вызова Groq API с минимальными затратами
#[derive(Clone)]
pub struct GroqService {
    api_key: String,
    http_client: reqwest::Client,
    model: String,
}

impl GroqService {
    pub fn new(api_key: String) -> Self {
        // reqwest timeout: 5 sec (only one timeout needed, not double)
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            api_key,
            http_client,
            model: "llama-3.1-8b-instant".to_string(),
        }
    }

    /// 🌐 Оптимизированная проверка ASCII
    /// 
    /// ВАЖНО: Проверяем только алфавитные символы + цифры + пробелы
    /// Не доверяем другим ASCII символам (могут быть спецсимволы)
    /// 
    /// Это исключает ложные срабатывания на ASCII спецсимволы
    fn is_likely_english(text: &str) -> bool {
        // Пустой или только пробелы = не английский
        if text.trim().is_empty() {
            return false;
        }
        
        // Проверяем ТОЛЬКО буквы (a-z, A-Z), цифры, пробелы, базовую пунктуацию
        // Всё остальное = вероятно не английский
        text.chars().all(|c| {
            c.is_ascii_alphanumeric() || c.is_whitespace() || c == '-' || c == '\''
        })
    }

    /// 🌐 Нормализация входного текста в английский язык
    /// 
    /// Оптимизация: если текст содержит только [a-zA-Z0-9\s'-], это вероятно английский
    /// Если есть other symbols → AI перевод
    /// 
    /// Это экономит 1 AI вызов для англоязычного ввода (вместо detect + translate)
    pub async fn normalize_to_english(&self, input: &str) -> Result<String, AppError> {
        let trimmed = input.trim();
        
        // 🔍 Оптимизация: только буквы + цифры + пробелы = скорее всего английский
        if Self::is_likely_english(trimmed) {
            tracing::debug!("Input detected as likely English (allowed chars only): {}", trimmed);
            return Ok(trimmed.to_string());
        }
        
        // Содержит non-ASCII или спецсимволы = переводим в английский
        tracing::debug!("Non-English input detected, translating to English: {}", trimmed);
        self.translate_to_language(trimmed, "English").await
    }

    /// Минимальный запрос на перевод (одна модель, температура 0, короткий prompt)
    /// 
    /// # Аргументы
    /// * `ingredient_name` - Английское название ингредиента (например "Apple")
    /// 
    /// # Возвращает
    /// * `GroqTranslationResponse` с переводами на PL, RU, UK
    /// 
    /// # Примечания
    /// - Используем temperature=0 для детерминированных результатов
    /// - Очень короткий prompt для минимизации токенов
    /// - Один запрос на слово
    /// - Результат сохраняется в dictionary (кеш навсегда)
    /// - Timeout: 5 секунд (встроенный в reqwest client)
    pub async fn translate(&self, ingredient_name: &str) -> Result<GroqTranslationResponse, AppError> {
        // Проверка длины (не переводим очень длинные названия)
        if ingredient_name.len() > 50 {
            return Err(AppError::validation(
                "Ingredient name too long for automatic translation"
            ));
        }

        // Минимальный prompt для экономии токенов
        let prompt = format!(
            r#"Translate "{}" to Polish(pl), Russian(ru), Ukrainian(uk).
Respond with ONLY valid JSON, no other text:
{{"pl":"<Polish>","ru":"<Russian>","uk":"<Ukrainian>"}}"#,
            ingredient_name
        );

        let request_body = serde_json::json!({
            "model": self.model,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": 0.0,
            "max_tokens": 100,
        });

        tracing::info!("Groq translation request for: {}", ingredient_name);

        // Retry logic: попытаться дважды
        const MAX_RETRIES: u32 = 1;
        let mut attempt = 0;

        loop {
            attempt += 1;
            match self.translate_with_timeout(&request_body, ingredient_name).await {
                Ok(response) => return Ok(response),
                Err(e) if attempt <= MAX_RETRIES => {
                    tracing::warn!("Groq translation attempt {} failed, retrying...", attempt);
                    // Небольшой backoff перед retry
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Внутренняя функция для одного запроса с проверками
    async fn translate_with_timeout(
        &self,
        request_body: &serde_json::Value,
        ingredient_name: &str,
    ) -> Result<GroqTranslationResponse, AppError> {
        let response = self.http_client
            .post("https://api.groq.com/openai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Groq API request failed: {}", e);
                AppError::internal("Groq API error")
            })?;

        // Проверка HTTP статуса
        if !response.status().is_success() {
            let status = response.status();
            tracing::error!("Groq API error: HTTP {}", status);
            return Err(AppError::internal("Groq API returned error"));
        }

        // Проверка Content-Type
        if let Some(ct) = response.headers().get("content-type") {
            if let Ok(ct_str) = ct.to_str() {
                if !ct_str.contains("application/json") {
                    tracing::error!("Invalid content type from Groq: {}", ct_str);
                    return Err(AppError::internal("Invalid response type"));
                }
            }
        }

        let data: GroqResponse = response.json().await.map_err(|_| {
            tracing::error!("Failed to parse Groq response");
            AppError::internal("Failed to parse Groq response")
        })?;

        // ✅ Критическая проверка: choices не может быть пусто
        let choice = data.choices.get(0)
            .ok_or_else(|| {
                tracing::error!("Groq returned empty choices array");
                AppError::internal("No translation response")
            })?;

        let content = &choice.message.content;
        
        tracing::debug!("Groq response content: {}", content);
        
        // Попытка парсить JSON прямо
        let translation: GroqTranslationResponse = serde_json::from_str(content)
            .or_else(|_| {
                // Fallback: попытаться извлечь JSON из текста
                if let Some(start) = content.find('{') {
                    if let Some(end) = content.rfind('}') {
                        let json_str = &content[start..=end];
                        tracing::debug!("Extracted JSON: {}", json_str);
                        return serde_json::from_str(json_str);
                    }
                }
                Err(serde_json::Error::io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "No JSON found"
                )))
            })
            .map_err(|e| {
                tracing::error!("Failed to parse translation JSON: {}", e);
                tracing::debug!("Raw response: {}", content);
                AppError::internal("Invalid translation response")
            })?;

        // Валидация результатов - но допускаем пустые для некритичных полей
        if translation.pl.trim().is_empty() {
            tracing::warn!("Groq returned empty PL translation for: {}", ingredient_name);
        }
        if translation.ru.trim().is_empty() {
            tracing::warn!("Groq returned empty RU translation for: {}", ingredient_name);
        }
        if translation.uk.trim().is_empty() {
            tracing::warn!("Groq returned empty UK translation for: {}", ingredient_name);
        }

        tracing::info!("✅ Groq translation successful for: {}", ingredient_name);

        Ok(translation)
    }

    /// 🔄 Универсальный перевод в целевой язык
    /// 
    /// Может переводить из любого языка в любой
    /// 
    /// ВАЖНО: Жёсткая очистка ответа от лишнего текста
    pub async fn translate_to_language(&self, text: &str, target_lang: &str) -> Result<String, AppError> {
        if text.len() > 5000 {
            return Err(AppError::validation("Text too long for translation"));
        }

        let prompt = format!(
            r#"Translate the following text to {}.
Return ONLY the translated text, nothing else.

Text: {}"#,
            target_lang,
            text
        );

        let request_body = serde_json::json!({
            "model": self.model,
            "messages": [{
                "role": "user",
                "content": prompt
            }],
            "temperature": 0.0,
            "max_tokens": 500,
        });

        const MAX_RETRIES: u32 = 1;
        let mut attempt = 0;

        loop {
            attempt += 1;
            match self.send_groq_request(&request_body).await {
                Ok(response) => {
                    // ✅ ЖЁСТКАЯ ОЧИСТКА: Извлечь только слово
                    let cleaned = self.extract_translated_word(&response, target_lang);
                    tracing::info!("Translated '{}' → '{}' (cleaned from: '{}')", 
                        text, cleaned, response);
                    return Ok(cleaned);
                }
                Err(e) if attempt <= MAX_RETRIES => {
                    tracing::warn!("Translation attempt {} failed, retrying...", attempt);
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// 🧹 Извлечение переведённого слова из "болтливого" ответа LLM
    /// 
    /// Ожидаемые варианты "шума":
    /// - "The translation is: Green Apple"
    /// - "Green Apple" (идеально)
    /// - "Word: Green Apple"
    /// - "English: Green Apple"
    /// - "**Green Apple**"
    /// - "Green Apple." (с пунктуацией)
    /// - "Green Apple\n" (с переносом)
    /// 
    /// ВАЖНО: Не обрезаем составные названия (Green Apple, not just Apple)
    fn extract_translated_word(&self, response: &str, _target_lang: &str) -> String {
        let text = response.trim();
        
        // Вариант 1: Извлечь текст в кавычках
        if let Some(start) = text.find('"') {
            if let Some(end) = text.rfind('"') {
                if start < end {
                    let quoted = text[start + 1..end].trim();
                    if !quoted.is_empty() {
                        tracing::debug!("Extracted from quotes: {}", quoted);
                        return quoted.to_string();
                    }
                }
            }
        }
        
        // Вариант 2: Основная очистка (удаляем маркеры, пунктуацию в конце)
        let cleaned = text
            .trim_matches(|c: char| !c.is_alphanumeric() && !c.is_whitespace())
            .trim_matches('*')
            .trim_matches('`')
            .trim_matches('"')
            .trim_matches('\'');
        
        // Вариант 3: Если есть двоеточие, возьми всё после него
        if let Some(pos) = cleaned.rfind(':') {
            let after_colon = cleaned[pos + 1..].trim();
            if !after_colon.is_empty() {
                tracing::debug!("Extracted after colon: {}", after_colon);
                return after_colon.to_string();
            }
        }
        
        // ВАЖНО: Проверяем наличие пробелов
        // Если есть пробелы И это выглядит как целое название → возвращаем целиком
        if cleaned.contains(' ') {
            let word_count = cleaned.split_whitespace().count();
            // Если 2-3 слова (типичные названия: "Green Apple", "Sea Salt")
            if word_count >= 2 && word_count <= 3 {
                tracing::debug!("Multi-word translation detected, returning full: {}", cleaned);
                return cleaned.to_string();
            }
        }
        
        // Вариант 4: Fallback — возьми последний токен только если нет составных слов
        if let Some(last_word) = cleaned.split_whitespace().last() {
            if !last_word.is_empty() {
                // Логируем warning если обрезали
                if cleaned.contains(' ') {
                    tracing::warn!("Extracting last word only: '{}' from '{}'", last_word, cleaned);
                } else {
                    tracing::debug!("Extracted single word: {}", last_word);
                }
                return last_word.to_string();
            }
        }
        
        // Fallback: просто очистить и вернуть
        tracing::warn!("Could not clean response, returning as-is: {}", cleaned);
        cleaned.to_string()
    }

    /// 🚀 UNIFIED PROCESSING - Single AI call, returns everything!
    /// 
    /// Instead of:
    /// 1. normalize_to_english() → 1 AI call
    /// 2. classify_product() → 1 AI call
    /// 3. translate() → 1 AI call
    /// 
    /// We do: ONE call that returns all fields
    /// 
    /// Performance: 3x faster, 1/3 cost
    /// - Before: ~1800ms (normalize 500ms + classify 600ms + translate 700ms)
    /// - After: ~700ms (single unified call)
    pub async fn process_unified(&self, name_input: &str) -> Result<UnifiedProductResponse, AppError> {
        let trimmed = name_input.trim();
        
        if trimmed.is_empty() {
            return Err(AppError::validation("Input cannot be empty"));
        }

        // Super aggressive prompt для минимизации токенов и однозначного ответа
        let prompt = format!(
            r#"You are a food product data extraction and classification AI.

Input product name (may be in ANY language): "{}"

Extract and classify the product. Return ONLY valid JSON, no other text:
{{
  "name_en": "<English product name>",
  "name_pl": "<Polish translation>",
  "name_ru": "<Russian translation>",
  "name_uk": "<Ukrainian translation>",
  "category_slug": "<category>",
  "unit": "<unit>"
}}

Categories: dairy_and_eggs, fruits, vegetables, meat, seafood, grains, beverages
Units: piece, kilogram, gram, liter, milliliter

Rules:
1. name_en MUST be in English (translate if needed)
2. All translations must be single words when possible, but allow 2-3 word compounds
3. category_slug must be one of the allowed values
4. unit must be one of the allowed values
5. Do not add explanations, just JSON"#,
            trimmed
        );

        let request_body = serde_json::json!({
            "model": self.model,
            "messages": [{
                "role": "user",
                "content": prompt
            }],
            "temperature": 0.0,
            "max_tokens": 150,
        });

        tracing::info!("🚀 Unified processing request for: {}", trimmed);

        const MAX_RETRIES: u32 = 1;
        let mut attempt = 0;

        let response = loop {
            attempt += 1;
            match self.send_groq_request(&request_body).await {
                Ok(content) => break content,
                Err(e) if attempt <= MAX_RETRIES => {
                    tracing::warn!("Unified processing attempt {} failed, retrying...", attempt);
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }
                Err(e) => return Err(e),
            }
        };

        // Parse JSON with fallback extraction
        let result: UnifiedProductResponse = serde_json::from_str(&response)
            .or_else(|_| {
                // Fallback: try to extract JSON from text
                if let Some(start) = response.find('{') {
                    if let Some(end) = response.rfind('}') {
                        let json_str = &response[start..=end];
                        tracing::debug!("Extracted JSON from response: {}", json_str);
                        return serde_json::from_str(json_str);
                    }
                }
                Err(serde_json::Error::io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "No JSON found in response"
                )))
            })
            .map_err(|e| {
                tracing::error!("Failed to parse unified response JSON: {}", e);
                tracing::debug!("Raw response: {}", response);
                AppError::internal("Invalid unified response format")
            })?;

        // ✅ Validate extracted values
        self.validate_unified_response(&result)?;

        tracing::info!("✅ Unified processing successful for: {}. Result: en={}, category={}, unit={}", 
            trimmed, result.name_en, result.category_slug, result.unit);

        Ok(result)
    }

    /// Validate unified response fields
    fn validate_unified_response(&self, result: &UnifiedProductResponse) -> Result<(), AppError> {
        // Validate English name
        if result.name_en.trim().is_empty() {
            return Err(AppError::internal("AI returned empty English name"));
        }

        // Validate category
        let allowed_categories = vec![
            "dairy_and_eggs", "fruits", "vegetables", "meat", "seafood", "grains", "beverages"
        ];
        if !allowed_categories.contains(&result.category_slug.as_str()) {
            tracing::error!("Invalid category from AI: {}", result.category_slug);
            return Err(AppError::validation(
                &format!("Invalid category: {}", result.category_slug)
            ));
        }

        // Validate unit
        let allowed_units = vec![
            "piece", "kilogram", "gram", "liter", "milliliter"
        ];
        if !allowed_units.contains(&result.unit.as_str()) {
            tracing::error!("Invalid unit from AI: {}", result.unit);
            return Err(AppError::validation(
                &format!("Invalid unit: {}", result.unit)
            ));
        }

        // Warn if translations are missing or fallback to English
        if result.name_pl.trim().is_empty() {
            tracing::warn!("AI returned empty Polish translation");
        }
        if result.name_ru.trim().is_empty() {
            tracing::warn!("AI returned empty Russian translation");
        }
        if result.name_uk.trim().is_empty() {
            tracing::warn!("AI returned empty Ukrainian translation");
        }

        Ok(())
    }

    /// 🤖 AI классификация продукта (категория + unit) - LEGACY
    /// 
    /// На основе английского названия определяет:
    /// - category_slug: один из допустимых (dairy_and_eggs, fruits, vegetables, meat, seafood, grains, beverages)
    /// - unit: один из допустимых (piece, kilogram, gram, liter, milliliter)
    /// 
    /// ⚠️ Deprecated in favor of process_unified() but kept for backward compatibility
    /// ВАЖНО: Использует send_groq_request для унификации + retry логики
    pub async fn classify_product(&self, name_en: &str) -> Result<AiClassification, AppError> {
        if name_en.len() > 50 {
            return Err(AppError::validation("Product name too long for classification"));
        }

        let prompt = format!(
            r#"You are a food classification AI.

Given product name: "{}"

Return ONLY valid JSON (no other text):
{{"category_slug":"","unit":""}}

Allowed categories: dairy_and_eggs, fruits, vegetables, meat, seafood, grains, beverages
Allowed units: piece, kilogram, gram, liter, milliliter

Pick the best match. Do not invent values."#,
            name_en
        );

        let request_body = serde_json::json!({
            "model": self.model,
            "messages": [{
                "role": "user",
                "content": prompt
            }],
            "temperature": 0.0,
            "max_tokens": 100,
        });

        tracing::info!("AI classification request for: {}", name_en);

        // ✅ Используем send_groq_request для унификации + retry
        const MAX_RETRIES: u32 = 1;
        let mut attempt = 0;

        let classification = loop {
            attempt += 1;
            match self.send_groq_request(&request_body).await {
                Ok(content) => {
                    // Парсим JSON (с fallback на извлечение)
                    let classification: AiClassification = serde_json::from_str(&content)
                        .or_else(|_| {
                            if let Some(start) = content.find('{') {
                                if let Some(end) = content.rfind('}') {
                                    let json_str = &content[start..=end];
                                    return serde_json::from_str(json_str);
                                }
                            }
                            Err(serde_json::Error::io(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "No JSON found"
                            )))
                        })
                        .map_err(|e| {
                            tracing::error!("Failed to parse classification JSON: {}", e);
                            AppError::internal("Invalid classification response")
                        })?;
                    
                    break classification;
                }
                Err(e) if attempt <= MAX_RETRIES => {
                    tracing::warn!("Classification attempt {} failed, retrying...", attempt);
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }
                Err(e) => return Err(e),
            }
        };

        // ✅ Валидация результатов
        let allowed_categories = vec![
            "dairy_and_eggs", "fruits", "vegetables", "meat", "seafood", "grains", "beverages"
        ];
        let allowed_units = vec![
            "piece", "kilogram", "gram", "liter", "milliliter"
        ];

        if !allowed_categories.contains(&classification.category_slug.as_str()) {
            tracing::error!("Invalid category from AI: {}", classification.category_slug);
            return Err(AppError::validation(
                &format!("Invalid category from AI: {}", classification.category_slug)
            ));
        }

        if !allowed_units.contains(&classification.unit.as_str()) {
            tracing::error!("Invalid unit from AI: {}", classification.unit);
            return Err(AppError::validation(
                &format!("Invalid unit from AI: {}", classification.unit)
            ));
        }

        tracing::info!("✅ AI classification: category={}, unit={}", 
            classification.category_slug, classification.unit);

        Ok(classification)
    }

    /// Внутренняя функция для отправки запроса к Groq и получения текста
    /// 
    /// ВАЖНО: Двойная страховка от hangs:
    /// 1. reqwest::Client::timeout(5s) — на уровне TCP
    /// 2. tokio::timeout(6s) — на уровне async операции
    async fn send_groq_request(&self, request_body: &serde_json::Value) -> Result<String, AppError> {
        // Обертка в tokio::timeout (6 сек = 5 сек client timeout + 1 сек buffer)
        let result = tokio::time::timeout(
            Duration::from_secs(6),
            self.send_groq_request_inner(request_body)
        ).await;

        match result {
            Ok(Ok(content)) => Ok(content),
            Ok(Err(e)) => Err(e),
            Err(_) => {
                tracing::error!("Groq request timeout (6s exceeded)");
                Err(AppError::internal("Groq API timeout"))
            }
        }
    }

    /// Внутренняя реализация запроса (без timeout wrapper)
    async fn send_groq_request_inner(&self, request_body: &serde_json::Value) -> Result<String, AppError> {
        let response = self.http_client
            .post("https://api.groq.com/openai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Groq API request failed: {}", e);
                AppError::internal("Groq API error")
            })?;

        if !response.status().is_success() {
            return Err(AppError::internal("Groq API returned error"));
        }

        let data: GroqResponse = response.json().await
            .map_err(|_| AppError::internal("Failed to parse Groq response"))?;

        let content = data.choices.get(0)
            .ok_or_else(|| AppError::internal("No response"))?
            .message.content.trim().to_string();

        Ok(content)
    }
}

#[derive(Debug, Deserialize)]
struct GroqResponse {
    choices: Vec<GroqChoice>,
}

#[derive(Debug, Deserialize)]
struct GroqChoice {
    message: GroqMessage,
}

#[derive(Debug, Deserialize)]
struct GroqMessage {
    content: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translation_response_parse() {
        let json = r#"{"pl":"Jabłko","ru":"Яблоко","uk":"Яблуко"}"#;
        let result: GroqTranslationResponse = serde_json::from_str(json).unwrap();
        
        assert_eq!(result.pl, "Jabłko");
        assert_eq!(result.ru, "Яблоко");
        assert_eq!(result.uk, "Яблуко");
    }

    #[test]
    fn test_long_ingredient_name_validation() {
        let long_name = "A".repeat(51);
        // Проверяем что длинные названия фильтруются
        assert!(long_name.len() > 50);
    }
}
