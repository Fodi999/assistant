import { apiFetch } from './client';
import { affiliateProducts, contentArticles } from '../lib/mockData';
import type { AffiliateProduct, ContentArticle } from '../types/admin';

export async function listCulinaryProducts(): Promise<AffiliateProduct[]> {
  try {
    return await apiFetch<AffiliateProduct[]>('/api/admin/culinary/products');
  } catch {
    return affiliateProducts.filter((product) => product.site === 'culinary');
  }
}

export async function listRecipes(): Promise<ContentArticle[]> {
  try {
    return await apiFetch<ContentArticle[]>('/api/admin/culinary/recipes');
  } catch {
    return contentArticles.filter((article) => article.site === 'culinary' && (article.type === 'recipe' || article.type === 'article'));
  }
}

export async function listReviews(): Promise<ContentArticle[]> {
  try {
    return await apiFetch<ContentArticle[]>('/api/admin/culinary/reviews');
  } catch {
    return contentArticles.filter((article) => article.site === 'culinary' && article.type === 'review');
  }
}
