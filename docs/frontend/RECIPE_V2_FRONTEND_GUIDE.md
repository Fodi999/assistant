# 🍳 Recipe V2 Frontend Integration Guide

## 📋 Overview

Recipe V2 is a modern, type-safe API for managing complex recipes with full I18n support and SaaS isolation.

---

## 🔐 Authentication & Headers

All requests require an `Authorization` header. `tenant_id` and `user_id` are extracted automatically from the JWT.

```typescript
const headers = {
  'Authorization': `Bearer ${accessToken}`,
  'Content-Type': 'application/json'
};
```

---

## 📖 Reading Recipes

### 1. Unified Search & List

```
GET    /api/recipes/v2                    - Список рецептов (пагинация)
```

#### Query Parameters

- `search`: Строка для поиска по названию или описанию рецепта
- `category`: Фильтр по категории (ID)
- `status`: Фильтр по статусу (`draft` или `published`)
- `limit`: Количество рецептов на странице
- `offset`: Смещение для пагинации

#### Пример запроса

```bash
GET /api/recipes/v2?search=борщ&limit=10&offset=0
Authorization: Bearer <token>
```

### 2. Get Recipe Details (Localized)

```
GET    /api/recipes/v2/:id                - Получить рецепт с переводами
```

#### Пример запроса

```bash
GET /api/recipes/v2/12345
Authorization: Bearer <token>
```

---

## ✍️ Creating & Editing Recipes

### 1. The Recipe Payload
The new `RecipeRequestDto` is flattened for easier form binding.

```json
{
  "name_en": "Classic Borsch",
  "name_ru": "Классический Борщ",
  "description_en": "Traditional beet soup",
  "description_ru": "Традиционный свекольный суп",
  "instructions_en": "1. Boil beets...",
  "instructions_ru": "1. Сварите свеклу...",
  "category_id": "uuid",
  "servings": 4,
  "status": "Published",
  "ingredients": [
    {
      "ingredient_id": "uuid",
      "quantity": 500.0,
      "unit": "g"
    }
  ]
}
```

### 2. Save Recipe
Use `POST /api/recipes/v2` for new recipes and `PUT /api/recipes/v2/{id}` for updates.

---

## 🧪 Testing with cURL

```bash
curl -X GET "http://localhost:8080/api/recipes/v2?search=soup" \
     -H "Authorization: Bearer $TOKEN"
```
