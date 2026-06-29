import { AppIcon, type AppIconName } from './AppIcon';
import { siteConfigs } from '../lib/siteConfig';
import { apiStatusLabels, revalidateStatusLabels } from '../lib/labels';
import type { AlmabuildSection, IconsSection, SiteKey } from '../types/admin';

export type AppPage =
  | 'dashboard'
  | 'affiliate'
  | 'content'
  | 'construction'
  | 'culinary'
  | 'icons'
  | 'leads'
  | 'suppliers'
  | 'analytics'
  | 'users'
  | 'settings'
  | 'about';

type NavItem = { page: AppPage; label: string; icon: AppIconName; shortcut: string };

interface SidebarProps {
  activeSite: SiteKey;
  activePage: AppPage;
  activeConstructionSection: AlmabuildSection;
  activeIconsSection: IconsSection;
  collapsed: boolean;
  onToggleCollapse: () => void;
  onSiteChange: (site: SiteKey) => void;
  onNavigate: (page: AppPage) => void;
  onConstructionSectionChange: (section: AlmabuildSection) => void;
  onIconsSectionChange: (section: IconsSection) => void;
}

const constructionSiteSections: Array<{ key: AlmabuildSection; label: string; note: string }> = [
  { key: 'services', label: 'Услуги', note: 'Верхний блок' },
  { key: 'materials', label: 'Материалы', note: 'Категории' },
  { key: 'projects', label: 'Проекты', note: 'Кейсы' },
  { key: 'estimate', label: 'Смета', note: 'Комплекты' },
  { key: 'contact', label: 'Контакты', note: 'Заявки' },
  { key: 'catalog', label: 'Каталог', note: 'Товары' }
];

const iconsSiteSections: Array<{ key: IconsSection; label: string; note: string; primary?: boolean }> = [
  { key: 'icons', label: 'Иконы', note: 'главный редактор', primary: true },
  { key: 'calendar', label: 'Календарь', note: 'заполнение по датам' },
  { key: 'prayers', label: 'Молитвы', note: 'просмотр из икон' },
  { key: 'saints', label: 'Святые', note: 'просмотр из икон' },
  { key: 'gospel', label: 'Евангелие', note: 'просмотр из икон' },
  { key: 'qr', label: 'QR-страницы', note: 'авто из икон' },
  { key: 'seo', label: 'SEO-страницы', note: 'просмотр' },
  { key: 'churches', label: 'Храмы', note: 'просмотр из икон' }
];

const SITE_NAV_ITEMS: Record<SiteKey, NavItem[]> = {
  culinary: [
    { page: 'dashboard', label: 'Панель CU', icon: 'dashboard', shortcut: '1' },
    { page: 'affiliate', label: 'Affiliate товары', icon: 'shop', shortcut: '2' },
    { page: 'content', label: 'Контент сайта', icon: 'cms', shortcut: '3' },
    { page: 'culinary', label: 'Ингредиенты', icon: 'catalog', shortcut: '4' },
    { page: 'leads', label: 'Заявки CU', icon: 'leads', shortcut: '5' },
    { page: 'analytics', label: 'Аналитика CU', icon: 'analytics', shortcut: 'A' },
    { page: 'users', label: 'Пользователи', icon: 'users', shortcut: 'U' },
    { page: 'settings', label: 'Настройки CU', icon: 'settings', shortcut: 'S' },
    { page: 'about', label: 'О системе', icon: 'shield', shortcut: '?' }
  ],
  construction: [
    { page: 'dashboard', label: 'Панель CO', icon: 'dashboard', shortcut: '1' },
    { page: 'content', label: 'Контент сайта', icon: 'building', shortcut: '2' },
    { page: 'affiliate', label: 'Поставки / affiliate', icon: 'shop', shortcut: '3' },
    { page: 'leads', label: 'Заявки CO', icon: 'leads', shortcut: '4' },
    { page: 'suppliers', label: 'Поставщики', icon: 'suppliers', shortcut: '5' },
    { page: 'analytics', label: 'Аналитика CO', icon: 'analytics', shortcut: 'A' },
    { page: 'users', label: 'Пользователи', icon: 'users', shortcut: 'U' },
    { page: 'settings', label: 'Настройки CO', icon: 'settings', shortcut: 'S' },
    { page: 'about', label: 'О системе', icon: 'shield', shortcut: '?' }
  ],
  icons: [
    { page: 'dashboard', label: 'Обзор IK', icon: 'dashboard', shortcut: '1' },
    { page: 'content', label: 'Контент сайта', icon: 'cms', shortcut: '2' },
    { page: 'analytics', label: 'Аналитика IK', icon: 'analytics', shortcut: 'A' },
    { page: 'users', label: 'Пользователи', icon: 'users', shortcut: 'U' },
    { page: 'settings', label: 'Настройки IK', icon: 'settings', shortcut: 'S' },
    { page: 'about', label: 'О системе', icon: 'shield', shortcut: '?' }
  ]
};

const defaultSitePages: Record<SiteKey, AppPage> = {
  culinary: 'content',
  construction: 'content',
  icons: 'content'
};

export function getSiteNavItems(site: SiteKey): NavItem[] {
  if (site === 'construction') return SITE_NAV_ITEMS.construction;
  if (site === 'icons') return SITE_NAV_ITEMS.icons;
  return SITE_NAV_ITEMS.culinary;
}

export function defaultPageForSite(site: SiteKey): AppPage {
  if (site === 'construction') return defaultSitePages.construction;
  if (site === 'icons') return defaultSitePages.icons;
  return defaultSitePages.culinary;
}

export function pageAllowedForSite(site: SiteKey, page: AppPage) {
  return getSiteNavItems(site).some((item) => item.page === page);
}

export function normalizeSitePage(site: SiteKey, page: AppPage): AppPage {
  if (pageAllowedForSite(site, page)) return page;
  return defaultPageForSite(site);
}

export function Sidebar({
  activeSite,
  activePage,
  activeConstructionSection,
  activeIconsSection,
  collapsed,
  onToggleCollapse,
  onSiteChange,
  onNavigate,
  onConstructionSectionChange,
  onIconsSectionChange
}: SidebarProps) {
  const visibleNavItems = getSiteNavItems(activeSite);
  const activeConfig = siteConfigs.find((site) => site.key === activeSite) ?? siteConfigs[0];
  const sectionLabel = activeSite === 'construction' ? 'Разделы строительного сайта' : activeSite === 'icons' ? 'Разделы сайта икон' : 'Разделы кулинарного сайта';

  return (
    <aside className={'sidebar' + (collapsed ? ' collapsed' : '')}>
      <div className="sidebar-brand">
        <div className="brand-mark">AF</div>
        <div>
          <p className="sidebar-title">ПАРТНЕРСКАЯ ОС</p>
          <p className="sidebar-subtitle">Мультисайтовая админ-панель</p>
        </div>
      </div>

      <div className="site-switcher" aria-label="Сайты">
        <p className="sidebar-section-label">Активный сайт</p>
        {siteConfigs.map((site) => {
          const active = site.key === activeSite;
          return (
            <button key={site.key} className={'site-switcher-option' + (active ? ' active' : '')} type="button" title={site.name} aria-current={active ? 'true' : undefined} onClick={() => onSiteChange(site.key)}>
              <span className="site-switcher-mark">{site.key === 'culinary' ? 'CU' : site.key === 'construction' ? 'CO' : 'IK'}</span>
              <span><strong>{site.name}</strong><small>{site.domain}</small></span>
              <i className={'site-dot ' + (site.apiStatus === 'online' ? 'prod' : 'warning')} />
            </button>
          );
        })}
      </div>

      <nav className="sidebar-nav" aria-label="Основная навигация">
        <p className="sidebar-section-label">{sectionLabel}</p>
        {visibleNavItems.map((item) => (
          <div key={item.page} className="sidebar-nav-group">
            <button className={'sidebar-link' + (activePage === item.page ? ' active' : '')} type="button" title={item.label} onClick={() => onNavigate(item.page)}>
              <AppIcon name={item.icon} />
              <span>{item.label}</span>
              <kbd>{item.shortcut}</kbd>
            </button>
            {activeSite === 'construction' && item.page === 'content' && !collapsed ? (
              <div className="sidebar-subnav" aria-label="Разделы Kazaxbud">
                {constructionSiteSections.map((section) => (
                  <button
                    key={section.key}
                    className={activePage === 'content' && activeConstructionSection === section.key ? 'active' : ''}
                    type="button"
                    onClick={() => onConstructionSectionChange(section.key)}
                  >
                    <span>{section.label}</span>
                    <small>{section.note}</small>
                  </button>
                ))}
              </div>
            ) : null}
            {activeSite === 'icons' && item.page === 'content' && !collapsed ? (
              <div className="sidebar-subnav" aria-label="Разделы сайта икон">
                {iconsSiteSections.map((section) => (
                  <button
                    key={section.key}
                    className={(activePage === 'content' && activeIconsSection === section.key ? 'active' : '') + (section.primary ? ' primary-source' : '')}
                    type="button"
                    onClick={() => onIconsSectionChange(section.key)}
                  >
                    <span>{section.label}</span>
                    <small>{section.note}</small>
                  </button>
                ))}
              </div>
            ) : null}
          </div>
        ))}
      </nav>

      <div className="sidebar-system">
        <div className="system-row"><span>API</span><strong><i className={'live-dot ' + (activeConfig.apiStatus === 'online' ? '' : 'warning')} /> {apiStatusLabels[activeConfig.apiStatus]}</strong></div>
        <div className="system-row"><span>Ревалидация</span><strong>{revalidateStatusLabels[activeConfig.revalidateStatus]}</strong></div>
        <div className="system-row"><span>Валюта</span><strong>{activeConfig.defaultCurrency}</strong></div>
        <div className="sidebar-user"><span>ДА</span><p><strong>Дима Админ</strong><small>Суперадминистратор</small></p></div>
      </div>

      <button className="sidebar-collapse" type="button" aria-pressed={collapsed} title={collapsed ? 'Развернуть меню' : 'Свернуть меню'} onClick={onToggleCollapse}>
        <AppIcon name="chevron-left" />
        <span>{collapsed ? 'Развернуть меню' : 'Свернуть меню'}</span>
      </button>
    </aside>
  );
}
