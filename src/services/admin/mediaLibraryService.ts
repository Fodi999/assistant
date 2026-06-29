import type { SiteId } from '../../types/admin';

export type MediaLibraryItem = {
  id: string;
  siteId: SiteId;
  url: string;
  altText: string;
  source: 'upload' | 'url';
  createdAt: string;
};

const storageKey = (siteId: SiteId) => `admin_media_library_${siteId}`;

export function isValidImageUrl(url: string) {
  if (!url.trim()) return false;
  if (url.startsWith('data:image/')) return true;

  try {
    const parsed = new URL(url);
    return parsed.protocol === 'http:' || parsed.protocol === 'https:';
  } catch {
    return false;
  }
}

function read(siteId: SiteId): MediaLibraryItem[] {
  const raw = localStorage.getItem(storageKey(siteId));
  if (!raw) return [];

  try {
    return JSON.parse(raw) as MediaLibraryItem[];
  } catch {
    return [];
  }
}

function write(siteId: SiteId, items: MediaLibraryItem[]) {
  localStorage.setItem(storageKey(siteId), JSON.stringify(items));
}

function createItem(siteId: SiteId, url: string, altText: string, source: MediaLibraryItem['source']): MediaLibraryItem {
  return {
    id: `${source}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    siteId,
    url,
    altText,
    source,
    createdAt: new Date().toISOString()
  };
}

export async function listMediaItems(siteId: SiteId): Promise<MediaLibraryItem[]> {
  return read(siteId);
}

export async function addMediaUrl(siteId: SiteId, url: string, altText: string): Promise<MediaLibraryItem[]> {
  if (!isValidImageUrl(url)) throw new Error('Введите корректный URL изображения.');
  const items = [createItem(siteId, url.trim(), altText.trim(), 'url'), ...read(siteId)];
  write(siteId, items);
  return items;
}

export async function addUploadedMedia(siteId: SiteId, file: File, altText: string): Promise<MediaLibraryItem[]> {
  const dataUrl = await new Promise<string>((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result || ''));
    reader.onerror = () => reject(new Error('Не удалось прочитать файл.'));
    reader.readAsDataURL(file);
  });

  const items = [createItem(siteId, dataUrl, altText.trim() || file.name, 'upload'), ...read(siteId)];
  write(siteId, items);
  return items;
}

export async function updateMediaAltText(siteId: SiteId, id: string, altText: string): Promise<MediaLibraryItem[]> {
  const items = read(siteId).map((item) => item.id === id ? { ...item, altText } : item);
  write(siteId, items);
  return items;
}

export async function removeMediaItem(siteId: SiteId, id: string): Promise<MediaLibraryItem[]> {
  const items = read(siteId).filter((item) => item.id !== id);
  write(siteId, items);
  return items;
}
