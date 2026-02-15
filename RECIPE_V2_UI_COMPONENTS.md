# 🎨 UI Components для Recipe V2

## Базовые компоненты (Tailwind CSS)

### `components/ui/Button.tsx`

```typescript
import { ButtonHTMLAttributes } from 'react';
import { cn } from '@/lib/utils';

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'danger';
}

export function Button({ variant = 'primary', className, ...props }: ButtonProps) {
  return (
    <button
      className={cn(
        'px-4 py-2 rounded-md font-medium transition-colors',
        variant === 'primary' && 'bg-blue-600 text-white hover:bg-blue-700',
        variant === 'secondary' && 'bg-gray-200 text-gray-800 hover:bg-gray-300',
        variant === 'danger' && 'bg-red-600 text-white hover:bg-red-700',
        'disabled:opacity-50 disabled:cursor-not-allowed',
        className
      )}
      {...props}
    />
  );
}
```

### `components/ui/Input.tsx`

```typescript
import { InputHTMLAttributes, forwardRef } from 'react';
import { cn } from '@/lib/utils';

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  error?: string;
}

export const Input = forwardRef<HTMLInputElement, InputProps>(
  ({ error, className, ...props }, ref) => {
    return (
      <div className="w-full">
        <input
          ref={ref}
          className={cn(
            'w-full px-3 py-2 border rounded-md',
            'focus:outline-none focus:ring-2 focus:ring-blue-500',
            error ? 'border-red-500' : 'border-gray-300',
            className
          )}
          {...props}
        />
        {error && <p className="text-sm text-red-600 mt-1">{error}</p>}
      </div>
    );
  }
);

Input.displayName = 'Input';
```

### `components/ui/Textarea.tsx`

```typescript
import { TextareaHTMLAttributes, forwardRef } from 'react';
import { cn } from '@/lib/utils';

interface TextareaProps extends TextareaHTMLAttributes<HTMLTextAreaElement> {
  error?: string;
}

export const Textarea = forwardRef<HTMLTextAreaElement, TextareaProps>(
  ({ error, className, ...props }, ref) => {
    return (
      <div className="w-full">
        <textarea
          ref={ref}
          className={cn(
            'w-full px-3 py-2 border rounded-md',
            'focus:outline-none focus:ring-2 focus:ring-blue-500',
            error ? 'border-red-500' : 'border-gray-300',
            className
          )}
          {...props}
        />
        {error && <p className="text-sm text-red-600 mt-1">{error}</p>}
      </div>
    );
  }
);

Textarea.displayName = 'Textarea';
```

### `components/ui/Select.tsx`

```typescript
import { SelectHTMLAttributes, forwardRef } from 'react';
import { cn } from '@/lib/utils';

interface SelectProps extends SelectHTMLAttributes<HTMLSelectElement> {
  error?: string;
}

export const Select = forwardRef<HTMLSelectElement, SelectProps>(
  ({ error, className, children, ...props }, ref) => {
    return (
      <div className="w-full">
        <select
          ref={ref}
          className={cn(
            'w-full px-3 py-2 border rounded-md bg-white',
            'focus:outline-none focus:ring-2 focus:ring-blue-500',
            error ? 'border-red-500' : 'border-gray-300',
            className
          )}
          {...props}
        >
          {children}
        </select>
        {error && <p className="text-sm text-red-600 mt-1">{error}</p>}
      </div>
    );
  }
);

Select.displayName = 'Select';
```

### `components/ui/Badge.tsx`

```typescript
import { cn } from '@/lib/utils';

interface BadgeProps {
  children: React.ReactNode;
  variant?: 'success' | 'warning' | 'info' | 'default';
  className?: string;
}

export function Badge({ children, variant = 'default', className }: BadgeProps) {
  return (
    <span
      className={cn(
        'inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium',
        variant === 'success' && 'bg-green-100 text-green-800',
        variant === 'warning' && 'bg-yellow-100 text-yellow-800',
        variant === 'info' && 'bg-blue-100 text-blue-800',
        variant === 'default' && 'bg-gray-100 text-gray-800',
        className
      )}
    >
      {children}
    </span>
  );
}
```

### `lib/utils.ts`

```typescript
import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
```

## 📋 Компонент просмотра рецепта

### `components/recipes/RecipeView.tsx`

```typescript
'use client';

import { Recipe } from '@/types/recipe';
import { Badge } from '@/components/ui/Badge';
import { Button } from '@/components/ui/Button';
import { useState } from 'react';

interface RecipeViewProps {
  recipe: Recipe;
  onPublish?: () => void;
  onDelete?: () => void;
}

export function RecipeView({ recipe, onPublish, onDelete }: RecipeViewProps) {
  const [currentLang, setCurrentLang] = useState(recipe.language);

  // Получить название на текущем языке
  const getName = () => {
    if (currentLang === recipe.language) return recipe.name;
    const translation = recipe.translations?.find((t) => t.language === currentLang);
    return translation?.name || recipe.name;
  };

  // Получить инструкции на текущем языке
  const getInstructions = () => {
    if (currentLang === recipe.language) return recipe.instructions;
    const translation = recipe.translations?.find((t) => t.language === currentLang);
    return translation?.instructions || recipe.instructions;
  };

  return (
    <div className="max-w-4xl mx-auto space-y-6">
      {/* Шапка */}
      <div className="flex items-start justify-between">
        <div className="flex-1">
          <h1 className="text-4xl font-bold mb-2">{getName()}</h1>
          <div className="flex items-center gap-2">
            <Badge variant={recipe.status === 'published' ? 'success' : 'warning'}>
              {recipe.status === 'published' ? '✅ Опубликован' : '📝 Черновик'}
            </Badge>
            <span className="text-sm text-gray-500">
              🍽️ {recipe.servings} {recipe.servings === 1 ? 'порция' : 'порций'}
            </span>
          </div>
        </div>

        <div className="flex gap-2">
          {recipe.status === 'draft' && onPublish && (
            <Button onClick={onPublish} variant="primary">
              Опубликовать
            </Button>
          )}
          {onDelete && (
            <Button onClick={onDelete} variant="danger">
              Удалить
            </Button>
          )}
        </div>
      </div>

      {/* Переключатель языков */}
      <div className="flex gap-2 border-b pb-4">
        {['ru', 'en', 'pl', 'uk'].map((lang) => {
          const hasTranslation = 
            lang === recipe.language || 
            recipe.translations?.some((t) => t.language === lang);

          return (
            <button
              key={lang}
              onClick={() => setCurrentLang(lang as any)}
              disabled={!hasTranslation}
              className={`
                px-4 py-2 rounded-md font-medium transition-colors
                ${currentLang === lang 
                  ? 'bg-blue-600 text-white' 
                  : hasTranslation 
                    ? 'bg-gray-200 text-gray-800 hover:bg-gray-300' 
                    : 'bg-gray-100 text-gray-400 cursor-not-allowed'
                }
              `}
            >
              {lang.toUpperCase()}
              {lang === recipe.language && ' 🌍'}
              {lang !== recipe.language && hasTranslation && ' ✅'}
            </button>
          );
        })}
      </div>

      {/* Инструкции */}
      <div>
        <h2 className="text-2xl font-semibold mb-4">Инструкции</h2>
        <div className="prose prose-lg max-w-none">
          <p className="whitespace-pre-wrap">{getInstructions()}</p>
        </div>
      </div>

      {/* Ингредиенты */}
      <div>
        <h2 className="text-2xl font-semibold mb-4">Ингредиенты</h2>
        <ul className="space-y-2">
          {recipe.ingredients.map((ingredient, index) => (
            <li key={index} className="flex items-center">
              <span className="w-2 h-2 bg-blue-600 rounded-full mr-3"></span>
              <span className="font-medium">{ingredient.catalog_ingredient_id}</span>
              <span className="text-gray-500 ml-2">
                - {ingredient.quantity} {ingredient.unit}
              </span>
            </li>
          ))}
        </ul>
      </div>

      {/* Метаинформация */}
      <div className="border-t pt-4 text-sm text-gray-500">
        <p>Создан: {new Date(recipe.created_at).toLocaleString('ru-RU')}</p>
        <p>Обновлен: {new Date(recipe.updated_at).toLocaleString('ru-RU')}</p>
      </div>
    </div>
  );
}
```

## 📋 Компонент списка рецептов

### `components/recipes/RecipeList.tsx`

```typescript
'use client';

import { useRecipes } from '@/hooks/useRecipes';
import { RecipeCard } from './RecipeCard';
import { Button } from '@/components/ui/Button';
import { useState } from 'react';
import Link from 'next/link';

export function RecipeList() {
  const [status, setStatus] = useState<'draft' | 'published' | undefined>();
  const { recipes, total, loading, error } = useRecipes({ status, limit: 20 });

  if (loading) {
    return <div className="text-center py-8">Загрузка рецептов...</div>;
  }

  if (error) {
    return (
      <div className="bg-red-50 border border-red-200 text-red-800 px-4 py-3 rounded">
        {error}
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Шапка с фильтрами */}
      <div className="flex items-center justify-between">
        <h1 className="text-3xl font-bold">Мои рецепты ({total})</h1>
        <Link href="/recipes/create">
          <Button variant="primary">+ Создать рецепт</Button>
        </Link>
      </div>

      {/* Фильтры */}
      <div className="flex gap-2">
        <Button
          variant={status === undefined ? 'primary' : 'secondary'}
          onClick={() => setStatus(undefined)}
        >
          Все
        </Button>
        <Button
          variant={status === 'draft' ? 'primary' : 'secondary'}
          onClick={() => setStatus('draft')}
        >
          Черновики
        </Button>
        <Button
          variant={status === 'published' ? 'primary' : 'secondary'}
          onClick={() => setStatus('published')}
        >
          Опубликованные
        </Button>
      </div>

      {/* Список рецептов */}
      {recipes.length === 0 ? (
        <div className="text-center py-12">
          <p className="text-gray-500 mb-4">Рецепты не найдены</p>
          <Link href="/recipes/create">
            <Button variant="primary">Создать первый рецепт</Button>
          </Link>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {recipes.map((recipe) => (
            <RecipeCard key={recipe.id} recipe={recipe} />
          ))}
        </div>
      )}
    </div>
  );
}
```

### `components/recipes/RecipeCard.tsx`

```typescript
import Link from 'next/link';
import { Recipe } from '@/types/recipe';
import { Badge } from '@/components/ui/Badge';

interface RecipeCardProps {
  recipe: Recipe;
}

export function RecipeCard({ recipe }: RecipeCardProps) {
  const translationCount = recipe.translations?.length || 0;

  return (
    <Link href={`/recipes/${recipe.id}`}>
      <div className="border rounded-lg p-4 hover:shadow-lg transition-shadow cursor-pointer">
        <div className="flex items-start justify-between mb-3">
          <h3 className="text-xl font-semibold line-clamp-2">{recipe.name}</h3>
          <Badge variant={recipe.status === 'published' ? 'success' : 'warning'}>
            {recipe.status === 'published' ? '✅' : '📝'}
          </Badge>
        </div>

        <p className="text-gray-600 text-sm line-clamp-3 mb-4">
          {recipe.instructions}
        </p>

        <div className="flex items-center justify-between text-sm text-gray-500">
          <span>🍽️ {recipe.servings} порций</span>
          <span>🌐 {translationCount}/3 переводов</span>
        </div>

        <div className="mt-3 text-xs text-gray-400">
          {new Date(recipe.created_at).toLocaleDateString('ru-RU')}
        </div>
      </div>
    </Link>
  );
}
```

## 📄 Страницы

### `app/recipes/page.tsx` (Список рецептов)

```typescript
import { RecipeList } from '@/components/recipes/RecipeList';

export default function RecipesPage() {
  return (
    <div className="container mx-auto px-4 py-8">
      <RecipeList />
    </div>
  );
}
```

### `app/recipes/[id]/page.tsx` (Просмотр рецепта)

```typescript
'use client';

import { useParams, useRouter } from 'next/navigation';
import { useRecipe } from '@/hooks/useRecipe';
import { recipeService } from '@/services/recipeService';
import { RecipeView } from '@/components/recipes/RecipeView';
import { Button } from '@/components/ui/Button';

export default function RecipePage() {
  const params = useParams();
  const router = useRouter();
  const { recipe, loading, error } = useRecipe(params.id as string);

  const handlePublish = async () => {
    if (!recipe) return;
    try {
      await recipeService.publish(recipe.id);
      router.refresh();
    } catch (err) {
      alert('Ошибка публикации рецепта');
    }
  };

  const handleDelete = async () => {
    if (!recipe) return;
    if (!confirm('Удалить рецепт?')) return;
    try {
      await recipeService.delete(recipe.id);
      router.push('/recipes');
    } catch (err) {
      alert('Ошибка удаления рецепта');
    }
  };

  if (loading) {
    return <div className="text-center py-8">Загрузка...</div>;
  }

  if (error || !recipe) {
    return (
      <div className="bg-red-50 border border-red-200 text-red-800 px-4 py-3 rounded">
        {error || 'Рецепт не найден'}
      </div>
    );
  }

  return (
    <div className="container mx-auto px-4 py-8">
      <Button
        variant="secondary"
        onClick={() => router.back()}
        className="mb-6"
      >
        ← Назад
      </Button>
      <RecipeView
        recipe={recipe}
        onPublish={handlePublish}
        onDelete={handleDelete}
      />
    </div>
  );
}
```

## 🚀 Готово к запуску!

Все компоненты готовы. Теперь можно:

1. Создать Next.js проект
2. Скопировать эти компоненты
3. Установить зависимости
4. Запустить `npm run dev`
5. Открыть http://localhost:3000/recipes/create

**Автоматические переводы работают из коробки!** 🌐✨
