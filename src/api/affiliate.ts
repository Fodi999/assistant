import { apiFetch } from './client';
import { withDataSource, type SourcedData } from './dataSource';
import { affiliateOffers, affiliateProducts } from '../lib/mockData';
import type { AffiliateOffer, AffiliateProduct, SiteKey } from '../types/admin';

async function fallback<T>(request: Promise<T>, value: T): Promise<T> {
  try {
    return await request;
  } catch {
    return value;
  }
}

export function listAffiliateProducts(site?: SiteKey): Promise<AffiliateProduct[]> {
  const mock = site ? affiliateProducts.filter((product) => product.site === site) : affiliateProducts;
  return fallback(apiFetch<AffiliateProduct[]>(`/api/admin/affiliate/products${site ? `?site=${site}` : ''}`), mock);
}

export function listAffiliateProductsWithSource(site?: SiteKey): Promise<SourcedData<AffiliateProduct[]>> {
  const mock = site ? affiliateProducts.filter((product) => product.site === site) : affiliateProducts;
  return withDataSource(apiFetch<AffiliateProduct[]>(`/api/admin/affiliate/products${site ? `?site=${site}` : ''}`), mock);
}

export function getAffiliateProduct(id: string): Promise<AffiliateProduct | null> {
  return fallback(apiFetch<AffiliateProduct>(`/api/admin/affiliate/products/${id}`), affiliateProducts.find((product) => product.id === id) ?? null);
}

export function createAffiliateProduct(payload: Partial<AffiliateProduct>): Promise<AffiliateProduct> {
  const mock: AffiliateProduct = { ...affiliateProducts[0], ...payload, id: `local-${Date.now()}` } as AffiliateProduct;
  return fallback(apiFetch<AffiliateProduct>('/api/admin/affiliate/products', { method: 'POST', body: JSON.stringify(payload) }), mock);
}

export function updateAffiliateProduct(id: string, payload: Partial<AffiliateProduct>): Promise<AffiliateProduct> {
  const current = affiliateProducts.find((product) => product.id === id) ?? affiliateProducts[0];
  return fallback(apiFetch<AffiliateProduct>(`/api/admin/affiliate/products/${id}`, { method: 'PATCH', body: JSON.stringify(payload) }), { ...current, ...payload });
}

export function deleteAffiliateProduct(id: string): Promise<void> {
  return fallback(apiFetch<void>(`/api/admin/affiliate/products/${id}`, { method: 'DELETE' }), undefined);
}

export function importAffiliateUrl(url: string, site: SiteKey): Promise<Partial<AffiliateProduct>> {
  return fallback(apiFetch<Partial<AffiliateProduct>>('/api/admin/affiliate/import-url', { method: 'POST', body: JSON.stringify({ url, site }) }), {
    site,
    affiliateUrl: url,
    merchant: new URL(url).hostname.replace(/^www\./, ''),
    network: 'custom'
  });
}

export function listAffiliateOffers(productId: string): Promise<AffiliateOffer[]> {
  return fallback(apiFetch<AffiliateOffer[]>(`/api/admin/affiliate/products/${productId}/offers`), affiliateOffers.filter((offer) => offer.productId === productId));
}
