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
    'hero 3/4 view on a clean matte slate plate with small pickled ginger and wasabi side garnish',
    'top view on a clean white ceramic plate with small pickled ginger and wasabi side garnish',
    'close-up macro detail of neat cut surfaces and toppings, with ginger and wasabi visible only at the edge',
    'premium menu catalog angle with soft natural shadows and a small ginger-wasabi garnish'
  ][index] || 'premium clean product catalog photo';

  return [
    `Product: ${title}.`,
    prompt || '',
    `Shot: ${shot}.`,
    'Use the uploaded reference image as the source of truth.',
    'Preserve the exact sushi roll identity, ingredients, roll count, cut shape, proportions and topping placement from the reference.',
    'Make the rolls straight, symmetrical, cleanly cut, appetizing and realistic.',
    'Add a small, realistic portion of pickled ginger and wasabi beside the sushi as a clean side garnish; keep it separate from the rolls and never let it cover the product.',
    'No delivery box, no plastic container, no packaging, no cardboard, no hands, no chopsticks, no extra sauce cup unless it is clearly present in the reference.',
    'No distorted rolls, no melted rice, no duplicated ingredients, no broken geometry, no extra text, no logos.',
    'High-resolution professional food photography, natural color, sharp focus, premium Dima Fomin catalog style.'
  ].filter(Boolean).join('\n');
}

export function generateShopProductImage(title: string, prompt: string | undefined, index: number, referenceUrls: string[], enhanced: boolean, scale: CmsImageScaleSettings): Promise<{ image_url: string }> {
  return aiGenerateArticleImage(title, shopImagePrompt(title, prompt, index), index, enhanced, referenceUrls, enhanced ? 'pro' : 'flash', 'product-white', scale);
}

export { uploadCmsReference as uploadShopReference };
