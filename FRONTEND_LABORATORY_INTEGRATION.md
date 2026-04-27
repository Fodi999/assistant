# Laboratory — Frontend Integration Contract

> Цель: фронт может построить экран "лаборатории" уже сегодня, без AI.
> Backend (Steps 1–7) уже отдаёт всё для нижней зоны.

## 1. Раскладка экрана (3 зоны)

```
┌────────────────────────────────────────────────────┐
│ ВЕРХНЯЯ — AI Copilot (вход идеи продукта)         │ ← Step 8 (позже)
│  • input "хочу клубничный соус без сахара"        │
│  • выдаёт черновик: ingredients[] + steps[]       │
├────────────────────────────────────────────────────┤
│ СРЕДНЯЯ — Конструктор                              │ ← уже работает
│  ┌──────────────┐  ┌────────────────────────────┐ │
│  │ Ingredients  │  │ Process steps              │ │
│  │ +slug, qty   │  │ technique, t°, мин         │ │
│  └──────────────┘  └────────────────────────────┘ │
├────────────────────────────────────────────────────┤
│ НИЖНЯЯ — Анализ (latest_analysis)                  │ ← уже работает
│  [What happens]   [Flavor]                         │
│  [Shelf life]     [Suggestions]   [Warnings]       │
└────────────────────────────────────────────────────┘
```

## 2. Эндпоинты (все под JWT, `Authorization: Bearer …`)

| Method | Path | Зачем |
|---|---|---|
| POST | `/api/laboratory/projects` | создать проект |
| GET | `/api/laboratory/projects` | список |
| GET | `/api/laboratory/projects/:id` | проект целиком |
| PATCH | `/api/laboratory/projects/:id` | name/desc/status |
| DELETE | `/api/laboratory/projects/:id` | удалить |
| POST | `/api/laboratory/projects/:id/ingredients` | добавить ингредиент |
| PATCH | `/api/laboratory/projects/:id/ingredients/:ing_id` | qty/role |
| DELETE | `/api/laboratory/projects/:id/ingredients/:ing_id` | удалить |
| POST | `/api/laboratory/projects/:id/steps` | добавить шаг |
| PATCH | `/api/laboratory/projects/:id/steps/:step_id` | t°/duration |
| DELETE | `/api/laboratory/projects/:id/steps/:step_id` | удалить |
| **POST** | **`/api/laboratory/projects/:id/analyze?lang=ru`** | **запустить анализ** |

`?lang=` ∈ `en|pl|uk|ru` — язык сообщений в effects/recommendations/warnings.

## 3. JSON-схема ответа `/analyze` (и `GET /:id`)

Всё, что нужно для нижней зоны, лежит в `latest_analysis`:

```ts
type LabProject = {
  id: string;
  name: string;
  description: string | null;
  target_product_type: string | null;   // "sauce" | "drink" | "dessert" | …
  status: "draft" | "ready" | "archived";
  ingredients: LabIngredient[];
  process_steps: LabStep[];
  latest_analysis: LabAnalysis | null;
};

type LabIngredient = {
  id: string;
  ingredient_slug: string;
  quantity: string;          // Decimal as string
  unit: string;              // "g" | "ml" | "pcs"
  role: string | null;       // "base" | "acid" | "fat" | …
  sort_order: number;
  notes: string | null;
};

type LabStep = {
  id: string;
  order_index: number;
  technique: string;         // "heat" | "blend" | "ferment" | …
  temperature_c: string | null;   // Decimal as string
  duration_min: number | null;
  target_slugs: string[];    // на какие ингредиенты воздействует
  notes: string | null;
};

type LabAnalysis = {
  id: string;
  shelf_life_days: number | null;
  estimated_cost: string | null;        // зарезервировано
  complexity_score: number | null;      // зарезервировано
  risk_level: "low" | "medium" | "high" | "critical";
  texture_result: unknown | null;       // зарезервировано (Step 9+)
  flavor_result: FlavorResult;          // см. ниже
  nutrition_result: unknown;            // {} пока (Step 9+)
  process_effects: ProcessEffects;      // см. ниже
  storage_recommendations: StorageRec[];
  pairing_suggestions: Pairing[];
  warnings: Warning[];
};
```

### 3.1 Зона "Что происходит с продуктом" — `process_effects`

```ts
type ProcessEffects = {
  step_effects: StepEffect[];
  global_effects: Effect[];   // эффекты на уровне всего проекта
};

type StepEffect = {
  step_id: string;
  order_index: number;
  technique: string;
  temperature_c: number | null;
  duration_min: number | null;
  effects: Effect[];
};

type Effect = {
  ingredient_slug: string;
  ingredient_name: string;          // локализовано под ?lang
  effect_type: string;              // "softening" | "moisture_release" | "denaturation" | "maillard" | …
  visual_token: string;             // "soften" | "juice_release" | "browning" | …  ← для иконок
  label: string;                    // короткий заголовок ("размягчение")
  message: string;                  // полное предложение для UI
  intensity: number;                // 0..1 — для прогресс-бара
  confidence: number;               // 0..1
  trigger_temperature_c: number | null;
  actual_temperature_c: number | null;
};
```

**UI-подсказка**: `visual_token` — стабильный enum для маппинга на иконку/анимацию.
`intensity` → ширина бара, `confidence` → opacity или "≈".

### 3.2 Зона "Вкус" — `flavor_result`

```ts
type FlavorResult = {
  sweetness: number | null;     // 0..10
  acidity:   number | null;
  bitterness: number | null;
  umami:     number | null;
  aroma:     number | null;
  dominant_profile:
    | "sweet_sour" | "sweet" | "acidic" | "umami_rich"
    | "bitter"    | "aromatic" | "balanced" | "unknown";
  balance_label: string;        // локализовано ("Сладко-кислый профиль")
  message: string;              // 1 предложение для подписи
};
```

**UI-подсказка**: 5 баров (sweet/acid/bitter/umami/aroma) + бейдж `dominant_profile`.

### 3.3 Зона "Срок хранения"

```ts
shelf_life_days: number | null;     // 0..30
risk_level: "low" | "medium" | "high" | "critical";

type StorageRec = {
  method: "refrigeration" | "freezing" | "pantry" | "pasteurization_advisory";
  label: string;                    // "Хранить при 0–4°C"
  message: string;                  // полное объяснение
  extra_days: number | null;        // +N дней к базовому сроку
  cost_impact:    "low" | "medium" | "high";
  quality_impact: "low" | "medium" | "high";
};
```

**UI-подсказка**: бейдж `risk_level` с цветом (green/yellow/orange/red),
большая цифра `shelf_life_days`, ниже — список карточек `StorageRec`.

### 3.4 Зона "Что добавить" — `pairing_suggestions`

```ts
type Pairing = {
  ingredient_slug: string;       // "cream", "almond", …
  ingredient_name: string;       // если slug есть в catalog — локализованное имя; иначе сам slug
  score: number;                 // 0..100
  reason: string;                // "Хорошо сочетается с «Абрикос» (pairs_with_dairy)"
  source: string;                // "culinary_behavior" | …
};
```

**UI-подсказка**: горизонтальный скролл "чипов" с `+` — клик по чипу
делает `POST /ingredients` с этим slug. Пары, уже добавленные в проект, движок отфильтрует сам.

### 3.5 Зона "Предупреждения"

```ts
type Warning = {
  kind: string;          // "high_risk_ph_aw" | "protein_unsafe_heat" | "no_profile_data" | …
  severity: "info" | "warning" | "high" | "critical";
  message: string;       // локализовано
  ingredient_slug?: string;
  step_id?: string;
};
```

**UI-подсказка**: цветной банер сверху нижней зоны, иконка по `severity`.

## 4. Реальный пример (apricot 200 g, heat 75 °C / 10 min, lang=ru)

```jsonc
"latest_analysis": {
  "shelf_life_days": 9,
  "risk_level": "low",
  "flavor_result": {
    "sweetness": 7.0, "acidity": 5.0, "bitterness": 1.0,
    "umami": 1.0, "aroma": 8.0,
    "dominant_profile": "sweet_sour",
    "balance_label": "Сладко-кислый профиль",
    "message": "Доминирует сладко-кислый баланс (сладость 7.0, кислотность 5.0). Хорош для соусов и десертов."
  },
  "process_effects": {
    "global_effects": [],
    "step_effects": [{
      "step_id": "…", "order_index": 0,
      "technique": "heat", "temperature_c": 75.0, "duration_min": 10,
      "effects": [
        { "effect_type": "softening", "visual_token": "soften",
          "ingredient_slug": "apricot", "ingredient_name": "Абрикос",
          "label": "размягчение", "intensity": 0.9, "confidence": 0.7,
          "trigger_temperature_c": 70.0, "actual_temperature_c": 75.0,
          "message": "Абрикос: эффект 'размягчение' активируется при 70°C (текущая 75°C)." },
        { "effect_type": "moisture_release", "visual_token": "juice_release",
          "ingredient_slug": "apricot", "ingredient_name": "Абрикос",
          "label": "выделение сока", "intensity": 0.7, "confidence": 0.7,
          "trigger_temperature_c": 60.0, "actual_temperature_c": 75.0,
          "message": "Абрикос: эффект 'выделение сока' активируется при 60°C (текущая 75°C)." }
      ]
    }]
  },
  "storage_recommendations": [{
    "method": "refrigeration",
    "label": "Хранить при 0–4°C",
    "message": "После обработки быстро охладить и хранить в холодильнике.",
    "extra_days": null, "cost_impact": "low", "quality_impact": "low"
  }],
  "pairing_suggestions": [
    { "ingredient_slug": "almond",     "ingredient_name": "almond",     "score": 90.0, "reason": "Хорошо сочетается с «Абрикос» (pairs_with_nuts)",  "source": "culinary_behavior" },
    { "ingredient_slug": "cream",      "ingredient_name": "cream",      "score": 90.0, "reason": "Хорошо сочетается с «Абрикос» (pairs_with_dairy)", "source": "culinary_behavior" },
    { "ingredient_slug": "mascarpone", "ingredient_name": "mascarpone", "score": 90.0, "reason": "Хорошо сочетается с «Абрикос» (pairs_with_dairy)", "source": "culinary_behavior" },
    { "ingredient_slug": "pistachio",  "ingredient_name": "pistachio",  "score": 90.0, "reason": "Хорошо сочетается с «Абрикос» (pairs_with_nuts)",  "source": "culinary_behavior" },
    { "ingredient_slug": "ricotta",    "ingredient_name": "ricotta",    "score": 90.0, "reason": "Хорошо сочетается с «Абрикос» (pairs_with_dairy)", "source": "culinary_behavior" },
    { "ingredient_slug": "walnut",     "ingredient_name": "walnut",     "score": 90.0, "reason": "Хорошо сочетается с «Абрикос» (pairs_with_nuts)",  "source": "culinary_behavior" },
    { "ingredient_slug": "yogurt",     "ingredient_name": "yogurt",     "score": 90.0, "reason": "Хорошо сочетается с «Абрикос» (pairs_with_dairy)", "source": "culinary_behavior" }
  ],
  "warnings": []
}
```

## 5. Рекомендованный поток фронта

```
1. user открывает /lab/:projectId
   GET /api/laboratory/projects/:id        → отрисовать средняя+нижняя зоны
2. user редактирует ingredients/steps
   POST/PATCH/DELETE …                     → обновить только среднюю зону
3. user жмёт "Анализировать"
   POST /api/laboratory/projects/:id/analyze?lang=<userLang>
   → ответ содержит весь project + новый latest_analysis
   → перерисовать только нижнюю зону
```

**Debounce**: не запускать `/analyze` автоматически — это запись в БД; только по клику.

## 6. Что появится позже (контракт уже зарезервирован)

| Поле | Шаг | Что добавит |
|---|---|---|
| `nutrition_result` | Step 9 | калории/БЖУ на 100 г и на проект |
| `texture_result`   | Step 10 | вязкость, структура |
| `estimated_cost`   | Step 11 | себестоимость по складу |
| `complexity_score` | Step 12 | сложность 1–10 |
| AI Copilot         | Step 8  | новый эндпоинт `POST /api/laboratory/copilot/suggest` → возвращает черновик `ingredients[]` + `steps[]`, фронт его принимает в среднюю зону |

Эти поля уже в схеме `latest_analysis`, фронту достаточно отрендерить их условно
(`if (a.nutrition_result?.calories_per_100g) …`).

## 7. Заголовки и ошибки

- Все эндпоинты возвращают `application/json`.
- Ошибки: `{"error": "...", "code": "..."}` с HTTP 4xx/5xx.
- 401 — токен истёк → редирект на login.
- 404 — проект не принадлежит пользователю.
- 422 — валидация (например, `quantity <= 0`).
