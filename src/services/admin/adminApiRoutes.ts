import type { SiteId } from '../../types/admin';
import type { AdminResourceKey } from './mockData';

type AdminResourceRoutes = {
  list: (siteId: SiteId) => string;
  getById: (id: string, siteId: SiteId) => string | null;
  create: (siteId: SiteId) => string | null;
  update: (id: string, siteId: SiteId) => string | null;
  remove: (id: string, siteId: SiteId) => string | null;
};

function encode(value: string) {
  return encodeURIComponent(value);
}

export const adminSiteUuidById: Record<SiteId, string> = {
  church: '00000000-0000-0000-0000-000000000101',
  construction: '00000000-0000-0000-0000-000000000102',
  kitchen: '00000000-0000-0000-0000-000000000103'
};

export function siteIdToApiParam(siteId: SiteId): string {
  return `site_id=${encode(adminSiteUuidById[siteId])}`;
}

function withSite(path: string, siteId: SiteId): string {
  return `${path}${path.includes('?') ? '&' : '?'}${siteIdToApiParam(siteId)}`;
}

export const adminApiRoutes: Record<AdminResourceKey, AdminResourceRoutes> = {
  catalog: {
    list: (siteId) => withSite('/api/admin/catalog/products', siteId),
    getById: (id, siteId) => withSite(`/api/admin/catalog/products/${encode(id)}`, siteId),
    create: (siteId) => withSite('/api/admin/catalog/products', siteId),
    update: (id, siteId) => withSite(`/api/admin/catalog/products/${encode(id)}`, siteId),
    remove: (id, siteId) => withSite(`/api/admin/catalog/products/${encode(id)}`, siteId)
  },
  cms: {
    list: (siteId) => withSite('/api/admin/cms/articles', siteId),
    getById: (id, siteId) => withSite(`/api/admin/cms/articles/${encode(id)}`, siteId),
    create: (siteId) => withSite('/api/admin/cms/articles', siteId),
    update: (id, siteId) => withSite(`/api/admin/cms/articles/${encode(id)}`, siteId),
    remove: (id, siteId) => withSite(`/api/admin/cms/articles/${encode(id)}`, siteId)
  },
  shop: {
    list: (siteId) => withSite('/api/admin/cms/shop-products', siteId),
    getById: (id, siteId) => withSite(`/api/admin/cms/shop-products/${encode(id)}`, siteId),
    create: (siteId) => withSite('/api/admin/cms/shop-products', siteId),
    update: (id, siteId) => withSite(`/api/admin/cms/shop-products/${encode(id)}`, siteId),
    remove: (id, siteId) => withSite(`/api/admin/cms/shop-products/${encode(id)}`, siteId)
  },
  orders: {
    list: () => '/api/admin/orders',
    getById: (id) => `/api/admin/orders/${encode(id)}`,
    create: () => '/api/admin/orders',
    update: (id) => `/api/admin/orders/${encode(id)}`,
    remove: (id) => `/api/admin/orders/${encode(id)}`
  },
  suppliers: {
    list: (siteId) => withSite('/api/admin/suppliers', siteId),
    getById: () => null,
    create: (siteId) => withSite('/api/admin/suppliers', siteId),
    update: (id, siteId) => withSite(`/api/admin/suppliers/${encode(id)}`, siteId),
    remove: () => null
  },
  leads: {
    list: (siteId) => withSite('/api/admin/leads', siteId),
    getById: () => null,
    create: () => null,
    update: (id, siteId) => withSite(`/api/admin/leads/${encode(id)}/status`, siteId),
    remove: () => null
  },
  users: {
    list: () => '/api/admin/users',
    getById: () => null,
    create: () => null,
    update: () => null,
    remove: (id) => `/api/admin/users/${encode(id)}`
  },
  analytics: {
    list: (siteId) => withSite('/api/admin/analytics/overview', siteId),
    getById: () => null,
    create: () => null,
    update: () => null,
    remove: () => null
  },
  settings: {
    list: (siteId) => withSite('/api/admin/dashboard', siteId),
    getById: (id) => withSite('/api/admin/dashboard', id as SiteId),
    create: () => null,
    update: () => null,
    remove: () => null
  }
};

export const adminHealthRoute = '/health';
export const adminVersionRoute = '/api/admin/version';
export const adminCatalogCategoriesRoute = '/api/admin/catalog/categories';
export const adminAnalyticsConnectionRoute = (siteId: SiteId) => withSite('/api/admin/analytics/connection', siteId);
export const adminAnalyticsOAuthUrlRoute = (siteId: SiteId) => withSite('/api/admin/analytics/oauth/url', siteId);
export const adminAnalyticsRealtimeRoute = (siteId: SiteId) => withSite('/api/admin/analytics/realtime', siteId);
export const adminDashboardRoute = (siteId: SiteId) => withSite('/api/admin/dashboard', siteId);
export const adminStatsRoute = '/api/admin/stats';
