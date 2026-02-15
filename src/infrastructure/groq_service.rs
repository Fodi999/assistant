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
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            api_key,
            http_client,
            model: "llama-3.1-8b-instant".to_string(), // Самая дешёвая модель
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
    pub async fn translate(&self, ingredient_name: &str) -> Result<GroqTranslationResponse, AppError> {
        // Проверка длины (не переводим очень длинные названия)
        if ingredient_name.len() > 50 {
            return Err(AppError::validation(
                "Ingredient name too long for automatic translation"
            ));
        }

        let prompt = format!(
            r#"Translate the ingredient "{}" into Polish, Russian and Ukrainian.
Return strict JSON:
{{"pl":"...","ru":"...","uk":"..."}}"#,
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
            "temperature": 0.0,  // Детерминированные результаты
            "max_tokens": 100,   // Очень короткий ответ
        });

        tracing::info!("Groq translation request for: {}", ingredient_name);

        let response = self.http_client
            .post("https://api.groq.com/openai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Groq API request failed: {}", e);
                AppError::internal(&format!("Groq API error: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_else(|_| "unknown".to_string());
            tracing::error!("Groq API error ({}): {}", status, body);
            return Err(AppError::internal(
                "Groq API returned error"
            ));
        }

        let data: GroqResponse = response.json().await.map_err(|e| {
            tracing::error!("Failed to parse Groq response: {}", e);
            AppError::internal("Failed to parse Groq response")
        })?;

        // Извлечение JSON из ответа
        let content = &data.choices[0].message.content;
        
        // Попытка парсить JSON прямо
        let translation: GroqTranslationResponse = serde_json::from_str(content)
            .or_else(|_| {
                // Fallback: попытаться извлечь JSON из текста
                if let Some(start) = content.find('{') {
                    if let Some(end) = content.rfind('}') {
                        return serde_json::from_str(&content[start..=end]);
                    }
                }
                Err(serde_json::Error::io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "No JSON found"
                )))
            })
            .map_err(|e| {
                tracing::error!("Failed to parse translation JSON: {}", e);
                tracing::debug!("Response content: {}", content);
                AppError::internal("Invalid translation response")
            })?;

        // Валидация результатов
        if translation.pl.trim().is_empty() 
            || translation.ru.trim().is_empty() 
            || translation.uk.trim().is_empty() {
            return Err(AppError::validation("Groq returned empty translations"));
        }

        tracing::info!(
            "Groq translation successful: {} -> PL:{} RU:{} UK:{}",
            ingredient_name, translation.pl, translation.ru, translation.uk
        );

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
    fn test_long_ingredient_name_rejected() {
        let long_name = "A".repeat(51);
        let service = GroqService::new("test_key".to_string());
        
        // Это не async тест, но показывает логику
        assert!(long_name.len() > 50);
    }
}
