# 🌍 Перевод категорий на русский язык

## 🎯 Проблема

Backend возвращает категории на **английском языке**, но в админ-панели нужно отображать их **на русском**.

```json
// Ответ от API
{
  "categories": [
    { "id": "uuid", "name": "Vegetables", "sort_order": 4 },
    { "id": "uuid", "name": "Fruits", "sort_order": 5 }
  ]
}
```

Нужно показывать: **"Овощи"** вместо "Vegetables" и **"Фрукты"** вместо "Fruits".

---

## ✅ Решение: Перевод на фронтенде

### 1. Создаём файл с переводами

```typescript
// utils/categoryTranslations.ts

/**
 * Словарь переводов категорий с английского на русский
 */
export const categoryTranslations: Record<string, string> = {
  'Dairy & Eggs': 'Молочные продукты и яйца',
  'Meat & Poultry': 'Мясо и птица',
  'Fish & Seafood': 'Рыба и морепродукты',
  'Vegetables': 'Овощи',
  'Fruits': 'Фрукты',
  'Grains & Pasta': 'Крупы и макароны',
  'Oils & Fats': 'Масла и жиры',
  'Spices & Herbs': 'Специи и травы',
  'Condiments & Sauces': 'Приправы и соусы',
  'Beverages': 'Напитки',
  'Nuts & Seeds': 'Орехи и семена',
  'Legumes': 'Бобовые',
  'Sweets & Baking': 'Сладости и выпечка',
  'Canned & Preserved': 'Консервы',
  'Frozen': 'Замороженные продукты'
};

/**
 * Функция для получения переведённого названия категории
 * @param englishName - Английское название категории
 * @returns Русское название или оригинальное, если перевод не найден
 */
export const translateCategory = (englishName: string): string => {
  return categoryTranslations[englishName] || englishName;
};

/**
 * Функция для получения всех категорий с переводами
 */
export const getAllTranslations = () => categoryTranslations;
```

---

### 2. Используем в хуке загрузки категорий

```typescript
// hooks/useCategories.ts
import { useState, useEffect } from 'react';
import { translateCategory } from '@/utils/categoryTranslations';

interface Category {
  id: string;
  name: string;        // Английское название (от API)
  name_ru?: string;    // Русское название (добавляем на фронте)
  sort_order: number;
}

export const useCategories = () => {
  const [categories, setCategories] = useState<Category[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchCategories = async () => {
      try {
        const response = await fetch(
          'https://ministerial-yetta-fodi999-c58d8823.koyeb.app/api/admin/categories',
          {
            headers: {
              'Authorization': `Bearer ${localStorage.getItem('admin_token')}`
            }
          }
        );

        if (!response.ok) {
          throw new Error('Не удалось загрузить категории');
        }

        const data = await response.json();
        
        // ✅ Добавляем русские переводы к каждой категории
        const categoriesWithTranslations = data.categories.map((cat: Category) => ({
          ...cat,
          name_ru: translateCategory(cat.name)
        }));
        
        // Сортируем по sort_order
        setCategories(
          categoriesWithTranslations.sort((a: Category, b: Category) => 
            a.sort_order - b.sort_order
          )
        );
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Неизвестная ошибка');
      } finally {
        setLoading(false);
      }
    };

    fetchCategories();
  }, []);

  return { categories, loading, error };
};
```

---

### 3. Используем в форме создания продукта

```tsx
// components/ProductForm.tsx
function ProductForm({ productId, onSuccess }: Props) {
  const { categories, loading: categoriesLoading } = useCategories();
  
  const [formData, setFormData] = useState({
    name_en: '',
    category_id: '',
    unit: 'kilogram',
    // ...
  });

  return (
    <form onSubmit={handleSubmit}>
      {/* ... другие поля ... */}

      <div>
        <label>Категория *</label>
        <select
          value={formData.category_id}
          onChange={e => setFormData({...formData, category_id: e.target.value})}
          required
          disabled={categoriesLoading}
        >
          <option value="">Выберите категорию...</option>
          {categories.map(cat => (
            <option key={cat.id} value={cat.id}>
              {cat.name_ru || cat.name}  {/* ← Показываем русское название */}
            </option>
          ))}
        </select>
        {categoriesLoading && <small>Загрузка категорий...</small>}
      </div>

      {/* ... остальные поля ... */}
    </form>
  );
}
```

---

### 4. Используем в таблице продуктов

```tsx
// components/ProductList.tsx
import { translateCategory } from '@/utils/categoryTranslations';

function ProductList() {
  const [products, setProducts] = useState<Product[]>([]);
  const [categories, setCategories] = useState<Category[]>([]);

  // Загружаем категории
  useEffect(() => {
    const fetchCategories = async () => {
      const token = localStorage.getItem('admin_token');
      const response = await fetch(`${API_URL}/api/admin/categories`, {
        headers: { 'Authorization': `Bearer ${token}` }
      });
      const data = await response.json();
      
      // Добавляем переводы
      const categoriesWithRu = data.categories.map((cat: Category) => ({
        ...cat,
        name_ru: translateCategory(cat.name)
      }));
      
      setCategories(categoriesWithRu);
    };
    
    fetchCategories();
  }, []);

  // Функция для получения русского названия по ID
  const getCategoryNameRu = (categoryId: string): string => {
    const category = categories.find(cat => cat.id === categoryId);
    return category?.name_ru || category?.name || 'Неизвестно';
  };

  return (
    <table>
      <thead>
        <tr>
          <th>Изображение</th>
          <th>Название</th>
          <th>Единица</th>
          <th>Категория</th>
          <th>Действия</th>
        </tr>
      </thead>
      <tbody>
        {products.map(product => (
          <tr key={product.id}>
            <td>
              {product.image_url ? (
                <img src={product.image_url} width="50" alt={product.name_en} />
              ) : (
                <span>Нет фото</span>
              )}
            </td>
            <td>{product.name_en}</td>
            <td>{product.unit}</td>
            <td>{getCategoryNameRu(product.category_id)}</td>  {/* ← Русское название */}
            <td>
              <button onClick={() => handleEdit(product)}>Редактировать</button>
              <button onClick={() => handleDelete(product.id)}>Удалить</button>
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
```

---

### 5. Альтернатива: Используем React Context

Если категории используются в многих местах, создайте контекст:

```tsx
// contexts/CategoriesContext.tsx
import { createContext, useContext, useEffect, useState } from 'react';
import { translateCategory } from '@/utils/categoryTranslations';

interface Category {
  id: string;
  name: string;
  name_ru: string;
  sort_order: number;
}

interface CategoriesContextType {
  categories: Category[];
  loading: boolean;
  getCategoryById: (id: string) => Category | undefined;
  getCategoryNameRu: (id: string) => string;
}

const CategoriesContext = createContext<CategoriesContextType | undefined>(undefined);

export const CategoriesProvider = ({ children }: { children: React.ReactNode }) => {
  const [categories, setCategories] = useState<Category[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const fetchCategories = async () => {
      try {
        const token = localStorage.getItem('admin_token');
        const response = await fetch(
          'https://ministerial-yetta-fodi999-c58d8823.koyeb.app/api/admin/categories',
          {
            headers: { 'Authorization': `Bearer ${token}` }
          }
        );
        
        const data = await response.json();
        
        const categoriesWithRu = data.categories.map((cat: any) => ({
          ...cat,
          name_ru: translateCategory(cat.name)
        }));
        
        setCategories(categoriesWithRu.sort((a: Category, b: Category) => 
          a.sort_order - b.sort_order
        ));
      } catch (error) {
        console.error('Failed to load categories:', error);
      } finally {
        setLoading(false);
      }
    };

    fetchCategories();
  }, []);

  const getCategoryById = (id: string) => {
    return categories.find(cat => cat.id === id);
  };

  const getCategoryNameRu = (id: string): string => {
    const category = getCategoryById(id);
    return category?.name_ru || 'Неизвестно';
  };

  return (
    <CategoriesContext.Provider value={{ 
      categories, 
      loading, 
      getCategoryById, 
      getCategoryNameRu 
    }}>
      {children}
    </CategoriesContext.Provider>
  );
};

// Хук для использования контекста
export const useCategories = () => {
  const context = useContext(CategoriesContext);
  if (!context) {
    throw new Error('useCategories must be used within CategoriesProvider');
  }
  return context;
};
```

**Использование:**

```tsx
// app/layout.tsx или pages/_app.tsx
import { CategoriesProvider } from '@/contexts/CategoriesContext';

export default function RootLayout({ children }) {
  return (
    <CategoriesProvider>
      {children}
    </CategoriesProvider>
  );
}

// В любом компоненте
function ProductList() {
  const { categories, getCategoryNameRu, loading } = useCategories();

  return (
    <div>
      {products.map(product => (
        <div key={product.id}>
          <h3>{product.name_en}</h3>
          <p>Категория: {getCategoryNameRu(product.category_id)}</p>
        </div>
      ))}
    </div>
  );
}
```

---

## 📊 Полная таблица переводов

| English | Русский |
|---------|---------|
| Dairy & Eggs | Молочные продукты и яйца |
| Meat & Poultry | Мясо и птица |
| Fish & Seafood | Рыба и морепродукты |
| Vegetables | Овощи |
| Fruits | Фрукты |
| Grains & Pasta | Крупы и макароны |
| Oils & Fats | Масла и жиры |
| Spices & Herbs | Специи и травы |
| Condiments & Sauces | Приправы и соусы |
| Beverages | Напитки |
| Nuts & Seeds | Орехи и семена |
| Legumes | Бобовые |
| Sweets & Baking | Сладости и выпечка |
| Canned & Preserved | Консервы |
| Frozen | Замороженные продукты |

---

## 🎯 Преимущества такого подхода

### ✅ Плюсы перевода на фронтенде:

1. **Гибкость**: Легко добавить новые языки (украинский, польский)
2. **Производительность**: Не требует изменений на backend
3. **Быстрота**: Переводы работают мгновенно без запросов к API
4. **Контроль**: Можно легко править переводы без деплоя backend

### ⚠️ Минусы (и как их решить):

1. **Дублирование переводов** → Используйте библиотеку i18n (next-i18next, react-i18next)
2. **Новые категории** → Fallback на английское название: `categoryTranslations[name] || name`
3. **Рассинхрон** → Если backend добавит категорию, фронт покажет английское до обновления словаря

---

## 🚀 Вариант с i18n библиотекой (рекомендуется для production)

### Установка

```bash
npm install react-i18next i18next
# или
npm install next-i18next  # для Next.js
```

### Конфигурация

```typescript
// i18n/config.ts
import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';

const resources = {
  ru: {
    translation: {
      categories: {
        'Dairy & Eggs': 'Молочные продукты и яйца',
        'Meat & Poultry': 'Мясо и птица',
        'Fish & Seafood': 'Рыба и морепродукты',
        'Vegetables': 'Овощи',
        'Fruits': 'Фрукты',
        'Grains & Pasta': 'Крупы и макароны',
        'Oils & Fats': 'Масла и жиры',
        'Spices & Herbs': 'Специи и травы',
        'Condiments & Sauces': 'Приправы и соусы',
        'Beverages': 'Напитки',
        'Nuts & Seeds': 'Орехи и семена',
        'Legumes': 'Бобовые',
        'Sweets & Baking': 'Сладости и выпечка',
        'Canned & Preserved': 'Консервы',
        'Frozen': 'Замороженные продукты'
      }
    }
  },
  en: {
    translation: {
      categories: {
        'Dairy & Eggs': 'Dairy & Eggs',
        'Meat & Poultry': 'Meat & Poultry',
        // ... и т.д.
      }
    }
  }
};

i18n
  .use(initReactI18next)
  .init({
    resources,
    lng: 'ru',
    fallbackLng: 'en',
    interpolation: {
      escapeValue: false
    }
  });

export default i18n;
```

### Использование

```tsx
import { useTranslation } from 'react-i18next';

function ProductForm() {
  const { t } = useTranslation();
  const { categories } = useCategories();

  return (
    <select>
      <option value="">Выберите категорию...</option>
      {categories.map(cat => (
        <option key={cat.id} value={cat.id}>
          {t(`categories.${cat.name}`)}
        </option>
      ))}
    </select>
  );
}
```

---

## 📝 Итого

**Рекомендация для вашего проекта:**

1. **MVP / Прототип** → Используйте простой словарь `categoryTranslations`
2. **Production** → Переходите на `react-i18next` или `next-i18next`

**Текущее решение (словарь) работает отлично для:**
- ✅ Быстрого старта
- ✅ Поддержки русского языка
- ✅ Простой кодовой базы

**Переход на i18n нужен когда:**
- 🌍 Добавляете 3+ языка (pl, uk, en, ru)
- 📦 Переводов становится >100 строк
- 🔄 Нужна динамическая смена языка

**Сейчас используйте простой словарь! Он уже готов и работает! 🚀**
