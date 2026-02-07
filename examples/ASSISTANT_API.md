# 🧭 Guided Assistant API

## Концепция

**Guided Assistant** — это state machine, который управляет UX и ведёт пользователя через процесс настройки ресторана.

### Философия 2026

- ✅ **Backend управляет UX** — решает что показывать и когда
- ✅ **Frontend = dumb renderer** — просто рисует UI по данным
- ✅ **Type-safe контракт** — никаких магических строк
- ✅ **Невозможные переходы игнорируются** — безопасность из коробки

---

## 🔄 Flow (State Machine)

```
Start
 ↓ start_inventory
InventorySetup   (Добавить продукты)
 ↓ finish_inventory
RecipeSetup      (Создать рецепты)
 ↓ finish_recipes
DishSetup        (Создать блюда)
 ↓ finish_dishes
Report           (Показать отчёт)
 ↓ view_report
Completed
```

---

## 📡 API Endpoints

### `GET /api/assistant/state`

Получить начальное состояние (всегда возвращает `Start`).

**Response:**
```json
{
  "message": "Добро пожаловать! Давай начнём с добавления продуктов.",
  "actions": [
    { "id": "start_inventory", "label": "📦 Добавить продукты" }
  ],
  "step": "Start",
  "progress": 0
}
```

### `POST /api/assistant/command`

Выполнить действие и получить новое состояние.

**Request:**
```json
{
  "step": "Start",
  "command": "start_inventory"
}
```

**Response:**
```json
{
  "message": "Добавь продукты на склад.",
  "actions": [
    { "id": "add_product", "label": "➕ Добавить продукт" },
    { "id": "finish_inventory", "label": "➡️ Перейти к рецептам" }
  ],
  "step": "InventorySetup",
  "progress": 25
}
```

---

## 🧪 Примеры использования

### 1️⃣ Получить начальное состояние

```bash
curl http://localhost:8080/api/assistant/state
```

### 2️⃣ Начать добавление продуктов

```bash
curl -X POST http://localhost:8080/api/assistant/command \
  -H "Content-Type: application/json" \
  -d '{
    "step": "Start",
    "command": "start_inventory"
  }'
```

### 3️⃣ Перейти к рецептам

```bash
curl -X POST http://localhost:8080/api/assistant/command \
  -H "Content-Type: application/json" \
  -d '{
    "step": "InventorySetup",
    "command": "finish_inventory"
  }'
```

### 4️⃣ Полный flow

```bash
# Start → InventorySetup
curl -X POST http://localhost:8080/api/assistant/command \
  -H "Content-Type: application/json" \
  -d '{"step": "Start", "command": "start_inventory"}'

# InventorySetup → RecipeSetup
curl -X POST http://localhost:8080/api/assistant/command \
  -H "Content-Type: application/json" \
  -d '{"step": "InventorySetup", "command": "finish_inventory"}'

# RecipeSetup → DishSetup
curl -X POST http://localhost:8080/api/assistant/command \
  -H "Content-Type: application/json" \
  -d '{"step": "RecipeSetup", "command": "finish_recipes"}'

# DishSetup → Report
curl -X POST http://localhost:8080/api/assistant/command \
  -H "Content-Type: application/json" \
  -d '{"step": "DishSetup", "command": "finish_dishes"}'

# Report → Completed
curl -X POST http://localhost:8080/api/assistant/command \
  -H "Content-Type: application/json" \
  -d '{"step": "Report", "command": "view_report"}'
```

---

## 🛡️ Безопасность переходов

Невалидные команды **игнорируются** — состояние не меняется:

```bash
# Попытка перепрыгнуть шаги
curl -X POST http://localhost:8080/api/assistant/command \
  -H "Content-Type: application/json" \
  -d '{"step": "Start", "command": "finish_recipes"}'

# Результат: step остаётся "Start"
```

---

## 📊 Состояния и прогресс

| Step | Progress | Описание |
|------|----------|----------|
| `Start` | 0% | Добро пожаловать |
| `InventorySetup` | 25% | Добавление продуктов |
| `RecipeSetup` | 50% | Создание рецептов |
| `DishSetup` | 75% | Создание блюд |
| `Report` | 100% | Отчёт готов |
| `Completed` | 100% | Завершено |

---

## 🎯 Команды (Actions)

| Command | Доступен на шаге | Переход |
|---------|------------------|---------|
| `start_inventory` | Start | → InventorySetup |
| `add_product` | InventorySetup | (не меняет step) |
| `finish_inventory` | InventorySetup | → RecipeSetup |
| `create_recipe` | RecipeSetup | (не меняет step) |
| `finish_recipes` | RecipeSetup | → DishSetup |
| `create_dish` | DishSetup | (не меняет step) |
| `finish_dishes` | DishSetup | → Report |
| `view_report` | Report | → Completed |

---

## 🧱 DDD Architecture

```
src/domain/assistant/
 ├── step.rs        # AssistantStep enum (состояния)
 ├── command.rs     # AssistantCommand enum (действия)
 ├── response.rs    # AssistantResponse (контракт UI)
 └── rules.rs       # next_step() — правила переходов

src/application/
 └── assistant_service.rs  # AssistantService (бизнес-логика)

src/interfaces/http/
 └── assistant.rs   # HTTP handlers
```

---

## 🚀 Что дальше?

### Фаза 2: Интеграция с реальными доменами

Сейчас команды `add_product`, `create_recipe`, `create_dish` ничего не делают.

**Следующий шаг:** подключить реальные домены:
- `add_product` → вызов `InventoryService::add_product()`
- `create_recipe` → вызов `RecipeService::create_recipe()`
- `create_dish` → вызов `MenuService::create_dish()`

### Фаза 3: Персистентность состояния

Сохранять `current_step` для каждого tenant в БД, чтобы пользователь мог вернуться позже.

### Фаза 4: AI Enhancement

Добавить LLM для генерации динамических подсказок на каждом шаге.

---

## ✅ Проверено

- ✅ Все переходы работают корректно
- ✅ Невалидные команды игнорируются
- ✅ Progress корректно рассчитывается
- ✅ JSON контракт стабилен
- ✅ Type-safe на всех уровнях
