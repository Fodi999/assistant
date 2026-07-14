import { apiFetch } from './client';

export type ChurchCalendarType = 'old_style' | 'new_style' | 'both';
export type ChurchDayType = 'saint' | 'feast' | 'fasting' | 'memorial' | 'gospel' | 'quiet';
export type ChurchLanguage = 'uk' | 'ru' | 'en';
export type ChurchContentStatus = 'draft' | 'published' | 'archived';
export type ChurchPrayerType = 'prayer' | 'akathist' | 'troparion' | 'kontakion' | 'velichanie' | 'modern';

export interface ChurchCalendarDay {
  id: string;
  siteId: string;
  dateOldStyle?: string | null;
  dateNewStyle?: string | null;
  calendarType: ChurchCalendarType;
  title: string;
  dayType: ChurchDayType;
  description: string;
  rank: number;
  status: ChurchContentStatus;
  isGlobal: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface ChurchIcon {
  id: string;
  siteId: string;
  calendarDayId?: string | null;
  title: string;
  slug: string;
  imageUrl: string;
  saintName: string;
  feastName: string;
  description: string;
  language: ChurchLanguage;
  /** Backend-managed: language versions sharing a slug get the same group. */
  translationGroupId?: string;
  status: ChurchContentStatus;
  isGlobal: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface ChurchPrayer {
  id: string;
  siteId: string;
  iconId?: string | null;
  calendarDayId?: string | null;
  slug: string;
  title: string;
  text: string;
  audioUrl?: string | null;
  qrCodeUrl?: string | null;
  imageUrl?: string | null;
  source?: string | null;
  sourceUrl?: string | null;
  note?: string | null;
  language: ChurchLanguage;
  prayerType: ChurchPrayerType;
  /** Backend-managed: language versions sharing a slug get the same group. */
  translationGroupId?: string;
  status: ChurchContentStatus;
  isGlobal: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface ChurchArticle {
  id: string;
  siteId: string;
  iconId?: string | null;
  calendarDayId?: string | null;
  title: string;
  slug: string;
  content: string;
  language: ChurchLanguage;
  seoTitle: string;
  seoDescription: string;
  status: ChurchContentStatus;
  isGlobal: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface ChurchGospel {
  id: string;
  siteId: string;
  iconId?: string | null;
  calendarDayId?: string | null;
  slug: string;
  title: string;
  reference: string;
  text: string;
  explanation: string;
  language: ChurchLanguage;
  status: ChurchContentStatus;
  isGlobal: boolean;
  createdAt: string;
  updatedAt: string;
}

export type ChurchGospelPayload = Partial<Omit<ChurchGospel, 'id' | 'siteId' | 'createdAt' | 'updatedAt'>>;

export interface ChurchSaint {
  id: string;
  siteId: string;
  iconId?: string | null;
  calendarDayId?: string | null;
  slug: string;
  name: string;
  shortDescription: string;
  biography: string;
  feastDay: string;
  imageUrl: string;
  language: ChurchLanguage;
  /** Backend-managed: language versions sharing a slug get the same group. */
  translationGroupId?: string;
  status: ChurchContentStatus;
  isGlobal: boolean;
  createdAt: string;
  updatedAt: string;
}

export type ChurchSaintPayload = Partial<Omit<ChurchSaint, 'id' | 'siteId' | 'createdAt' | 'updatedAt'>>;

export interface ChurchInfoTranslation {
  title: string;
  description: string;
  schedule: string;
  dedication: string;
  shrines: string;
  priest: string;
}

export const emptyChurchInfoTranslation: ChurchInfoTranslation = {
  title: '',
  description: '',
  schedule: '',
  dedication: '',
  shrines: '',
  priest: ''
};

export interface ChurchInfo {
  id: string;
  siteId: string;
  address: string;
  mapsUrl: string;
  phoneOrSite: string;
  priestPhone: string;
  imageUrl: string;
  galleryImages: string[];
  translations: Partial<Record<ChurchLanguage, ChurchInfoTranslation>>;
  status: ChurchContentStatus;
  createdAt: string;
  updatedAt: string;
}

export type ChurchInfoPayload = Partial<Omit<ChurchInfo, 'id' | 'siteId' | 'createdAt' | 'updatedAt'>>;

export type ChurchCalendarDayPayload = Partial<Omit<ChurchCalendarDay, 'id' | 'siteId' | 'createdAt' | 'updatedAt'>>;
export type ChurchIconPayload = Partial<Omit<ChurchIcon, 'id' | 'siteId' | 'createdAt' | 'updatedAt'>>;
export type ChurchPrayerPayload = Partial<Omit<ChurchPrayer, 'id' | 'siteId' | 'createdAt' | 'updatedAt'>>;
export type ChurchArticlePayload = Partial<Omit<ChurchArticle, 'id' | 'siteId' | 'createdAt' | 'updatedAt'>>;

export interface ChurchImportPreview {
  calendarDays: number;
  icons: number;
  prayers: number;
  articles: number;
  gospel: number;
  errors: string[];
  warnings: string[];
}

type ChurchQuery = {
  site?: string;
  siteId?: string;
  year?: string | number;
  month?: string | number;
  calendarDayId?: string;
  iconId?: string;
  language?: ChurchLanguage;
};

const basePath = '/api/admin/church-content';

export const churchContentApi = {
  listCalendarDays: (query?: ChurchQuery) => apiFetch<ChurchCalendarDay[]>(`${basePath}/calendar-days${queryString(query)}`),
  getCalendarDay: (id: string, query?: ChurchQuery) => apiFetch<ChurchCalendarDay>(`${basePath}/calendar-days/${id}${queryString(query)}`),
  createCalendarDay: (payload: ChurchCalendarDayPayload, query?: ChurchQuery) => apiFetch<ChurchCalendarDay>(`${basePath}/calendar-days${queryString(query)}`, request('POST', payload)),
  updateCalendarDay: (id: string, payload: ChurchCalendarDayPayload, query?: ChurchQuery) => apiFetch<ChurchCalendarDay>(`${basePath}/calendar-days/${id}${queryString(query)}`, request('PUT', payload)),
  deleteCalendarDay: (id: string, query?: ChurchQuery) => apiFetch<void>(`${basePath}/calendar-days/${id}${queryString(query)}`, request('DELETE')),

  listIcons: (query?: ChurchQuery) => apiFetch<ChurchIcon[]>(`${basePath}/icons${queryString(query)}`),
  getIcon: (id: string, query?: ChurchQuery) => apiFetch<ChurchIcon>(`${basePath}/icons/${id}${queryString(query)}`),
  createIcon: (payload: ChurchIconPayload, query?: ChurchQuery) => apiFetch<ChurchIcon>(`${basePath}/icons${queryString(query)}`, request('POST', payload)),
  updateIcon: (id: string, payload: ChurchIconPayload, query?: ChurchQuery) => apiFetch<ChurchIcon>(`${basePath}/icons/${id}${queryString(query)}`, request('PUT', payload)),
  deleteIcon: (id: string, query?: ChurchQuery) => apiFetch<void>(`${basePath}/icons/${id}${queryString(query)}`, request('DELETE')),

  listPrayers: (query?: ChurchQuery) => apiFetch<ChurchPrayer[]>(`${basePath}/prayers${queryString(query)}`),
  getPrayer: (id: string, query?: ChurchQuery) => apiFetch<ChurchPrayer>(`${basePath}/prayers/${id}${queryString(query)}`),
  createPrayer: (payload: ChurchPrayerPayload, query?: ChurchQuery) => apiFetch<ChurchPrayer>(`${basePath}/prayers${queryString(query)}`, request('POST', payload)),
  updatePrayer: (id: string, payload: ChurchPrayerPayload, query?: ChurchQuery) => apiFetch<ChurchPrayer>(`${basePath}/prayers/${id}${queryString(query)}`, request('PUT', payload)),
  deletePrayer: (id: string, query?: ChurchQuery) => apiFetch<void>(`${basePath}/prayers/${id}${queryString(query)}`, request('DELETE')),

  listArticles: (query?: ChurchQuery) => apiFetch<ChurchArticle[]>(`${basePath}/articles${queryString(query)}`),
  getArticle: (id: string, query?: ChurchQuery) => apiFetch<ChurchArticle>(`${basePath}/articles/${id}${queryString(query)}`),
  createArticle: (payload: ChurchArticlePayload, query?: ChurchQuery) => apiFetch<ChurchArticle>(`${basePath}/articles${queryString(query)}`, request('POST', payload)),
  updateArticle: (id: string, payload: ChurchArticlePayload, query?: ChurchQuery) => apiFetch<ChurchArticle>(`${basePath}/articles/${id}${queryString(query)}`, request('PUT', payload)),
  deleteArticle: (id: string, query?: ChurchQuery) => apiFetch<void>(`${basePath}/articles/${id}${queryString(query)}`, request('DELETE')),

  listSaints: (query?: ChurchQuery) => apiFetch<ChurchSaint[]>(`${basePath}/saints${queryString(query)}`),
  getSaint: (id: string, query?: ChurchQuery) => apiFetch<ChurchSaint>(`${basePath}/saints/${id}${queryString(query)}`),
  createSaint: (payload: ChurchSaintPayload, query?: ChurchQuery) => apiFetch<ChurchSaint>(`${basePath}/saints${queryString(query)}`, request('POST', payload)),
  updateSaint: (id: string, payload: ChurchSaintPayload, query?: ChurchQuery) => apiFetch<ChurchSaint>(`${basePath}/saints/${id}${queryString(query)}`, request('PUT', payload)),
  deleteSaint: (id: string, query?: ChurchQuery) => apiFetch<void>(`${basePath}/saints/${id}${queryString(query)}`, request('DELETE')),

  listGospel: (query?: ChurchQuery) => apiFetch<ChurchGospel[]>(`${basePath}/gospel${queryString(query)}`),
  getGospel: (id: string, query?: ChurchQuery) => apiFetch<ChurchGospel>(`${basePath}/gospel/${id}${queryString(query)}`),
  createGospel: (payload: ChurchGospelPayload, query?: ChurchQuery) => apiFetch<ChurchGospel>(`${basePath}/gospel${queryString(query)}`, request('POST', payload)),
  updateGospel: (id: string, payload: ChurchGospelPayload, query?: ChurchQuery) => apiFetch<ChurchGospel>(`${basePath}/gospel/${id}${queryString(query)}`, request('PUT', payload)),
  deleteGospel: (id: string, query?: ChurchQuery) => apiFetch<void>(`${basePath}/gospel/${id}${queryString(query)}`, request('DELETE')),

  previewImport: (query?: ChurchQuery) => apiFetch<ChurchImportPreview>(`${basePath}/import/preview${queryString(query)}`),
  applyImport: (query?: ChurchQuery) => apiFetch<ChurchImportPreview>(`${basePath}/import/apply${queryString(query)}`, request('POST')),

  getInfo: (query?: ChurchQuery) => apiFetch<ChurchInfo>(`${basePath}/info${queryString(query)}`),
  updateInfo: (payload: ChurchInfoPayload, query?: ChurchQuery) => apiFetch<ChurchInfo>(`${basePath}/info${queryString(query)}`, request('PUT', payload))
};

function queryString(query?: ChurchQuery) {
  const params = new URLSearchParams();
  if (query?.site) params.set('site', query.site);
  if (query?.siteId) params.set('site_id', query.siteId);
  if (query?.year) params.set('year', String(query.year));
  if (query?.month) params.set('month', String(query.month));
  if (query?.calendarDayId) params.set('calendar_day_id', query.calendarDayId);
  if (query?.iconId) params.set('icon_id', query.iconId);
  if (query?.language) params.set('language', query.language);
  const value = params.toString();
  return value ? `?${value}` : '';
}

function request(method: 'POST' | 'PUT' | 'DELETE', body?: unknown): RequestInit {
  return {
    method,
    body: body === undefined ? undefined : JSON.stringify(body)
  };
}
