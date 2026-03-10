# Blog Frontend — Integration Guide
# Подключение Blog Site к публичному API

## Stack: Next.js 14+ (App Router) + TypeScript + ISR

---

## 1. Environment Variables

```env
# .env.local
NEXT_PUBLIC_API_URL=https://ministerial-yetta-fodi999-c58d8823.koyeb.app
```

---

## 2. lib/public-api.ts — Public fetch client

```typescript
// lib/public-api.ts
// Все запросы без авторизации. ISR revalidate = 60 сек.

const API = process.env.NEXT_PUBLIC_API_URL!

async function publicFetch<T>(path: string, revalidate = 60): Promise<T> {
  const res = await fetch(`${API}${path}`, {
    next: { revalidate },
  })
  if (!res.ok) throw new Error(`HTTP ${res.status}: ${path}`)
  return res.json()
}

// ── ABOUT ─────────────────────────────────────────────────────────────────────
// GET /public/about
// → { title_en, title_ru, content_en, content_ru, image_url, ... }
export const getAbout = () =>
  publicFetch<About>('/public/about')

// ── EXPERTISE ─────────────────────────────────────────────────────────────────
// GET /public/expertise
// → [ { id, icon, title_en, title_ru, ... } ]
export const listExpertise = () =>
  publicFetch<Expertise[]>('/public/expertise')

// ── EXPERIENCE ────────────────────────────────────────────────────────────────
// GET /public/experience
// → [ { id, restaurant, country, position, start_year, end_year, description_ru, ... } ]
export const listExperience = () =>
  publicFetch<Experience[]>('/public/experience')

// ── GALLERY ───────────────────────────────────────────────────────────────────
// GET /public/gallery
// → [ { id, image_url, title_ru, alt_ru, description_ru, ... } ]
export const listGallery = () =>
  publicFetch<Gallery[]>('/public/gallery')

// ── ARTICLES ──────────────────────────────────────────────────────────────────
// GET /public/articles?page=1&limit=20&search=sushi
// → { data: Article[], total, page, limit }
export interface ArticleQuery {
  page?: number
  limit?: number
  search?: string
}

export const listArticles = ({ page = 1, limit = 20, search = '' }: ArticleQuery = {}) => {
  const params = new URLSearchParams({
    page: String(page),
    limit: String(limit),
    ...(search ? { search } : {}),
  })
  return publicFetch<ArticleListResponse>(`/public/articles?${params}`)
}

// GET /public/articles/:slug
// → Article (полная статья, только published)
export const getArticle = (slug: string) =>
  publicFetch<Article>(`/public/articles/${slug}`, 3600)   // кэш 1 час

// ── ARTICLE CATEGORIES ────────────────────────────────────────────────────────
// GET /public/article-categories
// → [ { id, slug, title_en, title_ru, ... } ]
export const listCategories = () =>
  publicFetch<ArticleCategory[]>('/public/article-categories')

// ── SITEMAP ───────────────────────────────────────────────────────────────────
// GET /public/articles-sitemap
// → [ { slug, updated_at } ]
export const getArticlesSitemap = () =>
  publicFetch<ArticleSitemapEntry[]>('/public/articles-sitemap', 3600)

// ── STATS ─────────────────────────────────────────────────────────────────────
// GET /public/stats
// → { articles_count, ingredients_count, tools_count, experience_years, countries }
export const getPublicStats = () =>
  publicFetch<PublicStats>('/public/stats', 600)
```

---

## 3. lib/blog-types.ts — TypeScript типы

```typescript
// lib/blog-types.ts

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
}

export interface Experience {
  id: string
  restaurant: string; country: string; position: string
  start_year?: number
  end_year?: number
  description_en: string; description_pl: string
  description_ru: string; description_uk: string
  order_index: number
}

export interface Gallery {
  id: string
  image_url: string
  title_en: string; title_pl: string; title_ru: string; title_uk: string
  description_en: string; description_pl: string
  description_ru: string; description_uk: string
  alt_en: string; alt_pl: string; alt_ru: string; alt_uk: string
  order_index: number
}

export interface Article {
  id: string
  slug: string
  title_en: string; title_pl: string; title_ru: string; title_uk: string
  content_en: string; content_pl: string; content_ru: string; content_uk: string
  image_url?: string
  seo_title: string
  seo_description: string
  published: boolean
  order_index: number
  created_at: string; updated_at: string
}

export interface ArticleListResponse {
  data: Article[]
  total: number
  page: number
  limit: number
}

export interface ArticleCategory {
  id: string
  slug: string
  title_en: string; title_pl: string; title_ru: string; title_uk: string
  order_index: number
}

export interface ArticleSitemapEntry {
  slug: string
  updated_at: string
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

## 4. Мультиязычность — хелпер

```typescript
// lib/i18n.ts
export type Locale = 'en' | 'pl' | 'ru' | 'uk'

// Взять нужный языковой ключ из объекта
// pick(article, 'title', 'ru') → article.title_ru
export function pick<T extends Record<string, unknown>>(
  obj: T,
  field: string,
  locale: Locale
): string {
  const key = `${field}_${locale}` as keyof T
  const fallback = `${field}_en` as keyof T
  return (obj[key] ?? obj[fallback] ?? '') as string
}
```

Использование:
```typescript
import { pick } from '@/lib/i18n'

const locale: Locale = 'ru'
pick(article, 'title', locale)    // article.title_ru
pick(article, 'content', locale)  // article.content_ru
```

---

## 5. app/sitemap.ts — динамический sitemap.xml

```typescript
// app/sitemap.ts
import { MetadataRoute } from 'next'
import { getArticlesSitemap } from '@/lib/public-api'

export default async function sitemap(): Promise<MetadataRoute.Sitemap> {
  const entries = await getArticlesSitemap()

  const articles = entries.map(e => ({
    url: `https://your-blog.com/articles/${e.slug}`,
    lastModified: new Date(e.updated_at),
    changeFrequency: 'weekly' as const,
    priority: 0.8,
  }))

  return [
    {
      url: 'https://your-blog.com',
      lastModified: new Date(),
      changeFrequency: 'daily',
      priority: 1,
    },
    {
      url: 'https://your-blog.com/about',
      lastModified: new Date(),
      changeFrequency: 'monthly',
      priority: 0.9,
    },
    ...articles,
  ]
}
```

---

## 6. app/articles/page.tsx — список статей + поиск + пагинация

```typescript
// app/articles/page.tsx
import { listArticles } from '@/lib/public-api'
import Link from 'next/link'

interface Props {
  searchParams: { page?: string; search?: string }
}

export default async function ArticlesPage({ searchParams }: Props) {
  const page   = Number(searchParams.page ?? 1)
  const search = searchParams.search ?? ''

  const { data: articles, total, limit } = await listArticles({ page, limit: 12, search })

  const totalPages = Math.ceil(total / limit)

  return (
    <main>
      {/* Поиск */}
      <form method="GET">
        <input name="search" defaultValue={search} placeholder="Поиск..." />
        <button type="submit">🔍</button>
      </form>

      {/* Список статей */}
      <ul>
        {articles.map(a => (
          <li key={a.id}>
            <Link href={`/articles/${a.slug}`}>
              {a.image_url && <img src={a.image_url} alt={a.title_ru} />}
              <h2>{a.title_ru}</h2>
              <p>{a.seo_description}</p>
            </Link>
          </li>
        ))}
      </ul>

      {/* Пагинация */}
      <nav>
        {page > 1 && (
          <Link href={`/articles?page=${page - 1}&search=${search}`}>← Назад</Link>
        )}
        <span>Страница {page} из {totalPages}</span>
        {page < totalPages && (
          <Link href={`/articles?page=${page + 1}&search=${search}`}>Вперёд →</Link>
        )}
      </nav>
    </main>
  )
}
```

---

## 7. app/articles/[slug]/page.tsx — статья + SEO metadata

```typescript
// app/articles/[slug]/page.tsx
import { getArticle, getArticlesSitemap } from '@/lib/public-api'
import { notFound } from 'next/navigation'
import type { Metadata } from 'next'

interface Props {
  params: { slug: string }
}

// Генерация статических путей (SSG) — для всех published статей
export async function generateStaticParams() {
  const entries = await getArticlesSitemap()
  return entries.map(e => ({ slug: e.slug }))
}

// SEO meta tags
export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const article = await getArticle(params.slug).catch(() => null)
  if (!article) return {}

  return {
    title: article.seo_title || article.title_ru,
    description: article.seo_description,
    openGraph: {
      title: article.seo_title || article.title_ru,
      description: article.seo_description,
      images: article.image_url ? [article.image_url] : [],
    },
  }
}

export default async function ArticlePage({ params }: Props) {
  const article = await getArticle(params.slug).catch(() => null)
  if (!article) notFound()

  return (
    <article>
      {article.image_url && (
        <img src={article.image_url} alt={article.title_ru} />
      )}
      <h1>{article.title_ru}</h1>
      <div dangerouslySetInnerHTML={{ __html: article.content_ru }} />
    </article>
  )
}
```

---

## 8. app/about/page.tsx — страница "О шефе"

```typescript
// app/about/page.tsx
import { getAbout, listExpertise, listExperience } from '@/lib/public-api'

export default async function AboutPage() {
  const [about, expertise, experience] = await Promise.all([
    getAbout(),
    listExpertise(),
    listExperience(),
  ])

  return (
    <main>
      {/* Hero */}
      <section>
        {about.image_url && <img src={about.image_url} alt={about.title_ru} />}
        <h1>{about.title_ru}</h1>
        <p>{about.content_ru}</p>
      </section>

      {/* Специализации */}
      <section>
        <h2>Специализации</h2>
        <ul>
          {expertise.sort((a, b) => a.order_index - b.order_index).map(e => (
            <li key={e.id}>
              <span>{e.icon}</span>
              <span>{e.title_ru}</span>
            </li>
          ))}
        </ul>
      </section>

      {/* Опыт */}
      <section>
        <h2>Опыт работы</h2>
        <ul>
          {experience.sort((a, b) => a.order_index - b.order_index).map(e => (
            <li key={e.id}>
              <strong>{e.restaurant}</strong> — {e.country}
              <br />
              <em>{e.position}</em>
              <br />
              {e.start_year} — {e.end_year ?? 'по сей день'}
              <br />
              {e.description_ru}
            </li>
          ))}
        </ul>
      </section>
    </main>
  )
}
```

---

## 9. app/gallery/page.tsx — галерея

```typescript
// app/gallery/page.tsx
import { listGallery } from '@/lib/public-api'

export default async function GalleryPage() {
  const items = await listGallery()

  return (
    <main>
      <h1>Галерея</h1>
      <div className="grid grid-cols-3 gap-4">
        {items.sort((a, b) => a.order_index - b.order_index).map(item => (
          <figure key={item.id}>
            <img
              src={item.image_url}
              alt={item.alt_ru || item.title_ru}
              title={item.title_ru}
            />
            <figcaption>{item.description_ru}</figcaption>
          </figure>
        ))}
      </div>
    </main>
  )
}
```

---

## 10. Все публичные маршруты (справочник)

```
GET /public/about
GET /public/expertise
GET /public/experience
GET /public/gallery
GET /public/articles                 → { data, total, page, limit }
GET /public/articles?page=2&limit=12
GET /public/articles?search=sushi
GET /public/articles/:slug           → Article (только published)
GET /public/article-categories       → [ { id, slug, title_* } ]
GET /public/articles-sitemap         → [ { slug, updated_at } ]
GET /public/stats                    → { articles_count, experience_years, countries, ... }
```

> ⚠️ Публичные маршруты не требуют авторизации. Возвращают только `published = true` статьи.

---

## 11. Структура проекта

```
blog-site/
├── app/
│   ├── layout.tsx
│   ├── page.tsx             ← Главная (stats + последние статьи)
│   ├── about/
│   │   └── page.tsx         ← О шефе
│   ├── articles/
│   │   ├── page.tsx         ← Список + поиск + пагинация
│   │   └── [slug]/
│   │       └── page.tsx     ← Статья + SEO
│   └── gallery/
│       └── page.tsx
├── lib/
│   ├── public-api.ts        ← Все API вызовы
│   ├── blog-types.ts        ← TypeScript типы
│   └── i18n.ts              ← Мультиязычный хелпер
└── app/
    └── sitemap.ts           ← Динамический sitemap.xml
```
