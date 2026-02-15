# 🔍 Поиск по Каталогу - Русские Названия

**Date**: 15 февраля 2026  
**Frontend Version**: React/Next.js  
**Status**: Production-Ready

---

## 1️⃣ API для Поиска

### Эндпоинт GET

```
GET /api/admin/products/search?q=молоко&lang=ru
```

**Query Parameters**:
```typescript
interface SearchParams {
  q: string;           // Поисковый запрос (любой язык)
  lang?: 'ru' | 'en' | 'pl' | 'uk';  // Язык поиска (default: 'en')
  category_id?: string;  // Фильтр по категории (optional)
  limit?: number;      // Макс результатов (default: 20)
  offset?: number;     // Для пагинации (default: 0)
}
```

**Response**:
```typescript
interface SearchResponse {
  data: Product[];
  total: number;
  limit: number;
  offset: number;
}

interface Product {
  id: string;
  name_en: string;
  name_ru: string;
  name_pl: string;
  name_uk: string;
  category_id: string;
  default_unit: string;
  image_url?: string;
  created_at: string;
}
```

---

## 2️⃣ Простой Компонент Поиска

### `components/CatalogSearch.tsx`

```typescript
'use client';

import { useState, useCallback } from 'react';
import debounce from 'lodash/debounce';

interface Product {
  id: string;
  name_en: string;
  name_ru: string;
  name_pl: string;
  name_uk: string;
  default_unit: string;
}

export default function CatalogSearch() {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<Product[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  // Debounce для предотвращения множественных запросов
  const debouncedSearch = useCallback(
    debounce(async (searchQuery: string) => {
      if (!searchQuery.trim()) {
        setResults([]);
        return;
      }

      setLoading(true);
      setError('');

      try {
        // ✅ Поиск по русским названиям
        const response = await fetch(
          `/api/admin/products/search?q=${encodeURIComponent(searchQuery)}&lang=ru`,
          {
            headers: {
              'Authorization': `Bearer ${localStorage.getItem('adminToken')}`
            }
          }
        );

        if (!response.ok) {
          throw new Error('Ошибка при поиске');
        }

        const data = await response.json();
        setResults(data.data || []);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Неизвестная ошибка');
        setResults([]);
      } finally {
        setLoading(false);
      }
    }, 300), // Задержка 300ms перед поиском
    []
  );

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    setQuery(value);
    debouncedSearch(value);
  };

  return (
    <div className="w-full max-w-2xl mx-auto p-4">
      <div className="mb-6">
        <label className="block text-sm font-medium text-gray-700 mb-2">
          🔍 Поиск по названиям
        </label>
        <input
          type="text"
          value={query}
          onChange={handleInputChange}
          placeholder="Введите название на русском... (молоко, говядина, масло)"
          className="w-full px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
        />
        <p className="text-xs text-gray-500 mt-1">
          Введите название на русском, английском, польском или украинском
        </p>
      </div>

      {/* Состояние загрузки */}
      {loading && (
        <div className="flex items-center justify-center py-8">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
          <span className="ml-3 text-gray-600">Поиск...</span>
        </div>
      )}

      {/* Ошибка */}
      {error && (
        <div className="mb-4 p-4 bg-red-50 text-red-700 rounded-lg border border-red-200">
          ❌ {error}
        </div>
      )}

      {/* Результаты поиска */}
      {!loading && query && results.length > 0 && (
        <div className="space-y-3">
          <h3 className="text-sm font-semibold text-gray-700">
            Найдено: {results.length} продуктов
          </h3>
          {results.map((product) => (
            <div
              key={product.id}
              className="p-4 border border-gray-200 rounded-lg hover:shadow-md transition-shadow"
            >
              <div className="flex justify-between items-start">
                <div className="flex-1">
                  <h4 className="font-semibold text-gray-900">
                    🇷🇺 {product.name_ru}
                  </h4>
                  <p className="text-sm text-gray-600">
                    🇬🇧 {product.name_en} | 🇵🇱 {product.name_pl} | 🇺🇦 {product.name_uk}
                  </p>
                  <p className="text-xs text-gray-500 mt-1">
                    Единица: <span className="font-mono">{product.default_unit}</span>
                  </p>
                </div>
                <button
                  className="ml-4 px-3 py-1 bg-blue-600 text-white rounded text-sm hover:bg-blue-700"
                >
                  Выбрать
                </button>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Нет результатов */}
      {!loading && query && results.length === 0 && !error && (
        <div className="text-center py-8 text-gray-500">
          <p>😕 Продукты не найдены</p>
          <p className="text-sm">Попробуйте другой поисковый запрос</p>
        </div>
      )}

      {/* Пустой статус */}
      {!query && (
        <div className="text-center py-8 text-gray-400">
          <p>Начните вводить название для поиска</p>
        </div>
      )}
    </div>
  );
}
```

---

## 3️⃣ Продвинутый Компонент с Фильтрами

### `components/CatalogSearchAdvanced.tsx`

```typescript
'use client';

import { useState, useCallback } from 'react';
import debounce from 'lodash/debounce';

interface SearchFilters {
  query: string;
  lang: 'ru' | 'en' | 'pl' | 'uk';
  category_id?: string;
  limit: number;
  offset: number;
}

interface Product {
  id: string;
  name_en: string;
  name_ru: string;
  name_pl: string;
  name_uk: string;
  default_unit: string;
  category_id: string;
}

interface Category {
  id: string;
  name: string;
}

export default function CatalogSearchAdvanced() {
  const [filters, setFilters] = useState<SearchFilters>({
    query: '',
    lang: 'ru',
    limit: 20,
    offset: 0
  });

  const [results, setResults] = useState<Product[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [total, setTotal] = useState(0);

  // Категории (можно получить из API)
  const categories: Category[] = [
    { id: 'dairy_and_eggs', name: '🥛 Молочные продукты' },
    { id: 'meat', name: '🥩 Мясо' },
    { id: 'fruits', name: '🍎 Фрукты' },
    { id: 'vegetables', name: '🥕 Овощи' },
    { id: 'grains', name: '🌾 Зерновые' },
    { id: 'seafood', name: '🐟 Морепродукты' },
    { id: 'beverages', name: '🥤 Напитки' }
  ];

  const debouncedSearch = useCallback(
    debounce(async (searchFilters: SearchFilters) => {
      if (!searchFilters.query.trim()) {
        setResults([]);
        return;
      }

      setLoading(true);
      setError('');

      try {
        // Построить URL с параметрами
        const params = new URLSearchParams({
          q: searchFilters.query,
          lang: searchFilters.lang,
          limit: searchFilters.limit.toString(),
          offset: searchFilters.offset.toString()
        });

        if (searchFilters.category_id) {
          params.append('category_id', searchFilters.category_id);
        }

        const response = await fetch(
          `/api/admin/products/search?${params}`,
          {
            headers: {
              'Authorization': `Bearer ${localStorage.getItem('adminToken')}`
            }
          }
        );

        if (!response.ok) {
          throw new Error('Ошибка при поиске');
        }

        const data = await response.json();
        setResults(data.data || []);
        setTotal(data.total || 0);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Неизвестная ошибка');
        setResults([]);
      } finally {
        setLoading(false);
      }
    }, 300),
    []
  );

  const handleQueryChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newFilters = { ...filters, query: e.target.value, offset: 0 };
    setFilters(newFilters);
    debouncedSearch(newFilters);
  };

  const handleLangChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const newFilters = {
      ...filters,
      lang: e.target.value as 'ru' | 'en' | 'pl' | 'uk',
      offset: 0
    };
    setFilters(newFilters);
    debouncedSearch(newFilters);
  };

  const handleCategoryChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const newFilters = {
      ...filters,
      category_id: e.target.value || undefined,
      offset: 0
    };
    setFilters(newFilters);
    debouncedSearch(newFilters);
  };

  return (
    <div className="w-full max-w-4xl mx-auto p-4">
      {/* Поиск и фильтры */}
      <div className="mb-6 space-y-4">
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-2">
            🔍 Поиск продуктов
          </label>
          <input
            type="text"
            value={filters.query}
            onChange={handleQueryChange}
            placeholder="Введите название (молоко, говядина, масло)..."
            className="w-full px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
          />
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              Язык поиска
            </label>
            <select
              value={filters.lang}
              onChange={handleLangChange}
              className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
            >
              <option value="ru">🇷🇺 Русский</option>
              <option value="en">🇬🇧 English</option>
              <option value="pl">🇵🇱 Polski</option>
              <option value="uk">🇺🇦 Українська</option>
            </select>
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              Категория
            </label>
            <select
              value={filters.category_id || ''}
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
      </div>

      {/* Загрузка */}
      {loading && (
        <div className="flex items-center justify-center py-8">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
          <span className="ml-3 text-gray-600">Поиск...</span>
        </div>
      )}

      {/* Ошибка */}
      {error && (
        <div className="mb-4 p-4 bg-red-50 text-red-700 rounded-lg">
          ❌ {error}
        </div>
      )}

      {/* Результаты */}
      {!loading && filters.query && results.length > 0 && (
        <div className="space-y-3">
          <div className="flex justify-between items-center">
            <h3 className="text-sm font-semibold text-gray-700">
              Найдено: {total} продуктов
            </h3>
            <span className="text-xs text-gray-500">
              Показано: {results.length} из {total}
            </span>
          </div>

          {results.map((product) => (
            <div
              key={product.id}
              className="p-4 border border-gray-200 rounded-lg hover:shadow-md transition-shadow"
            >
              <div className="flex justify-between items-start">
                <div className="flex-1">
                  <h4 className="font-semibold text-gray-900">
                    {filters.lang === 'ru' ? '🇷🇺' : 
                     filters.lang === 'en' ? '🇬🇧' :
                     filters.lang === 'pl' ? '🇵🇱' : '🇺🇦'}{' '}
                    {filters.lang === 'ru' ? product.name_ru :
                     filters.lang === 'en' ? product.name_en :
                     filters.lang === 'pl' ? product.name_pl :
                     product.name_uk}
                  </h4>
                  <p className="text-sm text-gray-600 mt-1">
                    🇷🇺 {product.name_ru} | 
                    🇬🇧 {product.name_en} | 
                    🇵🇱 {product.name_pl}
                  </p>
                  <p className="text-xs text-gray-500 mt-2">
                    Единица: <span className="font-mono bg-gray-100 px-2 py-1 rounded">{product.default_unit}</span>
                  </p>
                </div>
                <button className="ml-4 px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700">
                  Выбрать
                </button>
              </div>
            </div>
          ))}

          {/* Пагинация */}
          {total > filters.limit && (
            <div className="flex justify-center gap-2 mt-4">
              <button
                onClick={() => {
                  const newFilters = { ...filters, offset: Math.max(0, filters.offset - filters.limit) };
                  setFilters(newFilters);
                  debouncedSearch(newFilters);
                }}
                disabled={filters.offset === 0}
                className="px-4 py-2 border border-gray-300 rounded disabled:opacity-50"
              >
                ← Назад
              </button>
              <span className="px-4 py-2 text-gray-600">
                {Math.floor(filters.offset / filters.limit) + 1} / {Math.ceil(total / filters.limit)}
              </span>
              <button
                onClick={() => {
                  const newFilters = { ...filters, offset: filters.offset + filters.limit };
                  setFilters(newFilters);
                  debouncedSearch(newFilters);
                }}
                disabled={filters.offset + filters.limit >= total}
                className="px-4 py-2 border border-gray-300 rounded disabled:opacity-50"
              >
                Вперед →
              </button>
            </div>
          )}
        </div>
      )}

      {/* Нет результатов */}
      {!loading && filters.query && results.length === 0 && !error && (
        <div className="text-center py-8 text-gray-500">
          <p>😕 Продукты не найдены</p>
          <p className="text-sm">Попробуйте изменить поисковый запрос или фильтры</p>
        </div>
      )}

      {/* Пустое состояние */}
      {!filters.query && (
        <div className="text-center py-8 text-gray-400">
          <p>🔍 Начните вводить название продукта</p>
        </div>
      )}
    </div>
  );
}
```

---

## 4️⃣ Использование в Приложении

### Просто добавьте компонент на страницу каталога:

**`app/admin/catalog/page.tsx`**

```typescript
'use client';

import CatalogSearchAdvanced from '@/components/CatalogSearchAdvanced';

export default function CatalogPage() {
  return (
    <div className="min-h-screen bg-gray-50 p-8">
      <div className="mb-8">
        <h1 className="text-3xl font-bold text-gray-900 mb-2">
          📚 Каталог Продуктов
        </h1>
        <p className="text-gray-600">
          Поиск и управление ингредиентами в базе данных
        </p>
      </div>

      {/* Компонент поиска */}
      <CatalogSearchAdvanced />

      {/* Или более простая версия */}
      {/* <CatalogSearch /> */}
    </div>
  );
}
```

---

## 5️⃣ Backend API (Русский поиск)

Ваш бэкенд должен поддерживать поиск. Вот пример на Rust:

### `src/interfaces/http/admin_catalog.rs`

```rust
/// Search products by name in any language
pub async fn search_products(
    Query(params): Query<SearchProductsParams>,
    State(service): State<Arc<AdminCatalogService>>,
) -> Result<Json<SearchResponse>, AppError> {
    let query = params.q.to_lowercase().trim().to_string();
    
    if query.is_empty() {
        return Err(AppError::validation("Search query cannot be empty"));
    }

    let limit = params.limit.unwrap_or(20).min(100);
    let offset = params.offset.unwrap_or(0);
    let lang = params.lang.unwrap_or("en".to_string());

    // Поиск по названиям в указанном языке
    let results = service
        .search_ingredients_by_language(&query, &lang, params.category_id.as_deref(), limit, offset)
        .await?;

    Ok(Json(SearchResponse {
        data: results.products,
        total: results.total,
        limit,
        offset,
    }))
}

#[derive(Deserialize)]
pub struct SearchProductsParams {
    pub q: String,                    // Search query
    pub lang: Option<String>,         // Language: ru, en, pl, uk
    pub category_id: Option<String>,  // Category filter
    pub limit: Option<i32>,           // Max results
    pub offset: Option<i32>,          // Pagination offset
}

#[derive(Serialize)]
pub struct SearchResponse {
    pub data: Vec<Product>,
    pub total: i32,
    pub limit: i32,
    pub offset: i32,
}
```

---

## 6️⃣ Установка Зависимостей

```bash
# Install debounce utility
npm install lodash
npm install --save-dev @types/lodash

# Или используйте встроенное решение без внешней библиотеки
```

---

## 7️⃣ Альтернатива без Debounce

Если не хотите устанавливать `lodash`, вот встроенное решение:

```typescript
const debouncedSearch = useCallback(() => {
  const timer = setTimeout(async () => {
    if (!query.trim()) {
      setResults([]);
      return;
    }

    setLoading(true);
    try {
      const response = await fetch(
        `/api/admin/products/search?q=${encodeURIComponent(query)}&lang=ru`,
        {
          headers: {
            'Authorization': `Bearer ${localStorage.getItem('adminToken')}`
          }
        }
      );
      const data = await response.json();
      setResults(data.data || []);
    } catch (err) {
      setError('Ошибка при поиске');
    } finally {
      setLoading(false);
    }
  }, 300);

  return () => clearTimeout(timer);
}, [query]);

useEffect(() => {
  debouncedSearch();
}, [query, debouncedSearch]);
```

---

## 8️⃣ Примеры Поисковых Запросов

```bash
# Поиск по русски
curl 'https://api.fodi.app/api/admin/products/search?q=молоко&lang=ru'

# Поиск по английски
curl 'https://api.fodi.app/api/admin/products/search?q=milk&lang=en'

# Поиск с фильтром по категории
curl 'https://api.fodi.app/api/admin/products/search?q=молоко&lang=ru&category_id=dairy_and_eggs'

# Поиск с пагинацией
curl 'https://api.fodi.app/api/admin/products/search?q=говядина&lang=ru&limit=10&offset=0'
```

---

## 9️⃣ Тестирование

### Локально:
```bash
# Terminal 1: Backend
cd /Users/dmitrijfomin/Desktop/assistant
cargo run --release

# Terminal 2: Frontend
npm run dev

# Browser: http://localhost:3000/admin/catalog
```

### На продакшене:
```bash
# Откройте https://ваш-фронтенд/admin/catalog
# Введите в поиск: "молоко"
# Должны появиться результаты с русскими названиями
```

---

## 🔟 Возможные Улучшения

- ✅ Поиск по всем языкам
- ✅ Фильтр по категориям
- ✅ Пагинация результатов
- ✅ Подсказки (автодополнение)
- ✅ История поисков
- ✅ Избранные продукты
- ✅ Быстрое добавление в рецепты

---

## Чек-лист Интеграции

- [ ] Скопируйте `CatalogSearchAdvanced.tsx` в `components/`
- [ ] Добавьте компонент на страницу каталога
- [ ] Установите зависимость `lodash` (если нужна)
- [ ] Протестируйте поиск с русскими названиями
- [ ] Добавьте стили (если нужны)
- [ ] Проверьте работу на мобильных
- [ ] Деплой на продакшен

---

*Updated: 15 февраля 2026*  
*Поиск по каталогу: Русский, английский, польский, украинский*  
*Status: Production Ready ✅*
