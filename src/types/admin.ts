export interface LoginResponse {
  token: string;
  expires_in: number;
}

export interface ApiErrorPayload {
  message?: string;
  error?: string;
}

export interface CmsArticle {
  id: string;
  slug: string;
  category?: string;
  title_ru?: string | null;
  title_en?: string | null;
  title_pl?: string | null;
  title_uk?: string | null;
  content_en?: string | null;
  content_ru?: string | null;
  content_pl?: string | null;
  content_uk?: string | null;
  image_url?: string | null;
  author_name?: string | null;
  author_avatar_url?: string | null;
  author_avatar_position?: string | null;
  seo_title?: string | null;
  seo_description?: string | null;
  seo_title_en?: string | null;
  seo_title_ru?: string | null;
  seo_title_pl?: string | null;
  seo_title_uk?: string | null;
  seo_description_en?: string | null;
  seo_description_ru?: string | null;
  seo_description_pl?: string | null;
  seo_description_uk?: string | null;
  published?: boolean;
  published_at?: string | number[] | null;
  updated_at?: string | number[];
}

export interface CmsArticleListResponse {
  items: CmsArticle[];
  total: number;
}

export interface AdminStats {
  total_users: number;
  total_restaurants: number;
}

export interface AdminUser {
  id: string;
  email: string;
  name: string | null;
  restaurant_name: string;
  language: string;
  created_at: string;
  login_count: number;
  last_login_at: string | null;
}

export interface AdminUsersResponse {
  users: AdminUser[];
  total: number;
}

export interface AdminCategory {
  id: string;
  name_en: string;
  name_ru: string;
  name_pl: string;
  name_uk: string;
  sort_order: number;
}

export interface AdminProduct {
  id: string;
  slug?: string | null;
  name_en: string;
  name_ru?: string | null;
  name_pl?: string | null;
  name_uk?: string | null;
  category_id: string;
  unit: string;
  description?: string | null;
  description_en?: string | null;
  description_ru?: string | null;
  description_pl?: string | null;
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
  product_type?: string | null;
  seasons?: string[];
  seo_title?: string | null;
  seo_description?: string | null;
  seo_h1?: string | null;
  is_published: boolean;
}

export interface RecordSaleRequest {
  dish_id: string;
  quantity: number;
  selling_price_cents: number;
  recipe_cost_cents: number;
}

export interface PublicIngredient {
  slug: string;
  name_en: string;
  name_ru: string;
  name_pl: string;
  description_en?: string | null;
  description_ru?: string | null;
  description_pl?: string | null;
  image_url?: string | null;
  category_name_en?: string | null;
  category_name_ru?: string | null;
  category_name_pl?: string | null;
  calories_per_100g?: number | null;
  protein_per_100g?: number | null;
  fat_per_100g?: number | null;
  carbs_per_100g?: number | null;
}

export interface PublicIngredientListResponse {
  items: PublicIngredient[];
  total: number;
}

export interface ShopProductDraft {
  slug: string;
  category: string;
  name_en: string;
  name_ru: string;
  name_pl: string;
  name_uk: string;
  short_description_en: string;
  short_description_ru: string;
  short_description_pl: string;
  short_description_uk: string;
  description_en: string;
  description_ru: string;
  description_pl: string;
  description_uk: string;
  seo_title_en: string;
  seo_title_ru: string;
  seo_title_pl: string;
  seo_title_uk: string;
  seo_description_en: string;
  seo_description_ru: string;
  seo_description_pl: string;
  seo_description_uk: string;
  selling_points: string[];
  image_prompts: string[];
}

export interface ShopProduct extends ShopProductDraft {
  id: string;
  sku: string | null;
  image_urls: string[];
  price_cents: number | null;
  currency: string;
  stock_quantity: number;
  status: 'draft' | 'active' | 'archived';
  created_at: string | number[];
  updated_at: string | number[];
}
