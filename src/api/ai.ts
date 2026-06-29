import { apiFetch } from './client';
import type { AiGenerationRequest, AiGenerationResult, SiteKey, LanguageCode } from '../types/admin';

export interface AiVisionResult {
  id: string;
  site: SiteKey;
  imageUrl?: string | null;
  title?: string | null;
  description?: string | null;
  category?: string | null;
  slug?: string | null;
  priceHint?: string | null;
  materials: unknown[];
  affiliateProducts: unknown[];
  articleIdeas: unknown[];
  seoTitle?: string | null;
  seoDescription?: string | null;
  suggestions: string[];
  raw: unknown;
}

export interface AiImageRequest {
  site: SiteKey;
  title: string;
  description?: string;
  scene?: string;
  imageType?: 'auto' | 'construction' | 'article' | 'review' | 'product' | 'calendar';
  referenceUrls?: string[];
  variant?: number;
  enhanced?: boolean;
}

export interface AiImageResult {
  id: string;
  imageUrl: string;
  prompt: string;
  imageModel?: string;
}

function generate(path: string, payload: AiGenerationRequest): Promise<AiGenerationResult> {
  return apiFetch<AiGenerationResult>(path, { method: 'POST', body: JSON.stringify(payload) });
}

export const generateAffiliateProduct = (payload: AiGenerationRequest) => generate('/api/admin/ai/affiliate-product', payload);
export const generateSeo = (payload: AiGenerationRequest) => generate('/api/admin/ai/seo', payload);
export const generatePhotoPrompt = (payload: AiGenerationRequest) => generate('/api/admin/ai/photo-prompt', payload);
export const improveProductCard = (payload: AiGenerationRequest) => generate('/api/admin/ai/improve-product-card', payload);

export async function analyzePhotoWithGemini(file: File, site: SiteKey, language: LanguageCode, instruction: string): Promise<AiVisionResult> {
  const form = new FormData();
  form.append('image', file);
  form.append('site', site);
  form.append('language', language);
  form.append('instruction', instruction);
  return apiFetch<AiVisionResult>('/api/admin/ai/vision/photo', { method: 'POST', body: form });
}

export function generateAiImage(payload: AiImageRequest): Promise<AiImageResult> {
  return apiFetch<AiImageResult>('/api/admin/ai/image', { method: 'POST', body: JSON.stringify(payload) });
}
