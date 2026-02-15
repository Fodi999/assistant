# 🚀 Готовые Компоненты для User Поиска и Добавления на Склад

**Status**: Copy-Paste Ready ✅  
**Language**: TypeScript/React  
**Framework**: Next.js 14 + TailwindCSS

---

## 📦 Что находится в этом файле

1. `UserCatalogSearch.tsx` - Полный компонент с поиском + добавлением
2. `useInventory.ts` - Hook для работы со складом
3. `useCatalogSearch.ts` - Hook для поиска по каталогу
4. Примеры страниц `/catalog` и `/inventory`
5. Утилиты и типы

---

## 🎨 Компонент 1: UserCatalogSearch.tsx

### Файл: `components/UserCatalogSearch.tsx`

```typescript
'use client';

import { useState, useCallback } from 'react';
import debounce from 'lodash/debounce';
import { useCatalogSearch } from '@/hooks/useCatalogSearch';
import { useInventory } from '@/hooks/useInventory';
import type { Ingredient, AddToInventoryInput } from '@/types/catalog';

export default function UserCatalogSearch() {
  const { search, results, loading: searchLoading, error: searchError } = useCatalogSearch();
  const { addProduct, loading: addingLoading, error: addingError } = useInventory();

  const [query, setQuery] = useState('');
  const [selectedCategory, setSelectedCategory] = useState<string>('');
  const [selectedProduct, setSelectedProduct] = useState<Ingredient | null>(null);
  
  const [addForm, setAddForm] = useState({
    quantity: 1.0,
    price_per_unit_cents: 0,
    received_at: new Date().toISOString().split('T')[0]
  });

  // Категории (можно получить из API позже)
  const categories = [
    { id: 'dairy', name: '🥛 Молочные продукты' },
    { id: 'meat', name: '🥩 Мясо' },
    { id: 'vegetables', name: '🥕 Овощи' },
    { id: 'fruits', name: '🍎 Фрукты' },
  ];

  // Дебаунсированный поиск
  const debouncedSearch = useCallback(
    debounce(async (q: string, catId: string) => {
      if (!q.trim() && !catId) {
        return;
      }
      await search(q, catId);
    }, 300),
    [search]
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

  const handleAddToInventory = async () => {
    if (!selectedProduct) return;

    const input: AddToInventoryInput = {
      catalog_ingredient_id: selectedProduct.id,
      quantity: parseFloat(addForm.quantity.toString()),
      price_per_unit_cents: parseInt(addForm.price_per_unit_cents.toString()),
      received_at: new Date(addForm.received_at).toISOString()
    };

    const success = await addProduct(input);
    
    if (success) {
      setSelectedProduct(null);
      setAddForm({
        quantity: 1.0,
        price_per_unit_cents: 0,
        received_at: new Date().toISOString().split('T')[0]
      });
      // TODO: Show toast success
    }
  };

  return (
    <div className="w-full max-w-4xl mx-auto p-4">
      {/* 🔍 Поиск и фильтры */}
      <div className="mb-6 space-y-4 bg-white p-4 rounded-lg border border-gray-200">
        <h2 className="text-xl font-bold text-gray-900">
          🔍 Поиск Продуктов в Каталоге
        </h2>

        {/* Поле поиска */}
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

        {/* Фильтр по категории */}
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
      {searchLoading && (
        <div className="flex items-center justify-center py-8">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
          <span className="ml-3 text-gray-600">Поиск...</span>
        </div>
      )}

      {/* ❌ Ошибка поиска */}
      {searchError && (
        <div className="mb-4 p-4 bg-red-50 text-red-700 rounded-lg border border-red-200">
          ❌ {searchError}
        </div>
      )}

      {/* 📊 Результаты поиска */}
      {!searchLoading && query && results.length > 0 && (
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
                          <p>⏰ Срок: {ingredient.default_shelf_life_days} дней</p>
                        )}
                        <p>📏 Единица: {ingredient.default_unit}</p>
                        {ingredient.allergens.length > 0 && (
                          <p>⚠️ Аллергены: {ingredient.allergens.join(', ')}</p>
                        )}
                      </div>
                    </div>
                  </div>
                </div>

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
      {!searchLoading && query && results.length === 0 && !searchError && (
        <div className="text-center py-8 text-gray-500 bg-white border border-gray-200 rounded-lg">
          <p className="text-lg">😕 Продукты не найдены</p>
          <p className="text-sm">Попробуйте другой запрос</p>
        </div>
      )}

      {/* 🎯 Модальное окно добавления */}
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
                  ₽100 = 10000 копеек
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

              {/* Инфо о сроке годности */}
              {selectedProduct.default_shelf_life_days && (
                <div className="p-3 bg-blue-50 rounded text-sm text-blue-800">
                  ℹ️ Срок выставится на {selectedProduct.default_shelf_life_days} дней
                </div>
              )}

              {/* Ошибка */}
              {addingError && (
                <div className="p-3 bg-red-50 rounded text-sm text-red-700">
                  ❌ {addingError}
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

## 🪝 Hook 1: useCatalogSearch.ts

### Файл: `hooks/useCatalogSearch.ts`

```typescript
'use client';

import { useState } from 'react';
import type { Ingredient } from '@/types/catalog';

export function useCatalogSearch() {
  const [results, setResults] = useState<Ingredient[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string>('');

  const search = async (query: string, categoryId: string = '') => {
    setLoading(true);
    setError('');

    try {
      const params = new URLSearchParams();
      
      if (query.trim()) {
        params.append('q', query);
      }
      
      if (categoryId) {
        params.append('category_id', categoryId);
      }
      
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
      const message = err instanceof Error ? err.message : 'Неизвестная ошибка';
      setError(message);
      setResults([]);
    } finally {
      setLoading(false);
    }
  };

  return {
    results,
    loading,
    error,
    search
  };
}
```

---

## 🪝 Hook 2: useInventory.ts

### Файл: `hooks/useInventory.ts`

```typescript
'use client';

import { useState, useCallback } from 'react';
import type { AddToInventoryInput, InventoryItem } from '@/types/catalog';

export function useInventory() {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string>('');
  const [items, setItems] = useState<InventoryItem[]>([]);

  // Добавить продукт на склад
  const addProduct = useCallback(async (input: AddToInventoryInput): Promise<boolean> => {
    setLoading(true);
    setError('');

    try {
      const response = await fetch('/api/inventory/products', {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${localStorage.getItem('token')}`,
          'Content-Type': 'application/json'
        },
        body: JSON.stringify(input)
      });

      if (!response.ok) {
        const errorData = await response.json();
        throw new Error(errorData.message || 'Ошибка при добавлении');
      }

      // Успех!
      return true;
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Неизвестная ошибка';
      setError(message);
      return false;
    } finally {
      setLoading(false);
    }
  }, []);

  // Получить все товары на складе
  const fetchItems = useCallback(async () => {
    setLoading(true);
    setError('');

    try {
      const response = await fetch('/api/inventory/products', {
        headers: {
          'Authorization': `Bearer ${localStorage.getItem('token')}`
        }
      });

      if (!response.ok) {
        throw new Error('Ошибка при загрузке');
      }

      const data = await response.json();
      setItems(data);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Неизвестная ошибка';
      setError(message);
    } finally {
      setLoading(false);
    }
  }, []);

  // Удалить товар
  const deleteItem = useCallback(async (id: string): Promise<boolean> => {
    setLoading(true);
    setError('');

    try {
      const response = await fetch(`/api/inventory/products/${id}`, {
        method: 'DELETE',
        headers: {
          'Authorization': `Bearer ${localStorage.getItem('token')}`
        }
      });

      if (!response.ok) {
        throw new Error('Ошибка при удалении');
      }

      // Удалить из локального состояния
      setItems(items.filter(item => item.id !== id));
      return true;
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Неизвестная ошибка';
      setError(message);
      return false;
    } finally {
      setLoading(false);
    }
  }, [items]);

  return {
    items,
    loading,
    error,
    addProduct,
    fetchItems,
    deleteItem
  };
}
```

---

## 📦 Types: types/catalog.ts

### Файл: `types/catalog.ts`

```typescript
// Ингредиент из каталога
export interface Ingredient {
  id: string;
  category_id: string;
  name: string;
  default_unit: string;
  default_shelf_life_days?: number;
  allergens: string[];
  calories_per_100g?: number;
  seasons: string[];
  image_url?: string;
}

// Товар на складе
export interface InventoryItem {
  id: string;
  catalog_ingredient_id: string;
  ingredient_name: string;
  category_name: string;
  base_unit: string;
  image_url?: string;
  quantity: number;
  price_per_unit_cents: number;
  received_at: string;
  expires_at?: string;
  expiration_status: 'Expired' | 'ExpiresToday' | 'ExpiringSoon' | 'Fresh' | 'NoExpiration';
  created_at: string;
  updated_at: string;
}

// Форма для добавления товара
export interface AddToInventoryInput {
  catalog_ingredient_id: string;
  quantity: number;
  price_per_unit_cents: number;
  received_at: string;
  expires_at?: string;
}
```

---

## 🖼️ Страница 1: /app/catalog/page.tsx

### Файл: `app/catalog/page.tsx`

```typescript
'use client';

import { useRouter } from 'next/navigation';
import UserCatalogSearch from '@/components/UserCatalogSearch';
import Link from 'next/link';

export default function CatalogPage() {
  const router = useRouter();

  return (
    <div className="min-h-screen bg-gray-50">
      {/* Header */}
      <div className="bg-white border-b border-gray-200">
        <div className="max-w-4xl mx-auto px-4 py-6">
          <div className="flex justify-between items-center mb-4">
            <div>
              <h1 className="text-3xl font-bold text-gray-900">
                📚 Каталог Продуктов
              </h1>
              <p className="text-gray-600 mt-2">
                Найдите и добавьте продукты на свой склад
              </p>
            </div>
            <Link
              href="/inventory"
              className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700"
            >
              📦 Мой Склад
            </Link>
          </div>
        </div>
      </div>

      {/* Content */}
      <div className="py-8">
        <UserCatalogSearch />
      </div>
    </div>
  );
}
```

---

## 📦 Страница 2: /app/inventory/page.tsx

### Файл: `app/inventory/page.tsx`

```typescript
'use client';

import { useEffect } from 'react';
import Link from 'next/link';
import { useInventory } from '@/hooks/useInventory';
import type { InventoryItem } from '@/types/catalog';

const ExpirationStatusBadge = ({ status }: { status: string }) => {
  const colors = {
    'Expired': 'bg-red-100 text-red-800',
    'ExpiresToday': 'bg-red-50 text-red-700',
    'ExpiringSoon': 'bg-yellow-50 text-yellow-800',
    'Fresh': 'bg-green-50 text-green-800',
    'NoExpiration': 'bg-gray-50 text-gray-800'
  };

  const icons = {
    'Expired': '❌',
    'ExpiresToday': '⏰',
    'ExpiringSoon': '⚠️',
    'Fresh': '✅',
    'NoExpiration': '♾️'
  };

  return (
    <span className={`px-2 py-1 rounded text-xs font-semibold ${colors[status as keyof typeof colors]}`}>
      {icons[status as keyof typeof icons]} {status}
    </span>
  );
};

export default function InventoryPage() {
  const { items, loading, error, fetchItems, deleteItem } = useInventory();

  useEffect(() => {
    fetchItems();
  }, [fetchItems]);

  return (
    <div className="min-h-screen bg-gray-50">
      {/* Header */}
      <div className="bg-white border-b border-gray-200">
        <div className="max-w-4xl mx-auto px-4 py-6">
          <div className="flex justify-between items-center">
            <div>
              <h1 className="text-3xl font-bold text-gray-900">
                📦 Мой Склад
              </h1>
              <p className="text-gray-600 mt-2">
                {items.length} продукт(ов) на складе
              </p>
            </div>
            <Link
              href="/catalog"
              className="px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700"
            >
              ➕ Добавить продукт
            </Link>
          </div>
        </div>
      </div>

      {/* Content */}
      <div className="max-w-4xl mx-auto px-4 py-8">
        {/* Загрузка */}
        {loading && (
          <div className="flex items-center justify-center py-12">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
            <span className="ml-3 text-gray-600">Загрузка...</span>
          </div>
        )}

        {/* Ошибка */}
        {error && (
          <div className="mb-4 p-4 bg-red-50 text-red-700 rounded-lg border border-red-200">
            ❌ {error}
          </div>
        )}

        {/* Пусто */}
        {!loading && items.length === 0 && (
          <div className="text-center py-12 bg-white rounded-lg border border-gray-200">
            <p className="text-lg text-gray-500">📭 Склад пуст</p>
            <p className="text-sm text-gray-400 mt-2">
              <Link href="/catalog" className="text-blue-600 hover:underline">
                Добавьте продукты из каталога
              </Link>
            </p>
          </div>
        )}

        {/* Список товаров */}
        {!loading && items.length > 0 && (
          <div className="space-y-3">
            {items.map((item) => (
              <div
                key={item.id}
                className="bg-white border border-gray-200 rounded-lg p-4 hover:shadow-md transition-shadow"
              >
                <div className="flex justify-between items-start">
                  <div className="flex gap-4 flex-1">
                    {/* Изображение */}
                    {item.image_url && (
                      <img
                        src={item.image_url}
                        alt={item.ingredient_name}
                        className="w-16 h-16 rounded object-cover"
                      />
                    )}

                    {/* Информация */}
                    <div className="flex-1">
                      <h3 className="font-semibold text-gray-900">
                        {item.ingredient_name}
                      </h3>
                      <p className="text-sm text-gray-600 mt-1">
                        {item.category_name} • 📏 {item.quantity} {item.base_unit}
                      </p>

                      {/* Статус и дата истечения */}
                      <div className="flex items-center gap-2 mt-2">
                        <ExpirationStatusBadge status={item.expiration_status} />
                        {item.expires_at && (
                          <span className="text-xs text-gray-500">
                            {new Date(item.expires_at).toLocaleDateString('ru-RU')}
                          </span>
                        )}
                      </div>
                    </div>
                  </div>

                  {/* Цена и действия */}
                  <div className="text-right">
                    <p className="font-semibold text-gray-900">
                      ₽{(item.price_per_unit_cents / 100).toFixed(2)}
                    </p>
                    <button
                      onClick={() => deleteItem(item.id)}
                      className="mt-2 text-sm text-red-600 hover:text-red-700"
                    >
                      🗑️ Удалить
                    </button>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
```

---

## 🧪 Быстрая Проверка

### Установка зависимостей:
```bash
npm install lodash
npm install --save-dev @types/lodash
```

### Структура файлов:
```
src/
├── app/
│   ├── catalog/
│   │   └── page.tsx          # 👈 Поиск
│   └── inventory/
│       └── page.tsx          # 👈 Мой склад
├── components/
│   └── UserCatalogSearch.tsx # 👈 Основной компонент
├── hooks/
│   ├── useCatalogSearch.ts   # 👈 Hook для поиска
│   └── useInventory.ts       # 👈 Hook для склада
└── types/
    └── catalog.ts            # 👈 TypeScript типы
```

### Локальное тестирование:
```bash
# Terminal 1: Backend
cd /Users/dmitrijfomin/Desktop/assistant
cargo run --release

# Terminal 2: Frontend
npm run dev

# Browser:
# 1. http://localhost:3000/catalog
# 2. Поиск: "молоко"
# 3. Нажать "➕ Добавить"
# 4. Заполнить форму
# 5. Нажать "✅ Добавить"
# 6. Перейти на http://localhost:3000/inventory
```

### На Koyeb:
```
https://your-frontend/catalog
```

---

## 📋 Чек-лист Copy-Paste

- [ ] Создать файл `components/UserCatalogSearch.tsx`
- [ ] Создать файл `hooks/useCatalogSearch.ts`
- [ ] Создать файл `hooks/useInventory.ts`
- [ ] Создать файл `types/catalog.ts`
- [ ] Создать файл `app/catalog/page.tsx`
- [ ] Создать файл `app/inventory/page.tsx`
- [ ] Установить lodash: `npm install lodash @types/lodash`
- [ ] Протестировать локально
- [ ] Проверить поиск на русском (молоко, говядина)
- [ ] Проверить добавление на склад
- [ ] Проверить автовычисление даты истечения
- [ ] Деплой на Koyeb

---

## 🎨 Кастомизация

### Изменить категории:

В `UserCatalogSearch.tsx` обновить:
```typescript
const categories = [
  { id: 'dairy', name: '🥛 Молочные продукты' },
  { id: 'meat', name: '🥩 Мясо' },
  // Добавить свои...
];
```

### Изменить стили:

Все компоненты используют TailwindCSS, просто обновить className.

### Добавить toast уведомления:

```typescript
import { useToast } from '@/hooks/useToast'; // или react-hot-toast

// В handleAddToInventory:
if (success) {
  toast.success('✅ Добавлено на склад!');
}
```

---

*Updated: 15 февраля 2026*  
*Copy-Paste Ready ✅ Production Quality*
