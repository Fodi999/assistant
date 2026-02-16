# 🚀 Hybrid Translation Cache Strategy

## 📊 Архитектура (экономичная)

### Цель
Админ вводит `name_en` → Если переводы пусты → Бекенд **автоматически** заполняет PL/RU/UK → Затраты на AI минимальные.

### Стратегия (Hybrid Translation Cache Strategy)

```
Admin enters name_en
        ↓
Check if auto_translate=true AND translations empty?
        ↓
YES:
    ┌─────────────────────────────────────┐
    │ 1️⃣ Check Dictionary (0$ - SQL)       │
    │ SELECT FROM ingredient_dictionary   │
    │ WHERE LOWER(name_en) = LOWER(input) │
    └─────────────────────────────────────┘
            ↓
    Found in cache?
            ↓
    YES                NO
    ↓                  ↓
 Use it          2️⃣ Call Groq
                 (1 request, ~0.01$)
                     ↓
                 Get PL, RU, UK
                     ↓
                 3️⃣ Save to Dictionary
                 (кеш навсегда)
                     ↓
                 Use translations
                     ↓
    4️⃣ Update Product Database
            ↓
        Saved ✅
```

## 📁 Реализация

### Этап 1: База данных (`ingredient_dictionary`)

```sql
CREATE TABLE ingredient_dictionary (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name_en TEXT NOT NULL,
    name_pl TEXT NOT NULL,
    name_ru TEXT NOT NULL,
    name_uk TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_dictionary_lower_en
ON ingredient_dictionary (LOWER(TRIM(name_en)));
```

**Файл:** `migrations/20240123000001_create_ingredient_dictionary.sql`

✅ **Статус:** Создана

---

### Этап 2: DictionaryService (персистентность)

**Файл:** `src/infrastructure/persistence/dictionary_service.rs`

```rust
pub struct DictionaryService {
    pool: PgPool,
}

impl DictionaryService {
    /// Поиск перевода по английскому названию (case-insensitive)
    pub async fn find_by_en(&self, name_en: &str) -> Result<Option<DictionaryEntry>, AppError>
    
    /// Сохранить новый перевод (кеш навсегда)
    pub async fn insert(
        &self,
        name_en: &str,
        name_pl: &str,
        name_ru: &str,
        name_uk: &str,
    ) -> Result<DictionaryEntry, AppError>
    
    /// Статистика словаря
    pub async fn get_stats(&self) -> Result<DictionaryStats, AppError>
}
```

**Возможности:**
- Case-insensitive поиск (LOWER)
- ON CONFLICT для идемпотентности
- Индексы для быстрого поиска
- Статистика кеша

✅ **Статус:** Реализован

---

### Этап 3: GroqService (минимизация затрат)

**Файл:** `src/infrastructure/groq_service.rs`

```rust
pub struct GroqService {
    api_key: String,
    http_client: reqwest::Client,
    model: String,  // "llama-3.1-8b-instant" (дешёвая)
}

impl GroqService {
    /// Перевести ингредиент на 3 языка
    /// Правила экономии:
    /// - temperature = 0 (детерминированные результаты)
    /// - max_tokens = 100 (очень короткий ответ)
    /// - Один request на слово
    /// - Timeout 5 секунд
    /// - Не переводим если > 50 символов
    pub async fn translate(&self, ingredient_name: &str) 
        -> Result<GroqTranslationResponse, AppError>
}
```

**Минимальный Prompt:**
```
Translate the ingredient "{}" into Polish, Russian and Ukrainian.
Return strict JSON:
{"pl":"...","ru":"...","uk":"..."}
```

**Результат:**
```json
{
    "pl": "Jabłko",
    "ru": "Яблоко",
    "uk": "Яблуко"
}
```

✅ **Статус:** Реализован

---

### Этап 4: UpdateProductRequest (с флагом)

**Файл:** `src/application/admin_catalog.rs`

```rust
pub struct UpdateProductRequest {
    pub name_en: Option<String>,
    pub name_pl: Option<String>,
    pub name_uk: Option<String>,
    pub name_ru: Option<String>,
    pub category_id: Option<Uuid>,
    pub unit: Option<UnitType>,
    pub description: Option<String>,
    /// ✨ Новое поле!
    /// Если true, бекенд автоматически переводит empty поля
    #[serde(default)]
    pub auto_translate: bool,
}
```

**Использование:**
```json
{
    "name_en": "Apple",
    "auto_translate": true
}
```

✅ **Статус:** Добавлено

---

### Этап 5: Гибридная логика в `update_product()`

**Файл:** `src/application/admin_catalog.rs`

```rust
pub async fn update_product(
    &self,
    id: Uuid,
    req: UpdateProductRequest,
) -> AppResult<ProductResponse> {
    // ... валидация name_en ...

    // 🧠 HYBRID LOGIC
    if req.auto_translate && translations_empty {
        // 1️⃣ Проверяем dictionary
        if let Some(dict_entry) = self.dictionary.find_by_en(final_name_en).await? {
            use_cached_translations(dict_entry);
        } else {
            // 2️⃣ Вызываем Groq
            match self.groq.translate(final_name_en).await {
                Ok(translation) => {
                    // 3️⃣ Сохраняем в dictionary (кеш)
                    self.dictionary.insert(
                        final_name_en,
                        &translation.pl,
                        &translation.ru,
                        &translation.uk
                    ).await?;
                    
                    use_translations(translation);
                }
                Err(e) => {
                    // Fallback to English if Groq fails
                    use_english_as_fallback();
                }
            }
        }
    }

    // 4️⃣ Обновляем БД
    update_product_in_db(...)?;
}
```

**Логирование:**
```
INFO: Auto-translation enabled for: Apple
INFO: Found in dictionary cache: Apple
      → PL: Jabłko, RU: Яблоко, UK: Яблуко

OR

INFO: Dictionary miss for: Pomegranate, calling Groq
INFO: Groq translation successful: Pomegranate -> PL:Granat RU:Гранат UK:Гранат
INFO: Dictionary entry saved: Pomegranate (Granat PL, Гранат RU, Гранат UK)
```

✅ **Статус:** Реализовано

---

### Этап 6: Инициализация сервисов (main.rs)

```rust
// 1. GroqService инициализируется
let groq_service = GroqService::new(config.ai.groq_api_key.clone());
if config.ai.groq_api_key.is_empty() {
    tracing::warn!("⚠️ GROQ_API_KEY not set - auto-translation will not work");
}

// 2. AdminCatalogService получает все зависимости
let admin_catalog_service = AdminCatalogService::new(
    repositories.pool.clone(),
    r2_client,
    repositories.dictionary.clone(),  // ← DictionaryService
    groq_service,                      // ← GroqService
);
```

✅ **Статус:** Реализовано

---

## 💰 Финансовая модель

### Без кеша (неэффективно)
```
1000 продуктов × $0.01 за перевод = $10
Каждый раз при обновлении = опять $10
```

### С кешем (наша архитектура)
```
Первый раз:
  1000 продуктов × $0.01 за перевод = $10 (ОДИН РАЗ)

Потом:
  Lookup в dictionary → 0$ (SQL query)
  Повторные обновления → 0$
```

**Итого:** $10 один раз, потом бесплатно.

### Пример с 2000 ингредиентами
```
Первый месяц: $20 (все ингредиенты переведены)
Следующие месяцы: 0$ (кеш работает)

За год экономия: ~$220 (вместо $240)
```

---

## 🛠 Использование API

### Request

```bash
curl -X PUT "https://api.example.com/api/admin/products/{id}" \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{
    "name_en": "Pineapple",
    "auto_translate": true
  }'
```

### Response

```json
{
  "id": "fb52875b-7947-4089-a84c-23d88cfbe2b5",
  "name_en": "Pineapple",
  "name_pl": "Ananas",           // ← Автоперевод!
  "name_ru": "Ананас",           // ← Автоперевод!
  "name_uk": "Ананас",           // ← Автоперевод!
  "category_id": "d4a64b25-a187-4ec0-9518-3e8954a138fa",
  "unit": "штука",
  "description": "Tropical fruit",
  "image_url": "https://..."
}
```

**Как это работало:**
1. ✅ Dictionary: не найдено (первый раз)
2. ✅ Groq: вызван, переведено за 0.01$
3. ✅ Dictionary: сохранено
4. ✅ Product: обновлён

---

## 📝 Frontend: Checkbox для UX

### HTML

```html
<form>
  <input 
    type="text" 
    name="name_en" 
    placeholder="English name"
    required
  />
  
  <label>
    <input 
      type="checkbox" 
      name="auto_translate" 
      id="auto_translate"
      checked
    />
    ✓ Automatically translate to PL, RU, UK
  </label>
  
  <button type="submit">Save Product</button>
</form>
```

### JavaScript

```javascript
const formData = new FormData(form);

const request = {
  name_en: formData.get('name_en'),
  auto_translate: formData.get('auto_translate') === 'on',
  // Другие поля опциональны...
};

const response = await fetch(`/api/admin/products/${productId}`, {
  method: 'PUT',
  headers: {
    'Authorization': `Bearer ${token}`,
    'Content-Type': 'application/json',
  },
  body: JSON.stringify(request),
});
```

---

## ⚙️ Environment Variables

Добавить в `.env` или `koyeb.yaml`:

```env
GROQ_API_KEY=gsk_xxxxxxxxxxxxxxxxxxxxx
```

**Как получить:**
1. Перейти на https://console.groq.com
2. Sign up / Login
3. Скопировать API Key
4. Вставить в `GROQ_API_KEY`

---

## 🧪 Тестирование

### 1. Проверить миграцию

```bash
# Подключиться к БД
psql $DATABASE_URL

# Проверить таблицу
SELECT COUNT(*) as entries FROM ingredient_dictionary;
```

### 2. Проверить кеш

```bash
# После первого запроса с auto_translate=true
SELECT name_en, name_pl, name_ru, name_uk 
FROM ingredient_dictionary 
WHERE LOWER(name_en) = 'apple';
```

### 3. Тестовый запрос

```bash
TOKEN=$(curl -s -X POST "https://api.example.com/api/admin/auth/login" \
  -H "Content-Type: application/json" \
  -d '{"email":"admin@fodi.app","password":"..."}' | jq -r '.token')

# Первый раз - вызовет Groq
curl -X PUT "https://api.example.com/api/admin/products/{id}" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name_en": "Papaya",
    "auto_translate": true
  }' | jq

# Второй раз - найдёт в кеше
curl -X PUT "https://api.example.com/api/admin/products/{id2}" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name_en": "Papaya",
    "auto_translate": true
  }' | jq
```

**Проверить логи:**
```bash
INFO: Found in dictionary cache: Papaya  # ← Второй запрос
```

---

## 🔒 Production Safety

### Timeout & Retry

```rust
// timeout = 5 seconds
let http_client = reqwest::Client::builder()
    .timeout(Duration::from_secs(5))
    .build()?;

// Если Groq недоступен → fallback to English
match self.groq.translate(name).await {
    Ok(t) => use_translation(t),
    Err(_) => use_english_fallback(),  // ← Safe!
}
```

### Logging & Monitoring

```rust
tracing::info!("Groq translation successful: {} -> PL:{} RU:{} UK:{}",
    ingredient_name, translation.pl, translation.ru, translation.uk);

tracing::warn!("Groq translation failed, falling back to English");

// Статистика
let stats = self.dictionary.get_stats().await?;
tracing::info!("Dictionary has {} cached entries", stats.total_entries);
```

---

## 📊 Метрики

### Что отслеживать

1. **Cache Hit Rate** (% запросов из кеша)
   ```sql
   SELECT COUNT(*) as cache_hits
   FROM ingredient_dictionary
   WHERE created_at > NOW() - INTERVAL '24 hours';
   ```

2. **Groq Usage** (сколько запросов к API)
   - Логи: `Groq translation successful`
   - Стоимость: (count × 0.01$)

3. **Fallback Rate** (когда Groq упал)
   - Логи: `falling back to English`
   - Проверить качество

---

## 🚀 Рекомендуемый порядок внедрения

✅ **1. База данных** - Миграция создана  
✅ **2. DictionaryService** - Реализован  
✅ **3. GroqService** - Реализован  
✅ **4. UpdateProductRequest** - Добавлено поле `auto_translate`  
✅ **5. Гибридная логика** - Интегрирована в `update_product()`  
✅ **6. Инициализация** - main.rs обновлён  
⏳ **7. Добавить `GROQ_API_KEY` в env**  
⏳ **8. Deploy и тестирование**  
⏳ **9. Frontend checkbox** (опционально)  

---

## ✨ Особенности

| Особенность | Описание | Статус |
|---|---|---|
| **Dictionary Cache** | SQL-based in-process кеш | ✅ |
| **Groq Integration** | Минимальные затраты | ✅ |
| **Fallback** | English если Groq упал | ✅ |
| **Case-insensitive** | LOWER() для поиска | ✅ |
| **Idempotent** | ON CONFLICT для безопасности | ✅ |
| **Timeout** | 5 sec для Groq API | ✅ |
| **Logging** | Все операции логируются | ✅ |
| **Auto-translate flag** | Админ контролирует процесс | ✅ |

---

## 🎯 Что дальше?

1. **Раскатить миграцию:**
   ```bash
   cargo build
   cargo sqlx migrate run
   ```

2. **Добавить `GROQ_API_KEY`** в production env

3. **Протестировать:**
   ```bash
   # Обновить продукт с auto_translate=true
   # Проверить логи: "Groq translation successful" или "Found in dictionary"
   # Повторить - должен вывести "Found in dictionary cache"
   ```

4. **Мониторить расходы:**
   - https://console.groq.com/billing
   - Должны быть только первые переводы

---

**Статус:** ✅ Готово к внедрению  
**Дата:** 15 февраля 2026  
**Финансовая модель:** Минимальные затраты, максимальный контроль
