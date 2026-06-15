import type { ContentType, LeadStatus, PublishStatus, SiteKey, SupplierType } from '../types/admin';

export const siteNames: Record<SiteKey, string> = {
  culinary: 'Кулинарный',
  construction: 'Строительный'
};

export const publishStatusLabels: Record<PublishStatus, string> = {
  draft: 'черновик',
  active: 'активно',
  published: 'опубликовано',
  archived: 'архив'
};

export const leadStatusLabels: Record<LeadStatus, string> = {
  new: 'новая',
  contacted: 'связались',
  quoted: 'смета',
  won: 'выиграно',
  lost: 'потеряно'
};

export const priorityLabels: Record<'high' | 'medium' | 'low', string> = {
  high: 'высокий',
  medium: 'средний',
  low: 'низкий'
};

export const seoStatusLabels: Record<'good' | 'needs_work' | 'critical', string> = {
  good: 'хорошо',
  needs_work: 'нужна работа',
  critical: 'критично'
};

export const contentTypeLabels: Record<ContentType, string> = {
  article: 'статья',
  review: 'обзор',
  comparison: 'сравнение',
  roundup: 'подборка',
  recipe: 'рецепт'
};

export const supplierTypeLabels: Record<SupplierType, string> = {
  marketplace: 'маркетплейс',
  local_supplier: 'локальный поставщик',
  manufacturer: 'производитель',
  affiliate_merchant: 'партнерский продавец'
};

export const apiStatusLabels: Record<'online' | 'limited' | 'offline', string> = {
  online: 'онлайн',
  limited: 'ограничено',
  offline: 'офлайн'
};

export const revalidateStatusLabels: Record<'ready' | 'queued' | 'failed', string> = {
  ready: 'готово',
  queued: 'в очереди',
  failed: 'ошибка'
};
