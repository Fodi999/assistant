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

/// Extended dictionary entry with status/source (for admin review)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DictionaryEntryFull {
    pub id: Uuid,
    pub name_en: String,
    pub name_pl: String,
    pub name_ru: String,
    pub name_uk: String,
    pub status: String,
    pub source: String,
    pub confidence: Option<f32>,
    pub created_at: time::OffsetDateTime,
    pub reviewed_at: Option<time::OffsetDateTime>,
}

/// Сервис для управления словарём переводов
#[derive(Clone)]
pub struct DictionaryService {
    pool: PgPool,
}

impl DictionaryService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Поиск АКТИВНОГО перевода по английскому названию.
    /// Pending/rejected записи НЕ возвращаются — только подтверждённые админом.
    pub async fn find_by_en(&self, name_en: &str) -> Result<Option<DictionaryEntry>, AppError> {
        let result = sqlx::query_as::<_, DictionaryEntry>(
            "SELECT id, name_en, name_pl, name_ru, name_uk 
             FROM ingredient_dictionary 
             WHERE LOWER(TRIM(name_en)) = LOWER(TRIM($1))
               AND status = 'active'",
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

    /// Поиск записи в ЛЮБОМ статусе (для проверки дубликатов перед insert)
    pub async fn exists_by_en(&self, name_en: &str) -> Result<bool, AppError> {
        let result = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM ingredient_dictionary 
             WHERE LOWER(TRIM(name_en)) = LOWER(TRIM($1))",
        )
        .bind(name_en)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::internal(&format!("Dictionary exists check failed: {}", e)))?;

        Ok(result > 0)
    }

    /// Сохранить подтверждённый перевод (source=manual, status=active).
    /// Используется при ручном добавлении админом.
    pub async fn insert(
        &self,
        name_en: &str,
        name_pl: &str,
        name_ru: &str,
        name_uk: &str,
    ) -> Result<DictionaryEntry, AppError> {
        let id = Uuid::new_v4();
        let name_en_trimmed = name_en.trim();

        let result = sqlx::query(
            r#"
            INSERT INTO ingredient_dictionary (id, name_en, name_pl, name_ru, name_uk, status, source)
            VALUES ($1, $2, $3, $4, $5, 'active', 'manual')
            ON CONFLICT (LOWER(TRIM(name_en))) DO NOTHING
            "#,
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

        let entry = self.find_by_en(name_en_trimmed).await?.ok_or_else(|| {
            AppError::internal("Failed to retrieve inserted dictionary entry")
        })?;

        if result.rows_affected() > 0 {
            tracing::info!("✅ Dictionary entry created: {} (manual, active)", entry.name_en);
        }

        Ok(entry)
    }

    /// Сохранить AI-перевод как PENDING (ожидает подтверждения админа).
    /// AI → pending → admin review → active.
    /// НЕ является source of truth, пока админ не подтвердит!
    pub async fn insert_pending(
        &self,
        name_en: &str,
        name_pl: &str,
        name_ru: &str,
        name_uk: &str,
        confidence: f32,
    ) -> Result<DictionaryEntryFull, AppError> {
        let id = Uuid::new_v4();
        let name_en_trimmed = name_en.trim();

        // Deduplication: check if ANY entry (active/pending/rejected) exists
        if self.exists_by_en(name_en_trimmed).await? {
            tracing::info!("📦 Dictionary entry already exists for '{}' — skipping pending insert", name_en_trimmed);
            // Return existing entry
            let existing = sqlx::query_as::<_, DictionaryEntryFull>(
                "SELECT id, name_en, name_pl, name_ru, name_uk, status, source, confidence, created_at, reviewed_at
                 FROM ingredient_dictionary 
                 WHERE LOWER(TRIM(name_en)) = LOWER(TRIM($1))",
            )
            .bind(name_en_trimmed)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::internal(&format!("Dictionary lookup failed: {}", e)))?;
            return Ok(existing);
        }

        sqlx::query(
            r#"
            INSERT INTO ingredient_dictionary (id, name_en, name_pl, name_ru, name_uk, status, source, confidence)
            VALUES ($1, $2, $3, $4, $5, 'pending', 'ai', $6)
            ON CONFLICT (LOWER(TRIM(name_en))) DO NOTHING
            "#,
        )
        .bind(id)
        .bind(name_en_trimmed)
        .bind(name_pl.trim())
        .bind(name_ru.trim())
        .bind(name_uk.trim())
        .bind(confidence)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Dictionary insert_pending failed: {}", e);
            AppError::internal(&format!("Failed to insert pending dictionary entry: {}", e))
        })?;

        let entry = sqlx::query_as::<_, DictionaryEntryFull>(
            "SELECT id, name_en, name_pl, name_ru, name_uk, status, source, confidence, created_at, reviewed_at
             FROM ingredient_dictionary 
             WHERE LOWER(TRIM(name_en)) = LOWER(TRIM($1))",
        )
        .bind(name_en_trimmed)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::internal(&format!("Dictionary fetch after insert failed: {}", e)))?;

        tracing::info!(
            "🟡 Dictionary PENDING: {} → RU:{}, PL:{}, UK:{} (confidence: {:.2})",
            entry.name_en, entry.name_ru, entry.name_pl, entry.name_uk, confidence
        );

        Ok(entry)
    }

    /// Список всех pending-записей для admin review
    pub async fn list_pending(&self) -> Result<Vec<DictionaryEntryFull>, AppError> {
        let entries = sqlx::query_as::<_, DictionaryEntryFull>(
            "SELECT id, name_en, name_pl, name_ru, name_uk, status, source, confidence, created_at, reviewed_at
             FROM ingredient_dictionary 
             WHERE status = 'pending'
             ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::internal(&format!("Failed to list pending: {}", e)))?;

        Ok(entries)
    }

    /// Список всех записей (для admin dictionary page)
    pub async fn list_all(&self) -> Result<Vec<DictionaryEntryFull>, AppError> {
        let entries = sqlx::query_as::<_, DictionaryEntryFull>(
            "SELECT id, name_en, name_pl, name_ru, name_uk, status, source, confidence, created_at, reviewed_at
             FROM ingredient_dictionary 
             ORDER BY status ASC, created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::internal(&format!("Failed to list dictionary: {}", e)))?;

        Ok(entries)
    }

    /// Admin подтверждает AI-перевод → pending → active
    pub async fn approve(&self, id: Uuid) -> Result<DictionaryEntryFull, AppError> {
        sqlx::query(
            "UPDATE ingredient_dictionary 
             SET status = 'active', reviewed_at = now() 
             WHERE id = $1 AND status = 'pending'",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(&format!("Failed to approve: {}", e)))?;

        let entry = sqlx::query_as::<_, DictionaryEntryFull>(
            "SELECT id, name_en, name_pl, name_ru, name_uk, status, source, confidence, created_at, reviewed_at
             FROM ingredient_dictionary WHERE id = $1",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::internal(&format!("Entry not found: {}", e)))?;

        tracing::info!("✅ Dictionary APPROVED: {} ({}→active)", entry.name_en, entry.source);
        Ok(entry)
    }

    /// Admin подтверждает с исправлениями → pending → active (обновляет переводы)
    pub async fn approve_with_edits(
        &self,
        id: Uuid,
        name_ru: &str,
        name_pl: &str,
        name_uk: &str,
    ) -> Result<DictionaryEntryFull, AppError> {
        sqlx::query(
            "UPDATE ingredient_dictionary 
             SET name_ru = $2, name_pl = $3, name_uk = $4, 
                 status = 'active', reviewed_at = now()
             WHERE id = $1",
        )
        .bind(id)
        .bind(name_ru.trim())
        .bind(name_pl.trim())
        .bind(name_uk.trim())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(&format!("Failed to approve with edits: {}", e)))?;

        let entry = sqlx::query_as::<_, DictionaryEntryFull>(
            "SELECT id, name_en, name_pl, name_ru, name_uk, status, source, confidence, created_at, reviewed_at
             FROM ingredient_dictionary WHERE id = $1",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::internal(&format!("Entry not found: {}", e)))?;

        tracing::info!("✅ Dictionary APPROVED (edited): {} → RU:{}, PL:{}, UK:{}", entry.name_en, entry.name_ru, entry.name_pl, entry.name_uk);
        Ok(entry)
    }

    /// Admin отклоняет AI-перевод → pending → rejected
    pub async fn reject(&self, id: Uuid) -> Result<(), AppError> {
        let result = sqlx::query(
            "UPDATE ingredient_dictionary 
             SET status = 'rejected', reviewed_at = now() 
             WHERE id = $1 AND status = 'pending'",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::internal(&format!("Failed to reject: {}", e)))?;

        if result.rows_affected() > 0 {
            tracing::info!("❌ Dictionary REJECTED: {}", id);
        }
        Ok(())
    }

    /// Получить статистику словаря (для отладки)
    pub async fn get_stats(&self) -> Result<DictionaryStats, AppError> {
        let stats = sqlx::query_as::<_, DictionaryStats>(
            "SELECT COUNT(*) as total_entries, MIN(created_at) as oldest_entry
             FROM ingredient_dictionary",
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
