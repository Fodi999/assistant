import { AppIcon, type AppIconName } from './AppIcon';

export type AppPage =
  | 'overview' | 'sites' | 'leads' | 'catalog' | 'materials' | 'suppliers'
  | 'projects' | 'seo' | 'analytics' | 'ai' | 'usb' | 'deployments' | 'settings';

export type ManagedSite = 'almabuild' | 'dima';

interface SidebarProps {
  activeSite: ManagedSite;
  activePage: AppPage;
  onSiteChange: (site: ManagedSite) => void;
  onNavigate: (page: AppPage) => void;
}

const SITE_SWITCH_ITEMS: Array<{ id: ManagedSite; label: string; domain: string; status: 'prod' | 'warning' }> = [
  { id: 'almabuild', label: 'KAZAXBUD', domain: 'kazaxbud.pages.dev', status: 'warning' },
  { id: 'dima', label: 'Dima Fomin', domain: 'dima-fomin.pl', status: 'prod' }
];

const NAV_ITEMS: Array<{ page: AppPage; label: string; icon: AppIconName; shortcut: string }> = [
  { page: 'overview', label: 'Обзор', icon: 'dashboard', shortcut: '1' },
  { page: 'sites', label: 'Сайт', icon: 'globe', shortcut: '2' },
  { page: 'leads', label: 'Заявки', icon: 'leads', shortcut: '3' },
  { page: 'catalog', label: 'Каталог', icon: 'catalog', shortcut: '4' },
  { page: 'materials', label: 'Материалы', icon: 'materials', shortcut: '5' },
  { page: 'suppliers', label: 'Поставщики', icon: 'suppliers', shortcut: '6' },
  { page: 'projects', label: 'Проекты', icon: 'folder', shortcut: '7' },
  { page: 'seo', label: 'SEO-фабрика', icon: 'seo', shortcut: '8' },
  { page: 'analytics', label: 'Аналитика', icon: 'analytics', shortcut: '9' },
  { page: 'ai', label: 'AI-студия', icon: 'bot', shortcut: 'A' },
  { page: 'usb', label: 'USB Key', icon: 'hard-drive', shortcut: 'U' },
  { page: 'deployments', label: 'Деплои', icon: 'deploy', shortcut: 'D' },
  { page: 'settings', label: 'Настройки', icon: 'settings', shortcut: 'S' }
];

function navItemAllowed(site: ManagedSite, page: AppPage) {
  if (site === 'almabuild' && page === 'catalog') return false;
  if (site === 'dima' && page === 'materials') return false;
  if (site === 'dima' && page === 'projects') return false;
  return true;
}

export function Sidebar({ activeSite, activePage, onSiteChange, onNavigate }: SidebarProps) {
  const visibleNavItems = NAV_ITEMS.filter((item) => navItemAllowed(activeSite, item.page));

  return (
    <aside className="sidebar">
      <div className="sidebar-brand">
        <div className="brand-mark">AO</div>
        <div>
          <p className="sidebar-title">ALMABUILD OS</p>
          <p className="sidebar-subtitle">Центр управления операциями</p>
        </div>
      </div>

      <div className="site-switcher" aria-label="Сайты">
        <p className="sidebar-section-label">Сайты</p>
        {SITE_SWITCH_ITEMS.map((site) => {
          const active = site.id === activeSite;
          return (
            <button key={site.id} className={'site-switcher-option' + (active ? ' active' : '')} type="button" aria-current={active ? 'true' : undefined} onClick={() => onSiteChange(site.id)}>
              <span className="site-switcher-mark">{site.label.slice(0, 2)}</span>
              <span><strong>{site.label}</strong><small>{site.domain}</small></span>
              <i className={'site-dot ' + site.status} />
            </button>
          );
        })}
      </div>

      <nav className="sidebar-nav" aria-label="Основная навигация">
        <p className="sidebar-section-label">Разделы управления</p>
        {visibleNavItems.map((item) => (
          <button key={item.page} className={'sidebar-link' + (activePage === item.page ? ' active' : '')} type="button" onClick={() => onNavigate(item.page)}>
            <AppIcon name={item.icon} />
            <span>{item.label}</span>
            <kbd>{item.shortcut}</kbd>
          </button>
        ))}
      </nav>

      <div className="sidebar-system">
        <div className="system-row"><span>API</span><strong><i className="live-dot" /> Онлайн</strong></div>
        <div className="system-row"><span>Локальный бэкенд</span><strong><i className="live-dot warning" /> Tauri</strong></div>
        <div className="system-row"><span>Версия</span><strong>v2.6.0</strong></div>
        <div className="sidebar-user"><span>ДА</span><p><strong>Дима Админ</strong><small>Суперадминистратор</small></p></div>
      </div>

      <button className="sidebar-collapse" type="button"><AppIcon name="chevron-left" /><span>Компактный режим</span></button>
    </aside>
  );
}
