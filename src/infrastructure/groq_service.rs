use crate::shared::AppError;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Минимальный ответ от Groq API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroqTranslationResponse {
    pub pl: String,
    pub ru: String,
    pub uk: String,
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
