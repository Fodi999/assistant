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

interface SidebarProps {
  activeSite: SiteKey;
  activePage: AppPage;
  collapsed: boolean;
  onToggleCollapse: () => void;
  onSiteChange: (site: SiteKey) => void;
  onNavigate: (page: AppPage) => void;
}

const NAV_ITEMS: Array<{ page: AppPage; label: string; icon: AppIconName; shortcut: string; site?: SiteKey }> = [
  { page: 'dashboard', label: 'Панель', icon: 'dashboard', shortcut: '1' },
  { page: 'sites', label: 'Сайты', icon: 'globe', shortcut: '2' },
  { page: 'affiliate', label: 'Партнерка', icon: 'shop', shortcut: '3' },
  { page: 'content', label: 'Контент', icon: 'cms', shortcut: '4' },
  { page: 'ai-studio', label: 'AI-студия', icon: 'bot', shortcut: '5' },
  { page: 'construction', label: 'Стройка', icon: 'building', shortcut: '6', site: 'construction' },
  { page: 'culinary', label: 'Кулинария', icon: 'catalog', shortcut: '7', site: 'culinary' },
  { page: 'leads', label: 'Заявки', icon: 'leads', shortcut: '8' },
  { page: 'suppliers', label: 'Поставщики', icon: 'suppliers', shortcut: '9' },
  { page: 'analytics', label: 'Аналитика', icon: 'analytics', shortcut: 'A' },
  { page: 'users', label: 'Пользователи', icon: 'users', shortcut: 'U' },
  { page: 'settings', label: 'Настройки', icon: 'settings', shortcut: 'S' },
  { page: 'about', label: 'О системе', icon: 'shield', shortcut: '?' }
];

export function pageAllowedForSite(site: ManagedSite, page: AppPage) {
  const item = NAV_ITEMS.find((nav) => nav.page === page);
  return !item?.site || item.site === site;
}

export function normalizeSitePage(site: ManagedSite, page: AppPage): AppPage {
  if (pageAllowedForSite(site, page)) return page;
  return site === 'construction' ? 'construction' : 'culinary';
}

export function Sidebar({ activeSite, activePage, collapsed, onToggleCollapse, onSiteChange, onNavigate }: SidebarProps) {
  const visibleNavItems = NAV_ITEMS.filter((item) => !item.site || item.site === activeSite);
  const activeConfig = siteConfigs.find((site) => site.key === activeSite) ?? siteConfigs[0];

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
        <p className="sidebar-section-label">Разделы</p>
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
