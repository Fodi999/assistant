import type { AdminResourceRow, LanguageCode, ResourceStatus, SiteId } from '../../../types/admin';

export type LocalizedText = Partial<Record<LanguageCode | string, string>>;

export function pickText(...values: unknown[]): string {
  for (const value of values) {
    if (typeof value === 'string' && value.trim()) return value.trim();
    if (value && typeof value === 'object') {
      const localized = value as LocalizedText;
      const text = localized.ru || localized.en || localized.pl || localized.kk || Object.values(localized).find((item) => typeof item === 'string' && item.trim());
      if (text) return text;
    }
  }

  return 'Untitled item';
}

export function mapStatus(value: unknown, fallback: ResourceStatus = 'draft'): ResourceStatus {
  if (value === true) return 'published';
  if (value === false || value == null) return fallback;

  const status = String(value).toLowerCase();
  if (status === 'published' || status === 'active' || status === 'draft' || status === 'archived' || status === 'new' || status === 'warning' || status === 'neutral') {
    return status;
  }

  if (status === 'won' || status === 'contacted' || status === 'quoted') return 'active';
  if (status === 'lost') return 'archived';

  return fallback;
}

export function mapLanguage(value: unknown, fallback: LanguageCode = 'en'): LanguageCode {
  const language = typeof value === 'string' ? value.toLowerCase() : fallback;
  if (language === 'ru' || language === 'pl' || language === 'en' || language === 'kk') return language;
  return fallback;
}

export function updatedLabel(value: unknown): string {
  if (typeof value !== 'string' || !value.trim()) return 'backend';
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
}

export function baseRow(row: Partial<AdminResourceRow> & { id: string }, siteId: SiteId): AdminResourceRow {
  return {
    siteId,
    title: row.title || 'Untitled item',
    type: row.type || 'Item',
    status: row.status || 'draft',
    owner: row.owner || 'Backend',
    updated: row.updated || updatedLabel(row.updatedAt),
    metric: row.metric || '-',
    ...row
  };
}
