# 🤖 Recipe AI Insights - Quick Reference

## 📋 Database Schema

```sql
recipe_ai_insights (
  id UUID,
  recipe_id UUID → recipes_v2(id),
  language VARCHAR(5),             -- ru/en/pl/uk
  steps_json JSONB,                -- Массив шагов
  validation_json JSONB,           -- Warnings/errors
  suggestions_json JSONB,          -- Улучшения
  feasibility_score INT (0..100),  -- Оценка реализуемости
  model VARCHAR(100),              -- "llama-3.1-8b-instant"
  created_at TIMESTAMP,
  updated_at TIMESTAMP,
  UNIQUE(recipe_id, language)
)
```

## 🚀 API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/recipes/v2/:id/insights/:lang` | Получить (или сгенерировать) инсайты |
| `POST` | `/api/recipes/v2/:id/insights/:lang` | Сгенерировать новые инсайты |
| `POST` | `/api/recipes/v2/:id/insights/:lang/refresh` | Обновить (force) |
| `GET` | `/api/recipes/v2/:id/insights` | Все языки |

## 💡 Примеры запросов

### Получить AI инсайты

```bash
curl -X GET http://localhost:8000/api/recipes/v2/UUID/insights/ru \
  -H "Authorization: Bearer JWT_TOKEN"
```

**Ответ:**
```json
{
  "insights": {
    "steps": [
      {
        "step_number": 1,
        "action": "Нарезать",
        "description": "Нарезать свеклу кубиками 2х2 см",
        "duration_minutes": 10,
        "temperature": null,
        "technique": "dice"
      }
    ],
    "validation": {
      "is_valid": true,
      "warnings": [...],
      "errors": [],
      "safety_checks": [...]
    },
    "suggestions": [
      {
        "title": "Использовать свежую свеклу",
        "description": "...",
        "impact": "taste",
        "confidence": 0.9
      }
    ],
    "feasibility_score": 85
  },
  "generated_in_ms": 2500
}
```

### Обновить инсайты

```bash
curl -X POST http://localhost:8000/api/recipes/v2/UUID/insights/ru/refresh \
  -H "Authorization: Bearer JWT_TOKEN"
```

## 📊 JSON Structures

### CookingStep

```json
{
  "step_number": 1,
  "action": "Варить",
  "description": "Варить свеклу в подсоленной воде 45 минут",
  "duration_minutes": 45,
  "temperature": "100°C",
  "technique": "boil",
  "ingredients_used": ["beet_id", "water", "salt"]
}
```

### ValidationIssue

```json
{
  "severity": "warning",
  "code": "LONG_COOKING_TIME",
  "message": "Время приготовления превышает 2 часа",
  "field": "duration"
}
```

### RecipeSuggestion

```json
{
  "suggestion_type": "improvement",
  "title": "Использовать свежую свеклу",
  "description": "Свежая свекла придаст более насыщенный вкус",
  "impact": "taste",
  "confidence": 0.9
}
```

## 🎨 Frontend TypeScript Types

```typescript
interface CookingStep {
  step_number: number;
  action: string;
  description: string;
  duration_minutes?: number;
  temperature?: string;
  technique?: string;
  ingredients_used: string[];
}

interface ValidationIssue {
  severity: 'warning' | 'error';
  code: string;
  message: string;
  field?: string;
}

interface RecipeValidation {
  is_valid: boolean;
  warnings: ValidationIssue[];
  errors: ValidationIssue[];
  missing_ingredients: string[];
  safety_checks: string[];
}

interface RecipeSuggestion {
  suggestion_type: 'improvement' | 'substitution' | 'technique';
  title: string;
  description: string;
  impact: 'taste' | 'texture' | 'nutrition' | 'cost';
  confidence: number;
}

interface RecipeAIInsights {
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

## 🔧 Service Usage (TypeScript)

```typescript
import { recipeInsightsService } from '@/services/recipeInsightsService';

// Получить или сгенерировать
const insights = await recipeInsightsService.getOrGenerate(recipeId, 'ru');

// Принудительно обновить
const fresh = await recipeInsightsService.refresh(recipeId, 'ru');

// Все языки
const all = await recipeInsightsService.getAll(recipeId);
```

## 🎯 Use Cases

### 1. Показать AI инсайты в рецепте

```tsx
<RecipeAIInsights recipeId={recipe.id} language="ru" />
```

### 2. Фильтр по качественным рецептам

```typescript
// Backend
const highQualityRecipes = await repository.get_high_quality_recipes(80, 20);

// Frontend
const recipes = await recipeService.list({ min_feasibility_score: 80 });
```

### 3. Кнопка "Обновить AI"

```tsx
<button onClick={() => recipeInsightsService.refresh(recipeId, 'ru')}>
  🔄 Обновить AI инсайты
</button>
```

## ⚡ Performance

- **Первая генерация**: 2-3 секунды (AI call)
- **Cached инсайты**: < 50ms (DB query)
- **Refresh**: 2-3 секунды (AI call + upsert)

## 📚 Files Structure

```
migrations/
└── 20260216000001_add_recipe_ai_insights.sql

src/
├── domain/
│   └── recipe_ai_insights.rs         # Types
├── infrastructure/
│   ├── groq_service.rs               # analyze_recipe()
│   └── persistence/
│       └── recipe_ai_insights_repository.rs
└── application/
    └── recipe_ai_insights_service.rs # Business logic
```

## ✅ Migration

```bash
# Apply migration
sqlx migrate run

# Проверить
psql $DATABASE_URL -c "\d recipe_ai_insights"
```

## 🔍 SQL Queries

### Рецепты с высокой оценкой

```sql
SELECT DISTINCT recipe_id, feasibility_score
FROM recipe_ai_insights
WHERE feasibility_score >= 80
ORDER BY feasibility_score DESC
LIMIT 20;
```

### Рецепты с предупреждениями

```sql
SELECT recipe_id, language, validation_json->'warnings'
FROM recipe_ai_insights
WHERE jsonb_array_length(validation_json->'warnings') > 0;
```

### Подсчет инсайтов по языкам

```sql
SELECT language, COUNT(*)
FROM recipe_ai_insights
GROUP BY language;
```

---

**Quick reference готов!** 🚀
