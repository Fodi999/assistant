#!/bin/bash
# 🚀 Recipe V2 Frontend - Автоматическая установка

set -e

echo "🍳 Recipe V2 Frontend Setup"
echo "============================="
echo ""

# Цвета для вывода
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Шаг 1: Создать Next.js проект
echo -e "${BLUE}📦 Шаг 1: Создание Next.js проекта...${NC}"
cd /Users/dmitrijfomin/Desktop/assistant
npx create-next-app@latest frontend --typescript --tailwind --app --yes

cd frontend

# Шаг 2: Установить зависимости
echo -e "${BLUE}📦 Шаг 2: Установка зависимостей...${NC}"
npm install axios react-hook-form zod @hookform/resolvers lucide-react clsx tailwind-merge

# Шаг 3: Создать .env.local
echo -e "${BLUE}⚙️  Шаг 3: Создание .env.local...${NC}"
cat > .env.local << 'EOF'
NEXT_PUBLIC_API_URL=https://ministerial-yetta-fodi999-c58d8823.koyeb.app
EOF

# Шаг 4: Создать структуру папок
echo -e "${BLUE}📁 Шаг 4: Создание структуры...${NC}"
mkdir -p app/recipes/create
mkdir -p app/recipes/\[id\]
mkdir -p components/recipes
mkdir -p components/ui
mkdir -p services
mkdir -p hooks
mkdir -p types
mkdir -p lib

# Шаг 5: Создать файлы
echo -e "${BLUE}📝 Шаг 5: Создание файлов...${NC}"

# types/recipe.ts
cat > types/recipe.ts << 'EOF'
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
EOF

# lib/utils.ts
cat > lib/utils.ts << 'EOF'
import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
EOF

# services/api.ts
cat > services/api.ts << 'EOF'
import axios from 'axios';

const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8000';

export const api = axios.create({
  baseURL: API_URL,
  headers: { 'Content-Type': 'application/json' },
});

if (typeof window !== 'undefined') {
  api.interceptors.request.use((config) => {
    const token = localStorage.getItem('auth_token');
    if (token) config.headers.Authorization = `Bearer ${token}`;
    return config;
  });
}
EOF

# services/recipeService.ts
cat > services/recipeService.ts << 'EOF'
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
EOF

# components/ui/Button.tsx
cat > components/ui/Button.tsx << 'EOF'
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
EOF

# components/ui/Input.tsx
cat > components/ui/Input.tsx << 'EOF'
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
EOF

# components/ui/Textarea.tsx
cat > components/ui/Textarea.tsx << 'EOF'
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
EOF

# components/ui/Select.tsx
cat > components/ui/Select.tsx << 'EOF'
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
EOF

# components/recipes/RecipeForm.tsx
cat > components/recipes/RecipeForm.tsx << 'EOF'
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
          placeholder="1. Сварить свеклу, морковь и капусту.
2. Добавить мясо и картофель.
3. Варить 2 часа."
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
EOF

# app/recipes/create/page.tsx
cat > app/recipes/create/page.tsx << 'EOF'
import { RecipeForm } from '@/components/recipes/RecipeForm';

export default function CreateRecipePage() {
  return (
    <div className="container mx-auto px-4 py-8">
      <RecipeForm />
    </div>
  );
}
EOF

# app/page.tsx
cat > app/page.tsx << 'EOF'
import Link from 'next/link';

export default function Home() {
  return (
    <div className="min-h-screen flex items-center justify-center">
      <div className="text-center">
        <h1 className="text-4xl font-bold mb-8">Recipe Manager V2</h1>
        <p className="text-gray-600 mb-8">
          🌐 Создавайте рецепты с автоматическим переводом на 4 языка
        </p>
        <Link
          href="/recipes/create"
          className="bg-blue-600 text-white px-6 py-3 rounded-md font-medium hover:bg-blue-700 inline-block"
        >
          Создать рецепт
        </Link>
      </div>
    </div>
  );
}
EOF

echo -e "${GREEN}✅ Установка завершена!${NC}"
echo ""
echo "📚 Следующие шаги:"
echo ""
echo "1. Получить JWT токен:"
echo "   curl -X POST https://ministerial-yetta-fodi999-c58d8823.koyeb.app/api/auth/login \\"
echo "     -H 'Content-Type: application/json' \\"
echo "     -d '{\"email\":\"dmitrijfomin@gmail.com\",\"password\":\"test123\"}' | jq -r .access_token"
echo ""
echo "2. Запустить dev сервер:"
echo "   cd frontend && npm run dev"
echo ""
echo "3. Открыть http://localhost:3000/recipes/create"
echo ""
echo "4. В консоли браузера выполнить:"
echo "   localStorage.setItem('auth_token', 'ВАШ_JWT_ТОКЕН');"
echo ""
echo "5. Создать рецепт и увидеть автоматические переводы! 🌐"
echo ""
