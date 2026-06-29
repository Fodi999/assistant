import type { CMSPageItem, SiteId } from '../../../types/admin';
import type { CreateCMSPageDto, UpdateCMSPageDto } from '../../../types/adminApi';
import { baseRow, mapLanguage, mapStatus, pickText, updatedLabel } from './shared';

type BackendArticle = {
  id: string;
  slug?: string | null;
  category?: string | null;
  title?: string | null;
  title_en?: string | null;
  title_ru?: string | null;
  title_pl?: string | null;
  title_uk?: string | null;
  published?: boolean;
  status?: string;
  updated_at?: string;
  updatedAt?: string;
  published_at?: string | null;
  content_en?: string | null;
  content_ru?: string | null;
  content_uk?: string | null;
  image_url?: string | null;
  author_name?: string | null;
  seo_title_en?: string | null;
  seo_title_ru?: string | null;
  seo_title_uk?: string | null;
  seo_description_en?: string | null;
  seo_description_ru?: string | null;
  seo_description_uk?: string | null;
  order_index?: number | null;
};

function clean<T extends Record<string, unknown>>(payload: T): Partial<T> {
  return Object.fromEntries(Object.entries(payload).filter(([, value]) => value !== undefined && value !== '')) as Partial<T>;
}

export const cmsAdapter = {
  fromBackend(article: BackendArticle, siteId: SiteId): CMSPageItem {
    const updatedAt = article.updated_at || article.updatedAt || article.published_at || undefined;
    return {
      ...baseRow({
        id: article.id,
        title: pickText(article.title, article.title_ru, article.title_en, article.title_pl, article.slug),
        slug: article.slug || undefined,
        type: article.category || 'Article',
        status: mapStatus(article.status ?? article.published, article.published ? 'published' : 'draft'),
        owner: 'CMS',
        updated: updatedLabel(updatedAt),
        updatedAt,
        metric: mapLanguage(undefined, 'en').toUpperCase(),
        language: mapLanguage(undefined, 'en'),
        backend: article
      }, siteId),
      resource: 'cms' as const
    };
  },

  toCreate(payload: CreateCMSPageDto) {
    const title = payload.localizedTitle?.en || payload.localizedTitle?.uk || payload.localizedTitle?.ru || payload.title?.trim() || payload.slug || 'Untitled article';
    return clean({
      title_en: title,
      title_ru: payload.localizedTitle?.ru,
      title_uk: payload.localizedTitle?.uk,
      slug: payload.slug,
      category: payload.type,
      content_en: payload.contentLocalized?.en || payload.content,
      content_ru: payload.contentLocalized?.ru,
      content_uk: payload.contentLocalized?.uk,
      image_url: payload.imageUrl,
      author_name: payload.authorName,
      seo_title_en: payload.seoTitle?.en,
      seo_title_ru: payload.seoTitle?.ru,
      seo_title_uk: payload.seoTitle?.uk,
      seo_description_en: payload.seoDescription?.en,
      seo_description_ru: payload.seoDescription?.ru,
      seo_description_uk: payload.seoDescription?.uk,
      published: payload.status === 'published' || payload.status === 'active',
      order_index: payload.orderIndex
    });
  },

  toUpdate(payload: UpdateCMSPageDto) {
    return clean({
      title_en: payload.localizedTitle?.en || payload.title?.trim(),
      title_ru: payload.localizedTitle?.ru,
      title_uk: payload.localizedTitle?.uk,
      slug: payload.slug,
      category: payload.type,
      content_en: payload.contentLocalized?.en || payload.content,
      content_ru: payload.contentLocalized?.ru,
      content_uk: payload.contentLocalized?.uk,
      image_url: payload.imageUrl,
      author_name: payload.authorName,
      seo_title_en: payload.seoTitle?.en,
      seo_title_ru: payload.seoTitle?.ru,
      seo_title_uk: payload.seoTitle?.uk,
      seo_description_en: payload.seoDescription?.en,
      seo_description_ru: payload.seoDescription?.ru,
      seo_description_uk: payload.seoDescription?.uk,
      published: payload.status ? payload.status === 'published' || payload.status === 'active' : undefined,
      order_index: payload.orderIndex
    });
  }
};
