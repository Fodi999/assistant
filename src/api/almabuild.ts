import { apiFetch } from './client';

export const almabuildSiteUrl = String(import.meta.env.VITE_ALMABUILD_SITE_URL || 'http://localhost:3000').replace(/\/+$/, '');

export type MaterialCategory = {
  index: string;
  slug: string;
  title: string;
  text: string;
  bullets: string[];
  photo: string;
};

export type Product = {
  categorySlug: string;
  category: string;
  title: string;
  spec: string;
  photo: string;
};

export type Kit = {
  title: string;
  text: string;
  items: string[];
};

export type Project = {
  title: string;
  meta: string;
  photo: string;
};

export type AlmabuildContent = {
  materialCategories: MaterialCategory[];
  products: Product[];
  kits: Kit[];
  projects: Project[];
};

export function getAlmabuildContent(): Promise<AlmabuildContent> {
  return apiFetch<AlmabuildContent>('/api/admin/almabuild/content');
}

export function saveAlmabuildContent(content: AlmabuildContent): Promise<AlmabuildContent> {
  return apiFetch<AlmabuildContent>('/api/admin/almabuild/content', {
    method: 'PUT',
    body: JSON.stringify(content)
  });
}


export type AlmabuildAiKind = 'material' | 'product' | 'kit' | 'project';

export function aiEditAlmabuildItem<T>(kind: AlmabuildAiKind, instruction: string, value: T): Promise<T> {
  return apiFetch<T>('/api/admin/almabuild/ai/edit', {
    method: 'POST',
    body: JSON.stringify({ kind, instruction, value })
  });
}
