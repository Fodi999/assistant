# ⚡ Recipe V2 Frontend - Шпаргалка

## 🚀 Быстрый деплой (5 команд)

```bash
# 1. Создать фронтенд
cd /Users/dmitrijfomin/Desktop/assistant
./setup_frontend.sh

# 2. Git push
cd frontend
git init && git add . && git commit -m "Initial commit"
git remote add origin https://github.com/Fodi999/assistant-frontend.git
git push -u origin main

# 3. Vercel deploy
# Зайти на vercel.com → Import → assistant-frontend
# ENV: NEXT_PUBLIC_API_URL = https://ministerial-yetta-fodi999-c58d8823.koyeb.app

# 4. Обновить CORS на Koyeb
# Dashboard → Settings → Environment:
# CORS_ALLOWED_ORIGINS=https://assistant-frontend.vercel.app,http://localhost:3000

# 5. Тест
# https://assistant-frontend.vercel.app/recipes/create
```

---

## 🔑 Получить JWT токен

```bash
# Через curl
curl -X POST https://ministerial-yetta-fodi999-c58d8823.koyeb.app/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"dmitrijfomin@gmail.com","password":"test123"}' \
  | jq -r .access_token

# Или в браузере (F12 Console):
fetch('https://ministerial-yetta-fodi999-c58d8823.koyeb.app/api/auth/login', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ email: 'dmitrijfomin@gmail.com', password: 'test123' })
}).then(r => r.json()).then(d => {
  localStorage.setItem('auth_token', d.access_token);
  console.log('✅ Token saved');
});
```

---

## 🌐 API Endpoints (Backend)

```
POST   /api/recipes/v2           - Создать рецепт
GET    /api/recipes/v2           - Список рецептов
GET    /api/recipes/v2/:id       - Получить рецепт
POST   /api/recipes/v2/:id/publish - Опубликовать
DELETE /api/recipes/v2/:id       - Удалить
```

---

## 📋 Пример запроса (создание рецепта)

```bash
curl -X POST https://ministerial-yetta-fodi999-c58d8823.koyeb.app/api/recipes/v2 \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Борщ украинский",
    "instructions": "Сварить свеклу, морковь и капусту. Добавить мясо и картофель. Варить 2 часа.",
    "language": "ru",
    "servings": 6,
    "ingredients": [{
      "catalog_ingredient_id": "8238ad5e-f9d2-4edd-8690-9ba68e07a3f8",
      "quantity": 0.5,
      "unit": "kg"
    }]
  }'
```

**Ответ**:
```json
{
  "id": "uuid",
  "name": "Борщ украинский",
  "language": "ru",
  "translations": [
    {"language": "en", "name": "Ukrainian Borscht", ...},
    {"language": "pl", "name": "Barszcz ukraiński", ...},
    {"language": "uk", "name": "Борщ український", ...}
  ]
}
```

---

## 🛠️ CORS Setup

### Koyeb Environment Variables

```env
CORS_ALLOWED_ORIGINS=https://assistant-frontend.vercel.app,http://localhost:3000
```

### Проверка CORS

```bash
# Test preflight
curl -X OPTIONS https://ministerial-yetta-fodi999-c58d8823.koyeb.app/api/recipes/v2 \
  -H "Origin: https://assistant-frontend.vercel.app" \
  -H "Access-Control-Request-Method: POST" \
  -v | grep access-control
```

---

## 🐛 Troubleshooting

| Проблема | Решение |
|----------|---------|
| CORS error | Проверить `CORS_ALLOWED_ORIGINS` на Koyeb + redeploy |
| 401 Unauthorized | Проверить токен в localStorage + TTL (15 мин) |
| Network timeout | Koyeb auto-sleep, подождать 10 сек |
| Token не сохраняется | Использовать HTTPS (Vercel), не HTTP |

---

## 📂 Структура файлов

```
frontend/
├── app/recipes/create/page.tsx    # Форма создания
├── components/recipes/RecipeForm.tsx
├── services/recipeService.ts       # API calls
├── types/recipe.ts                 # TypeScript types
└── .env.local                      # NEXT_PUBLIC_API_URL
```

---

## ✅ Production Checklist

- [ ] Frontend деплоен на Vercel
- [ ] CORS настроен на Koyeb
- [ ] JWT токен работает
- [ ] Создание рецепта работает
- [ ] Автопереводы работают (RU→EN,PL,UK)

---

## 📚 Полная документация

- `RECIPE_V2_FULL_DEPLOYMENT.md` - Полный гайд
- `RECIPE_V2_FRONTEND_QUICKSTART.md` - Быстрый старт
- `RECIPE_V2_UI_COMPONENTS.md` - UI компоненты
