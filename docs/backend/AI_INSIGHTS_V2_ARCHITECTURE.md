# Recipe AI Insights V2 - Professional Architecture

## 🎯 Цель
Превратить AI insights в **профессиональный инструмент шеф-технолога**, который:
- Дает структурированную технологию приготовления
- Валидирует рецепт на реалистичность
- Предлагает исправления и замены
- Работает через строгий JSON-контракт

---

## 📋 JSON Контракт (Схема ответа AI)

```json
{
  "steps": [
    {
      "n": 1,
      "title": "Подготовка ингредиентов",
      "details": [
        "Достать продукты из холодильника за 30 минут",
        "Просеять муку через мелкое сито",
        "Размягчить масло до комнатной температуры"
      ],
      "time_min": 10,
      "temp_c": null,
      "ccp": ["Масло должно быть мягким, но не растопленным"]
    },
    {
      "n": 2,
      "title": "Замес теста",
      "details": [
        "Смешать муку, разрыхлитель и соль",
        "Взбить масло с сахаром до пышной массы",
        "Добавить яйца по одному, тщательно вмешивая"
      ],
      "time_min": 15,
      "temp_c": null,
      "ccp": ["Тесто должно быть однородным без комков"]
    },
    {
      "n": 3,
      "title": "Выпекание",
      "details": [
        "Разогреть духовку до 175°C",
        "Выложить тесто в смазанную форму",
        "Выпекать 35-40 минут до золотистой корки"
      ],
      "time_min": 40,
      "temp_c": 175,
      "ccp": [
        "Проверка зубочисткой - должна выходить сухой",
        "Золотистая корка без подгорания"
      ]
    }
  ],
  "validation": {
    "errors": [
      {
        "code": "MISSING_BINDER",
        "message": "Для торта отсутствует связующий компонент (яйцо/мука/желатин)"
      },
      {
        "code": "NO_THERMAL_STEP",
        "message": "В инструкциях нет термической обработки (выпекание/варка/жарка)"
      }
    ],
    "warnings": [
      {
        "code": "NAME_MISMATCH",
        "message": "Название 'Торт' не соответствует составу ингредиентов"
      },
      {
        "code": "UNREALISTIC_TIME",
        "message": "Указанное время приготовления (5 мин) нереально для данного блюда"
      }
    ],
    "missing": [
      {
        "role": "binder",
        "examples": ["яйцо", "мука пшеничная", "желатин"]
      },
      {
        "role": "leavening",
        "examples": ["разрыхлитель", "сода пищевая", "дрожжи"]
      }
    ]
  },
  "suggestions": {
    "fixes": [
      {
        "title": "Сделать десертом без выпечки (чизкейк)",
        "changes": [
          "Добавить творожный сыр 500г",
          "Добавить желатин 20г",
          "Охлаждать в холодильнике 4 часа вместо выпекания"
        ]
      }
    ],
    "substitutions": [
      {
        "ingredient": "миндаль",
        "options": ["фундук", "грецкий орех", "кешью"],
        "note": "Внимание: миндаль - аллерген"
      },
      {
        "ingredient": "сливочное масло",
        "options": ["растительное масло 75%", "маргарин для выпечки"],
        "note": "Вкус будет отличаться"
      }
    ]
  },
  "feasibility_score": 35
}
```

---

## 🔍 Validation Codes (коды ошибок)

### Errors (критичные - блюдо невозможно)
- `MISSING_BINDER` - нет связующего (яйцо/мука/желатин/крахмал)
- `MISSING_BASE` - нет основы (для торта/пирога/хлеба)
- `NO_THERMAL_STEP` - нет термообработки для блюда, требующего её
- `UNREALISTIC_TIME` - нереальное время (2 мин для торта)
- `TEMPERATURE_CONFLICT` - противоречивые температурные режимы
- `QUANTITY_MISMATCH` - количество ингредиентов не соответствует порциям

### Warnings (некритичные - можно приготовить, но странно)
- `NAME_MISMATCH` - название не соответствует составу
- `NO_SALT_SWEET_BALANCE` - для сладкого нет подсластителя
- `NO_SEASONING` - для основного блюда нет приправ/соли
- `MISSING_LIQUID` - мало жидкости для теста/соуса
- `ALLERGEN_RISK` - присутствуют распространенные аллергены

---

## 📐 Роли ингредиентов (для валидации)

### Вариант A: Keyword Map (быстрый старт)

```rust
// src/domain/ingredient_roles.rs
pub enum IngredientRole {
    Base,       // основа (мука, рис, макароны, картофель)
    Binder,     // связующее (яйцо, желатин, крахмал)
    Leavening,  // разрыхлитель (сода, дрожжи, разрыхлитель)
    Liquid,     // жидкость (вода, молоко, бульон)
    Fat,        // жир (масло, маргарин, сливки)
    Protein,    // белок (мясо, рыба, птица, бобовые)
    Sweetener,  // подсластитель (сахар, мед, сироп)
    Seasoning,  // приправы (соль, перец, специи)
    Acid,       // кислота (лимон, уксус, томаты)
    Vegetable,  // овощи
    Fruit,      // фрукты
    Dairy,      // молочка
    Allergen,   // аллергены (орехи, морепродукты)
}

lazy_static! {
    static ref ROLE_KEYWORDS: HashMap<IngredientRole, Vec<&'static str>> = {
        let mut m = HashMap::new();
        m.insert(IngredientRole::Base, vec![
            "мука", "рис", "макарон", "картофель", "крупа", "flour", "rice"
        ]);
        m.insert(IngredientRole::Binder, vec![
            "яйцо", "желатин", "крахмал", "агар", "egg", "gelatin", "starch"
        ]);
        m.insert(IngredientRole::Leavening, vec![
            "разрыхлитель", "сода", "дрожжи", "baking powder", "yeast"
        ]);
        // ... и т.д.
        m
    };
}
```

### Вариант B: Database Roles (профессионально)

```sql
-- Будущая миграция (после MVP)
ALTER TABLE catalog_ingredients 
ADD COLUMN roles TEXT[] DEFAULT '{}';

-- Примеры:
UPDATE catalog_ingredients SET roles = ARRAY['base', 'binder'] WHERE name_en = 'wheat flour';
UPDATE catalog_ingredients SET roles = ARRAY['binder', 'protein'] WHERE name_en = 'egg';
UPDATE catalog_ingredients SET roles = ARRAY['allergen'] WHERE name_en LIKE '%almond%';
```

---

## 🏗️ Архитектура кода

```
src/
├── application/
│   ├── recipe_v2_ai_service.rs          // AI generation + translation
│   ├── recipe_v2_validator.rs           // Rule-based validation BEFORE AI
│   └── recipe_v2_insights_orchestrator.rs // Координирует validator + AI + translation
│
├── domain/
│   ├── recipe_ai_insights_v2.rs         // Новые структуры с профессиональной схемой
│   └── ingredient_roles.rs              // Роли ингредиентов для валидации
│
├── infrastructure/
│   ├── persistence/
│   │   └── recipe_ai_insights_repository.rs  // Уже есть, обновим схему
│   └── groq_service.rs                  // Уже есть, обновим prompt
│
└── interfaces/http/
    └── recipe_v2_insights.rs            // HTTP handlers для insights
```

---

## 🔄 Workflow генерации AI Insights

### 1. POST /api/recipes/v2 (создание рецепта)

```rust
async fn create_recipe(dto: CreateRecipeDto) -> RecipeResponseDto {
    // 1. Сохранить рецепт
    let recipe = recipe_service.create_recipe(dto).await?;
    
    // 2. Запустить генерацию insights в фоне (не блокировать ответ)
    tokio::spawn(async move {
        insights_orchestrator.generate_and_translate(recipe.id).await;
    });
    
    // 3. Вернуть рецепт сразу (без ожидания AI)
    Ok(recipe)
}
```

### 2. Orchestrator координирует процесс

```rust
// src/application/recipe_v2_insights_orchestrator.rs
pub async fn generate_and_translate(&self, recipe_id: RecipeId) {
    // 1. Rule-based валидация (быстро, ~5ms)
    let validation_result = self.validator.validate(&recipe).await?;
    
    // 2. AI генерация на default языке (медленно, ~2-3s)
    let insights = self.ai_service.generate_insights(
        recipe_id,
        recipe.language_default,
        validation_result  // передаем результаты валидации в prompt
    ).await?;
    
    // 3. Сохранить insights
    self.repository.save(&insights).await?;
    
    // 4. Перевести на другие языки (параллельно)
    let target_langs = vec!["en", "pl", "uk"];
    for lang in target_langs {
        if lang != recipe.language_default {
            tokio::spawn({
                let service = self.ai_service.clone();
                let recipe_id = recipe_id.clone();
                async move {
                    service.translate_insights(recipe_id, lang).await;
                }
            });
        }
    }
}
```

### 3. GET /api/recipes/v2/:id/insights?lang=ru

```rust
async fn get_insights(
    recipe_id: Uuid,
    lang: String
) -> Result<Json<AIInsightsResponse>, AppError> {
    match repository.find_by_recipe_and_lang(recipe_id, &lang).await? {
        Some(insights) => Ok(Json(insights)),
        None => {
            // Insights еще не сгенерированы
            // Можно вернуть 202 Accepted или запустить генерацию
            Err(AppError::not_found("AI insights are being generated"))
        }
    }
}
```

### 4. POST /api/recipes/v2/:id/insights/refresh

```rust
async fn refresh_insights(recipe_id: Uuid) -> StatusCode {
    // Удалить старые insights
    repository.delete_by_recipe(recipe_id).await?;
    
    // Перегенерировать
    tokio::spawn(async move {
        orchestrator.generate_and_translate(recipe_id).await;
    });
    
    Ok(StatusCode::ACCEPTED)  // 202 - processing started
}
```

---

## 🤖 AI Prompt (System Message)

```rust
const SYSTEM_PROMPT: &str = r#"
Ты профессиональный шеф-технолог с 20-летним опытом.

ЗАДАЧА:
Проанализируй рецепт и составь детальную технологическую карту.

ПРАВИЛА:
1. Верни ТОЛЬКО JSON без комментариев
2. Следуй СТРОГО указанной схеме
3. Если блюдо невозможно приготовить - укажи errors и низкий feasibility_score
4. НЕ выдумывай ингредиенты - используй только те, что даны
5. В missing/suggestions можешь предложить недостающие ингредиенты
6. Критические точки контроля (CCP) - это моменты, где легко ошибиться
7. Время должно быть реалистичным (торт не выпекается за 5 минут)

СТРУКТУРА:
- steps: разбей приготовление на логические этапы
- validation: найди ошибки и предупреждения
- suggestions: предложи исправления и замены
- feasibility_score: 0-100 (100 = идеальный рецепт, 0 = невозможно)

ЯЗЫК:
Отвечай на языке: {language}
"#;

fn build_prompt(recipe: &Recipe, validation: &ValidationResult, language: &str) -> String {
    format!(
        r#"{system_prompt}

РЕЦЕПТ:
Название: {name}
Инструкции: {instructions}
Порций: {servings}

ИНГРЕДИЕНТЫ:
{ingredients}

ПРЕДВАРИТЕЛЬНАЯ ВАЛИДАЦИЯ:
Ошибки: {errors}
Предупреждения: {warnings}
Недостающие роли: {missing}

Верни JSON по схеме:
{{
  "steps": [{{ "n": 1, "title": "...", "details": ["..."], "time_min": 10, "temp_c": null, "ccp": ["..."] }}],
  "validation": {{
    "errors": [{{ "code": "CODE", "message": "..." }}],
    "warnings": [{{ "code": "CODE", "message": "..." }}],
    "missing": [{{ "role": "role_name", "examples": ["..."] }}]
  }},
  "suggestions": {{
    "fixes": [{{ "title": "...", "changes": ["..."] }}],
    "substitutions": [{{ "ingredient": "...", "options": ["..."], "note": "..." }}]
  }},
  "feasibility_score": 85
}}
"#,
        system_prompt = SYSTEM_PROMPT,
        name = recipe.name_default,
        instructions = recipe.instructions_default,
        servings = recipe.servings,
        ingredients = format_ingredients(&recipe.ingredients),
        errors = format_errors(&validation.errors),
        warnings = format_warnings(&validation.warnings),
        missing = format_missing(&validation.missing_roles)
    )
}
```

---

## 🛡️ Контроль качества

### 1. Rule-based валидация (ПЕРЕД AI)

```rust
// src/application/recipe_v2_validator.rs
pub struct RecipeValidator {
    ingredient_roles: IngredientRoleService,
}

impl RecipeValidator {
    pub async fn validate(&self, recipe: &Recipe) -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut missing_roles = Vec::new();
        
        // Анализ ролей ингредиентов
        let roles = self.analyze_ingredient_roles(&recipe.ingredients).await;
        
        // Проверка по типу блюда (из названия)
        let dish_type = self.detect_dish_type(&recipe.name_default);
        
        match dish_type {
            DishType::Cake | DishType::Pie | DishType::Bread => {
                // Нужны: base + binder + leavening
                if !roles.contains(&IngredientRole::Binder) {
                    errors.push(ValidationError {
                        code: "MISSING_BINDER".to_string(),
                        message: format!("Для {} нет связующего компонента", dish_type),
                    });
                    missing_roles.push(MissingRole {
                        role: "binder",
                        examples: vec!["яйцо", "желатин", "крахмал"],
                    });
                }
                
                if !roles.contains(&IngredientRole::Leavening) {
                    warnings.push(ValidationWarning {
                        code: "MISSING_LEAVENING".to_string(),
                        message: "Нет разрыхлителя - тесто может не подняться".to_string(),
                    });
                }
            },
            DishType::Dessert => {
                if !roles.contains(&IngredientRole::Sweetener) {
                    warnings.push(ValidationWarning {
                        code: "NO_SWEET".to_string(),
                        message: "Для десерта нет подсластителя".to_string(),
                    });
                }
            },
            _ => {}
        }
        
        // Проверка термической обработки
        if self.requires_thermal(&dish_type) && !self.has_thermal_step(&recipe.instructions_default) {
            errors.push(ValidationError {
                code: "NO_THERMAL_STEP".to_string(),
                message: "В инструкциях нет термической обработки".to_string(),
            });
        }
        
        ValidationResult { errors, warnings, missing_roles }
    }
}
```

### 2. AI не добавляет ингредиенты

В prompt явно указываем:
```
НЕ добавляй ингредиенты в steps!
Используй ТОЛЬКО те ингредиенты, что перечислены.
Если чего-то не хватает - укажи в missing.
```

### 3. Проверка ответа AI

```rust
fn validate_ai_response(response: &str) -> Result<AIInsights, AppError> {
    // 1. Попытка парсинга JSON
    let insights: AIInsights = serde_json::from_str(response)
        .map_err(|e| AppError::internal(format!("AI returned invalid JSON: {}", e)))?;
    
    // 2. Проверка обязательных полей
    if insights.steps.is_empty() {
        return Err(AppError::internal("AI returned empty steps"));
    }
    
    // 3. Проверка feasibility_score в диапазоне
    if insights.feasibility_score < 0 || insights.feasibility_score > 100 {
        return Err(AppError::internal("Invalid feasibility_score"));
    }
    
    // 4. Проверка времени (не может быть отрицательным)
    for step in &insights.steps {
        if step.time_min < 0 {
            return Err(AppError::internal("Negative time in step"));
        }
    }
    
    Ok(insights)
}
```

---

## 📊 Performance & Caching

- **Rule-based validation**: ~5ms (синхронно, перед сохранением)
- **AI generation**: ~2-3s (асинхронно, после сохранения)
- **Translation**: ~1-2s per language (параллельно)
- **Кэширование**: в БД `recipe_ai_insights`, 1 запись на (recipe_id, language)

---

## 🚀 Endpoints (MVP)

```
POST   /api/recipes/v2                     # Создать рецепт + запустить AI в фоне
GET    /api/recipes/v2/:id                 # Получить рецепт
GET    /api/recipes/v2/:id/insights?lang=ru # Получить AI insights (202 если генерируются)
POST   /api/recipes/v2/:id/insights/refresh # Перегенерировать insights
POST   /api/recipes/v2/:id/publish         # Опубликовать (is_public=true)
GET    /api/recipes/v2/public?lang=pl&q=... # Поиск по публичным рецептам
```

---

## ✅ Чеклист реализации

### Phase 1: Структуры данных
- [ ] Создать `src/domain/recipe_ai_insights_v2.rs` с новой схемой
- [ ] Создать `src/domain/ingredient_roles.rs` с keyword map
- [ ] Обновить миграцию под новую JSON схему

### Phase 2: Валидатор
- [ ] Создать `src/application/recipe_v2_validator.rs`
- [ ] Реализовать детекцию типа блюда
- [ ] Реализовать анализ ролей ингредиентов
- [ ] Написать правила валидации (5-7 основных)

### Phase 3: AI Service
- [ ] Обновить Groq prompt под новую схему
- [ ] Добавить integration с validator
- [ ] Реализовать перевод insights

### Phase 4: Orchestrator
- [ ] Создать `recipe_v2_insights_orchestrator.rs`
- [ ] Координировать validator → AI → translation
- [ ] Асинхронная генерация после создания рецепта

### Phase 5: HTTP API
- [ ] Обновить `POST /api/recipes/v2` - запускать AI в фоне
- [ ] Добавить `GET /api/recipes/v2/:id/insights`
- [ ] Добавить `POST /api/recipes/v2/:id/insights/refresh`

### Phase 6: Testing
- [ ] Unit tests для validator
- [ ] Integration tests для AI service
- [ ] E2E test: create recipe → wait → get insights

---

## 📝 Примеры использования

### Frontend: Получение insights

```typescript
// 1. Создать рецепт
const recipe = await api.post('/api/recipes/v2', recipeData);

// 2. Сразу показать рецепт, insights генерируются в фоне
showRecipe(recipe);

// 3. Polling для проверки готовности insights
const checkInsights = setInterval(async () => {
  try {
    const insights = await api.get(`/api/recipes/v2/${recipe.id}/insights?lang=ru`);
    clearInterval(checkInsights);
    showAIInsights(insights);
  } catch (error) {
    if (error.status !== 404) {
      clearInterval(checkInsights);
    }
  }
}, 2000);
```

### Отображение на фронте

```tsx
<div className="ai-insights">
  {/* Технология */}
  <section>
    <h3>Технология приготовления</h3>
    {insights.steps.map(step => (
      <div key={step.n} className="step">
        <h4>Шаг {step.n}: {step.title}</h4>
        <ul>
          {step.details.map(d => <li>{d}</li>)}
        </ul>
        <div className="meta">
          {step.time_min && <span>⏱️ {step.time_min} мин</span>}
          {step.temp_c && <span>🌡️ {step.temp_c}°C</span>}
        </div>
        {step.ccp.length > 0 && (
          <div className="ccp">
            <strong>⚠️ Критические точки контроля:</strong>
            {step.ccp.map(c => <span>{c}</span>)}
          </div>
        )}
      </div>
    ))}
  </section>

  {/* Валидация */}
  {insights.validation.errors.length > 0 && (
    <section className="errors">
      <h3>❌ Ошибки</h3>
      {insights.validation.errors.map(e => (
        <div className="error">{e.message}</div>
      ))}
    </section>
  )}

  {/* Советы */}
  {insights.suggestions.fixes.length > 0 && (
    <section className="suggestions">
      <h3>💡 Рекомендации</h3>
      {insights.suggestions.fixes.map(fix => (
        <div>
          <strong>{fix.title}</strong>
          <ul>{fix.changes.map(c => <li>{c}</li>)}</ul>
        </div>
      ))}
    </section>
  )}

  {/* Оценка */}
  <div className="feasibility">
    <h3>Реалистичность рецепта</h3>
    <progress value={insights.feasibility_score} max="100" />
    <span>{insights.feasibility_score}/100</span>
  </div>
</div>
```

---

## 🎓 Почему это профессионально

1. **Строгий контракт**: JSON Schema гарантирует структуру
2. **Rule-based pre-validation**: Быстрая проверка без AI
3. **Критические точки контроля (CCP)**: HACCP-подход из пищевой промышленности
4. **Feasibility score**: Числовая оценка реалистичности
5. **Разделение concerns**: Validator → AI → Translation
6. **Асинхронность**: UI не ждёт AI (2-3 секунды)
7. **Переводы**: Insights доступны на всех языках
8. **Защита от фантазий AI**: Rule-based проверка + строгий prompt

---

**Автор**: Recipe AI Insights V2 Architecture
**Дата**: 2026-02-16
**Статус**: Ready for implementation
