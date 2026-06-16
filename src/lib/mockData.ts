import type {
  AffiliateOffer,
  AffiliateProduct,
  ConstructionBundle,
  ConstructionCalculatorPreset,
  ConstructionMaterial,
  ContentArticle,
  CurrencyCode,
  Lead,
  LanguageCode,
  SiteConfig,
  SiteDashboardMetrics,
  SiteKey,
  Supplier
} from '../types/admin';

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
  }
];

export const affiliateOffers: AffiliateOffer[] = [
  { id: 'offer-1', productId: 'aff-knife', network: 'amazon', merchant: 'Amazon EU', affiliateUrl: 'https://amazon.example/chef-knife', price: 79, currency: 'EUR', commissionPercent: 4.5, cookieDays: 24, isActive: true },
  { id: 'offer-2', productId: 'aff-knife', network: 'allegro', merchant: 'Allegro Smart', affiliateUrl: 'https://allegro.example/noz', price: 349, currency: 'PLN', commissionPercent: 5, cookieDays: 7, isActive: true },
  { id: 'offer-3', productId: 'aff-tiles', network: 'custom', merchant: 'Almaty Kerama', affiliateUrl: 'https://supplier.example/tiles', price: 6900, currency: 'KZT', commissionPercent: 8, cookieDays: 30, isActive: true },
  { id: 'offer-4', productId: 'aff-mixer', network: 'awin', merchant: 'Kitchen Pro', affiliateUrl: 'https://awin.example/mixer', price: 129, currency: 'EUR', commissionPercent: 6, cookieDays: 30, isActive: true }
];

export const affiliateProducts: AffiliateProduct[] = [
  {
    id: 'aff-knife',
    site: 'culinary',
    title: { ru: 'Поварской нож 20 см', pl: 'Noz szefa 20 cm', en: 'Chef knife 20 cm', kk: 'Аспаз пышагы 20 см' },
    slug: 'chef-knife-20cm',
    category: 'Профессиональные инструменты',
    network: 'allegro',
    merchant: 'Allegro Smart',
    affiliateUrl: 'https://allegro.example/noz',
    imageUrl: 'https://images.unsplash.com/photo-1593618998160-e34014e67546?auto=format&fit=crop&w=640&q=80',
    detailImageUrl: 'https://images.unsplash.com/photo-1593618998160-e34014e67546?auto=format&fit=crop&w=1200&q=80',
    price: 349,
    currency: 'PLN',
    commissionPercent: 5,
    cookieDays: 7,
    status: 'published',
    languages: ['ru', 'pl', 'en'],
    seoTitle: { ru: 'Лучший поварской нож для дома и ресторана', pl: 'Najlepszy noz szefa do domu i restauracji', en: 'Best chef knife for home and restaurant', kk: 'Уйге және мейрамханага аспаз пышагы' },
    seoDescription: { ru: 'Обзор ножа, плюсы, цена и affiliate-предложения.', pl: 'Recenzja noza, zalety, cena i oferty partnerskie.', en: 'Chef knife review with price and affiliate offers.', kk: 'Пышак шолуы, бага және сериктестик усыныстар.' },
    offers: affiliateOffers.filter((offer) => offer.productId === 'aff-knife')
  },
  {
    id: 'aff-tiles',
    site: 'construction',
    title: { ru: 'Керамогранит для ванной', pl: 'Gres do lazienki', en: 'Bathroom porcelain tiles', kk: 'Жуынатын бөлмеге керамогранит' },
    slug: 'bathroom-porcelain-tiles-almaty',
    category: 'Отделочные материалы',
    network: 'custom',
    merchant: 'Almaty Kerama',
    affiliateUrl: 'https://supplier.example/tiles',
    price: 6900,
    currency: 'KZT',
    commissionPercent: 8,
    cookieDays: 30,
    status: 'active',
    languages: ['ru', 'kk', 'en'],
    offers: affiliateOffers.filter((offer) => offer.productId === 'aff-tiles')
  },
  {
    id: 'aff-mixer',
    site: 'culinary',
    title: { ru: 'Планетарный миксер', pl: 'Mikser planetarny', en: 'Stand mixer', kk: 'Планетарлык миксер' },
    slug: 'stand-mixer-pro',
    category: 'Ресторанное оборудование',
    network: 'awin',
    merchant: 'Kitchen Pro',
    affiliateUrl: 'https://awin.example/mixer',
    price: 129,
    currency: 'EUR',
    commissionPercent: 6,
    cookieDays: 30,
    status: 'draft',
    languages: ['ru', 'pl', 'en'],
    offers: affiliateOffers.filter((offer) => offer.productId === 'aff-mixer')
  }
];

export const contentArticles: ContentArticle[] = [
  { id: 'content-1', site: 'culinary', type: 'review', title: { ru: 'Как выбрать поварской нож', pl: 'Jak wybrac noz szefa', en: 'How to choose a chef knife', kk: 'Аспаз пышагын калай тандау керек' }, slug: 'how-to-choose-chef-knife', excerpt: { ru: 'Критерии выбора, сталь, баланс и уход.', pl: 'Stal, balans i pielegnacja.', en: 'Steel, balance and maintenance.', kk: 'Болат, тепе-тендик және кутим.' }, status: 'published', languages: ['ru', 'pl', 'en'], affiliateProductIds: ['aff-knife'] },
  { id: 'content-2', site: 'construction', type: 'roundup', title: { ru: 'Топ материалов для ремонта ванной в Алматы', pl: 'Top materialow do lazienki', en: 'Top bathroom renovation materials', kk: 'Алматыда ванна жондеу материалдары' }, slug: 'bathroom-renovation-materials-almaty', excerpt: { ru: 'Подборка плитки, клея, гидроизоляции и работ.', pl: 'Plytki, klej i hydroizolacja.', en: 'Tiles, adhesive and waterproofing.', kk: 'Плитка, желим және гидроизоляция.' }, status: 'draft', languages: ['ru', 'kk'], affiliateProductIds: ['aff-tiles'] }
];

export const suppliers: Supplier[] = [
  { id: 'sup-1', name: 'Almaty Kerama', country: 'Kazakhstan', city: 'Алматы', categories: ['Плитка', 'Керамогранит'], contact: '+7 700 000 00 01', website: 'https://supplier.example', commissionTerms: '8% от подтвержденной заявки', type: 'local_supplier' },
  { id: 'sup-2', name: 'Kitchen Pro EU', country: 'Germany', city: 'Berlin', categories: ['Оборудование', 'Миксеры'], contact: 'partners@kitchen.example', website: 'https://kitchen.example', commissionTerms: 'Awin CPA 6%', type: 'affiliate_merchant' },
  { id: 'sup-3', name: 'Allegro', country: 'Poland', categories: ['Кухонные товары'], contact: 'affiliate@allegro.example', website: 'https://allegro.pl', commissionTerms: 'marketplace rate', type: 'marketplace' }
];

export const leads: Lead[] = [
  { id: 'lead-1', clientName: 'Алия Н.', contact: '+7 701 000 00 02', sourceSite: 'construction', category: 'Ремонт ванной', city: 'Алматы', message: 'Нужен расчет под ключ 42 м2.', status: 'new', potentialValue: 2800000, currency: 'KZT', createdAt: '2026-06-14T10:30:00Z' },
  { id: 'lead-2', clientName: 'Marek K.', contact: 'marek@example.com', sourceSite: 'culinary', category: 'Affiliate equipment', city: 'Warsaw', message: 'Zapytanie o mikser planetarny.', status: 'contacted', potentialValue: 129, currency: 'EUR', createdAt: '2026-06-13T16:10:00Z' },
  { id: 'lead-3', clientName: 'Ержан С.', contact: '+7 707 000 00 03', sourceSite: 'construction', category: 'Материалы', city: 'Алматы', message: 'Интересует плитка и доставка.', status: 'quoted', potentialValue: 640000, currency: 'KZT', createdAt: '2026-06-12T08:15:00Z' }
];

export const constructionMaterials: ConstructionMaterial[] = [
  { id: 'mat-1', title: { ru: 'Керамогранит 60x60', pl: 'Gres 60x60', en: 'Porcelain tile 60x60', kk: 'Керамогранит 60x60' }, slug: 'porcelain-60x60', category: 'Плитка', city: 'Алматы', supplierIds: ['sup-1'], unit: 'm2', materialPrice: 6900, workPrice: 4500, currency: 'KZT', marginPercent: 18, status: 'active' },
  { id: 'mat-2', title: { ru: 'Гидроизоляция санузла', pl: 'Hydroizolacja lazienki', en: 'Bathroom waterproofing', kk: 'Санузел гидроизоляциясы' }, slug: 'bathroom-waterproofing', category: 'Работы', city: 'Алматы', supplierIds: ['sup-1'], unit: 'm2', materialPrice: 1200, workPrice: 2800, currency: 'KZT', marginPercent: 22, status: 'published' }
];

export const calculatorPresets: ConstructionCalculatorPreset[] = [
  { id: 'calc-1', title: { ru: 'Ремонт ванной эконом', pl: 'Remont lazienki economy', en: 'Bathroom renovation economy', kk: 'Ванна жондеу эконом' }, city: 'Алматы', areaM2: 42, materialCost: 690000, workCost: 540000, marginPercent: 18, totalPrice: 1451400, currency: 'KZT' }
];

export const constructionBundles: ConstructionBundle[] = [
  { id: 'bundle-1', title: { ru: 'Ванная под ключ', pl: 'Lazienka pod klucz', en: 'Turnkey bathroom', kk: 'Дайын ванна' }, slug: 'turnkey-bathroom-almaty', city: 'Алматы', materials: ['Керамогранит', 'Клей', 'Гидроизоляция'], works: ['Демонтаж', 'Укладка', 'Затирка'], areaM2: 42, materialCost: 690000, workCost: 540000, totalPrice: 1451400, currency: 'KZT', supplierIds: ['sup-1'], leadFormEnabled: true, status: 'active' }
];

export const dashboardMetrics: SiteDashboardMetrics[] = siteConfigs.map((site) => {
  const siteLeads = leads.filter((lead) => lead.sourceSite === site.key);
  const products = affiliateProducts.filter((product) => product.site === site.key);
  return {
    site: site.key,
    visitors: site.key === 'construction' ? 18420 : 9360,
    affiliateClicks: site.key === 'construction' ? 1260 : 2140,
    leads: siteLeads.length,
    revenueEstimate: site.key === 'construction' ? 920000 : 1840,
    currency: site.defaultCurrency,
    publishedPages: contentArticles.filter((item) => item.site === site.key && item.status === 'published').length + products.filter((item) => item.status === 'published').length,
    aiDrafts: site.key === 'construction' ? 14 : 9,
    seoStatus: site.key === 'construction' ? 'needs_work' : 'good',
    topPages: [
      { title: site.key === 'construction' ? 'Ремонт ванной в Алматы' : 'Как выбрать поварской нож', path: site.key === 'construction' ? '/ru/remont-vannoy-almaty' : '/pl/how-to-choose-chef-knife', visitors: site.key === 'construction' ? 4200 : 3100, ctr: site.key === 'construction' ? 3.8 : 5.6 },
      { title: site.key === 'construction' ? 'Керамогранит 60x60' : 'Планетарные миксеры', path: site.key === 'construction' ? '/ru/materials/porcelain-60x60' : '/pl/stand-mixers', visitors: site.key === 'construction' ? 1800 : 1490, ctr: 4.1 }
    ],
    topProducts: products.map((product, index) => ({ productId: product.id, title: product.title.ru, clicks: 420 - index * 90, revenue: site.key === 'construction' ? 240000 - index * 60000 : 620 - index * 110 })),
    recentLeads: siteLeads,
    seoTasks: [
      { title: site.key === 'construction' ? 'Добавить KK meta для карточек материалов' : 'Усилить PL title у обзоров ножей', priority: 'high', status: 'draft' },
      { title: 'Проверить schema.org Product и Review', priority: 'medium', status: 'active' }
    ]
  };
});

export function siteLabel(site: SiteKey) {
  return siteConfigs.find((item) => item.key === site)?.name ?? site;
}
