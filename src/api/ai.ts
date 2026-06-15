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
  imageType?: 'auto' | 'construction' | 'article' | 'review' | 'product';
}

export interface AiImageResult {
  id: string;
  imageUrl: string;
  prompt: string;
}

function mockResult(request: AiGenerationRequest): AiGenerationResult {
  const base = request.sourceText || request.keywords?.join(' ') || 'affiliate product';
  return {
    id: `ai-${Date.now()}`,
    request,
    title: request.type === 'seo' ? `${base} | экспертный обзор и цены` : undefined,
    description: `AI draft for ${request.site} in ${request.language}: ${base}. Проверь факты, офферы и локальные нюансы перед публикацией.`,
    slug: base.toLowerCase().replace(/[^a-z0-9а-яё]+/gi, '-').replace(/^-|-$/g, '').slice(0, 72),
    photoPrompt: request.type === 'photo_prompt' ? `Professional product photo, ${base}, clean dark SaaS admin preview, realistic lighting` : undefined,
    qualityScore: request.type === 'quality_check' ? 82 : undefined,
    suggestions: ['Добавить цену и валюту', 'Проверить affiliate URL', 'Сгенерировать SEO description для всех языков'],
    createdAt: new Date().toISOString()
  };
}

async function safeGenerate(path: string, payload: AiGenerationRequest): Promise<AiGenerationResult> {
  try {
    return await apiFetch<AiGenerationResult>(path, { method: 'POST', body: JSON.stringify(payload) });
  } catch {
    return mockResult(payload);
  }
}

export const generateAffiliateProduct = (payload: AiGenerationRequest) => safeGenerate('/api/admin/ai/affiliate-product', payload);
export const generateSeo = (payload: AiGenerationRequest) => safeGenerate('/api/admin/ai/seo', payload);
export const generatePhotoPrompt = (payload: AiGenerationRequest) => safeGenerate('/api/admin/ai/photo-prompt', payload);
export const improveProductCard = (payload: AiGenerationRequest) => safeGenerate('/api/admin/ai/improve-product-card', payload);

export async function analyzePhotoWithGemini(file: File, site: SiteKey, language: LanguageCode, instruction: string): Promise<AiVisionResult> {
  const form = new FormData();
  form.append('image', file);
  form.append('site', site);
  form.append('language', language);
  form.append('instruction', instruction);
  try {
    return await apiFetch<AiVisionResult>('/api/admin/ai/vision/photo', { method: 'POST', body: form });
  } catch {
    const fallback = mockResult({ site, language, type: 'quality_check', sourceText: instruction });
    return {
      id: fallback.id,
      site,
      title: fallback.title || file.name,
      description: fallback.description,
      category: site === 'construction' ? 'Материалы' : 'Affiliate / обзор',
      slug: fallback.slug,
      priceHint: 'Проверить цену и поставщика вручную',
      materials: [],
      affiliateProducts: [],
      articleIdeas: [],
      suggestions: fallback.suggestions,
      raw: fallback
    };
  }
}

export async function generateAiImage(payload: AiImageRequest): Promise<AiImageResult> {
  try {
    return await apiFetch<AiImageResult>('/api/admin/ai/image', { method: 'POST', body: JSON.stringify(payload) });
  } catch {
    return {
      id: `img-${Date.now()}`,
      imageUrl: '',
      prompt: payload.scene || `Commercial image for ${payload.title}`
    };
  }
}
