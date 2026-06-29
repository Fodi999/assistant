import type { CatalogItem, SiteId } from '../../../types/admin';
import type { CreateCatalogItemDto, UpdateCatalogItemDto } from '../../../types/adminApi';
import { baseRow, mapLanguage, mapStatus, pickText, updatedLabel } from './shared';

type BackendCatalogProduct = {
  id: string;
  slug?: string | null;
  title?: string | null;
  name?: string | null;
  name_input?: string | null;
  name_en?: string | null;
  name_ru?: string | null;
  name_pl?: string | null;
  name_uk?: string | null;
  product_type?: string | null;
  unit?: string | null;
  is_published?: boolean;
  status?: string;
  updated_at?: string;
  updatedAt?: string;
  published_at?: string | null;
  description?: string | null;
  description_en?: string | null;
  description_ru?: string | null;
  description_uk?: string | null;
  image_url?: string | null;
  calories_per_100g?: number | null;
  protein_per_100g?: number | null;
  fat_per_100g?: number | null;
  carbs_per_100g?: number | null;
  fiber_per_100g?: number | null;
  sugar_per_100g?: number | null;
  density_g_per_ml?: number | null;
  typical_portion_g?: number | null;
  shelf_life_days?: number | null;
  seasons?: string[];
  seo_title?: string | null;
  seo_description?: string | null;
  seo_h1?: string | null;
};

function clean<T extends Record<string, unknown>>(payload: T): Partial<T> {
  return Object.fromEntries(Object.entries(payload).filter(([, value]) => value !== undefined && value !== '')) as Partial<T>;
}

export const catalogAdapter = {
  fromBackend(product: BackendCatalogProduct, siteId: SiteId): CatalogItem {
    const updatedAt = product.updated_at || product.updatedAt || product.published_at || undefined;
    const title = pickText(product.title, product.name, product.name_ru, product.name_en, product.name_pl, product.name_input, product.slug);

    return {
      ...baseRow({
        id: product.id,
        title,
        slug: product.slug || undefined,
        type: product.product_type || product.unit || 'Product',
        status: mapStatus(product.status ?? product.is_published, product.is_published ? 'published' : 'draft'),
        owner: 'Catalog',
        updated: updatedLabel(updatedAt),
        updatedAt,
        metric: product.unit || mapLanguage(undefined, 'en').toUpperCase(),
        language: mapLanguage(undefined, 'en'),
        backend: product
      }, siteId),
      resource: 'catalog' as const
    };
  },

  toCreate(payload: CreateCatalogItemDto) {
    const title = payload.name?.uk || payload.name?.ru || payload.name?.en || payload.title?.trim() || 'Untitled item';
    return clean({
      name_input: title,
      name_en: payload.name?.en || '',
      name_ru: payload.name?.ru || '',
      name_uk: payload.name?.uk || '',
      unit: payload.unit,
      description: payload.description?.en || payload.description?.uk || payload.description?.ru,
      image_url: payload.imageUrl,
      product_type: payload.productType,
      description_en: payload.description?.en,
      description_ru: payload.description?.ru,
      description_uk: payload.description?.uk,
      calories_per_100g: payload.caloriesPer100g,
      protein_per_100g: payload.proteinPer100g,
      fat_per_100g: payload.fatPer100g,
      carbs_per_100g: payload.carbsPer100g,
      fiber_per_100g: payload.fiberPer100g,
      sugar_per_100g: payload.sugarPer100g,
      density_g_per_ml: payload.densityGPerMl,
      typical_portion_g: payload.typicalPortionG,
      shelf_life_days: payload.shelfLifeDays,
      seasons: payload.seasons,
      seo_title: payload.seoTitle,
      seo_description: payload.seoDescription,
      seo_h1: payload.seoH1,
      auto_translate: true
    });
  },

  toUpdate(payload: UpdateCatalogItemDto) {
    return clean({
      name_en: payload.name?.en || payload.title,
      name_ru: payload.name?.ru,
      name_uk: payload.name?.uk,
      unit: payload.unit,
      description: payload.description?.en || payload.description?.uk || payload.description?.ru,
      image_url: payload.imageUrl,
      description_en: payload.description?.en,
      description_ru: payload.description?.ru,
      description_uk: payload.description?.uk,
      calories_per_100g: payload.caloriesPer100g,
      protein_per_100g: payload.proteinPer100g,
      fat_per_100g: payload.fatPer100g,
      carbs_per_100g: payload.carbsPer100g,
      fiber_per_100g: payload.fiberPer100g,
      sugar_per_100g: payload.sugarPer100g,
      density_g_per_ml: payload.densityGPerMl,
      typical_portion_g: payload.typicalPortionG,
      shelf_life_days: payload.shelfLifeDays,
      seasons: payload.seasons,
      product_type: payload.productType,
      seo_title: payload.seoTitle,
      seo_description: payload.seoDescription,
      seo_h1: payload.seoH1,
      auto_translate: false
    });
  }
};
