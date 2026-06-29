import type { LanguageCode, ResourceStatus, SiteId } from './admin';

type AdminResourceBaseDto = {
  siteId: SiteId;
  title?: string;
  slug?: string;
  type?: string;
  status?: ResourceStatus;
  owner?: string;
  metric?: string;
  language?: LanguageCode;
};

type AdminResourceUpdateDto = Partial<Omit<AdminResourceBaseDto, 'siteId'>> & {
  siteId?: SiteId;
};

export type LocalizedAdminTextDto = {
  uk?: string;
  ru?: string;
  en?: string;
};

export type CreateCatalogItemDto = AdminResourceBaseDto & {
  name?: LocalizedAdminTextDto;
  description?: LocalizedAdminTextDto;
  imageUrl?: string;
  productType?: string;
  unit?: string;
  caloriesPer100g?: number;
  proteinPer100g?: number;
  fatPer100g?: number;
  carbsPer100g?: number;
  fiberPer100g?: number;
  sugarPer100g?: number;
  densityGPerMl?: number;
  typicalPortionG?: number;
  shelfLifeDays?: number;
  seasons?: string[];
  seoTitle?: string;
  seoDescription?: string;
  seoH1?: string;
};
export type UpdateCatalogItemDto = AdminResourceUpdateDto & Partial<Omit<CreateCatalogItemDto, 'siteId'>>;

export type CreateCMSPageDto = AdminResourceBaseDto & {
  localizedTitle?: LocalizedAdminTextDto;
  content?: string;
  contentLocalized?: LocalizedAdminTextDto;
  imageUrl?: string;
  authorName?: string;
  seoTitle?: LocalizedAdminTextDto;
  seoDescription?: LocalizedAdminTextDto;
  publishedAt?: string | null;
  orderIndex?: number;
};
export type UpdateCMSPageDto = AdminResourceUpdateDto & Partial<Omit<CreateCMSPageDto, 'siteId'>>;

export type CreateShopProductDto = AdminResourceBaseDto & {
  name?: LocalizedAdminTextDto;
  shortDescription?: LocalizedAdminTextDto;
  description?: LocalizedAdminTextDto;
  seoTitle?: LocalizedAdminTextDto;
  seoDescription?: LocalizedAdminTextDto;
  sku?: string | null;
  priceCents?: number | null;
  currency?: string;
  stockQuantity?: number;
  imageUrls?: string[];
  sellingPoints?: string[];
};
export type UpdateShopProductDto = AdminResourceUpdateDto & Partial<Omit<CreateShopProductDto, 'siteId'>>;

export type CreateLeadDto = AdminResourceBaseDto & {
  clientName?: string;
  contact?: string;
  source?: string;
};
export type UpdateLeadDto = AdminResourceUpdateDto & {
  clientName?: string;
  contact?: string;
  source?: string;
};

export type CreateSupplierDto = AdminResourceBaseDto & {
  name?: string;
  country?: string;
  city?: string;
  currency?: string;
  categories?: string[];
  contact?: string;
  website?: string;
  commissionTerms?: string;
  supplierType?: string;
};
export type UpdateSupplierDto = AdminResourceUpdateDto & {
  name?: string;
  country?: string;
  city?: string;
  currency?: string;
  categories?: string[];
  contact?: string;
  website?: string;
  commissionTerms?: string;
  supplierType?: string;
};

export type UpdateSiteSettingsDto = {
  name?: string;
  domain?: string;
  defaultLanguage?: LanguageCode;
  ga4Id?: string;
  searchConsoleProperty?: string;
  apiUrl?: string;
  status?: ResourceStatus;
};
