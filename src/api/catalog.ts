import { apiFetch } from './client';
import type { AdminCategory, AdminProduct, PublicIngredientListResponse } from '../types/admin';

export interface AiDraftField<T> {
  value: T | null;
  source: string;
  confidence: string;
}

export interface AiProductDraft {
  names: {
    en: AiDraftField<string>;
    ru: AiDraftField<string>;
    pl: AiDraftField<string>;
    uk: AiDraftField<string>;
  };
  description_en: AiDraftField<string>;
  description_ru: AiDraftField<string>;
  description_pl: AiDraftField<string>;
  description_uk: AiDraftField<string>;
  product_type: AiDraftField<string>;
  unit: AiDraftField<string>;
  nutrition: {
    calories_per_100g: AiDraftField<number>;
    protein_per_100g: AiDraftField<number>;
    fat_per_100g: AiDraftField<number>;
    carbs_per_100g: AiDraftField<number>;
    fiber_per_100g: AiDraftField<number>;
    sugar_per_100g: AiDraftField<number>;
    density_g_per_ml: AiDraftField<number>;
    typical_portion_g: AiDraftField<number>;
    shelf_life_days: AiDraftField<number>;
  };
  seo: {
    seo_title: AiDraftField<string>;
    seo_description: AiDraftField<string>;
    seo_h1: AiDraftField<string>;
  };
  seasons: AiDraftField<string[]>;
  extended: AiExtendedProductProfile;
  confidence: number;
  needs_review: boolean;
  quality_warnings: Array<{
    field: string;
    label_ru: string;
    severity: string;
    message: string;
  }>;
}

export type AiProfileSection = Record<string, string | number | boolean | null | string[] | unknown[]>;

export interface AiExtendedProductProfile {
  macros?: AiProfileSection;
  vitamins?: AiProfileSection;
  minerals?: AiProfileSection;
  fatty_acids?: AiProfileSection;
  diet_flags?: AiProfileSection;
  allergens?: AiProfileSection;
  food_properties?: AiProfileSection;
  culinary?: AiProfileSection;
  health_profile?: AiProfileSection;
  sugar_profile?: AiProfileSection;
  processing_effects?: AiProfileSection;
  culinary_behavior?: AiProfileSection;
}

export interface AiCreateDraftResponse {
  draft: AiProductDraft;
  raw_input: string;
  model: string;
  cached: boolean;
  corrections: Array<{
    field: string;
    original_value: string;
    corrected_to: string;
    reason: string;
  }>;
}

export interface CreateAdminProductRequest {
  name_input: string;
  name_en?: string;
  name_ru?: string;
  name_pl?: string;
  name_uk?: string;
  unit?: 'gram' | 'kilogram' | 'liter' | 'milliliter' | 'piece' | 'bunch' | 'can' | 'bottle' | 'package';
  product_type?: string;
  description?: string;
  image_url?: string;
  description_en?: string;
  description_ru?: string;
  description_pl?: string;
  description_uk?: string;
  calories_per_100g?: number;
  protein_per_100g?: number;
  fat_per_100g?: number;
  carbs_per_100g?: number;
  fiber_per_100g?: number;
  sugar_per_100g?: number;
  density_g_per_ml?: number;
  typical_portion_g?: number;
  shelf_life_days?: number;
  seasons?: string[];
  seo_title?: string;
  seo_description?: string;
  seo_h1?: string;
  auto_translate?: boolean;
}

export interface ProductDataQuality {
  product_id: string;
  slug: string;
  name_en: string;
  score: number;
  weighted_score: number;
  status: string;
  missing_critical?: unknown[];
  missing_optional?: unknown[];
  sections?: Record<string, unknown>;
}

export interface IngredientState {
  id: string;
  ingredient_id: string;
  state: string;
  calories_per_100g?: number | null;
  protein_per_100g?: number | null;
  fat_per_100g?: number | null;
  carbs_per_100g?: number | null;
  fiber_per_100g?: number | null;
  water_percent?: number | null;
  shelf_life_hours?: number | null;
  storage_temp_c?: number | null;
  texture?: string | null;
  weight_change_percent?: number | null;
  state_type?: string | null;
  oil_absorption_g?: number | null;
  water_loss_percent?: number | null;
  glycemic_index?: number | null;
  cooking_method?: string | null;
  name_suffix_en?: string | null;
  name_suffix_pl?: string | null;
  name_suffix_ru?: string | null;
  name_suffix_uk?: string | null;
  notes_en?: string | null;
  notes_pl?: string | null;
  notes_ru?: string | null;
  notes_uk?: string | null;
  image_url?: string | null;
  data_score?: number | null;
}

export function listPublicIngredients(): Promise<PublicIngredientListResponse> {
  return apiFetch<PublicIngredientListResponse>('/public/ingredients-full');
}

export function listAdminProducts(): Promise<AdminProduct[]> {
  return apiFetch<AdminProduct[]>('/api/admin/catalog/products');
}

export function listAdminCategories(): Promise<AdminCategory[]> {
  return apiFetch<AdminCategory[]>('/api/admin/catalog/categories');
}

export function aiCreateProductDraft(input: string): Promise<AiCreateDraftResponse> {
  return apiFetch<AiCreateDraftResponse>('/api/admin/catalog/ai/create-product-draft', {
    method: 'POST',
    body: JSON.stringify({ input })
  });
}

export function aiGenerateProductImage(name: string, description?: string, force = false): Promise<{ image_url: string }> {
  return apiFetch<{ image_url: string }>('/api/admin/catalog/ai/generate-product-image', {
    method: 'POST',
    body: JSON.stringify({ name, description, force })
  });
}

export function createAdminProduct(payload: CreateAdminProductRequest): Promise<AdminProduct> {
  return apiFetch<AdminProduct>('/api/admin/catalog/products', {
    method: 'POST',
    body: JSON.stringify(payload)
  });
}

export function updateAdminProduct(id: string, payload: Partial<CreateAdminProductRequest>): Promise<AdminProduct> {
  return apiFetch<AdminProduct>(`/api/admin/catalog/products/${id}`, {
    method: 'PUT',
    body: JSON.stringify(payload)
  });
}

export function publishAdminProduct(id: string): Promise<AdminProduct> {
  return apiFetch<AdminProduct>(`/api/admin/catalog/products/${id}/publish`, { method: 'POST' });
}

export function unpublishAdminProduct(id: string): Promise<AdminProduct> {
  return apiFetch<AdminProduct>(`/api/admin/catalog/products/${id}/unpublish`, { method: 'POST' });
}

export function deleteAdminProduct(id: string): Promise<void> {
  return apiFetch<void>(`/api/admin/catalog/products/${id}`, { method: 'DELETE' });
}

export function getProductDataQuality(id: string): Promise<ProductDataQuality> {
  return apiFetch<ProductDataQuality>(`/api/admin/catalog/states/data-quality/${id}`);
}

export async function listProductStates(id: string): Promise<IngredientState[]> {
  const result = await apiFetch<{ states: IngredientState[] }>(`/api/admin/catalog/states/products/${id}`);
  return result.states || [];
}

export async function generateProductStates(id: string): Promise<void> {
  await apiFetch(`/api/admin/catalog/states/generate/${id}`, { method: 'POST' });
}

export async function deleteProductStates(id: string): Promise<void> {
  await apiFetch(`/api/admin/catalog/states/products/${id}`, { method: 'DELETE' });
}

export async function updateProductState(productId: string, state: string, payload: Partial<IngredientState>): Promise<IngredientState> {
  const result = await apiFetch<{ state: IngredientState }>(`/api/admin/catalog/states/products/${productId}/states/${state}`, {
    method: 'PUT',
    body: JSON.stringify(payload)
  });
  return result.state;
}

const NUTRITION_SECTION_PATHS: Record<keyof AiExtendedProductProfile, string> = {
  macros: 'macros',
  vitamins: 'vitamins',
  minerals: 'minerals',
  fatty_acids: 'fatty-acids',
  diet_flags: 'diet-flags',
  allergens: 'allergens',
  food_properties: 'food-props',
  culinary: 'culinary',
  health_profile: 'health-profile',
  sugar_profile: 'sugar-profile',
  processing_effects: 'processing-effects',
  culinary_behavior: 'culinary-behavior'
};

export async function saveExtendedProductProfile(productId: string, profile: AiExtendedProductProfile): Promise<void> {
  const requests = Object.entries(NUTRITION_SECTION_PATHS).flatMap(([section, path]) => {
    const payload = profile[section as keyof AiExtendedProductProfile];
    return payload && Object.keys(payload).length > 0
      ? [apiFetch(`/api/admin/nutrition/products/${productId}/${path}`, { method: 'PUT', body: JSON.stringify(payload) })]
      : [];
  });
  await Promise.all(requests);
}
