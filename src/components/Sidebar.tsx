import { AppIcon, type AppIconName } from './AppIcon';
import { siteConfigs } from '../lib/mockData';
import { apiStatusLabels, revalidateStatusLabels } from '../lib/labels';
import type { SiteKey } from '../types/admin';

export type AppPage =
  | 'dashboard'
  | 'overview'
  | 'sites'
  | 'affiliate'
  | 'content'
  | 'ai-studio'
  | 'construction'
  | 'culinary'
  | 'leads'
  | 'catalog'
  | 'materials'
  | 'projects'
  | 'seo'
  | 'suppliers'
  | 'analytics'
  | 'ai'
  | 'usb'
  | 'deployments'
  | 'users'
  | 'settings'
  | 'about';

export type ManagedSite = SiteKey | 'almabuild' | 'dima';
type NavItem = { page: AppPage; label: string; icon: AppIconName; shortcut: string };

interface SidebarProps {
  activeSite: SiteKey;
  activePage: AppPage;
  collapsed: boolean;
  onToggleCollapse: () => void;
  onSiteChange: (site: SiteKey) => void;
  onNavigate: (page: AppPage) => void;
}

const SITE_NAV_ITEMS: Record<SiteKey, NavItem[]> = {
  culinary: [
    { page: 'dashboard', label: 'Панель CU', icon: 'dashboard', shortcut: '1' },
    { page: 'sites', label: 'Сайт dima-fomin.pl', icon: 'globe', shortcut: '2' },
    { page: 'affiliate', label: 'Affiliate товары', icon: 'shop', shortcut: '3' },
    { page: 'content', label: 'Статьи и обзоры', icon: 'cms', shortcut: '4' },
    { page: 'ai-studio', label: 'AI для кулинарии', icon: 'bot', shortcut: '5' },
    { page: 'culinary', label: 'Ингредиенты', icon: 'catalog', shortcut: '6' },
    { page: 'leads', label: 'Заявки CU', icon: 'leads', shortcut: '7' },
    { page: 'analytics', label: 'Аналитика CU', icon: 'analytics', shortcut: 'A' },
    { page: 'users', label: 'Пользователи', icon: 'users', shortcut: 'U' },
    { page: 'settings', label: 'Настройки CU', icon: 'settings', shortcut: 'S' },
    { page: 'about', label: 'О системе', icon: 'shield', shortcut: '?' }
  ],
  construction: [
    { page: 'dashboard', label: 'Панель CO', icon: 'dashboard', shortcut: '1' },
    { page: 'sites', label: 'Сайт kazaxbud', icon: 'globe', shortcut: '2' },
    { page: 'construction', label: 'Kazaxbud CMS', icon: 'building', shortcut: '3' },
    { page: 'affiliate', label: 'Поставки / affiliate', icon: 'shop', shortcut: '4' },
    { page: 'ai-studio', label: 'AI для стройки', icon: 'bot', shortcut: '5' },
    { page: 'leads', label: 'Заявки CO', icon: 'leads', shortcut: '6' },
    { page: 'suppliers', label: 'Поставщики', icon: 'suppliers', shortcut: '7' },
    { page: 'analytics', label: 'Аналитика CO', icon: 'analytics', shortcut: 'A' },
    { page: 'users', label: 'Пользователи', icon: 'users', shortcut: 'U' },
    { page: 'settings', label: 'Настройки CO', icon: 'settings', shortcut: 'S' },
    { page: 'about', label: 'О системе', icon: 'shield', shortcut: '?' }
  ]
};

const defaultSitePages: Record<SiteKey, AppPage> = {
  culinary: 'content',
  construction: 'construction'
};

export function getSiteNavItems(site: ManagedSite): NavItem[] {
  if (site === 'construction' || site === 'almabuild') return SITE_NAV_ITEMS.construction;
  return SITE_NAV_ITEMS.culinary;
}

export function defaultPageForSite(site: ManagedSite): AppPage {
  return site === 'construction' || site === 'almabuild' ? defaultSitePages.construction : defaultSitePages.culinary;
}

export function pageAllowedForSite(site: ManagedSite, page: AppPage) {
  return getSiteNavItems(site).some((item) => item.page === page);
}

export function normalizeSitePage(site: ManagedSite, page: AppPage): AppPage {
  if (pageAllowedForSite(site, page)) return page;
  return defaultPageForSite(site);
}

export function Sidebar({ activeSite, activePage, collapsed, onToggleCollapse, onSiteChange, onNavigate }: SidebarProps) {
  const visibleNavItems = getSiteNavItems(activeSite);
  const activeConfig = siteConfigs.find((site) => site.key === activeSite) ?? siteConfigs[0];
  const sectionLabel = activeSite === 'construction' ? 'Разделы строительного сайта' : 'Разделы кулинарного сайта';

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
              <span className="site-switcher-mark">{site.key === 'culinary' ? 'CU' : 'CO'}</span>
              <span><strong>{site.name}</strong><small>{site.domain}</small></span>
              <i className={'site-dot ' + (site.apiStatus === 'online' ? 'prod' : 'warning')} />
            </button>
          );
        })}
      </div>

      <nav className="sidebar-nav" aria-label="Основная навигация">
        <p className="sidebar-section-label">{sectionLabel}</p>
        {visibleNavItems.map((item) => (
          <button key={item.page} className={'sidebar-link' + (activePage === item.page ? ' active' : '')} type="button" title={item.label} onClick={() => onNavigate(item.page)}>
            <AppIcon name={item.icon} />
            <span>{item.label}</span>
            <kbd>{item.shortcut}</kbd>
          </button>
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
