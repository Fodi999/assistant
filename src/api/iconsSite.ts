import { apiFetch } from './client';

export type IconsLang = 'uk' | 'ru' | 'en';
export type IconsStatus = 'draft' | 'published';

export interface IconTranslation {
  title?: string;
  shortDescription?: string;
  fullDescription?: string;
  category?: string;
  saintName?: string;
  prayerText?: string;
  gospelText?: string;
  lifeText?: string;
  historyText?: string;
  seoTitle?: string;
  seoDescription?: string;
  seoKeywords?: string;
}

export interface IconPage {
  id: string;
  slug: string;
  title: string;
  shortDescription: string;
  fullDescription: string;
  imageUrl: string;
  imageUrls?: string[];
  qrCodeUrl: string;
  category: string;
  saintName: string;
  prayerText: string;
  gospelText: string;
  lifeText: string;
  historyText: string;
  status: IconsStatus;
  seoTitle?: string;
  seoDescription?: string;
  seoKeywords?: string;
  translations?: Partial<Record<IconsLang, IconTranslation>>;
  calendarDate?: string;
  createdAt: string;
  updatedAt: string;
}

export interface PrayerPage {
  id: string;
  slug: string;
  title: string;
  text: string;
  category: string;
  relatedIcon?: string;
  status: IconsStatus;
  seoTitle?: string;
  seoDescription?: string;
}

export interface GospelReading {
  id: string;
  date: string;
  title: string;
  reference: string;
  text: string;
  explanation: string;
  status: IconsStatus;
  seoTitle?: string;
  seoDescription?: string;
}

export interface SaintPage {
  id: string;
  slug: string;
  name: string;
  shortDescription: string;
  biography: string;
  feastDay: string;
  imageUrl: string;
  relatedIcons: string[];
  prayers: string[];
  status: IconsStatus;
  seoTitle?: string;
  seoDescription?: string;
}

export interface SeoPageTranslation {
  title?: string;
  h1?: string;
  content?: string;
  targetKeyword?: string;
  blocks?: string[];
  seoTitle?: string;
  seoDescription?: string;
}

export interface SeoPage {
  id: string;
  slug: string;
  title: string;
  h1: string;
  content: string;
  pageType: string;
  targetKeyword: string;
  language: string;
  blocks: string[];
  status: IconsStatus;
  createdAt: string;
  updatedAt: string;
  imageUrl?: string;
  city?: string;
  seoTitle?: string;
  seoDescription?: string;
  translations?: Partial<Record<IconsLang, SeoPageTranslation>>;
}

export interface QrPage {
  id: string;
  qrId: string;
  iconId: string;
  slug: string;
  title: string;
  active: boolean;
  scanCount: number;
  createdAt: string;
  updatedAt: string;
  ownerName?: string;
  location?: string;
  customPrayer?: string;
}

export interface ChurchPage {
  id: string;
  slug: string;
  title: string;
  city: string;
  address: string;
  description: string;
  schedule: string;
  relatedIcons: string[];
  status: IconsStatus;
  donationUrl?: string;
  seoTitle?: string;
  seoDescription?: string;
}

export type CalendarDayKind = 'feast' | 'fast' | 'gospel' | 'prayer' | 'quiet';

export interface CalendarHero {
  year: string;
  title: string;
  monthTitle: string;
  prevLabel: string;
  prevHref: string;
  nextLabel: string;
  nextHref: string;
  featureTitle: string;
  featureNote: string;
  featureDate: string;
  featureHref: string;
  iconDayTitle: string;
  iconDayIconSlug: string;
  iconDayDate: string;
  iconDayPrayerSlug: string;
  infoPrimary: string;
  infoSecondary: string;
  todayDate: string;
  todayGospel: string;
  todayPrayerTitle: string;
  todayHref: string;
}

export interface CalendarDay {
  id: string;
  month?: string;
  day: string;
  gregorianDate?: string;
  julianDay?: string;
  julianDate?: string;
  label: string;
  note: string;
  kind: CalendarDayKind;
  imageUrl: string;
  iconSlug: string;
  prayerSlug: string;
  gospelSlug: string;
  detailHref: string;
  current: boolean;
  feast: boolean;
  textOnly: boolean;
  description: string;
}

export interface CalendarServiceCard {
  id: string;
  index: string;
  title: string;
  description: string;
  href: string;
}

export interface CalendarContent {
  hero: CalendarHero;
  days: CalendarDay[];
  services: CalendarServiceCard[];
}

export interface IconsSiteContent {
  icons: IconPage[];
  prayers: PrayerPage[];
  gospel: GospelReading[];
  saints: SaintPage[];
  pages: SeoPage[];
  qrPages: QrPage[];
  churches: ChurchPage[];
  calendar: CalendarContent;
  dashboard: {
    publishedPages: number;
    icons: number;
    prayers: number;
    qrPages: number;
    qrScans: number;
    churches: number;
    latestPages: Array<Record<string, unknown>>;
    seo: Array<Record<string, unknown>>;
  };
}

export function getIconsSiteContent(params?: { year?: string | number; month?: string | number }) {
  const query = new URLSearchParams();
  if (params?.year) query.set('year', String(params.year));
  if (params?.month) query.set('month', String(Number(params.month)));
  const suffix = query.toString() ? `?${query.toString()}` : '';
  return apiFetch<IconsSiteContent>(`/api/admin/icons-site/content${suffix}`);
}

export function saveIconsSiteContent(content: IconsSiteContent) {
  return apiFetch<IconsSiteContent>('/api/admin/icons-site/content', {
    method: 'PUT',
    body: JSON.stringify(content)
  });
}
