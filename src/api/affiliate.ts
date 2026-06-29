import { apiFetch } from './client';
import { withDataSource, type SourcedData } from './dataSource';
import type { AffiliateOffer, AffiliateProduct, SiteKey } from '../types/admin';

export function listAffiliateProducts(site?: SiteKey): Promise<AffiliateProduct[]> {
  return apiFetch<AffiliateProduct[]>(`/api/admin/affiliate/products${site ? `?site=${site}` : ''}`);
}

export function listAffiliateProductsWithSource(site?: SiteKey): Promise<SourcedData<AffiliateProduct[]>> {
  return withDataSource(apiFetch<AffiliateProduct[]>(`/api/admin/affiliate/products${site ? `?site=${site}` : ''}`), []);
}

export function getAffiliateProduct(id: string): Promise<AffiliateProduct | null> {
  return apiFetch<AffiliateProduct>(`/api/admin/affiliate/products/${id}`);
}

export function createAffiliateProduct(payload: Partial<AffiliateProduct>): Promise<AffiliateProduct> {
  return apiFetch<AffiliateProduct>('/api/admin/affiliate/products', { method: 'POST', body: JSON.stringify(payload) });
}

export function updateAffiliateProduct(id: string, payload: Partial<AffiliateProduct>): Promise<AffiliateProduct> {
  return apiFetch<AffiliateProduct>(`/api/admin/affiliate/products/${id}`, { method: 'PATCH', body: JSON.stringify(payload) });
}

export function deleteAffiliateProduct(id: string): Promise<void> {
  return apiFetch<void>(`/api/admin/affiliate/products/${id}`, { method: 'DELETE' });
}

export function importAffiliateUrl(url: string, site: SiteKey): Promise<Partial<AffiliateProduct>> {
  return apiFetch<Partial<AffiliateProduct>>('/api/admin/affiliate/import-url', { method: 'POST', body: JSON.stringify({ url, site }) });
}

export function listAffiliateOffers(productId: string): Promise<AffiliateOffer[]> {
  return apiFetch<AffiliateOffer[]>(`/api/admin/affiliate/products/${productId}/offers`);
}
