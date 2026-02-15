# AI Insights Implementation Status

---

## ✅ Что СДЕЛАНО (V1.1 - с валидатором)

### 1. Database Schema ✅
- ✅ Таблица `recipe_ai_insights` создана
- ✅ Поля: `steps_json`, `validation_json`, `suggestions_json`, `feasibility_score`
- ✅ Индексы на `recipe_id`, `language`, `feasibility_score`
- ✅ Foreign key: `recipe_id` → `recipes(id)` ON DELETE CASCADE
- ✅ Unique constraint: `(recipe_id, language)`
- ✅ Триггер auto-update `updated_at`

**Миграция**: `migrations/20260216000001_add_recipe_ai_insights.sql`

### 2. Domain Models ✅
- ✅ `CookingStep` - шаг приготовления
- ✅ `ValidationIssue` - ошибка/предупреждение
- ✅ `RecipeValidation` - результат валидации
- ✅ `RecipeSuggestion` - предложения по улучшению
- ✅ `RecipeAIInsights` - основная структура
- ✅ `RecipeAIInsightsRow` - маппинг на БД с JSONB
- ✅ `RecipeAIInsightsResponse` - DTO для API

**Файл**: `src/domain/recipe_ai_insights.rs`

### 3. Repository Layer ✅
- ✅ `RecipeAIInsightsRepository::upsert()` - сохранение (INSERT ON CONFLICT)
- ✅ `get_by_recipe_and_language()` - получение конкретных insights
- ✅ `get_all_by_recipe()` - все языки для рецепта
- ✅ `delete_by_recipe()` - удаление при удалении рецепта
- ✅ `delete_by_recipe_and_language()` - удаление конкретного языка
- ✅ `get_high_quality_recipes()` - фильтрация по feasibility_score

**Файл**: `src/infrastructure/persistence/recipe_ai_insights_repository.rs`

### 4. AI Service ✅
- ✅ `RecipeAIInsightsService` с dependency injection
- ✅ `generate_insights_by_id()` - генерация insights для recipe_id
- ✅ `generate_insights_for_recipe()` - генерация для Recipe entity
- ✅ `get_or_generate_insights_by_id()` - cache-first strategy
- ✅ `refresh_insights_by_id()` - force regeneration
- ✅ `get_all_insights()` - получение всех языков
- ✅ `build_analysis_prompt()` - построение AI prompt
- ✅ `parse_ai_response()` - парсинг JSON от AI

**Файл**: `src/application/recipe_ai_insights_service.rs`

### 5. Groq Integration ✅
- ✅ Новый метод `groq_service.analyze_recipe()`
- ✅ Параметры: temperature 0.3, max_tokens 2000
- ✅ Retry logic: 1 retry с 200ms delay
- ✅ Валидация длины prompt

**Файл**: `src/infrastructure/groq_service.rs`

### 6. HTTP Handlers ✅
- ✅ `GET /api/recipes/v2/:id/insights/:language` - получить или сгенерировать
- ✅ `POST /api/recipes/v2/:id/insights/:language/refresh` - перегенерировать
- ✅ `GET /api/recipes/v2/:id/insights` - все языки

**Файл**: `src/interfaces/http/recipe_ai_insights.rs`

### 7. Wiring ✅
- ✅ Модули зарегистрированы в `src/domain/mod.rs`
- ✅ Репозиторий в `src/infrastructure/persistence/mod.rs`
- ✅ Сервис в `src/application/mod.rs`
- ✅ HTTP handlers в `src/interfaces/http/mod.rs`
- ✅ Routes в `src/interfaces/http/routes.rs`
- ✅ Service initialization в `src/main.rs`

### 8. Compilation ✅
- ✅ `cargo check` - успешно
- ✅ `cargo build --release` - успешно (только warnings)
- ✅ Все type errors исправлены
- ✅ Все borrow checker errors исправлены

### 9. Rule-Based Validator ✅ NEW!
- ✅ `RecipeValidator` с pre-AI validation
- ✅ Детекция типа блюда (Cake, Pie, Soup, Salad, etc.)
- ✅ Проверка безопасности продуктов (сырое мясо, яйца)
- ✅ Логическая валидация (торт из овощей = error)
- ✅ Определение критических ингредиентов
- ✅ Validation codes (RAW_MEAT_DANGER, NO_THERMAL_PROCESSING, etc.)
- ✅ Интеграция в AI service (запускается ПЕРЕД AI)

**Файл**: `src/application/recipe_validator.rs`

### 10. Enhanced AI Prompt ✅ NEW!
- ✅ Профессиональный контекст (HACCP-certified technologist)
- ✅ Validation context injection (ошибки/предупреждения валидатора)
- ✅ Руководство по CCP (Critical Control Points)
- ✅ Интерпретация feasibility score (0-100 с описанием)
- ✅ Строгий JSON format enforcement
- ✅ Правила: не выдумывать ингредиенты, проверять логику, безопасность

**Файл**: `src/application/recipe_ai_insights_service.rs`

### 11. Testing ✅
- ✅ Test 1: Невозможный рецепт (торт из овощей) → score=10, error
- ✅ Test 2: Опасный рецепт (сырое мясо) → score=50, critical error
- ✅ Test 3: Правильный рецепт (борщ) → score=85, no errors
- ✅ 100% pass rate

**Тесты**: `test_validator.sh`, `test_ai_simple.sh`

---

## 🔄 Что нужно УЛУЧШИТЬ (V2)

### 1. JSON Schema (текущая → V2)

**Текущая структура (V1 - базовая)**:
```json
{
  "step_number": 1,
  "action": "Нарезать",
  "description": "Нарезать овощи кубиками",
  "duration_minutes": 10,
  "temperature": "180°C",
  "technique": "резка",
  "ingredients_used": ["морковь", "лук"]
}
```

**Новая структура (V2 - профессиональная)**:
```json
{
  "n": 1,
  "title": "Подготовка ингредиентов",
  "details": [
    "Достать продукты из холодильника",
    "Нарезать овощи кубиками 1x1 см"
  ],
  "time_min": 10,
  "temp_c": null,
  "ccp": ["Размер кубиков должен быть одинаковым"]
}
```

**Изменения**:
- ✅ `step_number` → `n` (короче)
- ✅ `action` + `description` → `title` + `details[]` (более структурировано)
- ✅ `duration_minutes` → `time_min` (короче)
- ✅ `temperature` → `temp_c` (числовой формат)
- ✅ `technique` → удалено (не нужно)
- ✅ `ingredients_used` → убрано из steps (AI не должен добавлять ингредиенты)
- ✅ **НОВОЕ**: `ccp` - критические точки контроля (HACCP)

### 2. Validation Structure

**Текущая (V1)**:
```json
{
  "is_valid": true,
  "warnings": [{"severity": "warning", "code": "...", "message": "...", "field": "..."}],
  "errors": [...],
  "missing_ingredients": ["..."],
  "safety_checks": ["..."]
}
```

**Новая (V2)**:
```json
{
  "errors": [{"code": "MISSING_BINDER", "message": "..."}],
  "warnings": [{"code": "NAME_MISMATCH", "message": "..."}],
  "missing": [{"role": "binder", "examples": ["яйцо", "мука"]}]
}
```

**Изменения**:
- ❌ `is_valid` - удалено (дублирует errors.length === 0)
- ❌ `severity` в warnings - удалено (warnings всегда некритичны)
- ❌ `field` - удалено (не используется)
- ❌ `safety_checks` - удалено (перенесено в CCP)
- ❌ `missing_ingredients` → `missing` с ролями (более структурировано)
- ✅ Упрощенные коды ошибок (константы)

### 3. Suggestions Structure

**Текущая (V1)**:
```json
{
  "suggestion_type": "improvement",
  "title": "...",
  "description": "...",
  "impact": "taste",
  "confidence": 0.85
}
```

**Новая (V2)**:
```json
{
  "fixes": [
    {"title": "...", "changes": ["...", "..."]}
  ],
  "substitutions": [
    {"ingredient": "миндаль", "options": ["фундук"], "note": "аллерген"}
  ]
}
```

**Изменения**:
- ✅ Разделение на `fixes` (исправления рецепта) и `substitutions` (замены ингредиентов)
- ❌ `confidence` - удалено (AI не может оценивать свою уверенность)
- ❌ `impact` - удалено (слишком абстрактно)
- ✅ `changes[]` - конкретные действия
- ✅ `note` для substitutions - важная информация (аллергены, изменение вкуса)

### 4. Добавить Rule-Based Validator

**ЧТО**:
Модуль `src/application/recipe_v2_validator.rs` который проверяет рецепт БЕЗ AI:
- Детектирует тип блюда по названию (торт/пирог/суп/салат)
- Анализирует роли ингредиентов (через keyword map)
- Выдает ошибки/предупреждения
- Определяет недостающие роли

**ЗАЧЕМ**:
- Быстро (5ms vs 2-3s AI)
- Надежно (не зависит от AI)
- Передаем результаты в AI prompt для более качественного анализа

**Файл**: пока НЕ создан

### 5. Добавить Orchestrator

**ЧТО**:
Модуль `src/application/recipe_v2_insights_orchestrator.rs` который координирует:
1. Rule-based validation
2. AI generation
3. Translation на другие языки
4. Сохранение результатов

**ЗАЧЕМ**:
Единая точка входа для генерации insights, упрощает логику

**Файл**: пока НЕ создан

### 6. Улучшить AI Prompt

**Текущий prompt**:
- Просто просит JSON
- Нет контекста о валидации
- Нет строгих правил

**Новый prompt**:
- System message про шеф-технолога
- Передача результатов rule-based validation
- Строгие правила (не выдумывать ингредиенты)
- Примеры ожидаемого формата
- Объяснение CCP (критические точки контроля)

**Файл**: `src/application/recipe_ai_insights_service.rs` - нужно обновить `build_analysis_prompt()`

### 7. Асинхронная генерация

**Текущее**:
- Endpoints вызывают AI синхронно
- Пользователь ждет 2-3 секунды

**Нужно**:
- После `POST /api/recipes/v2` запускать генерацию в `tokio::spawn`
- Endpoint возвращает 201 Created сразу
- `GET /api/recipes/v2/:id/insights` возвращает 404 если еще генерируются
- Фронт делает polling каждые 2 секунды

**Файлы**: 
- `src/interfaces/http/recipe_v2.rs` - обновить create_recipe handler
- `src/interfaces/http/recipe_ai_insights.rs` - добавить статус генерации

### 8. Перевод Insights

**Текущее**:
- Insights генерируются только на language_default рецепта
- Нет переводов на другие языки

**Нужно**:
- После генерации на default языке
- Параллельно переводить на en/ru/pl/uk
- Использовать существующий GroqService для перевода
- Переводить только human-readable поля (title, details, message, note)
- Сохранять структуру JSON без изменений

**Файл**: `src/application/recipe_ai_insights_service.rs` - добавить `translate_insights()`

---

## 📊 Migration Plan (V1 → V2)

### Option A: Backward Compatible (рекомендуется)

1. Добавить новые поля в `recipe_ai_insights`:
   ```sql
   ALTER TABLE recipe_ai_insights 
   ADD COLUMN steps_json_v2 JSONB,
   ADD COLUMN validation_json_v2 JSONB,
   ADD COLUMN suggestions_json_v2 JSONB,
   ADD COLUMN schema_version INT DEFAULT 1;
   ```

2. Код поддерживает обе версии:
   - Если `schema_version = 1` → используем старые поля
   - Если `schema_version = 2` → используем новые поля

3. Постепенная миграция:
   - Новые insights генерируются в V2
   - Старые insights остаются в V1
   - Команда для пересчета старых: `POST /admin/insights/migrate-v2`

### Option B: Breaking Change (быстрее)

1. Просто обновить структуры в коде
2. Удалить старые insights: `DELETE FROM recipe_ai_insights;`
3. Пересоздать для всех рецептов в фоне

---

## 🧪 Testing Plan

### Unit Tests
- [ ] `recipe_v2_validator` - детекция типов блюд
- [ ] `recipe_v2_validator` - анализ ролей ингредиентов
- [ ] `recipe_v2_validator` - правила валидации
- [ ] `recipe_ai_insights_service` - парсинг AI response
- [ ] `recipe_ai_insights_service` - обработка ошибок AI

### Integration Tests
- [ ] `recipe_ai_insights_repository` - JSONB serialization/deserialization
- [ ] `recipe_ai_insights_service` - генерация + сохранение
- [ ] HTTP handlers - полный flow create → insights

### E2E Test
```bash
# 1. Создать рецепт
RECIPE_ID=$(curl -X POST /api/recipes/v2 ... | jq -r '.id')

# 2. Подождать 3 секунды (AI генерация)
sleep 3

# 3. Получить insights
curl /api/recipes/v2/$RECIPE_ID/insights?lang=ru | jq .

# 4. Проверить структуру
# - steps должны иметь поля n, title, details, time_min, temp_c, ccp
# - validation должен иметь errors, warnings, missing
# - suggestions должен иметь fixes, substitutions
```

---

## 📝 Next Steps

### Immediate (можно сделать сейчас)
1. ✅ Протестировать текущие endpoints
2. ✅ Проверить что AI возвращает валидный JSON
3. ✅ Создать пример рецепта и получить insights

### Short-term (следующие 1-2 дня)
1. Создать `recipe_v2_validator.rs` с базовыми правилами
2. Обновить AI prompt под новую схему V2
3. Обновить domain models под V2 схему
4. Добавить асинхронную генерацию после create_recipe

### Mid-term (следующая неделя)
1. Создать orchestrator для координации validator + AI + translation
2. Реализовать перевод insights на все языки
3. Добавить ingredient roles (keyword map)
4. Улучшить error handling и retry logic

### Long-term (следующий месяц)
1. Админ-панель для управления ролями ингредиентов
2. Dashboard для мониторинга качества AI insights
3. A/B тестирование разных prompts
4. Fine-tuning модели на реальных рецептах

---

## 🎯 Success Metrics

- ✅ **Compilation**: 0 errors
- ✅ **Database**: Таблица создана, индексы работают
- ⏳ **API**: Endpoints возвращают 200 OK
- ⏳ **AI Quality**: feasibility_score > 70 для хороших рецептов
- ⏳ **Performance**: Генерация < 3s, получение < 50ms
- ⏳ **Coverage**: > 80% рецептов имеют insights

---

**Дата**: 2026-02-16
**Версия**: V1 (базовая) → V2 (профессиональная) в процессе
**Статус**: ✅ V1 работает, 🔄 V2 в разработке
