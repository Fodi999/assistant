import { apiFetch } from './client';
import type { AffiliateProduct, ContentArticle } from '../types/admin';

export function listCulinaryProducts(): Promise<AffiliateProduct[]> {
  return apiFetch<AffiliateProduct[]>('/api/admin/culinary/products');
}

export function listRecipes(): Promise<ContentArticle[]> {
  return apiFetch<ContentArticle[]>('/api/admin/culinary/recipes');
}

export function listReviews(): Promise<ContentArticle[]> {
  return apiFetch<ContentArticle[]>('/api/admin/culinary/reviews');
}
