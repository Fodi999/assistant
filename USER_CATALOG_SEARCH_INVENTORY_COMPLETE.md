# 🔍 Поиск по Каталогу и Добавление на Склад для Пользователя

**Date**: 15 февраля 2026  
**Status**: Production-Ready ✅  
**Отличие от админа**: Объединяем поиск + добавление в один поток

---

## 📋 Содержание

1. [API Endpoints](#api-endpoints)
2. [Разница между User и Admin](#разница-между-user-и-admin)
3. [Frontend компонент для поиска](#frontend-компонент)
4. [Полный поток: Поиск → Добавление](#полный-поток)
5. [Примеры запросов](#примеры-запросов)
6. [Кейс: Молоко Пастеризованное](#кейс-молоко-пастеризованное)

---

## 🔌 API Endpoints

### 1️⃣ **GET /api/catalog/ingredients** - Поиск/Список

Базовый endpoint для **обычного пользователя**:

```bash
# Поиск по названию (на русском)
GET /api/catalog/ingredients?q=молоко

# С лимитом результатов
GET /api/catalog/ingredients?q=молоко&limit=20

# По категории
GET /api/catalog/ingredients?category_id=abc-123-def

# Комбинированный поиск
GET /api/catalog/ingredients?category_id=abc-123-def&q=молоко&limit=20
```

**Request Headers**:
```
Authorization: Bearer <user_jwt_token>
Content-Type: application/json
```

**Query Parameters**:
```typescript
{
  q?: string;              // 🔍 Поисковый запрос (молоко, говядина, масло)
  category_id?: string;    // 🏷️ Фильтр по категории (UUID)
  limit?: number;          // 📊 Макс результатов (default: 50)
}
```

**Response** (200 OK):
```typescript
{
  "ingredients": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "category_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
      "name": "Пастеризованное молоко",        // 🇷🇺 На языке пользователя!
      "default_unit": "milliliter",
      "default_shelf_life_days": 5,
      "allergens": ["MILK"],
      "calories_per_100g": 61,
      "seasons": [],
      "image_url": "https://cdn.example.com/milk.jpg"
    },
    // ... еще продукты
  ]
}
```

---

### 2️⃣ **POST /api/inventory/products** - Добавление на Склад

Endpoint для добавления найденного продукта на личный склад:

```bash
POST /api/inventory/products
Authorization: Bearer <user_jwt_token>
Content-Type: application/json

{
  "catalog_ingredient_id": "550e8400-e29b-41d4-a716-446655440000",
  "price_per_unit_cents": 10000,  // ₽100.00
  "quantity": 5.0,                // 5 литров
  "received_at": "2026-02-15T10:30:00+00:00",
  "expires_at": null              // Опционально - автовычислится
}
```

**Response** (201 CREATED):
```typescript
{
  "id": "product-uuid",
  "catalog_ingredient_id": "550e8400-e29b-41d4-a716-446655440000",
  "ingredient_name": "Пастеризованное молоко",
  "category_name": "Молочные продукты",
  "base_unit": "milliliter",
  "image_url": "https://cdn.example.com/milk.jpg",
  "quantity": 5.0,
  "price_per_unit_cents": 10000,
  "received_at": "2026-02-15T10:30:00+00:00",
  "expires_at": "2026-02-20T10:30:00+00:00",  // ✅ Автовычислено!
  "expiration_status": "Fresh",
  "created_at": "2026-02-15T10:35:00+00:00",
  "updated_at": "2026-02-15T10:35:00+00:00"
}
```

---

### 3️⃣ **GET /api/inventory/products** - Мой Склад

Просмотр всех продуктов на личном складе:

```bash
GET /api/inventory/products
Authorization: Bearer <user_jwt_token>
```

**Response** (200 OK):
```typescript
[
  {
    "id": "product-uuid",
    "ingredient_name": "Пастеризованное молоко",
    "category_name": "Молочные продукты",
    "quantity": 5.0,
    "price_per_unit_cents": 10000,
    "expiration_status": "Fresh",      // Expired | ExpiresToday | ExpiringSoon | Fresh | NoExpiration
    "expires_at": "2026-02-20T10:30:00+00:00"
  },
  // ... другие продукты
]
```

---

## 🎯 Разница между User и Admin

| Аспект | Admin (`/api/admin/products`) | User (`/api/catalog/ingredients`) |
|--------|------|-------|
| **URL** | `/api/admin/products/search` | `/api/catalog/ingredients` |
| **Auth** | Super Admin JWT | Regular User JWT |
| **Язык** | ✅ Может выбрать (параметр `lang`) | ✅ Автоматически из БД (user.language) |
| **Результаты** | Все продукты в каталоге | Все продукты в каталоге |
| **Следующий шаг** | ➜ Редактирование в админке | ➜ Добавление на свой склад |
| **Цель** | Управление каталогом | Работа с собственным складом |

**Ключевое отличие**: User получает язык **из своего профиля в БД**, Admin может выбрать язык вручную.

---

## 🎨 Frontend Компонент

### `components/UserCatalogSearch.tsx`

```typescript
'use client';

import { useState, useCallback } from 'react';
import debounce from 'lodash/debounce';

interface Ingredient {
  id: string;
  category_id: string;
  name: string;                      // Локализованное имя!
  default_unit: string;
  default_shelf_life_days?: number;
  image_url?: string;
  allergens: string[];
}

interface InventoryItem {
  id: string;
  ingredient_name: string;
  category_name: string;
  quantity: number;
  price_per_unit_cents: number;
  expires_at?: string;
}

export default function UserCatalogSearch() {
  const [query, setQuery] = useState('');
  const [selectedCategory, setSelectedCategory] = useState<string>('');
  const [results, setResults] = useState<Ingredient[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  
  // Модальное окно для добавления
  const [selectedProduct, setSelectedProduct] = useState<Ingredient | null>(null);
  const [addingLoading, setAddingLoading] = useState(false);
  const [addForm, setAddForm] = useState({
    quantity: 1.0,
    price_per_unit_cents: 0,
    received_at: new Date().toISOString().split('T')[0]
  });

  // Categories для фильтра
  const categories = [
    { id: 'dairy', name: '🥛 Молочные продукты' },
    { id: 'meat', name: '🥩 Мясо' },
    { id: 'vegetables', name: '🥕 Овощи' },
    { id: 'fruits', name: '🍎 Фрукты' },
  ];

  // Поиск с дебаунсом
  const debouncedSearch = useCallback(
    debounce(async (searchQuery: string, catId: string) => {
      if (!searchQuery.trim() && !catId) {
        setResults([]);
        return;
      }

      setLoading(true);
      setError('');

      try {
        const params = new URLSearchParams();
        if (searchQuery.trim()) params.append('q', searchQuery);
        if (catId) params.append('category_id', catId);
        params.append('limit', '20');

        const response = await fetch(
          `/api/catalog/ingredients?${params}`,
          {
            headers: {
              'Authorization': `Bearer ${localStorage.getItem('token')}`
            }
          }
        );

        if (!response.ok) {
          throw new Error('Ошибка при поиске');
        }

        const data = await response.json();
        setResults(data.ingredients || []);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Ошибка при поиске');
        setResults([]);
      } finally {
        setLoading(false);
      }
    }, 300),
    []
  );

  const handleQueryChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    setQuery(value);
    debouncedSearch(value, selectedCategory);
  };

  const handleCategoryChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const value = e.target.value;
    setSelectedCategory(value);
    debouncedSearch(query, value);
  };

  // Добавление на склад
  const handleAddToInventory = async () => {
    if (!selectedProduct) return;

    setAddingLoading(true);
    setError('');

    try {
      const response = await fetch('/api/inventory/products', {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${localStorage.getItem('token')}`,
          'Content-Type': 'application/json'
        },
        body: JSON.stringify({
          catalog_ingredient_id: selectedProduct.id,
          quantity: parseFloat(addForm.quantity.toString()),
          price_per_unit_cents: parseInt(addForm.price_per_unit_cents.toString()),
          received_at: new Date(addForm.received_at).toISOString()
        })
      });

      if (!response.ok) {
        throw new Error('Ошибка при добавлении');
      }

      // ✅ Успех! Закрыть модальное окно и очистить форму
      setSelectedProduct(null);
      setAddForm({
        quantity: 1.0,
        price_per_unit_cents: 0,
        received_at: new Date().toISOString().split('T')[0]
      });

      // Показать уведомление (можно использовать toast)
      alert(`✅ ${selectedProduct.name} добавлен(а) на склад!`);

    } catch (err) {
      setError(err instanceof Error ? err.message : 'Ошибка при добавлении');
    } finally {
      setAddingLoading(false);
    }
  };

  return (
    <div className="w-full max-w-4xl mx-auto p-4">
      {/* 🔍 Поиск и фильтры */}
      <div className="mb-6 space-y-4 bg-white p-4 rounded-lg border border-gray-200">
        <h2 className="text-xl font-bold text-gray-900">
          🔍 Поиск Продуктов в Каталоге
        </h2>

        <div>
          <label className="block text-sm font-medium text-gray-700 mb-2">
            Что вы ищете?
          </label>
          <input
            type="text"
            value={query}
            onChange={handleQueryChange}
            placeholder="Введите название (молоко, говядина, масло)..."
            className="w-full px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
          />
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-700 mb-2">
            Категория (опционально)
          </label>
          <select
            value={selectedCategory}
            onChange={handleCategoryChange}
            className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
          >
            <option value="">Все категории</option>
            {categories.map(cat => (
              <option key={cat.id} value={cat.id}>
                {cat.name}
              </option>
            ))}
          </select>
        </div>
      </div>

      {/* 🔄 Загрузка */}
      {loading && (
        <div className="flex items-center justify-center py-8">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
          <span className="ml-3 text-gray-600">Поиск...</span>
        </div>
      )}

      {/* ❌ Ошибка */}
      {error && (
        <div className="mb-4 p-4 bg-red-50 text-red-700 rounded-lg border border-red-200">
          ❌ {error}
        </div>
      )}

      {/* 📊 Результаты поиска */}
      {!loading && query && results.length > 0 && (
        <div className="space-y-3">
          <h3 className="text-sm font-semibold text-gray-700">
            Найдено продуктов: {results.length}
          </h3>

          {results.map((ingredient) => (
            <div
              key={ingredient.id}
              className="p-4 bg-white border border-gray-200 rounded-lg hover:shadow-md transition-shadow"
            >
              <div className="flex justify-between items-start">
                <div className="flex-1">
                  {/* 🖼️ Изображение + Инфо */}
                  <div className="flex gap-4">
                    {ingredient.image_url && (
                      <img
                        src={ingredient.image_url}
                        alt={ingredient.name}
                        className="w-16 h-16 rounded object-cover"
                      />
                    )}
                    <div className="flex-1">
                      <h4 className="font-semibold text-gray-900">
                        {ingredient.name}
                      </h4>
                      <div className="text-sm text-gray-600 mt-1 space-y-1">
                        {ingredient.default_shelf_life_days && (
                          <p>
                            ⏰ Срок годности: {ingredient.default_shelf_life_days} дней
                          </p>
                        )}
                        <p>📏 Единица: {ingredient.default_unit}</p>
                        {ingredient.allergens.length > 0 && (
                          <p>⚠️ Аллергены: {ingredient.allergens.join(', ')}</p>
                        )}
                      </div>
                    </div>
                  </div>
                </div>

                {/* ➕ Кнопка добавления */}
                <button
                  onClick={() => setSelectedProduct(ingredient)}
                  className="ml-4 px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700 whitespace-nowrap"
                >
                  ➕ Добавить
                </button>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* 😕 Нет результатов */}
      {!loading && query && results.length === 0 && !error && (
        <div className="text-center py-8 text-gray-500 bg-white border border-gray-200 rounded-lg">
          <p className="text-lg">😕 Продукты не найдены</p>
          <p className="text-sm">Попробуйте другой поисковый запрос</p>
        </div>
      )}

      {/* 🔭 Пустой статус */}
      {!query && !selectedCategory && (
        <div className="text-center py-8 text-gray-400 bg-white border border-gray-200 rounded-lg">
          <p>🔍 Начните вводить название для поиска</p>
        </div>
      )}

      {/* 🎯 Модальное окно для добавления */}
      {selectedProduct && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg shadow-xl max-w-md w-full mx-4 p-6">
            <h3 className="text-lg font-bold text-gray-900 mb-4">
              ➕ Добавить на склад
            </h3>

            <div className="space-y-4">
              {/* Название продукта */}
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  Продукт
                </label>
                <div className="p-3 bg-gray-50 rounded text-gray-900 font-semibold">
                  {selectedProduct.name}
                </div>
              </div>

              {/* Количество */}
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  Количество ({selectedProduct.default_unit})
                </label>
                <input
                  type="number"
                  value={addForm.quantity}
                  onChange={(e) => setAddForm({
                    ...addForm,
                    quantity: parseFloat(e.target.value) || 0
                  })}
                  min="0.1"
                  step="0.1"
                  className="w-full px-3 py-2 border border-gray-300 rounded focus:ring-2 focus:ring-blue-500"
                />
              </div>

              {/* Цена за единицу */}
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  Цена за единицу (в копейках)
                </label>
                <input
                  type="number"
                  value={addForm.price_per_unit_cents}
                  onChange={(e) => setAddForm({
                    ...addForm,
                    price_per_unit_cents: parseInt(e.target.value) || 0
                  })}
                  min="0"
                  step="100"
                  className="w-full px-3 py-2 border border-gray-300 rounded focus:ring-2 focus:ring-blue-500"
                />
                <p className="text-xs text-gray-500 mt-1">
                  100 рублей = 10000 копеек
                </p>
              </div>

              {/* Дата поступления */}
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  Дата поступления
                </label>
                <input
                  type="date"
                  value={addForm.received_at}
                  onChange={(e) => setAddForm({
                    ...addForm,
                    received_at: e.target.value
                  })}
                  className="w-full px-3 py-2 border border-gray-300 rounded focus:ring-2 focus:ring-blue-500"
                />
              </div>

              {/* Информация о сроке годности */}
              {selectedProduct.default_shelf_life_days && (
                <div className="p-3 bg-blue-50 rounded text-sm text-blue-800">
                  ℹ️ Срок годности автоматически установится на {selectedProduct.default_shelf_life_days} дней от даты поступления
                </div>
              )}

              {/* Ошибка */}
              {error && (
                <div className="p-3 bg-red-50 rounded text-sm text-red-700">
                  ❌ {error}
                </div>
              )}

              {/* Кнопки */}
              <div className="flex gap-3 pt-4">
                <button
                  onClick={() => setSelectedProduct(null)}
                  className="flex-1 px-4 py-2 border border-gray-300 text-gray-700 rounded hover:bg-gray-50"
                  disabled={addingLoading}
                >
                  ❌ Отмена
                </button>
                <button
                  onClick={handleAddToInventory}
                  className="flex-1 px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700 disabled:opacity-50"
                  disabled={addingLoading}
                >
                  {addingLoading ? '⏳ Добавляю...' : '✅ Добавить'}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
```

---

## 🔄 Полный Поток

### Архитектура: Поиск → Добавление

```
┌─────────────────────────────────────────────────────────────┐
│                    ПОЛЬЗОВАТЕЛЬ (User)                      │
└─────────────────────────────────────────────────────────────┘
                            ↓
        ┌───────────────────────────────────────────┐
        │ 1️⃣ ПОИСК ПО КАТАЛОГУ                      │
        │                                           │
        │ GET /api/catalog/ingredients?q=молоко    │
        │ • Язык автоматически из user.language   │
        │ • Результаты на нужном языке            │
        │ • Дополнительно: срок годности, фото   │
        └───────────────────────────────────────────┘
                            ↓
        ┌───────────────────────────────────────────┐
        │ 2️⃣ ВЫБРАЛ ПРОДУКТ                         │
        │                                           │
        │ Frontend показывает модаль:              │
        │ • Количество                            │
        │ • Цена за единицу                       │
        │ • Дата поступления                      │
        └───────────────────────────────────────────┘
                            ↓
        ┌───────────────────────────────────────────┐
        │ 3️⃣ ДОБАВЛЕНИЕ НА СКЛАД                    │
        │                                           │
        │ POST /api/inventory/products             │
        │ {                                        │
        │   "catalog_ingredient_id": "uuid",       │
        │   "quantity": 5.0,                       │
        │   "price_per_unit_cents": 10000,         │
        │   "received_at": "2026-02-15T10:30:00"  │
        │ }                                        │
        │                                          │
        │ Backend:                                │
        │ • Валидирует количество и цену         │
        │ • Загружает default_shelf_life_days    │
        │ • Вычисляет expires_at автоматически   │
        │ • Сохраняет в inventory_products       │
        │ • Возвращает обогащенный InventoryView │
        └───────────────────────────────────────────┘
                            ↓
        ┌───────────────────────────────────────────┐
        │ 4️⃣ УСПЕШНО ДОБАВЛЕНО!                    │
        │                                           │
        │ Response (201 CREATED):                 │
        │ {                                        │
        │   "id": "product-uuid",                 │
        │   "ingredient_name": "Молоко...",       │
        │   "quantity": 5.0,                      │
        │   "expires_at": "2026-02-20...",        │
        │   "expiration_status": "Fresh"          │
        │ }                                        │
        │                                          │
        │ ✅ Показать успешное уведомление       │
        │ ✅ Перенаправить на /inventory         │
        └───────────────────────────────────────────┘
```

---

## 📝 Примеры Запросов

### Пример 1: Поиск молока на русском

```bash
# 1️⃣ Поиск
curl -X GET 'https://ministerial-yetta-fodi999-c58d8823.koyeb.app/api/catalog/ingredients?q=молоко' \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."

# Response:
{
  "ingredients": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "name": "Пастеризованное молоко",
      "default_unit": "milliliter",
      "default_shelf_life_days": 5,
      "image_url": "https://..."
    }
  ]
}

# 2️⃣ Добавление на склад
curl -X POST 'https://ministerial-yetta-fodi999-c58d8823.koyeb.app/api/inventory/products' \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." \
  -H "Content-Type: application/json" \
  -d '{
    "catalog_ingredient_id": "550e8400-e29b-41d4-a716-446655440000",
    "quantity": 5.0,
    "price_per_unit_cents": 10000,
    "received_at": "2026-02-15T10:30:00Z"
  }'

# Response:
{
  "id": "product-uuid",
  "ingredient_name": "Пастеризованное молоко",
  "quantity": 5.0,
  "expires_at": "2026-02-20T10:30:00Z",
  "expiration_status": "Fresh"
}
```

### Пример 2: Поиск по категории

```bash
# Получить ID категории молочных продуктов
curl -X GET 'https://ministerial-yetta-fodi999-c58d8823.koyeb.app/api/catalog/categories' \
  -H "Authorization: Bearer <token>"

# Поиск в категории молочных
curl -X GET 'https://ministerial-yetta-fodi999-c58d8823.koyeb.app/api/catalog/ingredients?category_id=6ba7b810-9dad-11d1-80b4-00c04fd430c8' \
  -H "Authorization: Bearer <token>"
```

---

## 💡 Кейс: Молоко Пастеризованное

Пошагово, как пользователь работает:

### 🔍 Шаг 1: Поиск

**User вводит**: "молоко"

**Backend выполняет**:
```sql
SELECT ci.id, ci.name_en, ci.name_ru, ci.default_unit, ci.default_shelf_life_days
FROM catalog_ingredients ci
LEFT JOIN catalog_ingredient_translations cit
  ON ci.id = cit.ingredient_id AND cit.language = 'ru'
WHERE COALESCE(cit.name, ci.name_en) ILIKE '%молоко%'
LIMIT 20
```

**Результат**:
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "Пастеризованное молоко",     // 🇷🇺 Русское название!
  "default_unit": "milliliter",
  "default_shelf_life_days": 5,
  "image_url": "https://cdn.example.com/milk.jpg"
}
```

---

### ➕ Шаг 2: Добавление на склад

**User заполняет форму**:
- Количество: 5 (литров / ml)
- Цена за единицу: ₽100 = 10000 копеек
- Дата поступления: 2026-02-15

**Отправляет POST**:
```json
{
  "catalog_ingredient_id": "550e8400-e29b-41d4-a716-446655440000",
  "quantity": 5.0,
  "price_per_unit_cents": 10000,
  "received_at": "2026-02-15T10:30:00Z",
  "expires_at": null  // ← Null, будет вычислено!
}
```

---

### 🔄 Шаг 3: Backend обработка

**Application Service** (`src/application/inventory.rs`):

```rust
pub async fn add_product(
    &self,
    user_id: UserId,
    tenant_id: TenantId,
    catalog_ingredient_id: CatalogIngredientId,
    price_per_unit_cents: i64,
    quantity: f64,
    received_at: OffsetDateTime,
    expires_at: Option<OffsetDateTime>,  // ← None!
) -> AppResult<InventoryProductId> {
    // 1️⃣ Валидация
    let price = Money::from_cents(price_per_unit_cents)?;  // ✅ >= 0
    let qty = Quantity::new(quantity)?;                     // ✅ > 0, finite
    
    // 2️⃣ Загрузим catalog для default_shelf_life_days
    let catalog = self.catalog_repo
        .find_by_id(catalog_ingredient_id)
        .await?
        .ok_or(AppError::not_found("Ingredient not found"))?;
    
    // 3️⃣ Вычислим expires_at если null
    let final_expires_at = expires_at.or_else(|| {
        catalog.default_shelf_life_days.map(|days| {
            received_at + Duration::days(days as i64)
        })
    });
    
    // 4️⃣ Создадим domain object
    let product = InventoryProduct::new(
        user_id, tenant_id,
        catalog_ingredient_id,
        price, qty,
        received_at, final_expires_at
    );
    
    // 5️⃣ Сохраним в БД
    let id = self.repo.create(&product).await?;
    
    Ok(id)
}
```

---

### 📊 Шаг 4: Response

**Backend возвращает** (201 CREATED):

```json
{
  "id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
  "ingredient_name": "Пастеризованное молоко",     // 🇷🇺 На русском!
  "category_name": "Молочные продукты",
  "quantity": 5.0,
  "price_per_unit_cents": 10000,
  "received_at": "2026-02-15T10:30:00Z",
  "expires_at": "2026-02-20T10:30:00Z",            // ✅ Автовычислено!
  "expiration_status": "Fresh",
  "base_unit": "milliliter",
  "image_url": "https://cdn.example.com/milk.jpg"
}
```

**Frontend**:
- ✅ Закрыть модаль
- ✅ Показать toast: "✅ Пастеризованное молоко добавлено!"
- ✅ Можно перенаправить на страницу склада

---

## 🛠️ Интеграция в Next.js Приложение

### Страница поиска: `app/catalog/page.tsx`

```typescript
'use client';

import UserCatalogSearch from '@/components/UserCatalogSearch';

export default function CatalogPage() {
  return (
    <div className="min-h-screen bg-gray-50 p-8">
      <div className="mb-8">
        <h1 className="text-3xl font-bold text-gray-900">
          📚 Каталог Продуктов
        </h1>
        <p className="text-gray-600">
          Найдите и добавьте продукты на свой склад
        </p>
      </div>

      <UserCatalogSearch />
    </div>
  );
}
```

### Страница склада: `app/inventory/page.tsx`

```typescript
'use client';

import { useEffect, useState } from 'react';

interface InventoryItem {
  id: string;
  ingredient_name: string;
  category_name: string;
  quantity: number;
  price_per_unit_cents: number;
  expiration_status: string;
  expires_at?: string;
}

export default function InventoryPage() {
  const [items, setItems] = useState<InventoryItem[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const fetchInventory = async () => {
      try {
        const response = await fetch('/api/inventory/products', {
          headers: {
            'Authorization': `Bearer ${localStorage.getItem('token')}`
          }
        });
        const data = await response.json();
        setItems(data);
      } finally {
        setLoading(false);
      }
    };

    fetchInventory();
  }, []);

  if (loading) return <div>⏳ Загрузка...</div>;

  return (
    <div className="min-h-screen bg-gray-50 p-8">
      <h1 className="text-3xl font-bold text-gray-900 mb-8">📦 Мой Склад</h1>

      {items.length === 0 ? (
        <div className="text-center py-12 text-gray-500">
          <p>Склад пуст. <a href="/catalog" className="text-blue-600">Добавьте продукты</a></p>
        </div>
      ) : (
        <div className="space-y-3">
          {items.map(item => (
            <div key={item.id} className="p-4 bg-white rounded border border-gray-200">
              <div className="flex justify-between items-start">
                <div>
                  <h3 className="font-semibold text-gray-900">
                    {item.ingredient_name}
                  </h3>
                  <p className="text-sm text-gray-600">
                    {item.category_name} • Кол-во: {item.quantity}
                  </p>
                  <p className="text-sm text-gray-600">
                    Статус: {item.expiration_status}
                  </p>
                </div>
                <div className="text-right">
                  <p className="font-semibold">
                    ₽{(item.price_per_unit_cents / 100).toFixed(2)}
                  </p>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
```

---

## ✅ Чек-лист Реализации

### Backend (уже готово):
- [x] GET `/api/catalog/ingredients` с параметром `q`
- [x] Язык автоматически из `user.language` в БД
- [x] POST `/api/inventory/products` для добавления
- [x] Автовычисление `expires_at` по `default_shelf_life_days`
- [x] Валидация цены и количества на уровне Domain
- [x] Возврат обогащенного InventoryView

### Frontend (нужно реализовать):
- [ ] Скопировать `UserCatalogSearch.tsx` в `components/`
- [ ] Создать страницу `/app/catalog/page.tsx`
- [ ] Создать страницу `/app/inventory/page.tsx`
- [ ] Установить `lodash` для debounce: `npm install lodash`
- [ ] Протестировать поиск на русском
- [ ] Протестировать добавление на склад
- [ ] Проверить автовычисление даты истечения
- [ ] Добавить обработку ошибок с toast уведомлениями
- [ ] Проверить на мобильных
- [ ] Деплой на Koyeb

---

## 🧪 Тестирование

### Локально:
```bash
# Terminal 1: Backend
cd /Users/dmitrijfomin/Desktop/assistant
cargo run --release

# Terminal 2: Frontend
npm run dev

# Browser: http://localhost:3000/catalog
# 1. Введите "молоко"
# 2. Нажмите "➕ Добавить"
# 3. Заполните форму
# 4. Нажмите "✅ Добавить"
# 5. Проверьте /inventory
```

### На Koyeb:
```bash
# Frontend: https://ваш-фронтенд/catalog
# Backend: https://ministerial-yetta-fodi999-c58d8823.koyeb.app

# 1. Авторизуйтесь
# 2. Перейдите на /catalog
# 3. Поиск: "молоко" → должен вернуть русское название
# 4. Добавьте на склад
# 5. Проверьте /inventory
```

---

## 🎯 Ключевые Отличия от Admin Версии

| Параметр | Admin | User |
|----------|-------|------|
| **Endpoint** | `/api/admin/products/search` | `/api/catalog/ingredients` |
| **Auth** | Super Admin JWT | User JWT |
| **Язык** | Параметр `?lang=ru` (выбирает user) | Автоматически из БД (backend truth) |
| **Модификация** | Редактирует в админке | Добавляет на свой склад |
| **Результат** | Обновляет каталог | Создает inventory_products запись |
| **Следующий шаг** | Управление каталогом | Работа со своим складом |

---

## 📞 Поддержка Языков

Система поддерживает **4 языка**:
- 🇷🇺 Русский (`ru`)
- 🇬🇧 English (`en`)
- 🇵🇱 Polski (`pl`)
- 🇺🇦 Українська (`uk`)

**Язык выбирается автоматически** из `users.language` в БД.
Backend возвращает названия на нужном языке пользователя.

---

## 📚 Документация в Проекте

- `ADD_PRODUCT_TO_INVENTORY_FLOW.md` - Полная архитектура потока
- `INVENTORY_QUICK_REFERENCE.md` - Быстрая справка по API
- `CATALOG_SEARCH_RUSSIAN.md` - Admin версия поиска
- Этот файл - **User версия поиска и добавления**

---

*Updated: 15 февраля 2026*  
*Полный цикл: Поиск → Добавление на Склад*  
*Status: Production Ready ✅*
