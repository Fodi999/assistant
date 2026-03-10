# Подключение двух Next.js фронтендов к Rust бекенду

## Архитектура

```
Rust API (Koyeb)
https://ministerial-yetta-fodi999-c58d8823.koyeb.app
│
├── /api/admin/*          ← Только для Admin Dashboard (JWT)
├── /api/admin/cms/*      ← Только для Admin Dashboard (JWT)
│
└── /public/*             ← Для Blog сайта (без авторизации)
    ├── /public/about
    ├── /public/expertise
    ├── /public/experience
    ├── /public/gallery
    ├── /public/articles
    ├── /public/articles/:slug
    ├── /public/articles-sitemap
    ├── /public/article-categories
    ├── /public/stats
    └── /public/tools/*
```

---

## Сайт 1: Admin Dashboard (Next.js)

### .env.local

```env
NEXT_PUBLIC_API_URL=https://ministerial-yetta-fodi999-c58d8823.koyeb.app
ADMIN_SECRET=your_super_admin_password
```

---

### lib/api.ts — базовый клиент

```typescript
const API = process.env.NEXT_PUBLIC_API_URL!

// Получить токен из localStorage
export function getToken(): string | null {
  if (typeof window === 'undefined') return null
  return localStorage.getItem('admin_token')
}

// Авторизованный fetch
export async function apiFetch<T>(
  path: string,
  options: RequestInit = {}
): Promise<T> {
  const token = getToken()
  const res = await fetch(`${API}${path}`, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
      ...options.headers,
    },
  })
  if (!res.ok) {
    const err = await res.json().catch(() => ({}))
    throw new Error(err.message || `HTTP ${res.status}`)
  }
  return res.json()
}
```

---

### lib/auth.ts — логин/логаут

```typescript
import { apiFetch } from './api'

interface LoginResponse {
  token: string
  expires_in: number
}

export async function adminLogin(password: string): Promise<void> {
  const data = await apiFetch<LoginResponse>('/api/admin/auth/login', {
    method: 'POST',
    body: JSON.stringify({ password }),
  })
  localStorage.setItem('admin_token', data.token)
  localStorage.setItem('admin_token_exp', String(Date.now() + data.expires_in * 1000))
}

export function adminLogout(): void {
  localStorage.removeItem('admin_token')
  localStorage.removeItem('admin_token_exp')
}

export function isTokenValid(): boolean {
  const exp = localStorage.getItem('admin_token_exp')
  if (!exp) return false
  return Date.now() < Number(exp)
}
```

---

### lib/cms.ts — все CMS методы

```typescript
import { apiFetch } from './api'

const BASE = '/api/admin/cms'

// ── About ──────────────────────────────────────────────────────────────────
export const getAbout = () =>
  apiFetch<About>(`${BASE}/about`)

export const updateAbout = (data: Partial<About>) =>
  apiFetch<About>(`${BASE}/about`, { method: 'PUT', body: JSON.stringify(data) })

// ── Expertise ──────────────────────────────────────────────────────────────
export const listExpertise = () =>
  apiFetch<Expertise[]>(`${BASE}/expertise`)

export const createExpertise = (data: CreateExpertise) =>
  apiFetch<Expertise>(`${BASE}/expertise`, { method: 'POST', body: JSON.stringify(data) })

export const updateExpertise = (id: string, data: Partial<Expertise>) =>
  apiFetch<Expertise>(`${BASE}/expertise/${id}`, { method: 'PUT', body: JSON.stringify(data) })

export const deleteExpertise = (id: string) =>
  apiFetch<void>(`${BASE}/expertise/${id}`, { method: 'DELETE' })

// ── Experience ─────────────────────────────────────────────────────────────
export const listExperience = () =>
  apiFetch<Experience[]>(`${BASE}/experience`)

export const createExperience = (data: CreateExperience) =>
  apiFetch<Experience>(`${BASE}/experience`, { method: 'POST', body: JSON.stringify(data) })

export const updateExperience = (id: string, data: Partial<Experience>) =>
  apiFetch<Experience>(`${BASE}/experience/${id}`, { method: 'PUT', body: JSON.stringify(data) })

export const deleteExperience = (id: string) =>
  apiFetch<void>(`${BASE}/experience/${id}`, { method: 'DELETE' })

// ── Gallery ────────────────────────────────────────────────────────────────
export const listGallery = () =>
  apiFetch<Gallery[]>(`${BASE}/gallery`)

export const createGallery = (data: CreateGallery) =>
  apiFetch<Gallery>(`${BASE}/gallery`, { method: 'POST', body: JSON.stringify(data) })

export const updateGallery = (id: string, data: Partial<Gallery>) =>
  apiFetch<Gallery>(`${BASE}/gallery/${id}`, { method: 'PUT', body: JSON.stringify(data) })

export const deleteGallery = (id: string) =>
  apiFetch<void>(`${BASE}/gallery/${id}`, { method: 'DELETE' })

// ── Articles ───────────────────────────────────────────────────────────────
export const listArticlesAdmin = () =>
  apiFetch<Article[]>(`${BASE}/articles`)

export const getArticleAdmin = (id: string) =>
  apiFetch<Article>(`${BASE}/articles/${id}`)

export const createArticle = (data: CreateArticle) =>
  apiFetch<Article>(`${BASE}/articles`, { method: 'POST', body: JSON.stringify(data) })

export const updateArticle = (id: string, data: Partial<Article>) =>
  apiFetch<Article>(`${BASE}/articles/${id}`, { method: 'PUT', body: JSON.stringify(data) })

export const deleteArticle = (id: string) =>
  apiFetch<void>(`${BASE}/articles/${id}`, { method: 'DELETE' })

// ── Image Upload ───────────────────────────────────────────────────────────
export interface UploadUrlResponse {
  upload_url: string   // PUT сюда файл напрямую
  url: string          // Финальный публичный URL
}

export async function getUploadUrl(
  folder: 'gallery' | 'articles' | 'about' | 'general',
  contentType: string = 'image/webp'
): Promise<UploadUrlResponse> {
  return apiFetch<UploadUrlResponse>(
    `${BASE}/upload-url?folder=${folder}&content_type=${encodeURIComponent(contentType)}`
  )
}

// Загрузить файл через presigned URL, вернуть публичный URL
export async function uploadImage(
  file: File,
  folder: 'gallery' | 'articles' | 'about' | 'general'
): Promise<string> {
  const { upload_url, url } = await getUploadUrl(folder, file.type)
  await fetch(upload_url, {
    method: 'PUT',
    body: file,
    headers: { 'Content-Type': file.type },
  })
  return url
}
```

---

### lib/cms-types.ts — TypeScript типы

```typescript
export interface About {
  id: string
  title_en: string; title_pl: string; title_ru: string; title_uk: string
  content_en: string; content_pl: string; content_ru: string; content_uk: string
  image_url?: string
  updated_at: string
}

export interface Expertise {
  id: string
  icon: string
  title_en: string; title_pl: string; title_ru: string; title_uk: string
  order_index: number
  created_at: string; updated_at: string
}
export type CreateExpertise = Omit<Expertise, 'id' | 'created_at' | 'updated_at'>

export interface Experience {
  id: string
  restaurant: string; country: string; position: string
  start_year?: number; end_year?: number
  description_en: string; description_pl: string; description_ru: string; description_uk: string
  order_index: number
  created_at: string; updated_at: string
}
export type CreateExperience = Omit<Experience, 'id' | 'created_at' | 'updated_at'>

export interface Gallery {
  id: string
  image_url: string
  title_en: string; title_pl: string; title_ru: string; title_uk: string
  description_en: string; description_pl: string; description_ru: string; description_uk: string
  alt_en: string; alt_pl: string; alt_ru: string; alt_uk: string
  order_index: number
  created_at: string; updated_at: string
}
export type CreateGallery = Omit<Gallery, 'id' | 'created_at' | 'updated_at'>

export interface Article {
  id: string
  slug: string
  title_en: string; title_pl: string; title_ru: string; title_uk: string
  content_en: string; content_pl: string; content_ru: string; content_uk: string
  image_url?: string
  seo_title: string; seo_description: string
  published: boolean
  order_index: number
  created_at: string; updated_at: string
}
export type CreateArticle = Omit<Article, 'id' | 'created_at' | 'updated_at'>

export interface ArticleListResponse {
  data: Article[]
  total: number
  page: number
  limit: number
}

export interface PublicStats {
  articles_count: number
  ingredients_count: number
  tools_count: number
  experience_years: number
  countries: number
}
```

---

### Защита роутов — middleware.ts (Admin Dashboard)

```typescript
// middleware.ts (корень проекта)
import { NextRequest, NextResponse } from 'next/server'

export function middleware(req: NextRequest) {
  const token = req.cookies.get('admin_token')?.value
  const isLoginPage = req.nextUrl.pathname === '/login'

  if (!token && !isLoginPage) {
    return NextResponse.redirect(new URL('/login', req.url))
  }
  return NextResponse.next()
}

export const config = {
  matcher: ['/dashboard/:path*'],
}
```

---

## Сайт 2: Blog / Portfolio (Next.js)

### .env.local

```env
NEXT_PUBLIC_API_URL=https://ministerial-yetta-fodi999-c58d8823.koyeb.app
```

---

### lib/public-api.ts — только публичные данные

```typescript
const API = process.env.NEXT_PUBLIC_API_URL!

async function get<T>(path: string): Promise<T> {
  const res = await fetch(`${API}${path}`, {
    next: { revalidate: 60 }, // ISR: обновлять каждые 60 сек
  })
  if (!res.ok) throw new Error(`HTTP ${res.status}`)
  return res.json()
}

// About
export const getAbout = () => get<About>('/public/about')

// Chef profile
export const getExpertise = () => get<Expertise[]>('/public/expertise')
export const getExperience = () => get<Experience[]>('/public/experience')
export const getGallery = () => get<Gallery[]>('/public/gallery')
export const getStats = () => get<PublicStats>('/public/stats')

// Articles
export const getArticles = (page = 1, limit = 12, search = '') =>
  get<ArticleListResponse>(
    `/public/articles?page=${page}&limit=${limit}${search ? `&search=${encodeURIComponent(search)}` : ''}`
  )

export const getArticle = (slug: string) =>
  get<Article>(`/public/articles/${slug}`)

export const getArticleCategories = () =>
  get<ArticleCategory[]>('/public/article-categories')

// Sitemap (для generateSitemapEntries)
export const getArticlesSitemap = () =>
  get<SitemapRow[]>('/public/articles-sitemap')

// Kitchen Tools
export const getNutrition = (slug: string) =>
  get<NutritionResponse>(`/public/tools/nutrition?slug=${slug}`)

export const convertIngredient = (slug: string, value: number, from: string, to: string) =>
  get(`/public/tools/ingredient-convert?slug=${slug}&value=${value}&from_unit=${from}&to_unit=${to}`)
```

---

### app/sitemap.ts — автоматический sitemap.xml

```typescript
import { getArticlesSitemap } from '@/lib/public-api'
import { MetadataRoute } from 'next'

export default async function sitemap(): Promise<MetadataRoute.Sitemap> {
  const articles = await getArticlesSitemap()

  const staticRoutes: MetadataRoute.Sitemap = [
    { url: 'https://dima-fomin.pl', lastModified: new Date(), priority: 1 },
    { url: 'https://dima-fomin.pl/about', lastModified: new Date(), priority: 0.9 },
    { url: 'https://dima-fomin.pl/articles', lastModified: new Date(), priority: 0.8 },
    { url: 'https://dima-fomin.pl/tools', lastModified: new Date(), priority: 0.7 },
  ]

  const articleRoutes: MetadataRoute.Sitemap = articles.map((a) => ({
    url: `https://dima-fomin.pl/articles/${a.slug}`,
    lastModified: new Date(a.updated_at),
    priority: 0.7,
  }))

  return [...staticRoutes, ...articleRoutes]
}
```

---

### app/articles/[slug]/page.tsx — динамические страницы с SEO

```typescript
import { getArticle, getArticlesSitemap } from '@/lib/public-api'
import { notFound } from 'next/navigation'
import { Metadata } from 'next'

// generateStaticParams — пробилдить все статьи при деплое
export async function generateStaticParams() {
  const articles = await getArticlesSitemap()
  return articles.map((a) => ({ slug: a.slug }))
}

// SEO metadata
export async function generateMetadata({ params }: { params: { slug: string } }): Promise<Metadata> {
  try {
    const article = await getArticle(params.slug)
    return {
      title: article.seo_title,
      description: article.seo_description,
      openGraph: {
        title: article.seo_title,
        description: article.seo_description,
        images: article.image_url ? [article.image_url] : [],
      },
    }
  } catch {
    return { title: 'Article not found' }
  }
}

export default async function ArticlePage({ params }: { params: { slug: string } }) {
  let article
  try {
    article = await getArticle(params.slug)
  } catch {
    notFound()
  }

  return (
    <article>
      <h1>{article.title_ru}</h1>   {/* или title_en, в зависимости от i18n */}
      <div dangerouslySetInnerHTML={{ __html: article.content_ru }} />
    </article>
  )
}
```

---

## CORS — убедись что домены разрешены

В `CORS_ALLOWED_ORIGINS` на Koyeb добавь оба фронтенда:

```
CORS_ALLOWED_ORIGINS=https://your-admin-site.vercel.app,https://your-blog-site.vercel.app,http://localhost:3000,http://localhost:3001
```

---

## Итоговая схема

```
Admin Dashboard                   Blog / Portfolio
(Vercel / localhost:3001)         (Vercel / localhost:3000)
        │                                   │
        │ Bearer TOKEN (JWT)                │ без токена
        ▼                                   ▼
/api/admin/cms/*              /public/about
/api/admin/catalog/*          /public/articles?page=1&search=sushi
/api/admin/auth/*             /public/articles/:slug
                              /public/expertise
                              /public/experience
                              /public/gallery
                              /public/stats
                              /public/articles-sitemap → sitemap.xml
                              /public/tools/*
                                    │
                              Rust API на Koyeb
```
