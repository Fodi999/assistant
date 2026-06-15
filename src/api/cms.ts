import { apiFetch } from './client';
import type { CmsArticle } from '../types/admin';

export interface CmsArticleCategory {
  id: string;
  slug: string;
  title_en: string;
  title_pl: string;
  title_ru: string;
  title_uk: string;
  order_index: number;
  created_at: string;
}

export interface AiArticleDraft {
  slug: string;
  category: string;
  title_en: string;
  title_ru: string;
  title_pl: string;
  title_uk: string;
  content_en: string;
  content_ru: string;
  content_pl: string;
  content_uk: string;
  seo_title: string;
  seo_description: string;
  seo_title_en?: string;
  seo_title_ru?: string;
  seo_title_pl?: string;
  seo_title_uk?: string;
  seo_description_en?: string;
  seo_description_ru?: string;
  seo_description_pl?: string;
  seo_description_uk?: string;
  image_prompts: string[];
}

export interface CmsImageScaleSettings {
  widthCm?: number;
  heightCm?: number;
  depthCm?: number;
  weightKg?: number;
  photoScenarios: string[];
  scaleReference?: string;
  customScaleReference?: string;
}

export interface AboutPageContent {
  id: string;
  title_en: string;
  title_pl: string;
  title_ru: string;
  title_uk: string;
  content_en: string;
  content_pl: string;
  content_ru: string;
  content_uk: string;
  image_url?: string | null;
  updated_at: string | number[];
}

export interface GalleryItem {
  id: string;
  image_url: string;
  category_id?: string | null;
  category_slug?: string | null;
  slug: string;
  status: string;
  title_en: string;
  title_pl: string;
  title_ru: string;
  title_uk: string;
  description_en: string;
  description_pl: string;
  description_ru: string;
  description_uk: string;
  alt_en: string;
  alt_pl: string;
  alt_ru: string;
  alt_uk: string;
  order_index: number;
  updated_at?: string | number[];
}

export function getAboutPage(): Promise<AboutPageContent> {
  return apiFetch<AboutPageContent>('/api/admin/cms/about');
}

export function updateAboutPage(payload: Partial<AboutPageContent>): Promise<AboutPageContent> {
  return apiFetch<AboutPageContent>('/api/admin/cms/about', {
    method: 'PUT',
    body: JSON.stringify(payload)
  });
}

export async function uploadAboutPhoto(file: File): Promise<string> {
  return uploadCmsReference(file);
}

export function listGallery(): Promise<GalleryItem[]> {
  return apiFetch<GalleryItem[]>('/api/admin/cms/gallery');
}

export function createGalleryItem(payload: Partial<GalleryItem> & Pick<GalleryItem, 'image_url'>): Promise<GalleryItem> {
  return apiFetch<GalleryItem>('/api/admin/cms/gallery', {
    method: 'POST',
    body: JSON.stringify(payload)
  });
}

export function updateGalleryItem(id: string, payload: Partial<GalleryItem>): Promise<GalleryItem> {
  return apiFetch<GalleryItem>(`/api/admin/cms/gallery/${id}`, {
    method: 'PUT',
    body: JSON.stringify(payload)
  });
}

export function deleteGalleryItem(id: string): Promise<void> {
  return apiFetch<void>(`/api/admin/cms/gallery/${id}`, { method: 'DELETE' });
}

export async function uploadGalleryPhoto(file: File): Promise<string> {
  return uploadCmsReference(file);
}

export function listArticles(): Promise<CmsArticle[]> {
  return apiFetch<CmsArticle[]>('/api/admin/cms/articles');
}

export function listArticleCategories(): Promise<CmsArticleCategory[]> {
  return apiFetch<CmsArticleCategory[]>('/api/admin/cms/article-categories');
}

export function aiCreateArticleDraft(topic: string, targetChars: number, imageCount: number): Promise<AiArticleDraft> {
  return apiFetch<AiArticleDraft>('/api/admin/cms/articles/ai/draft', {
    method: 'POST',
    body: JSON.stringify({ topic, target_chars: targetChars, image_count: imageCount })
  });
}

export function aiGenerateArticleImage(title: string, prompt: string | undefined, index: number, enhanced = false, referenceUrls: string[] = [], modelPreset = 'flash', scenePreset = 'editorial', scale: CmsImageScaleSettings): Promise<{ image_url: string }> {
  return apiFetch<{ image_url: string }>('/api/admin/cms/articles/ai/images', {
    method: 'POST',
    body: JSON.stringify({
      title,
      prompt,
      index,
      enhanced,
      reference_urls: referenceUrls,
      model_preset: modelPreset,
      scene_preset: scenePreset,
      width_cm: scale.widthCm,
      height_cm: scale.heightCm,
      depth_cm: scale.depthCm,
      weight_kg: scale.weightKg,
      photo_scenarios: scale.photoScenarios,
      scale_reference: scale.scaleReference,
      custom_scale_reference: scale.customScaleReference
    })
  });
}

export async function uploadCmsReference(file: File): Promise<string> {
  const form = new FormData();
  form.append('file', file);
  const result = await apiFetch<{ url: string }>('/api/admin/cms/articles/ai/reference-upload', {
    method: 'POST',
    body: form
  });
  return result.url;
}

export function createArticle(payload: Omit<CmsArticle, 'id' | 'updated_at'>): Promise<CmsArticle> {
  return apiFetch<CmsArticle>('/api/admin/cms/articles', {
    method: 'POST',
    body: JSON.stringify(payload)
  });
}

export function updateArticle(id: string, payload: Partial<CmsArticle>): Promise<CmsArticle> {
  return apiFetch<CmsArticle>(`/api/admin/cms/articles/${id}`, {
    method: 'PUT',
    body: JSON.stringify(payload)
  });
}

export function deleteArticle(id: string): Promise<void> {
  return apiFetch<void>(`/api/admin/cms/articles/${id}`, { method: 'DELETE' });
}

export function getUploadUrl(folder: string, contentType: string): Promise<{ upload_url: string; url: string }> {
  const query = new URLSearchParams({
    folder,
    content_type: contentType
  });
  return apiFetch<{ upload_url: string; url: string }>(`/api/admin/cms/upload-url?${query.toString()}`);
}
