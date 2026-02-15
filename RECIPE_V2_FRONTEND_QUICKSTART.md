# 🚀 Recipe V2 Frontend - Быстрый старт (10 минут)

## Шаг 1: Создать Next.js проект (2 минуты)

```bash
cd /Users/dmitrijfomin/Desktop/assistant
npx create-next-app@latest frontend --typescript --tailwind --app
# ✅ Use App Router? Yes
# ✅ Use Tailwind CSS? Yes
# ✅ Use `src/` directory? No
# ✅ Use TypeScript? Yes
cd frontend
```

## Шаг 2: Установить зависимости (1 минута)

```bash
npm install axios react-hook-form zod @hookform/resolvers lucide-react clsx tailwind-merge
```

## Шаг 3: Создать .env.local

```bash
cat > .env.local << 'EOF'
NEXT_PUBLIC_API_URL=https://ministerial-yetta-fodi999-c58d8823.koyeb.app
EOF
```

## Шаг 4: Скопировать код (5 минут)

### Создать структуру папок

```bash
mkdir -p app/recipes/{create,[id]/edit}
mkdir -p components/{recipes,ui}
mkdir -p services hooks types lib
```

### 1. Типы (`types/recipe.ts`)

```typescript
export type RecipeLanguage = 'ru' | 'en' | 'pl' | 'uk';
export type RecipeStatus = 'draft' | 'published';

export interface RecipeIngredient {
  catalog_ingredient_id: string;
  quantity: number;
  unit: string;
}

export interface Recipe {
  id: string;
  name: string;
  instructions: string;
  language: RecipeLanguage;
  servings: number;
  status: RecipeStatus;
  created_at: string;
  updated_at: string;
  ingredients: RecipeIngredient[];
  translations?: any[];
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
  name_ru: string;
  name_pl: string;
  name_uk: string;
}
```

### 2. Утилиты (`lib/utils.ts`)

```typescript
import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
```

### 3. API Client (`services/api.ts`)

```typescript
import axios from 'axios';

const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8000';

export const api = axios.create({
  baseURL: API_URL,
  headers: { 'Content-Type': 'application/json' },
});

// Добавить JWT токен из localStorage
if (typeof window !== 'undefined') {
  api.interceptors.request.use((config) => {
    const token = localStorage.getItem('auth_token');
    if (token) config.headers.Authorization = `Bearer ${token}`;
    return config;
  });
}
```

### 4. Recipe Service (`services/recipeService.ts`)

```typescript
import { api } from './api';
import { Recipe, CreateRecipeRequest } from '@/types/recipe';

export const recipeService = {
  async create(data: CreateRecipeRequest): Promise<Recipe> {
    const res = await api.post('/api/recipes/v2', data);
    return res.data;
  },
  
  async list() {
    const res = await api.get('/api/recipes/v2');
    return res.data;
  },
  
  async getById(id: string): Promise<Recipe> {
    const res = await api.get(`/api/recipes/v2/${id}`);
    return res.data;
  },
};
```

### 5. UI Components

**`components/ui/Button.tsx`**

```typescript
import { ButtonHTMLAttributes } from 'react';

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary';
}

export function Button({ variant = 'primary', className = '', ...props }: ButtonProps) {
  const baseStyle = 'px-4 py-2 rounded-md font-medium transition-colors disabled:opacity-50';
  const variantStyle = variant === 'primary' 
    ? 'bg-blue-600 text-white hover:bg-blue-700'
    : 'bg-gray-200 text-gray-800 hover:bg-gray-300';
  
  return <button className={`${baseStyle} ${variantStyle} ${className}`} {...props} />;
}
```

**`components/ui/Input.tsx`**

```typescript
import { InputHTMLAttributes, forwardRef } from 'react';

export const Input = forwardRef<HTMLInputElement, InputHTMLAttributes<HTMLInputElement>>(
  (props, ref) => (
    <input
      ref={ref}
      className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
      {...props}
    />
  )
);
Input.displayName = 'Input';
```

**`components/ui/Textarea.tsx`**

```typescript
import { TextareaHTMLAttributes, forwardRef } from 'react';

export const Textarea = forwardRef<HTMLTextAreaElement, TextareaHTMLAttributes<HTMLTextAreaElement>>(
  (props, ref) => (
    <textarea
      ref={ref}
      className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
      {...props}
    />
  )
);
Textarea.displayName = 'Textarea';
```

**`components/ui/Select.tsx`**

```typescript
import { SelectHTMLAttributes, forwardRef } from 'react';

export const Select = forwardRef<HTMLSelectElement, SelectHTMLAttributes<HTMLSelectElement>>(
  ({ children, ...props }, ref) => (
    <select
      ref={ref}
      className="w-full px-3 py-2 border border-gray-300 rounded-md bg-white focus:outline-none focus:ring-2 focus:ring-blue-500"
      {...props}
    >
      {children}
    </select>
  )
);
Select.displayName = 'Select';
```

### 6. Recipe Form (`components/recipes/RecipeForm.tsx`)

```typescript
'use client';

import { useState } from 'react';
import { useRouter } from 'next/navigation';
import { useForm } from 'react-hook-form';
import { recipeService } from '@/services/recipeService';
import { CreateRecipeRequest } from '@/types/recipe';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { Textarea } from '@/components/ui/Textarea';
import { Select } from '@/components/ui/Select';

interface FormData {
  name: string;
  instructions: string;
  language: 'ru' | 'en' | 'pl' | 'uk';
  servings: number;
}

export function RecipeForm() {
  const router = useRouter();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const { register, handleSubmit } = useForm<FormData>({
    defaultValues: { language: 'ru', servings: 4 }
  });

  const onSubmit = async (data: FormData) => {
    try {
      setLoading(true);
      setError('');
      
      const request: CreateRecipeRequest = {
        ...data,
        ingredients: [
          // Пример: хардкод для тестирования
          { catalog_ingredient_id: '8238ad5e-f9d2-4edd-8690-9ba68e07a3f8', quantity: 0.5, unit: 'kg' }
        ]
      };

      const recipe = await recipeService.create(request);
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

      <div>
        <label className="block text-sm font-medium mb-2">Название</label>
        <Input {...register('name', { required: true })} placeholder="Борщ украинский" />
      </div>

      <div>
        <label className="block text-sm font-medium mb-2">Язык</label>
        <Select {...register('language')}>
          <option value="ru">Русский (RU)</option>
          <option value="en">English (EN)</option>
          <option value="pl">Polski (PL)</option>
          <option value="uk">Українська (UK)</option>
        </Select>
        <p className="text-sm text-gray-500 mt-1">
          🌐 Автоматически переведется на остальные языки
        </p>
      </div>

      <div>
        <label className="block text-sm font-medium mb-2">Порции</label>
        <Input type="number" {...register('servings', { valueAsNumber: true })} min={1} />
      </div>

      <div>
        <label className="block text-sm font-medium mb-2">Инструкции</label>
        <Textarea
          {...register('instructions', { required: true })}
          rows={8}
          placeholder="1. Сварить свеклу...&#10;2. Добавить мясо..."
        />
        <p className="text-sm text-gray-500 mt-1">
          🌐 Инструкции также будут переведены
        </p>
      </div>

      <Button type="submit" disabled={loading}>
        {loading ? 'Создание...' : 'Создать рецепт'}
      </Button>
    </form>
  );
}
```

### 7. Страница создания (`app/recipes/create/page.tsx`)

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

### 8. Главная страница (`app/page.tsx`)

```typescript
import Link from 'next/link';

export default function Home() {
  return (
    <div className="min-h-screen flex items-center justify-center">
      <div className="text-center">
        <h1 className="text-4xl font-bold mb-8">Recipe Manager V2</h1>
        <Link
          href="/recipes/create"
          className="bg-blue-600 text-white px-6 py-3 rounded-md font-medium hover:bg-blue-700"
        >
          Создать рецепт
        </Link>
      </div>
    </div>
  );
}
```

## Шаг 5: Настроить аутентификацию (1 минута)

Временно (для тестирования) получите токен:

```bash
# Войти под существующим пользователем
curl -X POST https://ministerial-yetta-fodi999-c58d8823.koyeb.app/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"dmitrijfomin@gmail.com","password":"test123"}' | jq -r .access_token
```

Вставьте токен в консоль браузера:

```javascript
localStorage.setItem('auth_token', 'YOUR_JWT_TOKEN_HERE');
```

## Шаг 6: Запустить (1 минута)

```bash
npm run dev
```

Откройте: **http://localhost:3000/recipes/create**

## ✅ Тестирование

1. Откройте http://localhost:3000/recipes/create
2. Заполните форму:
   - Название: "Борщ украинский"
   - Язык: Русский (RU)
   - Порции: 6
   - Инструкции: "Сварить свеклу, морковь и капусту. Добавить мясо и картофель. Варить 2 часа."
3. Нажмите "Создать рецепт"
4. Backend автоматически переведет на EN, PL, UK! 🌐

## 🎯 Что происходит при создании?

```
Frontend → POST /api/recipes/v2
         ↓
Backend:
  1. ✅ Создает рецепт в БД
  2. ✅ Groq AI переводит название (ru→en,pl,uk)
  3. ✅ Groq AI переводит инструкции (ru→en,pl,uk)
  4. ✅ Сохраняет переводы в recipe_translations
  5. ✅ Возвращает рецепт с ID
         ↓
Frontend → Редирект на /recipes/:id
```

## 📚 Полная документация

- **Детали API**: `RECIPE_V2_FRONTEND_GUIDE.md`
- **UI Компоненты**: `RECIPE_V2_UI_COMPONENTS.md`
- **Backend**: `RECIPE_SYSTEM_IMPLEMENTATION.md`

---

**Готово за 10 минут!** 🚀✨
