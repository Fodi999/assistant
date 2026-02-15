# 🍳 Recipe V2 Frontend Implementation Guide

## Обзор

Полное руководство по созданию фронтенда для Recipe V2 с автоматическими переводами.

## 📋 API Endpoints (Backend)

```
POST   /api/recipes/v2                    - Создать рецепт с авто-переводами
GET    /api/recipes/v2                    - Список рецептов (пагинация)
GET    /api/recipes/v2/:id                - Получить рецепт с переводами
POST   /api/recipes/v2/:id/publish        - Опубликовать рецепт
DELETE /api/recipes/v2/:id                - Удалить рецепт (soft delete)
```

## 🏗️ Структура проекта

```
frontend/
├── src/
│   ├── app/
│   │   ├── recipes/
│   │   │   ├── page.tsx                 # Список рецептов
│   │   │   ├── create/
│   │   │   │   └── page.tsx             # Создание рецепта
│   │   │   └── [id]/
│   │   │       ├── page.tsx             # Просмотр рецепта
│   │   │       └── edit/
│   │   │           └── page.tsx         # Редактирование
│   │   ├── layout.tsx
│   │   └── page.tsx
│   │
│   ├── components/
│   │   ├── recipes/
│   │   │   ├── RecipeForm.tsx           # 🔑 Форма создания/редактирования
│   │   │   ├── RecipeList.tsx           # Список рецептов
│   │   │   ├── RecipeCard.tsx           # Карточка рецепта
│   │   │   ├── RecipeView.tsx           # Просмотр рецепта
│   │   │   ├── IngredientSelector.tsx   # Выбор ингредиентов из каталога
│   │   │   └── TranslationIndicator.tsx # Индикатор переводов
│   │   │
│   │   └── ui/
│   │       ├── Button.tsx
│   │       ├── Input.tsx
│   │       ├── Select.tsx
│   │       ├── Textarea.tsx
│   │       └── Badge.tsx
│   │
│   ├── services/
│   │   ├── recipeService.ts             # 🔑 API для рецептов
│   │   ├── catalogService.ts            # API для каталога ингредиентов
│   │   └── api.ts                       # Base HTTP client
│   │
│   ├── hooks/
│   │   ├── useRecipes.ts                # Загрузка списка рецептов
│   │   ├── useRecipe.ts                 # Загрузка одного рецепта
│   │   ├── useCatalogIngredients.ts     # Загрузка каталога
│   │   └── useAuth.ts                   # Аутентификация
│   │
│   ├── types/
│   │   └── recipe.ts                    # TypeScript типы для рецептов
│   │
│   └── lib/
│       └── utils.ts                     # Утилиты
│
├── public/
├── .env.local
├── next.config.js
├── package.json
├── tsconfig.json
└── tailwind.config.js
```

## 📦 Установка и настройка

### 1. Создание Next.js проекта

```bash
cd /Users/dmitrijfomin/Desktop/assistant
npx create-next-app@latest frontend --typescript --tailwind --app --no-src-dir
cd frontend
```

### 2. Установка зависимостей

```bash
npm install axios react-hook-form zod @hookform/resolvers lucide-react
npm install -D @types/node
```

### 3. Настройка переменных окружения

Создайте `.env.local`:

```env
NEXT_PUBLIC_API_URL=https://ministerial-yetta-fodi999-c58d8823.koyeb.app
# Для локальной разработки:
# NEXT_PUBLIC_API_URL=http://localhost:8000
```

## 📝 TypeScript типы

### `types/recipe.ts`

```typescript
export type RecipeLanguage = 'ru' | 'en' | 'pl' | 'uk';
export type RecipeStatus = 'draft' | 'published';

export interface RecipeIngredient {
  catalog_ingredient_id: string;
  quantity: number;
  unit: string;
}

export interface RecipeTranslation {
  id: string;
  recipe_id: string;
  language: RecipeLanguage;
  name: string;
  instructions: string;
  translated_at: string;
  translated_by: string;
}

export interface Recipe {
  id: string;
  tenant_id: string;
  name: string;
  instructions: string;
  language: RecipeLanguage;
  servings: number;
  status: RecipeStatus;
  created_at: string;
  updated_at: string;
  ingredients: RecipeIngredient[];
  translations?: RecipeTranslation[];
}

export interface CreateRecipeRequest {
  name: string;
  instructions: string;
  language: RecipeLanguage;
  servings: number;
  ingredients: RecipeIngredient[];
}

export interface CatalogIngredient {
  id: string;
  name_en: string;
  name_pl: string;
  name_ru: string;
  name_uk: string;
  category_id: string;
  unit: string;
}
```

## 🔧 API Services

### `services/api.ts` (Base HTTP Client)

```typescript
import axios, { AxiosError, AxiosInstance } from 'axios';

const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8000';

class ApiClient {
  private client: AxiosInstance;

  constructor() {
    this.client = axios.create({
      baseURL: API_URL,
      headers: {
        'Content-Type': 'application/json',
      },
    });

    // Interceptor для добавления JWT токена
    this.client.interceptors.request.use((config) => {
      const token = localStorage.getItem('auth_token');
      if (token) {
        config.headers.Authorization = `Bearer ${token}`;
      }
      return config;
    });

    // Interceptor для обработки ошибок
    this.client.interceptors.response.use(
      (response) => response,
      (error: AxiosError) => {
        if (error.response?.status === 401) {
          // Редирект на логин
          localStorage.removeItem('auth_token');
          window.location.href = '/login';
        }
        return Promise.reject(error);
      }
    );
  }

  get instance() {
    return this.client;
  }
}

export const api = new ApiClient().instance;
```

### `services/recipeService.ts`

```typescript
import { api } from './api';
import { Recipe, CreateRecipeRequest } from '@/types/recipe';

export interface RecipeListParams {
  status?: 'draft' | 'published';
  limit?: number;
  offset?: number;
}

export interface RecipeListResponse {
  recipes: Recipe[];
  total: number;
  limit: number;
  offset: number;
}

export const recipeService = {
  // 🔑 Создать рецепт с автоматическими переводами
  async create(data: CreateRecipeRequest): Promise<Recipe> {
    const response = await api.post<Recipe>('/api/recipes/v2', data);
    return response.data;
  },

  // Получить список рецептов
  async list(params: RecipeListParams = {}): Promise<RecipeListResponse> {
    const response = await api.get<RecipeListResponse>('/api/recipes/v2', {
      params: {
        status: params.status,
        limit: params.limit || 20,
        offset: params.offset || 0,
      },
    });
    return response.data;
  },

  // Получить рецепт по ID (с переводами)
  async getById(id: string): Promise<Recipe> {
    const response = await api.get<Recipe>(`/api/recipes/v2/${id}`);
    return response.data;
  },

  // Опубликовать рецепт
  async publish(id: string): Promise<Recipe> {
    const response = await api.post<Recipe>(`/api/recipes/v2/${id}/publish`);
    return response.data;
  },

  // Удалить рецепт (soft delete)
  async delete(id: string): Promise<void> {
    await api.delete(`/api/recipes/v2/${id}`);
  },
};
```

### `services/catalogService.ts`

```typescript
import { api } from './api';
import { CatalogIngredient } from '@/types/recipe';

export interface CatalogSearchParams {
  query?: string;
  limit?: number;
  offset?: number;
}

export interface CatalogSearchResponse {
  ingredients: CatalogIngredient[];
  total: number;
}

export const catalogService = {
  // Поиск ингредиентов в каталоге
  async search(params: CatalogSearchParams = {}): Promise<CatalogSearchResponse> {
    const response = await api.get<CatalogSearchResponse>('/api/admin/catalog', {
      params: {
        query: params.query || '',
        limit: params.limit || 50,
        offset: params.offset || 0,
      },
    });
    return response.data;
  },

  // Получить ингредиент по ID
  async getById(id: string): Promise<CatalogIngredient> {
    const response = await api.get<CatalogIngredient>(`/api/admin/catalog/${id}`);
    return response.data;
  },
};
```

## 🎣 React Hooks

### `hooks/useRecipes.ts`

```typescript
'use client';

import { useState, useEffect } from 'react';
import { recipeService, RecipeListParams } from '@/services/recipeService';
import { Recipe } from '@/types/recipe';

export function useRecipes(params: RecipeListParams = {}) {
  const [recipes, setRecipes] = useState<Recipe[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchRecipes = async () => {
      try {
        setLoading(true);
        const response = await recipeService.list(params);
        setRecipes(response.recipes);
        setTotal(response.total);
        setError(null);
      } catch (err: any) {
        setError(err.response?.data?.message || 'Failed to load recipes');
      } finally {
        setLoading(false);
      }
    };

    fetchRecipes();
  }, [params.status, params.limit, params.offset]);

  const refresh = async () => {
    setLoading(true);
    try {
      const response = await recipeService.list(params);
      setRecipes(response.recipes);
      setTotal(response.total);
      setError(null);
    } catch (err: any) {
      setError(err.response?.data?.message || 'Failed to refresh recipes');
    } finally {
      setLoading(false);
    }
  };

  return { recipes, total, loading, error, refresh };
}
```

### `hooks/useRecipe.ts`

```typescript
'use client';

import { useState, useEffect } from 'react';
import { recipeService } from '@/services/recipeService';
import { Recipe } from '@/types/recipe';

export function useRecipe(id: string | null) {
  const [recipe, setRecipe] = useState<Recipe | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!id) {
      setLoading(false);
      return;
    }

    const fetchRecipe = async () => {
      try {
        setLoading(true);
        const data = await recipeService.getById(id);
        setRecipe(data);
        setError(null);
      } catch (err: any) {
        setError(err.response?.data?.message || 'Failed to load recipe');
      } finally {
        setLoading(false);
      }
    };

    fetchRecipe();
  }, [id]);

  return { recipe, loading, error };
}
```

### `hooks/useCatalogIngredients.ts`

```typescript
'use client';

import { useState, useEffect } from 'react';
import { catalogService } from '@/services/catalogService';
import { CatalogIngredient } from '@/types/recipe';

export function useCatalogIngredients(query: string = '') {
  const [ingredients, setIngredients] = useState<CatalogIngredient[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchIngredients = async () => {
      try {
        setLoading(true);
        const response = await catalogService.search({ query, limit: 100 });
        setIngredients(response.ingredients);
        setError(null);
      } catch (err: any) {
        setError(err.response?.data?.message || 'Failed to load ingredients');
      } finally {
        setLoading(false);
      }
    };

    fetchIngredients();
  }, [query]);

  return { ingredients, loading, error };
}
```

## 🎨 Главный компонент - RecipeForm

### `components/recipes/RecipeForm.tsx`

```typescript
'use client';

import { useState } from 'react';
import { useRouter } from 'next/navigation';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { z } from 'zod';
import { recipeService } from '@/services/recipeService';
import { CreateRecipeRequest, RecipeLanguage } from '@/types/recipe';
import { IngredientSelector } from './IngredientSelector';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { Textarea } from '@/components/ui/Textarea';
import { Select } from '@/components/ui/Select';

const recipeSchema = z.object({
  name: z.string().min(3, 'Название должно быть минимум 3 символа'),
  instructions: z.string().min(10, 'Инструкции должны быть минимум 10 символов'),
  language: z.enum(['ru', 'en', 'pl', 'uk']),
  servings: z.number().min(1, 'Минимум 1 порция').max(100, 'Максимум 100 порций'),
});

type RecipeFormData = z.infer<typeof recipeSchema>;

export function RecipeForm() {
  const router = useRouter();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [ingredients, setIngredients] = useState<Array<{
    catalog_ingredient_id: string;
    quantity: number;
    unit: string;
  }>>([]);

  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<RecipeFormData>({
    resolver: zodResolver(recipeSchema),
    defaultValues: {
      language: 'ru',
      servings: 4,
    },
  });

  const onSubmit = async (data: RecipeFormData) => {
    if (ingredients.length === 0) {
      setError('Добавьте хотя бы один ингредиент');
      return;
    }

    try {
      setLoading(true);
      setError(null);

      const request: CreateRecipeRequest = {
        ...data,
        ingredients,
      };

      const recipe = await recipeService.create(request);
      
      // Успех! Переходим к просмотру рецепта
      router.push(`/recipes/${recipe.id}`);
    } catch (err: any) {
      setError(err.response?.data?.message || 'Ошибка создания рецепта');
    } finally {
      setLoading(false);
    }
  };

  return (
    <form onSubmit={handleSubmit(onSubmit)} className="max-w-2xl mx-auto space-y-6">
      <h1 className="text-3xl font-bold">Создать рецепт</h1>

      {error && (
        <div className="bg-red-50 border border-red-200 text-red-800 px-4 py-3 rounded">
          {error}
        </div>
      )}

      {/* Название рецепта */}
      <div>
        <label className="block text-sm font-medium mb-2">
          Название рецепта
        </label>
        <Input
          {...register('name')}
          placeholder="Борщ украинский"
          error={errors.name?.message}
        />
      </div>

      {/* Язык */}
      <div>
        <label className="block text-sm font-medium mb-2">
          Язык оригинала
        </label>
        <Select {...register('language')}>
          <option value="ru">Русский (RU)</option>
          <option value="en">English (EN)</option>
          <option value="pl">Polski (PL)</option>
          <option value="uk">Українська (UK)</option>
        </Select>
        <p className="text-sm text-gray-500 mt-1">
          🌐 Автоматически будет переведен на остальные языки
        </p>
      </div>

      {/* Количество порций */}
      <div>
        <label className="block text-sm font-medium mb-2">
          Количество порций
        </label>
        <Input
          type="number"
          {...register('servings', { valueAsNumber: true })}
          min={1}
          max={100}
          error={errors.servings?.message}
        />
      </div>

      {/* Инструкции */}
      <div>
        <label className="block text-sm font-medium mb-2">
          Инструкции по приготовлению
        </label>
        <Textarea
          {...register('instructions')}
          rows={8}
          placeholder="1. Сварить свеклу, морковь и капусту.&#10;2. Добавить мясо и картофель.&#10;3. Варить 2 часа."
          error={errors.instructions?.message}
        />
        <p className="text-sm text-gray-500 mt-1">
          🌐 Инструкции также будут автоматически переведены
        </p>
      </div>

      {/* Ингредиенты */}
      <div>
        <label className="block text-sm font-medium mb-2">
          Ингредиенты
        </label>
        <IngredientSelector
          ingredients={ingredients}
          onChange={setIngredients}
        />
      </div>

      {/* Кнопки */}
      <div className="flex gap-4">
        <Button
          type="submit"
          disabled={loading}
          variant="primary"
        >
          {loading ? 'Создание...' : 'Создать рецепт'}
        </Button>
        <Button
          type="button"
          variant="secondary"
          onClick={() => router.back()}
        >
          Отмена
        </Button>
      </div>
    </form>
  );
}
```

## 🔍 Компонент выбора ингредиентов

### `components/recipes/IngredientSelector.tsx`

```typescript
'use client';

import { useState } from 'react';
import { useCatalogIngredients } from '@/hooks/useCatalogIngredients';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { Select } from '@/components/ui/Select';
import { X } from 'lucide-react';

interface IngredientItem {
  catalog_ingredient_id: string;
  quantity: number;
  unit: string;
}

interface IngredientSelectorProps {
  ingredients: IngredientItem[];
  onChange: (ingredients: IngredientItem[]) => void;
}

export function IngredientSelector({ ingredients, onChange }: IngredientSelectorProps) {
  const [query, setQuery] = useState('');
  const [selectedId, setSelectedId] = useState('');
  const [quantity, setQuantity] = useState('1');
  const [unit, setUnit] = useState('kg');

  const { ingredients: catalog, loading } = useCatalogIngredients(query);

  const handleAdd = () => {
    if (!selectedId || !quantity) return;

    const newIngredient: IngredientItem = {
      catalog_ingredient_id: selectedId,
      quantity: parseFloat(quantity),
      unit,
    };

    onChange([...ingredients, newIngredient]);

    // Reset form
    setSelectedId('');
    setQuantity('1');
    setUnit('kg');
    setQuery('');
  };

  const handleRemove = (index: number) => {
    onChange(ingredients.filter((_, i) => i !== index));
  };

  const getIngredientName = (id: string) => {
    const ingredient = catalog.find((i) => i.id === id);
    return ingredient?.name_ru || ingredient?.name_en || 'Unknown';
  };

  return (
    <div className="space-y-4">
      {/* Форма добавления ингредиента */}
      <div className="grid grid-cols-12 gap-2">
        <div className="col-span-5">
          <Input
            type="text"
            placeholder="Поиск ингредиента..."
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          {query && catalog.length > 0 && (
            <div className="absolute z-10 mt-1 w-full bg-white border rounded-md shadow-lg max-h-48 overflow-y-auto">
              {catalog.map((item) => (
                <button
                  key={item.id}
                  type="button"
                  className="w-full text-left px-3 py-2 hover:bg-gray-100"
                  onClick={() => {
                    setSelectedId(item.id);
                    setQuery(item.name_ru || item.name_en);
                  }}
                >
                  {item.name_ru || item.name_en}
                </button>
              ))}
            </div>
          )}
        </div>

        <div className="col-span-2">
          <Input
            type="number"
            placeholder="Кол-во"
            value={quantity}
            onChange={(e) => setQuantity(e.target.value)}
            step="0.1"
            min="0"
          />
        </div>

        <div className="col-span-3">
          <Select value={unit} onChange={(e) => setUnit(e.target.value)}>
            <option value="kg">Килограмм</option>
            <option value="g">Грамм</option>
            <option value="l">Литр</option>
            <option value="ml">Миллилитр</option>
            <option value="piece">Штук</option>
          </Select>
        </div>

        <div className="col-span-2">
          <Button
            type="button"
            onClick={handleAdd}
            disabled={!selectedId || !quantity}
            variant="primary"
            className="w-full"
          >
            Добавить
          </Button>
        </div>
      </div>

      {/* Список добавленных ингредиентов */}
      {ingredients.length > 0 && (
        <div className="border rounded-md divide-y">
          {ingredients.map((item, index) => (
            <div key={index} className="flex items-center justify-between px-4 py-3">
              <div>
                <span className="font-medium">{getIngredientName(item.catalog_ingredient_id)}</span>
                <span className="text-gray-500 ml-2">
                  {item.quantity} {item.unit}
                </span>
              </div>
              <button
                type="button"
                onClick={() => handleRemove(index)}
                className="text-red-500 hover:text-red-700"
              >
                <X className="w-5 h-5" />
              </button>
            </div>
          ))}
        </div>
      )}

      {ingredients.length === 0 && (
        <p className="text-sm text-gray-500 text-center py-4">
          Ингредиенты не добавлены
        </p>
      )}
    </div>
  );
}
```

## 📄 Страница создания рецепта

### `app/recipes/create/page.tsx`

```typescript
import { RecipeForm } from '@/components/recipes/RecipeForm';

export default function CreateRecipePage() {
  return (
    <div className="container mx-auto px-4 py-8">
      <RecipeForm />
    </div>
  );
}
```

## 🚀 Запуск

```bash
cd frontend
npm run dev
```

Откройте http://localhost:3000/recipes/create

## ✅ Что происходит при создании рецепта

1. **Пользователь заполняет форму**:
   - Название: "Борщ украинский"
   - Язык: ru (русский)
   - Инструкции: "Сварить свеклу..."
   - Ингредиенты: Beets (0.5 kg)
   - Порции: 6

2. **Frontend отправляет POST /api/recipes/v2**:
```json
{
  "name": "Борщ украинский",
  "instructions": "Сварить свеклу, морковь и капусту...",
  "language": "ru",
  "servings": 6,
  "ingredients": [{
    "catalog_ingredient_id": "uuid",
    "quantity": 0.5,
    "unit": "kg"
  }]
}
```

3. **Backend автоматически**:
   - ✅ Создает рецепт в БД
   - ✅ Переводит название на EN, PL, UK через Groq AI
   - ✅ Переводит инструкции на EN, PL, UK
   - ✅ Сохраняет переводы в `recipe_translations`
   - ✅ Возвращает рецепт с ID

4. **Frontend перенаправляет** на страницу просмотра рецепта

## 🎯 Следующие шаги

1. ✅ Создать базовые UI компоненты (Button, Input, Select, Textarea)
2. ✅ Добавить просмотр рецепта с переводами
3. ✅ Добавить список рецептов с фильтрами
4. ✅ Добавить редактирование рецепта
5. ✅ Добавить публикацию/удаление

---

**Готово для production!** 🚀
