# 🛡️ Admin API Reference — Документация для Admin Panel

**Base URL (Production):** `https://ministerial-yetta-fodi999-c58d8823.koyeb.app`  
**Base URL (Local backend):** `http://localhost:8000`  
**Admin Panel порт:** `http://localhost:3001` (по умолчанию)  
**Prefix всех admin-эндпоинтов:** `/api/admin`

---

## ⚡ Ключевые отличия Admin API от обычного API

| | Обычный пользователь | Super Admin |
|---|---|---|
| Токен | JWT из `/api/auth/login` | JWT из `/api/admin/auth/login` |
| Хранить как | `access_token` | `admin_token` |
| Заголовок | `Authorization: Bearer <token>` | `Authorization: Bearer <admin_token>` |
| Что может | Только свои данные | Все пользователи, глобальный каталог |
| Роль в токене | `user` | `super_admin` |

---

## 🔐 Admin Auth API

### `POST /api/admin/auth/login`
Логин Super Admin. Учётные данные задаются на сервере через переменные окружения `ADMIN_EMAIL` и `ADMIN_PASSWORD_HASH`.

```typescript
// Request:
{
  email: string,
  password: string
}

// Response 200:
{
  token: string,        // Admin JWT токен
  expires_in: number    // Время жизни в секундах (обычно 86400 = 24ч)
}

// Response 401: Неверные credentials
// Response 429: Rate limit exceeded
```

**Пример:**
```typescript
async function adminLogin(email: string, password: string) {
  const res = await fetch(`${BASE_URL}/api/admin/auth/login`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ email, password })
  });
  const data = await res.json();
  // Сохранить токен
  localStorage.setItem('admin_token', data.token);
  return data;
}
```

---

### `GET /api/admin/auth/verify`
Проверить, что admin-токен действителен. Используйте при инициализации страницы.

```typescript
// Headers: Authorization: Bearer <admin_token>
// Response 200:
{
  message: "Token is valid",
  role: "super_admin"
}

// Response 401: Токен недействителен или истёк
```

**Использование:**
```typescript
// При загрузке admin panel:
async function checkAdminAuth() {
  const token = localStorage.getItem('admin_token');
  if (!token) return redirect('/login');
  
  const res = await fetch(`${BASE_URL}/api/admin/auth/verify`, {
    headers: { 'Authorization': `Bearer ${token}` }
  });
  
  if (!res.ok) {
    localStorage.removeItem('admin_token');
    redirect('/login');
  }
}
```

---

## 👥 Admin Users API

> Все эндпоинты требуют: `Authorization: Bearer <admin_token>`

### `GET /api/admin/users`
Получить список всех зарегистрированных пользователей (ресторанов).

```typescript
// Response 200:
{
  users: [
    {
      id: string,              // UUID пользователя
      email: string,
      name: string | null,     // display_name
      restaurant_name: string, // Название ресторана
      language: string,        // "ru" | "en" | "pl" | "uk"
      created_at: string,      // ISO 8601
      login_count: number,     // Количество входов
      last_login_at: string | null  // Последний вход
    }
  ],
  total: number
}

// Отсортировано по login_count DESC (самые активные первые)
```

**Пример в React:**
```typescript
async function getUsers() {
  const res = await adminFetch('/api/admin/users');
  const data = await res.json();
  return data.users; // массив пользователей
}
```

---

### `DELETE /api/admin/users/:id`
**⚠️ КАСКАДНОЕ УДАЛЕНИЕ** — удаляет пользователя, его ресторан (tenant) и ВСЕ связанные данные (инвентарь, рецепты, блюда и т.д.).

```typescript
// :id — UUID пользователя из поля users[n].id

// Response 200: OK
// Response 404: Пользователь не найден
// Response 500: Ошибка базы данных
```

**⚠️ Предупреждение:** Это необратимо. Удалите пользователя только после подтверждения.

```typescript
async function deleteUser(userId: string) {
  const confirmed = window.confirm('Удалить пользователя и все его данные? Это необратимо!');
  if (!confirmed) return;
  
  const res = await adminFetch(`/api/admin/users/${userId}`, { method: 'DELETE' });
  if (res.ok) {
    alert('Пользователь удалён');
    // обновить список
  }
}
```

---

### `GET /api/admin/stats`
Сводная статистика по всем пользователям и ресторанам.

```typescript
// Response 200:
{
  total_users: number,       // Количество пользователей
  total_restaurants: number  // Количество ресторанов (tenants)
}
```

---

## 📦 Admin Catalog API — Продукты

> Глобальный каталог ингредиентов. Виден всем пользователям системы.
> Все эндпоинты требуют: `Authorization: Bearer <admin_token>`

### `GET /api/admin/products`
Список всех ингредиентов в глобальном каталоге.

```typescript
// Response 200: Array<ProductResponse>
[
  {
    id: string,             // UUID продукта
    name_en: string,        // Название на английском (обязательно)
    name_ru: string | null, // Название на русском
    name_pl: string | null, // Название на польском
    name_uk: string | null, // Название на украинском
    category_id: string,    // UUID категории
    unit: UnitType,         // "kilogram" | "liter" | "piece" | ...
    description: string | null,
    image_url: string | null
  }
]
```

---

### `GET /api/admin/products/:id`
Получить один продукт по ID.

```typescript
// Response 200: ProductResponse (см. выше)
// Response 404: Not found
```

---

### `POST /api/admin/products`
Создать новый ингредиент в глобальном каталоге.

**Режим 1 — Smart (рекомендуется):** Введите название на любом языке, бэкенд автоматически переведёт на все языки и определит категорию через AI.

```typescript
// Request:
{
  name_input: string,        // 🧠 Название на ЛЮБОМ языке: "Молоко", "Milk", "Mleko"
  auto_translate: boolean,   // true = автоперевод через AI (по умолчанию true)
  
  // Ручные переопределения (опционально):
  name_en?: string,
  name_ru?: string,
  name_pl?: string,
  name_uk?: string,
  
  category_id?: string,      // UUID категории (если не указан — AI определит)
  unit?: UnitType,           // Если не указан — AI определит
  description?: string
}

// Response 201: ProductResponse
{
  id: string,
  name_en: string,
  name_ru: string,
  name_pl: string,
  name_uk: string,
  category_id: string,
  unit: string,
  description: string | null,
  image_url: null
}
```

**Режим 2 — Ручной:** Укажите все переводы вручную.

```typescript
// Request:
{
  name_input: "Milk",         // Всё равно нужен (используется как основа)
  auto_translate: false,      // Отключить AI
  name_en: "Milk",
  name_ru: "Молоко",
  name_pl: "Mleko",
  name_uk: "Молоко",
  category_id: "uuid-категории-молочных",
  unit: "liter"
}
```

**Допустимые значения `unit`:**
```typescript
type UnitType = 
  | "kilogram"   // кг
  | "gram"       // г
  | "liter"      // л
  | "milliliter" // мл
  | "piece"      // шт
  | "pack"       // упак
  | "bottle"     // бут
  | "can"        // банка
  | "box"        // коробка
  | "bunch"      // пучок
```

---

### `PUT /api/admin/products/:id` / `PATCH /api/admin/products/:id`
Обновить ингредиент. Все поля опциональны.

```typescript
// Request:
{
  name_en?: string,
  name_ru?: string,
  name_pl?: string,
  name_uk?: string,
  category_id?: string,
  unit?: UnitType,
  description?: string,
  image_url?: string,
  auto_translate?: boolean   // true = допереводить пустые поля через AI
}

// Response 200: ProductResponse
```

---

### `DELETE /api/admin/products/:id`
Удалить ингредиент из глобального каталога.

```typescript
// Response 204: No Content
// Response 404: Not found
```

---

## 🖼️ Admin Products — Изображения

### `POST /api/admin/products/:id/image`
Загрузить изображение напрямую (multipart/form-data).

```typescript
// Content-Type: multipart/form-data
// Поле: "file" или "image" (тип: image/*)

// Response 200:
{ image_url: string }  // Публичный URL изображения в R2

// Пример с FormData:
const formData = new FormData();
formData.append('file', imageFile, 'product.webp');

const res = await adminFetch(`/api/admin/products/${id}/image`, {
  method: 'POST',
  body: formData
  // ⚠️ НЕ ставьте Content-Type вручную — браузер сам добавит boundary
});
const { image_url } = await res.json();
```

---

### `GET /api/admin/products/:id/image-url`
Получить presigned URL для прямой загрузки в R2 с фронтенда (без прохода через сервер).

```typescript
// Query params:
// content_type? = "image/webp" | "image/jpeg" | "image/png" (default: "image/webp")

// GET /api/admin/products/:id/image-url?content_type=image/webp

// Response 200:
{
  upload_url: string,   // Presigned PUT URL (действует ~15 мин)
  public_url: string    // Публичный URL, который будет доступен после загрузки
}

// Процесс:
// 1. GET /api/admin/products/:id/image-url → получить upload_url + public_url
// 2. PUT upload_url с binary данными файла (Content-Type: image/webp)
// 3. PUT /api/admin/products/:id/image → сохранить public_url в БД

async function uploadImageViaPresigned(productId: string, file: File) {
  // Шаг 1: Получить presigned URL
  const { upload_url, public_url } = await adminFetch(
    `/api/admin/products/${productId}/image-url?content_type=${file.type}`
  ).then(r => r.json());
  
  // Шаг 2: Загрузить файл напрямую в R2
  await fetch(upload_url, {
    method: 'PUT',
    body: file,
    headers: { 'Content-Type': file.type }
  });
  
  // Шаг 3: Сохранить URL в базе данных
  await adminFetch(`/api/admin/products/${productId}/image`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ image_url: public_url })
  });
  
  return public_url;
}
```

---

### `PUT /api/admin/products/:id/image`
Сохранить URL изображения в базе данных (после прямой загрузки в R2).

```typescript
// Request:
{ image_url: string }

// Response 200: OK
```

---

### `DELETE /api/admin/products/:id/image`
Удалить изображение продукта (из R2 и из базы данных).

```typescript
// Response 204: No Content
```

---

## 📂 Admin Catalog API — Категории

> Категории ингредиентов: Молочные, Мясо, Овощи и т.д.  
> Все эндпоинты требуют: `Authorization: Bearer <admin_token>`

### `GET /api/admin/categories`
Список всех категорий с переводами на все языки.

```typescript
// Response 200: Array<CategoryResponse>
[
  {
    id: string,      // UUID категории
    name_en: string, // "Dairy Products"
    name_ru: string, // "Молочные продукты"
    name_pl: string, // "Produkty mleczne"
    name_uk: string, // "Молочні продукти"
    sort_order: number  // Порядок сортировки (меньше = выше)
  }
]
```

---

### `POST /api/admin/categories`
Создать новую категорию ингредиентов.

```typescript
// Request:
{
  name_en: string,   // ОБЯЗАТЕЛЬНО: "Spices"
  name_ru: string,   // ОБЯЗАТЕЛЬНО: "Специи"
  name_pl: string,   // ОБЯЗАТЕЛЬНО: "Przyprawy"
  name_uk: string,   // ОБЯЗАТЕЛЬНО: "Спеції"
  sort_order: number // ОБЯЗАТЕЛЬНО: 10 (порядок в списке)
}

// Response 201: CategoryResponse
```

---

### `PUT /api/admin/categories/:id`
Обновить категорию. Все поля опциональны.

```typescript
// Request:
{
  name_en?: string,
  name_ru?: string,
  name_pl?: string,
  name_uk?: string,
  sort_order?: number
}

// Response 200: CategoryResponse
```

---

### `DELETE /api/admin/categories/:id`
Удалить категорию.  
⚠️ Сначала переназначьте продукты этой категории, иначе возможен FK constraint.

```typescript
// Response 204: No Content
// Response 500: Если есть продукты, привязанные к этой категории
```

---

## 🛠️ Утилита — Admin Fetch Helper

Добавьте в ваш проект:

```typescript
// lib/admin-client.ts

const BASE_URL = process.env.NEXT_PUBLIC_API_URL 
  ?? 'https://ministerial-yetta-fodi999-c58d8823.koyeb.app';

function getAdminToken(): string | null {
  return typeof window !== 'undefined' 
    ? localStorage.getItem('admin_token') 
    : null;
}

export async function adminFetch(
  path: string, 
  options: RequestInit = {}
): Promise<Response> {
  const token = getAdminToken();
  
  const res = await fetch(`${BASE_URL}${path}`, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...(token ? { 'Authorization': `Bearer ${token}` } : {}),
      ...options.headers
    }
  });
  
  if (res.status === 401) {
    // Admin токен истёк
    localStorage.removeItem('admin_token');
    window.location.href = '/admin/login';
    throw new Error('Admin session expired');
  }
  
  if (!res.ok) {
    const error = await res.text();
    throw new Error(`Admin API error ${res.status}: ${error}`);
  }
  
  return res;
}

// Специальная версия без Content-Type (для multipart/form-data)
export async function adminFetchFile(
  path: string,
  options: RequestInit = {}
): Promise<Response> {
  const token = getAdminToken();
  
  const headers: HeadersInit = {};
  if (token) headers['Authorization'] = `Bearer ${token}`;
  
  // Добавляем пользовательские заголовки (кроме Content-Type!)
  if (options.headers) {
    Object.assign(headers, options.headers);
  }
  
  const res = await fetch(`${BASE_URL}${path}`, {
    ...options,
    headers
  });
  
  if (res.status === 401) {
    localStorage.removeItem('admin_token');
    window.location.href = '/admin/login';
    throw new Error('Admin session expired');
  }
  
  return res;
}
```

---

## 📋 TypeScript типы для Admin API

```typescript
// types/admin.ts

export type UnitType = 
  | 'kilogram' | 'gram' | 'liter' | 'milliliter'
  | 'piece' | 'pack' | 'bottle' | 'can' | 'box' | 'bunch';

export interface AdminLoginResponse {
  token: string;
  expires_in: number;
}

export interface AdminVerifyResponse {
  message: string;
  role: 'super_admin';
}

export interface AdminUserInfo {
  id: string;
  email: string;
  name: string | null;
  restaurant_name: string;
  language: 'ru' | 'en' | 'pl' | 'uk';
  created_at: string;
  login_count: number;
  last_login_at: string | null;
}

export interface AdminUsersResponse {
  users: AdminUserInfo[];
  total: number;
}

export interface AdminStats {
  total_users: number;
  total_restaurants: number;
}

export interface ProductResponse {
  id: string;
  name_en: string;
  name_ru: string | null;
  name_pl: string | null;
  name_uk: string | null;
  category_id: string;
  unit: UnitType;
  description: string | null;
  image_url: string | null;
}

export interface CreateProductRequest {
  name_input: string;        // Название на любом языке
  auto_translate?: boolean;  // default: true
  name_en?: string;
  name_ru?: string;
  name_pl?: string;
  name_uk?: string;
  category_id?: string;
  unit?: UnitType;
  description?: string;
}

export interface UpdateProductRequest {
  name_en?: string;
  name_ru?: string;
  name_pl?: string;
  name_uk?: string;
  category_id?: string;
  unit?: UnitType;
  description?: string;
  image_url?: string;
  auto_translate?: boolean;
}

export interface CategoryResponse {
  id: string;
  name_en: string;
  name_ru: string;
  name_pl: string;
  name_uk: string;
  sort_order: number;
}

export interface CreateCategoryRequest {
  name_en: string;
  name_ru: string;
  name_pl: string;
  name_uk: string;
  sort_order: number;
}

export interface UpdateCategoryRequest {
  name_en?: string;
  name_ru?: string;
  name_pl?: string;
  name_uk?: string;
  sort_order?: number;
}

export interface ImageUploadUrlResponse {
  upload_url: string;
  public_url: string;
}
```

---

## 🗺️ Полная карта Admin API

```
POST   /api/admin/auth/login          → AdminLoginResponse
GET    /api/admin/auth/verify         → { message, role }  [🔒 admin]

GET    /api/admin/users               → AdminUsersResponse  [🔒 admin]
DELETE /api/admin/users/:id           → 200 OK             [🔒 admin]
GET    /api/admin/stats               → AdminStats         [🔒 admin]

GET    /api/admin/catalog/products            → ProductResponse[]   [🔒 admin]
GET    /api/admin/catalog/products/:id        → ProductResponse     [🔒 admin]
POST   /api/admin/catalog/products            → 201 ProductResponse [🔒 admin]
PUT    /api/admin/catalog/products/:id        → ProductResponse     [🔒 admin]
PATCH  /api/admin/catalog/products/:id        → ProductResponse     [🔒 admin]
DELETE /api/admin/catalog/products/:id        → 204                 [🔒 admin]

POST   /api/admin/catalog/products/:id/image     → { image_url }           [🔒 admin] (multipart)
GET    /api/admin/catalog/products/:id/image-url → { upload_url, public_url } [🔒 admin]
PUT    /api/admin/catalog/products/:id/image     → 200 OK                  [🔒 admin]
DELETE /api/admin/catalog/products/:id/image     → 204                     [🔒 admin]

GET    /api/admin/catalog/categories          → CategoryResponse[]    [🔒 admin]
POST   /api/admin/catalog/categories          → 201 CategoryResponse  [🔒 admin]
PUT    /api/admin/catalog/categories/:id      → CategoryResponse      [🔒 admin]
DELETE /api/admin/catalog/categories/:id      → 204                   [🔒 admin]
```

---

## ⚙️ Переменные окружения на сервере

Admin-панель работает только если на Koyeb установлены:

```bash
ADMIN_EMAIL=admin@yourapp.com
ADMIN_PASSWORD_HASH=$argon2id$v=19$...  # argon2 hash пароля
ADMIN_JWT_SECRET=your-32-char-secret    # минимум 32 символа
ADMIN_TOKEN_TTL_HOURS=24               # время жизни токена
CORS_ALLOWED_ORIGINS=http://localhost:3000,http://localhost:3001,https://your-admin.vercel.app
```
