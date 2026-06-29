import { useState } from 'react';
import { useAdminAuth } from './components/admin/AuthContext';
import { AdminLayout } from './components/admin/AdminLayout';
import { type AppPage } from './components/admin/AdminSidebar';
import {
  ACTIVE_SITE_STORAGE_KEY,
  activeSiteIdToLegacyKey,
  getActiveSiteOptions,
  normalizeActiveSiteId,
  type ActiveSiteId
} from './components/admin/ActiveSiteContext';
import { DataTable } from './components/admin/DataTable';
import { EmptyState } from './components/admin/EmptyState';
import { StatusBadge } from './components/admin/StatusBadge';
import { AnalyticsPage } from './pages/admin/AnalyticsPage';
import { CalendarPage } from './pages/admin/CalendarPage';
import { ChurchContentPage } from './pages/admin/ChurchContentPage';
import { CatalogPage } from './pages/admin/CatalogPage';
import { CMSPage } from './pages/admin/CMSPage';
import { LeadsPage } from './pages/admin/LeadsPage';
import { MediaPage } from './pages/admin/MediaPage';
import { OrdersPage } from './pages/admin/OrdersPage';
import { SettingsPage } from './pages/admin/SettingsPage';
import { ShopPage } from './pages/admin/ShopPage';
import { SuppliersPage } from './pages/admin/SuppliersPage';
import { UsersPage } from './pages/admin/UsersPage';
import { LoginPage } from './pages/admin/LoginPage';
import { DashboardPage } from './pages/shared/DashboardPage';

type PageBySite = Record<ActiveSiteId, AppPage>;

const defaultPageBySite: PageBySite = {
  church: 'dashboard',
  construction: 'dashboard',
  kitchen: 'dashboard'
};

export function App() {
  const { authState, authError, loading, login, logout } = useAdminAuth();
  const [activeSiteId, setActiveSiteId] = useState<ActiveSiteId>(() => normalizeActiveSiteId(localStorage.getItem(ACTIVE_SITE_STORAGE_KEY)));
  const [pageBySite, setPageBySite] = useState<PageBySite>(defaultPageBySite);
  const [sidebarCollapsed, setSidebarCollapsed] = useState(() => localStorage.getItem('admin_sidebar_collapsed') === 'true');
  const activeSite = activeSiteIdToLegacyKey(activeSiteId);
  const activePage = normalizePageForSite(pageBySite[activeSiteId] ?? 'dashboard', activeSiteId);
  const [dataError] = useState<string | null>(null);

  function navigate(page: AppPage) {
    setPageBySite((current) => ({ ...current, [activeSiteId]: normalizePageForSite(page, activeSiteId) }));
  }

  function changeSite(siteId: ActiveSiteId) {
    localStorage.setItem(ACTIVE_SITE_STORAGE_KEY, siteId);
    setPageBySite((current) => ({ ...defaultPageBySite, ...current }));
    setActiveSiteId(siteId);
  }

  function toggleSidebar() {
    setSidebarCollapsed((current) => {
      const next = !current;
      localStorage.setItem('admin_sidebar_collapsed', String(next));
      return next;
    });
  }

  function renderPage() {
    switch (activePage) {
      case 'dashboard': return <DashboardPage activeSite={activeSite} onNavigate={navigate} />;
      case 'sites': return <SitesOverview />;
      case 'cms': return <CMSPage />;
      case 'church-content': return activeSiteId === 'church'
        ? <ChurchContentPage />
        : <AdminPlaceholder page="Church Content" icon="calendar" description="Church content editor is available only for the church site." />;
      case 'icons': return activeSiteId === 'church'
        ? <ChurchContentPage />
        : <AdminPlaceholder page="Icons" icon="qr" description="Legacy icons editor is hidden. Use the site-specific content editor." />;
      case 'calendar': return activeSiteId === 'church' ? <ChurchContentPage /> : <CalendarPage />;
      case 'leads': return <LeadsPage />;
      case 'catalog': return <CatalogPage />;
      case 'shop': return <ShopPage />;
      case 'orders': return <OrdersPage />;
      case 'suppliers': return <SuppliersPage />;
      case 'media': return <MediaPage />;
      case 'translations': return <AdminPlaceholder page="Translations" icon="globe" description="Единая панель переводов готова как раздел навигации; backend-интеграция будет следующим этапом." />;
      case 'analytics': return <AnalyticsPage />;
      case 'users': return <UsersPage />;
      case 'ai-studio': return <AdminPlaceholder page="AI Studio" icon="bot" description="AI-инструменты оставлены в меню CRM и ждут подключения готовых сценариев." />;
      case 'settings': return <SettingsPage />;
      default: return <DashboardPage activeSite={activeSite} onNavigate={navigate} />;
    }
  }

  if (authState === 'checking') return <main className="login-page"><p className="page-muted">Проверяем подключение к backend...</p></main>;
  if (authState === 'anonymous') return <LoginPage loading={loading} error={authError} onLogin={login} />;

  return (
    <AdminLayout
      activePage={activePage}
      activeSiteId={activeSiteId}
      collapsed={sidebarCollapsed}
      connectionState={dataError ? 'limited' : 'online'}
      onNavigate={navigate}
      onSiteChange={changeSite}
      onToggleSidebar={toggleSidebar}
      onLogout={logout}
    >
      {renderPage()}
    </AdminLayout>
  );
}

function normalizePageForSite(page: AppPage, siteId: ActiveSiteId): AppPage {
  if (siteId === 'church' && (page === 'icons' || page === 'calendar')) return 'church-content';
  if (siteId !== 'church' && page === 'church-content') return 'dashboard';
  return page;
}

function SitesOverview() {
  const rows = getActiveSiteOptions();

  return (
    <section className="admin-dashboard-page">
      <div className="admin-page-hero">
        <div>
          <p>Sites</p>
          <h2>Управление сайтами</h2>
          <span>church, construction и kitchen в одном CRM-каркасе.</span>
        </div>
      </div>

      <DataTable
        rows={rows}
        getRowKey={(site) => site.id}
        columns={[
          { key: 'site', header: 'Site', render: (site) => <strong>{site.id}</strong> },
          { key: 'name', header: 'Name', render: (site) => site.name },
          { key: 'domain', header: 'Domain', render: (site) => site.domain },
          { key: 'language', header: 'Language', render: (site) => site.language },
          { key: 'status', header: 'Status', render: (site) => <StatusBadge status={site.status} /> },
          { key: 'api', header: 'API', render: (site) => <StatusBadge status={site.apiStatus} /> }
        ]}
      />
    </section>
  );
}

function AdminPlaceholder({ page, icon, description }: { page: string; icon: 'calendar' | 'catalog' | 'image' | 'globe' | 'bot' | 'qr'; description: string }) {
  return (
    <section className="admin-dashboard-page">
      <div className="admin-page-hero">
        <div>
          <p>{page}</p>
          <h2>{page}</h2>
          <span>{description}</span>
        </div>
      </div>
      <EmptyState
        icon={icon}
        title="Раздел готов к подключению"
        description="Пока используются аккуратные mock/empty состояния, чтобы не завязываться на неготовые backend endpoints."
      />
    </section>
  );
}
