import type { SiteId } from '../../types/admin';
import type { LocalizedAdminTextDto } from '../../types/adminApi';

export type SitePageKey = 'home' | 'about';
export type SiteSectionKey = 'services' | 'products';

export type SitePageDraft = {
  key: SitePageKey;
  title: LocalizedAdminTextDto;
  content: LocalizedAdminTextDto;
  slug: string;
  imageUrl: string;
  seoTitle: LocalizedAdminTextDto;
  seoDescription: LocalizedAdminTextDto;
  status: 'draft' | 'published';
  updatedAt: string;
};

export type SiteSectionDraft = {
  key: SiteSectionKey;
  title: LocalizedAdminTextDto;
  items: LocalizedAdminTextDto[];
};

export type SiteFooterDraft = {
  contactTitle: LocalizedAdminTextDto;
  contactText: LocalizedAdminTextDto;
  email: string;
  phone: string;
  address: string;
};

export type SitePublishingDraft = {
  pages: Record<SitePageKey, SitePageDraft>;
  sections: Record<SiteSectionKey, SiteSectionDraft>;
  footer: SiteFooterDraft;
};

const storageKey = (siteId: SiteId) => `admin_site_publishing_${siteId}`;
const now = () => new Date().toISOString();

const seedBySite: Record<SiteId, SitePublishingDraft> = {
  church: createSeed('Православные иконы', 'О мастерской и коллекции', 'Каталог икон'),
  construction: createSeed('Ремонт и строительство', 'О команде AlmaBuild', 'Услуги'),
  kitchen: createSeed('Culinary content hub', 'О проекте', 'Продукты')
};

function text(uk: string, ru = uk, en = uk): LocalizedAdminTextDto {
  return { uk, ru, en };
}

function createPage(key: SitePageKey, titleValue: string, slug: string): SitePageDraft {
  return {
    key,
    title: text(titleValue),
    content: text('Короткий редакционный блок для страницы.'),
    slug,
    imageUrl: '',
    seoTitle: text(titleValue),
    seoDescription: text('SEO description для страницы.'),
    status: 'draft',
    updatedAt: now()
  };
}

function createSeed(homeTitle: string, aboutTitle: string, sectionTitle: string): SitePublishingDraft {
  return {
    pages: {
      home: createPage('home', homeTitle, 'home'),
      about: createPage('about', aboutTitle, 'about')
    },
    sections: {
      services: {
        key: 'services',
        title: text(sectionTitle),
        items: [text('Основной раздел'), text('Консультация'), text('Поддержка')]
      },
      products: {
        key: 'products',
        title: text('Products'),
        items: [text('Featured item'), text('Popular item'), text('New item')]
      }
    },
    footer: {
      contactTitle: text('Контакты'),
      contactText: text('Свяжитесь с нами для консультации.'),
      email: 'admin@fodi.app',
      phone: '',
      address: ''
    }
  };
}

function read(siteId: SiteId): SitePublishingDraft {
  const raw = localStorage.getItem(storageKey(siteId));
  if (!raw) return seedBySite[siteId];

  try {
    return { ...seedBySite[siteId], ...JSON.parse(raw) } as SitePublishingDraft;
  } catch {
    return seedBySite[siteId];
  }
}

function write(siteId: SiteId, draft: SitePublishingDraft) {
  localStorage.setItem(storageKey(siteId), JSON.stringify(draft));
}

export async function getSitePublishingDraft(siteId: SiteId): Promise<SitePublishingDraft> {
  return read(siteId);
}

export async function saveSitePage(siteId: SiteId, page: SitePageDraft): Promise<SitePublishingDraft> {
  const draft = read(siteId);
  const next = {
    ...draft,
    pages: {
      ...draft.pages,
      [page.key]: { ...page, updatedAt: now() }
    }
  };
  write(siteId, next);
  return next;
}

export async function saveSiteSections(siteId: SiteId, sections: SitePublishingDraft['sections']): Promise<SitePublishingDraft> {
  const draft = read(siteId);
  const next = { ...draft, sections };
  write(siteId, next);
  return next;
}

export async function saveSiteFooter(siteId: SiteId, footer: SiteFooterDraft): Promise<SitePublishingDraft> {
  const draft = read(siteId);
  const next = { ...draft, footer };
  write(siteId, next);
  return next;
}
