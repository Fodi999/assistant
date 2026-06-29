import type { AdminResourceRow, SiteId } from '../../../types/admin';
import type { CreateShopProductDto, UpdateShopProductDto } from '../../../types/adminApi';
import { baseRow, mapLanguage, mapStatus, pickText, updatedLabel } from './shared';

type BackendShopProduct = {
  id: string;
  slug?: string;
  name?: string;
  title?: string;
  name_en?: string;
  name_ru?: string;
  name_pl?: string;
  name_uk?: string;
  category?: string;
  sku?: string | null;
  status?: string;
  price_cents?: number | null;
  currency?: string;
  stock_quantity?: number;
  updated_at?: string;
  updatedAt?: string;
  short_description_en?: string;
  short_description_ru?: string;
  short_description_pl?: string;
  short_description_uk?: string;
  description_en?: string;
  description_ru?: string;
  description_pl?: string;
  description_uk?: string;
  seo_title_en?: string;
  seo_title_ru?: string;
  seo_title_pl?: string;
  seo_title_uk?: string;
  seo_description_en?: string;
  seo_description_ru?: string;
  seo_description_pl?: string;
  seo_description_uk?: string;
  selling_points?: string[];
  image_urls?: string[];
};

function clean<T extends Record<string, unknown>>(payload: T): Partial<T> {
  return Object.fromEntries(Object.entries(payload).filter(([, value]) => value !== undefined && value !== '')) as Partial<T>;
}

function cleanImageUrls(urls: string[] | undefined) {
  return (urls || []).filter((url) => {
    try {
      const parsed = new URL(url);
      return parsed.protocol === 'http:' || parsed.protocol === 'https:';
    } catch {
      return false;
    }
  });
}

function toBackendShopPayload(payload: CreateShopProductDto | UpdateShopProductDto) {
  const title = payload.name?.en || payload.name?.uk || payload.name?.ru || payload.name?.pl || payload.title?.trim() || payload.slug || 'Untitled product';
  return clean({
    name_en: title,
    name_ru: payload.name?.ru,
    name_pl: payload.name?.pl,
    name_uk: payload.name?.uk,
    slug: payload.slug,
    sku: payload.sku,
    category: payload.type,
    short_description_en: payload.shortDescription?.en,
    short_description_ru: payload.shortDescription?.ru,
    short_description_pl: payload.shortDescription?.pl,
    short_description_uk: payload.shortDescription?.uk,
    description_en: payload.description?.en,
    description_ru: payload.description?.ru,
    description_pl: payload.description?.pl,
    description_uk: payload.description?.uk,
    seo_title_en: payload.seoTitle?.en,
    seo_title_ru: payload.seoTitle?.ru,
    seo_title_pl: payload.seoTitle?.pl,
    seo_title_uk: payload.seoTitle?.uk,
    seo_description_en: payload.seoDescription?.en,
    seo_description_ru: payload.seoDescription?.ru,
    seo_description_pl: payload.seoDescription?.pl,
    seo_description_uk: payload.seoDescription?.uk,
    selling_points: payload.sellingPoints,
    image_urls: cleanImageUrls(payload.imageUrls),
    price_cents: payload.priceCents,
    currency: payload.currency,
    stock_quantity: payload.stockQuantity,
    status: payload.status
  });
}

export const shopAdapter = {
  fromBackend(product: BackendShopProduct, siteId: SiteId): AdminResourceRow {
    const updatedAt = product.updated_at || product.updatedAt;
    const price = product.price_cents ? `${product.currency || ''} ${(product.price_cents / 100).toFixed(2)}`.trim() : product.currency;

    return baseRow({
      id: product.id,
      title: pickText(product.title, product.name, product.name_ru, product.name_en, product.name_pl, product.slug),
      slug: product.slug,
      type: product.category || 'Shop product',
      status: mapStatus(product.status, 'draft'),
      owner: product.sku || 'Shop',
      updated: updatedLabel(updatedAt),
      updatedAt,
      metric: price || `${product.stock_quantity ?? 0} stock`,
      language: mapLanguage(undefined, 'en'),
      backend: product
    }, siteId);
  },

  toCreate(payload: CreateShopProductDto) {
    return toBackendShopPayload(payload);
  },

  toUpdate(payload: UpdateShopProductDto) {
    return toBackendShopPayload(payload);
  }
};
