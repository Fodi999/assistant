# 🐛 Inventory Enrichment Bug - "Unknown" Product Name

**Date**: 15 февраля 2026  
**Status**: КРИТИЧЕСКИЙ БАГ  
**Priority**: P0 - Блокирует запуск подписок

---

## 🔴 Проблема

При добавлении продукта в inventory, API возвращает:

```json
{
  "product": {
    "name": "Unknown",
    "category": "Молочные продукты и яйця"
  }
}
```

Но при поиске того же продукта в каталоге:

```json
{
  "name": "Яблоко"
}
```

**Категория показывается правильно, название - нет!**

---

## 🔍 Root Cause Analysis

### Текущий SQL в `inventory.rs`:

```sql
SELECT 
    ip.id,
    ip.catalog_ingredient_id,
    COALESCE(cit_user.name, cit_en.name, 'Unknown') as ingredient_name,
    COALESCE(cct_user.name, cct_en.name, 'Unknown') as category_name,
    ci.default_unit::TEXT as base_unit,
    ci.image_url,
    ...
FROM inventory_products ip
INNER JOIN catalog_ingredients ci 
    ON ip.catalog_ingredient_id = ci.id
LEFT JOIN catalog_ingredient_translations cit_user 
    ON cit_user.ingredient_id = ci.id AND cit_user.language = $2
LEFT JOIN catalog_ingredient_translations cit_en 
    ON cit_en.ingredient_id = ci.id AND cit_en.language = 'en'
LEFT JOIN catalog_categories cc 
    ON ci.category_id = cc.id
LEFT JOIN catalog_category_translations cct_user 
    ON cct_user.category_id = cc.id AND cct_user.language = $2
LEFT JOIN catalog_category_translations cct_en 
    ON cct_en.category_id = cc.id AND cct_en.language = 'en'
WHERE ip.tenant_id = $1
```

### Проблемы:

1. ✅ **Категории работают** - возвращается "Молочные продукты и яйця"
2. ❌ **Названия продуктов не работают** - возвращается "Unknown"

**Вывод**: 
- Либо в таблице `catalog_ingredient_translations` нет переводов
- Либо JOIN неправильный

---

## 🧪 Тестирование

### Шаг 1: Проверяем существование переводов

```bash
# Через psql (если есть доступ)
psql $DATABASE_URL -c "
SELECT 
    ci.id, 
    cit.language, 
    cit.name 
FROM catalog_ingredients ci
LEFT JOIN catalog_ingredient_translations cit ON cit.ingredient_id = ci.id
WHERE ci.id = '72acbc7d-dcef-488f-873f-75a6201f9411'
ORDER BY cit.language;
"
```

Ожидаемый результат:
```
id                                    | language | name
--------------------------------------+----------+---------
72acbc7d-dcef-488f-873f-75a6201f9411 | en       | Apple
72acbc7d-dcef-488f-873f-75a6201f9411 | ru       | Яблоко
72acbc7d-dcef-488f-873f-75a6201f9411 | pl       | Jabłko
72acbc7d-dcef-488f-873f-75a6201f9411 | uk       | Яблуко
```

### Шаг 2: Проверяем через API

```bash
# Поиск (работает)
curl -G "https://ministerial-yetta-fodi999-c58d8823.koyeb.app/api/catalog/ingredients" \
  --data-urlencode "q=яблоко" \
  -H "Authorization: Bearer $TOKEN"

# Inventory (не работает)
curl -G "https://ministerial-yetta-fodi999-c58d8823.koyeb.app/api/inventory/products" \
  -H "Authorization: Bearer $TOKEN"
```

---

## 🔧 Возможные Причины

### 1️⃣ Гипотеза: Нет переводов в `catalog_ingredient_translations`

Если таблица `catalog_ingredient_translations` пустая или не содержит нужных переводов:

```sql
-- Проверка
SELECT COUNT(*) FROM catalog_ingredient_translations;
```

**Решение**: Заполнить таблицу переводами.

---

### 2️⃣ Гипотеза: Неправильный JOIN

Возможно проблема в том, что:
- `catalog_categories` имеет таблицу переводов `catalog_category_translations`
- `catalog_ingredients` тоже имеет `catalog_ingredient_translations`

Но JOIN работает по-разному.

**Проверка в коде catalog search**:

```bash
grep -A 20 "search_ingredients" src/application/catalog.rs
```

---

### 3️⃣ Гипотеза: Hybrid Translation Cache

Возможно, поиск использует гибридный кеш переводов, а inventory - прямой SQL.

**Проверка**:
```bash
grep -r "hybrid_translation" src/
grep -r "translation_cache" src/
```

---

## ✅ Решение

### Вариант 1: Добавить fallback на все языки

```sql
COALESCE(
    cit_user.name,     -- Язык пользователя (ru)
    cit_en.name,       -- Английский
    cit_ru.name,       -- Русский (если user не ru)
    cit_pl.name,       -- Польский
    cit_uk.name,       -- Украинский
    'Unknown'
) as ingredient_name
```

### Вариант 2: Использовать тот же подход, что в catalog search

Скопировать логику из `CatalogService::search_ingredients()`.

### Вариант 3: Использовать Hybrid Translation Cache

Если catalog search использует кеш переводов, то inventory должен использовать тот же.

---

## 📝 Action Plan

### Задача 1: Диагностика
- [ ] Подключиться к production БД
- [ ] Проверить наличие переводов в `catalog_ingredient_translations`
- [ ] Сравнить SQL запросы в `catalog.rs` и `inventory.rs`

### Задача 2: Исправление
- [ ] Выбрать правильное решение (1, 2 или 3)
- [ ] Обновить SQL запрос в `inventory.rs`
- [ ] Добавить unit test
- [ ] Проверить на локальной БД

### Задача 3: Тестирование
- [ ] Деплой на продакшен
- [ ] Проверить через API
- [ ] Убедиться что `name != "Unknown"`

### Задача 4: Мониторинг
- [ ] Добавить логирование в enrichment слой
- [ ] Добавить метрику "Unknown products count"
- [ ] Алерт если > 0

---

## 🚨 Критичность

**Почему это P0**:

1. UI будет выглядеть сломанным
2. Пользователи не смогут понять что у них в inventory
3. Блокирует запуск подписок
4. Влияет на UX всего приложения

**Временное решение (workaround)**:

Если нет времени на полное исправление, можно:
```sql
COALESCE(cit_user.name, cit_en.name, ci.id::TEXT) as ingredient_name
```

Чтобы хотя бы показать ID вместо "Unknown".

---

*Next Step*: Подключиться к production БД и проверить `catalog_ingredient_translations`
