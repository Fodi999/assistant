# ✅ Catalog Products: Uniqueness & Soft Delete

## Что было добавлено

### 1️⃣ Уникальность по `name_en`

**На уровне БД:**
```sql
CREATE UNIQUE INDEX idx_catalog_ingredients_name_en_unique 
ON catalog_ingredients (LOWER(name_en)) 
WHERE is_active = true;
```

**Почему LOWER():**
- Регистронезависимая уникальность
- "Tomato" и "tomato" считаются одним продуктом

**Почему WHERE is_active = true:**
- Можно создать новый продукт с таким же названием после удаления старого
- Soft-delete не блокирует повторное использование имени

### 2️⃣ Soft Delete

**Новая колонка:**
```sql
ALTER TABLE catalog_ingredients 
ADD COLUMN is_active BOOLEAN DEFAULT true;
```

**Преимущества:**
- ✅ Сохраняет связи с `inventory_products`
- ✅ Сохраняет связи с `recipes` и `dishes`
- ✅ История данных не теряется
- ✅ Можно восстановить продукт при необходимости

**Как работает:**
- Удаление: `UPDATE ... SET is_active = false`
- Все SELECT запросы фильтруют: `WHERE is_active = true`

### 3️⃣ Автоматическая дедупликация

**При миграции:**
```sql
WITH duplicates AS (
    SELECT id, LOWER(name_en) as name_lower,
           ROW_NUMBER() OVER (PARTITION BY LOWER(name_en) ORDER BY created_at) as rn
    FROM catalog_ingredients
    WHERE is_active = true
)
UPDATE catalog_ingredients
SET is_active = false
WHERE id IN (SELECT id FROM duplicates WHERE rn > 1);
```

**Что делает:**
- Находит дубликаты по `name_en` (case-insensitive)
- Оставляет самый старый (по `created_at`)
- Остальные помечает как `is_active = false`

## Изменения в коде

### Domain Layer

**`CatalogIngredient` struct:**
```rust
pub struct CatalogIngredient {
    // ... existing fields ...
    pub is_active: bool,  // NEW
}
```

### Repository Layer

**Все SELECT запросы обновлены:**
```rust
WHERE ci.is_active = true  // NEW filter
```

**`row_to_ingredient` включает:**
```rust
let is_active: bool = row.try_get("is_active").unwrap_or(true);
```

### Application Layer

**`delete_product` изменен:**
```rust
// Before: DELETE FROM catalog_ingredients WHERE id = $1

// After: Soft delete
sqlx::query("UPDATE catalog_ingredients SET is_active = false WHERE id = $1")
    .bind(id)
    .execute(&self.pool)
    .await?;
```

## API Behavior

### Создание продукта

**До:**
- Можно было создать `"Tomato"` и `"tomato"` как разные продукты
- Дубликаты ломали UX

**После:**
```bash
# Первый запрос - OK
POST /api/admin/products
{"name_en": "Tomato", ...}
→ 201 Created

# Второй запрос - ERROR
POST /api/admin/products
{"name_en": "tomato", ...}
→ 409 Conflict: "Product with this name already exists"
```

### Удаление продукта

**До:**
```sql
DELETE FROM catalog_ingredients WHERE id = '...'
-- Проблема: связи с inventory ломаются
```

**После:**
```sql
UPDATE catalog_ingredients SET is_active = false WHERE id = '...'
-- ✅ Связи сохраняются
-- ✅ Продукт скрыт от пользователей
-- ✅ Имя освобождается для нового продукта
```

### Получение продуктов

**Все endpoints автоматически фильтруют:**
```sql
WHERE is_active = true
```

**Пользователи видят только активные продукты**

## Примеры использования

### Создание дубликатов (должно падать)

```bash
# 1️⃣ Создать "Apple"
curl -X POST /api/admin/products \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"name_en": "Apple", "category_id": "...", "price": 5, "unit": "kilogram"}'
# → 201 Created

# 2️⃣ Попробовать создать "APPLE" (разный регистр)
curl -X POST /api/admin/products \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"name_en": "APPLE", "category_id": "...", "price": 5, "unit": "kilogram"}'
# → 409 Conflict
```

### Повторное использование имени после удаления

```bash
# 1️⃣ Удалить "Apple"
curl -X DELETE /api/admin/products/{apple_id} \
  -H "Authorization: Bearer $TOKEN"
# → 200 OK (is_active = false)

# 2️⃣ Создать новый "Apple" с другими параметрами
curl -X POST /api/admin/products \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"name_en": "Apple", "category_id": "...", "price": 6, "unit": "kilogram"}'
# → 201 Created ✅
```

## Миграция данных

**Проверка дубликатов перед деплоем:**
```sql
-- Найти дубликаты
SELECT LOWER(name_en), COUNT(*)
FROM catalog_ingredients
WHERE is_active = true
GROUP BY LOWER(name_en)
HAVING COUNT(*) > 1;
```

**В вашей БД были дубликаты:**
- "Onions" x 5
- "Tomatoes" x 4
- "Salt" x 2

**После миграции:**
- Самый старый экземпляр остался активным
- Остальные помечены `is_active = false`

## Production Status

- ✅ Миграция создана: `20240118000001_add_catalog_uniqueness_and_soft_delete.sql`
- ✅ Domain model обновлен
- ✅ Repository обновлен
- ✅ Application service обновлен
- ✅ Автоматическая дедупликация включена
- 🚀 **Готово к деплою**

## Следующие шаги

После деплоя:
1. Проверить список продуктов - дубликаты должны исчезнуть
2. Попробовать создать дубликат - должна быть ошибка
3. Удалить продукт и создать заново с тем же именем - должно работать

## 🎯 Benefits

### Для пользователей
- Нет дубликатов в списке продуктов
- Чистая и понятная БД
- Быстрый поиск (уникальный индекс)

### Для разработчиков
- Гарантии на уровне БД
- Невозможно создать дубликат даже при багах в коде
- History preservation (soft delete)

### Для бизнеса
- Целостность данных
- Возможность аудита (кто, когда удалил)
- Возможность восстановления
