import { apiFetch } from './client';

export const almabuildSiteUrl = String(import.meta.env.VITE_ALMABUILD_SITE_URL || 'https://kazaxbud.pages.dev').replace(/\/+$/, '');

export type MaterialCategory = {
  index: string;
  slug: string;
  title: string;
  titleRu?: string;
  titleKk?: string;
  titleEn?: string;
  text: string;
  textRu?: string;
  textKk?: string;
  textEn?: string;
  bullets: string[];
  bulletsRu?: string[];
  bulletsKk?: string[];
  bulletsEn?: string[];
  photo: string;
  imageUrl?: string;
  detailImageUrl?: string;
  price?: string;
  categorySlug?: string;
  unit?: string;
  availability?: string;
  city?: string;
  supplier?: string;
  purchasePrice?: string;
  purchaseCurrency?: string;
  salePrice?: string;
  saleCurrency?: string;
  marginPercent?: string;
  status?: string;
  languages?: string[];
  seoTitle?: string;
  seoDescription?: string;
};

export type Product = {
  categorySlug: string;
  category: string;
  categoryRu?: string;
  categoryKk?: string;
  categoryEn?: string;
  title: string;
  titleRu?: string;
  titleKk?: string;
  titleEn?: string;
  spec: string;
  specRu?: string;
  specKk?: string;
  specEn?: string;
  photo: string;
};

export type Kit = {
  title: string;
  titleRu?: string;
  titleKk?: string;
  titleEn?: string;
  text: string;
  textRu?: string;
  textKk?: string;
  textEn?: string;
  items: string[];
  itemsRu?: string[];
  itemsKk?: string[];
  itemsEn?: string[];
};

export type Project = {
  title: string;
  titleRu?: string;
  titleKk?: string;
  titleEn?: string;
  meta: string;
  metaRu?: string;
  metaKk?: string;
  metaEn?: string;
  photo: string;
  imageUrls?: string[];
};

export type AlmabuildContent = {
  materialCategories: MaterialCategory[];
  products: Product[];
  kits: Kit[];
  projects: Project[];
};

export type AlmabuildLead = {
  id: string;
  createdAt: string;
  name: string;
  phone: string;
  type: string;
  area: string;
  comment: string;
  items: string[];
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

export function listAlmabuildLeads(): Promise<AlmabuildLead[]> {
  return apiFetch<AlmabuildLead[]>('/api/admin/almabuild/leads');
}


export type AlmabuildAiKind = 'material' | 'product' | 'kit' | 'project';

export function aiEditAlmabuildItem<T>(kind: AlmabuildAiKind, instruction: string, value: T): Promise<T> {
  return apiFetch<T>('/api/admin/almabuild/ai/edit', {
    method: 'POST',
    body: JSON.stringify({ kind, instruction, value })
  });
}

export type MaterialsFromPhotoRequest = {
  image: File;
  count: number;
  instruction?: string;
  existingCount?: number;
  existing?: MaterialCategory[];
  detailImage?: File | null;
  price?: string;
  categorySlug?: string;
  categoryTitle?: string;
  unit?: string;
  availability?: string;
  city?: string;
  supplier?: string;
  purchasePrice?: string;
  purchaseCurrency?: string;
  salePrice?: string;
  saleCurrency?: string;
  marginPercent?: string;
  status?: string;
  languages?: string[];
  seoTitle?: string;
  seoDescription?: string;
};

export type MaterialsFromPhotoResponse = {
  materials: MaterialCategory[];
};

export function generateAlmabuildMaterialsFromPhoto(request: MaterialsFromPhotoRequest): Promise<MaterialsFromPhotoResponse> {
  const form = new FormData();
  form.set('image', request.image);
  form.set('count', String(Math.min(12, Math.max(1, request.count))));
  form.set('instruction', request.instruction || '');
  form.set('existingCount', String(request.existingCount ?? 0));
  form.set('existing', JSON.stringify(request.existing || []));
  if (request.detailImage) form.set('detailImage', request.detailImage);
  form.set('price', request.price || '');
  form.set('categorySlug', request.categorySlug || '');
  form.set('categoryTitle', request.categoryTitle || '');
  form.set('unit', request.unit || '');
  form.set('availability', request.availability || '');
  form.set('city', request.city || 'Алматы');
  form.set('supplier', request.supplier || '');
  form.set('purchasePrice', request.purchasePrice || '');
  form.set('purchaseCurrency', request.purchaseCurrency || 'KZT');
  form.set('salePrice', request.salePrice || '');
  form.set('saleCurrency', request.saleCurrency || 'KZT');
  form.set('marginPercent', request.marginPercent || '');
  form.set('status', request.status || 'draft');
  form.set('languages', (request.languages || ['RU']).join(','));
  form.set('seoTitle', request.seoTitle || '');
  form.set('seoDescription', request.seoDescription || '');
  return apiFetch<MaterialsFromPhotoResponse>('/api/admin/almabuild/ai/materials-from-photo', {
    method: 'POST',
    body: form
  });
}
