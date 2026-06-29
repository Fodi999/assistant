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

function shopImagePrompt(title: string, prompt: string | undefined, index: number): string {
  const shot = [
    'hero 3/4 catalog view, same product arrangement as the reference',
    'top catalog view, same product arrangement as the reference',
    'safe closer crop of the same product, no changed geometry',
    'premium menu catalog angle, same product arrangement as the reference'
  ][index] || 'premium clean product catalog photo';

  return [
    `Product: ${title}.`,
    prompt || '',
    `Shot: ${shot}.`,
    'Use the uploaded reference image as the source of truth.',
    'This is a reference product retouch and background cleanup task, not a creative redesign.',
    'Preserve the exact sushi roll identity, ingredients, roll count, cut shape, proportions and topping placement from the reference.',
    'Keep the exact number of pieces from the reference. Do not add, remove, merge, split or duplicate pieces.',
    'Keep the exact filling layout, rice layer, nori/seaweed line, fish/cheese/vegetable colors and topping placement from the reference.',
    'Make the rolls straight, symmetrical, cleanly cut, appetizing and realistic, with each piece having believable equal size.',
    'No delivery box, no plastic container, no packaging, no cardboard, no hands, no chopsticks, no extra sauce cup unless it is clearly present in the reference.',
    'No new ingredients, no decorative garnish, no roe/topping changes unless already present in the reference.',
    'No distorted rolls, no melted rice, no fused pieces, no duplicated ingredients, no broken geometry, no extra text, no logos.',
    'High-resolution professional food photography, natural color, sharp focus, premium Dima Fomin catalog style.'
  ].filter(Boolean).join('\n');
}

export function generateShopProductImage(title: string, prompt: string | undefined, index: number, referenceUrls: string[], enhanced: boolean, scale: CmsImageScaleSettings): Promise<{ image_url: string }> {
  const useProModel = enhanced || referenceUrls.length > 0;
  return aiGenerateArticleImage(title, shopImagePrompt(title, prompt, index), index, useProModel, referenceUrls, useProModel ? 'pro' : 'flash', 'product-white', scale);
}

export { uploadCmsReference as uploadShopReference };
