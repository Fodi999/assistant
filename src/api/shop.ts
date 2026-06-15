import { apiFetch } from './client';
import { aiGenerateArticleImage, uploadCmsReference, type CmsImageScaleSettings } from './cms';
import type { ShopProduct, ShopProductDraft } from '../types/admin';

export function listShopProducts(): Promise<ShopProduct[]> {
  return apiFetch<ShopProduct[]>('/api/admin/cms/shop-products');
}

export function aiCreateShopProductDraft(product: string, imageCount: number): Promise<ShopProductDraft> {
  return apiFetch<ShopProductDraft>('/api/admin/cms/shop-products/ai/draft', {
    method: 'POST',
    body: JSON.stringify({ product, image_count: imageCount })
  });
}

export function createShopProduct(product: Omit<ShopProduct, 'id' | 'created_at' | 'updated_at'>): Promise<ShopProduct> {
  return apiFetch<ShopProduct>('/api/admin/cms/shop-products', {
    method: 'POST',
    body: JSON.stringify(product)
  });
}

export function updateShopProductStatus(id: string, status: ShopProduct['status']): Promise<ShopProduct> {
  return apiFetch<ShopProduct>(`/api/admin/cms/shop-products/${id}/status`, {
    method: 'PUT',
    body: JSON.stringify({ status })
  });
}

export function deleteShopProduct(id: string): Promise<void> {
  return apiFetch<void>(`/api/admin/cms/shop-products/${id}`, { method: 'DELETE' });
}

export function generateShopProductImage(title: string, prompt: string | undefined, index: number, referenceUrls: string[], enhanced: boolean, scale: CmsImageScaleSettings): Promise<{ image_url: string }> {
  return aiGenerateArticleImage(title, prompt, index, enhanced, referenceUrls, enhanced ? 'pro' : 'flash', 'delivery-product', scale);
}

export { uploadCmsReference as uploadShopReference };
