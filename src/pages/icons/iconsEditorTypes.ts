import type {
  CalendarDay,
  ChurchPage,
  GospelReading,
  IconPage,
  PrayerPage,
  QrPage,
  SaintPage,
  SeoPage
} from '../../api/iconsSite';

export type IconsItem = CalendarDay | IconPage | PrayerPage | SaintPage | GospelReading | QrPage | SeoPage | ChurchPage;
export type IconPhotoAspect = 'source' | 'square' | 'landscape' | 'portrait' | 'wide';
export type EditorTabKey = 'main' | 'texts' | 'seo' | 'photo-ai' | 'calendar' | 'qr' | 'publish';
export type EditorFieldsView = 'all' | 'main' | 'texts' | 'seo';
export type IconTextScopeKey = 'icon' | 'saints' | 'church';
export type IconTextTabKey = 'description' | 'prayer' | 'gospel' | 'life' | 'history';
export type IconTextPatch = Record<string, string | number | boolean | string[] | undefined | IconPage['translations']>;
