export interface LoginResponse {
  token: string;
  expires_in: number;
}

export interface ApiErrorPayload {
  message?: string;
  error?: string;
}

export type IconsSection = 'calendar' | 'icons' | 'prayers' | 'saints' | 'gospel' | 'qr' | 'seo' | 'churches';

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
  siteId?: SiteId;
  title?: string;
  type?: string;
  owner?: string;
  updated?: string;
  metric?: string;
  language?: LanguageCode;
  sku: string | null;
  image_urls: string[];
  price_cents: number | null;
  currency: string;
  stock_quantity: number;
  status: 'draft' | 'active' | 'archived';
  created_at: string | number[];
  updated_at: string | number[];
}

export type SiteKey = 'culinary' | 'construction' | 'icons';
export type SiteId = 'church' | 'construction' | 'kitchen';
export type ResourceStatus = 'active' | 'published' | 'draft' | 'archived' | 'new' | 'warning' | 'neutral';
export type AlmabuildSection = 'services' | 'materials' | 'catalog' | 'projects' | 'estimate' | 'contact';
export type LanguageCode = 'ru' | 'pl' | 'en' | 'kk';
export type CurrencyCode = 'PLN' | 'KZT' | 'EUR' | 'USD';
export type AffiliateNetwork = 'amazon' | 'allegro' | 'ceneo' | 'awin' | 'custom';
export type PublishStatus = 'draft' | 'active' | 'published' | 'archived';
export type ContentType = 'article' | 'review' | 'comparison' | 'roundup' | 'recipe';
export type LeadStatus = 'new' | 'contacted' | 'quoted' | 'won' | 'lost';
export type SupplierType = 'marketplace' | 'local_supplier' | 'manufacturer' | 'affiliate_merchant';
export type AiGenerationType = 'product_description' | 'seo' | 'slug' | 'photo_prompt' | 'translation' | 'quality_check';

export type LocalizedText = Record<LanguageCode, string>;

export interface AdminResourceRow {
  id: string;
  siteId: SiteId;
  title: string;
  slug?: string;
  type: string;
  status: ResourceStatus;
  owner: string;
  updated: string;
  updatedAt?: string;
  metric: string;
  language?: LanguageCode;
  backend?: unknown;
}

export interface CatalogItem extends AdminResourceRow {
  resource: 'catalog';
}

export interface CMSPageItem extends AdminResourceRow {
  resource: 'cms';
}

export interface Order extends AdminResourceRow {
  resource: 'orders';
}

export interface User extends AdminResourceRow {
  resource: 'users';
  email?: string;
  role?: string;
}

export interface AnalyticsRow extends AdminResourceRow {
  resource: 'analytics';
}

export interface SiteSettings {
  siteId: SiteId;
  name: string;
  domain: string;
  defaultLanguage: LanguageCode;
  ga4Id: string;
  searchConsoleProperty: string;
  apiUrl: string;
  status: ResourceStatus;
}

export interface SiteConfig {
  key: SiteKey;
  name: string;
  domain: string;
  primaryLanguage: LanguageCode;
  languages: LanguageCode[];
  status: PublishStatus;
  apiStatus: 'online' | 'limited' | 'offline';
  revalidateStatus: 'ready' | 'queued' | 'failed';
  defaultCurrency: CurrencyCode;
  region: string;
}

export interface AffiliateOffer {
  id: string;
  productId: string;
  network: AffiliateNetwork;
  merchant: string;
  affiliateUrl: string;
  price?: number;
  currency: CurrencyCode;
  commissionPercent?: number;
  cookieDays?: number;
  isActive: boolean;
}

export interface AffiliateProduct {
  id: string;
  site: SiteKey;
  title: LocalizedText;
  slug: string;
  category: string;
  network: AffiliateNetwork;
  merchant: string;
  affiliateUrl: string;
  imageUrl?: string;
  detailImageUrl?: string;
  price?: number;
  currency: CurrencyCode;
  commissionPercent?: number;
  cookieDays?: number;
  status: PublishStatus;
  languages: LanguageCode[];
  seoTitle?: LocalizedText;
  seoDescription?: LocalizedText;
  offers?: AffiliateOffer[];
  createdAt?: string;
  updatedAt?: string;
}

export interface ContentArticle {
  id: string;
  site: SiteKey;
  type: ContentType;
  title: LocalizedText;
  slug: string;
  excerpt: LocalizedText;
  status: PublishStatus;
  languages: LanguageCode[];
  affiliateProductIds: string[];
  seoTitle?: LocalizedText;
  seoDescription?: LocalizedText;
  publishedAt?: string;
  updatedAt?: string;
}

export interface ConstructionMaterial {
  id: string;
  title: LocalizedText;
  slug: string;
  category: string;
  city: string;
  supplierIds: string[];
  unit: 'm2' | 'm3' | 'piece' | 'kg' | 'bag' | 'hour';
  materialPrice?: number;
  workPrice?: number;
  currency: CurrencyCode;
  marginPercent?: number;
  status: PublishStatus;
}

export interface ConstructionCalculatorPreset {
  id: string;
  title: LocalizedText;
  city: string;
  areaM2: number;
  materialCost: number;
  workCost: number;
  marginPercent: number;
  totalPrice: number;
  currency: CurrencyCode;
  updatedAt?: string;
}

export interface ConstructionBundle {
  id: string;
  title: LocalizedText;
  slug: string;
  city: string;
  materials: string[];
  works: string[];
  areaM2?: number;
  materialCost?: number;
  workCost?: number;
  totalPrice?: number;
  currency: CurrencyCode;
  supplierIds: string[];
  leadFormEnabled: boolean;
  status: PublishStatus;
}

export interface Lead {
  id: string;
  siteId?: SiteId;
  title?: string;
  type?: string;
  owner?: string;
  updated?: string;
  metric?: string;
  language?: LanguageCode;
  clientName: string;
  contact: string;
  sourceSite: SiteKey;
  category: string;
  city?: string;
  message: string;
  status: LeadStatus;
  potentialValue?: number;
  currency: CurrencyCode;
  createdAt: string;
}

export interface Supplier {
  id: string;
  siteId?: SiteId;
  title?: string;
  status?: ResourceStatus;
  owner?: string;
  updated?: string;
  metric?: string;
  language?: LanguageCode;
  name: string;
  country: string;
  city?: string;
  categories: string[];
  contact: string;
  website?: string;
  commissionTerms?: string;
  type: SupplierType;
}

export interface AiGenerationRequest {
  site: SiteKey;
  language: LanguageCode;
  type: AiGenerationType;
  sourceText?: string;
  productId?: string;
  tone?: 'commercial' | 'expert' | 'seo' | 'short';
  keywords?: string[];
}

export interface AiGenerationResult {
  id: string;
  request: AiGenerationRequest;
  title?: string;
  description?: string;
  slug?: string;
  photoPrompt?: string;
  qualityScore?: number;
  suggestions: string[];
  createdAt: string;
}

export interface SiteDashboardMetrics {
  site: SiteKey;
  visitors: number;
  affiliateClicks: number;
  leads: number;
  revenueEstimate: number;
  currency: CurrencyCode;
  publishedPages: number;
  aiDrafts: number;
  seoStatus: 'good' | 'needs_work' | 'critical';
  topPages: Array<{ title: string; path: string; visitors: number; ctr: number }>;
  topProducts: Array<{ productId: string; title: string; clicks: number; revenue: number }>;
  recentLeads: Lead[];
  seoTasks: Array<{ title: string; priority: 'high' | 'medium' | 'low'; status: PublishStatus }>;
}
