import type { CurrencyCode, LanguageCode, SiteConfig, SiteKey } from '../types/admin';

export const languages: LanguageCode[] = ['ru', 'pl', 'en', 'kk'];
export const currencies: CurrencyCode[] = ['KZT', 'PLN', 'EUR', 'USD'];

export const siteConfigs: SiteConfig[] = [
  {
    key: 'culinary',
    name: 'Кулинарный сайт',
    domain: 'dima-fomin.pl',
    primaryLanguage: 'pl',
    languages: ['ru', 'pl', 'en', 'kk'],
    status: 'active',
    apiStatus: 'online',
    revalidateStatus: 'ready',
    defaultCurrency: 'PLN',
    region: 'EU / PL'
  },
  {
    key: 'construction',
    name: 'Строительный сайт',
    domain: 'kazaxbud.pages.dev',
    primaryLanguage: 'ru',
    languages: ['ru', 'kk', 'en', 'pl'],
    status: 'active',
    apiStatus: 'online',
    revalidateStatus: 'ready',
    defaultCurrency: 'KZT',
    region: 'Алматы'
  },
  {
    key: 'icons',
    name: 'Сайт икон',
    domain: 'svet-ikony.local',
    primaryLanguage: 'ru',
    languages: ['ru', 'en', 'pl', 'kk'],
    status: 'active',
    apiStatus: 'limited',
    revalidateStatus: 'ready',
    defaultCurrency: 'EUR',
    region: 'QR / православные иконы'
  }
];

export function siteLabel(site: SiteKey) {
  return siteConfigs.find((item) => item.key === site)?.name ?? site;
}
