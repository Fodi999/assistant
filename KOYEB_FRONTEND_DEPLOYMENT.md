# 🚀 Frontend Deployment для Recipe V2

## Обзор

Полное руководство по деплою Next.js фронтенда для Recipe V2 на Vercel и связыванию с Koyeb backend API.

## 📋 Архитектура

```
┌─────────────────────────────────────────┐
│  Koyeb Frontend (Next.js)               │
│  URL: https://your-app.koyeb.app        │
│                                         │
│  - Recipe creation form                 │
│  - Recipe list & view                   │
│  - Auto-translation UI                  │
└─────────────────┬───────────────────────┘
                  │
                  │ HTTP requests
                  │ Authorization: Bearer JWT
                  │
                  ▼
┌─────────────────────────────────────────┐
│  Koyeb Backend (Rust/Actix)             │
│  URL: ministerial-yetta-fodi999-...     │
│         .koyeb.app                      │
│                                         │
│  - POST /api/recipes/v2                 │
│  - GET  /api/recipes/v2                 │
│  - POST /api/auth/login                 │
└─────────────────┬───────────────────────┘
                  │
                  │ Database queries
                  │
                  ▼
┌─────────────────────────────────────────┐
│  Neon PostgreSQL                        │
│  - recipes_v2                           │
│  - recipe_translations                  │
│  - recipe_ingredients_v2                │
└─────────────────────────────────────────┘
```

## 🎯 План деплоя

### Вариант 1: Vercel (РЕКОМЕНДУЕТСЯ для Next.js)
- ✅ Бесплатный tier
- ✅ Автоматический деплой из GitHub
- ✅ Быстрый CDN
- ✅ Zero-config для Next.js
- ✅ Автоматический HTTPS

### Вариант 2: Koyeb (Alternative)
- ✅ Бесплатный tier
- ✅ Docker-based deployment
- ⚠️ Требует Dockerfile для Next.js
- ⚠️ Auto-sleep на free tier

**Выбор: Vercel для фронтенда, Koyeb остается для backend**

---

## 🚀 Деплой на Vercel (Рекомендуется)

### Шаг 1: Создать Next.js проект локально

```bash
cd /Users/dmitrijfomin/Desktop/assistant
npx create-next-app@latest frontend --typescript --tailwind --app
cd frontend
npm install axios react-hook-form zod @hookform/resolvers lucide-react clsx tailwind-merge
```

### Шаг 2: Скопировать готовый код

Используйте файлы из `RECIPE_V2_FRONTEND_QUICKSTART.md` или запустите:

```bash
cd /Users/dmitrijfomin/Desktop/assistant
chmod +x setup_frontend.sh
./setup_frontend.sh
```

### Шаг 3: Создать .env.local для production

```bash
cat > frontend/.env.local << 'EOF'
NEXT_PUBLIC_API_URL=https://ministerial-yetta-fodi999-c58d8823.koyeb.app
EOF
```

### Шаг 4: Создать Git репозиторий для фронтенда

```bash
cd frontend
git init
git add .
git commit -m "Initial commit: Recipe V2 frontend"

# Создать новый репозиторий на GitHub
# Название: assistant-frontend
```

```bash
git remote add origin https://github.com/Fodi999/assistant-frontend.git
git branch -M main
git push -u origin main
```

### Шаг 5: Деплой на Vercel

1. **Зайти на https://vercel.com**
2. **Войти через GitHub**
3. **Import Project**
4. **Выбрать репозиторий `assistant-frontend`**
5. **Настроить Environment Variables**:
   ```
   NEXT_PUBLIC_API_URL = https://ministerial-yetta-fodi999-c58d8823.koyeb.app
   ```
6. **Deploy**

✅ Готово! Ваш фронтенд будет доступен на `https://assistant-frontend.vercel.app`

---

## 🔧 Настройка CORS на Backend

Чтобы Vercel фронтенд мог обращаться к Koyeb backend, нужно настроить CORS.

### Обновить `src/main.rs`

<function_calls>
<invoke name="read_file">
<parameter name="filePath">/Users/dmitrijfomin/Desktop/assistant/src/main.rs