# 🔴 СРОЧНОЕ ИСПРАВЛЕНИЕ: User Catalog Search

**Дата**: 15 февраля 2026  
**Статус**: КРИТИЧЕСКИЙ БАГ  
**Проблема**: `/api/catalog/ingredients` возвращает пустой массив

---

## 🐛 Симптомы

```bash
# User endpoint (НЕ РАБОТАЕТ)
curl "https://api.fodi.app/api/catalog/ingredients?q=milk" \
  -H "Authorization: Bearer $TOKEN"
# Результат: {"ingredients":[]}

# Admin endpoint (РАБОТАЕТ)
curl "https://api.fodi.app/api/admin/products?limit=5" \
  -H "Authorization: Bearer $ADMIN_TOKEN"
# Результат: {products: [{...}]}
```

---

## 🔍 Диагностика

### ✅ Backend работает
- Сервер здоров (logs показывают)
- База данных подключена
- Продукты созданы через admin panel

### ❌ User search не работает
- Возвращает пустой массив
- Даже без query параметра
- Даже на английском "milk"

---

## 🎯 КОРНЕВАЯ ПРИЧИНА

User endpoint `/api/catalog/ingredients` использует **НЕПРАВИЛЬНЫЙ SQL**:

### ❌ ТЕКУЩИЙ КОД (НЕПРАВИЛЬНО):

```rust
// src/infrastructure/persistence/catalog_ingredient_repository.rs
async fn search(&self, query: &str, language: Language, limit: i64) -> AppResult<Vec<CatalogIngredient>> {
    let lang_code = language.code();
    
    let sql = r#"
        SELECT 
            ci.id, ci.category_id, ci.name_pl, ci.name_en, ci.name_uk, ci.name_ru,
            ci.default_unit::text as default_unit, 
            ci.default_shelf_life_days,
            ARRAY(SELECT unnest(ci.allergens)::text) as allergens, 
            ci.calories_per_100g, 
            ARRAY(SELECT unnest(ci.seasons)::text) as seasons, 
            ci.image_url,
            ci.is_active,
            COALESCE(cit_user.name, cit_en.name) as search_name
        FROM catalog_ingredients ci
        LEFT JOIN catalog_ingredient_translations cit_user 
            ON cit_user.ingredient_id = ci.id AND cit_user.language = $2
        LEFT JOIN catalog_ingredient_translations cit_en 
            ON cit_en.ingredient_id = ci.id AND cit_en.language = 'en'
        WHERE COALESCE(ci.is_active, true) = true 
          AND COALESCE(cit_user.name, cit_en.name) ILIKE '%' || $1 || '%'
        ORDER BY COALESCE(cit_user.name, cit_en.name) ASC
        LIMIT $3
    "#;
```

### 🔴 ПРОБЛЕМА:

SQL джойнит `catalog_ingredient_translations`, но **эта таблица ПУСТАЯ**!

Продукты создаются с полями:
- `name_en`
- `name_ru`
- `name_pl`
- `name_uk`

Прямо в таблице `catalog_ingredients`!

Таблица `catalog_ingredient_translations` НЕ ИСПОЛЬЗУЕТСЯ!

---

## ✅ ПРАВИЛЬНОЕ РЕШЕНИЕ

### Вариант 1: Поиск по базовым полям (БЕЗ translations table)

```rust
async fn search(&self, query: &str, language: Language, limit: i64) -> AppResult<Vec<CatalogIngredient>> {
    let lang_code = language.code();
    
    // Выбрать колонку для поиска на основе языка
    let search_column = match lang_code {
        "ru" => "ci.name_ru",
        "pl" => "ci.name_pl",
        "uk" => "ci.name_uk",
        _ => "ci.name_en",
    };
    
    let sql = format!(r#"
        SELECT 
            ci.id, ci.category_id, 
            ci.name_pl, ci.name_en, ci.name_uk, ci.name_ru,
            ci.default_unit::text as default_unit, 
            ci.default_shelf_life_days,
            ARRAY(SELECT unnest(ci.allergens)::text) as allergens, 
            ci.calories_per_100g, 
            ARRAY(SELECT unnest(ci.seasons)::text) as seasons, 
            ci.image_url,
            ci.is_active
        FROM catalog_ingredients ci
        WHERE COALESCE(ci.is_active, true) = true 
          AND {} ILIKE '%' || $1 || '%'
        ORDER BY {} ASC
        LIMIT $2
    "#, search_column, search_column);

    let rows = sqlx::query(&sql)
        .bind(query)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

    rows.iter()
        .map(Self::row_to_ingredient)
        .collect()
}
```

### Вариант 2: Поиск по ВСЕМ языкам сразу

```rust
async fn search(&self, query: &str, language: Language, limit: i64) -> AppResult<Vec<CatalogIngredient>> {
    let sql = r#"
        SELECT 
            ci.id, ci.category_id, 
            ci.name_pl, ci.name_en, ci.name_uk, ci.name_ru,
            ci.default_unit::text as default_unit, 
            ci.default_shelf_life_days,
            ARRAY(SELECT unnest(ci.allergens)::text) as allergens, 
            ci.calories_per_100g, 
            ARRAY(SELECT unnest(ci.seasons)::text) as seasons, 
            ci.image_url,
            ci.is_active
        FROM catalog_ingredients ci
        WHERE COALESCE(ci.is_active, true) = true 
          AND (
              ci.name_en ILIKE '%' || $1 || '%' OR
              ci.name_ru ILIKE '%' || $1 || '%' OR
              ci.name_pl ILIKE '%' || $1 || '%' OR
              ci.name_uk ILIKE '%' || $1 || '%'
          )
        ORDER BY ci.name_en ASC
        LIMIT $2
    "#;

    let rows = sqlx::query(sql)
        .bind(query)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

    rows.iter()
        .map(Self::row_to_ingredient)
        .collect()
}
```

---

## 🚀 БЫСТРОЕ ИСПРАВЛЕНИЕ

### Шаг 1: Исправить `search` метод

Откройте:
```
src/infrastructure/persistence/catalog_ingredient_repository.rs
```

Найдите функцию `search` (строка ~85)

Замените SQL на **Вариант 2** (поиск по всем языкам)

### Шаг 2: Исправить `search_by_category`

Та же проблема в функции `search_by_category` (строка ~120)

### Шаг 3: Исправить `list`

Та же проблема в функции `list` (строка ~240)

### Шаг 4: Пересобрать и задеплоить

```bash
# Локально проверить
cargo build --release

# Закоммитить
git add src/infrastructure/persistence/catalog_ingredient_repository.rs
git commit -m "fix: User catalog search - use base columns instead of translations table"
git push

# Koyeb автоматически задеплоит
```

---

## 🧪 ПРОВЕРКА

После деплоя:

```bash
# Зарегистрировать юзера
TOKEN=$(curl -s POST "https://api.fodi.app/api/auth/register" \
  -H "Content-Type: application/json" \
  -d '{"email":"test@test.com","password":"Test123!","restaurant_name":"Test","owner_name":"Test"}' \
  | jq -r '.access_token')

# Поиск "cocoa" (английский)
curl "https://api.fodi.app/api/catalog/ingredients?q=cocoa" \
  -H "Authorization: Bearer $TOKEN" | jq .

# Должно вернуть: {"ingredients":[{"id":"...","name":"Cocoa",...}]}

# Поиск "какао" (русский)
curl "https://api.fodi.app/api/catalog/ingredients?q=какао" \
  -H "Authorization: Bearer $TOKEN" | jq .

# Должно вернуть: {"ingredients":[{"id":"...","name":"Какао",...}]}
```

---

## 📝 ИТОГО

### Архитектурная ошибка:

1. ❌ SQL джойнит пустую таблицу `catalog_ingredient_translations`
2. ✅ Данные лежат в базовых полях `name_en`, `name_ru`, etc.
3. ✅ Нужно искать напрямую по этим полям

### Время исправления:
- 5 минут на правку кода
- 2 минуты на деплой
- 1 минута на тест

**Total: 8 минут** ⏱️

---

*Status: WAITING FOR FIX*  
*Next: Deploy and verify*
