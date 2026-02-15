# 🤖 Recipe AI Insights - Implementation Guide

## Обзор

Новая фича: AI-инсайты для рецептов. Отдельная таблица `recipe_ai_insights` хранит структурированный анализ от AI, включая:

- 📋 **Детальные шаги** приготовления (с временем, температурой, техниками)
- ✅ **Валидация** рецепта (предупреждения, ошибки, недостающие ингредиенты)
- 💡 **Предложения** по улучшению (замены ингредиентов, техники)
- 📊 **Оценка реализуемости** (0-100)

## 🏗️ Архитектура

### База данных

```sql
recipe_ai_insights
├── id (UUID)
├── recipe_id (UUID) → recipes_v2(id)
├── language (VARCHAR) - ru/en/pl/uk
├── steps_json (JSONB) - массив шагов
├── validation_json (JSONB) - warnings/errors
├── suggestions_json (JSONB) - улучшения
├── feasibility_score (INT) - 0..100
├── model (VARCHAR) - "llama-3.1-8b-instant"
├── created_at (TIMESTAMP)
└── updated_at (TIMESTAMP)

UNIQUE(recipe_id, language)
```

### Rust структуры

```rust
// Шаг приготовления
CookingStep {
    step_number: i32,
    action: String,              // "Нарезать", "Варить"
    description: String,         // Полное описание
    duration_minutes: Option<i32>,
    temperature: Option<String>, // "180°C", "medium heat"
    technique: Option<String>,   // "dice", "julienne"
    ingredients_used: Vec<String>
}

// Валидация
RecipeValidation {
    is_valid: bool,
    warnings: Vec<ValidationIssue>,
    errors: Vec<ValidationIssue>,
    missing_ingredients: Vec<String>,
    safety_checks: Vec<String>
}

// Предложение
RecipeSuggestion {
    suggestion_type: String,     // "improvement", "substitution"
    title: String,
    description: String,
    impact: String,              // "taste", "nutrition", "cost"
    confidence: f32              // 0.0 - 1.0
}
```

## 🚀 API Endpoints (новые)

### 1. Генерация AI инсайтов

```
POST /api/recipes/v2/:id/insights/:language
```

**Пример запроса:**
```bash
curl -X POST http://localhost:8000/api/recipes/v2/UUID/insights/ru \
  -H "Authorization: Bearer JWT_TOKEN"
```

**Ответ:**
```json
{
  "insights": {
    "id": "uuid",
    "recipe_id": "uuid",
    "language": "ru",
    "steps": [
      {
        "step_number": 1,
        "action": "Нарезать",
        "description": "Нарезать свеклу кубиками 2х2 см",
        "duration_minutes": 10,
        "temperature": null,
        "technique": "dice",
        "ingredients_used": ["beet_id"]
      },
      {
        "step_number": 2,
        "action": "Варить",
        "description": "Варить свеклу в подсоленной воде",
        "duration_minutes": 45,
        "temperature": "100°C",
        "technique": "boil",
        "ingredients_used": ["beet_id", "water", "salt"]
      }
    ],
    "validation": {
      "is_valid": true,
      "warnings": [
        {
          "severity": "warning",
          "code": "LONG_COOKING_TIME",
          "message": "Время приготовления превышает 2 часа - требует планирования",
          "field": "duration"
        }
      ],
      "errors": [],
      "missing_ingredients": [],
      "safety_checks": [
        "Убедитесь, что вода кипит перед добавлением ингредиентов",
        "Не оставляйте кастрюлю без присмотра"
      ]
    },
    "suggestions": [
      {
        "suggestion_type": "improvement",
        "title": "Использовать свежую свеклу вместо консервированной",
        "description": "Свежая свекла придаст более насыщенный вкус и яркий цвет борщу",
        "impact": "taste",
        "confidence": 0.9
      },
      {
        "suggestion_type": "substitution",
        "title": "Альтернатива говядине",
        "description": "Можно заменить говядину на свинину или курицу для более легкого варианта",
        "impact": "nutrition",
        "confidence": 0.75
      }
    ],
    "feasibility_score": 85,
    "model": "llama-3.1-8b-instant",
    "created_at": "2026-02-15T10:30:00Z",
    "updated_at": "2026-02-15T10:30:00Z"
  },
  "generated_in_ms": 2500
}
```

### 2. Получить существующие инсайты

```
GET /api/recipes/v2/:id/insights/:language
```

**Ответ:**
- Если существуют: возвращает cached insights
- Если нет: генерирует новые автоматически

### 3. Обновить инсайты (force refresh)

```
POST /api/recipes/v2/:id/insights/:language/refresh
```

Принудительно генерирует новые инсайты (перезаписывает старые).

### 4. Получить инсайты для всех языков

```
GET /api/recipes/v2/:id/insights
```

**Ответ:**
```json
{
  "insights": [
    { "language": "ru", "steps": [...], ... },
    { "language": "en", "steps": [...], ... },
    { "language": "pl", "steps": [...], ... },
    { "language": "uk", "steps": [...], ... }
  ]
}
```

## 🎨 Frontend Integration

### Компонент отображения инсайтов

```typescript
// types/recipe.ts
export interface CookingStep {
  step_number: number;
  action: string;
  description: string;
  duration_minutes?: number;
  temperature?: string;
  technique?: string;
  ingredients_used: string[];
}

export interface ValidationIssue {
  severity: 'warning' | 'error';
  code: string;
  message: string;
  field?: string;
}

export interface RecipeValidation {
  is_valid: boolean;
  warnings: ValidationIssue[];
  errors: ValidationIssue[];
  missing_ingredients: string[];
  safety_checks: string[];
}

export interface RecipeSuggestion {
  suggestion_type: 'improvement' | 'substitution' | 'technique';
  title: string;
  description: string;
  impact: 'taste' | 'texture' | 'nutrition' | 'cost';
  confidence: number;
}

export interface RecipeAIInsights {
  id: string;
  recipe_id: string;
  language: string;
  steps: CookingStep[];
  validation: RecipeValidation;
  suggestions: RecipeSuggestion[];
  feasibility_score: number;
  model: string;
  created_at: string;
  updated_at: string;
}
```

### Service для AI инсайтов

```typescript
// services/recipeInsightsService.ts
import { api } from './api';
import { RecipeAIInsights } from '@/types/recipe';

export const recipeInsightsService = {
  // Получить или сгенерировать инсайты
  async getOrGenerate(recipeId: string, language: string): Promise<RecipeAIInsights> {
    const res = await api.get(`/api/recipes/v2/${recipeId}/insights/${language}`);
    return res.data.insights;
  },

  // Принудительно обновить инсайты
  async refresh(recipeId: string, language: string): Promise<RecipeAIInsights> {
    const res = await api.post(`/api/recipes/v2/${recipeId}/insights/${language}/refresh`);
    return res.data.insights;
  },

  // Получить все инсайты (все языки)
  async getAll(recipeId: string): Promise<RecipeAIInsights[]> {
    const res = await api.get(`/api/recipes/v2/${recipeId}/insights`);
    return res.data.insights;
  },
};
```

### Компонент отображения

```typescript
// components/recipes/RecipeAIInsights.tsx
'use client';

import { useState, useEffect } from 'react';
import { recipeInsightsService } from '@/services/recipeInsightsService';
import { RecipeAIInsights } from '@/types/recipe';

interface Props {
  recipeId: string;
  language: string;
}

export function RecipeAIInsights({ recipeId, language }: Props) {
  const [insights, setInsights] = useState<RecipeAIInsights | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);

  useEffect(() => {
    const fetchInsights = async () => {
      try {
        setLoading(true);
        const data = await recipeInsightsService.getOrGenerate(recipeId, language);
        setInsights(data);
      } catch (err) {
        console.error('Failed to load insights:', err);
      } finally {
        setLoading(false);
      }
    };

    fetchInsights();
  }, [recipeId, language]);

  const handleRefresh = async () => {
    try {
      setRefreshing(true);
      const data = await recipeInsightsService.refresh(recipeId, language);
      setInsights(data);
    } catch (err) {
      console.error('Failed to refresh insights:', err);
    } finally {
      setRefreshing(false);
    }
  };

  if (loading) return <div>Загрузка AI инсайтов...</div>;
  if (!insights) return <div>Нет данных</div>;

  return (
    <div className="space-y-6">
      {/* Шапка с оценкой */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold">AI Инсайты</h2>
          <p className="text-sm text-gray-500">
            Модель: {insights.model} • {new Date(insights.updated_at).toLocaleString()}
          </p>
        </div>
        <div className="flex items-center gap-4">
          <div className="text-center">
            <div className="text-3xl font-bold text-blue-600">{insights.feasibility_score}%</div>
            <div className="text-sm text-gray-500">Реализуемость</div>
          </div>
          <button
            onClick={handleRefresh}
            disabled={refreshing}
            className="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50"
          >
            {refreshing ? 'Обновление...' : '🔄 Обновить'}
          </button>
        </div>
      </div>

      {/* Шаги приготовления */}
      <div>
        <h3 className="text-xl font-semibold mb-4">📋 Шаги приготовления</h3>
        <div className="space-y-4">
          {insights.steps.map((step) => (
            <div key={step.step_number} className="border rounded-lg p-4">
              <div className="flex items-start gap-4">
                <div className="flex-shrink-0 w-8 h-8 bg-blue-600 text-white rounded-full flex items-center justify-center font-bold">
                  {step.step_number}
                </div>
                <div className="flex-1">
                  <div className="font-semibold text-lg">{step.action}</div>
                  <p className="text-gray-700 mt-1">{step.description}</p>
                  <div className="flex gap-4 mt-2 text-sm text-gray-500">
                    {step.duration_minutes && (
                      <span>⏱️ {step.duration_minutes} мин</span>
                    )}
                    {step.temperature && (
                      <span>🌡️ {step.temperature}</span>
                    )}
                    {step.technique && (
                      <span>🔧 {step.technique}</span>
                    )}
                  </div>
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Валидация */}
      <div>
        <h3 className="text-xl font-semibold mb-4">✅ Валидация</h3>
        
        {insights.validation.errors.length > 0 && (
          <div className="mb-4">
            <h4 className="font-semibold text-red-600 mb-2">Ошибки:</h4>
            {insights.validation.errors.map((issue, i) => (
              <div key={i} className="bg-red-50 border border-red-200 rounded p-3 mb-2">
                <span className="font-medium">{issue.code}:</span> {issue.message}
              </div>
            ))}
          </div>
        )}

        {insights.validation.warnings.length > 0 && (
          <div className="mb-4">
            <h4 className="font-semibold text-yellow-600 mb-2">Предупреждения:</h4>
            {insights.validation.warnings.map((issue, i) => (
              <div key={i} className="bg-yellow-50 border border-yellow-200 rounded p-3 mb-2">
                <span className="font-medium">{issue.code}:</span> {issue.message}
              </div>
            ))}
          </div>
        )}

        {insights.validation.safety_checks.length > 0 && (
          <div>
            <h4 className="font-semibold text-blue-600 mb-2">Безопасность:</h4>
            <ul className="list-disc list-inside space-y-1">
              {insights.validation.safety_checks.map((check, i) => (
                <li key={i} className="text-gray-700">{check}</li>
              ))}
            </ul>
          </div>
        )}
      </div>

      {/* Предложения */}
      <div>
        <h3 className="text-xl font-semibold mb-4">💡 Предложения по улучшению</h3>
        <div className="space-y-3">
          {insights.suggestions.map((suggestion, i) => (
            <div key={i} className="border rounded-lg p-4">
              <div className="flex items-start justify-between">
                <div className="flex-1">
                  <h4 className="font-semibold">{suggestion.title}</h4>
                  <p className="text-gray-700 mt-1">{suggestion.description}</p>
                  <div className="flex gap-3 mt-2 text-sm">
                    <span className="text-gray-500">
                      Влияние: <span className="font-medium">{suggestion.impact}</span>
                    </span>
                    <span className="text-gray-500">
                      Уверенность: <span className="font-medium">{Math.round(suggestion.confidence * 100)}%</span>
                    </span>
                  </div>
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
```

## 🎯 Use Cases

### 1. Просмотр рецепта с AI инсайтами

```typescript
// app/recipes/[id]/page.tsx
import { RecipeView } from '@/components/recipes/RecipeView';
import { RecipeAIInsights } from '@/components/recipes/RecipeAIInsights';

export default function RecipePage({ params }: { params: { id: string } }) {
  return (
    <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
      {/* Левая колонка: основная информация */}
      <RecipeView recipeId={params.id} />
      
      {/* Правая колонка: AI инсайты */}
      <RecipeAIInsights recipeId={params.id} language="ru" />
    </div>
  );
}
```

### 2. Лента рецептов с оценкой качества

```typescript
// Фильтр по высококачественным рецептам
const highQualityRecipes = await recipeService.list({
  min_feasibility_score: 80,
  limit: 20
});
```

### 3. Кнопка "Обновить AI инсайты"

Пользователь может вручную обновить инсайты если:
- Изменил рецепт
- Хочет получить новые предложения
- AI выдал неточности

## ⚡ Performance

### Кеширование

- ✅ Инсайты сохраняются в БД после первой генерации
- ✅ Повторные запросы возвращают cached данные
- ✅ Refresh доступен по требованию

### Время генерации

- Первая генерация: ~2-3 секунды (Groq AI call)
- Cached инсайты: < 50ms (database query)

### Масштабируемость

- Инсайты генерируются асинхронно
- Можно добавить background job для pre-generation
- JSONB индексы для быстрого поиска

## 🚀 Next Steps

1. ✅ Миграция БД создана
2. ✅ Domain модели готовы
3. ✅ Repository готов
4. ✅ AI Service готов
5. ⏳ HTTP endpoints (add to routes)
6. ⏳ Frontend components
7. ⏳ Testing

---

**Готово к интеграции!** 🎉
