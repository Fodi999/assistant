import { AppIcon, type AppIconName } from '../AppIcon';
import { SiteSwitcher } from './SiteSwitcher';
import type { ActiveSiteId } from './ActiveSiteContext';
import { useActiveSite } from './ActiveSiteContext';

export type AppPage =
  | 'dashboard'
  | 'sites'
  | 'cms'
  | 'church-content'
  | 'icons'
  | 'calendar'
  | 'leads'
  | 'catalog'
  | 'shop'
  | 'orders'
  | 'suppliers'
  | 'media'
  | 'translations'
  | 'analytics'
  | 'users'
  | 'ai-studio'
  | 'settings';

export type AdminNavItem = {
  page: AppPage;
  label: string;
  icon: AppIconName;
};

export const adminNavItems: AdminNavItem[] = [
  { page: 'dashboard', label: 'Dashboard', icon: 'dashboard' },
  { page: 'sites', label: 'Sites', icon: 'globe' },
  { page: 'cms', label: 'CMS', icon: 'cms' },
  { page: 'church-content', label: 'Church Content', icon: 'calendar' },
  { page: 'icons', label: 'Icons', icon: 'qr' },
  { page: 'calendar', label: 'Calendar', icon: 'calendar' },
  { page: 'leads', label: 'Leads', icon: 'leads' },
  { page: 'catalog', label: 'Catalog', icon: 'catalog' },
  { page: 'shop', label: 'Shop', icon: 'shop' },
  { page: 'orders', label: 'Orders', icon: 'package' },
  { page: 'suppliers', label: 'Suppliers', icon: 'suppliers' },
  { page: 'media', label: 'Media', icon: 'image' },
  { page: 'translations', label: 'Translations', icon: 'globe' },
  { page: 'analytics', label: 'Analytics', icon: 'analytics' },
  { page: 'users', label: 'Users', icon: 'users' },
  { page: 'ai-studio', label: 'AI Studio', icon: 'bot' },
  { page: 'settings', label: 'Settings', icon: 'settings' }
];

export const adminPageLabels = adminNavItems.reduce<Record<AppPage, string>>((labels, item) => {
  labels[item.page] = item.label;
  return labels;
}, {} as Record<AppPage, string>);

type AdminSidebarProps = {
  activePage: AppPage;
  collapsed: boolean;
  onNavigate: (page: AppPage) => void;
  onSiteChange: (siteId: ActiveSiteId) => void;
  onToggleCollapse: () => void;
};

export function AdminSidebar({ activePage, collapsed, onNavigate, onSiteChange, onToggleCollapse }: AdminSidebarProps) {
  const { activeSite } = useActiveSite();
  const visibleItems = adminNavItems.filter((item) => {
    if (item.page === 'church-content') return activeSite.id === 'church';
    if (item.page === 'icons') return false;
    if (item.page === 'calendar') return activeSite.id !== 'church';
    return true;
  });

  return (
    <aside className={'admin-sidebar' + (collapsed ? ' collapsed' : '')} data-site-accent={activeSite.accent}>
      <div className="admin-sidebar-brand">
        <div className="admin-brand-mark">A</div>
        <div>
          <p className="admin-sidebar-title">Admin CRM</p>
          <p className="admin-sidebar-subtitle">Sites control</p>
        </div>
      </div>

      <SiteSwitcher collapsed={collapsed} onSiteChange={onSiteChange} />

      <nav className="admin-sidebar-nav" aria-label="CRM navigation">
        {visibleItems.map((item) => (
          <button
            key={item.page}
            className={'admin-sidebar-link' + (activePage === item.page ? ' active' : '')}
            type="button"
            title={item.label}
            aria-current={activePage === item.page ? 'page' : undefined}
            onClick={() => onNavigate(item.page)}
          >
            <AppIcon name={item.icon} />
            <span>{item.label}</span>
          </button>
        ))}
      </nav>

      <div className="admin-sidebar-footer">
        <div className="admin-system-line">
          <span>Backend</span>
          <strong><i className={'admin-live-dot ' + activeSite.apiStatus} />{activeSite.apiStatus}</strong>
        </div>
        <div className="admin-system-line">
          <span>Site ID</span>
          <strong>{activeSite.id}</strong>
        </div>
      </div>

      <button className="admin-sidebar-toggle" type="button" aria-pressed={collapsed} onClick={onToggleCollapse}>
        <AppIcon name="chevron-left" />
        <span>{collapsed ? 'Expand' : 'Collapse'}</span>
      </button>
    </aside>
  );
}
