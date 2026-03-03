# 📡 Frontend API Reference — Полная документация

**Base URL (Production):** `https://ministerial-yetta-fodi999-c58d8823.koyeb.app`  
**Base URL (Local):** `http://localhost:8000`  
**API Prefix:** `/api`

---

## ⚡ Критические исправления (прямо сейчас)

### 1. `dishes.map is not a function` и `dishes.filter is not a function`

**Причина:** Бэкенд возвращает объект с пагинацией, а не голый массив.

**Реальный ответ `GET /api/dishes`:**
```json
{
  "items": [...],
  "total": 5,
  "page": 1,
  "per_page": 50
}
```

**Исправление в вашем коде:**
```typescript
// ❌ БЫЛО (неверно):
const dishes = await apiFetch('/api/dishes').then(r => r.json());
dishes.map(...) // TypeError: dishes.map is not a function

// ✅ СТАЛО (верно):
const response = await apiFetch('/api/dishes').then(r => r.json());
const dishes: Dish[] = response.items ?? response.dishes ?? [];
dishes.map(...) // Работает!
```

**Для `ai-alerts.tsx` (строка 46):**
```typescript
// Всегда защищайте массивы:
const dishes = useDishesStore(s => s.dishes) ?? [];
const lossDishes = dishes.filter(d => (d.profit_margin_percent || 0) < 15);
```

### 2. `MISSING_MESSAGE: dishes.actions.recalculate`

Добавьте в ваш файл локализации `messages/ru.json`:
```json
{
  "dishes": {
    "actions": {
      "recalculate": "🔄 Пересчитать себестоимость",
      "add": "Добавить блюдо",
      "edit": "Редактировать",
      "delete": "Удалить"
    },
    "title": "Меню",
    "empty": "Блюда не найдены",
    "food_cost": "Food Cost",
    "margin": "Маржа",
    "selling_price": "Цена продажи",
    "cost_price": "Себестоимость"
  }
}
```

---

## 🔐 Auth API

### `POST /api/auth/register`
Регистрация нового пользователя.
```typescript
// Request:
{
  email: string,
  password: string,       // min 8 символов
  restaurant_name: string, // ОБЯЗАТЕЛЬНО
  owner_name?: string,
  language?: "ru" | "en" | "pl" | "uk"
}

// Response 200:
{
  access_token: string,   // JWT, истекает через 15 мин
  refresh_token: string,  // Refresh, истекает через 30 дней
  token_type: "Bearer",
  user_id: string,        // UUID
  tenant_id: string       // UUID ресторана
}
```

### `POST /api/auth/login`
```typescript
// Request:
{ email: string, password: string }

// Response 200: (то же что register)
{ access_token, refresh_token, token_type, user_id, tenant_id }
```

### `POST /api/auth/refresh`
Обновление токена без повторного логина.
```typescript
// Request:
{ refresh_token: string }

// Response 200:
{ access_token, refresh_token, token_type, user_id, tenant_id }
```

---

## 👤 User API (требует Authorization: Bearer)

### `GET /api/me`
Получить профиль текущего пользователя.
```typescript
// Response 200:
{
  id: string,
  email: string,
  restaurant_name: string,
  tenant_id: string,
  language: "ru" | "en" | "pl" | "uk",
  avatar_url?: string
}
```

### `POST /api/profile/avatar/upload-url`
Получить presigned URL для загрузки аватара напрямую в R2/S3.
```typescript
// Response 200:
{ upload_url: string, public_url: string }
// 1. PUT upload_url с файлом (Content-Type: image/*)
// 2. PUT /api/profile/avatar с { avatar_url: public_url }
```

### `PUT /api/profile/avatar`
```typescript
// Request:
{ avatar_url: string }
```

---

## 🤖 Assistant Bot (State Machine)

### `GET /api/assistant/state`
Получить текущее состояние бота и доступные действия.
```typescript
// Response 200:
{
  step: "Start" | "InventorySetup" | "RecipeSetup" | "DishSetup" | "Report",
  message: string,       // Подсказка на языке пользователя
  progress: number,      // 0-100
  actions: [
    { id: string, label: string }  // Доступные кнопки
  ],
  warnings: [
    { level: "warning" | "error", message: string }
  ]
}
```

### `POST /api/assistant/command`
Отправить команду боту.
```typescript
// ⚠️ ВАЖНО: формат команды — объект с полем "type"
// Request - начать инвентаризацию:
{ "command": { "type": "start_inventory" } }

// Request - добавить продукт:
{
  "command": {
    "type": "add_product",
    "payload": {
      "catalog_ingredient_id": "uuid",
      "price_per_unit_cents": 500,
      "quantity": 10.0,
      "received_at": "2026-03-01T10:00:00Z",  // ISO 8601
      "expires_at": "2026-03-08T10:00:00Z"
    }
  }
}

// Request - завершить инвентаризацию:
{ "command": { "type": "finish_inventory" } }

// Все доступные команды:
// start_inventory | add_product | finish_inventory
// create_recipe   | finish_recipes
// create_dish     | finish_dishes
// view_report

// Response 200: (то же что GET /api/assistant/state)
```

---

## 📦 Catalog API (справочник ингредиентов)

### `GET /api/catalog/categories`
Все категории ингредиентов (молочные, мясо, овощи...).
```typescript
// Response 200:
[
  { id: string, name: string, slug: string }
]
```

### `GET /api/catalog/ingredients?query=milk&category_id=uuid`
Поиск ингредиентов в общем каталоге.
```typescript
// Query params:
// query?     - строка поиска
// category_id? - фильтр по категории

// Response 200:
{
  "ingredients": [
    {
      id: string,
      name: string,            // Локализованное название
      category_id: string,
      default_unit: string,    // "kilogram" | "piece" | "liter" ...
      default_shelf_life_days?: number,
      allergens: string[],
      calories_per_100g?: number,
      image_url?: string
    }
  ]
}
// ⚠️ Массив лежит в поле "ingredients", не в корне!
```

---

## 🏪 Inventory API (склад ресторана)

### `GET /api/inventory/products?page=1&per_page=50`
Список продуктов на складе с локализованными названиями.
```typescript
// Response 200:
{
  items: [
    {
      id: string,
      catalog_ingredient_id: string,
      name: string,               // Локализованное название
      category_name: string,
      price_per_unit_cents: number,
      quantity: number,
      unit: string,
      received_at: string,        // ISO 8601
      expires_at: string,         // ISO 8601
      days_until_expiry: number,  // Дней до истечения срока
      is_expired: boolean
    }
  ],
  total: number,
  page: number,
  per_page: number
}
```

### `POST /api/inventory/products`
Добавить продукт на склад.
```typescript
// Request:
{
  catalog_ingredient_id: string,  // UUID из /api/catalog/ingredients
  price_per_unit_cents: number,   // Цена в копейках (100 = 1 руб.)
  quantity: number,               // float (10.5 кг)
  received_at: string,            // ISO 8601 "2026-03-01T10:00:00Z"
  expires_at: string              // ISO 8601
}

// Response 201:
{ id, catalog_ingredient_id, price_per_unit_cents, quantity, received_at, expires_at }
```

### `PUT /api/inventory/products/:id`
```typescript
// Request:
{ price_per_unit_cents?: number, quantity?: number }
```

### `DELETE /api/inventory/products/:id`
Удалить продукт со склада. `Response 204`

### `GET /api/inventory/dashboard`
Сводка по складу (количество продуктов, истекающие, стоимость).
```typescript
// Response 200:
{
  total_items: number,
  expiring_soon_count: number,  // Истекает в ближайшие 3 дня
  expired_count: number,
  total_value_cents: number
}
```

### `GET /api/inventory/alerts`
Предупреждения по складу (истекающие продукты).
```typescript
// Response 200:
[
  { id, product_name, expires_at, days_until_expiry, alert_type: "expiring_soon" | "expired" }
]
```

### `GET /api/inventory/health`
Общее "здоровье" склада (процент свежих продуктов).

### `POST /api/inventory/process-expirations`
Пометить просроченные продукты как списанные. Вызывать по расписанию (cron).

### `GET /api/inventory/reports/loss`
Отчёт по списаниям (убытки от просрочки).

---

## 📖 Recipes V2 API (рецепты с переводами)

### `GET /api/recipes/v2`
Список рецептов текущего пользователя (язык определяется автоматически из профиля).
```typescript
// Query params: limit?, offset?, search?
// Response 200:
[
  {
    id: string,
    name: string,           // Локализованное
    instructions: string,   // Локализованное
    language: string,
    servings: number,
    image_url?: string,
    total_cost_cents?: number,
    cost_per_serving_cents?: number,
    status: "draft" | "published",
    is_public: boolean,
    created_at: string,
    updated_at: string,
    ingredients: [
      {
        id: string,
        catalog_ingredient_id: string,
        catalog_ingredient_name?: string,
        quantity: number,
        unit: string,
        cost_at_use_cents?: number
      }
    ]
  }
]
// ⚠️ Возвращает МАССИВ, не объект с пагинацией!
```

### `POST /api/recipes/v2`
Создать рецепт (body limit: 10 MB).
```typescript
// Request:
{
  name: string,
  instructions: string,
  language: "ru" | "en" | "pl" | "uk",
  servings: number,
  image_url?: string,
  ingredients: [
    {
      catalog_ingredient_id: string,  // UUID из каталога
      quantity: number,               // Decimal (0.5, 200.0)
      unit: string                    // "kg", "g", "piece", "ml", "l"
    }
  ]
}
// Response 201: RecipeResponseDto
```

### `GET /api/recipes/v2/:id`
Получить рецепт по ID (локализованный).

### `PUT /api/recipes/v2/:id` / `PATCH /api/recipes/v2/:id`
Обновить рецепт. Body: UpdateRecipeDto (то же что Create).

### `DELETE /api/recipes/v2/:id` → `204`

### `POST /api/recipes/v2/:id/publish`
Опубликовать рецепт (сделать публичным).

### `POST /api/recipes/v2/:id/image`
Загрузить изображение для рецепта (multipart/form-data).

### `GET /api/recipes/v2/:id/image-url`
Получить presigned URL для загрузки изображения напрямую в R2.

---

## 🧠 AI Insights API (нейронный анализ рецептов)

### `GET /api/recipes/v2/:id/insights/:language`
**Главный эндпоинт нейронного анализа.** Возвращает или генерирует AI-инсайты рецепта.

Если инсайты уже есть в кэше → возвращает мгновенно (1-2 мс).  
Если нет → генерирует через Groq AI (5-10 сек) и кэширует.

```typescript
// language: "ru" | "en" | "pl" | "uk"
// Response 200:
{
  insights: {
    id: string,
    recipe_id: string,
    language: string,
    feasibility_score: number,   // 0-100, оценка реализуемости рецепта
    
    // Пошаговая технология приготовления
    steps: [
      {
        step_number: number,
        action: string,              // "Нарезать", "Варить"
        description: string,
        duration_minutes?: number,
        temperature?: string,        // "180°C"
        technique?: string,          // "julienne", "dice"
        ingredients_used: string[]   // UUID ингредиентов
      }
    ],
    
    // Проверка логики рецепта
    validation: {
      is_valid: boolean,
      warnings: [{ severity, code, message, field? }],
      errors: [{ severity, code, message, field? }],
      missing_ingredients: string[],
      safety_checks: string[]
    },
    
    // Советы по улучшению
    suggestions: [
      {
        suggestion_type: "improvement" | "substitution" | "technique",
        title: string,
        description: string,
        impact: "taste" | "texture" | "nutrition" | "cost",
        confidence: number   // 0.0 - 1.0
      }
    ],
    
    model: string,
    created_at: string
  },
  generation_time_ms: number   // ~2 если из кэша, ~8000 если свежий
}
```

### `POST /api/recipes/v2/:id/insights/:language/refresh`
Принудительно перегенерировать AI-анализ (если рецепт изменился). `Response 201`

### `GET /api/recipes/v2/:id/insights`
Получить AI-анализ на всех доступных языках.
```typescript
// Response 200: Array<InsightsResponse>
```

---

## 🍽️ Dishes API (меню ресторана)

### `GET /api/dishes?page=1&per_page=50&active_only=false`
```typescript
// ⚠️ ВАЖНО: ответ — ОБЪЕКТ с пагинацией, не массив!
// Response 200:
{
  items: [   // ← используйте response.items, не response напрямую!
    {
      id: string,
      name: string,
      recipe_id: string,
      description?: string,
      selling_price_cents: number,
      recipe_cost_cents?: number,      // null если не пересчитано
      food_cost_percent?: number,      // null если нет себестоимости
      profit_margin_percent?: number,  // null если нет себестоимости
      active: boolean,
      image_url?: string
    }
  ],
  total: number,
  page: number,
  per_page: number
}

// Правильный код:
const data = await getDishes();
const dishes = data.items ?? [];  // ← всегда fallback на []
```

### `POST /api/dishes`
Создать блюдо из рецепта. Себестоимость рассчитывается автоматически.
```typescript
// Request:
{
  recipe_id: string,              // UUID рецепта из /api/recipes/v2
  name: string,
  description?: string,
  selling_price_cents: number,    // 150000 = 1500.00 руб.
  image_url?: string
}
// Response 201: Dish (с рассчитанными food_cost_percent и profit_margin_percent)
```

### `POST /api/dishes/recalculate-all`
Пересчитать себестоимость ВСЕХ блюд по актуальным ценам склада.  
Вызывать после изменения цен на продукты в инвентаре. `Response 200`

---

## 📊 Menu Engineering API (экономика меню)

### `GET /api/menu-engineering/analysis?period_days=30&language=ru`
BCG-матрица блюд: Stars, Plowhorses, Puzzles, Dogs.
```typescript
// Response 200:
{
  tenant_id: string,
  period_days: number,
  total_revenue_cents: number,
  average_margin_cents: number,
  categories: {
    stars: DishPerformance[],      // Высокая маржа + высокие продажи
    plowhorses: DishPerformance[], // Низкая маржа + высокие продажи
    puzzles: DishPerformance[],    // Высокая маржа + низкие продажи
    dogs: DishPerformance[]        // Низкая маржа + низкие продажи
  }
}

// DishPerformance:
{
  dish_id: string,
  name: string,
  food_cost_cents: number,
  selling_price_cents: number,
  margin_cents: number,
  sales_count: number,
  ai_recommendation?: string
}
```

### `POST /api/menu-engineering/sales`
Записать продажу блюда (обычно вызывается из POS-системы).
```typescript
// Request:
{
  dish_id: string,
  quantity: number,
  sold_at?: string  // ISO 8601, default: now
}
```

---

## 📋 Reports API

### `GET /api/reports/summary`
Сводный финансовый отчёт ресторана.
```typescript
// Response 200:
{
  revenue_cents: number,
  cost_cents: number,
  profit_cents: number,
  food_cost_percent: number,
  top_dishes: [...],
  low_margin_dishes: [...]
}
```

---

## 🧩 Tenant Ingredients (кастомные ингредиенты ресторана)

Если в общем каталоге нет нужного ингредиента, ресторан может добавить свой.

### `GET /api/tenant/ingredients` — Список кастомных ингредиентов
### `POST /api/tenant/ingredients` — Добавить кастомный ингредиент
### `GET /api/tenant/ingredients/search?q=...` — Поиск среди кастомных
### `GET /api/tenant/ingredients/:id` — Один ингредиент
### `PUT /api/tenant/ingredients/:id` — Обновить
### `DELETE /api/tenant/ingredients/:id` — Удалить

---

## 🛡️ Общие правила

### Authorization Header (все `/api` эндпоинты кроме auth):
```
Authorization: Bearer <access_token>
```

### Обработка ошибок:
```typescript
// Все ошибки возвращаются в формате:
// Статус: 400 | 401 | 403 | 404 | 422 | 500

// 422 Unprocessable Entity (неверные поля):
"Failed to deserialize the JSON body into the target type: missing field `restaurant_name`"

// 401 Unauthorized:
{ "error": "Unauthorized" }

// 404 Not Found:
{ "error": "Not found" }

// Правильная обработка в TypeScript:
async function apiFetch(url: string, options?: RequestInit) {
  const res = await fetch(BASE_URL + url, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${getToken()}`,
      ...options?.headers
    }
  });
  
  if (res.status === 401) {
    // Токен истёк — используем refresh token
    await refreshToken();
    return apiFetch(url, options); // Повторяем запрос
  }
  
  if (!res.ok) {
    const error = await res.text();
    throw new Error(error);
  }
  
  return res;
}
```

### Деньги (Money):
- **Всегда в копейках/центах** (integer): `150000` = `1500.00 ₽`
- При вводе: `parseFloat(input) * 100 |> Math.round`
- При выводе: `cents / 100`
- Форматирование: `(cents / 100).toLocaleString('ru-RU', { style: 'currency', currency: 'RUB' })`

### Даты:
- Формат: **ISO 8601 / RFC 3339**: `"2026-03-01T10:00:00Z"`
- В JavaScript: `new Date().toISOString()` → `"2026-03-03T12:00:00.000Z"` ✅

### Язык:
- Язык пользователя **хранится в базе данных** (в профиле), не в заголовке запроса
- Бэкенд автоматически возвращает данные на языке пользователя
- Поддерживаемые языки: `ru`, `en`, `pl`, `uk`

---

## 🔄 Типичный флоу фронтенда

```
1. Регистрация/Логин → получить access_token + refresh_token
2. GET /api/assistant/state → показать онбординг-шаги бота
3. GET /api/catalog/ingredients → показать каталог для инвентаризации
4. POST /api/inventory/products → добавить продукты на склад
5. POST /api/recipes/v2 → создать рецепт с ингредиентами
6. GET /api/recipes/v2/:id/insights/ru → получить AI-анализ рецепта
7. POST /api/dishes → создать блюдо с ценой продажи
8. GET /api/menu-engineering/analysis → посмотреть экономику меню
9. GET /api/reports/summary → финансовый отчёт
```
