# 🚀 Recipe V2 Frontend Deployment - Полное руководство

## Архитектура

```
Vercel Frontend (Next.js)
  https://assistant-frontend.vercel.app
           │
           │ HTTPS requests + JWT
           │ CORS enabled
           ▼
Koyeb Backend (Rust)
  https://ministerial-yetta-fodi999-c58d8823.koyeb.app
           │
           │ SQL queries
           ▼
Neon PostgreSQL
  recipes_v2 + recipe_translations
```

---

## 📋 Быстрый старт (15 минут)

### Шаг 1: Создать фронтенд локально (3 мин)

```bash
cd /Users/dmitrijfomin/Desktop/assistant

# Использовать готовый скрипт
chmod +x setup_frontend.sh
./setup_frontend.sh
```

Или вручную:

```bash
npx create-next-app@latest frontend --typescript --tailwind --app
cd frontend
npm install axios react-hook-form zod @hookform/resolvers lucide-react clsx tailwind-merge
```

Скопируйте код из `RECIPE_V2_FRONTEND_QUICKSTART.md`.

### Шаг 2: Настроить CORS на backend (2 мин)

#### 2.1 Обновить переменные окружения на Koyeb

Зайдите в Koyeb Dashboard → Ваше приложение → Settings → Environment variables

Добавьте или обновите:

```env
CORS_ALLOWED_ORIGINS=https://assistant-frontend.vercel.app,http://localhost:3000
```

**ВАЖНО**: Разделитель - запятая без пробелов!

#### 2.2 Redeploy backend

После изменения переменных Koyeb автоматически сделает redeploy. Подождите 1-2 минуты.

### Шаг 3: Создать GitHub репозиторий (2 мин)

```bash
cd frontend
git init
git add .
git commit -m "feat: Recipe V2 frontend with auto-translations"

# Создать репозиторий на GitHub: assistant-frontend
git remote add origin https://github.com/Fodi999/assistant-frontend.git
git branch -M main
git push -u origin main
```

### Шаг 4: Деплой на Vercel (5 мин)

1. **Зайти на https://vercel.com**
2. **Sign in with GitHub**
3. **New Project → Import Git Repository**
4. **Выбрать `Fodi999/assistant-frontend`**
5. **Configure Project**:
   - Framework Preset: Next.js (auto-detected)
   - Root Directory: `./` (default)
   
6. **Environment Variables** (добавить):
   ```
   NEXT_PUBLIC_API_URL = https://ministerial-yetta-fodi999-c58d8823.koyeb.app
   ```

7. **Deploy**

✅ Через 2-3 минуты ваш фронтенд будет доступен!

URL будет вида: `https://assistant-frontend.vercel.app`

### Шаг 5: Обновить CORS с реальным URL (3 мин)

После деплоя Vercel даст вам финальный URL. Обновите CORS на Koyeb:

```env
CORS_ALLOWED_ORIGINS=https://assistant-frontend.vercel.app,http://localhost:3000
```

---

## 🧪 Тестирование

### 1. Войти в систему

Откройте: `https://assistant-frontend.vercel.app`

Пока нет страницы логина, используйте консоль браузера (F12):

```javascript
// Получить токен с backend
fetch('https://ministerial-yetta-fodi999-c58d8823.koyeb.app/api/auth/login', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    email: 'dmitrijfomin@gmail.com',
    password: 'test123'
  })
})
.then(r => r.json())
.then(data => {
  localStorage.setItem('auth_token', data.access_token);
  console.log('✅ Logged in!');
  location.reload();
});
```

### 2. Создать рецепт

Перейдите на: `https://assistant-frontend.vercel.app/recipes/create`

Заполните форму:
- **Название**: Борщ украинский
- **Язык**: Русский (RU)
- **Порции**: 6
- **Инструкции**: Сварить свеклу, морковь и капусту. Добавить мясо и картофель. Варить 2 часа.

Нажмите **"Создать рецепт"**

### 3. Проверить автоматические переводы

Backend автоматически:
- ✅ Переведет название на EN, PL, UK
- ✅ Переведет инструкции на EN, PL, UK
- ✅ Сохранит все переводы в БД

---

## 🛠️ Локальная разработка

### 1. Запустить backend локально

```bash
cd /Users/dmitrijfomin/Desktop/assistant

# Установить переменные окружения
export DATABASE_URL="postgresql://..."
export JWT_SECRET="your_secret"
export GROQ_API_KEY="gsk_..."
export CORS_ALLOWED_ORIGINS="http://localhost:3000"

# Запустить
RUST_LOG=debug cargo run --release
```

### 2. Запустить frontend локально

```bash
cd frontend

# .env.local должен указывать на локальный backend
echo "NEXT_PUBLIC_API_URL=http://localhost:8000" > .env.local

npm run dev
```

Откройте: **http://localhost:3000/recipes/create**

---

## 🔧 Настройка CORS (подробно)

### Backend: `src/infrastructure/config.rs`

Текущая конфигурация:

```rust
cors: CorsConfig {
    allowed_origins: env::var("CORS_ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:3000".to_string())
        .split(',')
        .map(|s| s.trim().to_string())
        .collect(),
},
```

### Backend: `src/interfaces/http/routes.rs`

CORS middleware применяется ко всем роутам:

```rust
let cors = CorsLayer::new()
    .allow_origin(cors_origins.iter().map(|o| o.parse::<HeaderValue>().unwrap()).collect::<Vec<_>>())
    .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
    .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
    .allow_credentials(true);

Router::new()
    .route("/health", get(health_check))
    .nest("/api", api_routes)
    .layer(cors)
```

### Koyeb Environment Variables

```env
# Добавить в Koyeb Dashboard → Settings → Environment
CORS_ALLOWED_ORIGINS=https://assistant-frontend.vercel.app,http://localhost:3000
```

**Формат**:
- Разделитель: запятая `,`
- БЕЗ пробелов после запятой
- Полные URL с протоколом (`https://`)

---

## 📊 Проверка CORS

### Тест 1: Preflight request

```bash
curl -X OPTIONS https://ministerial-yetta-fodi999-c58d8823.koyeb.app/api/recipes/v2 \
  -H "Origin: https://assistant-frontend.vercel.app" \
  -H "Access-Control-Request-Method: POST" \
  -v
```

**Ожидаемый ответ**:

```
< HTTP/2 200
< access-control-allow-origin: https://assistant-frontend.vercel.app
< access-control-allow-methods: GET, POST, PUT, DELETE, OPTIONS
< access-control-allow-headers: authorization, content-type
< access-control-allow-credentials: true
```

### Тест 2: Actual request

```bash
curl -X POST https://ministerial-yetta-fodi999-c58d8823.koyeb.app/api/recipes/v2 \
  -H "Origin: https://assistant-frontend.vercel.app" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"Test","instructions":"Test","language":"ru","servings":1,"ingredients":[]}' \
  -v
```

**Ожидаемый response header**:

```
< access-control-allow-origin: https://assistant-frontend.vercel.app
```

---

## 🚨 Troubleshooting

### Проблема 1: CORS error в браузере

```
Access to fetch at 'https://...' from origin 'https://assistant-frontend.vercel.app' 
has been blocked by CORS policy
```

**Решение**:

1. Проверить `CORS_ALLOWED_ORIGINS` на Koyeb:
   ```bash
   # Должно включать ваш Vercel URL
   https://assistant-frontend.vercel.app,http://localhost:3000
   ```

2. Redeploy backend после изменения переменных

3. Очистить кеш браузера (Ctrl+Shift+R)

### Проблема 2: Токен не сохраняется

**Причина**: localStorage не работает на HTTP (только HTTPS)

**Решение**: Используйте Vercel (автоматический HTTPS)

### Проблема 3: 401 Unauthorized

```json
{"code":"AUTHENTICATION_ERROR","message":"Authentication failed"}
```

**Проверки**:

1. Токен в localStorage:
   ```javascript
   console.log(localStorage.getItem('auth_token'));
   ```

2. Токен отправляется в заголовке:
   ```javascript
   // В services/api.ts должно быть:
   api.interceptors.request.use((config) => {
     const token = localStorage.getItem('auth_token');
     if (token) config.headers.Authorization = `Bearer ${token}`;
     return config;
   });
   ```

3. Токен не истек (TTL 15 минут по умолчанию)

### Проблема 4: Network Error

**Причина**: Backend спит (Koyeb Free tier auto-sleep)

**Решение**: Подождите 5-10 секунд, повторите запрос

---

## 📦 Структура деплоя

### Frontend (Vercel)

```
assistant-frontend/
├── app/
│   ├── page.tsx                    # Главная страница
│   └── recipes/
│       ├── create/
│       │   └── page.tsx            # Создание рецепта
│       └── [id]/
│           └── page.tsx            # Просмотр рецепта
├── components/
│   ├── recipes/
│   │   └── RecipeForm.tsx          # Форма создания
│   └── ui/
│       ├── Button.tsx
│       ├── Input.tsx
│       ├── Textarea.tsx
│       └── Select.tsx
├── services/
│   ├── api.ts                      # HTTP client
│   └── recipeService.ts            # Recipe API
├── types/
│   └── recipe.ts                   # TypeScript типы
├── .env.local
├── next.config.js
├── package.json
└── tailwind.config.js
```

### Backend (Koyeb)

```
assistant/
├── src/
│   ├── application/
│   │   ├── recipe_v2_service.rs           # Recipe business logic
│   │   └── recipe_translation_service.rs  # AI translations
│   ├── domain/
│   │   └── recipe_v2.rs                   # Recipe entities
│   ├── infrastructure/
│   │   ├── groq_service.rs                # Groq AI client
│   │   └── config.rs                      # CORS config
│   └── interfaces/
│       └── http/
│           ├── recipe_v2.rs               # HTTP handlers
│           └── routes.rs                  # CORS middleware
└── Dockerfile
```

---

## ✅ Checklist финального деплоя

- [ ] Frontend создан локально
- [ ] Код скопирован из QUICKSTART guide
- [ ] GitHub репозиторий создан (assistant-frontend)
- [ ] Vercel подключен к GitHub
- [ ] Environment variable `NEXT_PUBLIC_API_URL` добавлена
- [ ] Frontend задеплоен на Vercel
- [ ] CORS настроен на backend (Koyeb)
- [ ] Backend redeploy выполнен
- [ ] Preflight CORS тест пройден
- [ ] Логин работает (токен сохраняется)
- [ ] Создание рецепта работает
- [ ] Автоматические переводы работают

---

## 🎯 Следующие шаги

1. **Добавить страницу логина**
   - Форма email + password
   - Редирект после успешного логина

2. **Добавить список рецептов**
   - GET /api/recipes/v2
   - Карточки рецептов
   - Фильтры (draft/published)

3. **Добавить просмотр рецепта**
   - Переключение языков (RU/EN/PL/UK)
   - Отображение ингредиентов
   - Кнопки "Опубликовать" / "Удалить"

4. **Добавить редактирование рецепта**
   - PUT endpoint на backend
   - Форма редактирования
   - Сохранение изменений

5. **Улучшить UI**
   - Индикатор переводов (🌐 3/3 languages)
   - Loading states
   - Error handling
   - Toast notifications

---

## 📚 Документация

- **Быстрый старт**: `RECIPE_V2_FRONTEND_QUICKSTART.md`
- **UI компоненты**: `RECIPE_V2_UI_COMPONENTS.md`
- **Полный гайд**: `RECIPE_V2_FRONTEND_GUIDE.md`
- **Backend API**: `RECIPE_SYSTEM_IMPLEMENTATION.md`

---

**Готово к production!** 🚀✨
