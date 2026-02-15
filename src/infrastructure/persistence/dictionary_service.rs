use crate::shared::AppError;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// Словарь переводов ингредиентов из таблицы ingredient_dictionary
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DictionaryEntry {
    pub id: Uuid,
    pub name_en: String,
    pub name_pl: String,
    pub name_ru: String,
    pub name_uk: String,
}

/// Сервис для управления локальным кешем переводов
#[derive(Clone)]
pub struct DictionaryService {
    pool: PgPool,
}

impl DictionaryService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Поиск перевода по английскому названию
    /// 
    /// # Аргументы
    /// * `name_en` - Английское название (case-insensitive поиск)
    /// 
    /// # Возвращает
    /// * `Some(DictionaryEntry)` если найдено в кеше
    /// * `None` если нет в кеше (нужен Groq)
    pub async fn find_by_en(&self, name_en: &str) -> Result<Option<DictionaryEntry>, AppError> {
        let result = sqlx::query_as::<_, DictionaryEntry>(
            "SELECT id, name_en, name_pl, name_ru, name_uk 
             FROM ingredient_dictionary 
             WHERE LOWER(TRIM(name_en)) = LOWER(TRIM($1))"
        )
        .bind(name_en)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Dictionary lookup failed: {}", e);
            AppError::internal(&format!("Failed to lookup dictionary: {}", e))
        })?;

        Ok(result)
    }

    /// Сохранить новый перевод в словарь (кеш навсегда)
    /// 
    /// # Аргументы
    /// * `name_en` - Английское название
    /// * `name_pl` - Польский перевод
    /// * `name_ru` - Русский перевод
    /// * `name_uk` - Украинский перевод
    /// 
    /// # Race Condition Safety
    /// Использует ON CONFLICT (LOWER(name_en)) DO NOTHING
    /// При конфликте возвращает существующую запись (findal lookup)
    pub async fn insert(
        &self,
        name_en: &str,
        name_pl: &str,
        name_ru: &str,
        name_uk: &str,
    ) -> Result<DictionaryEntry, AppError> {
        let id = Uuid::new_v4();
        let name_en_trimmed = name_en.trim();

        // 🔒 Race condition safe: ON CONFLICT DO NOTHING
        // Если race condition - другой процесс уже вставил запись
        // Мы её не обновляем, а просто игнорируем конфликт
        let result = sqlx::query(
            r#"
            INSERT INTO ingredient_dictionary (id, name_en, name_pl, name_ru, name_uk)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (LOWER(TRIM(name_en))) DO NOTHING
            "#
        )
        .bind(id)
        .bind(name_en_trimmed)
        .bind(name_pl.trim())
        .bind(name_ru.trim())
        .bind(name_uk.trim())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Dictionary insert failed: {}", e);
            AppError::internal(&format!("Failed to insert into dictionary: {}", e))
        })?;

        // 📝 Всегда возвращаем ТЕКУЩУЮ запись из БД (наша или из race condition)
        // Гарантирует консистентность даже при race conditions
        let entry = self.find_by_en(name_en_trimmed)
            .await?
            .ok_or_else(|| {
                tracing::error!("Dictionary entry not found after insert: {}", name_en_trimmed);
                AppError::internal("Failed to retrieve inserted dictionary entry")
            })?;

        if result.rows_affected() > 0 {
            tracing::info!("✅ Dictionary entry created: {} (PL: {}, RU: {}, UK: {})", 
                entry.name_en, entry.name_pl, entry.name_ru, entry.name_uk);
        } else {
            tracing::info!("📦 Dictionary entry already exists (race condition): {} (PL: {}, RU: {}, UK: {})", 
                entry.name_en, entry.name_pl, entry.name_ru, entry.name_uk);
        }

        Ok(entry)
    }

    /// Получить статистику словаря (для отладки)
    pub async fn get_stats(&self) -> Result<DictionaryStats, AppError> {
        let stats = sqlx::query_as::<_, DictionaryStats>(
            "SELECT COUNT(*) as total_entries, MIN(created_at) as oldest_entry
             FROM ingredient_dictionary"
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::internal(&format!("Failed to get dictionary stats: {}", e)))?;

        Ok(stats)
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct DictionaryStats {
    pub total_entries: i64,
    pub oldest_entry: Option<time::PrimitiveDateTime>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dict_entry_serde() {
        let entry = DictionaryEntry {
            id: Uuid::new_v4(),
            name_en: "Apple".to_string(),
            name_pl: "Jabłko".to_string(),
            name_ru: "Яблоко".to_string(),
            name_uk: "Яблуко".to_string(),
        };

        let json = serde_json::to_string(&entry).unwrap();
        let decoded: DictionaryEntry = serde_json::from_str(&json).unwrap();
        
        assert_eq!(decoded.name_en, "Apple");
        assert_eq!(decoded.name_ru, "Яблоко");
    }
}
