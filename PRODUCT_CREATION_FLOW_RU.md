# 📝 Как работает добавление продукта в каталог

## 🎯 Основная идея

Когда пользователь добавляет продукт, он может:
1. **Ввести только английское название** → система автоматически заполнит остальные 3 языка
2. **Ввести названия на всех языках** → система сохранит их как есть
3. **Ввести частично** → пустые поля заполнятся английским названием

---

## 🔄 Процесс создания продукта

### Шаг 1: Пользователь заполняет форму

```typescript
// Пример 1: Только английское название
{
  "name_en": "Pineapple",      // ✅ Обязательно
  "category_id": "uuid",        // ✅ Обязательно
  "unit": "piece"               // ✅ Обязательно
}

// Пример 2: С переводами
{
  "name_en": "Pineapple",
  "name_pl": "Ananas",          // Польский
  "name_uk": "Ананас",          // Украинский
  "name_ru": "Ананас",          // Русский
  "category_id": "uuid",
  "unit": "piece",
  "description": "Fresh tropical fruit"
}

// Пример 3: Частичный перевод
{
  "name_en": "Pineapple",
  "name_pl": "",                // Пусто → заполнится "Pineapple"
  "name_uk": "Ананас",          // Есть перевод
  "name_ru": "",                // Пусто → заполнится "Pineapple"
  "category_id": "uuid",
  "unit": "piece"
}
```

### Шаг 2: Backend обрабатывает запрос

**Код на Rust (backend):**

```rust
// src/services/admin_catalog_service.rs

pub async fn create_product(
    &self,
    req: CreateProductRequest,
) -> Result<ProductResponse> {
    // 🎯 Автозаполнение: если поле пустое → берём английское название
    let name_pl = if req.name_pl.as_ref().map_or(true, |s| s.trim().is_empty()) {
        req.name_en.clone()  // ← Копируем английское
    } else {
        req.name_pl.unwrap()
    };

    let name_uk = if req.name_uk.as_ref().map_or(true, |s| s.trim().is_empty()) {
        req.name_en.clone()  // ← Копируем английское
    } else {
        req.name_uk.unwrap()
    };

    let name_ru = if req.name_ru.as_ref().map_or(true, |s| s.trim().is_empty()) {
        req.name_en.clone()  // ← Копируем английское
    } else {
        req.name_ru.unwrap()
    };

    // Сохраняем в БД
    let query = r#"
        INSERT INTO catalog_ingredients 
        (id, name_en, name_pl, name_uk, name_ru, category_id, unit, description)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING *
    "#;

    sqlx::query_as::<_, ProductRow>(query)
        .bind(Uuid::new_v4())
        .bind(&req.name_en)    // English
        .bind(&name_pl)        // Polish (или English если пусто)
        .bind(&name_uk)        // Ukrainian (или English если пусто)
        .bind(&name_ru)        // Russian (или English если пусто)
        .bind(&req.category_id)
        .bind(&req.unit)
        .bind(&req.description)
        .fetch_one(&self.pool)
        .await?;

    Ok(product)
}
```

### Шаг 3: База данных PostgreSQL

```sql
-- Таблица catalog_ingredients
CREATE TABLE catalog_ingredients (
    id UUID PRIMARY KEY,
    name_en VARCHAR(255) NOT NULL,  -- Английский (обязательно)
    name_pl VARCHAR(255) NOT NULL,  -- Польский
    name_uk VARCHAR(255) NOT NULL,  -- Украинский
    name_ru VARCHAR(255) NOT NULL,  -- Русский
    category_id UUID NOT NULL,
    unit VARCHAR(50) NOT NULL,
    description TEXT,
    image_url TEXT,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Уникальность: case-insensitive по английскому названию
CREATE UNIQUE INDEX idx_catalog_name_en_unique 
ON catalog_ingredients (LOWER(name_en)) 
WHERE is_active = true;
```

---

## 🎬 Реальный пример

### Тест 1: Только английское название

**Запрос:**
```bash
POST /api/admin/products
{
  "name_en": "Pineapple",
  "category_id": "d4a64b25-a187-4ec0-9518-3e8954a138fa",
  "unit": "piece"
}
```

**Ответ:**
```json
{
  "id": "fb52875b-7947-4089-a84c-23d88cfbe2b5",
  "name_en": "Pineapple",
  "name_pl": "Pineapple",  ← Автозаполнено!
  "name_uk": "Pineapple",  ← Автозаполнено!
  "name_ru": "Pineapple",  ← Автозаполнено!
  "category_id": "d4a64b25-a187-4ec0-9518-3e8954a138fa",
  "unit": "piece",
  "description": null,
  "image_url": null
}
```

### Тест 2: С полными переводами

**Запрос:**
```bash
POST /api/admin/products
{
  "name_en": "Artichoke",
  "name_pl": "Karczoch",
  "name_uk": "Артишок",
  "name_ru": "Артишок",
  "category_id": "5a841ce0-2ea5-4230-a1f7-011fa445afdc",
  "unit": "piece",
  "description": "Fresh artichoke"
}
```

**Ответ:**
```json
{
  "id": "1c642a85-4866-411d-90c6-265b826a3981",
  "name_en": "Artichoke",
  "name_pl": "Karczoch",   ← Сохранен перевод
  "name_uk": "Артишок",    ← Сохранен перевод
  "name_ru": "Артишок",    ← Сохранен перевод
  "category_id": "5a841ce0-2ea5-4230-a1f7-011fa445afdc",
  "unit": "piece",
  "description": "Fresh artichoke",
  "image_url": null
}
```

---

## 🎨 Интерфейс на фронтенде

### Форма создания продукта

```tsx
function ProductCreateForm() {
  const [formData, setFormData] = useState({
    name_en: '',      // Обязательное
    name_pl: '',      // Опциональное
    name_uk: '',      // Опциональное
    name_ru: '',      // Опциональное
    category_id: '',
    unit: 'kilogram',
    description: ''
  });

  return (
    <form onSubmit={handleSubmit}>
      {/* Английское название - ОБЯЗАТЕЛЬНО */}
      <div>
        <label>🇬🇧 Name (English) *</label>
        <input
          type="text"
          value={formData.name_en}
          onChange={e => setFormData({...formData, name_en: e.target.value})}
          required
          placeholder="Enter product name in English"
        />
        <small>⚠️ Required field. Will be used for other languages if left empty.</small>
      </div>

      {/* Польский - ОПЦИОНАЛЬНО */}
      <div>
        <label>🇵🇱 Name (Polish)</label>
        <input
          type="text"
          value={formData.name_pl}
          onChange={e => setFormData({...formData, name_pl: e.target.value})}
          placeholder="Leave empty to use English name"
        />
        <small>💡 Empty = will use "{formData.name_en || 'English name'}"</small>
      </div>

      {/* Украинский - ОПЦИОНАЛЬНО */}
      <div>
        <label>🇺🇦 Name (Ukrainian)</label>
        <input
          type="text"
          value={formData.name_uk}
          onChange={e => setFormData({...formData, name_uk: e.target.value})}
          placeholder="Leave empty to use English name"
        />
        <small>💡 Empty = will use "{formData.name_en || 'English name'}"</small>
      </div>

      {/* Русский - ОПЦИОНАЛЬНО */}
      <div>
        <label>🇷🇺 Name (Russian)</label>
        <input
          type="text"
          value={formData.name_ru}
          onChange={e => setFormData({...formData, name_ru: e.target.value})}
          placeholder="Leave empty to use English name"
        />
        <small>💡 Empty = will use "{formData.name_en || 'English name'}"</small>
      </div>

      {/* Остальные поля... */}
      <button type="submit">Create Product</button>
    </form>
  );
}
```

---

## 📊 Схема работы

```
┌─────────────────────────────────────────────────────────────┐
│ 1. Пользователь заполняет форму                             │
│    name_en: "Pineapple" ✓                                   │
│    name_pl: [пусто]                                         │
│    name_uk: [пусто]                                         │
│    name_ru: [пусто]                                         │
└────────────────┬────────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────────┐
│ 2. Frontend отправляет на backend                           │
│    POST /api/admin/products                                 │
│    { name_en: "Pineapple", category_id: "...", unit: "..." }│
└────────────────┬────────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────────┐
│ 3. Backend проверяет и заполняет пустые поля                │
│    name_en: "Pineapple" ✓ (обязательно)                     │
│    name_pl: "" → "Pineapple" ✓ (автозаполнение)            │
│    name_uk: "" → "Pineapple" ✓ (автозаполнение)            │
│    name_ru: "" → "Pineapple" ✓ (автозаполнение)            │
└────────────────┬────────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────────┐
│ 4. Сохранение в PostgreSQL                                  │
│    INSERT INTO catalog_ingredients                          │
│    (name_en, name_pl, name_uk, name_ru, ...)               │
│    VALUES ('Pineapple', 'Pineapple', 'Pineapple',          │
│            'Pineapple', ...)                                │
└────────────────┬────────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────────┐
│ 5. Ответ клиенту                                            │
│    {                                                        │
│      "id": "uuid",                                          │
│      "name_en": "Pineapple",                                │
│      "name_pl": "Pineapple",                                │
│      "name_uk": "Pineapple",                                │
│      "name_ru": "Pineapple"                                 │
│    }                                                        │
└─────────────────────────────────────────────────────────────┘
```

---

## 💡 Преимущества этого подхода

### 1. Быстрое добавление продуктов
✅ Админ может быстро добавить 100 продуктов, вводя только английские названия
✅ Переводы можно добавить позже через редактирование

### 2. Гибкость
✅ Если есть перевод → вводим его сразу
✅ Если нет перевода → система автоматически подставит английский

### 3. Целостность данных
✅ Все 4 языка ВСЕГДА заполнены в БД
✅ Никогда не будет NULL или пустых строк
✅ Приложение может безопасно отображать продукт на любом языке

### 4. Легко редактировать
✅ Позже можно обновить только один язык:
```bash
PUT /api/admin/products/:id
{
  "name_pl": "Ananas"  # Только польский
}
# Остальные языки останутся без изменений
```

---

## 🔄 Workflow для администратора

### Сценарий 1: Быстрое добавление
```
1. Админ видит: нужно добавить 50 новых продуктов
2. Открывает Excel, копирует английские названия
3. В админке быстро создаёт продукты (только name_en)
4. Система автоматически заполняет все языки английским
5. Готово! Продукты доступны во всех ресторанах
6. Позже переводчик добавит переводы через редактирование
```

### Сценарий 2: С переводами сразу
```
1. Админ создаёт продукт с полными переводами
2. Вводит все 4 названия вручную
3. Система сохраняет как есть
4. Готово! Продукт сразу с качественными переводами
```

### Сценарий 3: Частичный перевод
```
1. Админ знает украинский перевод, но не знает польский
2. Вводит: name_en + name_uk, оставляет name_pl пустым
3. Система заполнит name_pl и name_ru английским
4. Позже можно обновить только польский
```

---

## 🎯 Итого

**Правила системы:**
1. ✅ `name_en` — ОБЯЗАТЕЛЬНОЕ поле
2. ✅ `name_pl`, `name_uk`, `name_ru` — ОПЦИОНАЛЬНЫЕ
3. ✅ Пустые поля → автоматически заполняются английским
4. ✅ Все 4 языка ВСЕГДА сохраняются в БД
5. ✅ Можно редактировать каждый язык отдельно

**Это даёт:**
- 🚀 Быструю работу (не нужно заполнять 4 поля каждый раз)
- 🎯 Гибкость (можно добавить переводы позже)
- 🛡️ Целостность данных (нет NULL значений)
- 🌍 Поддержку 4 языков из коробки

---

## 📝 Пример для фронтенда

### Минимальный payload (быстрое добавление)
```json
{
  "name_en": "Mango",
  "category_id": "d4a64b25-a187-4ec0-9518-3e8954a138fa",
  "unit": "piece"
}
```
**Результат в БД:**
- name_en: "Mango"
- name_pl: "Mango"
- name_uk: "Mango"
- name_ru: "Mango"

### Полный payload (с переводами)
```json
{
  "name_en": "Mango",
  "name_pl": "Mango",
  "name_uk": "Манго",
  "name_ru": "Манго",
  "category_id": "d4a64b25-a187-4ec0-9518-3e8954a138fa",
  "unit": "piece",
  "description": "Tropical sweet fruit"
}
```
**Результат в БД:**
- name_en: "Mango"
- name_pl: "Mango"
- name_uk: "Манго"
- name_ru: "Манго"

---

**🎉 Система готова! Можно добавлять продукты с переводами автоматически!**
